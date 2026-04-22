//! Integration test for the `image`-crate plugin surface.

#![cfg(feature = "image")]

mod common;

use std::io::Cursor;

use jbig2::{Bitmap, EncoderConfig, Jbig2Encoder};

#[test]
fn register_then_guess_format_and_decode() {
    // Build a tiny synthetic JBIG2 file via the public encoder, then feed it
    // to image::guess_format / image::load through the decoding-hook surface.
    let mut bm = Bitmap::new(48, 16).unwrap();
    for y in 0..16 {
        for x in 0..48 {
            if (x * 3 + y) % 7 == 0 {
                bm.set_pixel(x, y, 1);
            }
        }
    }

    let mut buf = Vec::new();
    let mut enc = Jbig2Encoder::new(&mut buf, EncoderConfig::balanced());
    enc.write_page(&bm).unwrap();
    enc.finish().unwrap();

    jbig2::register();
    // Hooks are global; re-registering is idempotent.
    jbig2::register();

    let decoder = jbig2::image_plugin::Jbig2ImageDecoder::new(Cursor::new(buf.clone()))
        .expect("adapter open");
    let (w, h) = image::ImageDecoder::dimensions(&decoder);
    assert_eq!(w, 48);
    assert_eq!(h, 16);

    let total = (w as usize) * (h as usize);
    let mut pixels = vec![0u8; total];
    image::ImageDecoder::read_image(decoder, &mut pixels).expect("read_image");
    for y in 0..h {
        for x in 0..w {
            let pix = pixels[(y * w + x) as usize];
            let expected = if bm.get_pixel(x as i32, y as i32) == 1 { 0 } else { 255 };
            assert_eq!(pix, expected, "pixel mismatch at ({x},{y})");
        }
    }
}
