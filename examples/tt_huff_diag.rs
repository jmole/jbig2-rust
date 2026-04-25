//! Diagnostic helper for the Huffman-heavy T.88 conformance streams.

#[path = "../tests/common/mod.rs"]
mod common;

use common::{conformance_dir, load_conformance_bmp, ReferenceImage};
use jbig2::coding::huffman::{standard_table, HuffReader};
use jbig2::segments::file_header::FileHeader;
use jbig2::segments::generic_region::{decode_generic_mmr, GenericRegionHeader};
use jbig2::segments::halftone_region::{decode_halftone_region, HalftoneRegionHeader};
use jbig2::segments::header::SegmentHeader;
use jbig2::segments::pattern_dictionary::{decode_pattern_dictionary, PatternDictionaryHeader};
use jbig2::segments::symbol_dictionary::{decode_symbol_dictionary, SymbolDictionaryHeader};
use jbig2::segments::text_region::{decode_text_region, TextRegionHeader};
use jbig2::segments::SegmentType;
use jbig2::Bitmap;
use jbig2::Jbig2Decoder;
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
        "trace-tt1-page3-text" => trace_tt1_page3_text(),
        "trace-tt5-refagg" => trace_tt5_refagg(),
        "trace-annex-h-page1" => trace_annex_h_page1(),
        _ => {
            eprintln!("usage: cargo run --example tt_huff_diag -- <command>");
            eprintln!("  list-tt1");
            eprintln!("  list-tt2");
            eprintln!("  list-tt8");
            eprintln!("  trace-tt2");
            eprintln!("  decode-tt1-text");
            eprintln!("  decode-tt1-halftone");
            eprintln!("  trace-tt1-page3-text");
            eprintln!("  trace-tt5-refagg");
            eprintln!("  trace-annex-h-page1");
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
                    "  SD: num_new={} num_ex={} dh_sel={} dw_sel={} bmsize_sel={} agginst_sel={} sdrefagg={} rtemplate={}",
                    sd.num_new_syms,
                    sd.num_ex_syms,
                    sd.sdhuff_dh,
                    sd.sdhuff_dw,
                    sd.sdhuff_bmsize,
                    sd.sdhuff_agginst,
                    sd.sdrefagg,
                    sd.sd_rtemplate
                ),
                Err(e) => println!("  SD parse err: {:?}", e),
            }
        }
        if matches!(
            sh.segment_type,
            SegmentType::ImmediateGenericRegion
                | SegmentType::ImmediateLosslessGenericRegion
                | SegmentType::IntermediateGenericRegion
        ) {
            let mut bcur = Cursor::new(body);
            match GenericRegionHeader::read(&mut bcur) {
                Ok(gh) => println!(
                    "  GR: w={} h={} x={} y={} op={:?} mmr={} template={} tpgdon={}",
                    gh.region.width,
                    gh.region.height,
                    gh.region.x,
                    gh.region.y,
                    gh.region.external_combination_op,
                    gh.mmr,
                    gh.template,
                    gh.tpgdon
                ),
                Err(e) => println!("  GR parse err: {:?}", e),
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
        } else if matches!(
            sh.segment_type,
            SegmentType::ImmediateLosslessHalftoneRegion
        ) && sh.page_association == 1
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

fn trace_tt1_page3_text() {
    let data = load_stream("codeStreamTest1_TT1.jb2");
    println!("TT1 full page-3 decode:");
    match Jbig2Decoder::new(Cursor::new(&data[..])).and_then(|mut dec| dec.decode_page(3)) {
        Ok(page) => println!("  OK {}x{}", page.bitmap.width(), page.bitmap.height()),
        Err(err) => println!("  ERR {err}"),
    }

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
            println!(
                "segment {} {:?}: declared body exceeds stream: {}..{} of {}",
                sh.number,
                sh.segment_type,
                body_start,
                body_end,
                data.len()
            );
            break;
        }
        let body = &data[body_start..body_end];
        if matches!(
            sh.segment_type,
            SegmentType::IntermediateTextRegion
                | SegmentType::ImmediateTextRegion
                | SegmentType::ImmediateLosslessTextRegion
        ) && sh.page_association == 3
        {
            let mut bcur = Cursor::new(body);
            let hdr = TextRegionHeader::read(&mut bcur).unwrap();
            let header_len = bcur.position() as usize;
            println!(
                "page-3 text seg={} refs={:?} body={}..{} header_len={} coded_len={}",
                sh.number,
                sh.referred,
                body_start,
                body_end,
                header_len,
                body.len() - header_len
            );
            println!(
                "  w={} h={} sbhuff={} sbrefine={} log_sbstrips={} sbstrips={} sbrtemplate={} num_inst={}",
                hdr.region.width,
                hdr.region.height,
                hdr.sbhuff,
                hdr.sbrefine,
                hdr.log_sbstrips,
                1u32 << hdr.log_sbstrips,
                hdr.sbr_template,
                hdr.num_instances
            );
        }
        cur.set_position(body_end as u64);
    }
}

