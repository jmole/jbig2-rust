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

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use criterion::{
    black_box, criterion_group, measurement::WallTime, BenchmarkGroup, BenchmarkId, Criterion,
    Throughput,
};
use jbig2::{Bitmap, EncoderConfig, Jbig2Decoder, Jbig2Encoder};

mod common;
#[path = "reference_codec/runners.rs"]
mod runners;
#[path = "reference_codec/summary.rs"]
mod summary;
#[path = "reference_codec/charting.rs"]
mod charting;

use common::*;
use runners::{jbig2dec_cmd, jbig2enc_cmd, measure_subprocess, t88_decode_cmd, t88_encode_cmd};
use summary::{ProbeRecord, print_summary, record};

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
