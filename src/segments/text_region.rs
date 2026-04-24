//! Text region segment (spec 7.4.3 / 6.4).
//!
//! The v1 implementation covers the arithmetic-coded, no-refinement path
//! (`SBHUFF = 0`, `SBREFINE = 0`). This handles every text region produced
//! by this crate's lossless symbol classifier plus the common output of
//! `jbig2enc -S` on patent-style scans.

use std::io::{Read, Write};

use crate::bitmap::Bitmap;
use crate::coding::huffman::{standard_table, HuffReader};
use crate::coding::mq::{MqContexts, MqDecoder, MqEncoder, MQ_NUM_CONTEXTS};
use crate::coding::mq_context::{IADS, IADT, IAFS, IAID, IAIT, IARDH, IARDW, IARDX, IARDY, IARI};
use crate::coding::mq_integer::{decode_iaid, decode_integer, encode_iaid, encode_integer, OOB};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::rgb_bitmap::RgbBitmap;
use crate::segments::page_information::CombinationOp;
use crate::segments::refinement_region::{decode_refinement_region, encode_refinement_region};
use crate::segments::region_info::RegionInfo;

/// Reference corner of a symbol instance (spec 6.4.5 / 7.4.3.1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefCorner {
    /// Bottom-left.
    BL = 0,
    /// Top-left.
    TL = 1,
    /// Bottom-right.
    BR = 2,
    /// Top-right.
    TR = 3,
}

impl RefCorner {
    /// Convert from a 2-bit field.
    pub fn from_u8(v: u8) -> Self {
        match v & 0x3 {
            0 => Self::BL,
            1 => Self::TL,
            2 => Self::BR,
            _ => Self::TR,
        }
    }
}

/// Parsed text region segment header.
#[derive(Clone, Debug)]
pub struct TextRegionHeader {
    /// Region info (width, height, page coords, combination op).
    pub region: RegionInfo,
    /// `SBHUFF`.
    pub sbhuff: bool,
    /// `SBREFINE`.
    pub sbrefine: bool,
    /// `LOGSBSTRIPS` (0..3) — strip size is `1 << logsbstrips`.
    pub log_sbstrips: u8,
    /// `REFCORNER`.
    pub ref_corner: RefCorner,
    /// `TRANSPOSED`.
    pub transposed: bool,
    /// `SBCOMBOP` combination operator.
    pub sbcombop: CombinationOp,
    /// `SBDEFPIXEL` default pixel before any symbols are drawn.
    pub default_pixel: u8,
    /// `SBDSOFFSET` (signed, 5-bit).
    pub sbds_offset: i8,
    /// `SBRTEMPLATE` (refinement template).
    pub sbr_template: bool,
    /// `SBHUFFFS` selector.
    pub sbhuff_fs: u8,
    /// `SBHUFFDS` selector.
    pub sbhuff_ds: u8,
    /// `SBHUFFDT` selector.
    pub sbhuff_dt: u8,
    /// `SBHUFFRDW` selector.
    pub sbhuff_rdw: u8,
    /// `SBHUFFRDH` selector.
    pub sbhuff_rdh: u8,
    /// `SBHUFFRDX` selector.
    pub sbhuff_rdx: u8,
    /// `SBHUFFRDY` selector.
    pub sbhuff_rdy: u8,
    /// `SBHUFFRSIZE` selector.
    pub sbhuff_rsize: bool,
    /// Refinement AT pixels (only used when `sbrefine = true` and
    /// `sbr_template = false`).
    pub rat: [(i8, i8); 2],
    /// `SBNUMINSTANCES`.
    pub num_instances: u32,
}

impl TextRegionHeader {
    /// Parse the text region header (region info + flags + optional
    /// refinement AT + symbol instance count). For the arithmetic-only path
    /// there is no Huffman-flags sub-field.
    pub fn read<R: Read>(r: &mut R) -> Jbig2Result<Self> {
        let region = RegionInfo::read(r)?;
        let mut fb = [0u8; 2];
        r.read_exact(&mut fb)?;
        let flags = u16::from_be_bytes(fb);
        let sbhuff = (flags & 0x0001) != 0;
        let sbrefine = (flags & 0x0002) != 0;
        let log_sbstrips = ((flags >> 2) & 0x3) as u8;
        let ref_corner = RefCorner::from_u8(((flags >> 4) & 0x3) as u8);
        let transposed = ((flags >> 6) & 0x1) != 0;
        let sbcombop = CombinationOp::from_u8(((flags >> 7) & 0x3) as u8);
        let default_pixel = ((flags >> 9) & 0x1) as u8;
        let raw_dsoffset = ((flags >> 10) & 0x1F) as u8;
        let sbds_offset = if raw_dsoffset > 0x0F {
            (raw_dsoffset as i16 - 0x20) as i8
        } else {
            raw_dsoffset as i8
        };
        let sbr_template = ((flags >> 15) & 0x1) != 0;

        let mut sbhuff_fs = 0;
        let mut sbhuff_ds = 0;
        let mut sbhuff_dt = 0;
        let mut sbhuff_rdw = 0;
        let mut sbhuff_rdh = 0;
        let mut sbhuff_rdx = 0;
        let mut sbhuff_rdy = 0;
        let mut sbhuff_rsize = false;
        if sbhuff {
            let mut hb = [0u8; 2];
            r.read_exact(&mut hb)?;
            let huff_flags = u16::from_be_bytes(hb);
            sbhuff_fs = (huff_flags & 0x3) as u8;
            sbhuff_ds = ((huff_flags >> 2) & 0x3) as u8;
            sbhuff_dt = ((huff_flags >> 4) & 0x3) as u8;
            sbhuff_rdw = ((huff_flags >> 6) & 0x3) as u8;
            sbhuff_rdh = ((huff_flags >> 8) & 0x3) as u8;
            sbhuff_rdx = ((huff_flags >> 10) & 0x3) as u8;
            sbhuff_rdy = ((huff_flags >> 12) & 0x3) as u8;
            sbhuff_rsize = ((huff_flags >> 14) & 0x1) != 0;
        }

        let mut rat = [(0i8, 0i8); 2];
        if sbrefine && !sbr_template {
            for i in 0..2 {
                let mut b = [0u8; 2];
                r.read_exact(&mut b)?;
                rat[i] = (b[0] as i8, b[1] as i8);
            }
        }

        let mut nb = [0u8; 4];
        r.read_exact(&mut nb)?;
        let num_instances = u32::from_be_bytes(nb);

        Ok(Self {
            region,
            sbhuff,
            sbrefine,
            log_sbstrips,
            ref_corner,
            transposed,
            sbcombop,
            default_pixel,
            sbds_offset,
            sbr_template,
            sbhuff_fs,
            sbhuff_ds,
            sbhuff_dt,
            sbhuff_rdw,
            sbhuff_rdh,
            sbhuff_rdx,
            sbhuff_rdy,
            sbhuff_rsize,
            rat,
            num_instances,
        })
    }

