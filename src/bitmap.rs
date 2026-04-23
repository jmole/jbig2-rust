//! Row-packed bi-level bitmap used throughout JBIG2 encode/decode.
//!
//! Pixels are stored one bit per pixel, packed MSB-first within each byte, with
//! one row per `stride` bytes. Pixel value `1` is "ink on" (black on white
//! paper) unless a page's default-pixel flag inverts the interpretation.
//!
//! The layout is chosen so that
//!
//! * a row of `W` pixels occupies `ceil(W / 8)` bytes,
//! * consecutive rows are separated by a constant `stride`,
//! * we can take u64 views over a row by reading four bytes at a time into the
//!   low half of a register, which is the hot loop shape used by the generic
//!   region decoder.
//!
//! All geometry operations are done in terms of signed `i32` inside the JBIG2
//! spec; we mirror that here to avoid accidental unsigned wraps when computing
//! neighbourhood offsets.

use crate::error::{Jbig2Error, Jbig2Result};

/// Bitmap combination operator, mirrored onto the packed blitter.
///
/// Kept here rather than in `segments::page_information` so
/// [`Bitmap::composite`] can live in the same module as the packed data it
/// operates on. The [`From`] impls below bridge the `CombinationOp` type
/// the segment code uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlitOp {
    /// Destination := destination OR source (the default for page assembly
    /// and text-region symbol placement).
    Or,
    /// Destination := destination AND source.
    And,
    /// Destination := destination XOR source.
    Xor,
    /// Destination := NOT(destination XOR source) (aka XNOR).
    XNor,
    /// Destination := source (source overwrites destination).
    Replace,
}

/// A 1-bit-per-pixel bitmap with row-packed storage (MSB first).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitmap {
    width: u32,
    height: u32,
    stride: usize,
    data: Vec<u8>,
}

impl Bitmap {
    /// Create a new empty (all-zero) bitmap of the given size.
    ///
    /// Rows are padded to a whole number of bytes; unused trailing bits on
    /// each row are kept zero.
    pub fn new(width: u32, height: u32) -> Jbig2Result<Self> {
        let stride = stride_for(width)?;
        let len = stride
            .checked_mul(height as usize)
            .ok_or(Jbig2Error::InvalidGeometry("bitmap too large"))?;
        Ok(Self {
            width,
            height,
            stride,
            data: vec![0u8; len],
        })
    }

    /// Create a bitmap filled with a given pixel value (0 or 1).
    pub fn filled(width: u32, height: u32, value: u8) -> Jbig2Result<Self> {
        let mut bm = Self::new(width, height)?;
        if value != 0 {
            let tail_bits = (width % 8) as u8;
            let tail_mask = if tail_bits == 0 {
                0xFF
            } else {
                0xFFu8 << (8 - tail_bits)
            };
            for row in 0..height as usize {
                let r = bm.row_mut(row);
                for b in r.iter_mut() {
                    *b = 0xFF;
                }
                if tail_mask != 0xFF {
                    let last = r.len() - 1;
                    r[last] = tail_mask;
                }
            }
        }
        Ok(bm)
    }

    /// Construct a bitmap from an already-packed buffer. `stride` must be at
    /// least `ceil(width / 8)` and `data.len()` must equal
    /// `stride * height`.
    pub fn from_packed(
        width: u32,
        height: u32,
        stride: usize,
        data: Vec<u8>,
    ) -> Jbig2Result<Self> {
        let min_stride = stride_for(width)?;
        if stride < min_stride {
            return Err(Jbig2Error::InvalidGeometry("stride too small"));
        }
        let expected = stride
            .checked_mul(height as usize)
            .ok_or(Jbig2Error::InvalidGeometry("bitmap too large"))?;
        if data.len() != expected {
            return Err(Jbig2Error::InvalidGeometry("data length != stride * height"));
        }
        Ok(Self {
            width,
            height,
            stride,
            data,
        })
    }

    /// Width in pixels.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Number of bytes per row (>= ceil(width/8)).
    #[inline]
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// Borrow the raw packed buffer.
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Borrow the raw packed buffer mutably.
    #[inline]
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Borrow one packed row.
    #[inline]
    pub fn row(&self, y: usize) -> &[u8] {
        let start = y * self.stride;
        &self.data[start..start + self.stride]
    }

    /// Mutably borrow one packed row.
    #[inline]
    pub fn row_mut(&mut self, y: usize) -> &mut [u8] {
        let start = y * self.stride;
        &mut self.data[start..start + self.stride]
    }

