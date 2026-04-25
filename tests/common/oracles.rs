use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use jbig2::Bitmap;

static WARNED_MISSING_JBIG2DEC: OnceLock<()> = OnceLock::new();
static WARNED_MISSING_JBIG2_IMAGEIO: OnceLock<()> = OnceLock::new();
static ORACLE_SEQ: AtomicUsize = AtomicUsize::new(0);

pub fn decode_with_jbig2dec(jbig2_bytes: &[u8]) -> Option<Bitmap> {
    let Some(bin) = jbig2dec_bin() else {
        WARNED_MISSING_JBIG2DEC.get_or_init(|| {
            eprintln!("symbol refinement corpus: skipping jbig2dec oracle (missing JBIG2DEC_BIN / jbig2dec on PATH)");
        });
        return None;
    };

    let workdir = oracle_workdir("jbig2dec");
    let input = workdir.join("input.jb2");
    let output = workdir.join("output.pbm");
    std::fs::write(&input, jbig2_bytes).expect("write jbig2dec oracle input");

    let status = Command::new(&bin)
        .arg("--format")
        .arg("pbm")
        .arg("-o")
        .arg(&output)
        .arg(&input)
        .status()
        .unwrap_or_else(|err| panic!("spawn jbig2dec {:?}: {err}", bin));
    assert!(status.success(), "jbig2dec {:?} exited with {status}", bin);
    Some(load_pbm_p4(&output))
}

pub fn decode_with_jbig2_imageio(jbig2_bytes: &[u8]) -> Option<Bitmap> {
    let Some(cmd) = jbig2_imageio_cmd() else {
        WARNED_MISSING_JBIG2_IMAGEIO.get_or_init(|| {
            eprintln!(
                "symbol refinement corpus: skipping jbig2-imageio oracle (missing JBIG2_IMAGEIO_CMD)"
            );
        });
        return None;
    };

    let workdir = oracle_workdir("jbig2-imageio");
    let input = workdir.join("input.jb2");
    let output = workdir.join("output.pbm");
    std::fs::write(&input, jbig2_bytes).expect("write jbig2-imageio oracle input");

    let program = &cmd[0];
    let status = Command::new(program)
        .args(&cmd[1..])
        .arg(&input)
        .arg(&output)
        .status()
        .unwrap_or_else(|err| panic!("spawn jbig2-imageio {:?}: {err}", program));
    assert!(
        status.success(),
        "jbig2-imageio {:?} exited with {status}",
        program
    );
    Some(load_pbm_p4(&output))
}

fn oracle_workdir(tag: &str) -> PathBuf {
    let seq = ORACLE_SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_tmp")
        .join(format!("{tag}-{}-{seq}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create oracle workdir");
    dir
}

fn jbig2dec_bin() -> Option<PathBuf> {
    let candidates = [
        std::env::var_os("JBIG2DEC_BIN").map(PathBuf::from),
        Some(PathBuf::from("/opt/homebrew/bin/jbig2dec")),
        Some(PathBuf::from("/usr/local/bin/jbig2dec")),
        which_on_path("jbig2dec"),
    ];
    candidates.into_iter().flatten().find(|path| path.is_file())
}

fn jbig2_imageio_cmd() -> Option<Vec<OsString>> {
    let raw = std::env::var("JBIG2_IMAGEIO_CMD").ok()?;
    match shlex::split(&raw) {
        Some(parts) if !parts.is_empty() => Some(parts.into_iter().map(OsString::from).collect()),
        _ => None,
    }
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

fn load_pbm_p4(path: &Path) -> Bitmap {
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
    assert!(cursor < data.len(), "pbm truncated before raster");
    cursor += 1;

    let stride = ((width + 7) / 8) as usize;
    let needed = stride * height as usize;
    assert!(
        data.len().saturating_sub(cursor) >= needed,
        "pbm raster shorter than declared {}x{}",
        width,
        height
    );

    let mut bm = Bitmap::new(width, height).unwrap();
    for y in 0..height as usize {
        let src = &data[cursor + y * stride..cursor + (y + 1) * stride];
        bm.row_mut(y).copy_from_slice(src);
    }
    if width & 7 != 0 {
        let mask = 0xFFu8 << (8 - (width & 7));
        for y in 0..height as usize {
            let row = bm.row_mut(y);
            let last = row.len() - 1;
            row[last] &= mask;
        }
    }
    bm
}
