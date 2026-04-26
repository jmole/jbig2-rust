//! Corpus runner: runs the validator and the configured set of sandboxed
//! decoders (`jbig2-rust` always, `jbig2dec` and the ITU sample decoder
//! opt-in via `--with-c-decoders`) over every fixture under
//! `tests/validator-corpus/`.
//!
//! Three operating modes:
//!
//! * Default — produces `target/corpus-report.md` with a per-fixture
//!   per-implementation matrix. Informational; non-zero exit only on
//!   validator-internal errors.
//! * `--baseline <path>` — same fixture sweep, but writes the observed
//!   per-impl `Verdict` to `<path>` so a maintainer can hand-review each
//!   cell and merge the result back into per-fixture `expected.toml`. This
//!   is the entry point for slice 3 of the decoder-conformance plan.
//! * `--strict` — same fixture sweep, exits non-zero if any cell
//!   mismatches the per-impl `[decoder.<impl>].verdict` recorded in
//!   `expected.toml`. This is the regression entry point that
//!   `tests/corpus_validator_strict.rs` and CI invoke.
//!
//! Hard rules kept from the original design:
//!   * Validator runs are pure Rust and always happen.
//!   * Sandboxed C-decoder runs are opt-in via `--with-c-decoders` and are
//!     only forwarded to fixtures the validator already marked invalid.
//!   * `jbig2-rust` runs only against fixtures whose `expected.toml` shape
//!     is `decoder-fixture` or `both`, so we do not pay decoder cost on the
//!     hundreds of synthetic / annex-h-bitflip fixtures whose contract is
//!     validator-only.
//!   * Every external invocation goes through `jbig2::util::sandbox` so we
//!     never escape into the host environment.
//!   * Before fixture sweep, a vendor-SHA preflight verifies configured
//!     vendor binaries still match `tools/conformance/known-issues.ron`.
//!     Override only for local debugging with
//!     `JBIG2_CORPUS_NO_VENDOR_CHECK=1`. For the sanitizer-canary deferral
//!     rationale and sequencing rule, see
//!     `docs/02-sandbox-preflights.md` ("Implementation status").

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use jbig2::util::sandbox::{KillReason, Sandbox, SandboxOutcome};
use jbig2::util::vendor_anchor::{load_anchors, short_sha, AnchorStatus, VendorAnchor};
use jbig2::validator::corpus::{Expected, Shape, Verdict};
use jbig2::validator::{validate, Lens, Report};
use sha2::{Digest, Sha256};

const CORPUS_RELATIVE: &str = "tests/validator-corpus";
const REPORT_RELATIVE: &str = "target/corpus-report.md";
const PER_TOOL_BUDGET_SECS: u64 = 600;
const PER_TOOL_INVALID_CAP: usize = 256;
const RUST_DECODER_BINARY: &str = "target/release/jbig2-decode";
const PRECHECK_NO_VENDOR_ENV: &str = "JBIG2_CORPUS_NO_VENDOR_CHECK";
const PRECHECK_EXIT_CODE: i32 = 4;

