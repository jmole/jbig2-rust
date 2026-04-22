//! Generic region segment (spec 7.4.6 / 6.2).
//!
//! Decodes / encodes a region bitmap using one of:
//!
//! * MMR (`MMR = 1`): T.6 coded line-by-line
//! * Arithmetic coding with GB template 0..3 (+ AMD2 extended when
//!   `EXTTEMPLATE = 1`)
//!
//! Supports the optional TPGD (typical-prediction generic direct) scheme that
//! short-circuits rows identical to the previous one.

use std::io::{Read, Write};

use crate::bitmap::Bitmap;
use crate::coding::mmr::{BitReader as MmrBitReader, BitWriter as MmrBitWriter, decode_t6_line, encode_t6_line, row_to_bools};
use crate::coding::mq::{MqContexts, MqDecoder, MqEncoder, MQ_NUM_CONTEXTS};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::region_info::RegionInfo;

/// Header for a generic region segment (region info + flags + AT pixels).
#[derive(Clone, Debug)]
pub struct GenericRegionHeader {
    /// Region info (width, height, page coordinates, combination op).
    pub region: RegionInfo,
    /// `true` when MMR (T.6) coding is used.
    pub mmr: bool,
    /// Template selector (0..3).
    pub template: u8,
    /// Typical-prediction generic direct coding.
    pub tpgdon: bool,
    /// Extended 12-AT template (AMD2), only valid with `template == 0`.
    pub ext_template: bool,
    /// AT pixel offsets. Only as many as the selected template needs are
    /// meaningful. Unused slots are `(0, 0)`.
    pub at: [(i8, i8); 12],
}

impl GenericRegionHeader {
    /// Parse the generic region header (region info + 1 flags byte + AT
    /// pixels).
    pub fn read<R: Read>(r: &mut R) -> Jbig2Result<Self> {
        let region = RegionInfo::read(r)?;
        let mut fb = [0u8; 1];
        r.read_exact(&mut fb).map_err(Jbig2Error::from)?;
        let flags = fb[0];
        let mmr = (flags & 0x01) != 0;
        let template = (flags >> 1) & 0x03;
        let tpgdon = (flags & 0x08) != 0;
        let ext_template = (flags & 0x10) != 0;
        if mmr && template != 0 {
            return Err(Jbig2Error::InvalidSegmentHeader(
                "MMR generic region with non-zero template",
            ));
        }

        let mut at = [(0i8, 0i8); 12];
        if !mmr {
            let n_at = if template == 0 {
                if ext_template { 12 } else { 4 }
            } else {
                1
            };
            let mut buf = vec![0u8; 2 * n_at];
            r.read_exact(&mut buf).map_err(Jbig2Error::from)?;
            for i in 0..n_at {
                at[i] = (buf[2 * i] as i8, buf[2 * i + 1] as i8);
            }
        }

        Ok(Self {
            region,
            mmr,
            template,
            tpgdon,
            ext_template,
            at,
        })
    }

    /// Emit the generic region header.
    pub fn write<W: Write>(&self, w: &mut W) -> Jbig2Result<()> {
        self.region.write(w)?;
        let flags = (self.mmr as u8)
            | ((self.template & 0x03) << 1)
            | ((self.tpgdon as u8) << 3)
            | ((self.ext_template as u8) << 4);
        w.write_all(&[flags]).map_err(Jbig2Error::from)?;
        if !self.mmr {
            let n_at = if self.template == 0 {
                if self.ext_template { 12 } else { 4 }
            } else {
                1
            };
            let mut buf = Vec::with_capacity(2 * n_at);
            for i in 0..n_at {
                buf.push(self.at[i].0 as u8);
                buf.push(self.at[i].1 as u8);
            }
            w.write_all(&buf).map_err(Jbig2Error::from)?;
        }
        Ok(())
    }
}

/// SLTP CX values per template, used when TPGDON=1 (spec 6.2.5.7 + figures
/// 8..11). These values correspond to the AT pixels at their nominal
/// positions encoded together with the fixed template pixels in a specific
/// "all-1-neighbourhood" pattern the spec shows in the figures.
const SLTP_CX: [u32; 4] = [0x9B25, 0x0795, 0x00E5, 0x0195];

