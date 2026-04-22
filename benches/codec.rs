//! Criterion benchmarks for the hot encode/decode paths.
//!
//! These benchmarks are intentionally small so they run in <30s on a
//! laptop. For larger-scale comparisons against `jbig2enc`, use the CLI
//! binary with a patent-TIFF corpus.

use std::io::Cursor;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jbig2::{Bitmap, EncoderConfig, Jbig2Decoder, Jbig2Encoder};

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

fn glyph_repeat_bitmap(w: u32, h: u32) -> Bitmap {
    let mut bm = Bitmap::new(w, h).unwrap();
    let shapes: [&[(i32, i32)]; 3] = [
        &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1), (0, 2), (1, 2), (2, 2)],
        &[(1, 0), (0, 1), (1, 1), (2, 1), (1, 2), (1, 3)],
        &[(0, 0), (0, 1), (0, 2), (1, 2), (2, 0), (2, 1), (2, 2)],
    ];
    for row in 0..(h / 10) as usize {
        for col in 0..(w / 10) as usize {
            let s = shapes[col % 3];
            for &(dx, dy) in s {
                bm.set_pixel(col as i32 * 10 + dx, row as i32 * 10 + dy, 1);
            }
        }
    }
    bm
}

fn bench_encode_generic(c: &mut Criterion) {
    let bm = stripe_bitmap(512, 256);
    c.bench_function("encode/generic/512x256/diagonals", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(4096);
            let mut enc = Jbig2Encoder::new(&mut out, EncoderConfig::fast());
            enc.write_page(black_box(&bm)).unwrap();
            enc.finish().unwrap();
            out
        });
    });
}

fn bench_encode_symbol_lossless(c: &mut Criterion) {
    let bm = glyph_repeat_bitmap(400, 200);
    c.bench_function("encode/sym-lossless/400x200/glyphs", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(4096);
            let mut enc = Jbig2Encoder::new(&mut out, EncoderConfig::balanced());
            enc.write_page(black_box(&bm)).unwrap();
            enc.finish().unwrap();
            out
        });
    });
}

fn bench_decode_generic(c: &mut Criterion) {
    let bm = stripe_bitmap(512, 256);
    let mut out = Vec::new();
    let mut enc = Jbig2Encoder::new(&mut out, EncoderConfig::fast());
    enc.write_page(&bm).unwrap();
    enc.finish().unwrap();
    c.bench_function("decode/generic/512x256/diagonals", |b| {
        b.iter(|| {
            let mut dec = Jbig2Decoder::new(Cursor::new(out.clone())).unwrap();
            black_box(dec.decode_page(1).unwrap())
        });
    });
}

criterion_group!(
    benches,
    bench_encode_generic,
    bench_encode_symbol_lossless,
    bench_decode_generic
);
criterion_main!(benches);
