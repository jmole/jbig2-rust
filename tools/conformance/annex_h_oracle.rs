use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use image::{ImageReader, Rgba, RgbaImage};

const PAGE_WIDTH: u32 = 64;
const PAGE_HEIGHT: u32 = 56;
const DARK_THRESHOLD: u8 = 64;
const CELL_THRESHOLD: u8 = 128;
const SAMPLE_RADIUS: i32 = 4;
const EXPECTED_RASTER_SHA256: &str =
    "975e63be32f6dd9c4367dd25ae268cd5701b888717656236c98c31ee8bb35db4";

fn main() {
    if let Err(err) = run() {
        eprintln!("annex-h-oracle: {err}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    let img = ImageReader::open(&args.source)
        .map_err(|err| format!("open {:?}: {err}", args.source))?
        .decode()
        .map_err(|err| format!("decode {:?}: {err}", args.source))?
        .to_rgba8();

    let (bitmap, geometry) = extract_page_bitmap(&img)?;
    fs::create_dir_all(&args.out_dir).map_err(|err| format!("create {:?}: {err}", args.out_dir))?;

    let page0 = args.out_dir.join("annex-h-page-00.bmp");
    write_bmp_1bpp(&page0, PAGE_WIDTH, PAGE_HEIGHT, &bitmap)?;

    if args.duplicate_page_1 {
        let page1 = args.out_dir.join("annex-h-page-01.bmp");
        write_bmp_1bpp(&page1, PAGE_WIDTH, PAGE_HEIGHT, &bitmap)?;
    }

    let overlay = args.out_dir.join("annex-h-sampling-overlay.png");
    write_overlay(&overlay, &img, &geometry)?;

    let packed = pack_rows(PAGE_WIDTH, PAGE_HEIGHT, &bitmap)?;
    let raster_sha = sha256_hex(&packed);
    let provenance = args.out_dir.join("annex-h-oracle-provenance.md");
    write_provenance(&provenance, &args.source, &geometry, &raster_sha)?;

    println!(
        "crop: top={} bottom={}",
        geometry.crop_top, geometry.crop_bottom
    );
    println!(
        "grid: left={} right={} top={} bottom={}",
        geometry.left, geometry.right, geometry.top, geometry.bottom
    );
    println!(
        "cell spacing: x={:.6} y={:.6}",
        geometry.x_step, geometry.y_step
    );
    println!(
        "wrote Annex H BMP oracle: {:?}{}",
        page0,
        if args.duplicate_page_1 {
            " and annex-h-page-01.bmp"
        } else {
            ""
        }
    );
    println!("wrote sampling overlay: {:?}", overlay);
    println!("wrote provenance: {:?}", provenance);
    println!("packed raster sha256: {raster_sha}");

    if raster_sha != EXPECTED_RASTER_SHA256 {
        return Err(format!(
            "unexpected packed raster sha256: {raster_sha}; expected {EXPECTED_RASTER_SHA256}"
        ));
    }

    if let Some(path) = args.jbig2dec_probe.as_deref() {
        cross_check_jbig2dec(path, &bitmap)?;
    }

    Ok(())
}

struct Args {
    source: PathBuf,
    out_dir: PathBuf,
    duplicate_page_1: bool,
    jbig2dec_probe: Option<PathBuf>,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut source = root
            .join("vendor")
            .join("T-REC-T.88-201808")
            .join("spec")
            .join("annex-h-oracle-source.png");
        let mut out_dir = root.join("vendor").join("T-REC-T.88-201808").join("spec");
        let mut jbig2dec_probe = Some(
            root.join("target")
                .join("conformance-matrix")
                .join("annex-h-jbig2dec.pbm"),
        );
        let mut duplicate_page_1 = true;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--source" => {
                    source = PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--source requires a path".to_string())?,
                    );
                }
                "--out-dir" => {
                    out_dir = PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--out-dir requires a path".to_string())?,
                    );
                }
                "--single-page" => duplicate_page_1 = false,
                "--no-jbig2dec-probe" => jbig2dec_probe = None,
                "-h" | "--help" => {
                    println!(
                        "usage: annex-h-oracle [--source PNG] [--out-dir DIR] [--single-page] [--no-jbig2dec-probe]"
                    );
                    std::process::exit(0);
                }
                other => return Err(format!("unexpected argument {other:?}")),
            }
        }

        Ok(Self {
            source,
            out_dir,
            duplicate_page_1,
            jbig2dec_probe,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct Geometry {
    crop_top: u32,
    crop_bottom: u32,
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
    x_step: f64,
    y_step: f64,
}

fn extract_page_bitmap(img: &RgbaImage) -> Result<(Vec<u8>, Geometry), String> {
    let mut geometry = detect_grid(img)?;
    geometry.x_step = (geometry.right - geometry.left) as f64 / PAGE_WIDTH as f64;
    geometry.y_step = (geometry.bottom - geometry.top) as f64 / PAGE_HEIGHT as f64;

    let mut bits = vec![0u8; (PAGE_WIDTH * PAGE_HEIGHT) as usize];
    for y in 0..PAGE_HEIGHT {
        for x in 0..PAGE_WIDTH {
            let cx = sample_x(&geometry, x);
            let cy = sample_y(&geometry, y);
            let luma = median_luma(img, cx.round() as i32, cy.round() as i32)?;
            bits[(y * PAGE_WIDTH + x) as usize] = u8::from(luma < CELL_THRESHOLD);
        }
    }

    Ok((bits, geometry))
}

fn detect_grid(img: &RgbaImage) -> Result<Geometry, String> {
    let (width, height) = img.dimensions();
    let dark_in_row = |y| {
        (0..width)
            .filter(|&x| luma(img.get_pixel(x, y).0) < DARK_THRESHOLD)
            .count()
    };

    let horizontal_lines = (0..height)
        .filter(|&y| dark_in_row(y) * 100 > width as usize * 95)
        .collect::<Vec<_>>();

    let (top, bottom) = select_horizontal_grid(&horizontal_lines)?;
    let grid_height = bottom - top + 1;

    if horizontal_lines.len() < PAGE_HEIGHT as usize + 1 {
        return Err(format!(
            "expected at least {} horizontal grid lines, found {}",
            PAGE_HEIGHT + 1,
            horizontal_lines.len()
        ));
    }

    let vertical_lines = (0..width)
        .filter(|&x| {
            (top..=bottom)
                .filter(|&y| luma(img.get_pixel(x, y).0) < DARK_THRESHOLD)
                .count() as u32
                == grid_height
        })
        .collect::<Vec<_>>();

    if vertical_lines.len() < 2 {
        return Err(format!(
            "expected at least left/right vertical grid lines, found {}",
            vertical_lines.len()
        ));
    }

    let left = *vertical_lines
        .first()
        .ok_or_else(|| "no vertical grid lines".to_string())?;
    let right = *vertical_lines
        .last()
        .ok_or_else(|| "no vertical grid lines".to_string())?;

    Ok(Geometry {
        crop_top: top,
        crop_bottom: bottom,
        left,
        right,
        top,
        bottom,
        x_step: 0.0,
        y_step: 0.0,
    })
}

fn select_horizontal_grid(lines: &[u32]) -> Result<(u32, u32), String> {
    let needed = PAGE_HEIGHT as usize + 1;
    if lines.len() < needed {
        return Err(format!(
            "expected at least {needed} horizontal grid lines, found {}",
            lines.len()
        ));
    }

    let mut best: Option<(f64, u32, u32)> = None;
    for window in lines.windows(needed) {
        let first = window[0];
        let last = *window.last().unwrap();
        let step = (last - first) as f64 / PAGE_HEIGHT as f64;
        let max_err = window
            .iter()
            .enumerate()
            .map(|(idx, &line)| {
                let expected = first as f64 + idx as f64 * step;
                (line as f64 - expected).abs()
            })
            .fold(0.0, f64::max);

        if best
            .map(|(best_err, _, _)| max_err < best_err)
            .unwrap_or(true)
        {
            best = Some((max_err, first, last));
        }
    }

    let (max_err, first, last) = best.ok_or_else(|| "no horizontal grid candidate".to_string())?;
    if max_err > 2.0 {
        return Err(format!(
            "best horizontal grid candidate has max spacing error {max_err:.3}px"
        ));
    }
    Ok((first, last))
}

fn sample_x(geometry: &Geometry, x: u32) -> f64 {
    geometry.left as f64 + (x as f64 + 0.5) * geometry.x_step
}

fn sample_y(geometry: &Geometry, y: u32) -> f64 {
    geometry.top as f64 + (y as f64 + 0.5) * geometry.y_step
}

fn median_luma(img: &RgbaImage, cx: i32, cy: i32) -> Result<u8, String> {
    let (width, height) = img.dimensions();
    let mut values = Vec::with_capacity(((SAMPLE_RADIUS * 2 + 1).pow(2)) as usize);
    for dy in -SAMPLE_RADIUS..=SAMPLE_RADIUS {
        for dx in -SAMPLE_RADIUS..=SAMPLE_RADIUS {
            let x = cx + dx;
            let y = cy + dy;
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                return Err(format!("sample point out of bounds at ({x}, {y})"));
            }
            values.push(luma(img.get_pixel(x as u32, y as u32).0));
        }
    }
    values.sort_unstable();
    Ok(values[values.len() / 2])
}

fn luma(rgba: [u8; 4]) -> u8 {
    let [r, g, b, a] = rgba;
    let a = a as u32;
    let r = (r as u32 * a + 255 * (255 - a)) / 255;
    let g = (g as u32 * a + 255 * (255 - a)) / 255;
    let b = (b as u32 * a + 255 * (255 - a)) / 255;
    ((r * 299 + g * 587 + b * 114) / 1000) as u8
}

fn write_bmp_1bpp(path: &Path, width: u32, height: u32, bits: &[u8]) -> Result<(), String> {
    let row_stride = width.div_ceil(32) as usize * 4;
    let image_size = row_stride * height as usize;
    let pixel_offset = 14 + 40 + 8;
    let file_size = pixel_offset + image_size;
    let packed = pack_rows_with_stride(width, height, bits, row_stride)?;

    let mut out = Vec::with_capacity(file_size);
    out.extend(b"BM");
    out.extend((file_size as u32).to_le_bytes());
    out.extend([0u8; 4]);
    out.extend((pixel_offset as u32).to_le_bytes());
    out.extend(40u32.to_le_bytes());
    out.extend((width as i32).to_le_bytes());
    out.extend((height as i32).to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    out.extend((image_size as u32).to_le_bytes());
    out.extend(0i32.to_le_bytes());
    out.extend(0i32.to_le_bytes());
    out.extend(2u32.to_le_bytes());
    out.extend(2u32.to_le_bytes());
    out.extend([255, 255, 255, 0]);
    out.extend([0, 0, 0, 0]);

    for y in (0..height as usize).rev() {
        let start = y * row_stride;
        out.extend(&packed[start..start + row_stride]);
    }

    fs::write(path, out).map_err(|err| format!("write {path:?}: {err}"))
}

fn pack_rows(width: u32, height: u32, bits: &[u8]) -> Result<Vec<u8>, String> {
    pack_rows_with_stride(width, height, bits, width.div_ceil(8) as usize)
}

fn pack_rows_with_stride(
    width: u32,
    height: u32,
    bits: &[u8],
    stride: usize,
) -> Result<Vec<u8>, String> {
    if bits.len() != (width * height) as usize {
        return Err(format!(
            "bitmap has {} pixels, expected {}",
            bits.len(),
            width * height
        ));
    }
    let mut packed = vec![0u8; stride * height as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            if bits[y * width as usize + x] != 0 {
                packed[y * stride + (x >> 3)] |= 0x80 >> (x & 7);
            }
        }
    }
    Ok(packed)
}

fn write_overlay(path: &Path, img: &RgbaImage, geometry: &Geometry) -> Result<(), String> {
    let mut overlay = img.clone();
    let blue = Rgba([0, 96, 255, 160]);
    let bright_blue = Rgba([0, 180, 255, 220]);
    let crop_green = Rgba([160, 255, 180, 70]);
    let grid_red = Rgba([255, 140, 140, 230]);
    fill_non_sampling_area(&mut overlay, geometry, crop_green);
    draw_rect(
        &mut overlay,
        geometry.left,
        geometry.top,
        geometry.right,
        geometry.bottom,
        grid_red,
        1,
    );
    for y in 0..PAGE_HEIGHT {
        for x in 0..PAGE_WIDTH {
            let cx = sample_x(geometry, x).round() as i32;
            let cy = sample_y(geometry, y).round() as i32;
            draw_rect_i32(
                &mut overlay,
                cx - SAMPLE_RADIUS,
                cy - SAMPLE_RADIUS,
                cx + SAMPLE_RADIUS,
                cy + SAMPLE_RADIUS,
                blue,
                1,
            );
            blend_pixel(&mut overlay, cx, cy, bright_blue);
        }
    }
    overlay
        .save(path)
        .map_err(|err| format!("write overlay {path:?}: {err}"))
}

fn fill_non_sampling_area(img: &mut RgbaImage, geometry: &Geometry, color: Rgba<u8>) {
    let right = img.width().saturating_sub(1) as i32;
    let bottom = img.height().saturating_sub(1) as i32;
    let grid_left = geometry.left as i32;
    let grid_right = geometry.right as i32;
    let grid_top = geometry.top as i32;
    let grid_bottom = geometry.bottom as i32;

    if grid_top > 0 {
        fill_rect_i32(img, 0, 0, right, grid_top - 1, color);
    }
    if grid_bottom < bottom {
        fill_rect_i32(img, 0, grid_bottom + 1, right, bottom, color);
    }
    if grid_left > 0 {
        fill_rect_i32(img, 0, grid_top, grid_left - 1, grid_bottom, color);
    }
    if grid_right < right {
        fill_rect_i32(img, grid_right + 1, grid_top, right, grid_bottom, color);
    }
}

fn draw_rect(
    img: &mut RgbaImage,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    color: Rgba<u8>,
    thickness: u32,
) {
    draw_rect_i32(
        img,
        left as i32,
        top as i32,
        right as i32,
        bottom as i32,
        color,
        thickness as i32,
    );
}

fn draw_rect_i32(
    img: &mut RgbaImage,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    color: Rgba<u8>,
    thickness: i32,
) {
    for inset in 0..thickness {
        for x in left + inset..=right - inset {
            blend_pixel(img, x, top + inset, color);
            blend_pixel(img, x, bottom - inset, color);
        }
        for y in top + inset..=bottom - inset {
            blend_pixel(img, left + inset, y, color);
            blend_pixel(img, right - inset, y, color);
        }
    }
}

fn fill_rect_i32(
    img: &mut RgbaImage,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    color: Rgba<u8>,
) {
    for y in top..=bottom {
        for x in left..=right {
            blend_pixel(img, x, y, color);
        }
    }
}

fn blend_pixel(img: &mut RgbaImage, x: i32, y: i32, over: Rgba<u8>) {
    if x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
        return;
    }
    let base = img.get_pixel_mut(x as u32, y as u32);
    let a = over[3] as u32;
    for idx in 0..3 {
        base[idx] = ((over[idx] as u32 * a + base[idx] as u32 * (255 - a)) / 255) as u8;
    }
    base[3] = 255;
}

