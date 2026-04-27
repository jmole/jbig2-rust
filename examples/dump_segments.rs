//! Print the parsed structural tree of any JBIG2 stream
//! using the validator's strict parser.

use std::env;
use std::fs;

use jbig2::validator::parse_for_dump;

fn main() {
    let path = env::args().nth(1).expect("usage: dump_segments <file.jb2>");
    let bytes = fs::read(&path).expect("read");
    let tree = parse_for_dump(&bytes);
    println!("file = {} ({} bytes)", path, tree.input_len);
    println!("file_header = {:?}", tree.file_header);
    println!("first_segment_offset = {}", tree.first_segment_offset);
    println!("segments ({}):", tree.segments.len());
    for (i, seg) in tree.segments.iter().enumerate() {
        println!(
            "  [{i:>3}] off=0x{:04X} hdr_len={:>2} num={:>4} type={:?} flags=0x{:02X} page={} refs={:?} dlen={:?} body={} parsed={:?}",
            seg.offset,
            seg.header_len,
            seg.header.number,
            seg.header.segment_type,
            seg.header.flags,
            seg.header.page_association,
            seg.header.referred,
            seg.header.data_length,
            seg.body.len(),
            seg.parsed
        );
    }
    println!("diagnostics ({}):", tree.diagnostics.len());
    for d in &tree.diagnostics {
        println!(
            "  {} sev={:?} off={} {}",
            d.check_id, d.severity, d.byte_offset, d.message
        );
    }
}