#[derive(Parser, Debug)]
#[command(name = "corpus-validator")]
#[command(about = "Run the T.88 validator and sandboxed decoders over the corpus")]
struct Args {
    /// Glob-style filter on fixture path (substring match).
    #[arg(long)]
    filter: Option<String>,
    /// Conformance lens to apply.
    #[arg(long, value_enum, default_value_t = LensArg::StrictT88)]
    lens: LensArg,
    /// Also run sandboxed C decoders (`jbig2dec`, ITU sample decoder)
    /// against validator-invalid fixtures.
    #[arg(long, default_value_t = false)]
    with_c_decoders: bool,
    /// Disable the `jbig2-rust` row even when the binary is present.
    #[arg(long, default_value_t = false)]
    without_rust: bool,
    /// Path to the `jbig2-decode` rust child binary (overrides the
    /// auto-detected `target/release/jbig2-decode`).
    #[arg(long)]
    rust_decoder: Option<PathBuf>,
    /// Path to `jbig2dec` (overrides auto-detected vendor build).
    #[arg(long)]
    jbig2dec: Option<PathBuf>,
    /// Path to ITU-T T.88 sample-software `jbig2` binary.
    #[arg(long)]
    itu_jbig2: Option<PathBuf>,
    /// Output report path (defaults to `target/corpus-report.md`).
    #[arg(long)]
    report: Option<PathBuf>,
    /// Maximum walltime for each decoder in seconds.
    #[arg(long, default_value_t = PER_TOOL_BUDGET_SECS)]
    per_tool_budget_secs: u64,
    /// Maximum number of validator-invalid fixtures forwarded to each C
    /// tool (does not apply to the `jbig2-rust` row, which is bounded by
    /// the count of decoder-fixture shaped fixtures).
    #[arg(long, default_value_t = PER_TOOL_INVALID_CAP)]
    per_tool_invalid_cap: usize,
    /// Emit observed per-impl `Verdict` per fixture to the given path
    /// instead of running the report. The maintainer hand-reviews each
    /// cell and merges the reviewed verdicts into the corresponding
    /// `expected.toml`.
    #[arg(long)]
    baseline: Option<PathBuf>,
    /// Compare each cell against the recorded `expected.toml` and exit
    /// non-zero on mismatch. This is the regression entry point.
    #[arg(long, default_value_t = false)]
    strict: bool,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum LensArg {
    StrictT88,
    Jbig2decInterop,
    ItuT88Interop,
    ImageioInterop,
}

impl From<LensArg> for Lens {
    fn from(value: LensArg) -> Self {
        match value {
            LensArg::StrictT88 => Self::StrictT88,
            LensArg::Jbig2decInterop => Self::Jbig2decInterop,
            LensArg::ItuT88Interop => Self::ItuT88Interop,
            LensArg::ImageioInterop => Self::ImageioInterop,
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let corpus_root = root.join(CORPUS_RELATIVE);
    let report_path = args
        .report
        .clone()
        .unwrap_or_else(|| root.join(REPORT_RELATIVE));

    let fixtures = collect_fixtures(&corpus_root, args.filter.as_deref())?;
    eprintln!(
        "corpus-validator: discovered {} fixtures under {}",
        fixtures.len(),
        corpus_root.display()
    );

    let decoders = resolve_decoders(&root, &args);
    eprintln!(
        "corpus-validator: configured decoders: {}",
        decoders
            .iter()
            .map(|d| d.name.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    match run_preflight(&root, &decoders) {
        Ok(lines) => {
            for line in lines {
                eprintln!("{line}");
            }
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(PRECHECK_EXIT_CODE);
        }
    }

    let sandbox = Sandbox::for_decoder()
        .ro_path(corpus_root.clone())
        .ro_path(root.join("vendor"))
        .ro_path(root.join("target"))
        .rw_path(PathBuf::from("/tmp"));

    let mut report = CorpusReport::new(args.lens.into());
    for fixture in &fixtures {
        let bytes = fs::read(&fixture.stream_path)
            .with_context(|| format!("read {}", fixture.stream_path.display()))?;
        let validator_report = validate(&bytes, args.lens.into());
        let expected = read_expected_for(&fixture.expected_path);
        report.record_fixture(fixture, &validator_report, expected);
    }

    let invalid_count = report.fixtures.iter().filter(|f| f.invalid).count();
    let decoder_fixture_count = report
        .fixtures
        .iter()
        .filter(|f| f.runs_decoder_row())
        .count();

    for decoder in &decoders {
        if decoder.always_run_decoder_fixtures {
            run_decoder_against_decoder_fixtures(&sandbox, decoder, &fixtures, &mut report, &args)?;
        }
        if decoder.run_against_validator_invalid {
            run_decoder_for_invalid(&sandbox, decoder, &fixtures, &mut report, &args)?;
        }
    }

    eprintln!(
        "corpus-validator: validator-invalid={invalid_count}, decoder-fixture-rows={decoder_fixture_count}"
    );

    if let Some(baseline_path) = args.baseline.as_ref() {
        if let Some(parent) = baseline_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = report.render_baseline_toml();
        fs::write(baseline_path, body)
            .with_context(|| format!("write baseline {}", baseline_path.display()))?;
        eprintln!(
            "corpus-validator: wrote baseline {}",
            baseline_path.display()
        );
        return Ok(());
    }

    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&report_path, report.render_markdown())
        .with_context(|| format!("write report {}", report_path.display()))?;
    eprintln!("corpus-validator: wrote {}", report_path.display());

    if args.strict {
        let configured: std::collections::BTreeSet<String> =
            decoders.iter().map(|d| d.name.to_string()).collect();
        let mismatches = report.collect_strict_mismatches(&configured);
        if !mismatches.is_empty() {
            eprintln!(
                "corpus-validator --strict: {} expectation mismatch(es):",
                mismatches.len()
            );
            for m in &mismatches {
                eprintln!("  {m}");
            }
            std::process::exit(3);
        }
        eprintln!("corpus-validator --strict: all cells match recorded expectations");
    }

    if report.has_validator_unexpected() {
        std::process::exit(2);
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct Fixture {
    relative: String,
    stream_path: PathBuf,
    expected_path: PathBuf,
}

fn collect_fixtures(root: &Path, filter: Option<&str>) -> Result<Vec<Fixture>> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return Ok(out);
    }
    visit(root, root, &mut out, filter)?;
    out.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(out)
}

fn visit(root: &Path, dir: &Path, out: &mut Vec<Fixture>, filter: Option<&str>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            visit(root, &path, out, filter)?;
        } else if path.file_name().and_then(|n| n.to_str()) == Some("stream.jb2") {
            let parent = path.parent().unwrap_or(root);
            let relative = parent
                .strip_prefix(root)
                .unwrap_or(parent)
                .to_string_lossy()
                .replace('\\', "/");
            if let Some(filter) = filter {
                if !relative.contains(filter) {
                    continue;
                }
            }
            let expected_path = parent.join("expected.toml");
            out.push(Fixture {
                relative,
                stream_path: path,
                expected_path,
            });
        }
    }
    Ok(())
}

fn read_expected_for(path: &Path) -> Option<Expected> {
    if !path.exists() {
        return None;
    }
    match Expected::read(path) {
        Ok(e) => Some(e),
        Err(err) => {
            eprintln!("corpus-validator: skipping malformed expected.toml ({err})");
            None
        }
    }
}

#[derive(Clone, Debug)]
struct DecoderConfig {
    name: &'static str,
    binary: PathBuf,
    args_template: fn(&Path) -> Vec<String>,
    /// Run against every fixture whose shape is `decoder-fixture` or
    /// `both`, regardless of validator outcome. The `jbig2-rust` row uses
    /// this; C decoders do not, so we never feed a clean input back to
    /// `jbig2dec` just to watch it crash.
    always_run_decoder_fixtures: bool,
    /// Run against validator-invalid fixtures (existing behaviour).
    run_against_validator_invalid: bool,
}

#[derive(Debug)]
enum PreflightError {
    LoadAnchors(String),
    MissingAnchor {
        decoder: String,
        binary_rel: String,
    },
    CheckFailed {
        decoder: String,
        anchor_path: String,
        error: String,
    },
    Mismatch {
        decoder: String,
        anchor_path: String,
        expected: String,
        actual: String,
    },
}

impl std::fmt::Display for PreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadAnchors(err) => write!(
                f,
                "corpus-validator: preflight FAILED: could not read vendor anchors from tools/conformance/known-issues.ron: {err}"
            ),
            Self::MissingAnchor {
                decoder,
                binary_rel,
            } => write!(
                f,
                "corpus-validator: preflight FAILED for {decoder}\n  {binary_rel} is under vendor/ but no GitSha pin was found in tools/conformance/known-issues.ron\n  Add vendor: GitSha {{ path: \"vendor/<submodule>\", sha: \"<commit>\" }} to known-issues.ron\n  Override (local debugging only): {PRECHECK_NO_VENDOR_ENV}=1"
            ),
            Self::CheckFailed {
                decoder,
                anchor_path,
                error,
            } => write!(
                f,
                "corpus-validator: preflight FAILED for {decoder}\n  failed to read current vendor anchor for {anchor_path}: {error}\n  Override (local debugging only): {PRECHECK_NO_VENDOR_ENV}=1"
            ),
            Self::Mismatch {
                decoder,
                anchor_path,
                expected,
                actual,
            } => write!(
                f,
                "corpus-validator: preflight FAILED for {decoder}\n  {anchor_path} is at {}, known-issues.ron pins {}\n  Either: git -C {anchor_path} checkout {expected}\n  Or: update the pin in tools/conformance/known-issues.ron and regenerate baselines.\n  Override (local debugging only): {PRECHECK_NO_VENDOR_ENV}=1",
                short_sha(actual),
                short_sha(expected),
            ),
        }
    }
}

