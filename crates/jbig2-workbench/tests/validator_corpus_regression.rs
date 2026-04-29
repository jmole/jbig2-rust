//! End-to-end regression test over `tests/validator-corpus/`.
//!
//! For every fixture (`stream.jb2` + `expected.toml`) we re-run the validator
//! under the strict T.88 lens. Behaviour depends on the fixture's `shape`:
//!
//! * `validator-fixture` / `both` — the recorded `primary_check_id` must
//!   appear in the validator's findings (or, if it is `clean`, the validator
//!   must produce zero findings). `check_ids` is retained as historical
//!   evidence and is not hard-asserted.
//! * `decoder-fixture` — the validator must terminate without panicking. We
//!   do not enforce a particular finding set (the contract is on
//!   `[decoder.<impl>]` blocks, which `corpus-validator --strict` consumes).
//!
//! This catches accidental regressions in either the structural parser or
//! any catalog check, and also catches drift between the persisted fixtures
//! and the current validator behaviour.
//!
//! Gated behind the `validator-corpus` feature so contributors who only want
//! to ship a small change to the encoder/decoder do not need to materialise
//! the corpus.

#![cfg(feature = "validator-corpus")]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use jbig2::validator::{validate, Lens};
use jbig2_workbench::validator_corpus::{Expected, Shape};
use sha2::{Digest, Sha256};

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("validator-corpus")
}

fn collect_fixtures(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !root.exists() {
        return out;
    }
    walk(root, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.file_name() == Some(std::ffi::OsStr::new("expected.toml")) {
            out.push(path);
        }
    }
}

#[test]
fn validator_corpus_regression() {
    let root = corpus_root();
    let fixtures = collect_fixtures(&root);
    assert!(
        !fixtures.is_empty(),
        "no validator-corpus fixtures found under {}; regenerate with `cargo run --bin corpus-mint --features validator-corpus`",
        root.display()
    );

    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for expected_path in &fixtures {
        if let Some(message) = seed_anchor_drift_message(&root, expected_path) {
            failures.push(message);
        }

        let stream_path = expected_path.with_file_name("stream.jb2");
        if !stream_path.exists() {
            continue;
        }
        checked += 1;
        let bytes = match fs::read(&stream_path) {
            Ok(b) => b,
            Err(e) => {
                failures.push(format!("{}: read failed: {e}", stream_path.display()));
                continue;
            }
        };
        let expected = match Expected::read(expected_path) {
            Ok(e) => e,
            Err(e) => {
                failures.push(format!("{e}"));
                continue;
            }
        };
        let report = validate(&bytes, Lens::StrictT88);
        let actual: BTreeSet<String> = report
            .findings
            .iter()
            .map(|f| f.check_id.as_str().to_string())
            .collect();

        let rel = expected_path
            .strip_prefix(&root)
            .unwrap_or(expected_path)
            .display();

        match expected.shape {
            // decoder-fixture: contract is on `[decoder.<impl>]` blocks,
            // checked by `corpus-validator --strict`. The only requirement
            // here is that the validator terminated.
            Shape::DecoderFixture => continue,
            Shape::ValidatorFixture | Shape::Both => {}
        }

        let Some(validator) = expected.validator.as_ref() else {
            failures.push(format!(
                "{}: shape = {:?} requires a [validator] block",
                rel, expected.shape
            ));
            continue;
        };

        if validator.primary_check_id == "clean" {
            if !report.findings.is_empty() {
                failures.push(format!(
                    "{}: expected clean stream but validator produced {}",
                    rel,
                    sorted_join(&actual)
                ));
            }
            continue;
        }

        // `unknown` is a pre-baseline placeholder for fixtures whose
        // primary id has not been classified yet. We still want to make sure
        // the validator runs to completion without panicking, but we do not
        // enforce a specific CheckId.
        if validator.primary_check_id == "unknown" {
            continue;
        }

        if !actual.contains(&validator.primary_check_id) {
            failures.push(format!(
                "{}: missing primary check id {}; got {{{}}}",
                rel,
                validator.primary_check_id,
                sorted_join(&actual)
            ));
            continue;
        }
    }

    if !failures.is_empty() {
        panic!(
            "validator-corpus regression: {} of {} fixtures regressed:\n{}",
            failures.len(),
            checked,
            failures.join("\n")
        );
    }
}