    /// Emit the text region header.
    pub fn write<W: Write>(&self, w: &mut W) -> Jbig2Result<()> {
        self.region.write(w)?;
        let raw_dsoffset = if self.sbds_offset < 0 {
            (self.sbds_offset as i16 + 0x20) as u16
        } else {
            self.sbds_offset as u16
        };
        let flags: u16 = (self.sbhuff as u16)
            | ((self.sbrefine as u16) << 1)
            | ((self.log_sbstrips as u16 & 0x3) << 2)
            | ((self.ref_corner as u16 & 0x3) << 4)
            | ((self.transposed as u16) << 6)
            | ((combop_to_u8(self.sbcombop) as u16 & 0x3) << 7)
            | ((self.default_pixel as u16 & 0x1) << 9)
            | ((raw_dsoffset & 0x1F) << 10)
            | ((self.sbr_template as u16) << 15);
        w.write_all(&flags.to_be_bytes())?;
        if self.sbhuff {
            let huff_flags: u16 = (self.sbhuff_fs as u16 & 0x3)
                | ((self.sbhuff_ds as u16 & 0x3) << 2)
                | ((self.sbhuff_dt as u16 & 0x3) << 4)
                | ((self.sbhuff_rdw as u16 & 0x3) << 6)
                | ((self.sbhuff_rdh as u16 & 0x3) << 8)
                | ((self.sbhuff_rdx as u16 & 0x3) << 10)
                | ((self.sbhuff_rdy as u16 & 0x3) << 12)
                | ((self.sbhuff_rsize as u16) << 14);
            w.write_all(&huff_flags.to_be_bytes())?;
        }
        if self.sbrefine && !self.sbr_template {
            for i in 0..2 {
                w.write_all(&[self.rat[i].0 as u8, self.rat[i].1 as u8])?;
            }
        }
        w.write_all(&self.num_instances.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
struct CanonicalCode {
    symbol: usize,
    code: u64,
    len: u8,
}

fn build_canonical_codes(lengths: &[u8]) -> Jbig2Result<Vec<CanonicalCode>> {
    let mut ordered: Vec<(usize, u8)> = lengths
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, len)| *len > 0)
        .collect();
    ordered.sort_by_key(|(idx, len)| (*len, *idx));
    let mut out = Vec::with_capacity(ordered.len());
    let mut last_len: u32 = 0;
    let mut code: u64 = 0;
    let mut first = true;
    for (symbol, len_u8) in ordered {
        let len = len_u8 as u32;
        if first {
            code = 0;
            first = false;
        } else {
            code += 1;
            if len > last_len {
                code <<= len - last_len;
            }
        }
        if code >= (1u64 << len) {
            return Err(Jbig2Error::InvalidHuffman(
                "text region: over-allocated canonical code table",
            ));
        }
        last_len = len;
        out.push(CanonicalCode {
            symbol,
            code,
            len: len_u8,
        });
    }
    Ok(out)
}

fn decode_canonical_symbol(r: &mut HuffReader<'_>, codes: &[CanonicalCode]) -> Jbig2Result<usize> {
    let mut bits = 0u64;
    for len in 1..=64 {
        bits = (bits << 1) | r.read_bit()? as u64;
        for code in codes {
            if code.len as u32 == len && code.code == bits {
                return Ok(code.symbol);
            }
        }
    }
    Err(Jbig2Error::InvalidHuffman(
        "text region: no matching canonical symbol code",
    ))
}

fn select_text_region_fs_table(sel: u8) -> Jbig2Result<crate::coding::huffman::HuffTable> {
    match sel {
        0 => standard_table(6),
        1 => standard_table(7),
        3 => Err(Jbig2Error::Unsupported(
            "text region: user-defined SBHUFFFS not yet implemented",
        )),
        _ => Err(Jbig2Error::Unsupported(
            "text region: unsupported SBHUFFFS selector",
        )),
    }
}

fn select_text_region_ds_table(sel: u8) -> Jbig2Result<crate::coding::huffman::HuffTable> {
    match sel {
        0 => standard_table(8),
        1 => standard_table(9),
        2 => standard_table(10),
        3 => Err(Jbig2Error::Unsupported(
            "text region: user-defined SBHUFFDS not yet implemented",
        )),
        _ => Err(Jbig2Error::Unsupported(
            "text region: unsupported SBHUFFDS selector",
        )),
    }
}

fn select_text_region_dt_table(sel: u8) -> Jbig2Result<crate::coding::huffman::HuffTable> {
    match sel {
        0 => standard_table(11),
        1 => standard_table(12),
        2 => standard_table(13),
        3 => Err(Jbig2Error::Unsupported(
            "text region: user-defined SBHUFFDT not yet implemented",
        )),
        _ => Err(Jbig2Error::Unsupported(
            "text region: unsupported SBHUFFDT selector",
        )),
    }
}

fn decode_text_region_symbol_id_table(
    r: &mut HuffReader<'_>,
    num_syms: usize,
) -> Jbig2Result<Vec<CanonicalCode>> {
    let mut run_code_lens = [0u8; 35];
    for i in 0..17 {
        let b = r.read_bits(8)? as u8;
        run_code_lens[i * 2] = b >> 4;
        run_code_lens[i * 2 + 1] = b & 0x0f;
    }
    run_code_lens[34] = r.read_bits(4)? as u8;
    let run_codes = build_canonical_codes(&run_code_lens)?;

    // The T.88 sample decoder does not reconstruct the intermediate
    // SBNUMSYMS code-length array from 7.4.3.1.7 literally. Instead it
    // expands each decoded run code directly into one concrete symbol-ID
    // canonical code using `(run_prefix << run_code) + occurrence_index`,
    // with total code length `run_prefix_len + run_code`. TT1 is authored
    // against that decoder, so we mirror the same construction.
    let mut by_symbol = vec![
        CanonicalCode {
            symbol: 0,
            code: 0,
            len: 0,
        };
        num_syms
    ];
    let mut run_counts = [0u64; 35];
    for (sym_idx, slot) in by_symbol.iter_mut().enumerate() {
        let run_idx = decode_canonical_symbol(r, &run_codes)?;
        let run = run_codes.iter().find(|code| code.symbol == run_idx).ok_or(
            Jbig2Error::InvalidHuffman("text region: decoded run code missing canonical entry"),
        )?;
        let code = (run.code << run_idx) | run_counts[run_idx];
        let len = run.len.saturating_add(run_idx as u8);
        run_counts[run_idx] += 1;
        *slot = CanonicalCode {
            symbol: sym_idx,
            code,
            len,
        };
    }
    r.byte_align();
    by_symbol.sort_by_key(|code| (code.len, code.symbol));
    Ok(by_symbol)
}

fn decode_text_region_huffman(
    header: &TextRegionHeader,
    body: &[u8],
    sbsyms: &[&Bitmap],
) -> Jbig2Result<Bitmap> {
    if sbsyms.is_empty() {
        return Err(Jbig2Error::OutOfRange(
            "text region: symbol library is empty",
        ));
    }
    if header.sbrefine {
        return Err(Jbig2Error::Unsupported(
            "text region: Huffman + refinement not yet implemented",
        ));
    }

    let fs_table = select_text_region_fs_table(header.sbhuff_fs)?;
    let ds_table = select_text_region_ds_table(header.sbhuff_ds)?;
    let dt_table = select_text_region_dt_table(header.sbhuff_dt)?;

    let mut region = Bitmap::filled(
        header.region.width,
        header.region.height,
        header.default_pixel,
    )?;
    let mut r = HuffReader::new(body);
    let symbol_codes = decode_text_region_symbol_id_table(&mut r, sbsyms.len())?;

    let sb_strips = 1u32 << header.log_sbstrips;
    let mut first_s = true;
    let init_dt = dt_table.decode(&mut r)?.ok_or(Jbig2Error::InvalidHuffman(
        "text region: initial SBHUFFDT returned OOB",
    ))?;
    let mut strip_t = -(init_dt as i64) * sb_strips as i64;

    let mut ninst = 0u32;
    while ninst < header.num_instances {
        let dt = dt_table.decode(&mut r)?.ok_or(Jbig2Error::InvalidHuffman(
            "text region: SBHUFFDT returned OOB",
        ))?;
        strip_t = strip_t.saturating_add((dt as i64).saturating_mul(sb_strips as i64));
        let mut cur_s = 0i64;
        loop {
            if first_s {
                let dfs = fs_table.decode(&mut r)?.ok_or(Jbig2Error::InvalidHuffman(
                    "text region: SBHUFFFS returned OOB",
                ))?;
                cur_s = dfs as i64;
                first_s = false;
            } else {
                let ids = ds_table.decode(&mut r)?;
                let ids = match ids {
                    Some(v) => v,
                    None => break,
                };
                cur_s = cur_s.saturating_add(ids as i64 + header.sbds_offset as i64);
            }

            // Match the T.88 sample decoder's Huffman-text quirk:
            // it ignores STRIPT here and uses `T + SBSTRIPS`.
            let t_bits = if header.log_sbstrips == 0 {
                0
            } else {
                r.read_bits(header.log_sbstrips as u32)? as i64
            };
            let cur_t = t_bits + sb_strips as i64;

            let id = decode_canonical_symbol(&mut r, &symbol_codes)?;
            if id >= sbsyms.len() {
                return Err(Jbig2Error::OutOfRange(
                    "text region: symbol ID out of range",
                ));
            }
            let ib = sbsyms[id];
            composite_instance(
                &mut region,
                ib,
                &mut cur_s,
                cur_t,
                header.transposed,
                header.ref_corner,
                header.sbcombop,
            );
            ninst += 1;
            if ninst >= header.num_instances {
                break;
            }
        }
    }

    Ok(region)
}

fn combop_to_u8(op: CombinationOp) -> u8 {
    match op {
        CombinationOp::Or => 0,
        CombinationOp::And => 1,
        CombinationOp::Xor => 2,
        CombinationOp::XNor => 3,
        CombinationOp::Replace => 0, // fallback; text regions never use Replace
    }
}

/// Placement of one symbol instance inside a text region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolInstance {
    /// Symbol identifier (index into `SBSYMS`).
    pub id: u32,
    /// X coordinate (top-left, un-transposed).
    pub x: i32,
    /// Y coordinate (top-left, un-transposed).
    pub y: i32,
    /// When `Some`, the encoder emits this instance as a refinement of
    /// `sbsyms[id]`. Decoders re-read the same fields when `SBREFINE = 1`
    /// is set on the surrounding region.
    pub refinement: Option<RefinedInstance>,
}

