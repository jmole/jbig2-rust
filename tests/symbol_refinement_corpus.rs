#![cfg(test)]

mod common;

use std::collections::HashMap;
use std::io::Cursor;

use common::corpus::{load_bucket, SizeBucket};
use common::oracles::{decode_with_jbig2_imageio, decode_with_jbig2dec};
use jbig2::coding::mq::{MqContexts, MqDecoder, MQ_NUM_CONTEXTS};
use jbig2::coding::mq_context::{IADS, IADT, IAFS, IAID, IAIT, IARDH, IARDW, IARDX, IARDY, IARI};
use jbig2::coding::mq_integer::{decode_iaid, decode_integer};
use jbig2::segments::{decode_refinement_region, decode_symbol_dictionary, sym_code_len};
use jbig2::segments::{
    FileHeader, SegmentHeader, SegmentType, SymbolDictionaryHeader, TextRegionHeader,
};
use jbig2::symbol::cc::extract_components;
use jbig2::symbol::classify::classify_lossy;
use jbig2::{Bitmap, EncoderConfig, Jbig2Decoder, Jbig2Encoder};

fn encode_with_config(bitmap: &Bitmap, cfg: EncoderConfig) -> Result<Vec<u8>, String> {
    let mut encoded = Vec::new();
    let mut enc = Jbig2Encoder::new(&mut encoded, cfg);
    enc.write_page(bitmap).map_err(|err| err.to_string())?;
    enc.finish().map_err(|err| err.to_string())?;
    Ok(encoded)
}

fn encode(bitmap: &Bitmap) -> Result<Vec<u8>, String> {
    encode_with_config(bitmap, EncoderConfig::max_compression())
}

fn round_trip(encoded: &[u8]) -> Result<Bitmap, String> {
    let mut dec = Jbig2Decoder::new(Cursor::new(encoded)).map_err(|err| err.to_string())?;
    dec.decode_page(1)
        .map(|page| page.bitmap)
        .map_err(|err| err.to_string())
}

fn corpus_page(bucket: SizeBucket, page_idx: usize) -> Bitmap {
    load_bucket(bucket, 25).unwrap_or_else(|err| panic!("load {bucket:?} bucket: {err}"))[page_idx]
        .1
        .clone()
}

fn assert_round_trip(bucket: SizeBucket, page_idx: usize) {
    let source = corpus_page(bucket, page_idx);
    let encoded =
        encode(&source).unwrap_or_else(|err| panic!("{bucket:?}[{page_idx}] encode failed: {err}"));
    let decoded = round_trip(&encoded).unwrap_or_else(|err| {
        panic!(
            "{bucket:?}[{page_idx}] decode failed: {err}\n{}",
            debug_stream(&source, &encoded)
        )
    });
    assert_eq!(
        decoded, source,
        "{bucket:?}[{page_idx}] round trip mismatch"
    );
    if let Some(jbig2dec_bitmap) = decode_with_jbig2dec(&encoded) {
        assert!(
            jbig2dec_bitmap == source,
            "{bucket:?}[{page_idx}] jbig2dec oracle mismatch: {}",
            bitmap_diff_summary(&source, &jbig2dec_bitmap)
        );
    }
    if let Some(jbig2_imageio_bitmap) = decode_with_jbig2_imageio(&encoded) {
        assert!(
            jbig2_imageio_bitmap == source,
            "{bucket:?}[{page_idx}] jbig2-imageio oracle mismatch: {}",
            bitmap_diff_summary(&source, &jbig2_imageio_bitmap)
        );
    }
}

