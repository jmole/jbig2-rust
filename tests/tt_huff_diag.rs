//! Temporary diagnostic for Huffman-coded conformance files.

mod common;

use common::conformance_dir;
use jbig2::Bitmap;
use jbig2::coding::huffman::{standard_table, HuffReader};
use jbig2::segments::file_header::FileHeader;
use jbig2::segments::halftone_region::{decode_halftone_region, HalftoneRegionHeader};
use jbig2::segments::header::SegmentHeader;
use jbig2::segments::pattern_dictionary::{decode_pattern_dictionary, PatternDictionaryHeader};
use jbig2::segments::symbol_dictionary::decode_symbol_dictionary;
use jbig2::segments::symbol_dictionary::SymbolDictionaryHeader;
use jbig2::segments::text_region::decode_text_region;
use jbig2::segments::text_region::TextRegionHeader;
use jbig2::segments::SegmentType;
use std::collections::HashMap;
use std::io::Cursor;

#[test]
#[ignore]
fn list_tt2_segments() {
    let data = std::fs::read(conformance_dir().join("codeStreamTest1_TT2.jb2")).unwrap();
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
        if body_end > data.len() { break; }
        let body = &data[body_start..body_end];
        println!("Segment: type={:?}, dlen={:?}, body_start={}, body_end={}",
            sh.segment_type, sh.data_length, body_start, body_end);
        if matches!(sh.segment_type, SegmentType::IntermediateTextRegion | SegmentType::ImmediateTextRegion | SegmentType::ImmediateLosslessTextRegion) {
            let mut bcur = Cursor::new(body);
            match TextRegionHeader::read(&mut bcur) {
                Ok(th) => println!("  TR: w={} h={} sbhuff={} sbrefine={} num_inst={}",
                    th.region.width, th.region.height, th.sbhuff, th.sbrefine, th.num_instances),
                Err(e) => println!("  TR parse err: {:?}", e),
            }
        }
        cur.set_position(body_end as u64);
    }
}

#[test]
#[ignore]
fn list_tt1_segments() {
    let data = std::fs::read(conformance_dir().join("codeStreamTest1_TT1.jb2")).unwrap();
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
            "Segment: page={} type={:?}, dlen={:?}, body_start={}, body_end={}",
            sh.page_association, sh.segment_type, sh.data_length, body_start, body_end
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
                    "  TR: w={} h={} sbhuff={} sbrefine={} num_inst={}",
                    th.region.width, th.region.height, th.sbhuff, th.sbrefine, th.num_instances
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

#[test]
#[ignore]
fn trace_tt2() {
    let data = std::fs::read(conformance_dir().join("codeStreamTest1_TT2.jb2")).unwrap();
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
        println!("Segment: type={:?}, dlen={:?}, body_start={}, body_end={}",
            sh.segment_type, sh.data_length, body_start, body_end);
        if matches!(sh.segment_type, SegmentType::IntermediateTextRegion | SegmentType::ImmediateTextRegion | SegmentType::ImmediateLosslessTextRegion) {
            let mut bcur = Cursor::new(body);
            match TextRegionHeader::read(&mut bcur) {
                Ok(th) => println!("  TR: w={} h={} sbhuff={} sbrefine={} num_inst={}",
                    th.region.width, th.region.height, th.sbhuff, th.sbrefine, th.num_instances),
                Err(e) => println!("  TR parse err: {:?}", e),
            }
        }
        if matches!(sh.segment_type, SegmentType::SymbolDictionary) {
            let mut bcur = Cursor::new(body);
            let sh = SymbolDictionaryHeader::read(&mut bcur).unwrap();
            let header_len = bcur.position() as usize;
            let sd_body = &body[header_len..];
            println!("SD: header_len={} sd_body_len={} num_new={} num_ex={} dh_sel={} dw_sel={} bmsize_sel={} agginst_sel={} sdrefagg={}",
                header_len, sd_body.len(),                 sh.num_new_syms, sh.num_ex_syms,
                sh.sdhuff_dh, sh.sdhuff_dw, sh.sdhuff_bmsize, sh.sdhuff_agginst, sh.sdrefagg);
            println!("  raw bytes: {:02x?}", sd_body);
            // Trace decoding
            let dh_table = standard_table(if sh.sdhuff_dh == 0 { 4 } else { 5 }).unwrap();
            let dw_table = standard_table(if sh.sdhuff_dw == 0 { 2 } else { 3 }).unwrap();
            let b1_table = standard_table(1).unwrap();
            let mut r = HuffReader::new(sd_body);
            let mut new_count = 0;
            let mut hc_h = 0i32;
            while new_count < sh.num_new_syms {
                let hcdh = dh_table.decode(&mut r).unwrap().unwrap();
                hc_h += hcdh;
                println!("  HCDH={}, hc_h={}, pos={}", hcdh, hc_h, r.byte_pos());
                let mut widths = vec![];
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
                    let totw = widths.iter().sum::<i32>() as usize;
                    let bpr = (totw + 7) / 8;
                    let total_bytes = bpr * hc_h as usize;
                    println!("    uncompressed: totw={} bpr={} bytes={}", totw, bpr, total_bytes);
                    r.skip_bytes(total_bytes);
                } else {
                    let pos = r.byte_pos();
                    let mut buf = jbig2::coding::mmr_lut::MmrBitBuf::new(&sd_body[pos..]);
                    for &w in &widths {
                        let _ = jbig2::segments::generic_region::decode_generic_mmr_streaming(&mut buf, w as u32, hc_h as u32).unwrap();
                        let bp = buf.bit_pos();
                        let aligned = (bp + 7) & !7;
                        let pad = aligned - bp;
                        if pad > 0 { buf.consume_padding(pad as u8).unwrap(); }
                    }
                    let consumed = buf.byte_pos();
                    println!("    BMSIZE>0 per-symbol MMR: consumed {} bytes for {} widths", consumed, widths.len());
                    r.skip_bytes(consumed);
                }
                new_count += widths.len() as u32;
            }
            // Export-flag run
            let total = sh.num_new_syms as i64;
            let mut idx = 0i64;
            let mut cur = 0;
            while idx < total {
                let run = b1_table.decode(&mut r).unwrap().unwrap() as i64;
                println!("  EX run={}, idx={}, cur={}, pos={}", run, idx, cur, r.byte_pos());
                idx += run;
                cur ^= 1;
            }
            println!("  final pos={}, body_len={}", r.byte_pos(), sd_body.len());
        }
        cur.set_position(body_end as u64);
    }
}