    /// Read a single pixel (0 or 1). Returns 0 for out-of-bounds coordinates —
    /// this is the convention the JBIG2 spec uses for template pixels outside
    /// the region.
    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || (x as u32) >= self.width || (y as u32) >= self.height {
            return 0;
        }
        let byte = self.data[(y as usize) * self.stride + (x as usize >> 3)];
        (byte >> (7 - (x & 7))) & 1
    }

    /// Write a single pixel. Silently ignores out-of-bounds writes.
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, v: u8) {
        if x < 0 || y < 0 || (x as u32) >= self.width || (y as u32) >= self.height {
            return;
        }
        let idx = (y as usize) * self.stride + (x as usize >> 3);
        let mask = 1u8 << (7 - (x & 7));
        if v != 0 {
            self.data[idx] |= mask;
        } else {
            self.data[idx] &= !mask;
        }
    }

    /// Zero every pixel.
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    /// Grow the bitmap's height in place, filling the newly added rows with
    /// `fill` (0 or 1). Width, stride, and existing pixel data are preserved
    /// by reusing the backing `Vec<u8>`: the unknown-height page path calls
    /// this once per progressive segment, so avoiding the per-row scratch
    /// `Vec`/`copy_from_slice` pair that a fresh-`Bitmap` + copy implementation
    /// incurs materially cuts page-growth cost on long streams.
    ///
    /// Shrinking is not supported (pass the same height or larger). Returns
    /// an error if `new_height` would overflow `stride * height`.
    pub fn grow_height(&mut self, new_height: u32, fill: u8) -> Jbig2Result<()> {
        if new_height <= self.height {
            return Ok(());
        }
        let added = new_height as usize - self.height as usize;
        let added_bytes = self
            .stride
            .checked_mul(added)
            .ok_or(Jbig2Error::InvalidGeometry("bitmap too large"))?;
        let new_len = self
            .data
            .len()
            .checked_add(added_bytes)
            .ok_or(Jbig2Error::InvalidGeometry("bitmap too large"))?;
        self.data.reserve_exact(added_bytes);

        if fill == 0 {
            self.data.resize(new_len, 0);
        } else {
            // Match `Bitmap::filled`: fill every pixel of each added row but
            // keep the trailing sub-byte bits zero so padding stays canonical.
            let tail_bits = (self.width % 8) as u8;
            let tail_mask = if tail_bits == 0 {
                0xFF
            } else {
                0xFFu8 << (8 - tail_bits)
            };
            self.data.resize(new_len, 0xFF);
            if tail_mask != 0xFF {
                let first_added = self.height as usize;
                for y in first_added..new_height as usize {
                    let row_end = y * self.stride + self.stride;
                    self.data[row_end - 1] = tail_mask;
                }
            }
        }
        self.height = new_height;
        Ok(())
    }

    /// Composite `src` onto `self` at pixel position `(dst_x, dst_y)` using
    /// `op`. This is the packed-row fast path used by both page-level
    /// region compositing and text-region symbol placement.
    ///
    /// Source and destination bounds are clipped so callers don't have to
    /// pre-check geometry: any portion of the source that falls outside
    /// `self` is silently ignored.
    pub fn composite(&mut self, src: &Bitmap, dst_x: i32, dst_y: i32, op: BlitOp) {
        let dst_w = self.width as i32;
        let dst_h = self.height as i32;
        let src_w = src.width as i32;
        let src_h = src.height as i32;

        let x_start = dst_x.max(0);
        let y_start = dst_y.max(0);
        let x_end = (dst_x + src_w).min(dst_w);
        let y_end = (dst_y + src_h).min(dst_h);
        if x_start >= x_end || y_start >= y_end {
            return;
        }
        let src_x_off = (x_start - dst_x) as usize;
        let src_y_off = (y_start - dst_y) as usize;
        let copy_w = (x_end - x_start) as usize;
        let copy_h = (y_end - y_start) as usize;

        let dst_stride = self.stride;
        let src_stride = src.stride;
        let dst_start_bit = x_start as usize;
        let src_start_bit = src_x_off;

        for row in 0..copy_h {
            let dst_row_base = (y_start as usize + row) * dst_stride;
            let src_row_base = (src_y_off + row) * src_stride;
            composite_row(
                &mut self.data[dst_row_base..dst_row_base + dst_stride],
                dst_start_bit,
                &src.data[src_row_base..src_row_base + src_stride],
                src_start_bit,
                copy_w,
                op,
            );
        }
    }

    /// Borrow the raw packed buffer split at `y * stride`.
    ///
    /// Returns `(above, &row_and_below)` so the caller can keep shared
    /// references to rows `0..y` while mutating row `y`. This is the
    /// primary hot-path accessor used by the generic-region decoder, which
    /// needs to consult rows `y-2` and `y-1` while building row `y`.
    #[inline]
    pub fn split_above(&mut self, y: usize) -> (&[u8], &mut [u8]) {
        let pivot = y * self.stride;
        let (above, here) = self.data.split_at_mut(pivot);
        let cur = &mut here[..self.stride];
        (above, cur)
    }

    /// Compute an FxHash-style 64-bit hash of the packed pixel data, without
    /// allocating. This is the primary key used by the lossless symbol
    /// dictionary to deduplicate exact matches.
    pub fn pixel_hash(&self) -> u64 {
        // FxHash-like mixing — not cryptographic but fast and well-distributed
        // for short byte sequences.
        const K: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        let mut h: u64 = self.width as u64;
        h = h.rotate_left(5).wrapping_mul(K) ^ self.height as u64;
        for &b in &self.data {
            h = h.rotate_left(5).wrapping_mul(K) ^ b as u64;
        }
        h
    }
}