/// Refinement payload attached to a [`SymbolInstance`] when it is coded
/// against the referenced symbol rather than blitted verbatim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefinedInstance {
    /// `RDW`: extra columns added to the reference's width.
    pub rdw: i32,
    /// `RDH`: extra rows added to the reference's height.
    pub rdh: i32,
    /// `RDX`: refinement-region reference offset on the X axis.
    pub rdx: i32,
    /// `RDY`: refinement-region reference offset on the Y axis.
    pub rdy: i32,
    /// Refined target bitmap. Its dimensions must equal
    /// `(reference.width + rdw, reference.height + rdh)`.
    pub target: Bitmap,
}

impl SymbolInstance {
    /// Construct an unrefined instance (the common case).
    pub fn placement(id: u32, x: i32, y: i32) -> Self {
        Self {
            id,
            x,
            y,
            refinement: None,
        }
    }
}

/// Decode an arithmetic-coded text region body, compositing every symbol
/// instance into a fresh bitmap of size `(header.region.width,
/// header.region.height)`. `sbsyms` is the full symbol library as defined
/// in 6.4.5.
///
/// This is a convenience wrapper that allocates a fresh MQ context pool
/// for the duration of the call. Multi-page decoders should prefer
/// [`decode_text_region_with_contexts`] and hand in a caller-owned pool
/// so the allocation can be amortized across segments.
pub fn decode_text_region(
    header: &TextRegionHeader,
    body: &[u8],
    sbsyms: &[&Bitmap],
) -> Jbig2Result<Bitmap> {
    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    decode_text_region_with_contexts(header, body, sbsyms, &mut cxs)
}

