//! Text region segment (spec 7.4.3 / 6.4).
//!
//! The v1 implementation covers the arithmetic-coded, no-refinement path
//! (`SBHUFF = 0`, `SBREFINE = 0`). This handles every text region produced
//! by this crate's lossless symbol classifier plus the common output of
//! `jbig2enc -S` on patent-style scans.

use std::io::{Read, Write};

use crate::bitmap::Bitmap;
use crate::coding::mq::{MqContexts, MqDecoder, MqEncoder, MQ_NUM_CONTEXTS};
use crate::coding::mq_context::{IADS, IADT, IAFS, IAID, IAIT, IARI};
use crate::coding::mq_integer::{decode_iaid, decode_integer, encode_iaid, encode_integer, OOB};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::page_information::CombinationOp;
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

/// Parsed text region segment header (arithmetic variant).
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

        if sbhuff {
            // Skip the 2-byte Huffman flags field; we don't decode Huffman
            // regions yet, but callers may still want the header fields.
            let mut skip = [0u8; 2];
            r.read_exact(&mut skip)?;
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
            // Zero-filled Huffman flags for forward compatibility with the
            // decoder side; we never set them from the encoder.
            w.write_all(&[0, 0])?;
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolInstance {
    /// Symbol identifier (index into `SBSYMS`).
    pub id: u32,
    /// X coordinate (top-left, un-transposed).
    pub x: i32,
    /// Y coordinate (top-left, un-transposed).
    pub y: i32,
}

/// Decode an arithmetic-coded text region body, compositing every symbol
/// instance into a fresh bitmap of size `(header.region.width,
/// header.region.height)`. `sbsyms` is the full symbol library as defined
/// in 6.4.5.
pub fn decode_text_region(
    header: &TextRegionHeader,
    body: &[u8],
    sbsyms: &[Bitmap],
) -> Jbig2Result<Bitmap> {
    if header.sbhuff {
        return Err(Jbig2Error::Unsupported(
            "text region: Huffman coding not yet implemented",
        ));
    }
    if header.sbrefine {
        return Err(Jbig2Error::Unsupported(
            "text region: symbol refinement not yet implemented",
        ));
    }
    if sbsyms.is_empty() {
        return Err(Jbig2Error::OutOfRange(
            "text region: symbol library is empty",
        ));
    }

    let code_len = sym_code_len(sbsyms.len() as u32);
    let sb_strips = 1u32 << header.log_sbstrips;

    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    let mut dec = MqDecoder::new(body);

    let mut region = Bitmap::filled(
        header.region.width,
        header.region.height,
        header.default_pixel,
    )?;

    // Step 2: decode initial STRIPT, negate.
    let s0 = decode_integer(&mut dec, &mut cxs, IADT).ok_or(
        Jbig2Error::InvalidHuffman("text region: IADT initial returned OOB"),
    )?;
    let mut strip_t: i64 = -(s0 as i64) * sb_strips as i64;
    let mut first_s: i64 = 0;
    let mut ninst: u32 = 0;

    while ninst < header.num_instances {
        let dt = decode_integer(&mut dec, &mut cxs, IADT).ok_or(
            Jbig2Error::InvalidHuffman("text region: strip IADT returned OOB"),
        )?;
        strip_t = strip_t.saturating_add((dt as i64).saturating_mul(sb_strips as i64));

        let mut cur_s: i64;
        let mut first = true;
        cur_s = 0;
        loop {
            if first {
                let dfs = decode_integer(&mut dec, &mut cxs, IAFS).ok_or(
                    Jbig2Error::InvalidHuffman("text region: IAFS returned OOB"),
                )?;
                first_s = first_s.saturating_add(dfs as i64);
                cur_s = first_s;
                first = false;
            } else {
                let ids = decode_integer(&mut dec, &mut cxs, IADS);
                match ids {
                    None => break,
                    Some(v) => {
                        cur_s = cur_s.saturating_add(v as i64 + header.sbds_offset as i64);
                    }
                }
            }
            let current_t = if sb_strips != 1 {
                decode_integer(&mut dec, &mut cxs, IAIT).ok_or(Jbig2Error::InvalidHuffman(
                    "text region: IAIT returned OOB",
                ))? as i64
            } else {
                0
            };
            let t_abs = strip_t.saturating_add(current_t);

            let id = decode_iaid(&mut dec, &mut cxs, IAID, code_len);
            if (id as usize) >= sbsyms.len() {
                return Err(Jbig2Error::OutOfRange("text region: symbol ID out of range"));
            }

            let r = if header.sbrefine {
                decode_integer(&mut dec, &mut cxs, IARI).ok_or(Jbig2Error::InvalidHuffman(
                    "text region: IARI returned OOB",
                ))?
            } else {
                0
            };
            let _ = r; // no-refinement: always 0

            let ib = &sbsyms[id as usize];
            composite_instance(
                &mut region,
                ib,
                &mut cur_s,
                t_abs,
                header.transposed,
                header.ref_corner,
                header.sbcombop,
            );
            ninst += 1;
        }
    }

    Ok(region)
}

/// Encode a text region body from a list of symbol instances. Instances
/// must be given in the natural composition order (increasing strip_t, then
/// increasing S within a strip). Returns the encoded bytes.
pub fn encode_text_region(
    header: &TextRegionHeader,
    instances: &[SymbolInstance],
    symbols: &[Bitmap],
) -> Jbig2Result<Vec<u8>> {
    if header.sbhuff || header.sbrefine {
        return Err(Jbig2Error::Unsupported(
            "text region encoder: only arithmetic, no-refine path supported",
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

    let code_len = sym_code_len(symbols.len() as u32);
    let sb_strips = 1i64 << header.log_sbstrips;

    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    let mut enc = MqEncoder::new(32 + instances.len() * 16);

    // Mirror the decoder's accumulator: STRIPT starts negative, the first
    // DT encodes the first strip index, and cur_s is advanced by
    // `width - 1 + sbdsoffset` after each instance (for TL/BL, untransposed).
    let mut idx = 0usize;
    let mut prev_strip_idx: i64 = 0;
    let mut first_strip = true;
    let mut first_s_prev: i64 = 0;

    encode_integer(&mut enc, &mut cxs, IADT, 0)?;

    while idx < instances.len() {
        let strip_idx = instances[idx].y as i64 / sb_strips;
        let dt = if first_strip {
            first_strip = false;
            strip_idx
        } else {
            strip_idx - prev_strip_idx
        };
        prev_strip_idx = strip_idx;
        encode_integer(&mut enc, &mut cxs, IADT, dt as i32)?;

        let strip_end = strip_idx.saturating_add(1).saturating_mul(sb_strips);
        let strip_begin = strip_idx.saturating_mul(sb_strips);
        let mut cur_s: i64 = 0;
        let mut first_in_strip = true;
        while idx < instances.len() {
            let ins = instances[idx];
            let y = ins.y as i64;
            if y < strip_begin || y >= strip_end {
                break;
            }
            let s = ins.x as i64;
            let t = y - strip_begin;
            let sym_w = symbols
                .get(ins.id as usize)
                .ok_or(Jbig2Error::OutOfRange(
                    "text region: instance references out-of-range symbol",
                ))?
                .width() as i64;

            if first_in_strip {
                let dfs = s - first_s_prev;
                first_s_prev = s;
                encode_integer(&mut enc, &mut cxs, IAFS, dfs as i32)?;
                first_in_strip = false;
            } else {
                // Decoder update: cur_s := cur_s + IDS + SBDSOFFSET, then we
                // also post-advance cur_s by width-1 of the previous symbol.
                // That post-advance has already been applied below; so here
                // we just compute IDS = s - cur_s - SBDSOFFSET.
                let ids = s - cur_s - header.sbds_offset as i64;
                encode_integer(&mut enc, &mut cxs, IADS, ids as i32)?;
            }
            if sb_strips != 1 {
                encode_integer(&mut enc, &mut cxs, IAIT, t as i32)?;
            }
            encode_iaid(&mut enc, &mut cxs, IAID, code_len, ins.id);
            // Post-advance to match composite_instance's TL-corner behaviour.
            cur_s = s + sym_w - 1;
            idx += 1;
        }
        encode_integer(&mut enc, &mut cxs, IADS, OOB)?;
    }

    Ok(enc.finish())
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
    blit(region, ib, x0, y0, op);

    // Post-blit cur_s update for TL/BL (un-transposed) or TL/TR (transposed).
    match (transposed, corner) {
        (false, RefCorner::TL | RefCorner::BL) => *cur_s += ib.width() as i64 - 1,
        (true, RefCorner::TL | RefCorner::TR) => *cur_s += ib.height() as i64 - 1,
        _ => {}
    }
}

fn blit(region: &mut Bitmap, ib: &Bitmap, x0: i32, y0: i32, op: CombinationOp) {
    let rw = region.width() as i32;
    let rh = region.height() as i32;
    for iy in 0..ib.height() as i32 {
        let ry = y0 + iy;
        if ry < 0 || ry >= rh {
            continue;
        }
        for ix in 0..ib.width() as i32 {
            let rx = x0 + ix;
            if rx < 0 || rx >= rw {
                continue;
            }
            let s = ib.get_pixel(ix, iy);
            let d = region.get_pixel(rx, ry);
            let out = match op {
                CombinationOp::Or => s | d,
                CombinationOp::And => s & d,
                CombinationOp::Xor => s ^ d,
                CombinationOp::XNor => 1 ^ (s ^ d),
                CombinationOp::Replace => s,
            };
            region.set_pixel(rx, ry, out);
        }
    }
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
            rat: [(0, 0); 2],
            num_instances: instances.len() as u32,
        };
        let body = encode_text_region(&header, &instances, &syms).unwrap();
        decode_text_region(&header, &body, &syms).unwrap()
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
            SymbolInstance { id: 0, x: 0, y: 0 },
            SymbolInstance { id: 0, x: 10, y: 0 },
            SymbolInstance { id: 0, x: 20, y: 0 },
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
            SymbolInstance { id: 0, x: 0, y: 0 },
            SymbolInstance { id: 1, x: 8, y: 0 },
            SymbolInstance { id: 0, x: 0, y: 10 },
            SymbolInstance { id: 1, x: 10, y: 10 },
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