/// Compute the minimum stride (in bytes) needed to hold `width` pixels.
#[inline]
pub fn stride_for(width: u32) -> Jbig2Result<usize> {
    if width == 0 {
        return Ok(0);
    }
    Ok(((width as usize) + 7) >> 3)
}

/// Read a single bit from a packed row at pixel position `x`.
///
/// Returns 0 for `x < 0` and for `x >= width`. This matches the JBIG2
/// convention for template pixels that fall outside the region.
///
/// The hot paths in the generic-region decoder call this for every
/// template reference; keep the body branch-light.
#[inline(always)]
pub fn read_bit(row: &[u8], x: i32, width: i32) -> u32 {
    // Single unsigned compare catches both x<0 (wraps to huge) and x>=width.
    if (x as u32) >= (width as u32) {
        return 0;
    }
    let idx = (x as usize) >> 3;
    let shift = 7 - (x as u32 & 7);
    ((row[idx] >> shift) & 1) as u32
}

/// OR-set bit `x` within a packed row. Silently ignores `x >= row.len() * 8`.
#[inline(always)]
pub fn set_bit(row: &mut [u8], x: i32) {
    let idx = (x as usize) >> 3;
    let shift = 7 - (x as u32 & 7);
    row[idx] |= 1u8 << shift;
}

/// Blit `count` bits from `src` starting at pixel offset `src_x` into `dst`
/// starting at pixel offset `dst_x`, combining with `op`.
///
/// The row buffers use MSB-first packed storage. The fast path is the
/// byte-aligned case (`src_x & 7 == dst_x & 7`), where we can combine full
/// source bytes into destination bytes; the general path uses a 16-bit
/// sliding accumulator so each iteration still produces a complete aligned
/// source byte for the current destination position. That keeps the cost
/// per compositing at ~`count / 8` iterations instead of `count` per-pixel
/// set/get calls.
#[inline]
pub fn composite_row(
    dst: &mut [u8],
    dst_x: usize,
    src: &[u8],
    src_x: usize,
    count: usize,
    op: BlitOp,
) {
    if count == 0 {
        return;
    }
    let dst_bit = dst_x & 7;
    let src_bit = src_x & 7;
    if dst_bit == src_bit {
        composite_aligned(dst, dst_x, src, src_x, count, op);
    } else {
        composite_shifted(dst, dst_x, src, src_x, count, op);
    }
}

#[inline(always)]
fn apply_op_masked(dst_byte: &mut u8, src_byte: u8, mask: u8, op: BlitOp) {
    let d = *dst_byte;
    let combined = match op {
        BlitOp::Or => d | (src_byte & mask),
        BlitOp::And => (d & !mask) | (d & src_byte & mask),
        BlitOp::Xor => d ^ (src_byte & mask),
        BlitOp::XNor => (d & !mask) | (!(d ^ src_byte) & mask),
        BlitOp::Replace => (d & !mask) | (src_byte & mask),
    };
    *dst_byte = combined;
}

