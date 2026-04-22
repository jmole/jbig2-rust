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
