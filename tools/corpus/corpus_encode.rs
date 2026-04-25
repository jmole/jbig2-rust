use std::env;
use std::fs;
use std::io::BufWriter;
use std::path::Path;
use std::process::ExitCode;

use jbig2::{Bitmap, Coding, EncoderConfig, GenericTemplate, Jbig2Encoder, Mode};

const THRESHOLD: u8 = 128;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("corpus-encode: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [cmd, input, output] if cmd == "bmp" => {
            let bitmap = load_bitmap(Path::new(input))?;
            save_bmp_1bpp(Path::new(output), &bitmap)
        }
        [cmd, preset, input, output] if cmd == "rust" => {
            let bitmap = load_bitmap(Path::new(input))?;
            encode_rust(preset, Path::new(output), &bitmap)
        }
        _ => Err(
            "usage: corpus-encode bmp <input-image> <output.bmp> | corpus-encode rust <preset> <input-image> <output.jb2>"
                .to_string(),
        ),
    }
}

fn load_bitmap(path: &Path) -> Result<Bitmap, String> {
    let gray = image::open(path)
        .map_err(|err| format!("open {path:?}: {err}"))?
        .into_luma8();
    let (width, height) = gray.dimensions();
    let mut bitmap = Bitmap::new(width, height).map_err(|err| err.to_string())?;
    let width_usize = width as usize;
    let tail_bits = (width & 7) as u8;
    let tail_mask = if tail_bits == 0 {
        0xFF
    } else {
        0xFFu8 << (8 - tail_bits)
    };

    for y in 0..height as usize {
        let src = &gray.as_raw()[y * width_usize..(y + 1) * width_usize];
        let row = bitmap.row_mut(y);
        for (x, &pix) in src.iter().enumerate() {
            if pix < THRESHOLD {
                row[x >> 3] |= 1u8 << (7 - (x & 7));
            }
        }
        if tail_mask != 0xFF {
            let last = row.len() - 1;
            row[last] &= tail_mask;
        }
    }
    Ok(bitmap)
}

fn encode_rust(preset: &str, output: &Path, bitmap: &Bitmap) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create {parent:?}: {err}"))?;
    }
    let file = fs::File::create(output).map_err(|err| format!("create {output:?}: {err}"))?;
    let mut encoder = Jbig2Encoder::new(BufWriter::new(file), rust_config(preset)?);
    encoder
        .write_page(bitmap)
        .map_err(|err| format!("rust encode write_page failed: {err}"))?;
    encoder
        .finish()
        .map(|_| ())
        .map_err(|err| format!("rust encode finish failed: {err}"))
}

fn rust_config(preset: &str) -> Result<EncoderConfig, String> {
    match preset {
        "fast" => Ok(EncoderConfig::fast()),
        "balanced" => Ok(EncoderConfig::balanced()),
        "max" => Ok(EncoderConfig::max_compression()),
        "generic-t0-no-tpgd" => Ok(EncoderConfig {
            mode: Mode::Generic,
            template: GenericTemplate::T0,
            coding: Coding::Arithmetic,
            adaptive_templates: None,
            refinement: false,
            generic_region_duplicate_line_removal: false,
            symbol_threshold: 0.97,
            refine_after_match: false,
        }),
        "generic-t0-tpgd" => Ok(EncoderConfig {
            generic_region_duplicate_line_removal: true,
            ..rust_config("generic-t0-no-tpgd")?
        }),
        "symbol-lossy-t85" => Ok(EncoderConfig {
            mode: Mode::SymbolLossy,
            template: GenericTemplate::T0,
            coding: Coding::Arithmetic,
            adaptive_templates: None,
            refinement: false,
            generic_region_duplicate_line_removal: true,
            symbol_threshold: 0.85,
            refine_after_match: false,
        }),
        other => Err(format!("unknown rust preset {other:?}")),
    }
}

fn save_bmp_1bpp(path: &Path, bitmap: &Bitmap) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create {parent:?}: {err}"))?;
    }
    let width = bitmap.width();
    let height = bitmap.height();
    let src_stride = bitmap.stride();
    let row_bytes = (((width + 31) / 32) * 4) as usize;
    let pixel_offset = 14 + 40 + 8;
    let image_size = row_bytes * height as usize;
    let file_size = pixel_offset + image_size;

    let mut out = Vec::with_capacity(file_size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(file_size as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 4]);
    out.extend_from_slice(&(pixel_offset as u32).to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&(height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(image_size as u32).to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0x00]);
    out.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    let padding = row_bytes.saturating_sub(src_stride);
    let zero_pad = vec![0u8; padding];
    for y in (0..height as usize).rev() {
        out.extend_from_slice(bitmap.row(y));
        out.extend_from_slice(&zero_pad);
    }
    fs::write(path, out).map_err(|err| format!("write {path:?}: {err}"))
}