fn composite_aligned(
    dst: &mut [u8],
    dst_x: usize,
    src: &[u8],
    src_x: usize,
    count: usize,
    op: BlitOp,
) {
    let bit_off = dst_x & 7; // == src_x & 7
    let mut d_idx = dst_x >> 3;
    let mut s_idx = src_x >> 3;
    let mut remaining = count;
    // Leading partial dst byte.
    if bit_off != 0 {
        let n = (8 - bit_off).min(remaining);
        // Mask for the lower (8 - bit_off) bits, trimmed to `n` bits of
        // contribution beginning at bit (7 - bit_off) in the byte.
        let mask = mask_range(bit_off, n);
        apply_op_masked(&mut dst[d_idx], src[s_idx], mask, op);
        remaining -= n;
        if remaining == 0 {
            return;
        }
        d_idx += 1;
        s_idx += 1;
    }
    // Full-byte middle section.
    let full = remaining / 8;
    for k in 0..full {
        apply_op_masked(&mut dst[d_idx + k], src[s_idx + k], 0xFF, op);
    }
    d_idx += full;
    s_idx += full;
    remaining -= full * 8;
    // Trailing partial byte.
    if remaining > 0 {
        let mask = mask_range(0, remaining);
        apply_op_masked(&mut dst[d_idx], src[s_idx], mask, op);
    }
}

fn composite_shifted(
    dst: &mut [u8],
    dst_x: usize,
    src: &[u8],
    src_x: usize,
    count: usize,
    op: BlitOp,
) {
    // Process one dst byte at a time. For each dst byte we need 8 source
    // bits aligned as if read from a src pointer with the same bit offset
    // as dst has. We assemble those bits by stitching together the tail of
    // one src byte with the head of the next.
    let mut remaining = count;
    let mut d_bit = (dst_x & 7) as u32;
    let mut d_idx = dst_x >> 3;
    let mut s_bit = (src_x & 7) as u32;
    let mut s_idx = src_x >> 3;

    while remaining > 0 {
        // Bits we can fit into the remainder of this dst byte.
        let room = 8 - d_bit as usize;
        let n = room.min(remaining);
        // Extract `n` source bits starting at s_bit of src[s_idx] (possibly
        // spilling into src[s_idx+1]).
        let s_room = 8 - s_bit as usize;
        let take_first = s_room.min(n);
        let first_byte = src.get(s_idx).copied().unwrap_or(0);
        let first_bits = (first_byte >> (s_room - take_first)) & mask_lower(take_first);
        let mut src_bits = first_bits as u16;
        let mut got = take_first;
        if got < n {
            let need = n - got;
            let second_byte = src.get(s_idx + 1).copied().unwrap_or(0);
            let high = (second_byte >> (8 - need)) & mask_lower(need);
            src_bits = (src_bits << need) | high as u16;
            got += need;
        }
        debug_assert_eq!(got, n);
        // Shift src_bits up so they sit at bits [7-d_bit .. 7-d_bit-n+1]
        // within a single byte.
        let shift_up = 8 - d_bit as usize - n;
        let shifted = (src_bits << shift_up) as u8;
        let mask = mask_range(d_bit as usize, n);
        apply_op_masked(&mut dst[d_idx], shifted, mask, op);

        remaining -= n;
        d_bit += n as u32;
        if d_bit == 8 {
            d_bit = 0;
            d_idx += 1;
        }
        s_bit += n as u32;
        while s_bit >= 8 {
            s_bit -= 8;
            s_idx += 1;
        }
    }
}

/// `n` bits set in the low positions of a byte (n in [0, 8]).
#[inline(always)]
fn mask_lower(n: usize) -> u8 {
    if n >= 8 {
        0xFF
    } else {
        (1u8 << n) - 1
    }
}

