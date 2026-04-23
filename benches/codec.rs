//! Criterion benchmarks for the hot encode/decode paths.
//!
//! Coverage is split into four tiers:
//!
//! * **Template-0 arithmetic decode** (`decode/generic/arith_t0`) — the
//!   primary performance signal for the nominal-template decoder fast
//!   path. All fixtures in this group pin `GenericTemplate::T0`
//!   explicitly, so reported numbers attribute cleanly to the template-0
//!   hot loop and cannot be confused with preset-level regressions.
//! * **Preset regression decode** (`decode/generic/arith_preset`) — the
//!   `EncoderConfig::fast()` path (currently template 3, no duplicate
//!   line removal). This exists only to catch regressions outside the
//!   template-0 fast path; it is *not* the headline number for the
//!   decoder performance work.
//! * **Arithmetic fallback decode** (`decode/generic/arith_fallbacks`) —
//!   templates 1/2/3 at nominal AT, extended template 0 (AMD2), and
//!   template 0 with a non-nominal AT override. These are the paths
//!   that still go through `decode_row_generic()` rather than the
//!   template-0 sliding-register fast path, so this group is the early
//!   warning for any regression on the generic row decoder and the
//!   signal driving whether more specialized fast paths are worth the
//!   maintenance cost.
//! * **MMR decode, symbol/text decode, and encode regressions** — full
//!   pages routed through the respective pipelines so refactors that
//!   touch the shared compositing / context-extraction code are caught
//!   in either direction.
//! * **Micro-benchmarks** — small stand-alone drills (arithmetic context
//!   building and a single MMR line decode) that let us verify
//!   inner-loop rewrites have the intended local speedup before looking
//!   at the end-to-end numbers.
//!
//! Real-world conformance inputs (TT10 for arithmetic generic) are picked
//! up automatically when the vendored T.88 corpus is present, so the same
//! harness measures both synthetic and real documents.

use std::io::Cursor;
use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use jbig2::coding::mq::{MqContexts, MqDecoder, MqEncoder, MQ_NUM_CONTEXTS};
use jbig2::segments::generic_region::{
    decode_generic_arith, encode_generic_arith, nominal_at, GenericRegionHeader,
};
use jbig2::segments::page_information::CombinationOp;
use jbig2::segments::region_info::RegionInfo;
use jbig2::{Bitmap, Coding, EncoderConfig, GenericTemplate, Jbig2Decoder, Jbig2Encoder, Mode};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Text-shaped bitmap: mostly-white rows broken up by synthetic "glyphs".
///
/// This is our canonical benchmark shape for `decode/generic/*`: it has the
/// low-density duplicate-line structure that typical arithmetic-coded
/// document pages have, which is what TPGD is designed for and what real
/// patent-TIFF workloads look like in practice.
fn text_shaped_bitmap(w: u32, h: u32) -> Bitmap {
    let mut bm = Bitmap::new(w, h).unwrap();
    // Put a horizontal rule every 40 lines and a dense "text block" band of 3
    // rows at every multiple of 12 lines, to get a mix of duplicate rows,
    // run-like content, and diagonal-style structure all on the same page.
    for y in 0..h as i32 {
        if y % 40 == 3 {
            // Horizontal ruler line, whole row.
            for x in 0..w as i32 {
                bm.set_pixel(x, y, 1);
            }
        } else if y % 12 < 3 {
            // Text band: 3 rows of "text" with glyph-style runs.
            let stripe = (y / 12) as u32;
            let shift = (stripe * 7) % 31;
            for x in 0..w as i32 {
                let u = (x as u32).wrapping_add(shift);
                // Pairs of 2-3 pixel runs with 4-pixel gaps, mimicking text.
                let g = u % 12;
                if g < 2 || g == 5 || g == 6 || g == 9 {
                    bm.set_pixel(x, y, 1);
                }
            }
        }
    }
    bm
}

/// Dense "diagonals" bitmap — stresses the arithmetic coder with ~9% density
/// and no long duplicate-row runs.
fn stripe_bitmap(w: u32, h: u32) -> Bitmap {
    let mut bm = Bitmap::new(w, h).unwrap();
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            if (x + y) % 11 == 0 {
                bm.set_pixel(x, y, 1);
            }
        }
    }
    bm
}

