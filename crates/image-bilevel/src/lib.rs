//! Packed 1-bit-per-pixel bilevel image type for the image-rs ecosystem.
#![forbid(unsafe_code)]

/// A bilevel pixel.
///
/// `false` is paper (`0` in PBM/JBIG2 convention) and `true` is ink (`1`).
/// This convention is the inverse of formats such as PNG, BMP, and TIFF when
/// they store one-bit grayscale samples.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Bilevel(pub bool);

/// A 1-bit-per-pixel image with packed row storage.
///
/// Rows are big-endian-bit packed into bytes: the leftmost pixel in a row is
/// stored in bit 7 of the first row byte. The row stride is
/// `width.div_ceil(8)` bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BilevelImage {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl BilevelImage {
    /// Create a fully-paper image of the given size.
    pub fn new(width: u32, height: u32) -> Self {
        let len = row_stride(width) * height as usize;
        Self {
            width,
            height,
            data: vec![0; len],
        }
    }

    /// Construct from existing packed row bytes.
    ///
    /// Returns `None` if `data` is not exactly `width.div_ceil(8) * height`
    /// bytes long.
    pub fn from_packed_rows(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        if data.len() != row_stride(width) * height as usize {
            return None;
        }
        Some(Self {
            width,
            height,
            data,
        })
    }

    /// Threshold an `image::GrayImage` into packed bilevel pixels.
    ///
    /// Pixels below `threshold` become ink; all other pixels become paper.
    pub fn from_luma8(image: &image::GrayImage, threshold: u8) -> Self {
        let mut out = Self::new(image.width(), image.height());
        let width = image.width() as usize;
        let raw = image.as_raw();
        for y in 0..image.height() as usize {
            for x in 0..width {
                if raw[y * width + x] < threshold {
                    out.set_pixel(x as u32, y as u32, Bilevel(true));
                }
            }
        }
        out
    }

    /// Image width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Image height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Raw packed image bytes.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Consume the image and return the raw packed image bytes.
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    /// Bytes per packed row.
    pub fn row_stride(&self) -> usize {
        row_stride(self.width)
    }

    /// Borrow one packed row.
    ///
    /// Panics if `y >= self.height()`.
    pub fn row(&self, y: u32) -> &[u8] {
        let stride = self.row_stride();
        let start = y as usize * stride;
        &self.data[start..start + stride]
    }

    /// Return one pixel.
    ///
    /// Panics if the coordinates are outside the image bounds.
    pub fn get_pixel(&self, x: u32, y: u32) -> Bilevel {
        assert!(x < self.width && y < self.height, "pixel out of bounds");
        let stride = self.row_stride();
        let byte = self.data[y as usize * stride + (x as usize >> 3)];
        Bilevel(((byte >> (7 - (x as usize & 7))) & 1) != 0)
    }

    /// Set one pixel.
    ///
    /// Panics if the coordinates are outside the image bounds.
    pub fn set_pixel(&mut self, x: u32, y: u32, value: Bilevel) {
        assert!(x < self.width && y < self.height, "pixel out of bounds");
        let stride = self.row_stride();
        let idx = y as usize * stride + (x as usize >> 3);
        let mask = 0x80 >> (x as usize & 7);
        if value.0 {
            self.data[idx] |= mask;
        } else {
            self.data[idx] &= !mask;
        }
    }

    /// Expand to 8-bpp grayscale using caller-chosen sample values.
    pub fn to_luma8(&self, ink: u8, paper: u8) -> image::GrayImage {
        let mut out = vec![paper; self.width as usize * self.height as usize];
        for y in 0..self.height as usize {
            let row = self.row(y as u32);
            for x in 0..self.width as usize {
                let bit = (row[x >> 3] >> (7 - (x & 7))) & 1;
                if bit != 0 {
                    out[y * self.width as usize + x] = ink;
                }
            }
        }
        image::GrayImage::from_raw(self.width, self.height, out)
            .expect("buffer sized width*height by construction")
    }

    /// Convert to `image::DynamicImage` using black ink on white paper.
    pub fn into_dynamic(self) -> image::DynamicImage {
        image::DynamicImage::ImageLuma8(self.to_luma8(0, 255))
    }
}

fn row_stride(width: u32) -> usize {
    width.div_ceil(8) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_round_trip() {
        let mut image = BilevelImage::new(9, 2);
        image.set_pixel(0, 0, Bilevel(true));
        image.set_pixel(8, 1, Bilevel(true));
        assert_eq!(image.row_stride(), 2);
        assert_eq!(image.get_pixel(0, 0), Bilevel(true));
        assert_eq!(image.get_pixel(8, 1), Bilevel(true));
        assert_eq!(image.data(), &[0x80, 0x00, 0x00, 0x80]);
    }

    #[test]
    fn luma_conversion_uses_threshold() {
        let gray = image::GrayImage::from_raw(3, 1, vec![0, 127, 128]).unwrap();
        let bilevel = BilevelImage::from_luma8(&gray, 128);
        assert_eq!(bilevel.data(), &[0b1100_0000]);
        assert_eq!(bilevel.to_luma8(0, 255).into_raw(), vec![0, 0, 255]);
    }
}
