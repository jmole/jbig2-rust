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
//!
//! # Hot path shape
//!
//! The arithmetic decoder is the single hottest loop in the crate, so the
//! implementation is deliberately layered:
//!
//! * [`decode_generic_bitmap`] owns the per-row setup: it grabs a reference
//!   to the previous two rows and a mutable slice to the row being built, so
//!   the inner loop never has to recompute row offsets.
//! * [`decode_row_t0_nominal`] and [`decode_row_t3_nominal`] are the
//!   nominal-AT fast paths for the two templates the crate's presets
//!   exercise most often. They maintain small sliding registers (one per
//!   row used by the template) and shift a new right-edge bit in per pixel
//!   instead of rebuilding the context from scratch, then paste the
//!   decoded pixel into the current row with a single OR into a packed
//!   byte.
//! * [`decode_row_generic`] is the fall-through for templates 1..3, the
//!   extended AMD2 template, and non-nominal AT pixel placements. It still
//!   works on row slices and uses direct byte reads via [`bitmap::read_bit`]
//!   rather than routing through [`Bitmap::get_pixel`], which keeps the
//!   overhead per template reference to a single masked byte load.
//!
//! The encoder side mirrors the same structure so that a change to the
//! context layout only has to be made once.
//!
//! # When is a second fast path worth it?
//!
//! A local decode microbenchmark decoded the same text-shaped 512×512 page
//! through every non-T0-nominal path. Measured on an Apple-silicon laptop
//! (`--release`, short-measurement profile):
//!
//! | Path                                 | Throughput   | vs T0 nominal |
//! |--------------------------------------|--------------|---------------|
//! | T0 nominal (arith_raw, no TPGD)      | ~38 MiB/s    | 1.0×          |
//! | T0 nominal (arith_raw, with TPGD)    | ~167 MiB/s   | 4.4×          |
//! | Template 1 nominal                   | ~18 MiB/s    | 0.47×         |
//! | Template 2 nominal                   | ~22 MiB/s    | 0.58×         |
//! | Template 3 nominal                   | ~20 MiB/s    | 0.53×         |
//! | Extended T0 (AMD2) nominal           | ~9 MiB/s     | 0.24×         |
//! | T0 with non-nominal AT               | ~14 MiB/s    | 0.37×         |
//!
//! Every non-T0-nominal path lands in the 2×–4× slower band, but that gap
//! only matters once a workload actually spends meaningful time there.
//! The shipped T.88 conformance corpus (TT9 arithmetic streams, TT10) is
//! template-0 nominal + TPGD, and every real-world encoder the crate
//! ships with — including `EncoderConfig::balanced()` /
//! `max_compression()` — picks template 0 at nominal AT. That makes the
//! fast path the overwhelmingly dominant path in practice.
//!
//! The follow-up plan therefore treats additional fast paths as
//! benchmark-driven work: we implement the T3 path because
//! [`crate::encoder::EncoderConfig::fast`] selects it, but T1/T2 and AMD2
//! remain deferred. The reopening criteria are:
//!
//! * An external workload shows >5% of total decode time in one of the
//!   remaining fallback rows (most likely Template 1/2 or AMD2), **or**
//! * A JBIG2 producer starts shipping files with AMD2 / non-nominal AT
//!   often enough to move the `arith_fallbacks` group above the
//!   `arith_raw` numbers on the files users actually decode.
//!
//! Until one of those is true, the cost of a second sliding-register row
//! decoder (new shift pipeline, new SLTP context handling, extra
//! cross-checks against `decode_row_generic`) is larger than the
//! throughput it would unlock.

use std::io::{Read, Write};