fn run_preflight(root: &Path, decoders: &[DecoderConfig]) -> Result<Vec<String>, PreflightError> {
    let anchors = load_anchors(root).map_err(PreflightError::LoadAnchors)?;
    let mut lines = Vec::new();
    let allow_override = std::env::var(PRECHECK_NO_VENDOR_ENV).ok().as_deref() == Some("1");

    for decoder in decoders {
        let binary_abs = if decoder.binary.is_absolute() {
            decoder.binary.clone()
        } else {
            root.join(&decoder.binary)
        };
        let Ok(binary_rel_path) = binary_abs.strip_prefix(root) else {
            lines.push(format!(
                "corpus-validator: preflight: {} skipped (binary not under vendor/<submodule>)",
                decoder.name
            ));
            continue;
        };
        let binary_rel = binary_rel_path.to_string_lossy().replace('\\', "/");
        if !binary_rel.starts_with("vendor/") {
            lines.push(format!(
                "corpus-validator: preflight: {} skipped (binary not under vendor/<submodule>)",
                decoder.name
            ));
            continue;
        }

        let anchor = match VendorAnchor::lookup_for_binary(&anchors, &binary_abs, root) {
            Some(anchor) => anchor,
            None => {
                let err = PreflightError::MissingAnchor {
                    decoder: decoder.name.to_string(),
                    binary_rel: binary_rel.clone(),
                };
                if allow_override {
                    lines.push(format!(
                        "corpus-validator: preflight: {} WARNING ({})",
                        decoder.name, err
                    ));
                    continue;
                }
                return Err(err);
            }
        };

        let anchor_path = anchor.path.to_string_lossy().replace('\\', "/");
        match anchor.check(root) {
            Ok(AnchorStatus::Ok { actual }) => lines.push(format!(
                "corpus-validator: preflight: {} OK ({} @ {})",
                decoder.name,
                anchor_path,
                short_sha(&actual)
            )),
            Ok(AnchorStatus::Mismatch { expected, actual }) => {
                let err = PreflightError::Mismatch {
                    decoder: decoder.name.to_string(),
                    anchor_path,
                    expected,
                    actual,
                };
                if allow_override {
                    lines.push(format!(
                        "corpus-validator: preflight: {} WARNING ({})",
                        decoder.name, err
                    ));
                    continue;
                }
                return Err(err);
            }
            Err(error) => {
                let err = PreflightError::CheckFailed {
                    decoder: decoder.name.to_string(),
                    anchor_path,
                    error,
                };
                if allow_override {
                    lines.push(format!(
                        "corpus-validator: preflight: {} WARNING ({})",
                        decoder.name, err
                    ));
                    continue;
                }
                return Err(err);
            }
        }
    }

    Ok(lines)
}

