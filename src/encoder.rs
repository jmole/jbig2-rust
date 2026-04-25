//! High-level JBIG2 encoder.
//!
//! The encoder emits a minimal sequential-organization JBIG2 file with one
//! page information segment plus either a generic region, a symbol dictionary
//! + text region, or a pattern dictionary + halftone region.
//!
//! See [`EncoderConfig`] for the tunables surfaced to callers.

use std::io::Write;

use crate::bitmap::Bitmap;
use crate::coding::mq::{MqContexts, MqEncoder, MQ_NUM_CONTEXTS};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::{
    file_header::FileHeader,
    generic_region::{encode_generic_arith, encode_generic_mmr, nominal_at, GenericRegionHeader},
    halftone_region::{encode_halftone_region_with_contexts, HalftoneRegionHeader},
    page_information::{CombinationOp, PageInformation},
    pattern_dictionary::{encode_pattern_dictionary_with_contexts, PatternDictionaryHeader},
    refinement_region::NOMINAL_REFINEMENT_AT,
    region_info::RegionInfo,
    symbol_dictionary::{encode_symbol_dictionary_with_contexts, SymbolDictionaryHeader},
    text_region::{
        encode_text_region_with_contexts, RefCorner, RefinedInstance, SymbolInstance,
        TextRegionHeader,
    },
    AtPixels, SegmentHeader, SegmentType,
};
use crate::symbol::{
    cc::extract_components,
    classify::{classify_lossy, xor_stats},
    identity::classify_identity,
};

/// Generic region template selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GenericTemplate {
    /// 16-pixel template (best ratio).
    T0,
    /// 13-pixel template.
    T1,
    /// 10-pixel 3-row template.
    T2,
    /// 10-pixel 2-row template.
    T3,
    /// 12-AT extended template (AMD2).
    Extended,
}

impl GenericTemplate {
    /// Map to a `GBTEMPLATE` byte and `EXTTEMPLATE` flag.
    pub fn as_bits(self) -> (u8, bool) {
        match self {
            Self::T0 => (0, false),
            Self::T1 => (1, false),
            Self::T2 => (2, false),
            Self::T3 => (3, false),
            Self::Extended => (0, true),
        }
    }
}

/// Coding method used by the encoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Coding {
    /// MQ arithmetic coding (best ratio).
    Arithmetic,
    /// T.6 MMR (fast, decent on line art).
    Mmr,
}

/// High-level encoding strategy for page content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mode {
    /// Emit the whole page as a single generic region.
    Generic,
    /// Extract exact-match symbols and emit a symbol dictionary + text region.
    SymbolLossless,
    /// Lossy classifier + optional refinement-against-dictionary instances.
    SymbolLossy,
}

/// Direct symbol-coding strategy for [`Jbig2Encoder::write_page_symbols`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum SymbolCoding {
    /// Exact-match deduplication only.
    Lossless,
    /// Lossy clustering with an optional refinement pass to recover the
    /// original glyph shapes.
    Lossy {
        /// Fractional similarity threshold passed to the lossy classifier.
        threshold: f32,
        /// Emit refinement data for instances that differ from the chosen
        /// dictionary representative.
        refine_after_match: bool,
    },
}

/// Page-local geometry for [`Jbig2Encoder::write_page_halftone`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HalftonePageOptions {
    /// Page width in pixels.
    pub page_width: u32,
    /// Page height in pixels.
    pub page_height: u32,
    /// Region X position on the page.
    pub region_x: u32,
    /// Region Y position on the page.
    pub region_y: u32,
    /// Region width in pixels.
    pub region_width: u32,
    /// Region height in pixels.
    pub region_height: u32,
    /// `HENABLESKIP`.
    pub enable_skip: bool,
    /// `HCOMBOP`.
    pub comb_op: CombinationOp,
    /// `HDEFPIXEL`.
    pub default_pixel: u8,
    /// `HGW`.
    pub grid_width: u32,
    /// `HGH`.
    pub grid_height: u32,
    /// `HGX` (scaled by 256).
    pub grid_x: i32,
    /// `HGY` (scaled by 256).
    pub grid_y: i32,
    /// `HRX` (scaled by 256).
    pub step_x: u16,
    /// `HRY` (scaled by 256).
    pub step_y: u16,
}

/// Encoder tunables. The three named constructors [`EncoderConfig::fast`],
/// [`EncoderConfig::balanced`], and [`EncoderConfig::max_compression`] expose
/// canonical speed/size tradeoffs.
#[derive(Clone, Debug)]
pub struct EncoderConfig {
    /// Page-coding strategy.
    pub mode: Mode,
    /// Generic region template.
    pub template: GenericTemplate,
    /// Coding method.
    pub coding: Coding,
    /// Override the AT pixel positions from the spec nominal defaults.
    pub adaptive_templates: Option<AtPixels>,
    /// Emit refinement regions where available.
    pub refinement: bool,
    /// Enable typical-prediction duplicate-line removal (TPGD) for top-level
    /// generic region pages. Symbol dictionaries, pattern dictionaries, and
    /// halftone bitplanes do not carry a corresponding flag on the wire.
    pub generic_region_duplicate_line_removal: bool,
    /// Lossy match threshold (used only in [`Mode::SymbolLossy`]).
    pub symbol_threshold: f32,
    /// Run a post-match refinement pass on matched symbols (lossy path).
    pub refine_after_match: bool,
    /// Lossless refinement/promotion heuristics for lossy-symbol + refinement mode.
    pub refinement_gate: RefinementGate,
    /// Try multiple lossless encodings and emit the smallest valid page stream.
    #[doc(hidden)]
    pub rate_select: bool,
}

