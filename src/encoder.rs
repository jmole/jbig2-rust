//! High-level JBIG2 encoder (single-page, generic-region-only in v1).
//!
//! The encoder emits a minimal sequential-organization JBIG2 file with one
//! page information segment, one generic-region segment (arithmetic, MMR, or
//! Huffman-free generic), and one end-of-page segment. Symbol-dictionary and
//! text-region emission will be added in M3; refinement in M4.
//!
//! See [`EncoderConfig`] for the tunables surfaced to callers.

use std::io::Write;

use crate::bitmap::Bitmap;
use crate::coding::mq::{MqContexts, MqEncoder, MQ_NUM_CONTEXTS};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::{
    file_header::FileHeader,
    generic_region::{encode_generic_arith, encode_generic_mmr, nominal_at, GenericRegionHeader},
    page_information::{CombinationOp, PageInformation},
    region_info::RegionInfo,
    symbol_dictionary::{encode_symbol_dictionary, SymbolDictionaryHeader},
    text_region::{encode_text_region, RefCorner, SymbolInstance, TextRegionHeader},
    SegmentHeader, SegmentType,
};
use crate::symbol::{
    cc::extract_components, classify::classify_lossy, identity::classify_identity,
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
    /// Extract exact-match symbols, encode a symbol dictionary + text region
    /// (M3 — currently falls back to [`Mode::Generic`]).
    SymbolLossless,
    /// Lossy classifier + dictionary refinement (M7 — currently falls back
    /// to [`Mode::Generic`]).
    SymbolLossy,
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
    pub adaptive_templates: Option<[(i8, i8); 12]>,
    /// Emit refinement regions where available.
    pub refinement: bool,
    /// Enable typical-prediction duplicate-line removal (TPGD).
    pub duplicate_line_removal: bool,
    /// Lossy match threshold (used only in [`Mode::SymbolLossy`]).
    pub symbol_threshold: f32,
    /// Run a post-match refinement pass on matched symbols (lossy path).
    pub refine_after_match: bool,
    /// Optional page association override for multi-page encoding.
    pub multi_page: bool,
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
            duplicate_line_removal: false,
            symbol_threshold: 0.97,
            refine_after_match: false,
            multi_page: false,
        }
    }

    /// Balanced preset: Template 0, arithmetic, symbol-lossless mode, TPGD.
    pub fn balanced() -> Self {
        Self {
            mode: Mode::SymbolLossless,
            template: GenericTemplate::T0,
            coding: Coding::Arithmetic,
            adaptive_templates: None,
            refinement: false,
            duplicate_line_removal: true,
            symbol_threshold: 0.97,
            refine_after_match: false,
            multi_page: false,
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
            duplicate_line_removal: true,
            symbol_threshold: 0.85,
            refine_after_match: true,
            multi_page: false,
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
    seg_no: u32,
    file_header_emitted: bool,
    pages_emitted: u32,
    total_pages: u32,
}

impl<W: Write> Jbig2Encoder<W> {
    /// Create an encoder for a single-page file.
    pub fn new(writer: W, cfg: EncoderConfig) -> Self {
        Self {
            writer,
            cfg,
            seg_no: 0,
            file_header_emitted: false,
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
        self.ensure_file_header()?;
        self.pages_emitted += 1;
        let page_assoc = self.pages_emitted;

        let page_info = PageInformation {
            width: bitmap.width(),
            height: bitmap.height(),
            x_resolution: 0,
            y_resolution: 0,
            is_lossless: !matches!(self.cfg.mode, Mode::SymbolLossy),
            may_contain_refinements: self.cfg.refinement,
            default_pixel: 0,
            default_combination_op: CombinationOp::Or,
            requires_aux_buffers: false,
            combination_op_override: false,
            may_contain_colour: false,
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

        match self.cfg.mode {
            Mode::SymbolLossless => self.encode_page_symbol(bitmap, page_assoc, 1.0)?,
            Mode::SymbolLossy => {
                self.encode_page_symbol(bitmap, page_assoc, self.cfg.symbol_threshold)?
            }
            Mode::Generic => self.encode_page_generic(bitmap, page_assoc)?,
        }

        self.emit_segment(
            SegmentType::EndOfPage,
            page_assoc,
            vec![],
            vec![false],
            |_w| Ok(()),
        )?;
        Ok(())
    }

    fn encode_page_generic(&mut self, bitmap: &Bitmap, page_assoc: u32) -> Jbig2Result<()> {
        let (template_id, ext_template) = self.cfg.template.as_bits();
        let at = self
            .cfg
            .adaptive_templates
            .unwrap_or_else(|| nominal_at(template_id, ext_template));
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
            tpgdon: self.cfg.duplicate_line_removal,
            ext_template,
            at,
        };

        let mut hdr_bytes = Vec::new();
        hdr.write(&mut hdr_bytes)?;

        let coded = match self.cfg.coding {
            Coding::Arithmetic => {
                let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
                let mut enc = MqEncoder::new(bitmap.data().len() / 4 + 16);
                encode_generic_arith(&mut enc, &mut cxs, &hdr, bitmap)?;
                enc.finish()
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
        )
    }

    fn encode_page_symbol(
        &mut self,
        bitmap: &Bitmap,
        page_assoc: u32,
        symbol_threshold: f32,
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
        let (symbols_raw, instance_symbol_raw) = if symbol_threshold < 1.0 {
            let cls = classify_lossy(&comps, symbol_threshold);
            (cls.symbols, cls.instance_symbol)
        } else {
            let cls = classify_identity(&comps);
            (cls.symbols, cls.instance_symbol)
        };

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
        let sd_body = encode_symbol_dictionary(&sd_hdr, &sorted_syms, 0)?;

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
        // by (y, x) so strips emit cleanly).
        let mut instances: Vec<SymbolInstance> = instance_symbol_raw
            .iter()
            .zip(comps.iter())
            .map(|(&sym_idx, c)| SymbolInstance {
                id: inv[sym_idx as usize],
                x: c.x as i32,
                y: c.y as i32,
            })
            .collect();
        instances.sort_by_key(|i| (i.y, i.x));

        // Choose a strip height of 2 rows unless the page is very short.
        let log_sbstrips: u8 = if bitmap.height() < 4 { 0 } else { 1 };

        let tr_hdr = TextRegionHeader {
            region: RegionInfo {
                width: bitmap.width(),
                height: bitmap.height(),
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            sbhuff: false,
            sbrefine: false,
            log_sbstrips,
            ref_corner: RefCorner::TL,
            transposed: false,
            sbcombop: CombinationOp::Or,
            default_pixel: 0,
            sbds_offset: 0,
            sbr_template: false,
            rat: [(0, 0); 2],
            num_instances: instances.len() as u32,
        };
        let tr_body = encode_text_region(&tr_hdr, &instances, &sorted_syms)?;
        let mut tr_hdr_bytes = Vec::new();
        tr_hdr.write(&mut tr_hdr_bytes)?;
        self.emit_segment(
            SegmentType::ImmediateLosslessTextRegion,
            page_assoc,
            vec![sd_seg_no],
            vec![false, false],
            |w| {
                w.write_all(&tr_hdr_bytes)?;
                w.write_all(&tr_body)?;
                Ok(())
            },
        )?;
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
        let mut body_buf = Vec::new();
        body(&mut body_buf)?;
        let seg_hdr = SegmentHeader {
            number: self.seg_no,
            segment_type: kind,
            deferred_non_retain: false,
            referred,
            retain_bits,
            page_association: page_assoc,
            data_length: Some(body_buf.len() as u32),
        };
        self.seg_no += 1;
        seg_hdr.write(&mut self.writer)?;
        self.writer
            .write_all(&body_buf)
            .map_err(Jbig2Error::from)?;
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

    /// Paint a few "glyph" rectangles so the identity classifier actually has
    /// duplicates to coalesce.
    fn glyph_page() -> Bitmap {
        let mut bm = Bitmap::new(200, 40).unwrap();
        // Three distinct glyph shapes.
        let shapes: [&[(i32, i32)]; 3] = [
            &[(0, 0), (0, 1), (1, 0), (1, 1), (2, 0), (2, 1), (0, 2), (1, 2), (2, 2)],
            &[(0, 0), (1, 0), (2, 0), (1, 1), (1, 2), (1, 3)],
            &[(0, 0), (0, 1), (0, 2), (1, 0), (1, 2), (2, 0), (2, 1), (2, 2)],
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

    #[test]
    fn symbol_lossless_round_trip_with_dedup() {
        let bm = glyph_page();
        let cfg = EncoderConfig {
            mode: Mode::SymbolLossless,
            template: GenericTemplate::T0,
            coding: Coding::Arithmetic,
            adaptive_templates: None,
            refinement: false,
            duplicate_line_removal: false,
            symbol_threshold: 0.97,
            refine_after_match: false,
            multi_page: false,
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
}