#[test]
#[ignore]
fn decode_tt1_page1_text_only() {
    let data = std::fs::read(conformance_dir().join("codeStreamTest1_TT1.jb2")).unwrap();
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
                    for bm in ex {
                        imports.push(bm);
                    }
                }
            }
            let import_refs: Vec<&Bitmap> = imports.iter().copied().collect();
            let decoded = decode_symbol_dictionary(&hdr, &body[header_len..], &import_refs).unwrap();
            println!(
                "decoded SD seg#{} page={} imports={} new={} exported={}",
                sh.number,
                sh.page_association,
                import_refs.len(),
                decoded.new_symbols.len(),
                decoded.exported.len()
            );
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
                    for bm in ex {
                        symbols.push(bm);
                    }
                }
            }
            let symbol_refs: Vec<&Bitmap> = symbols.iter().copied().collect();
            println!(
                "decoding page1 text: refs={:?} syms={} body_len={} sbhuff={} log_sbstrips={}",
                sh.referred,
                symbol_refs.len(),
                body.len() - header_len,
                hdr.sbhuff,
                hdr.log_sbstrips
            );
            let bm = decode_text_region(&hdr, &body[header_len..], &symbol_refs).unwrap();
            println!("decoded text region {}x{}", bm.width(), bm.height());
            break;
        }

        cur.set_position(body_end as u64);
    }
}

#[test]
#[ignore]
fn decode_tt1_page1_halftone_only() {
    let data = std::fs::read(conformance_dir().join("codeStreamTest1_TT1.jb2")).unwrap();
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
            let decoded = decode_pattern_dictionary(&hdr, &body[header_len..]).unwrap();
            println!(
                "decoded pattern dict seg#{} page={} patterns={} hdmmr={} template={}",
                sh.number,
                sh.page_association,
                decoded.len(),
                hdr.hdmmr,
                hdr.hd_template
            );
            pats.insert(sh.number, decoded);
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
            let refs: Vec<&Bitmap> = refs.iter().copied().collect();
            println!(
                "decoding page1 halftone: refs={:?} pats={} body_len={} hmmr={} hgw={} hgh={}",
                sh.referred,
                refs.len(),
                body.len() - header_len,
                hdr.hmmr,
                hdr.hgw,
                hdr.hgh
            );
            let mut mmr = jbig2::coding::mmr_lut::MmrBitBuf::new(&body[header_len..]);
            for plane in 0..4 {
                let _bm = jbig2::segments::generic_region::decode_generic_mmr_streaming(
                    &mut mmr,
                    hdr.hgw,
                    hdr.hgh,
                )
                .unwrap();
                if mmr.peek_bits(12) == 1 {
                    mmr.skip_bits(12).unwrap();
                    if mmr.peek_bits(12) == 1 {
                        mmr.skip_bits(12).unwrap();
                    }
                }
                let bp = mmr.bit_pos();
                let aligned = (bp + 7) & !7;
                let pad = aligned - bp;
                if pad > 0 {
                    mmr.consume_padding(pad as u8).unwrap();
                }
                println!("  plane {} consumed {} bytes", plane, mmr.byte_pos());
            }
            let bm = decode_halftone_region(&hdr, &body[header_len..], &refs).unwrap();
            println!("decoded halftone region {}x{}", bm.width(), bm.height());
            break;
        }

        cur.set_position(body_end as u64);
    }
}

#[test]
#[ignore]
fn list_tt8_segments() {
    let data = std::fs::read(conformance_dir().join("codeStreamTest3_TT8.jb2")).unwrap();
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
        cur.set_position(body_end as u64);
    }
}
