//! ITU-T T.88 conformance test driver.
//!
//! For each `TTn.jb2` shipped in the vendored conformance directory we decode
//! the stream and compare the page-0 bitmap against the matching
//! `..._TT00.bmp` reference produced by the reference encoder/decoder pair.
//!
//! Milestones unlock tests incrementally: M2 covers T9 and T10, M3 covers
//! T1..T3, and so on.

mod common;

use jbig2::{EncoderConfig, Jbig2Decoder, Jbig2Encoder};
use std::io::Cursor;

use common::{conformance_dir, load_bmp_path, load_conformance_bmp, ReferenceImage};

fn decode_page(jb2: &str) -> jbig2::DecodedPage {
    let path = conformance_dir().join(jb2);
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let mut dec = Jbig2Decoder::new(Cursor::new(data)).expect("parse file header");
    dec.decode_page_raw(1).expect("decode page 1")
}

fn assert_decoded_matches(jb2: &str, decoded: jbig2::DecodedPage, expected: ReferenceImage) {
    match expected {
        ReferenceImage::Mono(expected) => {
            let decoded = decoded.bitmap;
            assert_eq!(
                decoded.width(),
                expected.width(),
                "{jb2}: width mismatch {} vs {}",
                decoded.width(),
                expected.width()
            );
            assert_eq!(
                decoded.height(),
                expected.height(),
                "{jb2}: height mismatch {} vs {}",
                decoded.height(),
                expected.height()
            );
            if decoded != expected {
                for y in 0..decoded.height() {
                    if decoded.row(y as usize) != expected.row(y as usize) {
                        panic!("{jb2}: pixel mismatch, first differing row {y}");
                    }
                }
                panic!("{jb2}: bitmaps differ (height-independent)");
            }
        }
        ReferenceImage::Rgb(expected) => {
            let decoded = decoded.rgb_bitmap.expect("decoded rgb page");
            assert_eq!(
                decoded.width(),
                expected.width(),
                "{jb2}: width mismatch {} vs {}",
                decoded.width(),
                expected.width()
            );
            assert_eq!(
                decoded.height(),
                expected.height(),
                "{jb2}: height mismatch {} vs {}",
                decoded.height(),
                expected.height()
            );
            if decoded != expected {
                for y in 0..decoded.height() {
                    if decoded.row(y as usize) != expected.row(y as usize) {
                        panic!("{jb2}: RGB pixel mismatch, first differing row {y}");
                    }
                }
                panic!("{jb2}: RGB bitmaps differ (height-independent)");
            }
        }
    }
}

fn compare_to_reference(jb2: &str, bmp: &str) {
    let decoded = decode_page(jb2);
    assert_decoded_matches(jb2, decoded, load_conformance_bmp(bmp));
}

fn compare_page_to_reference(jb2: &str, page_no: u32, bmp: &str) {
    let path = conformance_dir().join(jb2);
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let mut dec = Jbig2Decoder::new(Cursor::new(data)).expect("parse file header");
    let decoded = dec
        .decode_page_raw(page_no)
        .unwrap_or_else(|e| panic!("decode page {page_no}: {e}"));
    assert_decoded_matches(jb2, decoded, load_conformance_bmp(bmp));
}

fn compare_path_page_to_reference(
    jb2: &str,
    jb2_path: &std::path::Path,
    page_no: u32,
    bmp_path: &std::path::Path,
) {
    let data = std::fs::read(jb2_path).unwrap_or_else(|e| panic!("read {jb2_path:?}: {e}"));
    let mut dec = Jbig2Decoder::new(Cursor::new(data)).expect("parse file header");
    let decoded = dec
        .decode_page_raw(page_no)
        .unwrap_or_else(|e| panic!("decode page {page_no}: {e}"));
    assert_decoded_matches(jb2, decoded, load_bmp_path(bmp_path));
}

// T.6 MMR generic region against the T.88 reference stream. The earlier
// desync at line 66 was resolved once the LUT-driven decoder in
// `crate::coding::mmr_lut` took over the hot path; see `tests/mmr_diag.rs`
// for a slow-vs-fast line-by-line cross-check over the same codestream.
#[test]
fn tt9_mmr_generic_region() {
    compare_to_reference("F01_200_TT9.jb2", "F01_200_TT9_TT00.bmp");
}