/// Lossless heuristics that decide whether differing matched symbols should
/// stay as refinements or be promoted to exact dictionary symbols.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RefinementGate {
    /// Promote differing instances whose XOR popcount is below this threshold.
    pub min_xor_popcount: u32,
    /// Promote differing instances whose XOR popcount divided by bitmap area
    /// exceeds this threshold.
    pub max_xor_fraction: f32,
    /// Promote an exact variant only when it appears at least this many times.
    pub promote_min_freq: u32,
    /// Fall back to generic coding if exact-match symbol reuse is weaker than
    /// this ratio. `1.0` preserves the historical always-symbol behavior.
    pub min_dedup_ratio: f32,
}

impl Default for RefinementGate {
    fn default() -> Self {
        Self {
            min_xor_popcount: 0,
            max_xor_fraction: 1.0,
            promote_min_freq: u32::MAX,
            min_dedup_ratio: 1.0,
        }
    }
}

impl EncoderConfig {
    /// Speed-first preset: Template 3, arithmetic, generic mode, no TPGD.
    pub fn fast() -> Self {
        Self {
            mode: Mode::Generic,
            template: GenericTemplate::T3,
            coding: Coding::Arithmetic,
            adaptive_templates: None,
            refinement: false,
            generic_region_duplicate_line_removal: false,
            symbol_threshold: 0.97,
            refine_after_match: false,
            refinement_gate: RefinementGate::default(),
            rate_select: false,
        }
    }

    /// Balanced preset: Template 0, arithmetic, symbol-lossless mode, TPGD,
    /// with rate selection against generic T0+TPGD for single-page streams.
    pub fn balanced() -> Self {
        Self {
            mode: Mode::SymbolLossless,
            template: GenericTemplate::T0,
            coding: Coding::Arithmetic,
            adaptive_templates: None,
            refinement: false,
            generic_region_duplicate_line_removal: true,
            symbol_threshold: 0.97,
            refine_after_match: false,
            refinement_gate: RefinementGate::default(),
            rate_select: true,
        }
    }

    /// Maximum-compression preset: Template 0, arithmetic, symbol-lossy at
    /// 0.85, refinement, TPGD.
    pub fn max_compression() -> Self {
        Self {
            mode: Mode::SymbolLossy,
            template: GenericTemplate::T0,
            coding: Coding::Arithmetic,
            adaptive_templates: None,
            refinement: true,
            generic_region_duplicate_line_removal: true,
            symbol_threshold: 0.85,
            refine_after_match: true,
            refinement_gate: RefinementGate {
                max_xor_fraction: 0.10,
                promote_min_freq: 3,
                min_dedup_ratio: 0.5,
                ..RefinementGate::default()
            },
            rate_select: true,
        }
    }

    fn symbol_coding(&self) -> Option<SymbolCoding> {
        match self.mode {
            Mode::Generic => None,
            Mode::SymbolLossless => Some(SymbolCoding::Lossless),
            Mode::SymbolLossy => Some(SymbolCoding::Lossy {
                threshold: self.symbol_threshold,
                refine_after_match: self.refinement && self.refine_after_match,
            }),
        }
    }

    fn should_rate_select(&self) -> bool {
        self.rate_select
            && matches!(self.coding, Coding::Arithmetic)
            && match self.mode {
                Mode::Generic => false,
                Mode::SymbolLossless => true,
                Mode::SymbolLossy => self.refinement && self.refine_after_match,
            }
    }
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self::balanced()
    }
}

/// Single-page JBIG2 encoder.
pub struct Jbig2Encoder<W: Write> {
    writer: W,
    cfg: EncoderConfig,
    mq_cxs: MqContexts,
    mq_enc: MqEncoder,
    mq_body: Vec<u8>,
    segment_buf: Vec<u8>,
    seg_no: u32,
    file_header_emitted: bool,
    stream_finalized: bool,
    pages_emitted: u32,
    total_pages: u32,
}

impl<W: Write> Jbig2Encoder<W> {
    /// Create an encoder for a single-page file.
    pub fn new(writer: W, cfg: EncoderConfig) -> Self {
        Self {
            writer,
            cfg,
            mq_cxs: MqContexts::new(MQ_NUM_CONTEXTS),
            mq_enc: MqEncoder::new(0),
            mq_body: Vec::new(),
            segment_buf: Vec::new(),
            seg_no: 0,
            file_header_emitted: false,
            stream_finalized: false,
            pages_emitted: 0,
            total_pages: 1,
        }
    }

    /// Declare the expected number of pages so the file header can carry a
    /// definite page count. Defaults to 1 when unset.
    pub fn with_num_pages(mut self, n: u32) -> Self {
        self.total_pages = n;
        self
    }

    fn begin_page(
        &mut self,
        width: u32,
        height: u32,
        is_lossless: bool,
        may_contain_refinements: bool,
        may_contain_colour: bool,
    ) -> Jbig2Result<u32> {
        self.ensure_file_header()?;
        self.pages_emitted += 1;
        let page_assoc = self.pages_emitted;

        let page_info = PageInformation {
            width,
            height,
            x_resolution: 0,
            y_resolution: 0,
            is_lossless,
            may_contain_refinements,
            default_pixel: 0,
            default_combination_op: CombinationOp::Or,
            requires_aux_buffers: false,
            combination_op_override: false,
            may_contain_colour,
            is_striped: false,
            maximum_stripe_size: 0,
        };
        self.emit_segment(
            SegmentType::PageInformation,
            page_assoc,
            vec![],
            vec![false],
            |w| page_info.write(w),
        )?;
        Ok(page_assoc)
    }

    fn end_page(&mut self, page_assoc: u32) -> Jbig2Result<()> {
        self.emit_segment(
            SegmentType::EndOfPage,
            page_assoc,
            vec![],
            vec![false],
            |_w| Ok(()),
        )
    }