/// Mask covering `count` consecutive bits within a byte, starting at bit
/// position `bit_off` (0 = MSB). `bit_off + count` must be <= 8.
#[inline(always)]
fn mask_range(bit_off: usize, count: usize) -> u8 {
    if count == 0 {
        return 0;
    }
    let low_mask = mask_lower(count);
    low_mask << (8 - bit_off - count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_round_trip() {
        let mut bm = Bitmap::new(17, 9).unwrap();
        assert_eq!(bm.stride(), 3);
        bm.set_pixel(0, 0, 1);
        bm.set_pixel(16, 8, 1);
        bm.set_pixel(8, 4, 1);
        assert_eq!(bm.get_pixel(0, 0), 1);
        assert_eq!(bm.get_pixel(16, 8), 1);
        assert_eq!(bm.get_pixel(8, 4), 1);
        assert_eq!(bm.get_pixel(1, 0), 0);
        assert_eq!(bm.get_pixel(-1, 0), 0);
        assert_eq!(bm.get_pixel(0, -1), 0);
        assert_eq!(bm.get_pixel(17, 0), 0);
    }

    #[test]
    fn filled_zero_and_one() {
        let white = Bitmap::filled(10, 2, 0).unwrap();
        let black = Bitmap::filled(10, 2, 1).unwrap();
        for y in 0..2 {
            for x in 0..10 {
                assert_eq!(white.get_pixel(x, y), 0);
                assert_eq!(black.get_pixel(x, y), 1);
            }
        }
        // Padding bits of "black" rows should be zero.
        assert_eq!(black.row(0)[1] & 0b0011_1111, 0);
    }

    #[test]
    fn composite_or_aligned() {
        let mut dst = Bitmap::new(32, 4).unwrap();
        let mut src = Bitmap::new(16, 2).unwrap();
        for x in 0..16 {
            src.set_pixel(x, 0, 1);
        }
        src.set_pixel(0, 1, 1);
        src.set_pixel(15, 1, 1);
        dst.composite(&src, 8, 1, BlitOp::Or);
        for x in 0..32 {
            let expect = if (8..24).contains(&x) { 1 } else { 0 };
            assert_eq!(dst.get_pixel(x, 1), expect, "row 1 pixel {x}");
        }
        // Row 0 of dst should be untouched (we composited into rows 1..3).
        for x in 0..32 {
            assert_eq!(dst.get_pixel(x, 0), 0);
        }
        // Row 2 of dst should have bits at (8, 2) and (23, 2).
        for x in 0..32 {
            let expect = if x == 8 || x == 23 { 1 } else { 0 };
            assert_eq!(dst.get_pixel(x, 2), expect, "row 2 pixel {x}");
        }
    }

    #[test]
    fn composite_or_shifted() {
        // Unaligned dst offset: dst_x = 3, so source bits land at dst bit
        // offset 3. Exercise the shifted path against an 11-pixel-wide source
        // that crosses a dst byte boundary.
        let mut dst = Bitmap::new(32, 2).unwrap();
        let mut src = Bitmap::new(11, 1).unwrap();
        let src_bits = [1u8, 0, 1, 1, 0, 1, 0, 0, 1, 1, 1];
        for (i, &b) in src_bits.iter().enumerate() {
            src.set_pixel(i as i32, 0, b);
        }
        dst.composite(&src, 3, 0, BlitOp::Or);
        for x in 0..32 {
            let expect = if (3..14).contains(&x) {
                src_bits[(x - 3) as usize]
            } else {
                0
            };
            assert_eq!(dst.get_pixel(x, 0), expect, "pixel {x}");
        }
    }

    #[test]
    fn composite_clips_against_dst_bounds() {
        let mut dst = Bitmap::new(16, 4).unwrap();
        let mut src = Bitmap::new(8, 3).unwrap();
        for x in 0..8 {
            for y in 0..3 {
                src.set_pixel(x, y, 1);
            }
        }
        // Placed partially out of bounds on both left and top.
        dst.composite(&src, -3, -1, BlitOp::Or);
        // Visible: src pixels (3..8, 1..3) land at dst (0..5, 0..2).
        for y in 0..4 {
            for x in 0..16 {
                let expect = if x < 5 && y < 2 { 1 } else { 0 };
                assert_eq!(dst.get_pixel(x, y), expect, "pixel ({x},{y})");
            }
        }
    }

    #[test]
    fn composite_xor_shifted() {
        let mut dst = Bitmap::filled(16, 1, 1).unwrap();
        let mut src = Bitmap::new(12, 1).unwrap();
        for x in 0..12 {
            src.set_pixel(x, 0, (x & 1) as u8);
        }
        dst.composite(&src, 2, 0, BlitOp::Xor);
        for x in 0..16 {
            let expected = if (2..14).contains(&x) {
                1 ^ ((x - 2) & 1) as u8
            } else {
                1
            };
            assert_eq!(dst.get_pixel(x, 0), expected, "pixel {x}");
        }
    }

    #[test]
    fn from_packed_validates() {
        let err = Bitmap::from_packed(8, 1, 0, vec![]).unwrap_err();
        assert!(matches!(err, Jbig2Error::InvalidGeometry(_)));
        let err = Bitmap::from_packed(8, 1, 1, vec![0; 2]).unwrap_err();
        assert!(matches!(err, Jbig2Error::InvalidGeometry(_)));
        let bm = Bitmap::from_packed(8, 1, 1, vec![0xA5]).unwrap();
        assert_eq!(bm.get_pixel(0, 0), 1);
        assert_eq!(bm.get_pixel(1, 0), 0);
    }
}