fn write_provenance(
    path: &Path,
    source: &Path,
    geometry: &Geometry,
    raster_sha: &str,
) -> Result<(), String> {
    let text = format!(
        "\
# Annex H Figure H.1 Oracle Provenance

- Source image: `{}`
- Spec figure: ITU-T T.88 Annex H, Figure H.1 (`Test datastream page bitmap`)
- Output pages: `annex-h-page-00.bmp`, `annex-h-page-01.bmp`
- Page dimensions: `{PAGE_WIDTH} x {PAGE_HEIGHT}`
- Crop bounds: `top={}`, `bottom={}`
- Grid bounds: `left={}`, `right={}`, `top={}`, `bottom={}`
- Cell spacing: `x={:.6}`, `y={:.6}`
- Sampling: median luminance over `9 x 9` windows at logical cell centers
- Classification: `median_luma < 128` maps to black/ink (`1`)
- BMP palette: index 0 = white, index 1 = black
- Packed logical raster SHA-256: `{raster_sha}`
- Note: Annex H states that pages 1 and 2 decode to identical bitmaps; `annex-h-page-01.bmp` is intentionally duplicated from `annex-h-page-00.bmp`.

The generated BMPs are spec-derived from the Figure H.1 grid. `jbig2dec` output may be used as a diagnostic cross-check, but it is not the source of this oracle.
",
        source.display(),
        geometry.crop_top,
        geometry.crop_bottom,
        geometry.left,
        geometry.right,
        geometry.top,
        geometry.bottom,
        geometry.x_step,
        geometry.y_step,
    );
    fs::write(path, text).map_err(|err| format!("write {path:?}: {err}"))
}