    fn ensure_file_header(&mut self) -> Jbig2Result<()> {
        if self.file_header_emitted {
            return Ok(());
        }
        FileHeader {
            sequential: true,
            unknown_page_count: false,
            uses_extended_template: matches!(self.cfg.template, GenericTemplate::Extended),
            uses_colour: false,
            num_pages: Some(self.total_pages),
        }
        .write(&mut self.writer)?;
        self.file_header_emitted = true;
        Ok(())
    }

    /// Encode one page bitmap.
    pub fn write_page(&mut self, bitmap: &Bitmap) -> Jbig2Result<()> {
        if self.can_rate_select_full_stream() {
            return self.write_rate_selected_page(bitmap);
        }
        match self.cfg.symbol_coding() {
            Some(coding) => self.write_page_symbols(bitmap, coding),
            None => {
                let page_assoc =
                    self.begin_page(bitmap.width(), bitmap.height(), true, false, false)?;
                self.encode_page_generic(bitmap, page_assoc)?;
                self.end_page(page_assoc)
            }
        }
    }

    fn can_rate_select_full_stream(&self) -> bool {
        self.cfg.should_rate_select()
            && !self.file_header_emitted
            && self.pages_emitted == 0
            && self.total_pages == 1
    }

    fn write_rate_selected_page(&mut self, bitmap: &Bitmap) -> Jbig2Result<()> {
        let mut generic_cfg = self.cfg.clone();
        generic_cfg.mode = Mode::Generic;
        generic_cfg.template = GenericTemplate::T0;
        generic_cfg.generic_region_duplicate_line_removal = true;
        generic_cfg.refinement = false;
        generic_cfg.refine_after_match = false;
        generic_cfg.rate_select = false;

        let mut symbol_cfg = self.cfg.clone();
        symbol_cfg.rate_select = false;

        let generic = Self::encode_page_to_vec(bitmap, generic_cfg);
        let symbol = Self::encode_page_to_vec(bitmap, symbol_cfg);
        let chosen = match (generic, symbol) {
            (Ok(generic), Ok(symbol)) => {
                if generic.len() <= symbol.len() {
                    generic
                } else {
                    symbol
                }
            }
            (Ok(generic), Err(_)) => generic,
            (Err(_), Ok(symbol)) => symbol,
            (Err(generic_err), Err(_symbol_err)) => return Err(generic_err),
        };

        self.writer.write_all(&chosen)?;
        self.file_header_emitted = true;
        self.stream_finalized = true;
        self.pages_emitted = 1;
        Ok(())
    }

    fn encode_page_to_vec(bitmap: &Bitmap, mut cfg: EncoderConfig) -> Jbig2Result<Vec<u8>> {
        cfg.rate_select = false;
        let mut encoder = Jbig2Encoder::new(Vec::new(), cfg);
        encoder.write_page(bitmap)?;
        encoder.finish()
    }

    /// Encode one page through the symbol-dictionary + text-region path.
    pub fn write_page_symbols(&mut self, bitmap: &Bitmap, coding: SymbolCoding) -> Jbig2Result<()> {
        self.validate_non_generic_template("symbol")?;
        let (symbol_threshold, may_contain_refinements, is_lossless) = match coding {
            SymbolCoding::Lossless => (1.0, false, true),
            SymbolCoding::Lossy {
                threshold,
                refine_after_match,
            } => {
                let refine = self.cfg.refinement && refine_after_match;
                (threshold, refine, refine)
            }
        };
        let page_assoc = self.begin_page(
            bitmap.width(),
            bitmap.height(),
            is_lossless,
            may_contain_refinements,
            false,
        )?;
        self.encode_page_symbol(bitmap, page_assoc, symbol_threshold, coding)?;
        self.end_page(page_assoc)
    }

    /// Encode one page from a caller-supplied pattern dictionary and halftone
    /// grid. The arithmetic/MMR coding choice and template come from the
    /// encoder's [`EncoderConfig`].
    pub fn write_page_halftone(
        &mut self,
        patterns: &[Bitmap],
        gray_values: &[u32],
        opts: HalftonePageOptions,
    ) -> Jbig2Result<()> {
        self.validate_non_generic_template("halftone")?;
        if patterns.is_empty() {
            return Err(Jbig2Error::InvalidConfig(
                "halftone page: need at least one pattern",
            ));
        }
        if gray_values.len() != (opts.grid_width as usize) * (opts.grid_height as usize) {
            return Err(Jbig2Error::InvalidConfig(
                "halftone page: gray-value count does not match grid dimensions",
            ));
        }
        let pattern_w = patterns[0].width();
        let pattern_h = patterns[0].height();
        if patterns
            .iter()
            .any(|pat| pat.width() != pattern_w || pat.height() != pattern_h)
        {
            return Err(Jbig2Error::InvalidConfig(
                "halftone page: mixed pattern sizes",
            ));
        }

        let page_assoc = self.begin_page(opts.page_width, opts.page_height, true, false, false)?;
        let (template_id, _ext_template) = self.cfg.template.as_bits();
        let halftone_template = template_id.min(3);
        let pd_hdr = PatternDictionaryHeader {
            hdmmr: matches!(self.cfg.coding, Coding::Mmr),
            hd_template: halftone_template,
            hdpw: u8::try_from(pattern_w).map_err(|_| {
                Jbig2Error::InvalidConfig("halftone page: pattern width exceeds u8")
            })?,
            hdph: u8::try_from(pattern_h).map_err(|_| {
                Jbig2Error::InvalidConfig("halftone page: pattern height exceeds u8")
            })?,
            gray_max: patterns.len() as u32 - 1,
        };
        let pd_body = encode_pattern_dictionary_with_contexts(&pd_hdr, patterns, &mut self.mq_cxs)?;
        let mut pd_hdr_bytes = Vec::new();
        pd_hdr.write(&mut pd_hdr_bytes)?;
        let pd_seg_no = self.seg_no;
        self.emit_segment(
            SegmentType::PatternDictionary,
            page_assoc,
            vec![],
            vec![false],
            |w| {
                w.write_all(&pd_hdr_bytes)?;
                w.write_all(&pd_body)?;
                Ok(())
            },
        )?;

        let ht_hdr = HalftoneRegionHeader {
            region: RegionInfo {
                width: opts.region_width,
                height: opts.region_height,
                x: opts.region_x,
                y: opts.region_y,
                external_combination_op: opts.comb_op,
                colour_extension: false,
            },
            hmmr: matches!(self.cfg.coding, Coding::Mmr),
            htemplate: halftone_template,
            enable_skip: opts.enable_skip,
            hcombop: opts.comb_op,
            hdef_pixel: opts.default_pixel,
            hgw: opts.grid_width,
            hgh: opts.grid_height,
            hgx: opts.grid_x,
            hgy: opts.grid_y,
            hrx: opts.step_x,
            hry: opts.step_y,
        };
        let ht_body = encode_halftone_region_with_contexts(
            &ht_hdr,
            gray_values,
            patterns.len(),
            &mut self.mq_cxs,
        )?;
        let mut ht_hdr_bytes = Vec::new();
        ht_hdr.write(&mut ht_hdr_bytes)?;
        self.emit_segment(
            SegmentType::ImmediateLosslessHalftoneRegion,
            page_assoc,
            vec![pd_seg_no],
            vec![false, false],
            |w| {
                w.write_all(&ht_hdr_bytes)?;
                w.write_all(&ht_body)?;
                Ok(())
            },
        )?;
        self.end_page(page_assoc)
    }

