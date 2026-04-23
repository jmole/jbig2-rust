//! Diagnostic helper for the Huffman-heavy T.88 conformance streams.

#[path = "../tests/common/mod.rs"]
mod common;

use common::conformance_dir;
use jbig2::Bitmap;
use jbig2::coding::huffman::{HuffReader, standard_table};
use jbig2::segments::file_header::FileHeader;
use jbig2::segments::halftone_region::{HalftoneRegionHeader, decode_halftone_region};
use jbig2::segments::header::SegmentHeader;
use jbig2::segments::pattern_dictionary::{PatternDictionaryHeader, decode_pattern_dictionary};
use jbig2::segments::symbol_dictionary::{SymbolDictionaryHeader, decode_symbol_dictionary};
use jbig2::segments::text_region::{TextRegionHeader, decode_text_region};
use jbig2::segments::SegmentType;
use std::collections::HashMap;
use std::io::Cursor;

fn main() {
    let cmd = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "usage".to_string());
    match cmd.as_str() {
        "list-tt1" => list_segments("codeStreamTest1_TT1.jb2"),
        "list-tt2" => list_segments("codeStreamTest1_TT2.jb2"),
        "list-tt8" => list_segments("codeStreamTest3_TT8.jb2"),
        "trace-tt2" => trace_tt2(),
        "decode-tt1-text" => decode_tt1_page1_text_only(),
        "decode-tt1-halftone" => decode_tt1_page1_halftone_only(),
        _ => {
            eprintln!("usage: cargo run --example tt_huff_diag -- <command>");
            eprintln!("  list-tt1");
            eprintln!("  list-tt2");
            eprintln!("  list-tt8");
            eprintln!("  trace-tt2");
            eprintln!("  decode-tt1-text");
            eprintln!("  decode-tt1-halftone");
        }
    }
}

fn load_stream(name: &str) -> Vec<u8> {
    std::fs::read(conformance_dir().join(name)).expect("read conformance stream")
}

fn list_segments(name: &str) {
    let data = load_stream(name);
    let mut cur = Cursor::new(&data[..]);
    let _fh = FileHeader::read(&mut cur).unwrap();
    loop {
        let sh = match SegmentHeader::read(&mut cur) {
            Ok(s) => s,
            Err(_) => break,
        };
        let len = sh.data_length.unwrap_or(0) as usize;
        let body_start = cur.position() as usize;
        let body_end = body_start + len;
        if body_end > data.len() {
            break;
        }
        let body = &data[body_start..body_end];
        println!(
            "Segment: page={} num={} type={:?}, refs={:?}, dlen={:?}",
            sh.page_association, sh.number, sh.segment_type, sh.referred, sh.data_length
        );
        if matches!(
            sh.segment_type,
            SegmentType::IntermediateTextRegion
                | SegmentType::ImmediateTextRegion
                | SegmentType::ImmediateLosslessTextRegion
        ) {
            let mut bcur = Cursor::new(body);
            match TextRegionHeader::read(&mut bcur) {
                Ok(th) => println!(
                    "  TR: w={} h={} sbhuff={} sbrefine={} color_ext={} num_inst={}",
                    th.region.width,
                    th.region.height,
                    th.sbhuff,
                    th.sbrefine,
                    th.region.colour_extension,
                    th.num_instances
                ),
                Err(e) => println!("  TR parse err: {:?}", e),
            }
        }
        if matches!(sh.segment_type, SegmentType::SymbolDictionary) {
            let mut bcur = Cursor::new(body);
            match SymbolDictionaryHeader::read(&mut bcur) {
                Ok(sd) => println!(
                    "  SD: num_new={} num_ex={} dh_sel={} dw_sel={} bmsize_sel={} agginst_sel={} sdrefagg={}",
                    sd.num_new_syms,
                    sd.num_ex_syms,
                    sd.sdhuff_dh,
                    sd.sdhuff_dw,
                    sd.sdhuff_bmsize,
                    sd.sdhuff_agginst,
                    sd.sdrefagg
                ),
                Err(e) => println!("  SD parse err: {:?}", e),
            }
        }
        cur.set_position(body_end as u64);
    }
}