fn cross_check_jbig2dec(path: &Path, bits: &[u8]) -> Result<(), String> {
    if !path.is_file() {
        println!(
            "jbig2dec diagnostic cross-check skipped: {:?} missing",
            path
        );
        return Ok(());
    }
    let pages = parse_pbm_sequence(path)?;
    let Some((width, height, page0)) = pages.first() else {
        println!(
            "jbig2dec diagnostic cross-check skipped: no pages in {:?}",
            path
        );
        return Ok(());
    };
    if (*width, *height) != (PAGE_WIDTH, PAGE_HEIGHT) {
        println!(
            "jbig2dec diagnostic cross-check skipped: page 0 is {}x{}",
            width, height
        );
        return Ok(());
    }
    let expected = pack_rows(PAGE_WIDTH, PAGE_HEIGHT, bits)?;
    if page0 == &expected {
        println!("jbig2dec diagnostic cross-check: page 0 matches generated BMP raster");
    } else {
        println!("jbig2dec diagnostic cross-check: page 0 differs from generated BMP raster");
    }
    if pages.len() > 1 && pages[1].0 == PAGE_WIDTH && pages[1].1 == PAGE_HEIGHT {
        if pages[1].2 == expected {
            println!("jbig2dec diagnostic cross-check: page 1 matches generated BMP raster");
        } else {
            println!("jbig2dec diagnostic cross-check: page 1 differs from generated BMP raster");
        }
    }
    Ok(())
}