/// Symbol/text-heavy layout: 3 repeating "glyph" shapes tiled into a grid.
///
/// This is what the symbol-dictionary + text-region path encodes, and what
/// the text-region decoder has to place instance-by-instance during decode.
fn glyph_repeat_bitmap(w: u32, h: u32) -> Bitmap {
    let mut bm = Bitmap::new(w, h).unwrap();
    let shapes: [&[(i32, i32)]; 3] = [
        &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1), (0, 2), (1, 2), (2, 2)],
        &[(1, 0), (0, 1), (1, 1), (2, 1), (1, 2), (1, 3)],
        &[(0, 0), (0, 1), (0, 2), (1, 2), (2, 0), (2, 1), (2, 2)],
    ];
    let gx = 10u32;
    let gy = 10u32;
    for row in 0..(h / gy) as usize {
        for col in 0..(w / gx) as usize {
            let s = shapes[col % 3];
            for &(dx, dy) in s {
                bm.set_pixel(col as i32 * gx as i32 + dx, row as i32 * gy as i32 + dy, 1);
            }
        }
    }
    bm
}

/// Encode `bitmap` into a full JBIG2 file using `cfg`. Returns the bytes the
/// decoder benchmarks operate on.
fn encode_page(bitmap: &Bitmap, cfg: EncoderConfig) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut enc = Jbig2Encoder::new(&mut buf, cfg);
    enc.write_page(bitmap).unwrap();
    enc.finish().unwrap();
    buf
}

/// Optionally load a vendored conformance file. Returns `None` when the
/// vendored corpus isn't available so the harness still works on fresh
/// clones.
fn conformance_bytes(name: &str) -> Option<Vec<u8>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("vendor/T-REC-T.88-201808/Software/JBIG2_ConformanceData-A20180829")
        .join(name);
    std::fs::read(&path).ok()
}

// ---------------------------------------------------------------------------
// Full-page decode benchmarks (the primary performance signal)
// ---------------------------------------------------------------------------

/// Build an explicit template-0 arithmetic generic `EncoderConfig`. The
/// primary decode benchmark numbers pin template 0 directly so results
/// always reflect the optimized nominal-template hot loop rather than
/// whatever template a preset happens to select today.
fn t0_generic_arith(duplicate_line_removal: bool) -> EncoderConfig {
    EncoderConfig {
        mode: Mode::Generic,
        template: GenericTemplate::T0,
        coding: Coding::Arithmetic,
        adaptive_templates: None,
        refinement: false,
        generic_region_duplicate_line_removal: duplicate_line_removal,
        symbol_threshold: 0.97,
        refine_after_match: false,
    }
}

/// Primary arithmetic decode benchmark: pins template 0 so all fixtures
/// go through the nominal-template fast path. Two shapes are measured:
/// low-density text-like pages (which also benefit from duplicate-line
/// removal) and dense diagonals (no TPGD short-circuits, so the MQ loop
/// and row decoder are fully exercised).
fn bench_decode_generic_arith_t0(c: &mut Criterion) {
    let mut g = c.benchmark_group("decode/generic/arith_t0");
    for (label, w, h) in [("512x512", 512u32, 512u32), ("1024x1024", 1024, 1024)] {
        let bm = text_shaped_bitmap(w, h);
        let bytes = encode_page(&bm, t0_generic_arith(true));
        g.throughput(Throughput::Bytes((w as u64) * (h as u64) / 8));
        g.bench_with_input(BenchmarkId::new("text_shaped", label), &bytes, |b, data| {
            b.iter(|| {
                let mut dec = Jbig2Decoder::new(Cursor::new(data.as_slice())).unwrap();
                black_box(dec.decode_page(1).unwrap())
            });
        });
    }
    // Dense diagonals — duplicate-line removal disabled so every row runs
    // through the template-0 decode loop with no TPGD short-circuit.
    let bm = stripe_bitmap(512, 512);
    let bytes = encode_page(&bm, t0_generic_arith(false));
    g.throughput(Throughput::Bytes(512 * 512 / 8));
    g.bench_with_input(BenchmarkId::new("diagonals", "512x512"), &bytes, |b, data| {
        b.iter(|| {
            let mut dec = Jbig2Decoder::new(Cursor::new(data.as_slice())).unwrap();
            black_box(dec.decode_page(1).unwrap())
        });
    });
    g.finish();
}