fn trace_tt2() {
    let data = load_stream("codeStreamTest1_TT2.jb2");
    let mut cur = Cursor::new(&data[..]);
    let _fh = FileHeader::read(&mut cur).unwrap();
    loop {
        let sh = match SegmentHeader::read(&mut cur) {
            Ok(s) => s,
            Err(_) => break,
        };
        let len = sh.data_length.unwrap_or(0) as usize;
        let body_start = cur.position() as usize;
        let body_end = body_start + len;
        if body_end > data.len() {
            break;
        }
        let body = &data[body_start..body_end];
        println!(
            "Segment: type={:?}, dlen={:?}, body_start={}, body_end={}",
            sh.segment_type, sh.data_length, body_start, body_end
        );
        if matches!(sh.segment_type, SegmentType::SymbolDictionary) {
            let mut bcur = Cursor::new(body);
            let hdr = SymbolDictionaryHeader::read(&mut bcur).unwrap();
            let header_len = bcur.position() as usize;
            let sd_body = &body[header_len..];
            let dh_table = standard_table(if hdr.sdhuff_dh == 0 { 4 } else { 5 }).unwrap();
            let dw_table = standard_table(if hdr.sdhuff_dw == 0 { 2 } else { 3 }).unwrap();
            let b1_table = standard_table(1).unwrap();
            let mut r = HuffReader::new(sd_body);
            let mut new_count = 0u32;
            let mut hc_h = 0i32;
            while new_count < hdr.num_new_syms {
                let hcdh = dh_table.decode(&mut r).unwrap().unwrap();
                hc_h += hcdh;
                println!("  HCDH={}, hc_h={}, pos={}", hcdh, hc_h, r.byte_pos());
                let mut widths = Vec::new();
                let mut sym_w = 0i32;
                loop {
                    match dw_table.decode(&mut r).unwrap() {
                        Some(v) => {
                            sym_w += v;
                            widths.push(sym_w);
                            println!("    DW={}, w={}, pos={}", v, sym_w, r.byte_pos());
                        }
                        None => {
                            println!("    OOB DW (end of class), pos={}", r.byte_pos());
                            break;
                        }
                    }
                }
                let bmsize = b1_table.decode(&mut r).unwrap().unwrap();
                r.byte_align();
                println!("  BMSIZE={}, after-align pos={}", bmsize, r.byte_pos());
                if bmsize == 0 {
                    let total_width = widths.iter().sum::<i32>() as usize;
                    let bytes_per_row = (total_width + 7) / 8;
                    r.skip_bytes(bytes_per_row * hc_h as usize);
                } else {
                    let pos = r.byte_pos();
                    let mut buf = jbig2::coding::mmr_lut::MmrBitBuf::new(&sd_body[pos..]);
                    for &w in &widths {
                        let _ = jbig2::segments::generic_region::decode_generic_mmr_streaming(
                            &mut buf,
                            w as u32,
                            hc_h as u32,
                        )
                        .unwrap();
                        let bp = buf.bit_pos();
                        let aligned = (bp + 7) & !7;
                        let pad = aligned - bp;
                        if pad > 0 {
                            buf.consume_padding(pad as u8).unwrap();
                        }
                    }
                    r.skip_bytes(buf.byte_pos());
                }
                new_count += widths.len() as u32;
            }
        }
        cur.set_position(body_end as u64);
    }
}

fn decode_tt1_page1_text_only() {
    let data = load_stream("codeStreamTest1_TT1.jb2");
    let mut cur = Cursor::new(&data[..]);
    let _fh = FileHeader::read(&mut cur).unwrap();
    let mut dicts: HashMap<u32, Vec<Bitmap>> = HashMap::new();

    loop {
        let sh = match SegmentHeader::read(&mut cur) {
            Ok(s) => s,
            Err(_) => break,
        };
        let len = sh.data_length.unwrap_or(0) as usize;
        let body_start = cur.position() as usize;
        let body_end = body_start + len;
        if body_end > data.len() {
            break;
        }
        let body = &data[body_start..body_end];
        if matches!(sh.segment_type, SegmentType::SymbolDictionary) {
            let mut bcur = Cursor::new(body);
            let hdr = SymbolDictionaryHeader::read(&mut bcur).unwrap();
            let header_len = bcur.position() as usize;
            let mut imports = Vec::new();
            for seg in &sh.referred {
                if let Some(ex) = dicts.get(seg) {
                    imports.extend(ex.iter());
                }
            }
            let decoded = decode_symbol_dictionary(&hdr, &body[header_len..], &imports).unwrap();
            dicts.insert(sh.number, decoded.exported);
        } else if matches!(sh.segment_type, SegmentType::ImmediateLosslessTextRegion)
            && sh.page_association == 1
        {
            let mut bcur = Cursor::new(body);
            let hdr = TextRegionHeader::read(&mut bcur).unwrap();
            let header_len = bcur.position() as usize;
            let mut symbols = Vec::new();
            for seg in &sh.referred {
                if let Some(ex) = dicts.get(seg) {
                    symbols.extend(ex.iter());
                }
            }
            let bm = decode_text_region(&hdr, &body[header_len..], &symbols).unwrap();
            println!("decoded text region {}x{}", bm.width(), bm.height());
            break;
        }
        cur.set_position(body_end as u64);
    }
}

fn decode_tt1_page1_halftone_only() {
    let data = load_stream("codeStreamTest1_TT1.jb2");
    let mut cur = Cursor::new(&data[..]);
    let _fh = FileHeader::read(&mut cur).unwrap();
    let mut pats: HashMap<u32, Vec<Bitmap>> = HashMap::new();

    loop {
        let sh = match SegmentHeader::read(&mut cur) {
            Ok(s) => s,
            Err(_) => break,
        };
        let len = sh.data_length.unwrap_or(0) as usize;
        let body_start = cur.position() as usize;
        let body_end = body_start + len;
        if body_end > data.len() {
            break;
        }
        let body = &data[body_start..body_end];
        if matches!(sh.segment_type, SegmentType::PatternDictionary) && sh.page_association == 1 {
            let mut bcur = Cursor::new(body);
            let hdr = PatternDictionaryHeader::read(&mut bcur).unwrap();
            let header_len = bcur.position() as usize;
            pats.insert(
                sh.number,
                decode_pattern_dictionary(&hdr, &body[header_len..]).unwrap(),
            );
        } else if matches!(sh.segment_type, SegmentType::ImmediateLosslessHalftoneRegion)
            && sh.page_association == 1
        {
            let mut bcur = Cursor::new(body);
            let hdr = HalftoneRegionHeader::read(&mut bcur).unwrap();
            let header_len = bcur.position() as usize;
            let mut refs = Vec::new();
            for seg in &sh.referred {
                if let Some(ps) = pats.get(seg) {
                    refs.extend(ps.iter());
                }
            }
            let bm = decode_halftone_region(&hdr, &body[header_len..], &refs).unwrap();
            println!("decoded halftone region {}x{}", bm.width(), bm.height());
            break;
        }
        cur.set_position(body_end as u64);
    }
}