use crate::bitmap::{self, Bitmap};
use crate::coding::mmr::{encode_t6_line, row_to_bools, BitWriter as MmrBitWriter};
use crate::coding::mq::{MqContexts, MqDecoder, MqEncoder, MQ_NUM_CONTEXTS};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::{region_info::RegionInfo, AtPixels};

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
    pub at: AtPixels,
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

        let n_at = if mmr {
            0
        } else if template == 0 {
            if ext_template {
                12
            } else {
                4
            }
        } else {
            1
        };
        let mut at = [(0i8, 0i8); 12];
        if !mmr {
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
            at: AtPixels::new(at, n_at as u8),
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
                if self.ext_template {
                    12
                } else {
                    4
                }
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
pub fn nominal_at(template: u8, ext_template: bool) -> AtPixels {
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
    AtPixels::new(
        out,
        match (template, ext_template) {
            (0, false) => 4,
            (0, true) => 12,
            _ => 1,
        },
    )
}

/// Return `true` when `at` matches the spec-default AT positions for the
/// given template. The nominal-AT fast path can only be used when this is
/// true (AT pixels have to be at the well-known positions the sliding
/// registers are sized for).
#[inline]
fn at_is_nominal(template: u8, ext_template: bool, at: &AtPixels) -> bool {
    let n = match (template, ext_template) {
        (0, false) => 4,
        (0, true) => 12,
        _ => 1,
    };
    let nom = nominal_at(template, ext_template);
    at.as_slice()[..n] == nom.as_slice()[..n]
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
    at: &AtPixels,
) -> Jbig2Result<Bitmap> {
    let mut bitmap = Bitmap::new(width, height)?;
    if width == 0 || height == 0 {
        return Ok(bitmap);
    }
    let sltp_cx = SLTP_CX[template as usize];
    let stride = bitmap.stride();
    let w_signed = width as i32;
    // Sentinel zero-filled row used whenever y-1 or y-2 falls above the
    // region top; this lets the inner loops always address a real slice
    // of length `stride` rather than special-casing boundary rows.
    let zero_row = vec![0u8; stride];
    let nominal = at_is_nominal(template, ext_template, at);
    let mut ltp: u8 = 0;
    for y in 0..height as usize {
        if tpgdon {
            let sltp = dec.decode(cxs, sltp_cx as usize);
            ltp ^= sltp;
        }
        // TPGD duplicate-row shortcut: copy the previous row in packed form.
        if tpgdon && ltp == 1 {
            if y > 0 {
                let (above, cur) = bitmap.split_above(y);
                let prev = &above[(y - 1) * stride..y * stride];
                cur.copy_from_slice(prev);
            }
            continue;
        }
        let (above, cur) = bitmap.split_above(y);
        let p1: &[u8] = if y >= 1 {
            &above[(y - 1) * stride..y * stride]
        } else {
            &zero_row
        };
        let p2: &[u8] = if y >= 2 {
            &above[(y - 2) * stride..(y - 1) * stride]
        } else {
            &zero_row
        };
        match (template, ext_template, nominal) {
            (0, false, true) => decode_row_t0_nominal(dec, cxs, width, p2, p1, cur),
            (0, false, false) => decode_row_generic::<0, false>(dec, cxs, width, p2, p1, cur, at),
            (0, true, _) => decode_row_generic::<0, true>(dec, cxs, width, p2, p1, cur, at),
            (1, _, _) => decode_row_generic::<1, false>(dec, cxs, width, p2, p1, cur, at),
            (2, _, _) => decode_row_generic::<2, false>(dec, cxs, width, p2, p1, cur, at),
            (3, false, true) => decode_row_t3_nominal(dec, cxs, width, p1, cur),
            (3, _, _) => decode_row_generic::<3, false>(dec, cxs, width, p2, p1, cur, at),
            _ => return Err(Jbig2Error::InvalidSegmentHeader("unknown GB template")),
        }
        // Mask padding bits on the last row byte so downstream bitmap
        // comparisons don't see stale bits from the sliding registers.
        if (width & 7) != 0 {
            let tail = (width & 7) as u8;
            let mask = 0xFFu8 << (8 - tail);
            let last = cur.len() - 1;
            cur[last] &= mask;
        }
        let _ = w_signed;
    }
    Ok(bitmap)
}

/// Template-0 standard, nominal AT, fast-path decoder.
///
/// Maintains three small sliding registers that hold just the template
/// bits in the right relative positions and rebuilds the final 16-bit
/// context with a handful of shifts per pixel. This replaces 16
/// independent bounds-checked bit reads per pixel with three masked byte
/// loads, which is where most of the arithmetic-decode speedup comes
/// from.
#[inline(always)]
fn decode_row_t0_nominal(
    dec: &mut MqDecoder<'_>,
    cxs: &mut MqContexts,
    width: u32,
    p2: &[u8],
    p1: &[u8],
    cur: &mut [u8],
) {
    let w = width as i32;
    let mut p2_bits = bitmap::RowBitCursor::new(p2, width);
    let mut p1_bits = bitmap::RowBitCursor::new(p1, width);
    let mut cur_bits = bitmap::RowBitBuffer::new(cur, width);
    // Pre-fill the right edges of the two upper-row registers so that the
    // register content at x=0 already holds pix(0,y-2), pix(1,y-2),
    // pix(2,y-2) and pix(0..3, y-1) in the positions required by
    // `compose_ctx_t0`.
    let mut cx_y2: u32 = (p2_bits.next_bit() << 2) | (p2_bits.next_bit() << 1) | p2_bits.next_bit();
    let mut cx_y1: u32 = (p1_bits.next_bit() << 3)
        | (p1_bits.next_bit() << 2)
        | (p1_bits.next_bit() << 1)
        | p1_bits.next_bit();
    let mut cx_y0: u32 = 0;
    for _x in 0..w {
        let cx = compose_ctx_t0(cx_y2, cx_y1, cx_y0);
        let d = dec.decode(cxs, cx as usize) as u32;
        cur_bits.push_bit(d);
        cx_y2 = ((cx_y2 << 1) | p2_bits.next_bit()) & 0x1F;
        cx_y1 = ((cx_y1 << 1) | p1_bits.next_bit()) & 0x7F;
        cx_y0 = ((cx_y0 << 1) | d) & 0xF;
    }
    cur_bits.finish();
}

/// Template-3 standard, nominal AT, fast-path decoder.
///
/// T3 uses a 10-bit context spanning one upper row plus the four already
/// decoded bits on the current row. The nominal AT pixel sits at `(x+2, y-1)`,
/// so a single 6-bit sliding window over `p1` holds `b8..b4` plus `a1`.
#[inline(always)]
fn decode_row_t3_nominal(
    dec: &mut MqDecoder<'_>,
    cxs: &mut MqContexts,
    width: u32,
    p1: &[u8],
    cur: &mut [u8],
) {
    let w = width as i32;
    let mut p1_bits = bitmap::RowBitCursor::new(p1, width);
    let mut cur_bits = bitmap::RowBitBuffer::new(cur, width);
    let mut cx_y1: u32 = (p1_bits.next_bit() << 2) | (p1_bits.next_bit() << 1) | p1_bits.next_bit();
    let mut cx_y0: u32 = 0;
    for _x in 0..w {
        let cx = compose_ctx_t3(cx_y1, cx_y0);
        let d = dec.decode(cxs, cx as usize) as u32;
        cur_bits.push_bit(d);
        cx_y1 = ((cx_y1 << 1) | p1_bits.next_bit()) & 0x3F;
        cx_y0 = ((cx_y0 << 1) | d) & 0xF;
    }
    cur_bits.finish();
}

/// Compose the 16-bit GB template-0 context from the three sliding
/// registers used by [`decode_row_t0_nominal`].
///
/// Bit layout produced here is identical to what `build_gb_context_slow`
/// returns for the same pixels, and the two are cross-validated by
/// `tests::fast_path_matches_slow_path` below.
#[inline(always)]
fn compose_ctx_t0(y2: u32, y1: u32, y0: u32) -> u32 {
    // y2 bits: [4]=a4(x-2,y-2) [3]=b11 [2]=b10 [1]=b9 [0]=a3(x+2,y-2)
    // y1 bits: [6]=a2 [5]=b8 [4]=b7 [3]=b6 [2]=b5 [1]=b4 [0]=a1
    // y0 bits: [3]=b3 [2]=b2 [1]=b1 [0]=b0
    ((y2 & 0x10) << 11)   // a4 -> C[15]
        | ((y2 & 0x01) << 14) // a3 -> C[14]
        | ((y1 & 0x40) << 7)  // a2 -> C[13]
        | ((y1 & 0x01) << 12) // a1 -> C[12]
        | ((y2 & 0x0E) << 8)  // b11..b9 -> C[11..9]
        | ((y1 & 0x3E) << 3)  // b8..b4 -> C[8..4]
        | y0 // b3..b0 -> C[3..0]
}

/// Compose the 10-bit GB template-3 context from the two sliding
/// registers used by [`decode_row_t3_nominal`].
#[inline(always)]
fn compose_ctx_t3(y1: u32, y0: u32) -> u32 {
    ((y1 & 0x01) << 9) | ((y1 & 0x3E) << 3) | y0
}

/// Fall-through decoder for templates 1..3, the extended AMD2 template,
/// and non-nominal AT placements. Still uses row-local slices and direct
/// byte reads, so it is significantly faster than the original
/// `get_pixel`-based implementation, but it does not shortcut the full
/// context build.
#[inline(always)]
fn decode_row_generic<const T: u8, const EXT: bool>(
    dec: &mut MqDecoder<'_>,
    cxs: &mut MqContexts,
    width: u32,
    p2: &[u8],
    p1: &[u8],
    cur: &mut [u8],
    at: &AtPixels,
) {
    let w = width as i32;
    for x in 0..w {
        let cx = build_ctx_from_rows::<T, EXT>(p2, p1, cur, x, w, at);
        let d = dec.decode(cxs, cx as usize);
        if d != 0 {
            let idx = (x as usize) >> 3;
            let shift = 7 - (x as u32 & 7);
            cur[idx] |= 1u8 << shift;
        }
    }
}

/// Row-slice-based context builder. Reads the needed template pixels
/// directly from the packed bytes of the rows and the portion of the
/// current row that has already been decoded. Used by both the decoder
/// and the encoder.
#[inline(always)]
fn build_ctx_from_rows<const T: u8, const EXT: bool>(
    p2: &[u8],
    p1: &[u8],
    cur: &[u8],
    x: i32,
    w: i32,
    at: &AtPixels,
) -> u32 {
    #[inline(always)]
    fn at_bit(row_m1: &[u8], row_m2: &[u8], cur: &[u8], x: i32, dx: i8, dy: i8, w: i32) -> u32 {
        match dy {
            0 => bitmap::read_bit(cur, x + dx as i32, w),
            -1 => bitmap::read_bit(row_m1, x + dx as i32, w),
            -2 => bitmap::read_bit(row_m2, x + dx as i32, w),
            _ => 0,
        }
    }
    match (T, EXT) {
        (0, false) => {
            let b11 = bitmap::read_bit(p2, x - 1, w);
            let b10 = bitmap::read_bit(p2, x, w);
            let b9 = bitmap::read_bit(p2, x + 1, w);
            let b8 = bitmap::read_bit(p1, x - 2, w);
            let b7 = bitmap::read_bit(p1, x - 1, w);
            let b6 = bitmap::read_bit(p1, x, w);
            let b5 = bitmap::read_bit(p1, x + 1, w);
            let b4 = bitmap::read_bit(p1, x + 2, w);
            let b3 = bitmap::read_bit(cur, x - 4, w);
            let b2 = bitmap::read_bit(cur, x - 3, w);
            let b1 = bitmap::read_bit(cur, x - 2, w);
            let b0 = bitmap::read_bit(cur, x - 1, w);
            let a1 = at_bit(p1, p2, cur, x, at[0].0, at[0].1, w);
            let a2 = at_bit(p1, p2, cur, x, at[1].0, at[1].1, w);
            let a3 = at_bit(p1, p2, cur, x, at[2].0, at[2].1, w);
            let a4 = at_bit(p1, p2, cur, x, at[3].0, at[3].1, w);
            (a4 << 15)
                | (a3 << 14)
                | (a2 << 13)
                | (a1 << 12)
                | (b11 << 11)
                | (b10 << 10)
                | (b9 << 9)
                | (b8 << 8)
                | (b7 << 7)
                | (b6 << 6)
                | (b5 << 5)
                | (b4 << 4)
                | (b3 << 3)
                | (b2 << 2)
                | (b1 << 1)
                | b0
        }
        (0, true) => {
            // Spec 6.2.5.4 / Figure 3(b) (verified against the
            // reference encoder in Jb2_MQLapper.cpp::CX_Encode):
            //   B3 = p1[x - 1]
            //   B2 = p1[x]
            //   B1 = p1[x + 1]
            //   B0 = cur[x - 1]
            //   A_i (i = 1..12) at bits 4..15
            let b3 = bitmap::read_bit(p1, x - 1, w);
            let b2 = bitmap::read_bit(p1, x, w);
            let b1 = bitmap::read_bit(p1, x + 1, w);
            let b0 = bitmap::read_bit(cur, x - 1, w);
            let mut cx: u32 = 0;
            let mut i = 12;
            while i > 0 {
                i -= 1;
                let a = at_bit(p1, p2, cur, x, at[i].0, at[i].1, w);
                cx = (cx << 1) | a;
            }
            (cx << 4) | (b3 << 3) | (b2 << 2) | (b1 << 1) | b0
        }
        (1, _) => {
            let b11 = bitmap::read_bit(p2, x - 1, w);
            let b10 = bitmap::read_bit(p2, x, w);
            let b9 = bitmap::read_bit(p2, x + 1, w);
            let b8 = bitmap::read_bit(p2, x + 2, w);
            let b7 = bitmap::read_bit(p1, x - 2, w);
            let b6 = bitmap::read_bit(p1, x - 1, w);
            let b5 = bitmap::read_bit(p1, x, w);
            let b4 = bitmap::read_bit(p1, x + 1, w);
            let b3 = bitmap::read_bit(p1, x + 2, w);
            let b2 = bitmap::read_bit(cur, x - 3, w);
            let b1 = bitmap::read_bit(cur, x - 2, w);
            let b0 = bitmap::read_bit(cur, x - 1, w);
            let a1 = at_bit(p1, p2, cur, x, at[0].0, at[0].1, w);
            (a1 << 12)
                | (b11 << 11)
                | (b10 << 10)
                | (b9 << 9)
                | (b8 << 8)
                | (b7 << 7)
                | (b6 << 6)
                | (b5 << 5)
                | (b4 << 4)
                | (b3 << 3)
                | (b2 << 2)
                | (b1 << 1)
                | b0
        }
        (2, _) => {
            let b8 = bitmap::read_bit(p2, x - 1, w);
            let b7 = bitmap::read_bit(p2, x, w);
            let b6 = bitmap::read_bit(p2, x + 1, w);
            let b5 = bitmap::read_bit(p1, x - 2, w);
            let b4 = bitmap::read_bit(p1, x - 1, w);
            let b3 = bitmap::read_bit(p1, x, w);
            let b2 = bitmap::read_bit(p1, x + 1, w);
            let b1 = bitmap::read_bit(cur, x - 2, w);
            let b0 = bitmap::read_bit(cur, x - 1, w);
            let a1 = at_bit(p1, p2, cur, x, at[0].0, at[0].1, w);
            (a1 << 9)
                | (b8 << 8)
                | (b7 << 7)
                | (b6 << 6)
                | (b5 << 5)
                | (b4 << 4)
                | (b3 << 3)
                | (b2 << 2)
                | (b1 << 1)
                | b0
        }
        (3, _) => {
            let b8 = bitmap::read_bit(p1, x - 3, w);
            let b7 = bitmap::read_bit(p1, x - 2, w);
            let b6 = bitmap::read_bit(p1, x - 1, w);
            let b5 = bitmap::read_bit(p1, x, w);
            let b4 = bitmap::read_bit(p1, x + 1, w);
            let b3 = bitmap::read_bit(cur, x - 4, w);
            let b2 = bitmap::read_bit(cur, x - 3, w);
            let b1 = bitmap::read_bit(cur, x - 2, w);
            let b0 = bitmap::read_bit(cur, x - 1, w);
            let a1 = at_bit(p1, p2, cur, x, at[0].0, at[0].1, w);
            (a1 << 9)
                | (b8 << 8)
                | (b7 << 7)
                | (b6 << 6)
                | (b5 << 5)
                | (b4 << 4)
                | (b3 << 3)
                | (b2 << 2)
                | (b1 << 1)
                | b0
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
    at: &AtPixels,
) -> Jbig2Result<()> {
    let width = bitmap.width();
    let height = bitmap.height();
    if width == 0 || height == 0 {
        return Ok(());
    }
    let stride = bitmap.stride();
    let sltp_cx = SLTP_CX[template as usize];
    let nominal = at_is_nominal(template, ext_template, at);
    let zero_row = vec![0u8; stride];
    let w = width as i32;
    let mut ltp: u8 = 0;
    for y in 0..height as usize {
        let typical = tpgdon && y > 0 && is_typical(bitmap, y as u32);
        if tpgdon {
            let sltp = if typical { ltp ^ 1 } else { ltp };
            enc.encode(cxs, sltp_cx as usize, sltp);
            ltp ^= sltp;
        }
        if tpgdon && ltp == 1 {
            continue;
        }
        let cur = bitmap.row(y);
        let p1: &[u8] = if y >= 1 { bitmap.row(y - 1) } else { &zero_row };
        let p2: &[u8] = if y >= 2 { bitmap.row(y - 2) } else { &zero_row };
        match (template, ext_template, nominal) {
            (0, false, true) => encode_row_t0_nominal(enc, cxs, width, p2, p1, cur),
            (0, false, false) => {
                encode_row_generic::<0, false>(enc, cxs, w, p2, p1, cur, at);
            }
            (0, true, _) => {
                encode_row_generic::<0, true>(enc, cxs, w, p2, p1, cur, at);
            }
            (1, _, _) => encode_row_generic::<1, false>(enc, cxs, w, p2, p1, cur, at),
            (2, _, _) => encode_row_generic::<2, false>(enc, cxs, w, p2, p1, cur, at),
            (3, false, true) => encode_row_t3_nominal(enc, cxs, width, p1, cur),
            (3, _, _) => encode_row_generic::<3, false>(enc, cxs, w, p2, p1, cur, at),
            _ => return Err(Jbig2Error::InvalidSegmentHeader("unknown GB template")),
        }
    }
    Ok(())
}

#[inline(always)]
fn encode_row_t0_nominal(
    enc: &mut MqEncoder,
    cxs: &mut MqContexts,
    width: u32,
    p2: &[u8],
    p1: &[u8],
    cur: &[u8],
) {
    let w = width as i32;
    let mut p2_bits = bitmap::RowBitCursor::new(p2, width);
    let mut p1_bits = bitmap::RowBitCursor::new(p1, width);
    let mut cur_bits = bitmap::RowBitCursor::new(cur, width);
    let mut cx_y2: u32 = (p2_bits.next_bit() << 2) | (p2_bits.next_bit() << 1) | p2_bits.next_bit();
    let mut cx_y1: u32 = (p1_bits.next_bit() << 3)
        | (p1_bits.next_bit() << 2)
        | (p1_bits.next_bit() << 1)
        | p1_bits.next_bit();
    let mut cx_y0: u32 = 0;
    for _x in 0..w {
        let cx = compose_ctx_t0(cx_y2, cx_y1, cx_y0);
        let bit = cur_bits.next_bit();
        enc.encode(cxs, cx as usize, bit as u8);
        cx_y2 = ((cx_y2 << 1) | p2_bits.next_bit()) & 0x1F;
        cx_y1 = ((cx_y1 << 1) | p1_bits.next_bit()) & 0x7F;
        cx_y0 = ((cx_y0 << 1) | bit) & 0xF;
    }
}

#[inline(always)]
fn encode_row_t3_nominal(
    enc: &mut MqEncoder,
    cxs: &mut MqContexts,
    width: u32,
    p1: &[u8],
    cur: &[u8],
) {
    let w = width as i32;
    let mut p1_bits = bitmap::RowBitCursor::new(p1, width);
    let mut cur_bits = bitmap::RowBitCursor::new(cur, width);
    let mut cx_y1: u32 = (p1_bits.next_bit() << 2) | (p1_bits.next_bit() << 1) | p1_bits.next_bit();
    let mut cx_y0: u32 = 0;
    for _x in 0..w {
        let cx = compose_ctx_t3(cx_y1, cx_y0);
        let bit = cur_bits.next_bit();
        enc.encode(cxs, cx as usize, bit as u8);
        cx_y1 = ((cx_y1 << 1) | p1_bits.next_bit()) & 0x3F;
        cx_y0 = ((cx_y0 << 1) | bit) & 0xF;
    }
}

#[inline(always)]
fn encode_row_generic<const T: u8, const EXT: bool>(
    enc: &mut MqEncoder,
    cxs: &mut MqContexts,
    w: i32,
    p2: &[u8],
    p1: &[u8],
    cur: &[u8],
    at: &AtPixels,
) {
    for x in 0..w {
        let cx = build_ctx_from_rows::<T, EXT>(p2, p1, cur, x, w, at);
        let bit = bitmap::read_bit(cur, x, w);
        enc.encode(cxs, cx as usize, bit as u8);
    }
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
///
/// Uses the LUT-driven packed-row fast path in
/// [`crate::coding::mmr_lut`], which decodes run/control codes via 13-bit
/// and 7-bit lookup tables and writes directly into the destination
/// bitmap without the intermediate `Vec<bool>` round-trips that the
/// slow reference path performs. The slow path in
/// [`crate::coding::mmr`] is retained as a correctness baseline and is
/// exercised by the cross-validation tests in
/// [`crate::coding::mmr_lut::tests`].
#[cfg(feature = "mmr")]
pub fn decode_generic_mmr(data: &[u8], width: u32, height: u32) -> Jbig2Result<Bitmap> {
    let mut buf = crate::coding::mmr_lut::MmrBitBuf::new(data);
    decode_generic_mmr_streaming(&mut buf, width, height)
}

/// Decode an MMR-coded generic region from an existing [`MmrBitBuf`]
/// reader. Useful when several MMR images are concatenated back-to-back
/// (without intervening EOFB markers) inside a larger stream — for
/// example, the per-symbol bitmaps of a Huffman-coded symbol dictionary
/// height class with `BMSIZE > 0`.
#[cfg(feature = "mmr")]
pub fn decode_generic_mmr_streaming(
    buf: &mut crate::coding::mmr_lut::MmrBitBuf<'_>,
    width: u32,
    height: u32,
) -> Jbig2Result<Bitmap> {
    let mut bm = Bitmap::new(width, height)?;
    if width == 0 || height == 0 {
        return Ok(bm);
    }
    let stride = bm.stride();
    // For the very first line there is no previous row, so supply an
    // all-zero scratch buffer. From row 1 onward we can split the
    // bitmap and point the decoder directly at the packed rows; this
    // avoids copying decoded output back into the bitmap on every line.
    let zero_prev = vec![0u8; stride];
    {
        let first = bm.row_mut(0);
        crate::coding::mmr_lut::decode_t6_line_packed(buf, &zero_prev, first, width)?;
    }
    for y in 1..height as usize {
        let (above, cur) = bm.split_above(y);
        let prev = &above[(y - 1) * stride..y * stride];
        crate::coding::mmr_lut::decode_t6_line_packed(buf, prev, cur, width)?;
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
                external_combination_op: crate::segments::page_information::CombinationOp::Or,
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
        assert_eq!(
            &out, bm,
            "mismatch with template={template} ext={ext} tpgdon={tpgdon}"
        );
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

    /// The nominal-AT fast path and the generic-AT fall-through are required
    /// to produce identical contexts for every pixel. Pick a synthetic
    /// bitmap, run both context builders on every position, and assert
    /// equality so refactors of either path can't silently drift apart.
    #[test]
    fn fast_path_matches_slow_path() {
        let bm = random_bitmap(80, 24, 0.35, 0xc0de);
        let w = bm.width() as i32;
        let at = nominal_at(0, false);
        let zero = vec![0u8; bm.stride()];
        for y in 0..bm.height() as usize {
            let p1 = if y >= 1 { bm.row(y - 1) } else { &zero[..] };
            let p2 = if y >= 2 { bm.row(y - 2) } else { &zero[..] };
            let cur = bm.row(y);
            let mut cx_y2: u32 = (bitmap::read_bit(p2, 0, w) << 2)
                | (bitmap::read_bit(p2, 1, w) << 1)
                | bitmap::read_bit(p2, 2, w);
            let mut cx_y1: u32 = (bitmap::read_bit(p1, 0, w) << 3)
                | (bitmap::read_bit(p1, 1, w) << 2)
                | (bitmap::read_bit(p1, 2, w) << 1)
                | bitmap::read_bit(p1, 3, w);
            let mut cx_y0: u32 = 0;
            for x in 0..w {
                let got = compose_ctx_t0(cx_y2, cx_y1, cx_y0);
                let want = build_ctx_from_rows::<0, false>(p2, p1, cur, x, w, &at);
                assert_eq!(
                    got, want,
                    "template-0 context mismatch at (x={x}, y={y}): got=0x{got:04x} want=0x{want:04x}"
                );
                let d = bitmap::read_bit(cur, x, w);
                cx_y2 = ((cx_y2 << 1) | bitmap::read_bit(p2, x + 3, w)) & 0x1F;
                cx_y1 = ((cx_y1 << 1) | bitmap::read_bit(p1, x + 4, w)) & 0x7F;
                cx_y0 = ((cx_y0 << 1) | d) & 0xF;
            }
        }
    }

    #[test]
    fn fast_path_matches_slow_path_t3() {
        let bm = random_bitmap(80, 24, 0.35, 0xd00d);
        let w = bm.width() as i32;
        let at = nominal_at(3, false);
        let zero = vec![0u8; bm.stride()];
        for y in 0..bm.height() as usize {
            let p1 = if y >= 1 { bm.row(y - 1) } else { &zero[..] };
            let cur = bm.row(y);
            let mut cx_y1: u32 = (bitmap::read_bit(p1, 0, w) << 2)
                | (bitmap::read_bit(p1, 1, w) << 1)
                | bitmap::read_bit(p1, 2, w);
            let mut cx_y0: u32 = 0;
            for x in 0..w {
                let got = compose_ctx_t3(cx_y1, cx_y0);
                let want = build_ctx_from_rows::<3, false>(&zero, p1, cur, x, w, &at);
                assert_eq!(
                    got, want,
                    "template-3 context mismatch at (x={x}, y={y}): got=0x{got:04x} want=0x{want:04x}"
                );
                let d = bitmap::read_bit(cur, x, w);
                cx_y1 = ((cx_y1 << 1) | bitmap::read_bit(p1, x + 3, w)) & 0x3F;
                cx_y0 = ((cx_y0 << 1) | d) & 0xF;
            }
        }
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