/// Decode the T.88 conformance arithmetic generic page (TT10). TT10 is
/// template 0 + TPGD, matching the primary hot path, and it is the
/// external correctness + performance gate for the arithmetic generic
/// work. Skipped if the vendored corpus isn't present.
fn bench_decode_generic_arith_tt10(c: &mut Criterion) {
    let Some(bytes) = conformance_bytes("F01_200_TT10.jb2") else {
        return;
    };
    let mut g = c.benchmark_group("decode/generic/arith_t0");
    g.throughput(Throughput::Bytes(bytes.len() as u64));
    g.bench_function("tt10_conformance", |b| {
        b.iter(|| {
            let mut dec = Jbig2Decoder::new(Cursor::new(bytes.as_slice())).unwrap();
            black_box(dec.decode_page(1).unwrap())
        });
    });
    g.finish();
}

/// Preset regression coverage: measures `EncoderConfig::fast()` (template
/// 3, no duplicate-line removal) end-to-end so a regression in that
/// preset does not go unnoticed. Kept in a separate Criterion group from
/// the template-0 benchmarks above so headline numbers don't mix
/// template-0 hot-loop gains with template-3 preset behaviour.
fn bench_decode_generic_arith_preset(c: &mut Criterion) {
    let mut g = c.benchmark_group("decode/generic/arith_preset");
    let bm = stripe_bitmap(512, 512);
    let bytes = encode_page(&bm, EncoderConfig::fast());
    g.throughput(Throughput::Bytes(512 * 512 / 8));
    g.bench_with_input(
        BenchmarkId::new("fast_preset_diagonals", "512x512"),
        &bytes,
        |b, data| {
            b.iter(|| {
                let mut dec = Jbig2Decoder::new(Cursor::new(data.as_slice())).unwrap();
                black_box(dec.decode_page(1).unwrap())
            });
        },
    );
    g.finish();
}

#[cfg(feature = "mmr")]
fn bench_decode_generic_mmr(c: &mut Criterion) {
    let mut g = c.benchmark_group("decode/generic/mmr");
    for (label, w, h) in [("512x512", 512u32, 512u32), ("1024x1024", 1024, 1024)] {
        let bm = text_shaped_bitmap(w, h);
        let bytes = encode_page(
            &bm,
            EncoderConfig {
                mode: Mode::Generic,
                template: GenericTemplate::T0,
                coding: Coding::Mmr,
                adaptive_templates: None,
                refinement: false,
                generic_region_duplicate_line_removal: false,
                symbol_threshold: 0.97,
                refine_after_match: false,
            },
        );
        g.throughput(Throughput::Bytes((w as u64) * (h as u64) / 8));
        g.bench_with_input(BenchmarkId::new("text_shaped", label), &bytes, |b, data| {
            b.iter(|| {
                let mut dec = Jbig2Decoder::new(Cursor::new(data.as_slice())).unwrap();
                black_box(dec.decode_page(1).unwrap())
            });
        });
    }
    g.finish();
}

fn bench_decode_symbol_heavy(c: &mut Criterion) {
    let mut g = c.benchmark_group("decode/symbol");
    for (label, w, h) in [("1024x512", 1024u32, 512u32), ("2048x1024", 2048, 1024)] {
        let bm = glyph_repeat_bitmap(w, h);
        let bytes = encode_page(&bm, EncoderConfig::balanced());
        g.throughput(Throughput::Bytes((w as u64) * (h as u64) / 8));
        g.bench_with_input(BenchmarkId::new("lossless", label), &bytes, |b, data| {
            b.iter(|| {
                let mut dec = Jbig2Decoder::new(Cursor::new(data.as_slice())).unwrap();
                black_box(dec.decode_page(1).unwrap())
            });
        });
    }
    g.finish();
}

// ---------------------------------------------------------------------------
// Full-page encode benchmarks (regression guards)
// ---------------------------------------------------------------------------

fn bench_encode_generic_arith(c: &mut Criterion) {
    let mut g = c.benchmark_group("encode/generic/arith");
    let bm = text_shaped_bitmap(512, 512);
    g.throughput(Throughput::Bytes(512 * 512 / 8));
    g.bench_function("text_shaped/512x512", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(8 * 1024);
            let mut enc = Jbig2Encoder::new(&mut out, EncoderConfig::fast());
            enc.write_page(black_box(&bm)).unwrap();
            enc.finish().unwrap();
            out
        });
    });
    g.finish();
}

fn bench_encode_symbol_lossless(c: &mut Criterion) {
    let mut g = c.benchmark_group("encode/symbol");
    let bm = glyph_repeat_bitmap(1024, 512);
    g.throughput(Throughput::Bytes(1024 * 512 / 8));
    g.bench_function("lossless/1024x512", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(16 * 1024);
            let mut enc = Jbig2Encoder::new(&mut out, EncoderConfig::balanced());
            enc.write_page(black_box(&bm)).unwrap();
            enc.finish().unwrap();
            out
        });
    });
    g.finish();
}

