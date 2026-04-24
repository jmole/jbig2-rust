//! ITU-T T.88 conformance test driver.
//!
//! For each `TTn.jb2` shipped in the vendored conformance directory we decode
//! the stream and compare the page-0 bitmap against the matching
//! `..._TT00.bmp` reference produced by the reference encoder/decoder pair.
//!
//! Milestones unlock tests incrementally: M2 covers T9 and T10, M3 covers
//! T1..T3, and so on.

mod common;

use jbig2::Jbig2Decoder;
use std::io::Cursor;

use common::{conformance_dir, load_conformance_bmp, ReferenceImage};

fn decode_page(jb2: &str) -> jbig2::DecodedPage {
    let path = conformance_dir().join(jb2);
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let mut dec = Jbig2Decoder::new(Cursor::new(data)).expect("parse file header");
    dec.decode_page(1).expect("decode page 1")
}

fn compare_to_reference(jb2: &str, bmp: &str) {
    let decoded = decode_page(jb2);
    let expected = load_conformance_bmp(bmp);
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

/// Validates the encoder path: take the T10 reference bitmap, encode it with
/// the arithmetic generic-region encoder, then decode the result and assert
/// pixel identity. This proves the encoder is round-trip safe at page scale.
#[test]
fn arithmetic_generic_round_trip_page_scale() {
    use jbig2::coding::mq::{MqContexts, MqEncoder, MQ_NUM_CONTEXTS};
    use jbig2::segments::file_header::FileHeader;
    use jbig2::segments::generic_region::{encode_generic_arith, nominal_at, GenericRegionHeader};
    use jbig2::segments::page_information::{CombinationOp, PageInformation};
    use jbig2::segments::region_info::RegionInfo;
    use jbig2::segments::{SegmentHeader, SegmentType};

    let expected = match load_conformance_bmp("F01_200_TT10_TT00.bmp") {
        ReferenceImage::Mono(bm) => bm,
        ReferenceImage::Rgb(_) => panic!("expected monochrome TT10 reference"),
    };
    let width = expected.width();
    let height = expected.height();

    let mut out = Vec::new();
    FileHeader {
        sequential: true,
        unknown_page_count: false,
        uses_extended_template: false,
        uses_colour: false,
        num_pages: Some(1),
    }
    .write(&mut out)
    .unwrap();

    let page = PageInformation {
        width,
        height,
        x_resolution: 0,
        y_resolution: 0,
        is_lossless: true,
        may_contain_refinements: false,
        default_pixel: 0,
        default_combination_op: CombinationOp::Or,
        requires_aux_buffers: false,
        combination_op_override: false,
        may_contain_colour: false,
        is_striped: false,
        maximum_stripe_size: 0,
    };
    SegmentHeader {
        number: 0,
        segment_type: SegmentType::PageInformation,
        deferred_non_retain: false,
        referred: vec![],
        retain_bits: vec![false],
        page_association: 1,
        data_length: Some(PageInformation::WIRE_SIZE),
    }
    .write(&mut out)
    .unwrap();
    page.write(&mut out).unwrap();

    let hdr = GenericRegionHeader {
        region: RegionInfo {
            width,
            height,
            x: 0,
            y: 0,
            external_combination_op: CombinationOp::Or,
            colour_extension: false,
        },
        mmr: false,
        template: 0,
        tpgdon: true,
        ext_template: false,
        at: nominal_at(0, false),
    };
    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    let mut enc = MqEncoder::new(expected.data().len() / 4);
    encode_generic_arith(&mut enc, &mut cxs, &hdr, &expected).unwrap();
    let coded = enc.finish();
    let mut hdr_bytes = Vec::new();
    hdr.write(&mut hdr_bytes).unwrap();
    let data_len = hdr_bytes.len() + coded.len();
    SegmentHeader {
        number: 1,
        segment_type: SegmentType::ImmediateLosslessGenericRegion,
        deferred_non_retain: false,
        referred: vec![],
        retain_bits: vec![false],
        page_association: 1,
        data_length: Some(data_len as u32),
    }
    .write(&mut out)
    .unwrap();
    out.extend_from_slice(&hdr_bytes);
    out.extend_from_slice(&coded);
    SegmentHeader {
        number: 2,
        segment_type: SegmentType::EndOfPage,
        deferred_non_retain: false,
        referred: vec![],
        retain_bits: vec![false],
        page_association: 1,
        data_length: Some(0),
    }
    .write(&mut out)
    .unwrap();

    let compressed_size = out.len();
    let uncompressed_size = (width as usize + 7) / 8 * height as usize;
    eprintln!(
        "page {}x{}: encoded={} bytes, raw={} bytes, ratio={:.2}",
        width,
        height,
        compressed_size,
        uncompressed_size,
        uncompressed_size as f64 / compressed_size as f64
    );

    let mut dec = Jbig2Decoder::new(Cursor::new(out)).unwrap();
    let page = dec.decode_page(1).unwrap();
    assert_eq!(
        page.bitmap, expected,
        "encoder round-trip produced mismatched bitmap"
    );
}