fn parse_pbm_sequence(path: &Path) -> Result<Vec<(u32, u32, Vec<u8>)>, String> {
    let data = fs::read(path).map_err(|err| format!("read {path:?}: {err}"))?;
    let mut cursor = 0usize;
    let mut pages = Vec::new();
    skip_ws_and_comments(&data, &mut cursor);
    while cursor < data.len() {
        if data.len().saturating_sub(cursor) < 2 || &data[cursor..cursor + 2] != b"P4" {
            return Err(format!("{path:?}: not a P4 PBM sequence"));
        }
        cursor += 2;
        let width = read_pbm_u32(&data, &mut cursor)?;
        let height = read_pbm_u32(&data, &mut cursor)?;
        if cursor >= data.len() || !data[cursor].is_ascii_whitespace() {
            return Err("pbm truncated before raster".to_string());
        }
        cursor += 1;
        let stride = width.div_ceil(8) as usize;
        let needed = stride * height as usize;
        if data.len().saturating_sub(cursor) < needed {
            return Err(format!("pbm raster shorter than declared {width}x{height}"));
        }
        pages.push((width, height, data[cursor..cursor + needed].to_vec()));
        cursor += needed;
        skip_ws_and_comments(&data, &mut cursor);
    }
    Ok(pages)
}