fn debug_stream(source: &Bitmap, encoded: &[u8]) -> String {
    let mut cur = Cursor::new(encoded);
    let mut out = String::new();
    let mut dicts: HashMap<u32, Vec<Bitmap>> = HashMap::new();
    let expected_syms = expected_sorted_symbols(source);
    if FileHeader::read(&mut cur).is_err() {
        return "failed to parse file header".to_string();
    }
    loop {
        let Ok(seg) = SegmentHeader::read(&mut cur) else {
            break;
        };
        out.push_str(&format!(
            "seg {} {:?} page={} referred={:?}\n",
            seg.number, seg.segment_type, seg.page_association, seg.referred
        ));
        let mut body = vec![0u8; seg.data_length.unwrap_or(0) as usize];
        if std::io::Read::read_exact(&mut cur, &mut body).is_err() {
            out.push_str("  failed to read body\n");
            break;
        }
        match seg.segment_type {
            SegmentType::SymbolDictionary => {
                match SymbolDictionaryHeader::read(&mut body.as_slice()) {
                    Ok(h) => {
                        out.push_str(&format!(
                            "  sd: num_ex={} num_new={} template={} refagg={}\n",
                            h.num_ex_syms, h.num_new_syms, h.sd_template, h.sdrefagg
                        ));
                        let header_len = body.len() - {
                            let mut tmp = body.as_slice();
                            let _ = SymbolDictionaryHeader::read(&mut tmp);
                            tmp.len()
                        };
                        match decode_symbol_dictionary(&h, &body[header_len..], &[]) {
                            Ok(decoded) => {
                                out.push_str(&format!(
                                    "  sd decoded: exported={}\n",
                                    decoded.exported.len()
                                ));
                                if decoded.exported.len() == expected_syms.len() {
                                    let mismatches = decoded
                                        .exported
                                        .iter()
                                        .zip(expected_syms.iter())
                                        .enumerate()
                                        .filter(|(_, (got, want))| got != want)
                                        .map(|(idx, _)| idx)
                                        .take(8)
                                        .collect::<Vec<_>>();
                                    out.push_str(&format!(
                                        "  sd compare: {} mismatches{}\n",
                                        mismatches.len(),
                                        if mismatches.is_empty() {
                                            String::new()
                                        } else {
                                            format!(" at {:?}", mismatches)
                                        }
                                    ));
                                } else {
                                    out.push_str(&format!(
                                        "  sd compare: expected {} symbols\n",
                                        expected_syms.len()
                                    ));
                                }
                                dicts.insert(seg.number, decoded.exported);
                            }
                            Err(err) => out.push_str(&format!("  sd decode failed: {err}\n")),
                        }
                    }
                    Err(err) => out.push_str(&format!("  sd header parse failed: {err}\n")),
                }
            }
            SegmentType::ImmediateLosslessTextRegion
            | SegmentType::ImmediateTextRegion
            | SegmentType::IntermediateTextRegion => {
                match TextRegionHeader::read(&mut body.as_slice()) {
                    Ok(h) => {
                        out.push_str(&format!(
                            "  tr: instances={} sbrefine={} strips={} region={}x{} at={:?}\n",
                            h.num_instances,
                            h.sbrefine,
                            h.log_sbstrips,
                            h.region.width,
                            h.region.height,
                            h.rat
                        ));
                        let header_len = body.len() - {
                            let mut tmp = body.as_slice();
                            let _ = TextRegionHeader::read(&mut tmp);
                            tmp.len()
                        };
                        let sbsyms = seg
                            .referred
                            .iter()
                            .flat_map(|ref_no| dicts.get(ref_no).into_iter().flat_map(|v| v.iter()))
                            .collect::<Vec<_>>();
                        out.push_str(&format!("  tr symbols: {}\n", sbsyms.len()));
                        out.push_str(&debug_text_region_body(&h, &body[header_len..], &sbsyms));
                    }
                    Err(err) => out.push_str(&format!("  tr header parse failed: {err}\n")),
                }
            }
            _ => {}
        }
        if matches!(seg.segment_type, SegmentType::EndOfFile) {
            break;
        }
    }
    out
}

fn expected_sorted_symbols(source: &Bitmap) -> Vec<Bitmap> {
    let comps = extract_components(source);
    let cls = classify_lossy(&comps, 0.85);
    let mut order: Vec<u32> = (0..cls.symbols.len() as u32).collect();
    order.sort_by_key(|&i| cls.symbols[i as usize].height());
    order
        .into_iter()
        .map(|i| cls.symbols[i as usize].clone())
        .collect()
}

fn bitmap_diff_summary(expected: &Bitmap, actual: &Bitmap) -> String {
    if expected.width() != actual.width() || expected.height() != actual.height() {
        return format!(
            "dimensions {}x{} vs {}x{}",
            expected.width(),
            expected.height(),
            actual.width(),
            actual.height()
        );
    }
    for y in 0..expected.height() as usize {
        if expected.row(y) != actual.row(y) {
            return format!("first differing row {}", y);
        }
    }
    "bitmaps differ".to_string()
}

