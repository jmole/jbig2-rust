//! Reference-implementation comparison benchmarks.
//!
//! This bench target measures wall-clock time for encoding and decoding
//! the same pages through three codecs side-by-side:
//!
//! * this crate (`Jbig2Decoder` / `Jbig2Encoder`),
//! * the normative **T.88 sample software** (`vendor/T-REC-T.88-201808`),
//! * Homebrew's **`jbig2enc`** (encoder) and **`jbig2dec`** (decoder).
//!
//! External tools add subprocess startup to each reported sample, which
//! is intentional: users picking between tools care about the total cost
//! of running them, not just the internal codec loop. Keeping the
//! process-level comparison in its own bench target preserves the
//! startup-noise-free numbers in
//! [`codec.rs`](./codec.rs) as the authoritative in-process hot-path
//! signal.
//!
//! Each group runs once per reference implementation per input file,
//! skipping the tool cleanly if its binary is missing. Before timing,
//! the setup pass runs the tool once and validates that the decoded
//! bitmap matches the vendored reference BMP (decode) or that the
//! encoder output round-trips back to the source bitmap (encode), so a
//! silently wrong tool cannot masquerade as "fast".
//!
//! Input size throughput is reported in **page bytes** (`width * height
//! / 8`) to keep decode and encode directly comparable even when the
//! compressed stream sizes differ.
//!
//! # Summary output
//!
//! After every run the bench harness prints a cross-tool summary table
//! to stderr (one block per `(side, case)`), followed by a `file://`
//! link to Criterion's HTML report. The table is built from two
//! sources:
//!
//! * **Byte sizes** are recorded synchronously during the pre-benchmark
//!   probe into [`ProbeRecord`] entries in [`probes()`].
//! * **Timings** are recovered post-run by parsing Criterion's
//!   `target/criterion/<group>/<tool>/<case>/new/estimates.json` files,
//!   which is already the statistical source of truth for this bench.
//!
//! Criterion's default per-bench stdout is preserved unchanged — the
//! table is additive and exists only because scanning 12+ sibling
//! Criterion rows for "did rust beat jbig2enc?" is a bad UX for the
//! deliverable of this bench target.

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use criterion::{
    black_box, criterion_group, measurement::WallTime, BenchmarkGroup, BenchmarkId, Criterion,
    Throughput,
};
use jbig2::{Bitmap, EncoderConfig, Jbig2Decoder, Jbig2Encoder};
use plotters::coord::Shift;
use plotters::prelude::*;

mod common;
use common::*;

// ---------------------------------------------------------------------------
// Probe record keeping
// ---------------------------------------------------------------------------

/// One row of the post-run summary table. Filled in by every bench
/// function before it hands off to Criterion, so the summary reflects
/// whichever subset of tools and cases actually ran on this host.
#[derive(Clone, Debug)]
struct ProbeRecord {
    /// `"decode"` or `"encode"` — the pipeline side. Used to find the
    /// Criterion group directory (`<side>_reference`) and to organise
    /// the summary.
    side: &'static str,
    tool: &'static str,
    case: &'static str,
    raw_bytes: u64,
    compressed_bytes: u64,
}

fn probes() -> &'static Mutex<Vec<ProbeRecord>> {
    static R: OnceLock<Mutex<Vec<ProbeRecord>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(Vec::new()))
}

fn record(r: ProbeRecord) {
    probes().lock().unwrap().push(r);
}

// ---------------------------------------------------------------------------
// Corpus
// ---------------------------------------------------------------------------

/// A single decode comparison case: a `.jb2` codestream paired with the
/// reference BMP the T.88 sample decoder produces, for correctness
/// cross-checking.
struct DecodeCase {
    tag: &'static str,
    jb2: &'static str,
    ref_bmp: &'static str,
}

const DECODE_CASES: &[DecodeCase] = &[
    DecodeCase {
        tag: "tt9_mmr",
        jb2: "F01_200_TT9.jb2",
        ref_bmp: "F01_200_TT9_TT00.bmp",
    },
    DecodeCase {
        tag: "tt10_arith",
        jb2: "F01_200_TT10.jb2",
        ref_bmp: "F01_200_TT10_TT00.bmp",
    },
];