fn skip_ws_and_comments(data: &[u8], cursor: &mut usize) {
    while *cursor < data.len() {
        match data[*cursor] {
            b' ' | b'\t' | b'\n' | b'\r' => *cursor += 1,
            b'#' => {
                while *cursor < data.len() && data[*cursor] != b'\n' {
                    *cursor += 1;
                }
            }
            _ => break,
        }
    }
}

fn read_pbm_u32(data: &[u8], cursor: &mut usize) -> Result<u32, String> {
    skip_ws_and_comments(data, cursor);
    let start = *cursor;
    while *cursor < data.len() && data[*cursor].is_ascii_digit() {
        *cursor += 1;
    }
    std::str::from_utf8(&data[start..*cursor])
        .map_err(|err| err.to_string())?
        .parse()
        .map_err(|err| format!("pbm dimension parse: {err}"))
}

fn sha256_hex(data: &[u8]) -> String {
    // Small, local SHA-256 implementation avoids adding a dependency for a diagnostic.
    const H0: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut msg = data.to_vec();
    let bit_len = (msg.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend(bit_len.to_be_bytes());

    let mut h = H0;
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (idx, bytes) in chunk.chunks_exact(4).enumerate() {
            w[idx] = u32::from_be_bytes(bytes.try_into().unwrap());
        }
        for idx in 16..64 {
            let s0 =
                w[idx - 15].rotate_right(7) ^ w[idx - 15].rotate_right(18) ^ (w[idx - 15] >> 3);
            let s1 = w[idx - 2].rotate_right(17) ^ w[idx - 2].rotate_right(19) ^ (w[idx - 2] >> 10);
            w[idx] = w[idx - 16]
                .wrapping_add(s0)
                .wrapping_add(w[idx - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for idx in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[idx])
                .wrapping_add(w[idx]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(value);
        }
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
}