/// Default nominal AT pixel positions from spec Table 5.
pub fn nominal_at(template: u8, ext_template: bool) -> [(i8, i8); 12] {
    let mut out = [(0i8, 0); 12];
    if template == 0 && !ext_template {
        out[0] = (3, -1);
        out[1] = (-3, -1);
        out[2] = (2, -2);
        out[3] = (-2, -2);
    } else if template == 0 && ext_template {
        out[0] = (-2, 0);
        out[1] = (0, -2);
        out[2] = (-2, -1);
        out[3] = (-1, -2);
        out[4] = (1, -2);
        out[5] = (2, -1);
        out[6] = (-3, 0);
        out[7] = (-4, 0);
        out[8] = (2, -2);
        out[9] = (3, -1);
        out[10] = (-2, -2);
        out[11] = (-3, -1);
    } else if template == 1 {
        out[0] = (3, -1);
    } else if template == 2 {
        out[0] = (2, -1);
    } else {
        out[0] = (2, -1);
    }
    out
}

/// Decode an arithmetic-coded generic region bitmap.
pub fn decode_generic_arith(
    dec: &mut MqDecoder<'_>,
    cxs: &mut MqContexts,
    header: &GenericRegionHeader,
) -> Jbig2Result<Bitmap> {
    decode_generic_bitmap(
        dec,
        cxs,
        header.region.width,
        header.region.height,
        header.template,
        header.ext_template,
        header.tpgdon,
        &header.at,
    )
}

/// Decode a generic bitmap without parsing a segment header. Used by the
/// symbol dictionary decoder, which reuses a shared MQ context pool across
/// symbols and therefore cannot use the full-segment wrapper.
pub fn decode_generic_bitmap(
    dec: &mut MqDecoder<'_>,
    cxs: &mut MqContexts,
    width: u32,
    height: u32,
    template: u8,
    ext_template: bool,
    tpgdon: bool,
    at: &[(i8, i8); 12],
) -> Jbig2Result<Bitmap> {
    let mut bitmap = Bitmap::new(width, height)?;
    if width == 0 || height == 0 {
        return Ok(bitmap);
    }
    let sltp_cx = SLTP_CX[template as usize];
    let mut ltp: u8 = 0;
    for y in 0..height {
        if tpgdon {
            let sltp = dec.decode(cxs, sltp_cx as usize);
            ltp ^= sltp;
        }
        if tpgdon && ltp == 1 {
            if y > 0 {
                let stride = bitmap.stride();
                let (up, down) = bitmap.data_mut().split_at_mut((y as usize) * stride);
                let prev = &up[up.len() - stride..];
                down[..stride].copy_from_slice(prev);
            }
            continue;
        }
        for x in 0..width as i32 {
            let cx = build_gb_context(&bitmap, x, y as i32, template, ext_template, at);
            let bit = dec.decode(cxs, cx as usize);
            if bit != 0 {
                bitmap.set_pixel(x, y as i32, 1);
            }
        }
    }
    Ok(bitmap)
}