// ---------------------------------------------------------------------------
// Micro-benchmarks: isolate arithmetic context building and MMR line decode
// ---------------------------------------------------------------------------

fn arith_encode_slice(bitmap: &Bitmap, template: u8, ext: bool, tpgdon: bool) -> Vec<u8> {
    arith_encode_slice_with_at(bitmap, template, ext, tpgdon, nominal_at(template, ext))
}

fn arith_encode_slice_with_at(
    bitmap: &Bitmap,
    template: u8,
    ext: bool,
    tpgdon: bool,
    at: jbig2::segments::AtPixels,
) -> Vec<u8> {
    let hdr = GenericRegionHeader {
        region: RegionInfo {
            width: bitmap.width(),
            height: bitmap.height(),
            x: 0,
            y: 0,
            external_combination_op: CombinationOp::Or,
            colour_extension: false,
        },
        mmr: false,
        template,
        tpgdon,
        ext_template: ext,
        at,
    };
    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    let mut enc = MqEncoder::new(bitmap.data().len());
    encode_generic_arith(&mut enc, &mut cxs, &hdr, bitmap).unwrap();
    enc.finish()
}

fn bench_arith_generic_decode_raw(c: &mut Criterion) {
    let mut g = c.benchmark_group("decode/generic/arith_raw");
    for (label, w, h, tpgdon) in [
        ("text_shaped_notpgd/512x512", 512u32, 512u32, false),
        ("text_shaped_tpgd/512x512", 512, 512, true),
        ("diagonals/512x512", 512, 512, false),
    ] {
        let bm = if label.starts_with("diag") {
            stripe_bitmap(w, h)
        } else {
            text_shaped_bitmap(w, h)
        };
        let bytes = arith_encode_slice(&bm, 0, false, tpgdon);
        let hdr = GenericRegionHeader {
            region: RegionInfo {
                width: w,
                height: h,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            mmr: false,
            template: 0,
            tpgdon,
            ext_template: false,
            at: nominal_at(0, false),
        };
        g.throughput(Throughput::Bytes((w as u64) * (h as u64) / 8));
        g.bench_with_input(BenchmarkId::from_parameter(label), &bytes, |b, data| {
            b.iter(|| {
                let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
                let mut dec = MqDecoder::new(data.as_slice());
                black_box(decode_generic_arith(&mut dec, &mut cxs, &hdr).unwrap())
            });
        });
    }
    g.finish();
}

/// Coverage for the arithmetic fallbacks that are still handled by
/// `decode_row_generic()` rather than the template-0 fast path:
///
/// * templates 1, 2, and 3 with their nominal AT positions,
/// * template 0 with the extended 12-AT (AMD2) template,
/// * template 0 with a non-nominal AT override so AT placement routes
///   through the generic row decoder even on the primary template.
///
/// The numbers here are deliberately separate from the template-0
/// nominal-AT group so regressions in these paths stand out instead of
/// being averaged into the headline fast-path throughput.
fn bench_arith_generic_decode_fallbacks(c: &mut Criterion) {
    let mut g = c.benchmark_group("decode/generic/arith_fallbacks");
    // Shared text-shaped fixture — low-density, mostly representative of
    // document pages, and exercises both duplicate-like rows (TPGDON=false
    // here on purpose) and short runs.
    let (w, h) = (512u32, 512u32);
    let bm = text_shaped_bitmap(w, h);
    g.throughput(Throughput::Bytes((w as u64) * (h as u64) / 8));

    // Templates 1/2/3 at nominal AT.
    for template in [1u8, 2, 3] {
        let at = nominal_at(template, false);
        let bytes = arith_encode_slice_with_at(&bm, template, false, false, at);
        let hdr = GenericRegionHeader {
            region: RegionInfo {
                width: w,
                height: h,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            mmr: false,
            template,
            tpgdon: false,
            ext_template: false,
            at,
        };
        let label = format!("template{template}_nominal/512x512");
        g.bench_with_input(BenchmarkId::from_parameter(label), &bytes, |b, data| {
            b.iter(|| {
                let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
                let mut dec = MqDecoder::new(data.as_slice());
                black_box(decode_generic_arith(&mut dec, &mut cxs, &hdr).unwrap())
            });
        });
    }

    // Extended template 0 (AMD2), nominal 12-AT.
    {
        let at = nominal_at(0, true);
        let bytes = arith_encode_slice_with_at(&bm, 0, true, false, at);
        let hdr = GenericRegionHeader {
            region: RegionInfo {
                width: w,
                height: h,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            mmr: false,
            template: 0,
            tpgdon: false,
            ext_template: true,
            at,
        };
        g.bench_with_input(
            BenchmarkId::from_parameter("template0_ext_nominal/512x512"),
            &bytes,
            |b, data| {
                b.iter(|| {
                    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
                    let mut dec = MqDecoder::new(data.as_slice());
                    black_box(decode_generic_arith(&mut dec, &mut cxs, &hdr).unwrap())
                });
            },
        );
    }

    // Template 0 with a non-nominal AT override. Nominal AT[0] is (3,-1);
    // swap it for (2,-2) so the nominal-AT fast path is disabled and the
    // generic row decoder takes over. AT[1..4] stay at the nominal values
    // for template 0 so we are not also changing context statistics more
    // than necessary.
    {
        let mut at_arr = nominal_at(0, false).as_array();
        at_arr[0] = (2, -2);
        at_arr[2] = (3, -1);
        let at = jbig2::segments::AtPixels::new(at_arr, 4);
        let bytes = arith_encode_slice_with_at(&bm, 0, false, false, at);
        let hdr = GenericRegionHeader {
            region: RegionInfo {
                width: w,
                height: h,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            mmr: false,
            template: 0,
            tpgdon: false,
            ext_template: false,
            at,
        };
        g.bench_with_input(
            BenchmarkId::from_parameter("template0_nonnominal_at/512x512"),
            &bytes,
            |b, data| {
                b.iter(|| {
                    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
                    let mut dec = MqDecoder::new(data.as_slice());
                    black_box(decode_generic_arith(&mut dec, &mut cxs, &hdr).unwrap())
                });
            },
        );
    }

    g.finish();
}

#[cfg(feature = "mmr")]
fn bench_mmr_line_decode(c: &mut Criterion) {
    use jbig2::coding::mmr::{decode_t6_line, encode_t6_line, BitReader, BitWriter};
    use jbig2::coding::mmr_lut::{decode_t6_line_packed, MmrBitBuf};
    // Emulate a realistic scanned-document line: alternating wide white
    // runs (hundreds of pixels) and shorter black runs.
    let width = 1728u32;
    let mut line = vec![false; width as usize];
    let mut x = 30usize;
    while x + 8 < width as usize {
        for k in 0..8 {
            line[x + k] = true;
        }
        x += 48;
    }
    let prev = vec![false; width as usize];
    let mut w = BitWriter::new();
    encode_t6_line(&mut w, &line, &prev);
    let bytes = w.finish();
    let stride = ((width as usize) + 7) / 8;
    let prev_row = vec![0u8; stride];

    let mut g = c.benchmark_group("decode/mmr/line");
    g.throughput(Throughput::Bytes((width as u64) / 8));
    g.bench_function("slow/1728px_mixed_runs", |b| {
        b.iter(|| {
            let mut r = BitReader::new(&bytes);
            black_box(decode_t6_line(&mut r, &prev, width).unwrap())
        });
    });
    g.bench_function("fast/1728px_mixed_runs", |b| {
        let mut cur = vec![0u8; stride];
        b.iter(|| {
            let mut buf = MmrBitBuf::new(&bytes);
            decode_t6_line_packed(&mut buf, &prev_row, &mut cur, width).unwrap();
            black_box(&cur);
        });
    });
    g.finish();
}

// ---------------------------------------------------------------------------

#[cfg(feature = "mmr")]
criterion_group!(
    benches,
    bench_decode_generic_arith_t0,
    bench_decode_generic_arith_tt10,
    bench_decode_generic_arith_preset,
    bench_decode_generic_mmr,
    bench_decode_symbol_heavy,
    bench_encode_generic_arith,
    bench_encode_symbol_lossless,
    bench_arith_generic_decode_raw,
    bench_arith_generic_decode_fallbacks,
    bench_mmr_line_decode,
);

#[cfg(not(feature = "mmr"))]
criterion_group!(
    benches,
    bench_decode_generic_arith_t0,
    bench_decode_generic_arith_tt10,
    bench_decode_generic_arith_preset,
    bench_decode_symbol_heavy,
    bench_encode_generic_arith,
    bench_encode_symbol_lossless,
    bench_arith_generic_decode_raw,
    bench_arith_generic_decode_fallbacks,
);

criterion_main!(benches);
