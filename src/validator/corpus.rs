//! Typed schema for `tests/validator-corpus/**/expected.toml`.
//!
//! Loaded by [`tests/validator_corpus_regression.rs`], by `tools/corpus-mint`,
//! and by `tools/corpus-validator`. Concentrating the schema here makes the
//! contract a single Rust struct: every consumer reads and writes through the
//! same `Expected` value, and `serde(deny_unknown_fields)` catches typos in
//! fixture authoring at PR time rather than as silent zero-coverage drift.
//!
//! ## Shape
//!
//! Fixtures are tagged with a `shape` discriminator that decides which blocks
//! must be present:
//!
//! - `validator-fixture` — the fixture asserts validator behaviour. The
//!   `[validator]` block is required and `[decoder.*]` blocks are absent.
//!   This is what `tools/corpus-mint` produces for `synthetic/`,
//!   `mutated/`, and `annex-h-bitflip/`.
//! - `decoder-fixture` — the fixture asserts decoder behaviour over a hostile
//!   input. The `[decoder.<impl>]` blocks are the contract; no `[validator]`
//!   block is required (validation runs in-process either way and the
//!   regression test only asserts the validator does not panic).
//! - `both` — for streams that simultaneously want validator-finding
//!   contract enforcement and decoder-verdict contract enforcement.
//!
//! ## Decoder verdicts
//!
//! The `Verdict` enum is a small classification of what each implementation
//! is expected to do on a hostile input. It is deliberately coarser than the
//! raw `SandboxOutcome` because two different kinds of crash (SIGABRT vs
//! SIGSEGV) carry the same regression meaning at this layer — see
//! `docs/05-external-decoder-taxonomy.md` for the rationale.

#![cfg(feature = "validator-corpus")]

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Shape discriminator. Decides which blocks must be present.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Shape {
    /// Asserts validator behaviour only. `[validator]` is required.
    ValidatorFixture,
    /// Asserts decoder behaviour. `[decoder.<impl>]` blocks are required.
    DecoderFixture,
    /// Asserts both. Both blocks are required.
    Both,
}

/// One implementation's expected behaviour on a fixture.
///
/// `verdict` is the contract; the rest are informational columns the harness
/// preserves alongside the assertion (e.g. so a reviewer reading
/// `expected.toml` can see when the verdict was last baselined and against
/// which upstream commit).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecoderExpectation {
    /// What the implementation is expected to do.
    pub verdict: Verdict,
    /// Behaviours this fixture must NOT exhibit. Optional. Common entries:
    /// `panic`, `oom`, `infinite-loop`. Empty when omitted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not: Vec<String>,
    /// Free-form notes the human reviewer left when baselining.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Bugzilla / upstream bug status when the verdict was baselined. Hint,
    /// not a contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_status_at_baseline: Option<String>,
    /// One-line signature the harness recorded when the verdict was
    /// baselined (e.g. exit code + first line of stderr). Used by
    /// `corpus-validator --strict` only as supporting evidence in mismatch
    /// reports; an exact-match contract on this field would be too brittle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_signature: Option<String>,
}

/// Cross-implementation classification of a decoder run.
///
/// Two different crash signals are intentionally folded into `Crash`: the
/// regression-meaning of "the decoder died abnormally" is the same whether
/// the host kernel reports SIGSEGV or a sanitizer hit. The harness will
/// surface the raw signal in mismatch reports for evidence; the verdict is
/// what the contract enforces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Verdict {
    /// Decoder exited successfully and produced output.
    Ok,
    /// Decoder returned a typed error / non-zero exit code without crashing.
    /// This is the desired behaviour for hostile input.
    RejectErr,
    /// Decoder exited 0 but should have rejected. Discouraged outcome.
    RejectOk,
    /// Decoder aborted, segfaulted, hit a sanitizer, or otherwise died.
    Crash,
    /// Decoder hit the wall-clock or CPU budget.
    Timeout,
    /// Verdict has not been baselined yet. The strict regression mode treats
    /// `Unknown` as "do not score", so a fixture committed with `Unknown`
    /// runs in informational mode until a maintainer baselines it.
    Unknown,
}

impl Verdict {
    /// Stable string representation, matching the TOML spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "Ok",
            Self::RejectErr => "RejectErr",
            Self::RejectOk => "RejectOk",
            Self::Crash => "Crash",
            Self::Timeout => "Timeout",
            Self::Unknown => "Unknown",
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `[validator]` block. Required for `validator-fixture` and `both`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatorExpectation {
    /// `CheckId` the fixture is targeting, or `clean` if the stream must
    /// produce zero findings.
    pub primary_check_id: String,
    /// Every `CheckId` the validator was producing when the fixture was
    /// minted. Recorded as evidence; `corpus-validator --strict` checks that
    /// `primary_check_id` is still present, not that this set is unchanged
    /// — see `docs/01-corpus-drift-guards.md` for the rationale.
    #[serde(default)]
    pub check_ids: Vec<String>,
}

/// Top-level `expected.toml` document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    /// Discriminator that selects which blocks must be present.
    pub shape: Shape,
    /// `[validator]` block. Required for `validator-fixture`/`both`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validator: Option<ValidatorExpectation>,
    /// `[decoder.<impl>]` blocks keyed by implementation name (`rust`,
    /// `jbig2dec`, `itu_t88`, ...). Required for `decoder-fixture`/`both`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub decoder: BTreeMap<String, DecoderExpectation>,
}

