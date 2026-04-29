use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use jbig2::{Bitmap, Coding, EncoderConfig, GenericTemplate, Jbig2Encoder, Mode};
use jbig2_workbench::util::sandbox::{Sandbox, SandboxOutcome};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;

const SAMPLE_COUNT: usize = 20;
const SAMPLE_SEED: u64 = 0x4a42494732;
const THRESHOLD: u8 = 128;

#[derive(Clone, Debug)]
struct CorpusEntry {
    copied_path: PathBuf,
}

#[derive(Clone, Copy)]
enum Check {
    BitIdentical,
    DecodeOnly,
}

#[derive(Clone, Copy)]
struct EncoderCase {
    name: &'static str,
    config: fn() -> EncoderConfig,
    check: Check,
}

#[test]
fn jbig2dec_decodes_rust_encoder_outputs() -> Result<(), String> {
    let Some(jbig2dec) = jbig2dec_bin() else {
        eprintln!("skipping jbig2dec output compatibility test: jbig2dec not found");
        return Ok(());
    };

    let samples = sampled_corpus_entries(SAMPLE_COUNT)?;
    let workdir = workdir()?;
    let cases = encoder_cases();

    for (sample_idx, entry) in samples.iter().enumerate() {
        let source = load_bitmap(&entry.copied_path)?;
        for case in cases {
            let stream = encode_with_config(&source, (case.config)()).map_err(|err| {
                format!("{} {} encode failed: {err}", entry_label(entry), case.name)
            })?;
            let input = workdir.join(format!("{sample_idx:02}-{}.jb2", case.name));
            let output = workdir.join(format!("{sample_idx:02}-{}.pbm", case.name));
            fs::write(&input, &stream).map_err(|err| format!("write {input:?}: {err}"))?;
            decode_with_jbig2dec(&jbig2dec, &input, &output, entry, case.name)?;

            if matches!(case.check, Check::BitIdentical) {
                let decoded = load_pbm_p4(&output)?;
                if decoded != source {
                    return Err(format!(
                        "{} {} jbig2dec bitmap mismatch: {}",
                        entry_label(entry),
                        case.name,
                        bitmap_diff_summary(&source, &decoded)
                    ));
                }
            }
        }
    }

    Ok(())
}

fn encoder_cases() -> [EncoderCase; 6] {
    [
        EncoderCase {
            name: "fast",
            config: EncoderConfig::fast,
            check: Check::BitIdentical,
        },
        EncoderCase {
            name: "balanced",
            config: EncoderConfig::balanced,
            check: Check::BitIdentical,
        },
        EncoderCase {
            name: "max-compression",
            config: EncoderConfig::max_compression,
            // Max-compression can use refinement coding. Refinement context
            // assignment is implementation-defined, and the decoder now
            // follows the ITU sample context order for conformance streams;
            // keep this row as an external-decoder smoke test rather than a
            // pixel oracle against jbig2dec's private context order.
            check: Check::DecodeOnly,
        },
        EncoderCase {
            name: "generic-t0-no-tpgd",
            config: generic_t0_no_tpgd,
            check: Check::BitIdentical,
        },
        EncoderCase {
            name: "generic-t0-tpgd",
            config: generic_t0_tpgd,
            check: Check::BitIdentical,
        },
        EncoderCase {
            name: "symbol-lossy-t85",
            config: symbol_lossy_t85,
            check: Check::DecodeOnly,
        },
    ]
}

fn generic_t0_no_tpgd() -> EncoderConfig {
    EncoderConfig {
        mode: Mode::Generic,
        template: GenericTemplate::T0,
        coding: Coding::Arithmetic,
        adaptive_templates: None,
        refinement: false,
        generic_region_duplicate_line_removal: false,
        symbol_threshold: 0.97,
        refine_after_match: false,
        ..EncoderConfig::fast()
    }
}

fn generic_t0_tpgd() -> EncoderConfig {
    EncoderConfig {
        generic_region_duplicate_line_removal: true,
        ..generic_t0_no_tpgd()
    }
}

