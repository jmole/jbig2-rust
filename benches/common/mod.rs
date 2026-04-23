//! Shared helpers for reference-implementation comparison benchmarks.
//!
//! Benches that compare this crate against external codec binaries reuse
//! this module to locate the vendored T.88 corpus, discover reference
//! binaries on disk, and load the small bitmap formats those tools read
//! and write.
//!
//! External binaries are discovered in this order:
//!
//! 1. An explicit env-var override (`JBIG2_T88_BIN`, `JBIG2ENC_BIN`,
//!    `JBIG2DEC_BIN`).
//! 2. A default location that matches the vendored / Homebrew layout
//!    (`vendor/...` for T.88, `/opt/homebrew/bin` or `/usr/local/bin` for
//!    the Homebrew `jbig2enc` / `jbig2dec` packages).
//! 3. Whatever `which` finds on `PATH` as a last resort.
//!
//! Every probe also checks that the resolved binary behaves like the tool
//! we expect (e.g. `jbig2enc -V` prints `jbig2enc`), so a collision with
//! the identically-named T.88 `jbig2` binary never silently swaps tools.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use jbig2::Bitmap;

pub fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("vendor")
        .join("T-REC-T.88-201808")
        .join("Software")
        .join("JBIG2_ConformanceData-A20180829")
}

/// Absolute path to a per-benchmark scratch directory rooted under
/// `target/`, so external tools can write outputs without polluting the
/// workspace.
pub fn workdir_for(tag: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("bench_tmp")
        .join(tag);
    // Recreate clean so stale outputs never poison a run.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create bench workdir");
    dir
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

fn from_env_or(default: &Path, env_var: &str) -> Option<PathBuf> {
    if let Some(v) = std::env::var_os(env_var) {
        let p = PathBuf::from(v);
        if p.is_file() {
            return Some(p);
        }
        eprintln!(
            "reference bench: {env_var} is set to {:?} but that path is not a regular file; falling back to defaults",
            p
        );
    }
    if default.is_file() {
        return Some(default.to_path_buf());
    }
    None
}

/// Resolve the vendored T.88 sample-software `jbig2` binary. This is the
/// normative reference that handles both encode and decode through a
/// stem/extension CLI.
pub fn t88_ref_bin() -> Option<PathBuf> {
    let default = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("vendor/T-REC-T.88-201808/Software/JBIG2_SampleSoftware-A20180829/source/jbig2");
    from_env_or(&default, "JBIG2_T88_BIN")
}

/// Resolve the Homebrew `jbig2enc` binary. The Homebrew package installs
/// itself as `jbig2`, which collides with the T.88 sample binary name, so
/// the probe verifies `-V` output actually identifies as `jbig2enc`
/// before returning a path.
pub fn jbig2enc_bin() -> Option<PathBuf> {
    let candidates = [
        std::env::var_os("JBIG2ENC_BIN").map(PathBuf::from),
        Some(PathBuf::from("/opt/homebrew/bin/jbig2")),
        Some(PathBuf::from("/usr/local/bin/jbig2")),
        which_on_path("jbig2enc"),
        which_on_path("jbig2"),
    ];
    for maybe in candidates.into_iter().flatten() {
        if !maybe.is_file() {
            continue;
        }
        if let Ok(out) = Command::new(&maybe)
            .arg("-V")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            if combined.contains("jbig2enc") {
                return Some(maybe);
            }
        }
    }
    None
}

/// Resolve Ghostscript's `jbig2dec` binary (the Homebrew package of the
/// same name).
pub fn jbig2dec_bin() -> Option<PathBuf> {
    let candidates = [
        std::env::var_os("JBIG2DEC_BIN").map(PathBuf::from),
        Some(PathBuf::from("/opt/homebrew/bin/jbig2dec")),
        Some(PathBuf::from("/usr/local/bin/jbig2dec")),
        which_on_path("jbig2dec"),
    ];
    for maybe in candidates.into_iter().flatten() {
        if !maybe.is_file() {
            continue;
        }
        if let Ok(out) = Command::new(&maybe)
            .arg("-V")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            if combined.contains("jbig2dec") {
                return Some(maybe);
            }
        }
    }
    None
}

/// Parse a 1-bpp BMP (the format emitted by the T.88 sample decoder and
/// shipped alongside the conformance streams).
pub fn load_bmp_1bpp(path: &Path) -> Bitmap {
    let data = std::fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
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

    let pal0 = &data[54..58]; // (B, G, R, A)
    let zero_is_ink = pal0[0] <= 0x40 && pal0[1] <= 0x40 && pal0[2] <= 0x40;

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
        let last_bits = width_u & 7;
        if last_bits != 0 {
            let mask = 0xFFu8 << (8 - last_bits);
            let last = row.len() - 1;
            row[last] &= mask;
        }
    }
    bm
}

/// Parse a binary PBM (`P4`) file — the format emitted by `jbig2dec
/// --format pbm`. The PBM convention matches JBIG2 (`1 = ink`) so no
/// palette inversion is required.
pub fn load_pbm_p4(path: &Path) -> Bitmap {
    let data = std::fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    parse_pbm_p4(&data)
}

fn parse_pbm_p4(data: &[u8]) -> Bitmap {
    assert!(data.len() >= 3 && &data[0..2] == b"P4", "not a P4 PBM file");
    let mut cursor = 2usize;
    let skip_ws_and_comments = |cur: &mut usize| {
        while *cur < data.len() {
            match data[*cur] {
                b' ' | b'\t' | b'\n' | b'\r' => *cur += 1,
                b'#' => {
                    while *cur < data.len() && data[*cur] != b'\n' {
                        *cur += 1;
                    }
                }
                _ => break,
            }
        }
    };
    let read_u32 = |cur: &mut usize| -> u32 {
        skip_ws_and_comments(cur);
        let start = *cur;
        while *cur < data.len() && data[*cur].is_ascii_digit() {
            *cur += 1;
        }
        let s = std::str::from_utf8(&data[start..*cur]).expect("pbm digit ascii");
        s.parse().expect("pbm dimension parse")
    };
    let width = read_u32(&mut cursor);
    let height = read_u32(&mut cursor);
    // A single whitespace byte separates the header from the raster.
    assert!(cursor < data.len(), "pbm truncated before raster");
    cursor += 1;

    let stride = ((width + 7) / 8) as usize;
    assert!(
        data.len() >= cursor + stride * height as usize,
        "pbm raster shorter than declared {}x{}",
        width,
        height
    );
    let mut bm = Bitmap::new(width, height).unwrap();
    for y in 0..height {
        let row_start = cursor + y as usize * stride;
        let src = &data[row_start..row_start + stride];
        let row = bm.row_mut(y as usize);
        row.copy_from_slice(src);
        let last_bits = width & 7;
        if last_bits != 0 {
            let mask = 0xFFu8 << (8 - last_bits);
            let last = row.len() - 1;
            row[last] &= mask;
        }
    }
    bm
}

/// Run a command to completion, surfacing captured stderr on failure so
/// benchmark logs stay useful when a reference tool misbehaves.
///
/// stdout is intentionally *not* overridden: tools like `jbig2enc` pipe
/// their real output through stdout to a caller-supplied File, and
/// overwriting that here would silently produce empty output files.
pub fn run_quiet(cmd: &mut Command, label: &str) {
    let out = cmd
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("{label}: spawn failed: {e}"));
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        panic!("{label}: non-zero exit {}: {err}", out.status);
    }
}