/// A single encode comparison case: the reference BMP every tool reads,
/// plus a stable tag used for output file naming.
struct EncodeCase {
    tag: &'static str,
    bmp: &'static str,
}

const ENCODE_CASES: &[EncodeCase] = &[
    EncodeCase {
        tag: "tt9_page",
        bmp: "F01_200_TT9_TT00.bmp",
    },
    EncodeCase {
        tag: "tt10_page",
        bmp: "F01_200_TT10_TT00.bmp",
    },
];

// ---------------------------------------------------------------------------
// Shared helpers scoped to this bench target
// ---------------------------------------------------------------------------

fn page_bytes(bitmap: &Bitmap) -> u64 {
    (bitmap.width() as u64 * bitmap.height() as u64) / 8
}

/// Stable T.88 decode invocation: `jbig2 -i <stem> -f jb2 -o <out_stem>
/// -F bmp`. The T.88 binary always appends `00` to the output stem.
///
/// All cmd constructors fully own their stdio configuration: the runners
/// below only adjust stderr, because tools that deliver output through
/// stdout (jbig2enc) would otherwise have their output file truncated.
fn t88_decode_cmd(bin: &Path, input_stem: &Path, out_stem: &Path) -> Command {
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

fn t88_encode_cmd(bin: &Path, input_stem: &Path, out_stem: &Path) -> Command {
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

fn jbig2dec_cmd(bin: &Path, input: &Path, out: &Path) -> Command {
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

/// `jbig2enc` writes its JBIG2 stream to stdout by default; redirect that
/// into a file here so the timing includes only the tool's work and the
/// bench runner never overrides it.
fn jbig2enc_cmd(bin: &Path, input: &Path, out: &Path) -> Command {
    let mut c = Command::new(bin);
    c.arg(input).stdout(
        fs::File::create(out).unwrap_or_else(|e| panic!("create {out:?}: {e}")),
    );
    c
}

fn measure_subprocess<F>(iters: u64, mut build: F) -> Duration
where
    F: FnMut() -> Command,
{
    let mut total = Duration::ZERO;
    for _ in 0..iters {
        let mut cmd = build();
        // Silence diagnostics but leave stdout configuration alone — some
        // tools (e.g. jbig2enc) stream their output through stdout to a
        // pre-opened File.
        cmd.stderr(std::process::Stdio::null());
        let start = Instant::now();
        let out = cmd.status().expect("reference tool failed to spawn");
        total += start.elapsed();
        assert!(out.success(), "reference tool exited non-zero");
    }
    total
}

// ---------------------------------------------------------------------------
// Decode comparison
// ---------------------------------------------------------------------------

fn bench_decode_comparison(c: &mut Criterion) {
    let mut g = c.benchmark_group("decode/reference");
    g.sample_size(20);

    let t88 = t88_ref_bin();
    let jbig2dec = jbig2dec_bin();
    if t88.is_none() {
        eprintln!(
            "decode/reference: T.88 binary not found; set JBIG2_T88_BIN or build vendor/T-REC-T.88-201808 to include it"
        );
    }
    if jbig2dec.is_none() {
        eprintln!(
            "decode/reference: jbig2dec not found on PATH; install via `brew install jbig2dec` or set JBIG2DEC_BIN"
        );
    }

    for case in DECODE_CASES {
        let jb2_path = corpus_dir().join(case.jb2);
        if !jb2_path.is_file() {
            eprintln!(
                "decode/reference/{}: missing corpus file {:?}, skipping",
                case.tag, jb2_path
            );
            continue;
        }
        let expected = load_bmp_1bpp(&corpus_dir().join(case.ref_bmp));
        g.throughput(Throughput::Bytes(page_bytes(&expected)));

        bench_rust_decode(&mut g, case, &jb2_path, &expected);
        if let Some(bin) = t88.as_deref() {
            bench_t88_decode(&mut g, case, &jb2_path, &expected, bin);
        }
        if let Some(bin) = jbig2dec.as_deref() {
            bench_jbig2dec_decode(&mut g, case, &jb2_path, &expected, bin);
        }
    }

    g.finish();
}

fn bench_rust_decode(
    g: &mut BenchmarkGroup<'_, WallTime>,
    case: &DecodeCase,
    jb2_path: &Path,
    expected: &Bitmap,
) {
    let bytes = fs::read(jb2_path).expect("read jb2");
    // Validate once up front so we benchmark a known-correct path.
    let decoded = Jbig2Decoder::new(Cursor::new(bytes.as_slice()))
        .expect("parse jb2")
        .decode_page(1)
        .expect("decode page")
        .bitmap;
    assert_eq!(
        &decoded, expected,
        "rust decode of {} did not match reference bitmap",
        case.jb2
    );
    record(ProbeRecord {
        side: "decode",
        tool: "rust",
        case: case.tag,
        raw_bytes: page_bytes(expected),
        compressed_bytes: bytes.len() as u64,
    });
    g.bench_with_input(BenchmarkId::new("rust", case.tag), &bytes, |b, data| {
        b.iter(|| {
            let mut dec = Jbig2Decoder::new(Cursor::new(data.as_slice())).unwrap();
            black_box(dec.decode_page(1).unwrap())
        });
    });
}

fn bench_t88_decode(
    g: &mut BenchmarkGroup<'_, WallTime>,
    case: &DecodeCase,
    jb2_path: &Path,
    expected: &Bitmap,
    bin: &Path,
) {
    let work = workdir_for(&format!("decode_t88_{}", case.tag));
    // Stage the input next to the output stem so the T.88 CLI's
    // stem/extension model works without touching the vendored corpus.
    let input_stem = work.join("input");
    let staged_jb2 = input_stem.with_extension("jb2");
    fs::copy(jb2_path, &staged_jb2).expect("stage jb2 for t88");
    let out_stem = work.join("out");

    // Correctness probe.
    run_quiet(
        &mut t88_decode_cmd(bin, &input_stem, &out_stem),
        "t88 decode probe",
    );
    let produced = out_stem.with_file_name(format!(
        "{}00.bmp",
        out_stem.file_name().unwrap().to_string_lossy()
    ));
    let decoded = load_bmp_1bpp(&produced);
    assert_eq!(
        &decoded, expected,
        "T.88 decode of {} did not match reference bitmap",
        case.jb2
    );

    record(ProbeRecord {
        side: "decode",
        tool: "t88",
        case: case.tag,
        raw_bytes: page_bytes(expected),
        compressed_bytes: fs::metadata(&staged_jb2).map(|m| m.len()).unwrap_or(0),
    });
    g.bench_function(BenchmarkId::new("t88", case.tag), |b| {
        b.iter_custom(|iters| measure_subprocess(iters, || t88_decode_cmd(bin, &input_stem, &out_stem)))
    });
}

fn bench_jbig2dec_decode(
    g: &mut BenchmarkGroup<'_, WallTime>,
    case: &DecodeCase,
    jb2_path: &Path,
    expected: &Bitmap,
    bin: &Path,
) {
    let work = workdir_for(&format!("decode_jbig2dec_{}", case.tag));
    let out = work.join("out.pbm");
    run_quiet(
        &mut jbig2dec_cmd(bin, jb2_path, &out),
        "jbig2dec decode probe",
    );
    let decoded = load_pbm_p4(&out);
    assert_eq!(
        &decoded, expected,
        "jbig2dec decode of {} did not match reference bitmap",
        case.jb2
    );

    record(ProbeRecord {
        side: "decode",
        tool: "jbig2dec",
        case: case.tag,
        raw_bytes: page_bytes(expected),
        compressed_bytes: fs::metadata(jb2_path).map(|m| m.len()).unwrap_or(0),
    });
    let input = jb2_path.to_path_buf();
    g.bench_function(BenchmarkId::new("jbig2dec", case.tag), |b| {
        b.iter_custom(|iters| measure_subprocess(iters, || jbig2dec_cmd(bin, &input, &out)))
    });
}

// ---------------------------------------------------------------------------
// Encode comparison
// ---------------------------------------------------------------------------

fn bench_encode_comparison(c: &mut Criterion) {
    let mut g = c.benchmark_group("encode/reference");
    g.sample_size(20);

    let t88 = t88_ref_bin();
    let jbig2enc = jbig2enc_bin();
    if t88.is_none() {
        eprintln!("encode/reference: T.88 binary not found");
    }
    if jbig2enc.is_none() {
        eprintln!(
            "encode/reference: jbig2enc not found; install via `brew install jbig2enc` or set JBIG2ENC_BIN"
        );
    }

    for case in ENCODE_CASES {
        let bmp_path = corpus_dir().join(case.bmp);
        if !bmp_path.is_file() {
            eprintln!(
                "encode/reference/{}: missing BMP {:?}, skipping",
                case.tag, bmp_path
            );
            continue;
        }
        let source = load_bmp_1bpp(&bmp_path);
        g.throughput(Throughput::Bytes(page_bytes(&source)));

        bench_rust_encode(&mut g, case, &source);
        if let Some(bin) = t88.as_deref() {
            bench_t88_encode(&mut g, case, &bmp_path, &source, bin);
        }
        if let Some(bin) = jbig2enc.as_deref() {
            bench_jbig2enc_encode(&mut g, case, &bmp_path, &source, bin);
        }
    }

    g.finish();
}

fn record_encode(tool: &'static str, case: &EncodeCase, source: &Bitmap, encoded_len: usize) {
    record(ProbeRecord {
        side: "encode",
        tool,
        case: case.tag,
        raw_bytes: page_bytes(source),
        compressed_bytes: encoded_len as u64,
    });
}

fn bench_rust_encode(g: &mut BenchmarkGroup<'_, WallTime>, case: &EncodeCase, source: &Bitmap) {
    let cfg = EncoderConfig::fast();
    let mut probe = Vec::new();
    {
        let mut enc = Jbig2Encoder::new(&mut probe, cfg.clone());
        enc.write_page(source).unwrap();
        enc.finish().unwrap();
    }
    // Round-trip validate so "fast" can't mean "broken".
    let decoded = Jbig2Decoder::new(Cursor::new(probe.as_slice()))
        .unwrap()
        .decode_page(1)
        .unwrap()
        .bitmap;
    assert_eq!(&decoded, source, "rust encode round-trip mismatch");
    record_encode("rust", case, source, probe.len());

    g.bench_with_input(BenchmarkId::new("rust", case.tag), source, |b, bm| {
        b.iter(|| {
            let mut out = Vec::with_capacity(probe.len() + 1024);
            let mut enc = Jbig2Encoder::new(&mut out, cfg.clone());
            enc.write_page(black_box(bm)).unwrap();
            enc.finish().unwrap();
            out
        });
    });
}

fn bench_t88_encode(
    g: &mut BenchmarkGroup<'_, WallTime>,
    case: &EncodeCase,
    bmp_path: &Path,
    source: &Bitmap,
    bin: &Path,
) {
    let work = workdir_for(&format!("encode_t88_{}", case.tag));
    let input_stem = work.join("input");
    fs::copy(bmp_path, input_stem.with_extension("bmp"))
        .expect("stage bmp for t88 encode");
    let out_stem = work.join("out");
    run_quiet(
        &mut t88_encode_cmd(bin, &input_stem, &out_stem),
        "t88 encode probe",
    );
    let produced = out_stem.with_extension("jb2");
    let encoded = fs::read(&produced).expect("read t88 jb2");
    // Validate via this crate's decoder so we know the output decodes
    // back to the source page before we start timing.
    let decoded = Jbig2Decoder::new(Cursor::new(encoded.as_slice()))
        .unwrap()
        .decode_page(1)
        .unwrap()
        .bitmap;
    assert_eq!(&decoded, source, "T.88 encode round-trip mismatch");
    record_encode("t88", case, source, encoded.len());

    let input_stem_owned: PathBuf = input_stem.clone();
    let out_stem_owned: PathBuf = out_stem.clone();
    g.bench_function(BenchmarkId::new("t88", case.tag), |b| {
        b.iter_custom(|iters| {
            measure_subprocess(iters, || {
                t88_encode_cmd(bin, &input_stem_owned, &out_stem_owned)
            })
        })
    });
}

fn bench_jbig2enc_encode(
    g: &mut BenchmarkGroup<'_, WallTime>,
    case: &EncodeCase,
    bmp_path: &Path,
    source: &Bitmap,
    bin: &Path,
) {
    let work = workdir_for(&format!("encode_jbig2enc_{}", case.tag));
    let out = work.join("out.jb2");
    run_quiet(
        &mut jbig2enc_cmd(bin, bmp_path, &out),
        "jbig2enc encode probe",
    );
    let encoded = fs::read(&out).expect("read jbig2enc jb2");
    let decoded = Jbig2Decoder::new(Cursor::new(encoded.as_slice()))
        .unwrap()
        .decode_page(1)
        .unwrap()
        .bitmap;
    assert_eq!(&decoded, source, "jbig2enc encode round-trip mismatch");
    record_encode("jbig2enc", case, source, encoded.len());

    let input = bmp_path.to_path_buf();
    let out_owned = out.clone();
    g.bench_function(BenchmarkId::new("jbig2enc", case.tag), |b| {
        b.iter_custom(|iters| {
            measure_subprocess(iters, || jbig2enc_cmd(bin, &input, &out_owned))
        })
    });
}

criterion_group!(benches, bench_decode_comparison, bench_encode_comparison);

// ---------------------------------------------------------------------------
// Post-run summary and HTML report pointer
// ---------------------------------------------------------------------------

/// Extract Criterion's mean point estimate (nanoseconds per iteration)
/// from `estimates.json`. Criterion's schema has been stable since 0.3,
/// so a targeted string walk keeps us off the `serde_json` dev-dep.
fn parse_mean_ns(path: &Path) -> Option<f64> {
    let data = fs::read_to_string(path).ok()?;
    // `"mean":{"confidence_interval":{...},"point_estimate":<number>,...}`
    // is the stable shape. Anchor on the mean object, then the first
    // `"point_estimate":` inside it.
    let i = data.find("\"mean\":{")?;
    let rest = &data[i + "\"mean\":".len()..];
    let pe = rest.find("\"point_estimate\":")?;
    let after = &rest[pe + "\"point_estimate\":".len()..];
    let end = after.find([',', '}']).unwrap_or(after.len());
    after[..end].trim().parse().ok()
}

/// Mirror of criterion's `DEFAULT_OUTPUT_DIRECTORY` resolution so the
/// summary reads the same `target/criterion` that Criterion just wrote.
/// In hosted sandboxes `$CARGO_TARGET_DIR` often lives outside the
/// workspace, so hard-coding `./target/criterion` would silently miss
/// every estimates.json.
fn criterion_output_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("CRITERION_HOME") {
        return PathBuf::from(home);
    }
    if let Some(target) = std::env::var_os("CARGO_TARGET_DIR") {
        return PathBuf::from(target).join("criterion");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("criterion")
}

fn estimates_path(side: &str, tool: &str, case: &str) -> PathBuf {
    // Criterion sanitises `/` in group names to `_`; the groups here
    // are `decode/reference` and `encode/reference`.
    criterion_output_dir()
        .join(format!("{side}_reference"))
        .join(tool)
        .join(case)
        .join("new")
        .join("estimates.json")
}

fn fmt_ns(ns: f64) -> String {
    if ns >= 1e9 {
        format!("{:.3} s", ns / 1e9)
    } else if ns >= 1e6 {
        format!("{:.3} ms", ns / 1e6)
    } else if ns >= 1e3 {
        format!("{:.3} µs", ns / 1e3)
    } else {
        format!("{:.0} ns", ns)
    }
}

/// Preferred display order — rust first (the codec under test), then
/// the production-oriented Homebrew tools, then the normative ITU
/// reference last.
const TOOL_ORDER: &[&str] = &["rust", "jbig2enc", "jbig2dec", "t88"];

fn tool_rank(tool: &str) -> usize {
    TOOL_ORDER
        .iter()
        .position(|&o| o == tool)
        .unwrap_or(TOOL_ORDER.len())
}

fn print_summary() {
    let records = probes().lock().unwrap().clone();
    if records.is_empty() {
        return;
    }

    // Group by (side, case). BTreeMap keeps side alphabetical which is
    // fine (decode before encode); within each side we iterate the
    // declared case order so tt9 prints before tt10 regardless of the
    // lexical "tt10_arith" < "tt9_mmr" trap.
    let mut by_case: BTreeMap<(&'static str, &'static str), Vec<ProbeRecord>> = BTreeMap::new();
    for r in &records {
        by_case.entry((r.side, r.case)).or_default().push(r.clone());
    }

    let mut stderr = std::io::stderr().lock();
    use std::io::Write;
    let _ = writeln!(stderr);
    let _ = writeln!(
        stderr,
        "================ Reference codec comparison ================"
    );

    let ordered_cases = |side: &str| -> Vec<&'static str> {
        match side {
            "decode" => DECODE_CASES.iter().map(|c| c.tag).collect(),
            "encode" => ENCODE_CASES.iter().map(|c| c.tag).collect(),
            _ => Vec::new(),
        }
    };

    for side in ["decode", "encode"] {
        for case in ordered_cases(side) {
            let Some(tools) = by_case.get(&(side, case)) else {
                continue;
            };
            let mut tools: Vec<&ProbeRecord> = tools.iter().collect();
            tools.sort_by_key(|r| tool_rank(r.tool));

            let _ = writeln!(stderr);
            let _ = writeln!(stderr, "{side}/{case}");
            let _ = writeln!(
                stderr,
                "  {:<10} {:>12} {:>14} {:>12} {:>9}",
                "tool", "mean", "throughput", "bytes", "ratio"
            );
            let _ = writeln!(stderr, "  {}", "-".repeat(61));

            for r in tools {
                let est = estimates_path(r.side, r.tool, r.case);
                let (time_str, tput_str) = match parse_mean_ns(&est) {
                    Some(ns) => {
                        let seconds = ns / 1e9;
                        let mib_s = (r.raw_bytes as f64) / seconds / (1024.0 * 1024.0);
                        (fmt_ns(ns), format!("{mib_s:.1} MiB/s"))
                    }
                    None => ("n/a".to_string(), "n/a".to_string()),
                };
                let ratio = if r.compressed_bytes == 0 {
                    "  n/a".to_string()
                } else {
                    format!("{:.2}x", r.raw_bytes as f64 / r.compressed_bytes as f64)
                };
                let _ = writeln!(
                    stderr,
                    "  {:<10} {:>12} {:>14} {:>12} {:>9}",
                    r.tool, time_str, tput_str, r.compressed_bytes, ratio
                );
            }
        }
    }

    let _ = writeln!(stderr);
    let _ = writeln!(
        stderr,
        "Columns: mean = Criterion point estimate, throughput = raw page bytes / mean,"
    );
    let _ = writeln!(
        stderr,
        "         bytes = compressed stream size (input for decode, output for encode),"
    );
    let _ = writeln!(stderr, "         ratio = raw / compressed.");
    let _ = writeln!(
        stderr,
        "Subprocess rows (t88, jbig2enc, jbig2dec) include fork/exec overhead."
    );
    let _ = writeln!(
        stderr,
        "============================================================"
    );
    print_html_link(&mut stderr);
    match render_comparison_chart(&records) {
        Ok(Some(path)) => {
            let _ = writeln!(stderr);
            let _ = writeln!(
                stderr,
                "Cross-tool grouped bar chart (MiB/s, log scale, colored by tool):"
            );
            let _ = writeln!(stderr, "  file://{}", path.display());
        }
        Ok(None) => {
            // No estimates on disk yet; the table will already have said n/a.
        }
        Err(e) => {
            let _ = writeln!(stderr, "[warn] chart render failed: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// Custom comparison chart
// ---------------------------------------------------------------------------

/// Stable per-tool color palette used by the grouped bar chart. Kept
/// distinct under both normal and colorblind-ish viewing: rust = blue,
/// Homebrew jbig2{enc,dec} = orange, ITU T.88 reference = purple.
fn tool_color(tool: &str) -> RGBColor {
    match tool {
        "rust" => RGBColor(0x1F, 0x77, 0xB4),
        "jbig2enc" | "jbig2dec" => RGBColor(0xFF, 0x7F, 0x0E),
        "t88" => RGBColor(0x8E, 0x44, 0xAD),
        _ => RGBColor(0x55, 0x55, 0x55),
    }
}

/// Render a two-row grouped bar chart (decode on top, encode below)
/// comparing `MiB/s` throughput across the Rust crate and the external
/// reference tools for every case that produced an `estimates.json`.
///
/// Returns `Ok(None)` when no Criterion estimates are on disk yet —
/// typically because the run was `--quick`, which skips persistence.
/// The caller should degrade gracefully rather than treat that as an
/// error.
fn render_comparison_chart(
    records: &[ProbeRecord],
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    // Collate (side, case, tool) -> MiB/s from Criterion estimates. We
    // intentionally drop entries whose estimates.json is missing so the
    // chart surface matches what Criterion actually measured this run.
    let mut data: BTreeMap<(&'static str, &'static str, &'static str), f64> = BTreeMap::new();
    for r in records {
        let est = estimates_path(r.side, r.tool, r.case);
        if let Some(ns) = parse_mean_ns(&est) {
            if ns > 0.0 && r.raw_bytes > 0 {
                let mib = (r.raw_bytes as f64) / (ns / 1e9) / (1024.0 * 1024.0);
                data.insert((r.side, r.case, r.tool), mib);
            }
        }
    }
    if data.is_empty() {
        return Ok(None);
    }

    let out_path = criterion_output_dir().join("reference_codec_chart.svg");
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    {
        let root = SVGBackend::new(&out_path, (1200, 900)).into_drawing_area();
        root.fill(&WHITE)?;
        let root = root.titled(
            "Reference codec throughput comparison (higher is better)",
            ("sans-serif", 26),
        )?;

        let (top, bot) = root.split_vertically(430);

        let decode_cases: Vec<&'static str> = DECODE_CASES.iter().map(|c| c.tag).collect();
        let encode_cases: Vec<&'static str> = ENCODE_CASES.iter().map(|c| c.tag).collect();
        // Order within each side matches the summary table so the
        // chart and the stderr block read the same way.
        let decode_tools: &[&str] = &["rust", "jbig2dec", "t88"];
        let encode_tools: &[&str] = &["rust", "jbig2enc", "t88"];

        draw_side_chart(&top, "Decode", "decode", &decode_cases, decode_tools, &data)?;
        draw_side_chart(&bot, "Encode", "encode", &encode_cases, encode_tools, &data)?;

        root.present()?;
    }

    Ok(Some(out_path))
}

/// Draw one side (decode or encode) as a vertical grouped bar chart on
/// `area`. Bars are log-scaled on Y so a ~40x dynamic range between
/// Rust and T.88 stays readable; numeric labels above each bar keep the
/// absolute values visible since log axes are easy to misread.
fn draw_side_chart<DB>(
    area: &DrawingArea<DB, Shift>,
    title: &str,
    side: &'static str,
    cases: &[&'static str],
    tools: &[&str],
    data: &BTreeMap<(&'static str, &'static str, &'static str), f64>,
) -> Result<(), Box<dyn std::error::Error>>
where
    DB: DrawingBackend,
    DB::ErrorType: 'static,
{
    // Filter tools to those that actually produced a bar so the legend
    // doesn't lie when, say, jbig2enc was skipped.
    let present_tools: Vec<&str> = tools
        .iter()
        .copied()
        .filter(|t| cases.iter().any(|c| data.contains_key(&(side, *c, *t))))
        .collect();
    if present_tools.is_empty() {
        return Ok(());
    }

    let max_mib = cases
        .iter()
        .flat_map(|c| {
            present_tools
                .iter()
                .filter_map(|t| data.get(&(side, *c, *t)).copied())
        })
        .fold(0.0_f64, f64::max);
    // Log scale needs a strictly positive floor; pick a little below
    // the smallest value to keep every bar visible.
    let min_mib = cases
        .iter()
        .flat_map(|c| {
            present_tools
                .iter()
                .filter_map(|t| data.get(&(side, *c, *t)).copied())
        })
        .fold(f64::INFINITY, f64::min)
        .max(0.1);
    let y_min = (min_mib / 2.0).max(0.1);
    let y_max = (max_mib * 2.5).max(1.0);

    let mut chart = ChartBuilder::on(area)
        .caption(
            format!("{title} throughput (MiB/s, log scale)"),
            ("sans-serif", 22),
        )
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(70)
        .right_y_label_area_size(10)
        .build_cartesian_2d(-0.5..(cases.len() as f64) - 0.5, (y_min..y_max).log_scale())?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .x_labels(cases.len())
        .x_label_formatter(&|x| {
            let i = x.round() as i64;
            if i >= 0 && (i as usize) < cases.len() {
                cases[i as usize].to_string()
            } else {
                String::new()
            }
        })
        .x_desc("case")
        .y_desc("MiB/s")
        .label_style(("sans-serif", 14))
        .axis_desc_style(("sans-serif", 14))
        .draw()?;

    let group_width: f64 = 0.82;
    let bar_width: f64 = group_width / present_tools.len() as f64;

    for (t_idx, tool) in present_tools.iter().enumerate() {
        let color = tool_color(tool);
        let offset = (t_idx as f64) * bar_width - group_width / 2.0 + bar_width / 2.0;
        let bars: Vec<(f64, f64, f64)> = cases
            .iter()
            .enumerate()
            .filter_map(|(c_idx, case)| {
                let mib = *data.get(&(side, *case, *tool))?;
                let x_center = (c_idx as f64) + offset;
                Some((
                    x_center - bar_width / 2.0,
                    x_center + bar_width / 2.0,
                    mib,
                ))
            })
            .collect();

        let legend_color = color;
        chart
            .draw_series(
                bars.iter()
                    .map(|&(x0, x1, mib)| Rectangle::new([(x0, y_min), (x1, mib)], color.filled())),
            )?
            .label(*tool)
            .legend(move |(x, y)| {
                Rectangle::new([(x, y - 6), (x + 14, y + 6)], legend_color.filled())
            });

        // Value labels above each bar so log scale distortion doesn't
        // hide the actual number.
        for &(x0, x1, mib) in &bars {
            let x_mid = (x0 + x1) / 2.0;
            let label = if mib >= 100.0 {
                format!("{mib:.0}")
            } else if mib >= 10.0 {
                format!("{mib:.1}")
            } else {
                format!("{mib:.2}")
            };
            chart.draw_series(std::iter::once(Text::new(
                label,
                (x_mid, mib * 1.08),
                ("sans-serif", 12).into_font().color(&BLACK),
            )))?;
        }
    }

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.9))
        .border_style(BLACK.mix(0.3))
        .label_font(("sans-serif", 14))
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;

    Ok(())
}

fn print_html_link<W: std::io::Write>(stderr: &mut W) {
    let base = criterion_output_dir();
    let root = base.join("report").join("index.html");
    let _ = writeln!(stderr);
    if root.is_file() {
        let _ = writeln!(stderr, "Full Criterion HTML report:");
        let _ = writeln!(stderr, "  file://{}", root.display());
        for side in ["decode_reference", "encode_reference"] {
            let p = base.join(side).join("report").join("index.html");
            if p.is_file() {
                let _ = writeln!(stderr, "  file://{}", p.display());
            }
        }
    } else {
        let _ = writeln!(
            stderr,
            "HTML report not generated at {}.",
            root.display()
        );
        let _ = writeln!(
            stderr,
            "If you used --quick, re-run without it; --quick skips Criterion's disk output."
        );
    }
}

// Hand-rolled `main` so the summary table and HTML link print after
// everything Criterion writes out, including the per-report index.html
// that `final_summary()` materialises. Order matters: if the summary
// prints before `final_summary()`, the HTML link check races the
// report directory and always reports "not generated".
fn main() {
    benches();
    Criterion::default().configure_from_args().final_summary();
    print_summary();
}