/// Catch typos and catalog renames in synthetic fixtures: every
/// `primary_check_id` declared by `tools/corpus-mint` (and persisted
/// into `tests/validator-corpus/synthetic/*/expected.toml`) must
/// correspond to a real catalog check.
///
/// This is intentionally narrower than [`validator_corpus_regression`]:
/// the regression test runs the validator and compares full id sets,
/// which can fail for many reasons (parser drift, lens changes, etc.).
/// This test only checks naming consistency, so a failure here points
/// directly at a fixture or catalog renaming.
#[test]
fn synthetic_fixture_primary_check_ids_match_catalog() {
    let root = corpus_root().join("synthetic");
    let fixtures = collect_fixtures(&root);
    assert!(
        !fixtures.is_empty(),
        "no synthetic fixtures found under {}; regenerate with `cargo run --bin corpus-mint --features validator-corpus`",
        root.display()
    );

    let known: BTreeSet<String> = jbig2::validator::catalog::checks()
        .into_iter()
        .map(|check| check.id().as_str().to_string())
        .collect();

    let mut failures: Vec<String> = Vec::new();
    for path in &fixtures {
        let expected = match Expected::read(path) {
            Ok(e) => e,
            Err(e) => {
                failures.push(format!("{e}"));
                continue;
            }
        };
        let Some(validator) = expected.validator.as_ref() else {
            continue;
        };
        if validator.primary_check_id.is_empty()
            || validator.primary_check_id == "clean"
            || validator.primary_check_id == "unknown"
        {
            continue;
        }
        if !known.contains(&validator.primary_check_id) {
            let rel = path
                .strip_prefix(corpus_root())
                .unwrap_or(path)
                .display()
                .to_string();
            failures.push(format!(
                "{rel}: primary_check_id {:?} is not in the validator catalog",
                validator.primary_check_id,
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "synthetic fixture primary_check_id consistency: {} of {} fixtures reference unknown check ids:\n{}\n\
             If you renamed a catalog check, regenerate the fixtures with \
             `cargo run --bin corpus-mint --features validator-corpus`. \
             If you added a fixture, ensure tools/corpus-mint/main.rs uses an existing CheckId.",
            failures.len(),
            fixtures.len(),
            failures.join("\n"),
        );
    }
}

/// Every directory under `tests/validator-corpus/bugzilla/harvested/` must
/// contain a `stream.jb2`. This assertion catches the next "I added a
/// `meta.toml` but forgot the stream" mistake at PR time rather than via
/// silent zero-coverage drift in the regression set.
///
/// The companion `tracked/` directory is intentionally not asserted here:
/// fixtures under `tracked/` document upstream bugs whose attachments are
/// PDFs, zips, or other containers we deliberately do not extract — see
/// `tests/validator-corpus/bugzilla/INDEX.md`.
#[test]
fn bugzilla_harvested_fixtures_have_streams() {
    let harvested = corpus_root().join("bugzilla").join("harvested");
    if !harvested.exists() {
        return;
    }
    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for entry in fs::read_dir(&harvested)
        .expect("read harvested dir")
        .flatten()
    {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        checked += 1;
        let stream = dir.join("stream.jb2");
        let expected = dir.join("expected.toml");
        if !stream.exists() {
            failures.push(format!("{}: missing stream.jb2", dir.display()));
        }
        if !expected.exists() {
            failures.push(format!("{}: missing expected.toml", dir.display()));
        }
    }
    assert!(
        checked > 0,
        "no harvested bugzilla fixtures under {}",
        harvested.display()
    );
    if !failures.is_empty() {
        panic!(
            "bugzilla/harvested coverage gap ({} fixture(s)):\n{}\n\
             Either move the directory to bugzilla/tracked/ or add the missing stream.jb2.",
            failures.len(),
            failures.join("\n"),
        );
    }
}

fn sorted_join(set: &BTreeSet<String>) -> String {
    set.iter().cloned().collect::<Vec<_>>().join(", ")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SeedAnchor {
    seed_path: String,
    seed_sha256: String,
}

fn seed_anchor_drift_message(root: &Path, expected_path: &Path) -> Option<String> {
    let rel = expected_path
        .strip_prefix(root)
        .unwrap_or(expected_path)
        .display()
        .to_string();
    let meta_path = expected_path.with_file_name("meta.toml");
    if !meta_path.exists() {
        return None;
    }
    let meta_text = match fs::read_to_string(&meta_path) {
        Ok(text) => text,
        Err(err) => {
            return Some(format!(
                "{}: failed to read {}: {err}",
                rel,
                meta_path.display()
            ));
        }
    };
    let anchor = match parse_seed_anchor(&meta_text) {
        Ok(anchor) => anchor,
        Err(err) => return Some(format!("{rel}: {err}")),
    };
    let Some(anchor) = anchor else {
        return None;
    };

    let seed_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&anchor.seed_path);
    let current_bytes = match fs::read(&seed_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Some(format!(
                "{}: seed file {} cannot be read: {err}",
                rel,
                seed_path.display()
            ));
        }
    };
    let mut sha = Sha256::new();
    sha.update(&current_bytes);
    let current_sha = format!("{:x}", sha.finalize());
    if current_sha == anchor.seed_sha256 {
        return None;
    }

    Some(format!(
        "{}: {} has changed since this fixture was minted (was {}..., now {}...). Either revert the vendor bump or regenerate with `cargo run --bin corpus-mint --features validator-corpus`.",
        rel,
        anchor.seed_path,
        short_hash(&anchor.seed_sha256),
        short_hash(&current_sha),
    ))
}

fn parse_seed_anchor(meta_text: &str) -> Result<Option<SeedAnchor>, String> {
    let value: toml::Value =
        toml::from_str(meta_text).map_err(|err| format!("invalid meta.toml: {err}"))?;
    let Some(table) = value.as_table() else {
        return Err("meta.toml is not a table".to_string());
    };

    if let (Some(base_path), Some(base_sha256)) = (
        table.get("base_path").and_then(|v| v.as_str()),
        table.get("base_sha256").and_then(|v| v.as_str()),
    ) {
        return Ok(Some(SeedAnchor {
            seed_path: base_path.to_string(),
            seed_sha256: base_sha256.to_string(),
        }));
    }

    if let (Some(seed_path), Some(seed_sha256)) = (
        table.get("seed_path").and_then(|v| v.as_str()),
        table.get("seed_sha256").and_then(|v| v.as_str()),
    ) {
        return Ok(Some(SeedAnchor {
            seed_path: seed_path.to_string(),
            seed_sha256: seed_sha256.to_string(),
        }));
    }

    if let Some(seed_sha256) = table.get("seed_sha256").and_then(|v| v.as_str()) {
        let seed_path = match table.get("seed_name").and_then(|v| v.as_str()) {
            Some("annex-h") => "vendor/jbig2dec/annex-h.jbig2",
            Some(other) => {
                return Err(format!(
                    "meta.toml has seed_sha256 but unknown seed_name {:?}; add a seed_path mapping for drift checks",
                    other
                ));
            }
            None => {
                return Err(
                    "meta.toml has seed_sha256 but no seed_name/seed_path to locate the seed"
                        .to_string(),
                );
            }
        };
        return Ok(Some(SeedAnchor {
            seed_path: seed_path.to_string(),
            seed_sha256: seed_sha256.to_string(),
        }));
    }

    Ok(None)
}

fn short_hash(hash: &str) -> &str {
    hash.get(..8).unwrap_or(hash)
}

/// Sanity check: the canonical Annex H stream itself must validate clean.
#[test]
fn annex_h_validates_clean() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("vendor")
        .join("jbig2dec")
        .join("annex-h.jbig2");
    if !path.exists() {
        eprintln!(
            "skipping annex_h_validates_clean: {} not present",
            path.display()
        );
        return;
    }
    let bytes = fs::read(&path).expect("read annex-h.jbig2");
    let report = validate(&bytes, Lens::StrictT88);
    assert!(
        report.findings.is_empty(),
        "Annex H reference stream produced findings: {:#?}",
        report.findings
    );
}