/// As [`decode_text_region`] but reuses an externally-owned MQ context
/// pool. The pool is reset at entry (§7.4.3.1 requires a zero initial
/// state at the start of each text region), so callers are free to share
/// it with other arithmetic decode families between segments.
pub fn decode_text_region_with_contexts(
    header: &TextRegionHeader,
    body: &[u8],
    sbsyms: &[&Bitmap],
    cxs: &mut MqContexts,
) -> Jbig2Result<Bitmap> {
    if header.sbhuff {
        return decode_text_region_huffman(header, body, sbsyms);
    }
    if sbsyms.is_empty() {
        return Err(Jbig2Error::OutOfRange(
            "text region: symbol library is empty",
        ));
    }

    if cxs.len() < MQ_NUM_CONTEXTS {
        return Err(Jbig2Error::InvalidConfig(
            "text region: external MQ context pool is too small",
        ));
    }
    cxs.reset();
    let mut dec = MqDecoder::new(body);
    decode_text_region_body(&mut dec, cxs, header, sbsyms)
}

/// Decode a text-region body into a fresh bitmap using an externally
/// supplied MQ decoder. Unlike [`decode_text_region_with_contexts`], the
/// contexts are **not** reset on entry: callers that share the pool with
/// an enclosing symbol-dictionary aggregate decode (spec 6.5.8.2) must
/// keep the existing context state so that the nested text region
/// consumes bits from the same MQ stream.
///
/// `SBHUFF = 1` is handled by [`decode_text_region_with_contexts`] before
/// reaching this MQ-backed inner routine. `SBREFINE` may be 0 or 1 and is
/// routed through [`decode_refinement_region`] for the refined per-instance
/// bodies.
pub fn decode_text_region_body(
    dec: &mut MqDecoder<'_>,
    cxs: &mut MqContexts,
    header: &TextRegionHeader,
    sbsyms: &[&Bitmap],
) -> Jbig2Result<Bitmap> {
    let mut region = Bitmap::filled(
        header.region.width,
        header.region.height,
        header.default_pixel,
    )?;
    for_each_decoded_text_instance(dec, cxs, header, sbsyms, |ib, x0, y0, _| {
        region.composite(ib, x0, y0, header.sbcombop.into());
        Ok(())
    })?;
    Ok(region)
}

