//! Generic refinement region segment (spec 6.3 / 7.4.7).
//!
//! Refinement coding takes a reference bitmap and emits a template-coded
//! delta against it using the MQ coder. Unlike the generic region coder,
//! the context draws pixels from both the already-decoded portion of the
//! target bitmap *and* from a 3×3 neighbourhood in the reference bitmap,
//! which lets the adaptive probability estimator exploit the fact that
//! most refinement pixels agree with the reference.
//!
//! Two fixed templates are specified: `GRTEMPLATE = 0` uses a 13-pixel
//! context with two adaptive-template (AT) pixels (one in the target
//! neighbourhood, one in the reference neighbourhood), while
//! `GRTEMPLATE = 1` uses a smaller 10-pixel context with no AT pixels.
//! The typical-prediction flag `TPGRON` enables spec 6.3.5.6 per-pixel
//! prediction: for rows with `LTP = 1`, any pixel whose 3×3 reference
//! neighbourhood is uniform is reconstructed from that neighbourhood
//! without consulting the arithmetic coder at all.
//!
//! TODO(perf): the encoder still builds refinement contexts with per-pixel
//! `get_pixel` calls. If lossy symbol matching with refine-after-match becomes
//! a dominant workload, add a sliding-register fast path analogous to the
//! template-0 generic-region encoder.

use std::io::{Read, Write};

use crate::bitmap::Bitmap;
use crate::coding::mq::{MqContexts, MqDecoder, MqEncoder};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::{region_info::RegionInfo, AtPixels};

/// SLTP context index used when `TPGRON = 1`. The spec leaves the bit
/// assignment for the refinement template up to each implementation; we
/// mirror the choice made by the ITU-T T.88 reference encoder
/// (`Jb2_MQLapper.cpp::CX_RefEncode`), which is the codec used to
/// produce the official conformance suite. For template 0 the SLTP
/// context is `bit 4` set (the central reference pixel under that
/// layout), and for template 1 it is `bit 6` set.
const SLTP_CX_T0: usize = 0x0010;
const SLTP_CX_T1: usize = 0x0040;

/// Parsed refinement region header.
#[derive(Clone, Debug)]
pub struct RefinementRegionHeader {
    /// Region info (width/height/page coordinates).
    pub region: RegionInfo,
    /// `GRTEMPLATE` — 0 = 13-pixel context, 1 = 10-pixel context.
    pub template: u8,
    /// `TPGRON` — typical-prediction-for-generic-refinement.
    pub tpgron: bool,
    /// Adaptive template pixels (`GRAT0x/y` target + `GRAT1x/y` reference).
    /// Only meaningful for `template = 0`.
    pub at: AtPixels,
}

impl RefinementRegionHeader {
    /// Parse a refinement region header from a reader positioned at the
    /// region info field.
    pub fn read<R: Read>(r: &mut R) -> Jbig2Result<Self> {
        let region = RegionInfo::read(r)?;
        let mut fb = [0u8; 1];
        r.read_exact(&mut fb)?;
        let flags = fb[0];
        let template = flags & 0x01;
        let tpgron = (flags & 0x02) != 0;
        let mut at = [(0i8, 0i8); 2];
        if template == 0 {
            for i in 0..2 {
                let mut b = [0u8; 2];
                r.read_exact(&mut b)?;
                at[i] = (b[0] as i8, b[1] as i8);
            }
        }
        Ok(Self {
            region,
            template,
            tpgron,
            at: AtPixels::from_array(at),
        })
    }

    /// Emit the refinement region header.
    pub fn write<W: Write>(&self, w: &mut W) -> Jbig2Result<()> {
        self.region.write(w)?;
        let flags = (self.template & 0x01) | if self.tpgron { 0x02 } else { 0 };
        w.write_all(&[flags])?;
        if self.template == 0 {
            for i in 0..2 {
                w.write_all(&[self.at[i].0 as u8, self.at[i].1 as u8])?;
            }
        }
        Ok(())
    }
}

