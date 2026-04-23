//! Simple packed RGB bitmap used for AMD3 colour output pages.

use crate::bitmap::Bitmap;
use crate::error::{Jbig2Error, Jbig2Result};

/// An 8-bit-per-channel RGB bitmap with tightly-packed row-major storage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbBitmap {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl RgbBitmap {
    /// Create a new bitmap filled with a single RGB colour.
    pub fn filled(width: u32, height: u32, rgb: [u8; 3]) -> Jbig2Result<Self> {
        let len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|px| px.checked_mul(3))
            .ok_or(Jbig2Error::InvalidGeometry("rgb bitmap too large"))?;
        let mut data = vec![0u8; len];
        for px in data.chunks_exact_mut(3) {
            px.copy_from_slice(&rgb);
        }
        Ok(Self {
            width,
            height,
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

    /// Borrow the raw RGB buffer.
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Borrow one packed RGB row.
    #[inline]
    pub fn row(&self, y: usize) -> &[u8] {
        let stride = self.width as usize * 3;
        let start = y * stride;
        &self.data[start..start + stride]
    }

    /// Borrow one packed RGB row mutably.
    #[inline]
    pub fn row_mut(&mut self, y: usize) -> &mut [u8] {
        let stride = self.width as usize * 3;
        let start = y * stride;
        &mut self.data[start..start + stride]
    }

    /// Read one RGB pixel. Out-of-bounds returns white.
    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> [u8; 3] {
        if x < 0 || y < 0 || (x as u32) >= self.width || (y as u32) >= self.height {
            return [255, 255, 255];
        }
        let idx = ((y as usize) * self.width as usize + x as usize) * 3;
        [self.data[idx], self.data[idx + 1], self.data[idx + 2]]
    }

    /// Write one RGB pixel. Out-of-bounds writes are ignored.
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, rgb: [u8; 3]) {
        if x < 0 || y < 0 || (x as u32) >= self.width || (y as u32) >= self.height {
            return;
        }
        let idx = ((y as usize) * self.width as usize + x as usize) * 3;
        self.data[idx..idx + 3].copy_from_slice(&rgb);
    }

    /// Paint a 1-bpp mask into the bitmap using a solid colour.
    pub fn composite_mask_color(&mut self, mask: &Bitmap, x0: i32, y0: i32, rgb: [u8; 3]) {
        // TODO(perf): add a packed-row mask walker so large AMD3 regions do
        // not pay one get_pixel/set_pixel pair per foreground bit.
        for y in 0..mask.height() as i32 {
            for x in 0..mask.width() as i32 {
                if mask.get_pixel(x, y) != 0 {
                    self.set_pixel(x0 + x, y0 + y, rgb);
                }
            }
        }
    }

    /// Replace-copy another RGB bitmap into this one with clipping.
    pub fn composite_replace(&mut self, src: &RgbBitmap, x0: i32, y0: i32) {
        // TODO(perf): switch to row copies / clipped spans once colour pages
        // become hot enough to justify a packed fast path here.
        for y in 0..src.height as i32 {
            let dy = y0 + y;
            if dy < 0 || dy >= self.height as i32 {
                continue;
            }
            for x in 0..src.width as i32 {
                let dx = x0 + x;
                if dx < 0 || dx >= self.width as i32 {
                    continue;
                }
                self.set_pixel(dx, dy, src.get_pixel(x, y));
            }
        }
    }
}