/// Decode an arithmetic-coded colour-extended text region into an RGB bitmap.
///
/// The `colours` slice supplies one RGB triplet per decoded symbol instance.
pub fn decode_text_region_colour_with_contexts(
    header: &TextRegionHeader,
    body: &[u8],
    sbsyms: &[&Bitmap],
    colours: &[[u8; 3]],
    cxs: &mut MqContexts,
) -> Jbig2Result<RgbBitmap> {
    if header.sbhuff {
        return Err(Jbig2Error::Unsupported(
            "text region: colour extension on Huffman path not yet implemented",
        ));
    }
    if !header.region.colour_extension {
        return Err(Jbig2Error::InvalidConfig(
            "text region: colour decode requested without COLEXTFLAG",
        ));
    }
    if sbsyms.is_empty() {
        return Err(Jbig2Error::OutOfRange(
            "text region: symbol library is empty",
        ));
    }
    if colours.len() != header.num_instances as usize {
        return Err(Jbig2Error::InvalidConfig(
            "text region: colour count does not match instance count",
        ));
    }
    if cxs.len() < MQ_NUM_CONTEXTS {
        return Err(Jbig2Error::InvalidConfig(
            "text region: external MQ context pool is too small",
        ));
    }

    cxs.reset();
    let mut dec = MqDecoder::new(body);
    let mut region = RgbBitmap::filled(header.region.width, header.region.height, [255, 255, 255])?;
    for_each_decoded_text_instance(&mut dec, cxs, header, sbsyms, |ib, x0, y0, ninst| {
        region.composite_mask_color(ib, x0, y0, colours[ninst as usize]);
        Ok(())
    })?;
    Ok(region)
}

fn for_each_decoded_text_instance<F>(
    dec: &mut MqDecoder<'_>,
    cxs: &mut MqContexts,
    header: &TextRegionHeader,
    sbsyms: &[&Bitmap],
    mut emit: F,
) -> Jbig2Result<()>
where
    F: FnMut(&Bitmap, i32, i32, u32) -> Jbig2Result<()>,
{
    if header.sbhuff {
        return Err(Jbig2Error::Unsupported(
            "text region: SBHUFF path does not use the MQ decoder entrypoint",
        ));
    }
    if sbsyms.is_empty() {
        return Err(Jbig2Error::OutOfRange(
            "text region: symbol library is empty",
        ));
    }

    let code_len = sym_code_len(sbsyms.len() as u32);
    let sb_strips = 1u32 << header.log_sbstrips;
    let s0 = decode_integer(dec, cxs, IADT).ok_or(Jbig2Error::InvalidHuffman(
        "text region: IADT initial returned OOB",
    ))?;
    let mut strip_t: i64 = -(s0 as i64) * sb_strips as i64;
    let mut first_s: i64 = 0;
    let mut ninst: u32 = 0;

    while ninst < header.num_instances {
        let dt = decode_integer(dec, cxs, IADT).ok_or(Jbig2Error::InvalidHuffman(
            "text region: strip IADT returned OOB",
        ))?;
        strip_t = strip_t.saturating_add((dt as i64).saturating_mul(sb_strips as i64));

        let mut cur_s = 0i64;
        let mut first = true;
        loop {
            if first {
                let dfs = decode_integer(dec, cxs, IAFS)
                    .ok_or(Jbig2Error::InvalidHuffman("text region: IAFS returned OOB"))?;
                first_s = first_s.saturating_add(dfs as i64);
                cur_s = first_s;
                first = false;
            } else {
                match decode_integer(dec, cxs, IADS) {
                    None => break,
                    Some(v) => {
                        cur_s = cur_s.saturating_add(v as i64 + header.sbds_offset as i64);
                    }
                }
            }
            let t_abs = if header.sbrefine {
                0
            } else {
                let current_t = if sb_strips != 1 {
                    decode_integer(dec, cxs, IAIT)
                        .ok_or(Jbig2Error::InvalidHuffman("text region: IAIT returned OOB"))?
                        as i64
                } else {
                    0
                };
                strip_t.saturating_add(current_t)
            };

            let id = decode_iaid(dec, cxs, IAID, code_len);
            if (id as usize) >= sbsyms.len() {
                return Err(Jbig2Error::OutOfRange(
                    "text region: symbol ID out of range",
                ));
            }

            let r_flag = if header.sbrefine {
                decode_integer(dec, cxs, IARI)
                    .ok_or(Jbig2Error::InvalidHuffman("text region: IARI returned OOB"))?
            } else {
                0
            };

            let ib_owned: Bitmap;
            let ib: &Bitmap = if r_flag == 0 {
                sbsyms[id as usize]
            } else {
                let rdw = decode_integer(dec, cxs, IARDW).ok_or(Jbig2Error::InvalidHuffman(
                    "text region: IARDW returned OOB",
                ))?;
                let rdh = decode_integer(dec, cxs, IARDH).ok_or(Jbig2Error::InvalidHuffman(
                    "text region: IARDH returned OOB",
                ))?;
                let rdx = decode_integer(dec, cxs, IARDX).ok_or(Jbig2Error::InvalidHuffman(
                    "text region: IARDX returned OOB",
                ))?;
                let rdy = decode_integer(dec, cxs, IARDY).ok_or(Jbig2Error::InvalidHuffman(
                    "text region: IARDY returned OOB",
                ))?;
                let ref_bm = sbsyms[id as usize];
                let ib_w = ref_bm.width() as i32 + rdw;
                let ib_h = ref_bm.height() as i32 + rdh;
                if ib_w <= 0 || ib_h <= 0 {
                    return Err(Jbig2Error::OutOfRange(
                        "text region: refined instance has non-positive dimensions",
                    ));
                }
                let ref_dx = rdw.div_euclid(2) + rdx;
                let ref_dy = rdh.div_euclid(2) + rdy;
                ib_owned = decode_refinement_region(
                    dec,
                    cxs,
                    ib_w as u32,
                    ib_h as u32,
                    if header.sbr_template { 1 } else { 0 },
                    false,
                    &header.rat,
                    ref_bm,
                    ref_dx,
                    ref_dy,
                )?;
                &ib_owned
            };

            let (x0, y0) =
                instance_origin(ib, &mut cur_s, t_abs, header.transposed, header.ref_corner);
            emit(ib, x0, y0, ninst)?;
            ninst += 1;
            if ninst >= header.num_instances {
                break;
            }
        }
    }
    Ok(())
}