/// Build the GB template context for a pixel at `(x, y)`.
fn build_gb_context(
    bm: &Bitmap,
    x: i32,
    y: i32,
    template: u8,
    ext: bool,
    at: &[(i8, i8); 12],
) -> u32 {
    match (template, ext) {
        (0, false) => {
            let b11 = bm.get_pixel(x - 1, y - 2);
            let b10 = bm.get_pixel(x, y - 2);
            let b9 = bm.get_pixel(x + 1, y - 2);
            let b8 = bm.get_pixel(x - 2, y - 1);
            let b7 = bm.get_pixel(x - 1, y - 1);
            let b6 = bm.get_pixel(x, y - 1);
            let b5 = bm.get_pixel(x + 1, y - 1);
            let b4 = bm.get_pixel(x + 2, y - 1);
            let b3 = bm.get_pixel(x - 4, y);
            let b2 = bm.get_pixel(x - 3, y);
            let b1 = bm.get_pixel(x - 2, y);
            let b0 = bm.get_pixel(x - 1, y);
            let a1 = bm.get_pixel(x + at[0].0 as i32, y + at[0].1 as i32);
            let a2 = bm.get_pixel(x + at[1].0 as i32, y + at[1].1 as i32);
            let a3 = bm.get_pixel(x + at[2].0 as i32, y + at[2].1 as i32);
            let a4 = bm.get_pixel(x + at[3].0 as i32, y + at[3].1 as i32);
            ((a4 as u32) << 15)
                | ((a3 as u32) << 14)
                | ((a2 as u32) << 13)
                | ((a1 as u32) << 12)
                | ((b11 as u32) << 11)
                | ((b10 as u32) << 10)
                | ((b9 as u32) << 9)
                | ((b8 as u32) << 8)
                | ((b7 as u32) << 7)
                | ((b6 as u32) << 6)
                | ((b5 as u32) << 5)
                | ((b4 as u32) << 4)
                | ((b3 as u32) << 3)
                | ((b2 as u32) << 2)
                | ((b1 as u32) << 1)
                | (b0 as u32)
        }
        (1, _) => {
            let b11 = bm.get_pixel(x - 1, y - 2);
            let b10 = bm.get_pixel(x, y - 2);
            let b9 = bm.get_pixel(x + 1, y - 2);
            let b8 = bm.get_pixel(x + 2, y - 2);
            let b7 = bm.get_pixel(x - 2, y - 1);
            let b6 = bm.get_pixel(x - 1, y - 1);
            let b5 = bm.get_pixel(x, y - 1);
            let b4 = bm.get_pixel(x + 1, y - 1);
            let b3 = bm.get_pixel(x + 2, y - 1);
            let b2 = bm.get_pixel(x - 3, y);
            let b1 = bm.get_pixel(x - 2, y);
            let b0 = bm.get_pixel(x - 1, y);
            let a1 = bm.get_pixel(x + at[0].0 as i32, y + at[0].1 as i32);
            ((a1 as u32) << 12)
                | ((b11 as u32) << 11)
                | ((b10 as u32) << 10)
                | ((b9 as u32) << 9)
                | ((b8 as u32) << 8)
                | ((b7 as u32) << 7)
                | ((b6 as u32) << 6)
                | ((b5 as u32) << 5)
                | ((b4 as u32) << 4)
                | ((b3 as u32) << 3)
                | ((b2 as u32) << 2)
                | ((b1 as u32) << 1)
                | (b0 as u32)
        }
        (2, _) => {
            let b8 = bm.get_pixel(x - 1, y - 2);
            let b7 = bm.get_pixel(x, y - 2);
            let b6 = bm.get_pixel(x + 1, y - 2);
            let b5 = bm.get_pixel(x - 2, y - 1);
            let b4 = bm.get_pixel(x - 1, y - 1);
            let b3 = bm.get_pixel(x, y - 1);
            let b2 = bm.get_pixel(x + 1, y - 1);
            let b1 = bm.get_pixel(x - 2, y);
            let b0 = bm.get_pixel(x - 1, y);
            let a1 = bm.get_pixel(x + at[0].0 as i32, y + at[0].1 as i32);
            ((a1 as u32) << 9)
                | ((b8 as u32) << 8)
                | ((b7 as u32) << 7)
                | ((b6 as u32) << 6)
                | ((b5 as u32) << 5)
                | ((b4 as u32) << 4)
                | ((b3 as u32) << 3)
                | ((b2 as u32) << 2)
                | ((b1 as u32) << 1)
                | (b0 as u32)
        }
        (3, _) => {
            let b8 = bm.get_pixel(x - 3, y - 1);
            let b7 = bm.get_pixel(x - 2, y - 1);
            let b6 = bm.get_pixel(x - 1, y - 1);
            let b5 = bm.get_pixel(x, y - 1);
            let b4 = bm.get_pixel(x + 1, y - 1);
            let b3 = bm.get_pixel(x - 4, y);
            let b2 = bm.get_pixel(x - 3, y);
            let b1 = bm.get_pixel(x - 2, y);
            let b0 = bm.get_pixel(x - 1, y);
            let a1 = bm.get_pixel(x + at[0].0 as i32, y + at[0].1 as i32);
            ((a1 as u32) << 9)
                | ((b8 as u32) << 8)
                | ((b7 as u32) << 7)
                | ((b6 as u32) << 6)
                | ((b5 as u32) << 5)
                | ((b4 as u32) << 4)
                | ((b3 as u32) << 3)
                | ((b2 as u32) << 2)
                | ((b1 as u32) << 1)
                | (b0 as u32)
        }
        // Extended AMD2 template (template 0 + ext = true): 12 AT pixels, 4
        // fixed pixels. CX = A12..A1 B3..B0 (16 bits).
        (0, true) => {
            let b3 = bm.get_pixel(x, y - 1);
            let b2 = bm.get_pixel(x - 1, y - 1);
            let b1 = bm.get_pixel(x - 2, y);
            let b0 = bm.get_pixel(x - 1, y);
            let mut cx: u32 = 0;
            for i in (0..12).rev() {
                let a = bm.get_pixel(x + at[i].0 as i32, y + at[i].1 as i32);
                cx = (cx << 1) | a as u32;
            }
            (cx << 4)
                | ((b3 as u32) << 3)
                | ((b2 as u32) << 2)
                | ((b1 as u32) << 1)
                | (b0 as u32)
        }
        _ => 0,
    }
}

