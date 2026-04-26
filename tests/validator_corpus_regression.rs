//! End-to-end regression test over `tests/validator-corpus/`.
//!
//! For every fixture (`stream.jb2` + `expected.toml`) we re-run the validator
//! under the strict T.88 lens and assert that:
//!
//! * the recorded `primary_check_id` is one of the findings (or the fixture
//!   is marked `clean` and the validator returns no findings), and
//! * every id in the recorded `check_ids` set is still produced.
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

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("validator-corpus")
}

#[derive(Debug)]
struct Expected {
    primary: String,
    check_ids: BTreeSet<String>,
}

fn parse_expected(path: &Path) -> Expected {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut primary = String::new();
    let mut check_ids: BTreeSet<String> = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("primary_check_id") {
            let value = rest
                .trim_start_matches(|c: char| c.is_whitespace() || c == '=')
                .trim();
            primary = strip_quotes(value).to_string();
        } else if let Some(rest) = line.strip_prefix("check_ids") {
            let value = rest
                .trim_start_matches(|c: char| c.is_whitespace() || c == '=')
                .trim();
            let inner = value.trim_start_matches('[').trim_end_matches(']').trim();
            if !inner.is_empty() {
                for item in inner.split(',') {
                    let item = item.trim();
                    if item.is_empty() {
                        continue;
                    }
                    check_ids.insert(strip_quotes(item).to_string());
                }
            }
        }
    }
    Expected { primary, check_ids }
}

fn strip_quotes(value: &str) -> &str {
    value.trim_matches('"').trim_matches('\'')
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
        let expected = parse_expected(expected_path);
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

        if expected.primary == "clean" {
            if !report.findings.is_empty() {
                failures.push(format!(
                    "{}: expected clean stream but validator produced {}",
                    rel,
                    sorted_join(&actual)
                ));
            }
            continue;
        }

        // `unknown` is a placeholder used for harvested third-party fixtures
        // (e.g. Ghostscript Bugzilla) where the spec-violation has not been
        // hand-classified yet. We still want to make sure the validator runs
        // to completion without panicking, but we do not enforce a specific
        // CheckId.
        if expected.primary == "unknown" {
            continue;
        }

        if !actual.contains(&expected.primary) {
            failures.push(format!(
                "{}: missing primary check id {}; got {{{}}}",
                rel,
                expected.primary,
                sorted_join(&actual)
            ));
            continue;
        }

        let missing: Vec<&String> = expected
            .check_ids
            .iter()
            .filter(|id| !actual.contains(*id))
            .collect();
        if !missing.is_empty() {
            failures.push(format!(
                "{}: missing recorded check ids {:?}; got {{{}}}",
                rel,
                missing,
                sorted_join(&actual)
            ));
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
        let expected = parse_expected(path);
        if expected.primary.is_empty()
            || expected.primary == "clean"
            || expected.primary == "unknown"
        {
            continue;
        }
        if !known.contains(&expected.primary) {
            let rel = path
                .strip_prefix(corpus_root())
                .unwrap_or(path)
                .display()
                .to_string();
            failures.push(format!(
                "{rel}: primary_check_id {:?} is not in the validator catalog",
                expected.primary,
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

fn sorted_join(set: &BTreeSet<String>) -> String {
    set.iter().cloned().collect::<Vec<_>>().join(", ")
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
