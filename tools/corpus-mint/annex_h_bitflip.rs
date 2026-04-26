//! Annex H bit-flip sweep (T.88 validator stress harness).
//!
//! Performs an exhaustive single-bit sweep over `vendor/jbig2dec/annex-h.jbig2`
//! (860 bytes -> 6,880 mutated cases). Every mutation is fed through
//! `jbig2::validator::validate(..., Lens::StrictT88)`. Results are bucketed by
//! `(highest_severity, primary_check_id, segment_type)` and at most three
//! representative fixtures per non-clean bucket are persisted under
//! `tests/validator-corpus/annex-h-bitflip/`.
//!
//! No C decoder is invoked from this binary. Downstream sanitizer runs happen
//! later, in `tools/corpus-validator`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use jbig2::validator::corpus::Expected;
use jbig2::validator::{validate, Lens, Report, Severity};
use sha2::{Digest, Sha256};

const BUCKETING_RULE_VERSION: u32 = 1;
const FIXTURES_PER_BUCKET: usize = 3;
const BASE_PATH: &str = "vendor/jbig2dec/annex-h.jbig2";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct BucketKey {
    severity: &'static str,
    check_id: String,
    segment_type: String,
}

#[derive(Clone, Debug)]
struct CaseResult {
    byte_offset: usize,
    bit_index: u8,
    findings: Vec<String>,
}

#[derive(Default)]
struct Bucket {
    representatives: Vec<CaseResult>,
    total: usize,
}

pub fn run(root: &Path) -> Result<()> {
    let base_path = root.join(BASE_PATH);
    let base = fs::read(&base_path)
        .with_context(|| format!("failed to read base file {}", base_path.display()))?;

    let mut sha = Sha256::new();
    sha.update(&base);
    let base_sha = format!("{:x}", sha.finalize());

    let mut buckets: BTreeMap<BucketKey, Bucket> = BTreeMap::new();
    let mut clean_count = 0usize;

    for byte_offset in 0..base.len() {
        for bit_index in 0..8u8 {
            let mut mutated = base.clone();
            mutated[byte_offset] ^= 1 << bit_index;
            let report = validate(&mutated, Lens::StrictT88);
            assert_validator_invariants(&report);
            let (key, primary_finding_strs) = bucket_key(byte_offset, bit_index, &report);
            if let Some(key) = key {
                let entry = buckets.entry(key).or_default();
                entry.total += 1;
                if entry.representatives.len() < FIXTURES_PER_BUCKET {
                    entry.representatives.push(CaseResult {
                        byte_offset,
                        bit_index,
                        findings: primary_finding_strs,
                    });
                }
            } else {
                clean_count += 1;
            }
        }
    }

    let out_dir = root.join("tests/validator-corpus/annex-h-bitflip");
    if out_dir.exists() {
        fs::remove_dir_all(&out_dir).ok();
    }
    fs::create_dir_all(&out_dir)?;

    let mut index = String::new();
    index.push_str("# Annex H bit-flip sweep\n\n");
    index.push_str(&format!(
        "Base file: `{}` (sha256 `{}`, {} bytes)\n",
        BASE_PATH,
        base_sha,
        base.len()
    ));
    index.push_str(&format!(
        "Bucketing rule version: {BUCKETING_RULE_VERSION}\n"
    ));
    index.push_str(&format!(
        "Total cases: {} ({} clean, {} bucketed)\n\n",
        base.len() * 8,
        clean_count,
        buckets.values().map(|b| b.total).sum::<usize>()
    ));
    index.push_str("| Bucket | Severity | CheckId | SegmentType | Total cases | Persisted |\n");
    index.push_str("|---|---|---|---|---|---|\n");

    for (key, bucket) in &buckets {
        let bucket_dir_name = bucket_dirname(key);
        let bucket_dir = out_dir.join(&bucket_dir_name);
        fs::create_dir_all(&bucket_dir)?;
        for representative in &bucket.representatives {
            let case_dir = bucket_dir.join(format!(
                "byte-{:04}-bit-{}",
                representative.byte_offset, representative.bit_index
            ));
            fs::create_dir_all(&case_dir)?;
            let mut mutated = base.clone();
            mutated[representative.byte_offset] ^= 1 << representative.bit_index;
            fs::write(case_dir.join("stream.jb2"), &mutated)?;
            let meta = format!(
                "base_path = {:?}\nbase_sha256 = {:?}\nbyte_offset = {}\nbit_index = {}\nbucket = {:?}\nbucketing_rule_version = {}\n",
                BASE_PATH,
                base_sha,
                representative.byte_offset,
                representative.bit_index,
                bucket_dir_name,
                BUCKETING_RULE_VERSION,
            );
            fs::write(case_dir.join("meta.toml"), meta)?;
            Expected::validator(key.check_id.clone(), representative.findings.clone())
                .write(&case_dir.join("expected.toml"))
                .with_context(|| format!("write expected metadata for {}", case_dir.display()))?;
        }
        index.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} |\n",
            bucket_dir_name,
            key.severity,
            key.check_id,
            key.segment_type,
            bucket.total,
            bucket.representatives.len(),
        ));
    }

    fs::write(out_dir.join("INDEX.md"), index)
        .with_context(|| "failed to write annex H bit-flip INDEX.md")?;

    if buckets.is_empty() {
        return Err(anyhow!(
            "annex H bit-flip sweep produced no non-clean buckets; validator never fires on the base stream"
        ));
    }

    Ok(())
}

fn assert_validator_invariants(report: &Report) {
    assert!(
        report.is_sorted(),
        "validator report findings are unsorted: {:?}",
        report
            .findings
            .iter()
            .map(|f| f.byte_offset)
            .collect::<Vec<_>>()
    );
}

fn bucket_key(
    _byte_offset: usize,
    _bit_index: u8,
    report: &Report,
) -> (Option<BucketKey>, Vec<String>) {
    let primary = report.findings.iter().min_by_key(|finding| {
        let severity_rank = match finding.severity {
            Severity::Error => 0,
            Severity::Warning => 1,
            Severity::Info => 2,
        };
        (
            severity_rank,
            finding.byte_offset,
            finding.check_id.as_str(),
        )
    });
    let Some(finding) = primary else {
        return (None, Vec::new());
    };
    let severity = match finding.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };
    let segment_type = finding
        .segment_type
        .map(str::to_string)
        .unwrap_or_else(|| "none".to_string());
    let check_id = finding.check_id.as_str().to_string();
    let ids = report
        .findings
        .iter()
        .map(|f| f.check_id.as_str().to_string())
        .collect();
    (
        Some(BucketKey {
            severity,
            check_id,
            segment_type,
        }),
        ids,
    )
}

fn bucket_dirname(key: &BucketKey) -> String {
    format!(
        "bucket-{}-{}-{}",
        key.severity,
        key.check_id.replace('.', "_"),
        key.segment_type
    )
}