/// Encode a text region body from a list of symbol instances. Instances
/// must be given in the natural composition order (increasing strip_t, then
/// increasing S within a strip). Returns the encoded bytes.
pub fn encode_text_region(
    header: &TextRegionHeader,
    instances: &[SymbolInstance],
    symbols: &[Bitmap],
) -> Jbig2Result<Vec<u8>> {
    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    encode_text_region_with_contexts(header, instances, symbols, &mut cxs)
}

/// Encode a text region body using a caller-owned MQ context pool.
pub fn encode_text_region_with_contexts(
    header: &TextRegionHeader,
    instances: &[SymbolInstance],
    symbols: &[Bitmap],
    cxs: &mut MqContexts,
) -> Jbig2Result<Vec<u8>> {
    if header.sbhuff {
        return Err(Jbig2Error::Unsupported(
            "text region encoder: Huffman path not yet implemented",
        ));
    }
    let mut enc = MqEncoder::new(32 + instances.len() * 16);
    cxs.reset();
    encode_text_region_body(&mut enc, cxs, header, instances, symbols)?;
    Ok(enc.finish())
}

/// Encode a text-region body into an externally-owned MQ encoder /
/// context pool. Used by aggregate symbol-dictionary encoding so the
/// nested text region shares the SD's MQ stream and contexts.
pub fn encode_text_region_body(
    enc: &mut MqEncoder,
    cxs: &mut MqContexts,
    header: &TextRegionHeader,
    instances: &[SymbolInstance],
    symbols: &[Bitmap],
) -> Jbig2Result<()> {
    if header.sbhuff {
        return Err(Jbig2Error::Unsupported(
            "text region encoder: Huffman path not yet implemented",
        ));
    }
    if header.transposed {
        return Err(Jbig2Error::Unsupported(
            "text region encoder: transposed path not yet implemented",
        ));
    }
    if !matches!(header.ref_corner, RefCorner::TL) {
        return Err(Jbig2Error::Unsupported(
            "text region encoder: only TL reference corner supported",
        ));
    }
    if instances.len() as u32 != header.num_instances {
        return Err(Jbig2Error::InvalidConfig(
            "text region: instance count != header num_instances",
        ));
    }
    if symbols.is_empty() {
        return Err(Jbig2Error::InvalidConfig(
            "text region: symbol library is empty",
        ));
    }
    let any_refined = instances.iter().any(|i| i.refinement.is_some());
    if any_refined && !header.sbrefine {
        return Err(Jbig2Error::InvalidConfig(
            "text region: refined instance requires SBREFINE = 1 in header",
        ));
    }

    let code_len = sym_code_len(symbols.len() as u32);
    let sb_strips = 1i64 << header.log_sbstrips;

    let mut idx = 0usize;
    let mut prev_strip_idx: i64 = 0;
    let mut first_strip = true;
    let mut first_s_prev: i64 = 0;

    encode_integer(enc, cxs, IADT, 0)?;

    while idx < instances.len() {
        let strip_idx = instances[idx].y as i64 / sb_strips;
        let dt = if first_strip {
            first_strip = false;
            strip_idx
        } else {
            strip_idx - prev_strip_idx
        };
        prev_strip_idx = strip_idx;
        encode_integer(enc, cxs, IADT, dt as i32)?;

        let strip_end = strip_idx.saturating_add(1).saturating_mul(sb_strips);
        let strip_begin = strip_idx.saturating_mul(sb_strips);
        let mut cur_s: i64 = 0;
        let mut first_in_strip = true;
        while idx < instances.len() {
            let ins = &instances[idx];
            let y = ins.y as i64;
            if y < strip_begin || y >= strip_end {
                break;
            }
            let s = ins.x as i64;
            let t = y - strip_begin;
            let ref_sym = symbols.get(ins.id as usize).ok_or(Jbig2Error::OutOfRange(
                "text region: instance references out-of-range symbol",
            ))?;
            let sym_w = match &ins.refinement {
                Some(r) => (ref_sym.width() as i64) + r.rdw as i64,
                None => ref_sym.width() as i64,
            };

            if first_in_strip {
                let dfs = s - first_s_prev;
                first_s_prev = s;
                encode_integer(enc, cxs, IAFS, dfs as i32)?;
                first_in_strip = false;
            } else {
                let ids = s - cur_s - header.sbds_offset as i64;
                encode_integer(enc, cxs, IADS, ids as i32)?;
            }
            if sb_strips != 1 && !header.sbrefine {
                encode_integer(enc, cxs, IAIT, t as i32)?;
            }
            encode_iaid(enc, cxs, IAID, code_len, ins.id);
            if header.sbrefine {
                let r_flag = if ins.refinement.is_some() { 1 } else { 0 };
                encode_integer(enc, cxs, IARI, r_flag)?;
                if let Some(r) = &ins.refinement {
                    let expected_w = ref_sym.width() as i32 + r.rdw;
                    let expected_h = ref_sym.height() as i32 + r.rdh;
                    if expected_w <= 0 || expected_h <= 0 {
                        return Err(Jbig2Error::InvalidConfig(
                            "text region: refined instance has non-positive size",
                        ));
                    }
                    if r.target.width() as i32 != expected_w
                        || r.target.height() as i32 != expected_h
                    {
                        return Err(Jbig2Error::InvalidConfig(
                            "text region: refined target size != reference + (RDW, RDH)",
                        ));
                    }
                    encode_integer(enc, cxs, IARDW, r.rdw)?;
                    encode_integer(enc, cxs, IARDH, r.rdh)?;
                    encode_integer(enc, cxs, IARDX, r.rdx)?;
                    encode_integer(enc, cxs, IARDY, r.rdy)?;
                    let ref_dx = r.rdw.div_euclid(2) + r.rdx;
                    let ref_dy = r.rdh.div_euclid(2) + r.rdy;
                    encode_refinement_region(
                        enc,
                        cxs,
                        &r.target,
                        if header.sbr_template { 1 } else { 0 },
                        false,
                        &header.rat,
                        ref_sym,
                        ref_dx,
                        ref_dy,
                    )?;
                }
            }
            cur_s = s + sym_w - 1;
            idx += 1;
        }
        encode_integer(enc, cxs, IADS, OOB)?;
    }

    Ok(())
}