fn trace_tt5_refagg() {
    let data = load_stream("codeStreamTest1_TT5.jb2");
    println!("TT5 symbol dictionaries:");
    list_segments("codeStreamTest1_TT5.jb2");

    let decoded = Jbig2Decoder::new(Cursor::new(&data[..]))
        .and_then(|mut dec| dec.decode_page(1))
        .expect("decode TT5 page 1");
    let expected = match load_conformance_bmp("codeStreamTest1_TT5_TT00.bmp") {
        ReferenceImage::Mono(bm) => bm,
        ReferenceImage::Rgb(_) => panic!("TT5 oracle unexpectedly RGB"),
    };

    let mut equal = 0usize;
    let mut inverted = 0usize;
    let mut different = 0usize;
    let mut first_diff = None;
    let mut coords = Vec::new();
    for y in 0..decoded.bitmap.height() {
        for x in 0..decoded.bitmap.width() {
            let got = decoded.bitmap.get_pixel(x as i32, y as i32);
            let want = expected.get_pixel(x as i32, y as i32);
            if got == want {
                equal += 1;
            } else {
                if first_diff.is_none() {
                    first_diff = Some((x, y, got, want));
                }
                if got == (want ^ 1) {
                    inverted += 1;
                } else {
                    different += 1;
                }
                if coords.len() < 32 {
                    coords.push((x, y, got, want));
                }
            }
        }
    }
    println!(
        "TT5 page diff: equal={} inverted={} other={} first_diff={:?}",
        equal, inverted, different, first_diff
    );
    println!("TT5 first diffs: {coords:?}");
}

