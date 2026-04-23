use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub(crate) struct ExternalTimingConfig {
    pub(crate) sample_size: u64,
    pub(crate) measurement_time: Duration,
    pub(crate) warm_up_time: Duration,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ExternalTimingStats {
    pub(crate) mean_ns: f64,
}

fn parse_duration_arg(raw: &str) -> Option<Duration> {
    if let Some(ms) = raw.strip_suffix("ms") {
        return ms
            .trim()
            .parse::<f64>()
            .ok()
            .map(Duration::from_secs_f64)
            .map(|d| d.div_f64(1000.0));
    }
    if let Some(secs) = raw.strip_suffix('s') {
        return secs.trim().parse::<f64>().ok().map(Duration::from_secs_f64);
    }
    raw.trim().parse::<f64>().ok().map(Duration::from_secs_f64)
}

pub(crate) fn external_timing_config() -> ExternalTimingConfig {
    static CFG: OnceLock<ExternalTimingConfig> = OnceLock::new();
    *CFG.get_or_init(|| {
        let mut cfg = ExternalTimingConfig {
            sample_size: 20,
            measurement_time: Duration::from_secs(5),
            warm_up_time: Duration::from_secs(3),
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            if let Some(value) = arg.strip_prefix("--sample-size=") {
                if let Ok(sample_size) = value.parse::<u64>() {
                    cfg.sample_size = sample_size.max(1);
                }
                continue;
            }
            if let Some(value) = arg.strip_prefix("--measurement-time=") {
                if let Some(duration) = parse_duration_arg(value) {
                    cfg.measurement_time = duration;
                }
                continue;
            }
            if let Some(value) = arg.strip_prefix("--warm-up-time=") {
                if let Some(duration) = parse_duration_arg(value) {
                    cfg.warm_up_time = duration;
                }
                continue;
            }

            match arg.as_str() {
                "--sample-size" => {
                    if let Some(value) = args.next() {
                        if let Ok(sample_size) = value.parse::<u64>() {
                            cfg.sample_size = sample_size.max(1);
                        }
                    }
                }
                "--measurement-time" => {
                    if let Some(value) = args.next() {
                        if let Some(duration) = parse_duration_arg(&value) {
                            cfg.measurement_time = duration;
                        }
                    }
                }
                "--warm-up-time" => {
                    if let Some(value) = args.next() {
                        if let Some(duration) = parse_duration_arg(&value) {
                            cfg.warm_up_time = duration;
                        }
                    }
                }
                _ => {}
            }
        }
        cfg
    })
}

fn run_subprocess<F>(build: &mut F) -> Duration
where
    F: FnMut() -> Command,
{
    let mut cmd = build();
    cmd.stderr(std::process::Stdio::null());
    let start = Instant::now();
    let out = cmd.status().expect("reference tool failed to spawn");
    let elapsed = start.elapsed();
    assert!(out.success(), "reference tool exited non-zero");
    elapsed
}

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
        total += run_subprocess(&mut build);
    }
    total
}

pub(crate) fn sample_subprocess<F>(mut build: F) -> ExternalTimingStats
where
    F: FnMut() -> Command,
{
    let cfg = external_timing_config();
    let mut warmup_iters = 0u64;
    let warmup_start = Instant::now();
    while warmup_start.elapsed() < cfg.warm_up_time {
        run_subprocess(&mut build);
        warmup_iters += 1;
    }

    let warmup_ns = warmup_start.elapsed().as_secs_f64() * 1e9;
    let mean_iter_ns = if warmup_iters == 0 {
        run_subprocess(&mut build).as_secs_f64() * 1e9
    } else {
        warmup_ns / warmup_iters as f64
    };
    let target_sample_ns = (cfg.measurement_time.as_secs_f64() * 1e9) / cfg.sample_size as f64;
    let iters_per_sample = ((target_sample_ns / mean_iter_ns).ceil() as u64).max(1);

    let mut sample_total_ns = 0.0;
    for _ in 0..cfg.sample_size {
        let elapsed = measure_subprocess(iters_per_sample, || build());
        sample_total_ns += (elapsed.as_secs_f64() * 1e9) / iters_per_sample as f64;
    }

    ExternalTimingStats {
        mean_ns: sample_total_ns / cfg.sample_size as f64,
    }
}