fn instance_origin(
    ib: &Bitmap,
    cur_s: &mut i64,
    t: i64,
    transposed: bool,
    corner: RefCorner,
) -> (i32, i32) {
    // "Pre-blit" cur_s adjustment for BR/TR (un-transposed) or BL/BR
    // (transposed).
    match (transposed, corner) {
        (false, RefCorner::BR | RefCorner::TR) => *cur_s += ib.width() as i64 - 1,
        (true, RefCorner::BL | RefCorner::BR) => *cur_s += ib.height() as i64 - 1,
        _ => {}
    }
    let mut s_val = *cur_s;
    let mut t_val = t;
    if transposed {
        std::mem::swap(&mut s_val, &mut t_val);
    }
    // Anchor by reference corner.
    match corner {
        RefCorner::BL => t_val -= ib.height() as i64 - 1,
        RefCorner::BR => {
            t_val -= ib.height() as i64 - 1;
            s_val -= ib.width() as i64 - 1;
        }
        RefCorner::TR => s_val -= ib.width() as i64 - 1,
        RefCorner::TL => {}
    }

    // Blit ib at (s_val, t_val). For transposed regions, axes are already
    // swapped; x becomes s_val, y becomes t_val.
    let x0 = s_val as i32;
    let y0 = t_val as i32;

    // Post-blit cur_s update for TL/BL (un-transposed) or TL/TR (transposed).
    match (transposed, corner) {
        (false, RefCorner::TL | RefCorner::BL) => *cur_s += ib.width() as i64 - 1,
        (true, RefCorner::TL | RefCorner::TR) => *cur_s += ib.height() as i64 - 1,
        _ => {}
    }
    (x0, y0)
}

fn composite_instance(
    region: &mut Bitmap,
    ib: &Bitmap,
    cur_s: &mut i64,
    t: i64,
    transposed: bool,
    corner: RefCorner,
    op: CombinationOp,
) {
    let (x0, y0) = instance_origin(ib, cur_s, t, transposed, corner);
    blit(region, ib, x0, y0, op);
}

fn blit(region: &mut Bitmap, ib: &Bitmap, x0: i32, y0: i32, op: CombinationOp) {
    // Route through the shared packed-row blitter. Clipping is handled
    // inside [`Bitmap::composite`], so text-region placement no longer has
    // to iterate pixel by pixel.
    region.composite(ib, x0, y0, op.into());
}