    fn encode_page_generic(&mut self, bitmap: &Bitmap, page_assoc: u32) -> Jbig2Result<()> {
        let (template_id, ext_template) = self.cfg.template.as_bits();
        let at = self
            .cfg
            .adaptive_templates
            .unwrap_or_else(|| nominal_at(template_id, ext_template));
        // Some external decoders expose stale row padding when TPGD copies
        // non-byte-aligned rows. Keep TPGD to byte-aligned pages until the
        // padding behaviour is portable across decoders.
        let tpgdon = self.cfg.generic_region_duplicate_line_removal && (bitmap.width() & 7) == 0;
        let hdr = GenericRegionHeader {
            region: RegionInfo {
                width: bitmap.width(),
                height: bitmap.height(),
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            mmr: matches!(self.cfg.coding, Coding::Mmr),
            template: template_id,
            tpgdon,
            ext_template,
            at,
        };

        let mut hdr_bytes = Vec::new();
        hdr.write(&mut hdr_bytes)?;

        let coded = match self.cfg.coding {
            Coding::Arithmetic => {
                self.mq_cxs.reset();
                self.mq_enc.take_output_buffer(&mut self.mq_body);
                self.mq_enc.reset(bitmap.data().len() / 4 + 16);
                encode_generic_arith(&mut self.mq_enc, &mut self.mq_cxs, &hdr, bitmap)?;
                self.mq_enc.finish_into(&mut self.mq_body);
                std::mem::take(&mut self.mq_body)
            }
            Coding::Mmr => {
                #[cfg(feature = "mmr")]
                {
                    encode_generic_mmr(bitmap)?
                }
                #[cfg(not(feature = "mmr"))]
                {
                    return Err(Jbig2Error::InvalidConfig(
                        "MMR coding requested but feature disabled",
                    ));
                }
            }
        };

        self.emit_segment(
            SegmentType::ImmediateLosslessGenericRegion,
            page_assoc,
            vec![],
            vec![false],
            |w| {
                w.write_all(&hdr_bytes)?;
                w.write_all(&coded)?;
                Ok(())
            },
        )?;
        if matches!(self.cfg.coding, Coding::Arithmetic) {
            self.mq_body = coded;
        }
        Ok(())
    }

    fn encode_page_symbol(
        &mut self,
        bitmap: &Bitmap,
        page_assoc: u32,
        symbol_threshold: f32,
        coding: SymbolCoding,
    ) -> Jbig2Result<()> {
        // Step 1: connected-component extraction.
        let comps = extract_components(bitmap);
        if comps.is_empty() {
            // No foreground — fall back to generic encoding, which also
            // emits zero ink bytes.
            return self.encode_page_generic(bitmap, page_assoc);
        }
        // Step 2: classify. For `symbol_threshold = 1.0` we only merge exact
        // duplicates; at lower thresholds the WXOR-bucketed lossy classifier
        // allows near-matches per `jbig2enc`-style fractional agreement.
        let (mut symbols_raw, mut instance_symbol_raw, identity_symbol_count) =
            if symbol_threshold < 1.0 {
                let cls = classify_lossy(&comps, symbol_threshold);
                (cls.symbols, cls.instance_symbol, cls.identity_symbol_count)
            } else {
                let cls = classify_identity(&comps);
                let identity_symbol_count = cls.symbols.len() as u32;
                (cls.symbols, cls.instance_symbol, identity_symbol_count)
            };

        let dedup_ratio = identity_symbol_count as f32 / comps.len() as f32;
        if dedup_ratio > self.cfg.refinement_gate.min_dedup_ratio {
            return self.encode_page_generic(bitmap, page_assoc);
        }

        let want_refine = matches!(
            coding,
            SymbolCoding::Lossy {
                refine_after_match: true,
                ..
            }
        ) && self.cfg.refinement;
        if want_refine {
            let gate = self.cfg.refinement_gate;
            let mut promote_always = Vec::new();
            let mut promote_if_frequent: Vec<(u32, Bitmap, Vec<usize>)> = Vec::new();
            for (idx, (&sym_idx, component)) in
                instance_symbol_raw.iter().zip(comps.iter()).enumerate()
            {
                let dict_sym = &symbols_raw[sym_idx as usize];
                if component.bitmap == *dict_sym {
                    continue;
                }
                let stats = xor_stats(&component.bitmap, dict_sym);
                let area = (component.bitmap.width() as u64)
                    .saturating_mul(component.bitmap.height() as u64)
                    .max(1) as f32;
                let xor_fraction = stats.popcount as f32 / area;
                if stats.popcount < gate.min_xor_popcount {
                    promote_always.push(idx);
                } else if xor_fraction > gate.max_xor_fraction {
                    if let Some((_, _, indices)) =
                        promote_if_frequent
                            .iter_mut()
                            .find(|(group_sym, bitmap, _)| {
                                *group_sym == sym_idx && *bitmap == component.bitmap
                            })
                    {
                        indices.push(idx);
                    } else {
                        promote_if_frequent.push((sym_idx, component.bitmap.clone(), vec![idx]));
                    }
                }
            }

            for idx in promote_always {
                let new_idx = symbols_raw.len() as u32;
                symbols_raw.push(comps[idx].bitmap.clone());
                instance_symbol_raw[idx] = new_idx;
            }
            for (_, bitmap, indices) in promote_if_frequent {
                if (indices.len() as u32) < gate.promote_min_freq {
                    continue;
                }
                let new_idx = symbols_raw.len() as u32;
                symbols_raw.push(bitmap);
                for idx in indices {
                    instance_symbol_raw[idx] = new_idx;
                }
            }
        }

        // Step 3: sort symbols by height (ascending) so height-class
        // delta-coding is well-behaved. We keep a permutation map so each
        // instance still points at the right symbol.
        let n = symbols_raw.len();
        let mut order: Vec<u32> = (0..n as u32).collect();
        order.sort_by_key(|&i| symbols_raw[i as usize].height());
        let mut inv = vec![0u32; n];
        for (new_idx, &old_idx) in order.iter().enumerate() {
            inv[old_idx as usize] = new_idx as u32;
        }
        let sorted_syms: Vec<Bitmap> = order
            .iter()
            .map(|&i| symbols_raw[i as usize].clone())
            .collect();

        // Step 3: emit the symbol dictionary segment.
        let (template_id, _ext_template) = self.cfg.template.as_bits();
        // Symbol dict only allows templates 0..3 (no ext template).
        let sym_template = if template_id > 3 { 0 } else { template_id };
        let sd_hdr = SymbolDictionaryHeader::default_arithmetic(
            sym_template,
            sorted_syms.len() as u32,
            sorted_syms.len() as u32,
        );
        let sd_body =
            encode_symbol_dictionary_with_contexts(&sd_hdr, &sorted_syms, 0, &mut self.mq_cxs)?;

        let sd_seg_no = self.seg_no;
        let mut sd_hdr_bytes = Vec::new();
        sd_hdr.write(&mut sd_hdr_bytes)?;
        self.emit_segment(
            SegmentType::SymbolDictionary,
            0, // global — available to any page
            vec![],
            vec![false],
            |w| {
                w.write_all(&sd_hdr_bytes)?;
                w.write_all(&sd_body)?;
                Ok(())
            },
        )?;

        // Step 4: build the text region instances in reading order (sorted
        // by (y, x) so strips emit cleanly). When `refine_after_match` is
        // set we emit each component whose original bitmap differs from the
        // (possibly lossy-merged) dictionary representative as a refinement
        // against that representative — that recovers exact-pixel fidelity
        // even when the lossy classifier coalesced glyphs whose shapes
        // disagree by a handful of pixels.
        let mut instances: Vec<SymbolInstance> = instance_symbol_raw
            .iter()
            .zip(comps.iter())
            .map(|(&sym_idx, c)| {
                let dict_idx = inv[sym_idx as usize];
                debug_assert!(
                    (dict_idx as usize) < sorted_syms.len(),
                    "text region instance references dictionary index outside sorted symbol table"
                );
                let mut ins = SymbolInstance::placement(dict_idx, c.x as i32, c.y as i32);
                if want_refine {
                    let dict_sym = &sorted_syms[dict_idx as usize];
                    if c.bitmap != *dict_sym {
                        ins.refinement = Some(RefinedInstance {
                            rdw: c.bitmap.width() as i32 - dict_sym.width() as i32,
                            rdh: c.bitmap.height() as i32 - dict_sym.height() as i32,
                            rdx: 0,
                            rdy: 0,
                            target: c.bitmap.clone(),
                        });
                    }
                }
                ins
            })
            .collect();
        instances.sort_by_key(|i| (i.y, i.x));
        let any_refined = instances.iter().any(|i| i.refinement.is_some());

        // Choose a strip height of 2 rows unless the page is very short.
        let log_sbstrips: u8 = 0;

        let emit_text_region = |this: &mut Self,
                                region_y: u32,
                                region_height: u32,
                                region_instances: &[SymbolInstance]|
         -> Jbig2Result<()> {
            let region_refined = region_instances.iter().any(|ins| ins.refinement.is_some());
            let tr_hdr = TextRegionHeader {
                region: RegionInfo {
                    width: bitmap.width(),
                    height: region_height,
                    x: 0,
                    y: region_y,
                    external_combination_op: CombinationOp::Or,
                    colour_extension: false,
                },
                sbhuff: false,
                sbrefine: region_refined,
                log_sbstrips,
                ref_corner: RefCorner::TL,
                transposed: false,
                sbcombop: CombinationOp::Or,
                default_pixel: 0,
                sbds_offset: 0,
                sbr_template: false,
                sbhuff_fs: 0,
                sbhuff_ds: 0,
                sbhuff_dt: 0,
                sbhuff_rdw: 0,
                sbhuff_rdh: 0,
                sbhuff_rdx: 0,
                sbhuff_rdy: 0,
                sbhuff_rsize: false,
                rat: NOMINAL_REFINEMENT_AT.to_array_2(),
                num_instances: region_instances.len() as u32,
            };
            let tr_body = encode_text_region_with_contexts(
                &tr_hdr,
                region_instances,
                &sorted_syms,
                &mut this.mq_cxs,
            )?;
            let mut tr_hdr_bytes = Vec::new();
            tr_hdr.write(&mut tr_hdr_bytes)?;
            this.emit_segment(
                SegmentType::ImmediateLosslessTextRegion,
                page_assoc,
                vec![sd_seg_no],
                vec![false, false],
                |w| {
                    w.write_all(&tr_hdr_bytes)?;
                    w.write_all(&tr_body)?;
                    Ok(())
                },
            )
        };

        if any_refined {
            let mut start = 0usize;
            while start < instances.len() {
                let baseline_y = instances[start].y;
                let mut end = start + 1;
                while end < instances.len() && instances[end].y == baseline_y {
                    end += 1;
                }
                let mut local_instances = instances[start..end].to_vec();
                let mut region_height = 0u32;
                for ins in &mut local_instances {
                    ins.y -= baseline_y;
                    let ref_sym = &sorted_syms[ins.id as usize];
                    let h = match &ins.refinement {
                        Some(r) => r.target.height(),
                        None => ref_sym.height(),
                    };
                    region_height = region_height.max(ins.y.max(0) as u32 + h);
                }
                emit_text_region(
                    self,
                    baseline_y as u32,
                    region_height.max(1),
                    &local_instances,
                )?;
                start = end;
            }
        } else {
            emit_text_region(self, 0, bitmap.height(), &instances)?;
        }
        Ok(())
    }

    /// Finalise the stream and return the underlying writer.
    pub fn finish(mut self) -> Jbig2Result<W> {
        self.ensure_file_header()?;
        if self.pages_emitted < self.total_pages {
            return Err(Jbig2Error::InvalidConfig(
                "fewer pages emitted than declared",
            ));
        }
        if self.pages_emitted > self.total_pages {
            return Err(Jbig2Error::InvalidConfig(
                "more pages emitted than declared",
            ));
        }
        if self.stream_finalized {
            return Ok(self.writer);
        }
        // EOF segment so random-access-compatible readers terminate cleanly.
        let eof_seg = SegmentHeader {
            number: self.seg_no,
            segment_type: SegmentType::EndOfFile,
            deferred_non_retain: false,
            referred: vec![],
            retain_bits: vec![false],
            page_association: 0,
            data_length: Some(0),
        };
        eof_seg.write(&mut self.writer)?;
        Ok(self.writer)
    }

    fn emit_segment<F>(
        &mut self,
        kind: SegmentType,
        page_assoc: u32,
        referred: Vec<u32>,
        retain_bits: Vec<bool>,
        body: F,
    ) -> Jbig2Result<()>
    where
        F: FnOnce(&mut Vec<u8>) -> Jbig2Result<()>,
    {
        self.segment_buf.clear();
        body(&mut self.segment_buf)?;
        let seg_hdr = SegmentHeader {
            number: self.seg_no,
            segment_type: kind,
            deferred_non_retain: false,
            referred,
            retain_bits,
            page_association: page_assoc,
            data_length: Some(self.segment_buf.len() as u32),
        };
        self.seg_no += 1;
        seg_hdr.write(&mut self.writer)?;
        self.writer
            .write_all(&self.segment_buf)
            .map_err(Jbig2Error::from)?;
        Ok(())
    }

    fn validate_non_generic_template(&self, use_case: &'static str) -> Jbig2Result<()> {
        if matches!(self.cfg.template, GenericTemplate::Extended) {
            return Err(Jbig2Error::InvalidConfig(match use_case {
                "symbol" => "Extended template requires Mode::Generic",
                "halftone" => "Extended template is not supported for halftone encoding",
                _ => "Extended template is not supported for this encoder path",
            }));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Jbig2Decoder;
    use std::io::Cursor;

    fn stripe_bitmap(w: u32, h: u32) -> Bitmap {
        let mut bm = Bitmap::new(w, h).unwrap();
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                if (x + y) % 11 == 0 {
                    bm.set_pixel(x, y, 1);
                }
            }
        }
        bm
    }

    #[test]
    fn fast_preset_round_trip() {
        let bm = stripe_bitmap(64, 24);
        let mut buf = Vec::new();
        let mut enc = Jbig2Encoder::new(&mut buf, EncoderConfig::fast());
        enc.write_page(&bm).unwrap();
        enc.finish().unwrap();

        let mut dec = Jbig2Decoder::new(Cursor::new(buf)).unwrap();
        let page = dec.decode_page(1).unwrap();
        assert_eq!(page.bitmap, bm);
    }

    #[test]
    fn balanced_preset_round_trip() {
        let bm = stripe_bitmap(72, 32);
        let mut buf = Vec::new();
        let mut enc = Jbig2Encoder::new(&mut buf, EncoderConfig::balanced());
        enc.write_page(&bm).unwrap();
        enc.finish().unwrap();

        let mut dec = Jbig2Decoder::new(Cursor::new(buf)).unwrap();
        let page = dec.decode_page(1).unwrap();
        assert_eq!(page.bitmap, bm);
    }

    #[test]
    fn rate_selected_balanced_chooses_generic_when_smaller() {
        let bm = stripe_bitmap(96, 96);
        let rate_cfg = EncoderConfig::balanced();

        let mut generic_cfg = rate_cfg.clone();
        generic_cfg.mode = Mode::Generic;
        generic_cfg.template = GenericTemplate::T0;
        generic_cfg.generic_region_duplicate_line_removal = true;
        generic_cfg.rate_select = false;

        let mut symbol_cfg = rate_cfg.clone();
        symbol_cfg.rate_select = false;

        let generic = Jbig2Encoder::<Vec<u8>>::encode_page_to_vec(&bm, generic_cfg).unwrap();
        let symbol = Jbig2Encoder::<Vec<u8>>::encode_page_to_vec(&bm, symbol_cfg).unwrap();
        assert!(
            generic.len() < symbol.len(),
            "fixture should make generic smaller than symbol (generic={}, symbol={})",
            generic.len(),
            symbol.len()
        );

        let mut selected = Vec::new();
        let mut enc = Jbig2Encoder::new(&mut selected, rate_cfg);
        enc.write_page(&bm).unwrap();
        enc.finish().unwrap();
        assert_eq!(selected.len(), generic.len());

        let mut dec = Jbig2Decoder::new(Cursor::new(selected)).unwrap();
        let page = dec.decode_page(1).unwrap();
        assert_eq!(page.bitmap, bm);
    }

    #[test]
    fn max_compression_preset_round_trip() {
        let bm = stripe_bitmap(48, 32);
        let mut buf = Vec::new();
        let mut enc = Jbig2Encoder::new(&mut buf, EncoderConfig::max_compression());
        enc.write_page(&bm).unwrap();
        enc.finish().unwrap();

        let mut dec = Jbig2Decoder::new(Cursor::new(buf)).unwrap();
        let page = dec.decode_page(1).unwrap();
        assert_eq!(page.bitmap, bm);
    }

    #[test]
    fn max_compression_preset_recovers_lossy_variants() {
        let bm = glyph_page_with_lossy_variants();
        let mut buf = Vec::new();
        let mut enc = Jbig2Encoder::new(&mut buf, EncoderConfig::max_compression());
        enc.write_page(&bm).unwrap();
        enc.finish().unwrap();

        let mut dec = Jbig2Decoder::new(Cursor::new(buf.clone())).unwrap();
        let page = dec.decode_page(1).unwrap();
        assert_eq!(page.bitmap, bm);
    }

    #[test]
    fn rate_selected_max_chooses_generic_when_smaller() {
        let bm = stripe_bitmap(96, 96);
        let mut rate_cfg = EncoderConfig::max_compression();
        rate_cfg.refinement_gate.min_dedup_ratio = 1.0;

        let mut generic_cfg = rate_cfg.clone();
        generic_cfg.mode = Mode::Generic;
        generic_cfg.template = GenericTemplate::T0;
        generic_cfg.refinement = false;
        generic_cfg.refine_after_match = false;
        generic_cfg.generic_region_duplicate_line_removal = true;
        generic_cfg.rate_select = false;

        let mut symbol_cfg = rate_cfg.clone();
        symbol_cfg.rate_select = false;

        let generic = Jbig2Encoder::<Vec<u8>>::encode_page_to_vec(&bm, generic_cfg).unwrap();
        let symbol = Jbig2Encoder::<Vec<u8>>::encode_page_to_vec(&bm, symbol_cfg).unwrap();
        assert!(
            generic.len() < symbol.len(),
            "fixture should make generic smaller than symbol (generic={}, symbol={})",
            generic.len(),
            symbol.len()
        );

        let mut selected = Vec::new();
        let mut enc = Jbig2Encoder::new(&mut selected, rate_cfg);
        enc.write_page(&bm).unwrap();
        enc.finish().unwrap();
        assert_eq!(selected.len(), generic.len());

        let mut dec = Jbig2Decoder::new(Cursor::new(selected)).unwrap();
        let page = dec.decode_page(1).unwrap();
        assert_eq!(page.bitmap, bm);
    }

    /// Paint a few "glyph" rectangles so the identity classifier actually has
    /// duplicates to coalesce.
    fn glyph_page() -> Bitmap {
        let mut bm = Bitmap::new(200, 40).unwrap();
        // Three distinct glyph shapes.
        let shapes: [&[(i32, i32)]; 3] = [
            &[
                (0, 0),
                (0, 1),
                (1, 0),
                (1, 1),
                (2, 0),
                (2, 1),
                (0, 2),
                (1, 2),
                (2, 2),
            ],
            &[(0, 0), (1, 0), (2, 0), (1, 1), (1, 2), (1, 3)],
            &[
                (0, 0),
                (0, 1),
                (0, 2),
                (1, 0),
                (1, 2),
                (2, 0),
                (2, 1),
                (2, 2),
            ],
        ];
        // Lay the three shapes out ten times with 15-pixel spacing, two rows.
        for row in 0..2 {
            for col in 0..10 {
                let shape = &shapes[col % 3];
                let x0 = 5 + (col as i32) * 18;
                let y0 = 5 + (row as i32) * 18;
                for &(dx, dy) in shape.iter() {
                    bm.set_pixel(x0 + dx, y0 + dy, 1);
                }
            }
        }
        bm
    }

    /// Build a page where the lossy classifier should bucket two
    /// almost-identical glyph variants together. Returns the page
    /// bitmap; running the symbol-lossless encoder with `symbol_threshold
    /// = 0.85` and `refine_after_match = true` should pick one variant as
    /// the dictionary representative and emit the other instances as
    /// refinements against it, recovering the original page exactly.
    fn glyph_page_with_lossy_variants() -> Bitmap {
        let mut bm = Bitmap::new(200, 40).unwrap();
        // Two near-identical 3x3 glyph variants. `glyph_b` differs by one
        // extra center pixel only, so the WXOR disagreement is 1/8 = 0.125
        // and the `0.85` preset threshold will merge them.
        let glyph_a: &[(i32, i32)] = &[
            (0, 0),
            (1, 0),
            (2, 0),
            (0, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
        ];
        let glyph_b: &[(i32, i32)] = &[
            (0, 0),
            (1, 0),
            (2, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
        ];
        for row in 0..2 {
            for col in 0..10 {
                let shape = if col % 2 == 0 { glyph_a } else { glyph_b };
                let x0 = 5 + (col as i32) * 18;
                let y0 = 5 + (row as i32) * 18;
                for &(dx, dy) in shape.iter() {
                    bm.set_pixel(x0 + dx, y0 + dy, 1);
                }
            }
        }
        bm
    }

    #[test]
    fn refine_after_match_round_trip_recovers_lossy_variants() {
        let bm = glyph_page_with_lossy_variants();
        let cfg = EncoderConfig {
            mode: Mode::SymbolLossy,
            template: GenericTemplate::T0,
            coding: Coding::Arithmetic,
            adaptive_templates: None,
            refinement: true,
            generic_region_duplicate_line_removal: false,
            symbol_threshold: 0.85,
            refine_after_match: true,
            refinement_gate: RefinementGate::default(),
            rate_select: false,
        };
        let mut buf = Vec::new();
        let mut enc = Jbig2Encoder::new(&mut buf, cfg);
        enc.write_page(&bm).unwrap();
        enc.finish().unwrap();

        // The encoder should have exercised the SBREFINE = 1 path because
        // the two glyph variants get bucketed onto a single representative.
        let mut dec = Jbig2Decoder::new(Cursor::new(buf.clone())).unwrap();
        let mut num_sd = 0;
        let mut num_tr = 0;
        for h in dec.segment_headers() {
            match h.segment_type {
                SegmentType::SymbolDictionary => num_sd += 1,
                SegmentType::ImmediateLosslessTextRegion
                | SegmentType::ImmediateTextRegion
                | SegmentType::IntermediateTextRegion => num_tr += 1,
                _ => {}
            }
        }
        assert_eq!(num_sd, 1, "expected one symbol dictionary segment");
        assert!(num_tr >= 1, "expected at least one text region segment");

        let page = dec.decode_page(1).unwrap();
        assert_eq!(
            page.bitmap, bm,
            "refinement-on encoder should recover the original page bit-for-bit"
        );
    }

    #[test]
    fn symbol_lossless_round_trip_with_dedup() {
        let bm = glyph_page();
        let cfg = EncoderConfig {
            mode: Mode::SymbolLossless,
            template: GenericTemplate::T0,
            coding: Coding::Arithmetic,
            adaptive_templates: None,
            refinement: false,
            generic_region_duplicate_line_removal: false,
            symbol_threshold: 0.97,
            refine_after_match: false,
            refinement_gate: RefinementGate::default(),
            rate_select: false,
        };
        let mut buf = Vec::new();
        let mut enc = Jbig2Encoder::new(&mut buf, cfg);
        enc.write_page(&bm).unwrap();
        enc.finish().unwrap();

        // The file carries one symbol dictionary and one text region.
        let mut dec = Jbig2Decoder::new(Cursor::new(buf.clone())).unwrap();
        let mut num_sd = 0;
        let mut num_tr = 0;
        for h in dec.segment_headers() {
            match h.segment_type {
                SegmentType::SymbolDictionary => num_sd += 1,
                SegmentType::ImmediateLosslessTextRegion
                | SegmentType::ImmediateTextRegion
                | SegmentType::IntermediateTextRegion => num_tr += 1,
                _ => {}
            }
        }
        assert_eq!(num_sd, 1, "expected one symbol dictionary segment");
        assert_eq!(num_tr, 1, "expected one text region segment");

        let page = dec.decode_page(1).unwrap();
        assert_eq!(page.bitmap, bm);
    }

    #[test]
    fn explicit_symbol_coding_entry_point_round_trip() {
        let bm = glyph_page_with_lossy_variants();
        let cfg = EncoderConfig {
            refinement: true,
            ..EncoderConfig::balanced()
        };
        let mut buf = Vec::new();
        let mut enc = Jbig2Encoder::new(&mut buf, cfg);
        enc.write_page_symbols(
            &bm,
            SymbolCoding::Lossy {
                threshold: 0.85,
                refine_after_match: true,
            },
        )
        .unwrap();
        enc.finish().unwrap();

        let mut dec = Jbig2Decoder::new(Cursor::new(buf)).unwrap();
        let page = dec.decode_page(1).unwrap();
        assert_eq!(page.bitmap, bm);
    }

    #[test]
    fn halftone_entry_point_round_trip() {
        let mut p0 = Bitmap::new(3, 3).unwrap();
        p0.set_pixel(1, 0, 1);
        p0.set_pixel(0, 1, 1);
        p0.set_pixel(1, 1, 1);
        p0.set_pixel(2, 1, 1);
        p0.set_pixel(1, 2, 1);

        let mut p1 = Bitmap::new(3, 3).unwrap();
        for i in 0..3 {
            p1.set_pixel(i, i, 1);
        }

        let patterns = vec![p0.clone(), p1.clone()];
        let gray = [0u32, 1, 1, 0];
        let opts = HalftonePageOptions {
            page_width: 10,
            page_height: 10,
            region_x: 0,
            region_y: 0,
            region_width: 10,
            region_height: 10,
            enable_skip: false,
            comb_op: CombinationOp::Or,
            default_pixel: 0,
            grid_width: 2,
            grid_height: 2,
            grid_x: 0,
            grid_y: 0,
            step_x: 4 << 8,
            step_y: 4 << 8,
        };
        let mut buf = Vec::new();
        let mut enc = Jbig2Encoder::new(&mut buf, EncoderConfig::balanced());
        enc.write_page_halftone(&patterns, &gray, opts).unwrap();
        enc.finish().unwrap();

        let mut expect = Bitmap::new(10, 10).unwrap();
        expect.composite(&p0, 0, 0, crate::bitmap::BlitOp::Or);
        expect.composite(&p1, 4, 4, crate::bitmap::BlitOp::Or);
        expect.composite(&p1, 4, -4, crate::bitmap::BlitOp::Or);
        expect.composite(&p0, 8, 0, crate::bitmap::BlitOp::Or);

        let mut dec = Jbig2Decoder::new(Cursor::new(buf)).unwrap();
        let page = dec.decode_page(1).unwrap();
        assert_eq!(page.bitmap, expect);
    }
}