/// Whole-corpus clean-pass: every TT* stream shipped in
/// `vendor/T-REC-T.88-201808/.../JBIG2_ConformanceData-A20180829/` must
/// validate without an `Error`-severity finding under the ITU interop lens
/// (these streams are by definition what the ITU codec accepts).
///
/// We deliberately do not assert clean-pass under `Lens::StrictT88` here:
/// some conformance streams omit explicit symbol-dictionary references that
/// the strict reading of clause 7.3.2 requires, and we want strict-T88 to
/// continue reporting that divergence.
#[test]
fn itu_t88_conformance_corpus_validates_clean() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("vendor")
        .join("T-REC-T.88-201808")
        .join("Software")
        .join("JBIG2_ConformanceData-A20180829");
    if !dir.exists() {
        eprintln!("skipping: {} not present", dir.display());
        return;
    }
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read conformance dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("jb2"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no .jb2 files in {}", dir.display());

    let mut failures: Vec<String> = Vec::new();
    for path in &paths {
        let bytes = fs::read(path).expect("read conformance jb2");
        let report = validate(&bytes, Lens::ItuT88Interop);
        let bad: Vec<_> = report
            .findings
            .iter()
            .filter(|f| matches!(f.severity, jbig2::validator::Severity::Error))
            .collect();
        if !bad.is_empty() {
            failures.push(format!(
                "{}: {} error finding(s); first = {} {}",
                path.file_name().unwrap().to_string_lossy(),
                bad.len(),
                bad[0].check_id.as_str(),
                bad[0].message,
            ));
        }
    }
    if !failures.is_empty() {
        panic!(
            "ITU-T T.88 conformance corpus regressed:\n{}",
            failures.join("\n")
        );
    }
}