/// Encode an arithmetic-coded generic region into `enc`.
pub fn encode_generic_arith(
    enc: &mut MqEncoder,
    cxs: &mut MqContexts,
    header: &GenericRegionHeader,
    bitmap: &Bitmap,
) -> Jbig2Result<()> {
    if bitmap.width() != header.region.width || bitmap.height() != header.region.height {
        return Err(Jbig2Error::InvalidGeometry(
            "bitmap size != header region size",
        ));
    }
    encode_generic_bitmap(
        enc,
        cxs,
        bitmap,
        header.template,
        header.ext_template,
        header.tpgdon,
        &header.at,
    )
}

/// Encode a generic bitmap without writing a segment header. Companion of
/// [`decode_generic_bitmap`].
pub fn encode_generic_bitmap(
    enc: &mut MqEncoder,
    cxs: &mut MqContexts,
    bitmap: &Bitmap,
    template: u8,
    ext_template: bool,
    tpgdon: bool,
    at: &[(i8, i8); 12],
) -> Jbig2Result<()> {
    let width = bitmap.width();
    let height = bitmap.height();
    if width == 0 || height == 0 {
        return Ok(());
    }
    let sltp_cx = SLTP_CX[template as usize];
    let mut ltp: u8 = 0;
    for y in 0..height {
        let typical = tpgdon && y > 0 && is_typical(bitmap, y);
        if tpgdon {
            let sltp = if typical { ltp ^ 1 } else { ltp };
            enc.encode(cxs, sltp_cx as usize, sltp);
            ltp ^= sltp;
        }
        if tpgdon && ltp == 1 {
            continue;
        }
        for x in 0..width as i32 {
            let cx = build_gb_context(bitmap, x, y as i32, template, ext_template, at);
            let bit = bitmap.get_pixel(x, y as i32);
            enc.encode(cxs, cx as usize, bit);
        }
    }
    Ok(())
}

fn is_typical(bm: &Bitmap, y: u32) -> bool {
    if y == 0 {
        return false;
    }
    let cur = bm.row(y as usize);
    let prev = bm.row(y as usize - 1);
    cur == prev
}

/// Decode an MMR-coded generic region body.
#[cfg(feature = "mmr")]
pub fn decode_generic_mmr(data: &[u8], width: u32, height: u32) -> Jbig2Result<Bitmap> {
    let mut bm = Bitmap::new(width, height)?;
    if width == 0 || height == 0 {
        return Ok(bm);
    }
    let stride = bm.stride();
    let mut r = MmrBitReader::new(data);
    let mut prev_line = vec![false; width as usize];
    for y in 0..height {
        let line = decode_t6_line(&mut r, &prev_line, width)?;
        let row = crate::coding::mmr::bools_to_row(&line, stride);
        bm.row_mut(y as usize).copy_from_slice(&row);
        prev_line = line;
    }
    Ok(bm)
}