fn resolve_decoders(root: &Path, args: &Args) -> Vec<DecoderConfig> {
    let mut out = Vec::new();
    if !args.without_rust {
        if let Some(path) = args
            .rust_decoder
            .clone()
            .or_else(|| candidate_path(&root.join(RUST_DECODER_BINARY)))
        {
            out.push(DecoderConfig {
                name: "rust",
                binary: path,
                args_template: rust_args,
                always_run_decoder_fixtures: true,
                run_against_validator_invalid: false,
            });
        } else {
            eprintln!(
                "corpus-validator: jbig2-rust row disabled: {} not present (build with `cargo build --release --bin jbig2-decode`)",
                root.join(RUST_DECODER_BINARY).display()
            );
        }
    }
    if args.with_c_decoders {
        if let Some(path) = args
            .jbig2dec
            .clone()
            .or_else(|| candidate_path(&root.join("vendor/jbig2dec/jbig2dec")))
        {
            out.push(DecoderConfig {
                name: "jbig2dec",
                binary: path,
                args_template: jbig2dec_args,
                always_run_decoder_fixtures: false,
                run_against_validator_invalid: true,
            });
        }
        if let Some(path) = args.itu_jbig2.clone().or_else(|| {
            candidate_path(&root.join(
                "vendor/T-REC-T.88-201808/Software/JBIG2_SampleSoftware-A20180829/source/jbig2",
            ))
        }) {
            out.push(DecoderConfig {
                name: "itu_t88",
                binary: path,
                args_template: itu_jbig2_args,
                always_run_decoder_fixtures: false,
                run_against_validator_invalid: true,
            });
        }
    }
    out
}

fn rust_args(stream: &Path) -> Vec<String> {
    vec![stream.display().to_string()]
}

fn jbig2dec_args(stream: &Path) -> Vec<String> {
    vec![
        "--format".into(),
        "pbm".into(),
        "-o".into(),
        "/tmp/corpus-out.pbm".into(),
        stream.display().to_string(),
    ]
}

fn itu_jbig2_args(stream: &Path) -> Vec<String> {
    vec![
        "-i".into(),
        stream.display().to_string(),
        "-f".into(),
        "jb2".into(),
        "-o".into(),
        "/tmp/corpus-out".into(),
        "-F".into(),
        "bmp".into(),
    ]
}

fn candidate_path(p: &Path) -> Option<PathBuf> {
    if p.is_file() {
        Some(p.to_path_buf())
    } else {
        None
    }
}

fn run_decoder_for_invalid(
    sandbox: &Sandbox,
    decoder: &DecoderConfig,
    fixtures: &[Fixture],
    report: &mut CorpusReport,
    args: &Args,
) -> Result<()> {
    let invalid: Vec<&Fixture> = fixtures
        .iter()
        .filter(|f| report.invalid_relative(&f.relative))
        .filter(|f| !report.already_ran(decoder.name, &f.relative))
        .take(args.per_tool_invalid_cap)
        .collect();
    eprintln!(
        "corpus-validator: forwarding {} validator-invalid fixtures to {}",
        invalid.len(),
        decoder.name
    );

    drive_decoder(sandbox, decoder, &invalid, report, args)
}

fn run_decoder_against_decoder_fixtures(
    sandbox: &Sandbox,
    decoder: &DecoderConfig,
    fixtures: &[Fixture],
    report: &mut CorpusReport,
    args: &Args,
) -> Result<()> {
    let targets: Vec<&Fixture> = fixtures
        .iter()
        .filter(|f| report.runs_decoder_row_for(&f.relative))
        .filter(|f| !report.already_ran(decoder.name, &f.relative))
        .collect();
    eprintln!(
        "corpus-validator: forwarding {} decoder-fixture-shaped fixtures to {}",
        targets.len(),
        decoder.name
    );
    drive_decoder(sandbox, decoder, &targets, report, args)
}

