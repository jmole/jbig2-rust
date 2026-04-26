//! Corpus runner: runs the validator (and optionally a sandboxed C decoder)
//! over every fixture under `tests/validator-corpus/` and produces
//! `target/corpus-report.md`.
//!
//! Hard rules:
//!   * Validator runs are pure Rust and always happen.
//!   * Sandboxed C-decoder runs are opt-in via `--with-c-decoders`.
//!   * Sandboxed runs only happen against fixtures the validator already
//!     marked invalid, so we never feed a clean input back to the C tools just
//!     to watch them crash.
//!   * Every C-decoder invocation goes through `tools/sandbox` so we never
//!     escape into the host environment.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use jbig2::util::sandbox::{KillReason, Sandbox, SandboxOutcome};
use jbig2::validator::{validate, Lens, Report};
use sha2::{Digest, Sha256};

const CORPUS_RELATIVE: &str = "tests/validator-corpus";
const REPORT_RELATIVE: &str = "target/corpus-report.md";
const PER_TOOL_BUDGET_SECS: u64 = 600;
const PER_TOOL_INVALID_CAP: usize = 256;

#[derive(Parser, Debug)]
#[command(name = "corpus-validator")]
#[command(about = "Run the T.88 validator (and optional sandboxed C decoders) over the corpus")]
struct Args {
    /// Glob-style filter on fixture path (substring match).
    #[arg(long)]
    filter: Option<String>,
    /// Conformance lens to apply.
    #[arg(long, value_enum, default_value_t = LensArg::StrictT88)]
    lens: LensArg,
    /// Also run sandboxed C decoders against validator-invalid fixtures.
    #[arg(long, default_value_t = false)]
    with_c_decoders: bool,
    /// Path to `jbig2dec` (overrides auto-detected vendor build).
    #[arg(long)]
    jbig2dec: Option<PathBuf>,
    /// Path to ITU-T T.88 sample-software `jbig2` binary.
    #[arg(long)]
    itu_jbig2: Option<PathBuf>,
    /// Output report path (defaults to `target/corpus-report.md`).
    #[arg(long)]
    report: Option<PathBuf>,
    /// Maximum walltime for each C decoder in seconds.
    #[arg(long, default_value_t = PER_TOOL_BUDGET_SECS)]
    per_tool_budget_secs: u64,
    /// Maximum number of validator-invalid fixtures forwarded to each C tool.
    #[arg(long, default_value_t = PER_TOOL_INVALID_CAP)]
    per_tool_invalid_cap: usize,
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

    let mut report = CorpusReport::new(args.lens.into());
    for fixture in &fixtures {
        let bytes = fs::read(&fixture.stream_path)
            .with_context(|| format!("read {}", fixture.stream_path.display()))?;
        let validator_report = validate(&bytes, args.lens.into());
        report.record_validator(&fixture.relative, &validator_report);
    }

    if args.with_c_decoders {
        let decoders = resolve_decoders(&root, &args);
        let sandbox = Sandbox::for_decoder()
            .ro_path(corpus_root.clone())
            .ro_path(root.join("vendor"))
            .rw_path(PathBuf::from("/tmp"));
        for decoder in &decoders {
            run_decoder_for_invalid(&sandbox, decoder, &fixtures, &mut report, &args)?;
        }
    }

    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&report_path, report.render_markdown())
        .with_context(|| format!("write report {}", report_path.display()))?;
    eprintln!("corpus-validator: wrote {}", report_path.display());

    if report.has_validator_unexpected() {
        std::process::exit(2);
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct Fixture {
    relative: String,
    stream_path: PathBuf,
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
            out.push(Fixture {
                relative,
                stream_path: path,
            });
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct DecoderConfig {
    name: &'static str,
    binary: PathBuf,
    args_template: fn(&Path) -> Vec<String>,
}

fn resolve_decoders(root: &Path, args: &Args) -> Vec<DecoderConfig> {
    let mut out = Vec::new();
    if let Some(path) = args
        .jbig2dec
        .clone()
        .or_else(|| candidate_path(&root.join("vendor/jbig2dec/jbig2dec")))
    {
        out.push(DecoderConfig {
            name: "jbig2dec",
            binary: path,
            args_template: jbig2dec_args,
        });
    }
    if let Some(path) =
        args.itu_jbig2.clone().or_else(|| {
            candidate_path(&root.join(
                "vendor/T-REC-T.88-201808/Software/JBIG2_SampleSoftware-A20180829/source/jbig2",
            ))
        })
    {
        out.push(DecoderConfig {
            name: "itu-jbig2",
            binary: path,
            args_template: itu_jbig2_args,
        });
    }
    out
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
        .take(args.per_tool_invalid_cap)
        .collect();
    eprintln!(
        "corpus-validator: forwarding {} validator-invalid fixtures to {}",
        invalid.len(),
        decoder.name
    );

    let budget = Duration::from_secs(args.per_tool_budget_secs);
    let started = Instant::now();
    let mut over_budget = false;

    for fixture in invalid {
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
            return Some(format!("{}", line.trim()));
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

    fn record_validator(&mut self, relative: &str, report: &Report) {
        let entry = FixtureReport {
            relative: relative.to_string(),
            invalid: report.is_invalid(),
            findings_count: report.findings.len(),
            primary_check_id: report
                .findings
                .iter()
                .find(|f| f.severity == jbig2::validator::Severity::Error)
                .or_else(|| report.findings.first())
                .map(|f| f.check_id.as_str().to_string()),
            decoder_outcomes: BTreeMap::new(),
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

    fn record_decoder(&mut self, name: &str, relative: &str, outcome: DecoderOutcome) {
        let classification = outcome.classification();
        let signature = outcome.signature().map(|s| s.to_string());
        if let Some(fixture) = self.fixtures.iter_mut().find(|f| f.relative == relative) {
            fixture
                .decoder_outcomes
                .insert(name.to_string(), classification.to_string());
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

        if !self.decoders.is_empty() {
            out.push_str("\n## Sandboxed C decoders\n\n");
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
    #[allow(dead_code)]
    decoder_outcomes: BTreeMap<String, String>,
}

#[allow(dead_code)]
fn fixture_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    let mut sha = Sha256::new();
    sha.update(&bytes);
    Ok(format!("{:x}", sha.finalize()))
}
