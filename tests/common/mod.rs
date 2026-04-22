//! Shared test helpers: vendored conformance-file paths + a minimal 1-bpp
//! BMP reader.

use std::path::PathBuf;

use jbig2::Bitmap;

/// Absolute path to the vendored JBIG2 conformance directory.
pub fn conformance_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("vendor")
        .join("T-REC-T.88-201808")
        .join("Software")
        .join("JBIG2_ConformanceData-A20180829")
}

/// Load a 1-bpp BMP from the conformance directory and return it as a
/// [`Bitmap`] where `1 = ink`, with `(0,0)` in the top-left corner.
///
/// The BMP files shipped by the reference use palette index `0 = black`,
/// `1 = white`, so we invert on load to match the JBIG2 convention.
pub fn load_conformance_bmp(name: &str) -> Bitmap {
    let path = conformance_dir().join(name);
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    parse_bmp_1bpp(&data)
}

fn parse_bmp_1bpp(data: &[u8]) -> Bitmap {
    assert!(data.len() >= 54 && &data[0..2] == b"BM", "not a BMP file");
    let pixel_offset = u32::from_le_bytes(data[10..14].try_into().unwrap()) as usize;
    let dib_size = u32::from_le_bytes(data[14..18].try_into().unwrap()) as usize;
    assert!(dib_size >= 40, "not a BITMAPINFOHEADER");
    let width = i32::from_le_bytes(data[18..22].try_into().unwrap());
    let height_signed = i32::from_le_bytes(data[22..26].try_into().unwrap());
    let bpp = u16::from_le_bytes(data[28..30].try_into().unwrap());
    assert_eq!(bpp, 1, "expected 1-bpp BMP");
    let top_down = height_signed < 0;
    let height = height_signed.unsigned_abs();
    let width_u = width as u32;

    // Palette: entries at bytes 54..54+8 (4 bytes each for 2 entries).
    let pal0 = &data[54..58]; // (B, G, R, A)
    let _pal1 = &data[58..62];
    // If palette index 0 is "dark" (all low), then BMP-0 = ink; invert flag.
    let zero_is_ink = pal0[0] <= 0x40 && pal0[1] <= 0x40 && pal0[2] <= 0x40;

    // BMP rows are padded to 4-byte boundaries.
    let row_bytes = (((width_u + 31) / 32) * 4) as usize;
    let stride = ((width_u + 7) / 8) as usize;
    let mut bm = Bitmap::new(width_u, height).unwrap();
    for y in 0..height {
        let src_y = if top_down { y } else { height - 1 - y };
        let row_start = pixel_offset + src_y as usize * row_bytes;
        let src = &data[row_start..row_start + stride];
        let row = bm.row_mut(y as usize);
        if zero_is_ink {
            for (d, s) in row.iter_mut().zip(src) {
                *d = !*s;
            }
        } else {
            row.copy_from_slice(src);
        }
        // Clear padding bits beyond the image width in the last byte.
        let last_bits = width_u & 7;
        if last_bits != 0 {
            let mask = 0xFFu8 << (8 - last_bits);
            let last = row.len() - 1;
            row[last] &= mask;
        }
    }
    bm
}