fn symbol_lossy_t85() -> EncoderConfig {
    EncoderConfig {
        mode: Mode::SymbolLossy,
        template: GenericTemplate::T0,
        coding: Coding::Arithmetic,
        adaptive_templates: None,
        refinement: false,
        generic_region_duplicate_line_removal: true,
        symbol_threshold: 0.85,
        refine_after_match: false,
        ..EncoderConfig::fast()
    }
}

fn sampled_corpus_entries(n: usize) -> Result<Vec<CorpusEntry>, String> {
    let mut entries = read_manifest()?;
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(SAMPLE_SEED);
    entries.shuffle(&mut rng);
    entries.truncate(n.min(entries.len()));
    Ok(entries)
}

fn read_manifest() -> Result<Vec<CorpusEntry>, String> {
    let manifest = corpus_dir().join("manifest.tsv");
    let file = File::open(&manifest).map_err(|err| format!("open {manifest:?}: {err}"))?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for (line_no, line) in reader.lines().enumerate() {
        let line = line.map_err(|err| format!("read manifest line {}: {err}", line_no + 1))?;
        let mut parts = line.splitn(4, '\t');
        let _source_path = parts
            .next()
            .ok_or_else(|| format!("manifest line {} missing source path", line_no + 1))?;
        let copied_rel = parts
            .next()
            .ok_or_else(|| format!("manifest line {} missing copied path", line_no + 1))?;
        entries.push(CorpusEntry {
            copied_path: corpus_dir().join(copied_rel),
        });
    }
    if entries.is_empty() {
        return Err(format!(
            "manifest {:?} did not contain any entries",
            manifest
        ));
    }
    Ok(entries)
}

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benchmark-corpus")
        .join("patent-500")
}

fn workdir() -> Result<PathBuf, String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("jbig2dec_output_compat")
        .join(std::process::id().to_string());
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).map_err(|err| format!("create {dir:?}: {err}"))?;
    Ok(dir)
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

fn encode_with_config(bitmap: &Bitmap, cfg: EncoderConfig) -> Result<Vec<u8>, String> {
    let mut encoded = Vec::new();
    let mut encoder = Jbig2Encoder::new(&mut encoded, cfg);
    encoder
        .write_bitmap(bitmap)
        .map_err(|err| format!("write page: {err}"))?;
    encoder.finish().map_err(|err| format!("finish: {err}"))?;
    Ok(encoded)
}

fn jbig2dec_bin() -> Option<PathBuf> {
    let candidates = [
        std::env::var_os("JBIG2DEC_BIN").map(PathBuf::from),
        Some(PathBuf::from("/opt/homebrew/bin/jbig2dec")),
        Some(PathBuf::from("/usr/local/bin/jbig2dec")),
        which_on_path("jbig2dec"),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|candidate| is_jbig2dec(candidate))
}

fn which_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn is_jbig2dec(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let Ok(out) = Command::new(path)
        .arg("-V")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    else {
        return false;
    };
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    combined.contains("jbig2dec")
}