#[test]
fn tt10_arithmetic_generic_region() {
    compare_to_reference("F01_200_TT10.jb2", "F01_200_TT10_TT00.bmp");
}

// TT1..TT3 cover the Huffman-coded symbol path. TT1 additionally pulls in
// pattern-dictionary + halftone decode on page 1, while TT2 exercises the
// Huffman SD variant in isolation and TT3 stays on the arithmetic symbol
// path.
#[test]
fn tt1_pattern_plus_symbols() {
    compare_to_reference("codeStreamTest1_TT1.jb2", "codeStreamTest1_TT1_TT00.bmp");
}

#[test]
fn tt1_page3_refagg_symbols() {
    compare_page_to_reference("codeStreamTest1_TT1.jb2", 3, "codeStreamTest1_TT1_TT02.bmp");
}

#[test]
fn tt2_huffman_symbol_region() {
    compare_to_reference("codeStreamTest1_TT2.jb2", "codeStreamTest1_TT2_TT00.bmp");
}

#[test]
fn tt3_huffman_symbol_region() {
    compare_to_reference("codeStreamTest1_TT3.jb2", "codeStreamTest1_TT3_TT00.bmp");
}

// TT4..TT7: arithmetic-coded symbol dictionary + text region. These exercise
// the "export all new symbols" shortcut that the reference encoder bakes
// into the IAEX run-length stream (Jbig2ENC.cpp:434-435 / 514-515 emits
// `IAEX(0), IAEX(0)` regardless of SBNUMSYMS). Our decoder accepts that
// pattern when SDNUMEXSYMS == SDNUMNEWSYMS.
#[test]
fn tt4_arithmetic_symbol_region() {
    compare_to_reference("codeStreamTest1_TT4.jb2", "codeStreamTest1_TT4_TT00.bmp");
}

// TT5 exercises symbol-dictionary refinement/aggregate coding. The decoder
// intentionally follows the ITU sample encoder's refinement context order
// here, because the T.88 conformance streams were authored with that code.
#[test]
fn tt5_arithmetic_symbol_region_imports() {
    compare_to_reference("codeStreamTest1_TT5.jb2", "codeStreamTest1_TT5_TT00.bmp");
}

#[test]
fn tt6_arithmetic_symbol_region() {
    compare_to_reference("codeStreamTest2_TT6.jb2", "codeStreamTest2_TT6_TT00.bmp");
}

#[test]
fn tt7_arithmetic_symbol_region() {
    compare_to_reference("codeStreamTest1_TT7.jb2", "codeStreamTest1_TT7_TT00.bmp");
}

#[test]
fn tt8_colour_text_region() {
    compare_to_reference("codeStreamTest3_TT8.jb2", "codeStreamTest3_TT8_TT00.bmp");
}

#[test]
fn annex_h_page1_artifex_fixture() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    compare_path_page_to_reference(
        "annex-h.jbig2",
        &root.join("vendor").join("jbig2dec").join("annex-h.jbig2"),
        1,
        &root
            .join("vendor")
            .join("T-REC-T.88-201808")
            .join("spec")
            .join("annex-h-page-00.bmp"),
    );
}

/// Validates the encoder path: take the T10 reference bitmap, encode it through
/// the public encoder API, then decode the result and assert pixel identity.
#[test]
fn arithmetic_generic_round_trip_page_scale() {
    let expected = match load_conformance_bmp("F01_200_TT10_TT00.bmp") {
        ReferenceImage::Mono(bm) => bm,
        ReferenceImage::Rgb(_) => panic!("expected monochrome TT10 reference"),
    };

    let mut out = Vec::new();
    let mut enc = Jbig2Encoder::new(&mut out, EncoderConfig::fast());
    enc.write_bitmap(&expected).unwrap();
    enc.finish().unwrap();

    let compressed_size = out.len();
    let uncompressed_size = expected.data().len();
    eprintln!(
        "page {}x{}: encoded={} bytes, raw={} bytes, ratio={:.2}",
        expected.width(),
        expected.height(),
        compressed_size,
        uncompressed_size,
        uncompressed_size as f64 / compressed_size as f64
    );

    let mut dec = Jbig2Decoder::new(Cursor::new(out)).unwrap();
    let page = dec.decode_page_raw(1).unwrap();
    assert_eq!(
        page.bitmap, expected,
        "encoder round-trip produced mismatched bitmap"
    );
}