fn drive_decoder(
    sandbox: &Sandbox,
    decoder: &DecoderConfig,
    targets: &[&Fixture],
    report: &mut CorpusReport,
    args: &Args,
) -> Result<()> {
    let budget = Duration::from_secs(args.per_tool_budget_secs);
    let started = Instant::now();
    let mut over_budget = false;

    for fixture in targets {
        if started.elapsed() >= budget {
            over_budget = true;
            break;
        }
        let mut cmd = Command::new(&decoder.binary);
        cmd.args((decoder.args_template)(&fixture.stream_path));
        let outcome = match sandbox.run(cmd) {
            Ok(outcome) => DecoderOutcome::from_outcome(&outcome),
            Err(err) => DecoderOutcome::SpawnError(err.to_string()),
        };
        report.record_decoder(decoder.name, &fixture.relative, outcome);
    }
    if over_budget {
        report.note_truncation(decoder.name, args.per_tool_budget_secs);
    }
    Ok(())
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum DecoderOutcome {
    Ok,
    NonZeroExit {
        code: Option<i32>,
        signature: Option<String>,
    },
    SanitizerHit {
        signature: String,
    },
    Crash {
        signal: String,
        signature: Option<String>,
    },
    Timeout,
    SpawnError(String),
}

impl DecoderOutcome {
    fn from_outcome(outcome: &SandboxOutcome) -> Self {
        let stderr = String::from_utf8_lossy(&outcome.output.stderr);
        if let Some(reason) = outcome.kill_reason {
            match reason {
                KillReason::Timeout => return Self::Timeout,
                KillReason::AddressSpace => {
                    return Self::SanitizerHit {
                        signature: format!(
                            "sandbox: address-space limit hit ({} stderr bytes)",
                            outcome.output.stderr.len()
                        ),
                    };
                }
                KillReason::OutputBytes => {
                    return Self::NonZeroExit {
                        code: None,
                        signature: Some(format!(
                            "sandbox: output cap hit ({} bytes captured)",
                            outcome.output.stdout.len() + outcome.output.stderr.len()
                        )),
                    };
                }
            }
        }
        let san_signature = sanitizer_signature(&stderr);
        if let Some(sig) = san_signature {
            return Self::SanitizerHit { signature: sig };
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = outcome.output.status.signal() {
                return Self::Crash {
                    signal: signal_name(signal),
                    signature: stderr_signature(&stderr),
                };
            }
        }
        if outcome.output.status.success() {
            Self::Ok
        } else {
            Self::NonZeroExit {
                code: outcome.output.status.code(),
                signature: stderr_signature(&stderr),
            }
        }
    }

    fn classification(&self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::NonZeroExit { .. } => "EXIT",
            Self::SanitizerHit { .. } => "SAN",
            Self::Crash { .. } => "CRASH",
            Self::Timeout => "TIMEOUT",
            Self::SpawnError(_) => "SPAWN",
        }
    }

    /// Distil the classification into a `Verdict` the per-fixture
    /// `[decoder.<impl>]` block can record. Two distinct crash signals
    /// (SIGSEGV vs SIGABRT) both fold into `Verdict::Crash`; the
    /// classification + signature still surface as evidence in the report
    /// when the verdict mismatches expectations.
    fn verdict(&self) -> Verdict {
        match self {
            Self::Ok => Verdict::Ok,
            Self::NonZeroExit { .. } => Verdict::RejectErr,
            Self::SanitizerHit { .. } | Self::Crash { .. } => Verdict::Crash,
            Self::Timeout => Verdict::Timeout,
            // Spawn errors are harness bugs, not decoder verdicts. Leave
            // the cell `Unknown` so the maintainer notices.
            Self::SpawnError(_) => Verdict::Unknown,
        }
    }

    fn signature(&self) -> Option<&str> {
        match self {
            Self::SanitizerHit { signature } => Some(signature.as_str()),
            Self::Crash { signature, .. } => signature.as_deref(),
            Self::NonZeroExit { signature, .. } => signature.as_deref(),
            Self::SpawnError(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

fn sanitizer_signature(stderr: &str) -> Option<String> {
    let needles = [
        "AddressSanitizer:",
        "UndefinedBehaviorSanitizer:",
        "LeakSanitizer:",
        "ThreadSanitizer:",
        "MemorySanitizer:",
    ];
    for needle in needles {
        if let Some(idx) = stderr.find(needle) {
            let after = &stderr[idx..];
            let line = after.lines().next().unwrap_or("");
            return Some(line.trim().to_string());
        }
    }
    None
}

fn stderr_signature(stderr: &str) -> Option<String> {
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        return None;
    }
    let line = trimmed.lines().rev().find(|l| !l.trim().is_empty())?;
    Some(format!("{:.160}", line.trim()))
}

#[cfg(unix)]
fn signal_name(signal: i32) -> String {
    match signal {
        6 => "SIGABRT".into(),
        9 => "SIGKILL".into(),
        11 => "SIGSEGV".into(),
        7 => "SIGBUS".into(),
        8 => "SIGFPE".into(),
        4 => "SIGILL".into(),
        s => format!("signal {s}"),
    }
}

#[derive(Default)]
struct CorpusReport {
    lens: Lens,
    fixtures: Vec<FixtureReport>,
    decoders: BTreeMap<String, DecoderRollup>,
    truncations: Vec<(String, u64)>,
}

impl CorpusReport {
    fn new(lens: Lens) -> Self {
        Self {
            lens,
            ..Default::default()
        }
    }

    fn record_fixture(&mut self, fixture: &Fixture, report: &Report, expected: Option<Expected>) {
        let entry = FixtureReport {
            relative: fixture.relative.clone(),
            invalid: report.is_invalid(),
            findings_count: report.findings.len(),
            primary_check_id: report
                .findings
                .iter()
                .find(|f| f.severity == jbig2::validator::Severity::Error)
                .or_else(|| report.findings.first())
                .map(|f| f.check_id.as_str().to_string()),
            decoder_runs: BTreeMap::new(),
            expected,
        };
        self.fixtures.push(entry);
    }

    fn invalid_relative(&self, relative: &str) -> bool {
        self.fixtures
            .iter()
            .find(|f| f.relative == relative)
            .map(|f| f.invalid)
            .unwrap_or(false)
    }

    fn runs_decoder_row_for(&self, relative: &str) -> bool {
        self.fixtures
            .iter()
            .find(|f| f.relative == relative)
            .map(|f| f.runs_decoder_row())
            .unwrap_or(false)
    }

    fn already_ran(&self, decoder: &str, relative: &str) -> bool {
        self.fixtures
            .iter()
            .find(|f| f.relative == relative)
            .map(|f| f.decoder_runs.contains_key(decoder))
            .unwrap_or(false)
    }

    fn record_decoder(&mut self, name: &str, relative: &str, outcome: DecoderOutcome) {
        let classification = outcome.classification();
        let signature = outcome.signature().map(|s| s.to_string());
        let verdict = outcome.verdict();
        if let Some(fixture) = self.fixtures.iter_mut().find(|f| f.relative == relative) {
            fixture.decoder_runs.insert(
                name.to_string(),
                DecoderRunRecord {
                    verdict,
                    classification: classification.to_string(),
                    signature: signature.clone(),
                },
            );
        }
        let rollup = self.decoders.entry(name.to_string()).or_default();
        rollup.total += 1;
        match classification {
            "OK" => rollup.ok += 1,
            "EXIT" => rollup.exit += 1,
            "SAN" => rollup.sanitizer += 1,
            "CRASH" => rollup.crashes += 1,
            "TIMEOUT" => rollup.timeouts += 1,
            "SPAWN" => rollup.spawn_errors += 1,
            _ => {}
        }
        if let Some(sig) = signature {
            *rollup.signatures.entry(sig).or_insert(0) += 1;
        }
    }

    fn note_truncation(&mut self, name: &str, budget_secs: u64) {
        self.truncations.push((name.to_string(), budget_secs));
    }

    fn has_validator_unexpected(&self) -> bool {
        false
    }

    /// Emit `target/baseline.toml`-style document grouping observed
    /// verdicts by fixture. The maintainer reviews each cell and copies
    /// the reviewed verdict back into the corresponding `expected.toml`.
    fn render_baseline_toml(&self) -> String {
        let mut out = String::new();
        out.push_str(
            "# corpus-validator baseline.\n\
             # Hand-review each cell, then merge into the relevant\n\
             # `expected.toml::[decoder.<impl>]`. Verdict = Unknown means\n\
             # the harness did not run that impl on that fixture (binary\n\
             # missing or shape did not require it).\n\n",
        );
        for f in &self.fixtures {
            if !f.runs_decoder_row() {
                continue;
            }
            out.push_str(&format!("[fixture.{:?}]\n", f.relative));
            for (impl_name, run) in &f.decoder_runs {
                out.push_str(&format!(
                    "{}_verdict = {:?}\n{}_classification = {:?}\n",
                    impl_name,
                    run.verdict.as_str(),
                    impl_name,
                    run.classification,
                ));
                if let Some(sig) = run.signature.as_ref() {
                    out.push_str(&format!("{}_signature = {:?}\n", impl_name, sig));
                }
            }
            out.push('\n');
        }
        out
    }

    fn render_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Corpus report (lens = {:?})\n\n", self.lens));
        out.push_str(&format!("Total fixtures: {}\n\n", self.fixtures.len()));
        let invalid_count = self.fixtures.iter().filter(|f| f.invalid).count();
        out.push_str(&format!("Validator-invalid: {}\n\n", invalid_count));

        out.push_str("## Validator findings\n\n");
        out.push_str("| Fixture | Invalid | Findings | Primary CheckId |\n");
        out.push_str("|---|---|---|---|\n");
        for f in &self.fixtures {
            out.push_str(&format!(
                "| `{}` | {} | {} | {} |\n",
                f.relative,
                if f.invalid { "yes" } else { "no" },
                f.findings_count,
                f.primary_check_id.as_deref().unwrap_or("-"),
            ));
        }

        // Per-fixture per-impl matrix, restricted to fixtures whose shape
        // requires decoder-row coverage. Cells render as
        // `verdict[expected:<vY>]` so a single mismatching cell pops out.
        let decoder_rows: Vec<&FixtureReport> = self
            .fixtures
            .iter()
            .filter(|f| f.runs_decoder_row())
            .collect();
        if !decoder_rows.is_empty() {
            let impls = self.observed_decoder_names();
            out.push_str("\n## Per-fixture decoder matrix\n\n");
            out.push_str("Cell legend: `<observed>[exp=<expected>]` — `match` if observed equals expected, `MISMATCH` otherwise.\n\n");
            out.push_str("| Fixture |");
            for name in &impls {
                out.push_str(&format!(" {} |", name));
            }
            out.push('\n');
            out.push_str("|---|");
            for _ in &impls {
                out.push_str("---|");
            }
            out.push('\n');
            for f in &decoder_rows {
                out.push_str(&format!("| `{}` |", f.relative));
                for name in &impls {
                    out.push_str(&format!(" {} |", f.cell_for(name)));
                }
                out.push('\n');
            }
        }

        if !self.decoders.is_empty() {
            out.push_str("\n## Decoder rollups\n\n");
            for (name, rollup) in &self.decoders {
                out.push_str(&format!("### {}\n\n", name));
                out.push_str(&format!(
                    "- Total: {} (OK {}, EXIT {}, SAN {}, CRASH {}, TIMEOUT {}, SPAWN {})\n",
                    rollup.total,
                    rollup.ok,
                    rollup.exit,
                    rollup.sanitizer,
                    rollup.crashes,
                    rollup.timeouts,
                    rollup.spawn_errors,
                ));
                out.push_str("- Sanitizer / signature buckets (by frequency):\n");
                let mut buckets: Vec<(&String, &usize)> = rollup.signatures.iter().collect();
                buckets.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
                for (sig, count) in buckets.iter().take(20) {
                    out.push_str(&format!("  - {} x `{}`\n", count, sig));
                }
                out.push('\n');
            }
        }

        if !self.truncations.is_empty() {
            out.push_str("\n## Budget truncations\n\n");
            for (name, budget) in &self.truncations {
                out.push_str(&format!(
                    "- `{}` ran out at {}s; consider raising `--per-tool-budget-secs`.\n",
                    name, budget
                ));
            }
        }
        out
    }

    fn observed_decoder_names(&self) -> Vec<String> {
        let mut set: std::collections::BTreeSet<String> = self.decoders.keys().cloned().collect();
        for f in &self.fixtures {
            for k in f.decoder_runs.keys() {
                set.insert(k.clone());
            }
        }
        // Also include implementations that have an [decoder.<impl>] block
        // in expected.toml even if we didn't observe them this run; the
        // matrix should still surface that cell as `Unknown[exp=...]`.
        for f in &self.fixtures {
            if let Some(expected) = f.expected.as_ref() {
                for k in expected.decoder.keys() {
                    set.insert(k.clone());
                }
            }
        }
        set.into_iter().collect()
    }

    /// Walk every fixture that runs a decoder row. For each impl with a
    /// recorded `expected.toml::decoder.<impl>.verdict` that is not
    /// `Unknown`, produce a one-line mismatch description if observed
    /// disagrees. Returned in stable fixture order so CI logs diff cleanly.
    ///
    /// `configured_impls` is the set of decoder names that were configured
    /// for this run. Cells whose impl was not configured are skipped: a
    /// strict run only enforces expectations for the impls it actually has
    /// the binary for. CI is responsible for choosing which impls to
    /// configure (e.g. via `--with-c-decoders`); local `cargo test` runs
    /// only check the impls available without extra setup.
    fn collect_strict_mismatches(
        &self,
        configured_impls: &std::collections::BTreeSet<String>,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for f in &self.fixtures {
            if !f.runs_decoder_row() {
                continue;
            }
            let Some(expected) = f.expected.as_ref() else {
                continue;
            };
            for (name, exp) in &expected.decoder {
                if exp.verdict == Verdict::Unknown {
                    continue;
                }
                if !configured_impls.contains(name) {
                    continue;
                }
                let observed = f
                    .decoder_runs
                    .get(name)
                    .map(|r| r.verdict)
                    .unwrap_or(Verdict::Unknown);
                if observed != exp.verdict {
                    let signature = f
                        .decoder_runs
                        .get(name)
                        .and_then(|r| r.signature.clone())
                        .unwrap_or_else(|| "<no signature>".into());
                    out.push(format!(
                        "{} :: {} :: expected={} observed={} :: {}",
                        f.relative, name, exp.verdict, observed, signature
                    ));
                }
            }
        }
        out
    }
}

#[derive(Default)]
struct DecoderRollup {
    total: usize,
    ok: usize,
    exit: usize,
    sanitizer: usize,
    crashes: usize,
    timeouts: usize,
    spawn_errors: usize,
    signatures: BTreeMap<String, usize>,
}

#[derive(Debug)]
struct FixtureReport {
    relative: String,
    invalid: bool,
    findings_count: usize,
    primary_check_id: Option<String>,
    decoder_runs: BTreeMap<String, DecoderRunRecord>,
    expected: Option<Expected>,
}

impl FixtureReport {
    /// Whether this fixture's shape requires the per-impl decoder row.
    fn runs_decoder_row(&self) -> bool {
        matches!(
            self.expected.as_ref().map(|e| e.shape),
            Some(Shape::DecoderFixture) | Some(Shape::Both)
        )
    }

    fn cell_for(&self, impl_name: &str) -> String {
        let observed = self
            .decoder_runs
            .get(impl_name)
            .map(|r| r.verdict)
            .unwrap_or(Verdict::Unknown);
        let expected = self
            .expected
            .as_ref()
            .and_then(|e| e.decoder.get(impl_name))
            .map(|exp| exp.verdict)
            .unwrap_or(Verdict::Unknown);
        if expected == Verdict::Unknown {
            format!("`{}[exp=Unknown]`", observed)
        } else if observed == expected {
            format!("`{}` ✓", observed)
        } else {
            format!("`{}[exp={}]` MISMATCH", observed, expected)
        }
    }
}

#[derive(Debug)]
struct DecoderRunRecord {
    verdict: Verdict,
    classification: String,
    signature: Option<String>,
}

#[allow(dead_code)]
fn fixture_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    let mut sha = Sha256::new();
    sha.update(&bytes);
    Ok(format!("{:x}", sha.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock drift")
            .as_nanos();
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp-tests");
        fs::create_dir_all(&base).expect("create tmp-tests dir");
        let dir = base.join(format!("jbig2-{prefix}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let template_dir = dir.join(".git-template");
        fs::create_dir_all(&template_dir).expect("create git template dir");
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .env("GIT_TEMPLATE_DIR", &template_dir)
            .status()
            .expect("spawn git");
        assert!(
            status.success(),
            "git {:?} failed in {}",
            args,
            dir.display()
        );
    }

    fn head_sha(dir: &Path) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .expect("git rev-parse");
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn seed_known_issues(root: &Path, sha: &str) {
        let known_issues = root.join("tools/conformance/known-issues.ron");
        fs::create_dir_all(known_issues.parent().expect("parent")).expect("mkdir");
        fs::write(
            known_issues,
            format!(
                r#"
[
    (vendor: Some(GitSha(path: "vendor/jbig2dec", sha: "{sha}"))),
]
"#
            ),
        )
        .expect("write known issues");
    }

    fn make_vendor_repo(root: &Path) -> PathBuf {
        let repo = root.join("vendor/jbig2dec");
        fs::create_dir_all(&repo).expect("create repo");
        run_git(&repo, &["init", "."]);
        fs::write(repo.join("README"), "seed\n").expect("write file");
        run_git(&repo, &["add", "README"]);
        run_git(&repo, &["commit", "-m", "init"]);
        fs::write(repo.join("jbig2dec"), "#!/bin/sh\nexit 0\n").expect("write fake binary");
        repo
    }

    fn decoder_for(path: PathBuf) -> DecoderConfig {
        DecoderConfig {
            name: "jbig2dec",
            binary: path,
            args_template: jbig2dec_args,
            always_run_decoder_fixtures: false,
            run_against_validator_invalid: true,
        }
    }

    #[test]
    fn preflight_ok_for_matching_vendor_sha() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::remove_var(PRECHECK_NO_VENDOR_ENV);
        let root = unique_temp_dir("preflight-ok");
        let repo = make_vendor_repo(&root);
        let sha = head_sha(&repo);
        seed_known_issues(&root, &sha);
        let decoder = decoder_for(root.join("vendor/jbig2dec/jbig2dec"));

        let lines = run_preflight(&root, &[decoder]).expect("preflight ok");
        assert!(
            lines.iter().any(|l| l.contains("preflight: jbig2dec OK")),
            "{lines:#?}"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn preflight_mismatch_returns_error() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::remove_var(PRECHECK_NO_VENDOR_ENV);
        let root = unique_temp_dir("preflight-mismatch");
        let repo = make_vendor_repo(&root);
        seed_known_issues(&root, "deadbeef");
        let decoder = decoder_for(root.join("vendor/jbig2dec/jbig2dec"));

        let err = run_preflight(&root, &[decoder]).expect_err("expected mismatch");
        assert!(matches!(err, PreflightError::Mismatch { .. }), "{err:?}");

        // Keep the repo variable used so clippy does not complain.
        assert!(repo.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn preflight_mismatch_allows_override() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::remove_var(PRECHECK_NO_VENDOR_ENV);
        let root = unique_temp_dir("preflight-override");
        let _repo = make_vendor_repo(&root);
        seed_known_issues(&root, "deadbeef");
        let decoder = decoder_for(root.join("vendor/jbig2dec/jbig2dec"));

        std::env::set_var(PRECHECK_NO_VENDOR_ENV, "1");
        let lines = run_preflight(&root, &[decoder]).expect("override should pass");
        std::env::remove_var(PRECHECK_NO_VENDOR_ENV);
        assert!(
            lines
                .iter()
                .any(|l| l.contains("preflight: jbig2dec WARNING")),
            "{lines:#?}"
        );

        let _ = fs::remove_dir_all(root);
    }
}