fn decode_with_jbig2dec(
    jbig2dec: &Path,
    input: &Path,
    output: &Path,
    entry: &CorpusEntry,
    case_name: &str,
) -> Result<(), String> {
    let mut cmd = Command::new(jbig2dec);
    cmd.arg("-q")
        .arg("--format")
        .arg("pbm")
        .arg("-o")
        .arg(output)
        .arg(input);

    let workdir = output.parent().unwrap_or_else(|| Path::new("."));
    let SandboxOutcome {
        output: out,
        kill_reason,
        wall_elapsed,
    } = decoder_sandbox(workdir).run(cmd).map_err(|err| {
        format!(
            "{} {case_name} jbig2dec spawn failed for {input:?}: {err}",
            entry_label(entry)
        )
    })?;
    if let Some(reason) = kill_reason {
        return Err(format!(
            "{} {case_name} jbig2dec sandbox-killed ({reason:?}) after {:.1}s for {input:?}: {}",
            entry_label(entry),
            wall_elapsed.as_secs_f32(),
            String::from_utf8_lossy(&out.stderr).trim_end(),
        ));
    }
    if !out.status.success() {
        return Err(format!(
            "{} {case_name} jbig2dec failed for {input:?} with {}: {}",
            entry_label(entry),
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}

fn decoder_sandbox(workdir: &Path) -> Sandbox {
    let mut sb = Sandbox::for_decoder()
        .ro_path(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .rw_path(workdir.to_path_buf())
        .rw_path(PathBuf::from("/tmp"));
    if let Ok(extra) = std::env::var("JBIG2_SANDBOX_EXTRA_RO") {
        for path in std::env::split_paths(&extra) {
            sb = sb.ro_path(path);
        }
    }
    sb
}

fn load_pbm_p4(path: &Path) -> Result<Bitmap, String> {
    let mut data = Vec::new();
    File::open(path)
        .map_err(|err| format!("open PBM {path:?}: {err}"))?
        .read_to_end(&mut data)
        .map_err(|err| format!("read PBM {path:?}: {err}"))?;
    parse_pbm_p4(&data).map_err(|err| format!("parse PBM {path:?}: {err}"))
}

fn parse_pbm_p4(data: &[u8]) -> Result<Bitmap, String> {
    if data.len() < 3 || &data[0..2] != b"P4" {
        return Err("not a binary P4 PBM".to_string());
    }
    let mut cursor = 2usize;
    let width = read_pbm_u32(data, &mut cursor)?;
    let height = read_pbm_u32(data, &mut cursor)?;
    if cursor >= data.len() || !data[cursor].is_ascii_whitespace() {
        return Err("missing PBM header/raster separator".to_string());
    }
    // P4 raster bytes are binary; after the dimensions, only the single
    // required whitespace separator belongs to the header.
    cursor += 1;

    let stride = width.div_ceil(8) as usize;
    let raster_len = stride * height as usize;
    if data.len() < cursor + raster_len {
        return Err(format!("raster shorter than declared {width}x{height}"));
    }

    let mut bitmap = Bitmap::new(width, height).map_err(|err| err.to_string())?;
    for y in 0..height as usize {
        let row_start = cursor + y * stride;
        let row = bitmap.row_mut(y);
        row.copy_from_slice(&data[row_start..row_start + stride]);
        let tail_bits = width & 7;
        if tail_bits != 0 {
            let last = row.len() - 1;
            row[last] &= 0xFFu8 << (8 - tail_bits);
        }
    }
    Ok(bitmap)
}

fn read_pbm_u32(data: &[u8], cursor: &mut usize) -> Result<u32, String> {
    skip_pbm_ws_and_comments(data, cursor);
    let start = *cursor;
    while *cursor < data.len() && data[*cursor].is_ascii_digit() {
        *cursor += 1;
    }
    if start == *cursor {
        return Err("expected PBM integer".to_string());
    }
    let s = std::str::from_utf8(&data[start..*cursor]).map_err(|err| err.to_string())?;
    s.parse()
        .map_err(|err| format!("invalid PBM integer {s:?}: {err}"))
}

fn skip_pbm_ws_and_comments(data: &[u8], cursor: &mut usize) {
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

fn bitmap_diff_summary(expected: &Bitmap, actual: &Bitmap) -> String {
    if expected.width() != actual.width() || expected.height() != actual.height() {
        return format!(
            "dimensions {}x{} vs {}x{}",
            expected.width(),
            expected.height(),
            actual.width(),
            actual.height()
        );
    }
    for y in 0..expected.height() as usize {
        if expected.row(y) != actual.row(y) {
            return format!("first differing row {y}");
        }
    }
    "bitmaps differ".to_string()
}

fn entry_label(entry: &CorpusEntry) -> String {
    entry
        .copied_path
        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(&entry.copied_path)
        .display()
        .to_string()
}
