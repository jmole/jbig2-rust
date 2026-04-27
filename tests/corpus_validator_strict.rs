//! Tiny shim that makes `cargo test --features validator-corpus` exercise
//! the `corpus-validator --strict` regression path on a small filtered
//! subset of the corpus.
//!
//! The full strict run lives in CI; this test only sanity-checks that
//! a contributor running `cargo test --features validator-corpus` locally
//! catches obvious decoder regressions on a few hand-picked fixtures
//! without paying the cost of the full corpus.
//!
//! Cargo provides `CARGO_BIN_EXE_<name>` env vars at test compile time
//! pointing at the freshly built binaries in the integration-test target
//! directory, so we can avoid hand-rolling `cargo build` and avoid the
//! "did you remember to build the release binary?" footgun.
//!
//! `corpus-validator --strict` only enforces expectations for impls it
//! has binaries for, so this shim runs without `--with-c-decoders`: the
//! local sanity check is "the rust decoder still matches its baseline,"
//! the C-decoder columns stay tracked in CI.

#![cfg(feature = "validator-corpus")]

use std::path::PathBuf;
use std::process::Command;

const STRICT_FILTER: &str = "bugzilla/harvested";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn corpus_validator_strict_smoke() {
    let validator = env!("CARGO_BIN_EXE_corpus-validator");
    let rust_decoder = env!("CARGO_BIN_EXE_jbig2-decode");

    let output = Command::new(validator)
        .current_dir(workspace_root())
        .arg("--strict")
        .arg("--filter")
        .arg(STRICT_FILTER)
        .arg("--rust-decoder")
        .arg(rust_decoder)
        .output()
        .expect("spawn corpus-validator");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "corpus-validator --strict --filter {STRICT_FILTER} failed (status = {:?})\n\
         stderr:\n{stderr}\nstdout:\n{stdout}",
        output.status,
    );
}