/// Encode an MMR-coded generic region body. Returns the encoded bytes
/// including the trailing EOFB.
#[cfg(feature = "mmr")]
pub fn encode_generic_mmr(bitmap: &Bitmap) -> Jbig2Result<Vec<u8>> {
    let width = bitmap.width();
    let height = bitmap.height();
    let mut w = MmrBitWriter::new();
    let mut prev_line = vec![false; width as usize];
    for y in 0..height {
        let cur = row_to_bools(bitmap.row(y as usize), width);
        encode_t6_line(&mut w, &cur, &prev_line);
        prev_line = cur;
    }
    crate::coding::mmr::write_t6_eofb(&mut w);
    Ok(w.finish())
}

/// Convenience: allocate a fresh MQ context pool for a single generic region.
pub fn fresh_mq_context() -> MqContexts {
    MqContexts::new(MQ_NUM_CONTEXTS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256PlusPlus;

    fn random_bitmap(width: u32, height: u32, density: f64, seed: u64) -> Bitmap {
        let mut bm = Bitmap::new(width, height).unwrap();
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                if rng.gen_bool(density) {
                    bm.set_pixel(x, y, 1);
                }
            }
        }
        bm
    }

    fn round_trip_arith(template: u8, ext: bool, tpgdon: bool, bm: &Bitmap) {
        let header = GenericRegionHeader {
            region: RegionInfo {
                width: bm.width(),
                height: bm.height(),
                x: 0,
                y: 0,
                external_combination_op:
                    crate::segments::page_information::CombinationOp::Or,
                colour_extension: false,
            },
            mmr: false,
            template,
            tpgdon,
            ext_template: ext,
            at: nominal_at(template, ext),
        };

        let mut cxs = fresh_mq_context();
        let mut enc = MqEncoder::new(bm.data().len());
        encode_generic_arith(&mut enc, &mut cxs, &header, bm).unwrap();
        let buf = enc.finish();

        let mut cxs = fresh_mq_context();
        let mut dec = MqDecoder::new(&buf);
        let out = decode_generic_arith(&mut dec, &mut cxs, &header).unwrap();
        assert_eq!(&out, bm, "mismatch with template={template} ext={ext} tpgdon={tpgdon}");
    }

    #[test]
    fn arith_round_trip_template0() {
        let bm = random_bitmap(33, 17, 0.08, 0x1234);
        round_trip_arith(0, false, false, &bm);
    }

    #[test]
    fn arith_round_trip_template1() {
        let bm = random_bitmap(40, 20, 0.05, 0x1);
        round_trip_arith(1, false, false, &bm);
    }

    #[test]
    fn arith_round_trip_template2() {
        let bm = random_bitmap(24, 24, 0.10, 0x2);
        round_trip_arith(2, false, false, &bm);
    }

    #[test]
    fn arith_round_trip_template3() {
        let bm = random_bitmap(60, 32, 0.07, 0x3);
        round_trip_arith(3, false, false, &bm);
    }

    #[test]
    fn arith_round_trip_extended_template() {
        let bm = random_bitmap(48, 24, 0.12, 0xAAAA);
        round_trip_arith(0, true, false, &bm);
    }

    #[test]
    fn arith_round_trip_tpgdon() {
        // Include some duplicate rows to exercise TPGD.
        let mut bm = random_bitmap(40, 20, 0.03, 0xbeef);
        for y in 1..20 {
            if y % 3 == 0 {
                let src = bm.row(y as usize - 1).to_vec();
                bm.row_mut(y as usize).copy_from_slice(&src);
            }
        }
        round_trip_arith(0, false, true, &bm);
    }

    #[cfg(feature = "mmr")]
    #[test]
    fn mmr_round_trip_small() {
        let bm = random_bitmap(40, 16, 0.1, 0xabc);
        let buf = encode_generic_mmr(&bm).unwrap();
        let back = decode_generic_mmr(&buf, bm.width(), bm.height()).unwrap();
        assert_eq!(&back, &bm);
    }
}
