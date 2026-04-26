mod annex_h_bitflip;
mod mutate;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use jbig2::validator::{validate, Lens};

fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    mint_synthetic(&root)?;
    annex_h_bitflip::run(&root)?;
    mutate::run(&root)?;
    Ok(())
}

fn mint_synthetic(root: &Path) -> Result<()> {
    let out = root.join("tests/validator-corpus/synthetic");
    fs::create_dir_all(&out)?;
    for fixture in synthetic_fixtures() {
        let dir = out.join(fixture.name);
        fs::create_dir_all(&dir)?;
        let stream = (fixture.build)();
        write_fixture(&dir, fixture.check_id, fixture.description, &stream)?;
    }
    Ok(())
}

struct Fixture {
    name: &'static str,
    check_id: &'static str,
    description: &'static str,
    build: fn() -> Vec<u8>,
}

fn synthetic_fixtures() -> Vec<Fixture> {
    vec![
        Fixture { name: "7.2.3-reserved-ref-count", check_id: "T88-7.2.3-001", description: "reserved referred-count encoding", build: reserved_ref_count },
        Fixture { name: "7.4.1-zero-region", check_id: "T88-7.4.1-001", description: "zero-width generic region", build: zero_region },
        Fixture { name: "7.4.2-symbol-reserved-flags", check_id: "T88-7.4.2-001", description: "symbol dictionary reserved flags", build: symbol_reserved_flags },
        Fixture { name: "7.4.2-sdrefagg-forward-reference", check_id: "T88-7.4.2-009", description: "SDREFAGG dictionary refers forward", build: sdrefagg_forward_ref },
        Fixture { name: "7.4.3-sbnuminsts-zero", check_id: "T88-7.4.3-002", description: "text region with zero instances", build: zero_text_instances },
        Fixture { name: "7.4.4-pattern-zero-size", check_id: "T88-7.4.4-001", description: "pattern dictionary with zero pattern width", build: pattern_zero_size },
        Fixture { name: "7.4.6-tpgdon-template3", check_id: "T88-7.4.6-001", description: "generic region TPGDON set on template 3", build: generic_tpgdon_template3 },
        Fixture { name: "7.4.7-refinement-reserved-flags", check_id: "T88-7.4.7-001", description: "refinement region reserved flag bits", build: refinement_reserved_flags },
        Fixture { name: "7.4.12-empty-profiles", check_id: "T88-7.4.12-001", description: "empty profiles body", build: empty_profiles },
        Fixture { name: "7.4.13-empty-table", check_id: "T88-7.4.13-001", description: "empty tables body", build: empty_tables },
    ]
}

pub(crate) fn write_fixture(
    dir: &Path,
    primary_check_id: &str,
    description: &str,
    stream: &[u8],
) -> Result<()> {
    fs::write(dir.join("stream.jb2"), stream)?;
    fs::write(
        dir.join("meta.toml"),
        format!("description = {:?}\nprimary_check_id = {:?}\n", description, primary_check_id),
    )?;
    let report = validate(stream, Lens::StrictT88);
    let ids = report
        .findings
        .iter()
        .map(|finding| format!("{:?}", finding.check_id.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        dir.join("expected.toml"),
        format!("primary_check_id = {:?}\ncheck_ids = [{}]\n", primary_check_id, ids),
    )
    .with_context(|| format!("write expected metadata for {}", dir.display()))?;
    Ok(())
}

fn base_file() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&jbig2::JBIG2_MAGIC);
    out.push(0x03);
    out
}

fn segment(number: u32, flags: u8, ref_count_byte: u8, referred: &[u32], page: u8, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&number.to_be_bytes());
    out.push(flags);
    out.push(ref_count_byte);
    for &r in referred {
        out.push(r as u8);
    }
    out.push(page);
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(body);
    out
}

fn eof(number: u32) -> Vec<u8> {
    segment(number, 0x33, 0x00, &[], 0, &[])
}

fn region_body(width: u32, height: u32, region_flags: u8, extra: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&width.to_be_bytes());
    body.extend_from_slice(&height.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.push(region_flags);
    body.extend_from_slice(extra);
    body
}

fn reserved_ref_count() -> Vec<u8> {
    let mut out = base_file();
    out.extend(segment(1, 0x33, 0xA0, &[], 0, &[]));
    out
}

fn zero_region() -> Vec<u8> {
    let mut out = base_file();
    out.extend(segment(1, 0x26, 0x00, &[], 1, &region_body(0, 1, 0, &[0])));
    out.extend(eof(2));
    out
}

fn symbol_reserved_flags() -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0xE000u16.to_be_bytes());
    body.extend_from_slice(&1u32.to_be_bytes());
    body.extend_from_slice(&1u32.to_be_bytes());
    let mut out = base_file();
    out.extend(segment(1, 0x00, 0x00, &[], 0, &body));
    out.extend(eof(2));
    out
}

fn sdrefagg_forward_ref() -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0x0002u16.to_be_bytes());
    body.extend_from_slice(&1u32.to_be_bytes());
    body.extend_from_slice(&1u32.to_be_bytes());
    let mut out = base_file();
    out.extend(segment(1, 0x00, 0x21, &[2], 0, &body));
    out.extend(segment(2, 0x00, 0x00, &[], 0, &body));
    out.extend(eof(3));
    out
}

fn zero_text_instances() -> Vec<u8> {
    let mut out = base_file();
    out.extend(segment(1, 0x00, 0x00, &[], 0, &symbol_header(1, 1)));
    let mut body = region_body(1, 1, 0, &[]);
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    out.extend(segment(2, 0x07, 0x21, &[1], 1, &body));
    out.extend(eof(3));
    out
}

fn pattern_zero_size() -> Vec<u8> {
    let mut body = vec![0, 0, 1];
    body.extend_from_slice(&0u32.to_be_bytes());
    let mut out = base_file();
    out.extend(segment(1, 0x10, 0x00, &[], 0, &body));
    out.extend(eof(2));
    out
}

fn generic_tpgdon_template3() -> Vec<u8> {
    let mut body = region_body(1, 1, 0, &[0x0E, 0, 0]);
    body.extend_from_slice(&[0xFF]);
    let mut out = base_file();
    out.extend(segment(1, 0x26, 0x00, &[], 1, &body));
    out.extend(eof(2));
    out
}

fn refinement_reserved_flags() -> Vec<u8> {
    let mut body = region_body(1, 1, 0, &[0xFC, 0, 0, 0, 0]);
    body.extend_from_slice(&[0xFF]);
    let mut out = base_file();
    out.extend(segment(1, 0x2A, 0x00, &[], 1, &body));
    out.extend(eof(2));
    out
}

fn empty_profiles() -> Vec<u8> {
    let mut out = base_file();
    out.extend(segment(1, 0x34, 0x00, &[], 0, &[]));
    out.extend(eof(2));
    out
}

fn empty_tables() -> Vec<u8> {
    let mut out = base_file();
    out.extend(segment(1, 0x35, 0x00, &[], 0, &[]));
    out.extend(eof(2));
    out
}

fn symbol_header(num_ex: u32, num_new: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
    body.extend_from_slice(&num_ex.to_be_bytes());
    body.extend_from_slice(&num_new.to_be_bytes());
    body.extend_from_slice(&[0xFF]);
    body
}
