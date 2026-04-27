use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

pub(crate) fn t88_decode_cmd(bin: &Path, input_stem: &Path, out_stem: &Path) -> Command {
    let mut c = Command::new(bin);
    c.arg("-i")
        .arg(input_stem)
        .arg("-f")
        .arg("jb2")
        .arg("-o")
        .arg(out_stem)
        .arg("-F")
        .arg("bmp")
        .stdout(std::process::Stdio::null());
    c
}

pub(crate) fn t88_encode_cmd(bin: &Path, input_stem: &Path, out_stem: &Path) -> Command {
    let mut c = Command::new(bin);
    c.arg("-i")
        .arg(input_stem)
        .arg("-f")
        .arg("bmp")
        .arg("-o")
        .arg(out_stem)
        .arg("-F")
        .arg("jb2")
        .stdout(std::process::Stdio::null());
    c
}

pub(crate) fn jbig2dec_cmd(bin: &Path, input: &Path, out: &Path) -> Command {
    let mut c = Command::new(bin);
    c.arg("-q")
        .arg("--format")
        .arg("pbm")
        .arg("-o")
        .arg(out)
        .arg(input)
        .stdout(std::process::Stdio::null());
    c
}

pub(crate) fn jbig2enc_cmd(bin: &Path, input: &Path, out: &Path) -> Command {
    let mut c = Command::new(bin);
    c.arg(input).stdout(
        fs::File::create(out).unwrap_or_else(|e| panic!("create {out:?}: {e}")),
    );
    c
}

pub(crate) fn measure_subprocess<F>(iters: u64, mut build: F) -> Duration
where
    F: FnMut() -> Command,
{
    let mut total = Duration::ZERO;
    for _ in 0..iters {
        let mut cmd = build();
        cmd.stderr(std::process::Stdio::null());
        let start = Instant::now();
        let out = cmd.status().expect("reference tool failed to spawn");
        total += start.elapsed();
        assert!(out.success(), "reference tool exited non-zero");
    }
    total
}