/// Default AT pixel positions for a refinement region. The spec nominal
/// placement is `(-1, -1)` for both the target-neighbourhood AT pixel
/// (index 0) and the reference-neighbourhood AT pixel (index 1).
pub const NOMINAL_REFINEMENT_AT: AtPixels = AtPixels::new(
    [
        (-1, -1),
        (-1, -1),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    2,
);

/// Decode a refinement region bitmap of size `width × height` against a
/// reference bitmap, using the arithmetic MQ coder.
///
/// `reference_dx` / `reference_dy` translate reference coordinates: the
/// reference pixel corresponding to target pixel `(x, y)` is at
/// `reference[x - reference_dx, y - reference_dy]`. For a standalone
/// refinement region this is typically `(0, 0)` (the reference bitmap is
/// the page bitmap masked to the region box).
///
/// Implements spec 6.3.5 for templates 0 and 1, with `TPGRON` (spec
/// 6.3.5.6) controlling per-row LTP state and per-pixel typical-prediction.
pub fn decode_refinement_region(
    dec: &mut MqDecoder<'_>,
    cxs: &mut MqContexts,
    width: u32,
    height: u32,
    template: u8,
    tpgron: bool,
    at: &[(i8, i8); 2],
    reference: &Bitmap,
    reference_dx: i32,
    reference_dy: i32,
) -> Jbig2Result<Bitmap> {
    if template > 1 {
        return Err(Jbig2Error::InvalidSegmentHeader(
            "generic refinement: unknown GRTEMPLATE",
        ));
    }
    let mut target = Bitmap::new(width, height)?;
    if width == 0 || height == 0 {
        return Ok(target);
    }
    let sltp_cx = if template == 0 {
        SLTP_CX_T0
    } else {
        SLTP_CX_T1
    };
    let mut ltp: u8 = 0;
    for y in 0..height as i32 {
        if tpgron {
            let sltp = dec.decode(cxs, sltp_cx);
            ltp ^= sltp;
        }
        for x in 0..width as i32 {
            let ref_x = x - reference_dx;
            let ref_y = y - reference_dy;
            if ltp == 1 {
                if let Some(v) = uniform_ref_neighbourhood(reference, ref_x, ref_y) {
                    target.set_pixel(x, y, v);
                    continue;
                }
            }
            let cx = build_refinement_ctx(template, &target, reference, x, y, ref_x, ref_y, at);
            let bit = dec.decode(cxs, cx);
            target.set_pixel(x, y, bit);
        }
    }
    Ok(target)
}

/// Encode `target` as a refinement of `reference` using the arithmetic MQ
/// coder. Mirror of [`decode_refinement_region`].
///
/// When `tpgron` is set, the encoder emits an SLTP bit per row and flips
/// `LTP` to `1` whenever doing so is lossless for that row (every pixel
/// whose 3×3 reference neighbourhood is uniform already matches the
/// uniform value). Predictable pixels are then skipped from the arithmetic
/// stream, which is where TPGR earns its bits.
pub fn encode_refinement_region(
    enc: &mut MqEncoder,
    cxs: &mut MqContexts,
    target: &Bitmap,
    template: u8,
    tpgron: bool,
    at: &[(i8, i8); 2],
    reference: &Bitmap,
    reference_dx: i32,
    reference_dy: i32,
) -> Jbig2Result<()> {
    if template > 1 {
        return Err(Jbig2Error::InvalidSegmentHeader(
            "generic refinement: unknown GRTEMPLATE",
        ));
    }
    let width = target.width();
    let height = target.height();
    if width == 0 || height == 0 {
        return Ok(());
    }
    let sltp_cx = if template == 0 {
        SLTP_CX_T0
    } else {
        SLTP_CX_T1
    };
    let mut ltp: u8 = 0;
    for y in 0..height as i32 {
        if tpgron {
            let row_ltp = row_typical_ok(target, reference, y, reference_dx, reference_dy);
            let desired_ltp: u8 = if row_ltp { 1 } else { 0 };
            let sltp = ltp ^ desired_ltp;
            enc.encode(cxs, sltp_cx, sltp);
            ltp = desired_ltp;
        }
        for x in 0..width as i32 {
            let ref_x = x - reference_dx;
            let ref_y = y - reference_dy;
            if ltp == 1 && uniform_ref_neighbourhood(reference, ref_x, ref_y).is_some() {
                continue;
            }
            let cx = build_refinement_ctx(template, target, reference, x, y, ref_x, ref_y, at);
            let bit = target.get_pixel(x, y);
            enc.encode(cxs, cx, bit);
        }
    }
    Ok(())
}

/// Build the refinement-region context word for target pixel `(x, y)`.
///
/// The bit assignment is implementation-defined by the spec (6.3.5.3
/// only requires it to be consistent across encoder and decoder). The T.88
/// conformance streams were authored by the ITU sample encoder, so this layout
/// mirrors `Jb2_MQLapper.cpp::CX_RefEncode`.
///
/// Template 0 (13-bit context):
///
/// ```text
///   bit 12 : ref(ref_x + GRAT1X, ref_y + GRAT1Y)   (default -1,-1)
///   bit 11 : target(x + GRAT0X, y + GRAT0Y)        (default -1,-1)
///   bit 10 : target(x,     y - 1)
///   bit  9 : target(x + 1, y - 1)
///   bit  8 : target(x - 1, y)
///   bit  7 : ref(ref_x + 0,       ref_y - 1)
///   bit  6 : ref(ref_x + 1,       ref_y - 1)
///   bit  5 : ref(ref_x - 1,       ref_y + 0)
///   bit  4 : ref(ref_x + 0,       ref_y + 0)
///   bit  3 : ref(ref_x + 1,       ref_y + 0)
///   bit  2 : ref(ref_x - 1,       ref_y - 1)
///   bit  1 : ref(ref_x + 0,       ref_y - 1)
///   bit  0 : ref(ref_x + 1,       ref_y - 1)
/// ```
///
/// Template 1 (10-bit context, no AT pixels):
///
/// ```text
///   bit  9 : target(x - 1, y - 1)
///   bit  8 : target(x,     y - 1)
///   bit  7 : target(x + 1, y - 1)
///   bit  6 : target(x - 1, y)
///   bit  5 : ref(ref_x + 0, ref_y - 1)
///   bit  4 : ref(ref_x - 1, ref_y + 0)
///   bit  3 : ref(ref_x + 0, ref_y + 0)
///   bit  2 : ref(ref_x + 1, ref_y + 0)
///   bit  1 : ref(ref_x + 0, ref_y + 1)
///   bit  0 : ref(ref_x + 1, ref_y + 1)
/// ```
#[inline]
fn build_refinement_ctx(
    template: u8,
    target: &Bitmap,
    reference: &Bitmap,
    x: i32,
    y: i32,
    ref_x: i32,
    ref_y: i32,
    at: &[(i8, i8); 2],
) -> usize {
    match template {
        0 => {
            let target_at = target.get_pixel(x + at[0].0 as i32, y + at[0].1 as i32) as u32;
            let ref_at = reference.get_pixel(ref_x + at[1].0 as i32, ref_y + at[1].1 as i32) as u32;
            let cx = (reference.get_pixel(ref_x + 1, ref_y - 1) as u32)
                | ((reference.get_pixel(ref_x, ref_y - 1) as u32) << 1)
                | ((reference.get_pixel(ref_x - 1, ref_y - 1) as u32) << 2)
                | ((reference.get_pixel(ref_x + 1, ref_y) as u32) << 3)
                | ((reference.get_pixel(ref_x, ref_y) as u32) << 4)
                | ((reference.get_pixel(ref_x - 1, ref_y) as u32) << 5)
                | ((reference.get_pixel(ref_x + 1, ref_y - 1) as u32) << 6)
                | ((reference.get_pixel(ref_x, ref_y - 1) as u32) << 7)
                | ((target.get_pixel(x - 1, y) as u32) << 8)
                | ((target.get_pixel(x + 1, y - 1) as u32) << 9)
                | ((target.get_pixel(x, y - 1) as u32) << 10)
                | (target_at << 11)
                | (ref_at << 12);
            cx as usize
        }
        1 => {
            let cx = (reference.get_pixel(ref_x + 1, ref_y + 1) as u32)
                | ((reference.get_pixel(ref_x, ref_y + 1) as u32) << 1)
                | ((reference.get_pixel(ref_x + 1, ref_y) as u32) << 2)
                | ((reference.get_pixel(ref_x, ref_y) as u32) << 3)
                | ((reference.get_pixel(ref_x - 1, ref_y) as u32) << 4)
                | ((reference.get_pixel(ref_x, ref_y - 1) as u32) << 5)
                | ((target.get_pixel(x - 1, y) as u32) << 6)
                | ((target.get_pixel(x + 1, y - 1) as u32) << 7)
                | ((target.get_pixel(x, y - 1) as u32) << 8)
                | ((target.get_pixel(x - 1, y - 1) as u32) << 9);
            cx as usize
        }
        _ => 0,
    }
}

/// Return `Some(v)` if every pixel of the 3×3 reference neighbourhood
/// centred at `(ref_x, ref_y)` has value `v`, else `None`. This is the
/// TPGR "uniform reference" test from spec 6.3.5.6.
#[inline]
fn uniform_ref_neighbourhood(reference: &Bitmap, ref_x: i32, ref_y: i32) -> Option<u8> {
    let v = reference.get_pixel(ref_x - 1, ref_y - 1);
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == -1 && dy == -1 {
                continue;
            }
            if reference.get_pixel(ref_x + dx, ref_y + dy) != v {
                return None;
            }
        }
    }
    Some(v)
}