fn trace_annex_h_page1() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("vendor")
        .join("jbig2dec")
        .join("annex-h.jbig2");
    let data = std::fs::read(&path).expect("read annex-h.jbig2");
    println!("annex-h path={} len={}", path.display(), data.len());
    match Jbig2Decoder::new(Cursor::new(&data[..])).and_then(|mut dec| dec.decode_page(1)) {
        Ok(page) => println!("page 1 OK {}x{}", page.bitmap.width(), page.bitmap.height()),
        Err(err) => println!("page 1 ERR {err}"),
    }

    let mut cur = Cursor::new(&data[..]);
    let _fh = FileHeader::read(&mut cur).unwrap();
    let mut dicts: HashMap<u32, Vec<Bitmap>> = HashMap::new();
    let mut pats: HashMap<u32, Vec<Bitmap>> = HashMap::new();
    loop {
        let header_start = cur.position() as usize;
        let sh = match SegmentHeader::read(&mut cur) {
            Ok(s) => s,
            Err(err) => {
                println!("segment header read stopped at {header_start}: {err}");
                break;
            }
        };
        let body_start = cur.position() as usize;
        let len = sh.data_length.unwrap_or(0) as usize;
        let body_end = body_start + len;
        println!(
            "seg={} page={} type={:?} refs={:?} header={}..{} body={}..{} len={}",
            sh.number,
            sh.page_association,
            sh.segment_type,
            sh.referred,
            header_start,
            body_start,
            body_start,
            body_end,
            len
        );
        if body_end > data.len() {
            println!(
                "  declared body exceeds stream by {} bytes",
                body_end - data.len()
            );
            break;
        }
        if sh.page_association == 1
            && matches!(
                sh.segment_type,
                SegmentType::ImmediateLosslessGenericRegion
                    | SegmentType::PatternDictionary
                    | SegmentType::ImmediateLosslessHalftoneRegion
                    | SegmentType::ImmediateLosslessTextRegion
            )
        {
            println!("  page-1 candidate body_len={}", len);
        }
        let body = &data[body_start..body_end];
        match sh.segment_type {
            SegmentType::SymbolDictionary
                if sh.page_association == 0 || sh.page_association == 1 =>
            {
                let mut bcur = Cursor::new(body);
                match SymbolDictionaryHeader::read(&mut bcur) {
                    Ok(hdr) => {
                        let header_len = bcur.position() as usize;
                        let mut imports = Vec::new();
                        for ref_no in &sh.referred {
                            if let Some(prev) = dicts.get(ref_no) {
                                imports.extend(prev.iter());
                            }
                        }
                        match decode_symbol_dictionary(&hdr, &body[header_len..], &imports) {
                            Ok(decoded) => {
                                println!("  decode SD OK exported={}", decoded.exported.len());
                                dicts.insert(sh.number, decoded.exported);
                            }
                            Err(err) => println!("  decode SD ERR {err}"),
                        }
                    }
                    Err(err) => println!("  parse SD ERR {err}"),
                }
            }
            SegmentType::ImmediateLosslessTextRegion if sh.page_association == 1 => {
                let mut bcur = Cursor::new(body);
                match TextRegionHeader::read(&mut bcur) {
                    Ok(hdr) => {
                        let header_len = bcur.position() as usize;
                        println!(
                            "  text hdr header_len={} sbhuff={} sbrefine={} log_sbstrips={} fs={} ds={} dt={} rsize={} num_inst={}",
                            header_len,
                            hdr.sbhuff,
                            hdr.sbrefine,
                            hdr.log_sbstrips,
                            hdr.sbhuff_fs,
                            hdr.sbhuff_ds,
                            hdr.sbhuff_dt,
                            hdr.sbhuff_rsize,
                            hdr.num_instances
                        );
                        let mut syms = Vec::new();
                        for ref_no in &sh.referred {
                            if let Some(prev) = dicts.get(ref_no) {
                                syms.extend(prev.iter());
                            }
                        }
                        match decode_text_region(&hdr, &body[header_len..], &syms) {
                            Ok(bm) => println!("  decode text OK {}x{}", bm.width(), bm.height()),
                            Err(err) => println!("  decode text ERR {err}"),
                        }
                    }
                    Err(err) => println!("  parse text ERR {err}"),
                }
            }
            SegmentType::ImmediateLosslessGenericRegion if sh.page_association == 1 => {
                let mut bcur = Cursor::new(body);
                match GenericRegionHeader::read(&mut bcur) {
                    Ok(hdr) => {
                        let header_len = bcur.position() as usize;
                        if hdr.mmr {
                            match decode_generic_mmr(
                                &body[header_len..],
                                hdr.region.width,
                                hdr.region.height,
                            ) {
                                Ok(bm) => {
                                    println!(
                                        "  decode generic MMR OK {}x{}",
                                        bm.width(),
                                        bm.height()
                                    )
                                }
                                Err(err) => println!("  decode generic MMR ERR {err}"),
                            }
                        } else {
                            println!("  generic is arithmetic; not decoded in this tracer");
                        }
                    }
                    Err(err) => println!("  parse generic ERR {err}"),
                }
            }
            SegmentType::PatternDictionary if sh.page_association == 1 => {
                let mut bcur = Cursor::new(body);
                match PatternDictionaryHeader::read(&mut bcur) {
                    Ok(hdr) => {
                        let header_len = bcur.position() as usize;
                        match decode_pattern_dictionary(&hdr, &body[header_len..]) {
                            Ok(decoded) => {
                                println!("  decode pattern OK count={}", decoded.len());
                                pats.insert(sh.number, decoded);
                            }
                            Err(err) => println!("  decode pattern ERR {err}"),
                        }
                    }
                    Err(err) => println!("  parse pattern ERR {err}"),
                }
            }
            SegmentType::ImmediateLosslessHalftoneRegion if sh.page_association == 1 => {
                let mut bcur = Cursor::new(body);
                match HalftoneRegionHeader::read(&mut bcur) {
                    Ok(hdr) => {
                        let header_len = bcur.position() as usize;
                        let mut refs = Vec::new();
                        for ref_no in &sh.referred {
                            if let Some(prev) = pats.get(ref_no) {
                                refs.extend(prev.iter());
                            }
                        }
                        match decode_halftone_region(&hdr, &body[header_len..], &refs) {
                            Ok(bm) => {
                                println!("  decode halftone OK {}x{}", bm.width(), bm.height())
                            }
                            Err(err) => println!("  decode halftone ERR {err}"),
                        }
                    }
                    Err(err) => println!("  parse halftone ERR {err}"),
                }
            }
            _ => {}
        }
        cur.set_position(body_end as u64);
    }
}