/// Return the number of bits required to encode every symbol ID in a
/// library of `num_syms` entries — `max(1, ceil(log2 num_syms))` for
/// arithmetic coding.
pub fn sym_code_len(num_syms: u32) -> u32 {
    if num_syms <= 1 {
        0
    } else {
        32 - (num_syms - 1).leading_zeros()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(pixel: u8, w: u32, h: u32) -> Bitmap {
        Bitmap::filled(w, h, pixel).unwrap()
    }

    fn round_trip(log_strips: u8, instances: Vec<SymbolInstance>, syms: Vec<Bitmap>) -> Bitmap {
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        for ins in &instances {
            let s = &syms[ins.id as usize];
            max_x = max_x.max((ins.x + s.width() as i32) as u32);
            max_y = max_y.max((ins.y + s.height() as i32) as u32);
        }
        let header = TextRegionHeader {
            region: RegionInfo {
                width: max_x + 4,
                height: max_y + 4,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            sbhuff: false,
            sbrefine: false,
            log_sbstrips: log_strips,
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
            rat: [(0, 0); 2],
            num_instances: instances.len() as u32,
        };
        let body = encode_text_region(&header, &instances, &syms).unwrap();
        let sbsyms_ref: Vec<&Bitmap> = syms.iter().collect();
        decode_text_region(&header, &body, &sbsyms_ref).unwrap()
    }

    #[test]
    fn header_round_trip() {
        let hdr = TextRegionHeader {
            region: RegionInfo {
                width: 100,
                height: 60,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            sbhuff: false,
            sbrefine: false,
            log_sbstrips: 1,
            ref_corner: RefCorner::TL,
            transposed: false,
            sbcombop: CombinationOp::Or,
            default_pixel: 0,
            sbds_offset: -2,
            sbr_template: false,
            sbhuff_fs: 0,
            sbhuff_ds: 0,
            sbhuff_dt: 0,
            sbhuff_rdw: 0,
            sbhuff_rdh: 0,
            sbhuff_rdx: 0,
            sbhuff_rdy: 0,
            sbhuff_rsize: false,
            rat: [(0, 0); 2],
            num_instances: 42,
        };
        let mut buf = Vec::new();
        hdr.write(&mut buf).unwrap();
        let back = TextRegionHeader::read(&mut buf.as_slice()).unwrap();
        assert_eq!(back.num_instances, 42);
        assert_eq!(back.sbds_offset, -2);
        assert_eq!(back.log_sbstrips, 1);
    }

    #[test]
    fn round_trip_single_strip_single_sym() {
        let syms = vec![glyph(1, 5, 7)];
        let instances = vec![
            SymbolInstance::placement(0, 0, 0),
            SymbolInstance::placement(0, 10, 0),
            SymbolInstance::placement(0, 20, 0),
        ];
        let bm = round_trip(0, instances, syms);
        // Three 5x7 filled rectangles.
        for (x0, _) in [(0, 0), (10, 0), (20, 0)].iter() {
            for dy in 0..7 {
                for dx in 0..5 {
                    assert_eq!(bm.get_pixel(x0 + dx, dy), 1);
                }
            }
        }
    }

    #[test]
    fn round_trip_multi_strip_multi_sym() {
        let syms = vec![glyph(1, 4, 6), glyph(1, 6, 8)];
        let instances = vec![
            SymbolInstance::placement(0, 0, 0),
            SymbolInstance::placement(1, 8, 0),
            SymbolInstance::placement(0, 0, 10),
            SymbolInstance::placement(1, 10, 10),
        ];
        let bm = round_trip(1, instances.clone(), syms.clone());
        // Spot-check a few pixels.
        assert_eq!(bm.get_pixel(0, 0), 1);
        assert_eq!(bm.get_pixel(3, 5), 1); // end of first 4x6
        assert_eq!(bm.get_pixel(8, 0), 1);
        assert_eq!(bm.get_pixel(13, 7), 1); // inside second 6x8 at row 0
        assert_eq!(bm.get_pixel(0, 10), 1);
        assert_eq!(bm.get_pixel(10, 17), 1);
    }

    #[test]
    fn round_trip_sbrefine_one_refined_one_plain() {
        // Two symbols in the library; one instance is plain, one is a
        // refined version of the second symbol.
        let base = glyph(1, 6, 8);
        let other = glyph(1, 5, 7);
        let syms_owned = vec![base.clone(), other.clone()];

        // Refined target = `other` with one bit flipped; same dimensions.
        let mut refined_target = other.clone();
        refined_target.set_pixel(2, 3, refined_target.get_pixel(2, 3) ^ 1);

        let instances = vec![
            SymbolInstance::placement(0, 0, 0),
            SymbolInstance {
                id: 1,
                x: 10,
                y: 0,
                refinement: Some(RefinedInstance {
                    rdw: 0,
                    rdh: 0,
                    rdx: 0,
                    rdy: 0,
                    target: refined_target.clone(),
                }),
            },
        ];

        let header = TextRegionHeader {
            region: RegionInfo {
                width: 24,
                height: 12,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            sbhuff: false,
            sbrefine: true,
            log_sbstrips: 0,
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
            rat: [(-1, -1), (-1, -1)],
            num_instances: instances.len() as u32,
        };
        let body = encode_text_region(&header, &instances, &syms_owned).unwrap();
        let sbsyms_ref: Vec<&Bitmap> = syms_owned.iter().collect();
        let bm = decode_text_region(&header, &body, &sbsyms_ref).unwrap();
        // Plain instance (base, 6x8) at (0, 0): every pixel set.
        for dy in 0..8 {
            for dx in 0..6 {
                assert_eq!(bm.get_pixel(dx, dy), 1, "plain pixel ({dx},{dy})");
            }
        }
        // Refined instance (other with flip) at (10, 0): all set except the
        // flipped one which started at value 1.
        assert_eq!(bm.get_pixel(10 + 2, 3), 0);
        assert_eq!(bm.get_pixel(10 + 0, 0), 1);
    }

    #[test]
    fn sym_code_len_boundaries() {
        assert_eq!(sym_code_len(1), 0);
        assert_eq!(sym_code_len(2), 1);
        assert_eq!(sym_code_len(3), 2);
        assert_eq!(sym_code_len(4), 2);
        assert_eq!(sym_code_len(5), 3);
        assert_eq!(sym_code_len(256), 8);
        assert_eq!(sym_code_len(257), 9);
    }
}