fn debug_text_region_body(header: &TextRegionHeader, body: &[u8], sbsyms: &[&Bitmap]) -> String {
    let mut out = String::new();
    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    let mut dec = MqDecoder::new(body);
    let code_len = sym_code_len(sbsyms.len() as u32);
    let sb_strips = 1u32 << header.log_sbstrips;
    let Ok(s0) = decode_integer(&mut dec, &mut cxs, IADT).ok_or("IADT initial OOB") else {
        out.push_str("  tr debug: failed initial IADT\n");
        return out;
    };
    let mut strip_t: i64 = -(s0 as i64) * sb_strips as i64;
    let mut first_s: i64 = 0;
    let mut ninst: u32 = 0;
    while ninst < header.num_instances {
        let Some(dt) = decode_integer(&mut dec, &mut cxs, IADT) else {
            out.push_str(&format!(
                "  tr debug: strip IADT OOB before instance {ninst}\n"
            ));
            return out;
        };
        strip_t = strip_t.saturating_add((dt as i64).saturating_mul(sb_strips as i64));
        let mut cur_s = 0i64;
        let mut first = true;
        loop {
            if first {
                let Some(dfs) = decode_integer(&mut dec, &mut cxs, IAFS) else {
                    out.push_str(&format!("  tr debug: IAFS OOB before instance {ninst}\n"));
                    return out;
                };
                first_s = first_s.saturating_add(dfs as i64);
                cur_s = first_s;
                first = false;
            } else {
                match decode_integer(&mut dec, &mut cxs, IADS) {
                    None => break,
                    Some(v) => cur_s = cur_s.saturating_add(v as i64 + header.sbds_offset as i64),
                }
            }
            let t_abs = if header.sbrefine {
                0
            } else if sb_strips != 1 {
                match decode_integer(&mut dec, &mut cxs, IAIT) {
                    Some(v) => strip_t.saturating_add(v as i64),
                    None => {
                        out.push_str(&format!("  tr debug: IAIT OOB before instance {ninst}\n"));
                        return out;
                    }
                }
            } else {
                strip_t
            };

            let id = decode_iaid(&mut dec, &mut cxs, IAID, code_len);
            out.push_str(&format!("    inst {ninst}: s={cur_s} t={t_abs} id={id}\n"));
            if (id as usize) >= sbsyms.len() {
                out.push_str("    -> id out of range\n");
                return out;
            }

            if header.sbrefine {
                let Some(r_flag) = decode_integer(&mut dec, &mut cxs, IARI) else {
                    out.push_str(&format!("    -> IARI OOB at instance {ninst}\n"));
                    return out;
                };
                out.push_str(&format!("      refine={r_flag}\n"));
                if r_flag != 0 {
                    let Some(rdw) = decode_integer(&mut dec, &mut cxs, IARDW) else {
                        out.push_str("      -> IARDW OOB\n");
                        return out;
                    };
                    let Some(rdh) = decode_integer(&mut dec, &mut cxs, IARDH) else {
                        out.push_str("      -> IARDH OOB\n");
                        return out;
                    };
                    let Some(rdx) = decode_integer(&mut dec, &mut cxs, IARDX) else {
                        out.push_str("      -> IARDX OOB\n");
                        return out;
                    };
                    let Some(rdy) = decode_integer(&mut dec, &mut cxs, IARDY) else {
                        out.push_str("      -> IARDY OOB\n");
                        return out;
                    };
                    out.push_str(&format!("      rdw={rdw} rdh={rdh} rdx={rdx} rdy={rdy}\n"));
                    let ref_bm = sbsyms[id as usize];
                    let ib_w = ref_bm.width() as i32 + rdw;
                    let ib_h = ref_bm.height() as i32 + rdh;
                    if ib_w <= 0 || ib_h <= 0 {
                        out.push_str("      -> non-positive refined size\n");
                        return out;
                    }
                    let ref_dx = rdw.div_euclid(2) + rdx;
                    let ref_dy = rdh.div_euclid(2) + rdy;
                    if let Err(err) = decode_refinement_region(
                        &mut dec,
                        &mut cxs,
                        ib_w as u32,
                        ib_h as u32,
                        if header.sbr_template { 1 } else { 0 },
                        false,
                        &header.rat,
                        ref_bm,
                        ref_dx,
                        ref_dy,
                    ) {
                        out.push_str(&format!("      -> refinement decode failed: {err}\n"));
                        return out;
                    }
                }
            }
            ninst += 1;
            if ninst >= header.num_instances {
                break;
            }
        }
    }
    out
}

#[test]
#[ignore = "regression harness for symbol refinement corpus failures"]
fn tiny_page0_round_trips_with_max_compression() {
    assert_round_trip(SizeBucket::Tiny, 0);
}

#[test]
#[ignore = "regression harness for symbol refinement corpus failures"]
fn small_page1_round_trips_with_max_compression() {
    assert_round_trip(SizeBucket::Small, 1);
}

#[test]
#[ignore = "regression harness for symbol refinement corpus failures"]
fn medium_page0_round_trips_with_max_compression() {
    assert_round_trip(SizeBucket::Medium, 0);
}

#[test]
#[ignore = "regression harness for symbol refinement corpus failures"]
fn large_page0_round_trips_with_max_compression() {
    assert_round_trip(SizeBucket::Large, 0);
}

#[test]
#[ignore = "oracle smoke test for non-refinement symbol streams"]
fn balanced_tiny_page0_round_trips_via_jbig2dec() {
    let source = corpus_page(SizeBucket::Tiny, 0);
    let encoded = encode_with_config(&source, EncoderConfig::balanced()).expect("encode balanced");
    if let Some(jbig2dec_bitmap) = decode_with_jbig2dec(&encoded) {
        assert!(
            jbig2dec_bitmap == source,
            "balanced tiny[0] jbig2dec mismatch: {}",
            bitmap_diff_summary(&source, &jbig2dec_bitmap)
        );
    }
}
