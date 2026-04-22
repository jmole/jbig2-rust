//! `image`-crate integration.
//!
//! Calling [`register()`] once (usually during process startup) installs a
//! decoding hook for the `.jb2` extension and a magic-byte detector for the
//! JBIG2 file-header signature, so callers of `image::open` or
//! `image::guess_format` can transparently read JBIG2 files.
//!
//! JBIG2 files can carry multiple pages; `image`'s [`ImageDecoder`] trait only
//! exposes a single image, so we return page 1 (the first page). Applications
//! that need random access to later pages should use [`crate::Jbig2Decoder`]
//! directly and call [`Jbig2Decoder::decode_page`] with a 1-based page index.

use std::io::{Cursor, Read, Seek};
use std::sync::Once;

use image::error::{DecodingError, ImageError, ImageFormatHint};
use image::hooks::{register_decoding_hook, register_format_detection_hook, GenericReader};
use image::{ColorType, ImageDecoder, ImageResult};

use crate::{Jbig2Decoder, Jbig2Error, JBIG2_MAGIC};

static INIT: Once = Once::new();

/// Install decoding + format-detection hooks for JBIG2 into `image`'s global
/// registry. Safe to call from any thread and idempotent — only the first
/// call registers hooks; subsequent calls are no-ops.
pub fn register() {
    INIT.call_once(|| {
        register_format_detection_hook("jb2".into(), &JBIG2_MAGIC, None);
        register_decoding_hook(
            "jb2".into(),
            Box::new(|r: GenericReader<'_>| -> ImageResult<Box<dyn ImageDecoder + '_>> {
                let decoder = Jbig2ImageDecoder::new(r).map_err(jbig2_to_image)?;
                Ok(Box::new(decoder))
            }),
        );
    });
}

fn jbig2_to_image(e: Jbig2Error) -> ImageError {
    ImageError::Decoding(DecodingError::new(
        ImageFormatHint::Name("jbig2".to_owned()),
        e.to_string(),
    ))
}

/// Thin adapter turning a [`Jbig2Decoder`] into an [`image::ImageDecoder`].
pub struct Jbig2ImageDecoder<R: Read + Seek> {
    inner: Jbig2Decoder<R>,
    width: u32,
    height: u32,
}

impl<R: Read + Seek> Jbig2ImageDecoder<R> {
    /// Parse the file header and enumerate segments.
    pub fn new(r: R) -> crate::Jbig2Result<Self> {
        let mut inner = Jbig2Decoder::new(r)?;
        let has_page_info = inner
            .segment_headers()
            .any(|sh| matches!(sh.segment_type, crate::segments::SegmentType::PageInformation));
        let (w, h) = if has_page_info {
            match inner.decode_page(1) {
                Ok(p) => (p.bitmap.width(), p.bitmap.height()),
                Err(_) => (0, 0),
            }
        } else {
            (0, 0)
        };
        Ok(Self {
            inner,
            width: w,
            height: h,
        })
    }

    /// Total number of pages in this file.
    pub fn num_pages(&self) -> u32 {
        self.inner.num_pages()
    }

    /// Drop the image-crate adapter and return the underlying decoder for
    /// callers that need multi-page access.
    pub fn into_inner(self) -> Jbig2Decoder<R> {
        self.inner
    }
}

impl<R: Read + Seek> ImageDecoder for Jbig2ImageDecoder<R> {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn color_type(&self) -> ColorType {
        ColorType::L8
    }

    fn read_image(mut self, buf: &mut [u8]) -> ImageResult<()>
    where
        Self: Sized,
    {
        let page = self.inner.decode_page(1).map_err(jbig2_to_image)?;
        let bm = page.bitmap;
        assert_eq!(buf.len(), (bm.width() as usize) * (bm.height() as usize));
        // Expand 1-bpp → 8-bpp. JBIG2 `1` = ink (black); image's L8 treats
        // `0` as black and `255` as white, so we map
        //   jbig2 0 (white/paper) → 255
        //   jbig2 1 (black/ink)   → 0
        let width = bm.width() as usize;
        for y in 0..bm.height() as usize {
            let row = bm.row(y);
            for x in 0..width {
                let bit = (row[x >> 3] >> (7 - (x & 7))) & 1;
                buf[y * width + x] = if bit == 0 { 255 } else { 0 };
            }
        }
        Ok(())
    }

    fn read_image_boxed(self: Box<Self>, buf: &mut [u8]) -> ImageResult<()> {
        (*self).read_image(buf)
    }
}

/// Convenience: decode every page in a JBIG2 byte buffer into a vector of
/// `image::GrayImage`s.
pub fn decode_all_pages_gray(bytes: &[u8]) -> crate::Jbig2Result<Vec<image::GrayImage>> {
    let mut inner = Jbig2Decoder::new(Cursor::new(bytes.to_vec()))?;
    let n = inner.num_pages();
    let mut out = Vec::with_capacity(n as usize);
    for p in 1..=n {
        let page = inner.decode_page(p)?;
        let bm = page.bitmap;
        let width = bm.width();
        let height = bm.height();
        let mut img = image::GrayImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let bit = bm.get_pixel(x as i32, y as i32);
                img.put_pixel(x, y, image::Luma([if bit == 0 { 255 } else { 0 }]));
            }
        }
        out.push(img);
    }
    Ok(out)
}