#[test]
fn parse_seed_anchor_supports_bitflip_meta_shape() {
    let meta = r#"
base_path = "vendor/jbig2dec/annex-h.jbig2"
base_sha256 = "abc123"
byte_offset = 10
"#;
    let anchor = parse_seed_anchor(meta).expect("parse").expect("anchor");
    assert_eq!(
        anchor,
        SeedAnchor {
            seed_path: "vendor/jbig2dec/annex-h.jbig2".to_string(),
            seed_sha256: "abc123".to_string(),
        }
    );
}

#[test]
fn parse_seed_anchor_supports_mutator_meta_shape() {
    let meta = r#"
seed_name = "annex-h"
seed_sha256 = "def456"
schedule = "bit-flip"
"#;
    let anchor = parse_seed_anchor(meta).expect("parse").expect("anchor");
    assert_eq!(
        anchor,
        SeedAnchor {
            seed_path: "vendor/jbig2dec/annex-h.jbig2".to_string(),
            seed_sha256: "def456".to_string(),
        }
    );
}

#[test]
fn parse_seed_anchor_ignores_non_seed_meta() {
    let meta = r#"
description = "synthetic fixture"
primary_check_id = "T88-7.4.4-001"
"#;
    let anchor = parse_seed_anchor(meta).expect("parse");
    assert!(anchor.is_none());
}