impl Expected {
    /// Read and deserialize `expected.toml` at `path`.
    pub fn read(path: &Path) -> Result<Self, ExpectedError> {
        let text = fs::read_to_string(path).map_err(|err| ExpectedError::Io {
            path: path.display().to_string(),
            err: err.to_string(),
        })?;
        Self::from_str(&text).map_err(|err| ExpectedError::Parse {
            path: path.display().to_string(),
            err: err.to_string(),
        })
    }

    /// Deserialize from a TOML string.
    pub fn from_str(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// Serialize and write to `path`. Pretty-prints in stable section order.
    pub fn write(&self, path: &Path) -> Result<(), ExpectedError> {
        let body = self
            .to_toml_string()
            .map_err(|err| ExpectedError::Serialize {
                path: path.display().to_string(),
                err: err.to_string(),
            })?;
        fs::write(path, body).map_err(|err| ExpectedError::Io {
            path: path.display().to_string(),
            err: err.to_string(),
        })
    }

    /// Render to a stable TOML string.
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Convenience constructor for `validator-fixture` shape.
    pub fn validator(primary_check_id: impl Into<String>, check_ids: Vec<String>) -> Self {
        Self {
            shape: Shape::ValidatorFixture,
            validator: Some(ValidatorExpectation {
                primary_check_id: primary_check_id.into(),
                check_ids,
            }),
            decoder: BTreeMap::new(),
        }
    }
}

/// Errors produced while reading or writing `expected.toml`.
#[derive(Debug)]
pub enum ExpectedError {
    /// `fs::read_to_string` / `fs::write` failed.
    Io {
        /// Path the harness was reading or writing.
        path: String,
        /// Underlying I/O error message.
        err: String,
    },
    /// `toml::from_str` failed.
    Parse {
        /// Path the harness was parsing.
        path: String,
        /// Underlying TOML parse error.
        err: String,
    },
    /// `toml::to_string_pretty` failed.
    Serialize {
        /// Path the harness was serializing.
        path: String,
        /// Underlying TOML serialization error.
        err: String,
    },
}

impl fmt::Display for ExpectedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, err } => write!(f, "{path}: io error: {err}"),
            Self::Parse { path, err } => write!(f, "{path}: parse error: {err}"),
            Self::Serialize { path, err } => write!(f, "{path}: serialize error: {err}"),
        }
    }
}

impl std::error::Error for ExpectedError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_validator_fixture() {
        let expected = Expected::validator(
            "T88-7.4.4-001",
            vec!["T88-7.4.4-001".into(), "T88-PARSE-001".into()],
        );
        let text = expected.to_toml_string().unwrap();
        let parsed = Expected::from_str(&text).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.shape, Shape::ValidatorFixture);
        assert_eq!(
            parsed.validator.as_ref().unwrap().primary_check_id,
            "T88-7.4.4-001"
        );
    }

    #[test]
    fn roundtrip_decoder_fixture() {
        let mut decoder = BTreeMap::new();
        decoder.insert(
            "rust".into(),
            DecoderExpectation {
                verdict: Verdict::RejectErr,
                not: vec!["panic".into(), "oom".into()],
                note: None,
                upstream_status_at_baseline: None,
                baseline_signature: None,
            },
        );
        decoder.insert(
            "jbig2dec".into(),
            DecoderExpectation {
                verdict: Verdict::RejectErr,
                not: vec![],
                note: None,
                upstream_status_at_baseline: Some("RESOLVED FIXED".into()),
                baseline_signature: Some("exit=1; jbig2dec: out of bounds".into()),
            },
        );
        let expected = Expected {
            shape: Shape::DecoderFixture,
            validator: None,
            decoder,
        };
        let text = expected.to_toml_string().unwrap();
        let parsed = Expected::from_str(&text).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn rejects_unknown_top_level_key() {
        let text = r#"
shape = "validator-fixture"
unknown_field = 1

[validator]
primary_check_id = "clean"
"#;
        let err = Expected::from_str(text).unwrap_err();
        assert!(
            err.to_string().contains("unknown_field"),
            "deny_unknown_fields not active: {err}"
        );
    }

    #[test]
    fn verdict_renames_to_pascal_case() {
        let body = r#"
shape = "decoder-fixture"

[decoder.rust]
verdict = "RejectErr"
"#;
        let parsed = Expected::from_str(body).unwrap();
        assert_eq!(parsed.decoder["rust"].verdict, Verdict::RejectErr);
    }
}