/// Return `true` when encoding row `y` with `LTP = 1` is lossless — that
/// is, every pixel in the row whose 3×3 reference neighbourhood is
/// uniform already matches the uniform value.
fn row_typical_ok(
    target: &Bitmap,
    reference: &Bitmap,
    y: i32,
    reference_dx: i32,
    reference_dy: i32,
) -> bool {
    for x in 0..target.width() as i32 {
        let ref_x = x - reference_dx;
        let ref_y = y - reference_dy;
        if let Some(v) = uniform_ref_neighbourhood(reference, ref_x, ref_y) {
            if target.get_pixel(x, y) != v {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::mq::{MqContexts, MqDecoder, MqEncoder, MQ_NUM_CONTEXTS};
    use crate::segments::page_information::CombinationOp;

    #[test]
    fn header_round_trip_template0() {
        let hdr = RefinementRegionHeader {
            region: RegionInfo {
                width: 50,
                height: 20,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            template: 0,
            tpgron: false,
            at: NOMINAL_REFINEMENT_AT,
        };
        let mut buf = Vec::new();
        hdr.write(&mut buf).unwrap();
        let back = RefinementRegionHeader::read(&mut buf.as_slice()).unwrap();
        assert_eq!(back.template, 0);
        assert_eq!(back.at, hdr.at);
    }

    #[test]
    fn header_round_trip_template1() {
        let hdr = RefinementRegionHeader {
            region: RegionInfo {
                width: 50,
                height: 20,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            template: 1,
            tpgron: true,
            at: AtPixels::from_array([(0, 0); 2]),
        };
        let mut buf = Vec::new();
        hdr.write(&mut buf).unwrap();
        let back = RefinementRegionHeader::read(&mut buf.as_slice()).unwrap();
        assert_eq!(back.template, 1);
        assert!(back.tpgron);
    }

    /// Deterministically flip a handful of pixels in `bm` to obtain a
    /// distinct reference image that still shares most of its pixels with
    /// the target, which is the common case where refinement coding pays
    /// off.
    fn corrupt(bm: &Bitmap, seed: u64) -> Bitmap {
        let mut out = bm.clone();
        let w = bm.width() as i32;
        let h = bm.height() as i32;
        let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xDEAD_BEEF;
        for _ in 0..(w * h / 16).max(8) {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let x = (state as i32).rem_euclid(w);
            let y = (state.rotate_right(11) as i32).rem_euclid(h);
            let cur = out.get_pixel(x, y);
            out.set_pixel(x, y, cur ^ 1);
        }
        out
    }

    fn checkerboard(w: u32, h: u32) -> Bitmap {
        let mut bm = Bitmap::new(w, h).unwrap();
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let v = (((x / 3) ^ (y / 2)) & 1) as u8;
                bm.set_pixel(x, y, v);
            }
        }
        bm
    }

    fn round_trip_one(
        template: u8,
        tpgron: bool,
        at: AtPixels,
        target: &Bitmap,
        reference: &Bitmap,
        dx: i32,
        dy: i32,
    ) {
        let mut enc_cxs = MqContexts::new(MQ_NUM_CONTEXTS);
        let mut enc = MqEncoder::new(target.data().len());
        encode_refinement_region(
            &mut enc,
            &mut enc_cxs,
            target,
            template,
            tpgron,
            &at.to_array_2(),
            reference,
            dx,
            dy,
        )
        .unwrap();
        let coded = enc.finish();

        let mut dec_cxs = MqContexts::new(MQ_NUM_CONTEXTS);
        let mut dec = MqDecoder::new(&coded);
        let back = decode_refinement_region(
            &mut dec,
            &mut dec_cxs,
            target.width(),
            target.height(),
            template,
            tpgron,
            &at.to_array_2(),
            reference,
            dx,
            dy,
        )
        .unwrap();
        assert_eq!(&back, target);
    }

    #[test]
    fn refinement_round_trip_template0_nominal_at() {
        let target = checkerboard(37, 19);
        let reference = corrupt(&target, 1);
        round_trip_one(0, false, NOMINAL_REFINEMENT_AT, &target, &reference, 0, 0);
    }

    #[test]
    fn refinement_round_trip_template0_with_tpgr() {
        let target = checkerboard(37, 19);
        let reference = corrupt(&target, 2);
        round_trip_one(0, true, NOMINAL_REFINEMENT_AT, &target, &reference, 0, 0);
    }

    #[test]
    fn refinement_round_trip_template1() {
        let target = checkerboard(24, 24);
        let reference = corrupt(&target, 3);
        round_trip_one(1, false, NOMINAL_REFINEMENT_AT, &target, &reference, 0, 0);
    }

    #[test]
    fn refinement_round_trip_template1_tpgr() {
        let target = checkerboard(24, 24);
        let reference = corrupt(&target, 4);
        round_trip_one(1, true, NOMINAL_REFINEMENT_AT, &target, &reference, 0, 0);
    }

    #[test]
    fn refinement_round_trip_template0_non_nominal_at() {
        let target = checkerboard(40, 20);
        let reference = corrupt(&target, 5);
        // Push both AT pixels off their nominal positions to exercise the
        // override paths.
        let at = AtPixels::from_array([(-2, 0), (0, -1)]);
        round_trip_one(0, false, at, &target, &reference, 0, 0);
    }

    #[test]
    fn refinement_round_trip_with_offset() {
        let target = checkerboard(30, 18);
        // Make the reference a spatially shifted copy of the target.
        let mut reference = Bitmap::new(30, 18).unwrap();
        for y in 0..18i32 {
            for x in 0..30i32 {
                reference.set_pixel(x, y, target.get_pixel(x - 1, y - 1));
            }
        }
        round_trip_one(0, false, NOMINAL_REFINEMENT_AT, &target, &reference, -1, -1);
    }

    #[test]
    fn refinement_identity_reference() {
        // When target == reference and TPGRON is on, the coder should
        // predict every pixel with zero LPS events on the non-SLTP
        // contexts.
        let target = checkerboard(32, 12);
        let reference = target.clone();
        round_trip_one(0, true, NOMINAL_REFINEMENT_AT, &target, &reference, 0, 0);
    }
}
