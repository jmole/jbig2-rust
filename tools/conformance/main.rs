use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Cursor, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use jbig2::{
    Bitmap, Coding, EncoderConfig, GenericTemplate, Jbig2Decoder, Jbig2Encoder, Mode, RgbBitmap,
};
use serde::Deserialize;

fn main() {
    if let Err(err) = run() {
        eprintln!("conformance-matrix: {err}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    let color = ColorMode::new(args.no_color);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let known_issues = KnownIssues::load(&root)?;
    if let Some(path) = &args.write_known_issues_doc {
        known_issues.write_markdown_doc(&root, path)?;
        return Ok(());
    }

    let workdir = root.join("target").join("conformance-matrix");
    reset_dir(&workdir)?;

    let tools = Tools::resolve(&root, &workdir);
    if args.self_check {
        run_self_check(&workdir, &tools)?;
        return Ok(());
    }

    let mut summaries = Vec::new();
    let mut strict_failed = false;

    match args.phase {
        Phase::Decode | Phase::Both => {
            let matrix = run_decode_matrix(&root, &workdir, &tools, args.target)?;
            let stats = render_matrix(&matrix, &known_issues, color);
            strict_failed |= stats.strict_failed();
            summaries.push(stats.summary_text());
        }
        Phase::Encode => {}
    }

    match args.phase {
        Phase::Encode | Phase::Both => {
            let matrix = run_encode_matrix(&root, &workdir, &tools, args.target)?;
            let stats = render_matrix(&matrix, &known_issues, color);
            strict_failed |= stats.strict_failed();
            summaries.push(stats.summary_text());
        }
        Phase::Decode => {}
    }

    if !summaries.is_empty() {
        let joined = summaries
            .iter()
            .map(|summary| summary.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        println!("SUMMARY: {joined}");
    }

    if args.strict_mode && strict_failed {
        return Err(
            "--strict: unexpected, drifted, or resolved known-issue cells present".to_string(),
        );
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Deserialize, Hash, PartialEq, Eq)]
enum Phase {
    Decode,
    Encode,
    Both,
}

impl Phase {
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "decode" => Ok(Self::Decode),
            "encode" => Ok(Self::Encode),
            "both" => Ok(Self::Both),
            other => Err(format!(
                "unknown --phase {other:?}; expected decode, encode, or both"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    SystemBinary,
    Jbig2enc,
    Jbig2dec,
    ItuT88,
    Java,
    Rust,
    All,
}

impl Target {
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "system-binary" => Ok(Self::SystemBinary),
            "jbig2enc" => Ok(Self::Jbig2enc),
            "jbig2dec" => Ok(Self::Jbig2dec),
            "itu-t88" => Ok(Self::ItuT88),
            "java" => Ok(Self::Java),
            "rust" => Ok(Self::Rust),
            "all" => Ok(Self::All),
            other => Err(format!("unknown --target {other:?}")),
        }
    }
}

#[derive(Clone, Debug)]
struct Args {
    phase: Phase,
    target: Target,
    no_color: bool,
    strict_mode: bool,
    self_check: bool,
    write_known_issues_doc: Option<PathBuf>,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut phase = Phase::Both;
        let mut target = Target::All;
        let mut no_color = false;
        let mut strict_mode = false;
        let mut self_check = false;
        let mut write_known_issues_doc = None;
        let mut iter = std::env::args().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--phase" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| "--phase requires a value".to_string())?;
                    phase = Phase::parse(&value)?;
                }
                "--target" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| "--target requires a value".to_string())?;
                    target = Target::parse(&value)?;
                }
                "--no-color" => no_color = true,
                "--strict" => strict_mode = true,
                "--self-check" => self_check = true,
                "--write-known-issues-doc" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| "--write-known-issues-doc requires a path".to_string())?;
                    write_known_issues_doc = Some(PathBuf::from(value));
                }
                "-h" | "--help" => {
                    println!(
                        "usage: conformance-matrix [--phase decode|encode|both] [--target system-binary|jbig2enc|jbig2dec|itu-t88|java|rust|all] [--no-color] [--strict] [--self-check] [--write-known-issues-doc PATH]"
                    );
                    std::process::exit(0);
                }
                other => return Err(format!("unexpected argument {other:?}")),
            }
        }

        match (phase, target) {
            (Phase::Encode, Target::Jbig2dec | Target::Java) => Err(format!(
                "--target {:?} has no encoder; use --phase decode",
                target
            )),
            (Phase::Decode, Target::Jbig2enc) => {
                Err("--target jbig2enc has no decoder; use --phase encode".to_string())
            }
            _ => Ok(Self {
                phase,
                target,
                no_color,
                strict_mode,
                self_check,
                write_known_issues_doc,
            }),
        }
    }
}

#[derive(Clone, Copy)]
struct ColorMode(bool);

impl ColorMode {
    fn new(no_color: bool) -> Self {
        Self(!no_color && std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal())
    }

    fn paint(self, text: &str, kind: CellKind) -> String {
        if !self.0 {
            return text.to_string();
        }
        match kind {
            CellKind::Ok => format!("\x1b[32m{text}\x1b[0m"),
            CellKind::KnownIssue => format!("\x1b[33m{text}\x1b[0m"),
            CellKind::Wontfix => format!("\x1b[1;35m{text}\x1b[0m"),
            CellKind::Resolved => format!("\x1b[36m{text}\x1b[0m"),
            CellKind::OurError => format!("\x1b[31m{text}\x1b[0m"),
            CellKind::ThirdPartyBreak => format!("\x1b[38;5;208m{text}\x1b[0m"),
            CellKind::Skip => format!("\x1b[90m{text}\x1b[0m"),
            CellKind::Lossy | CellKind::Blank => text.to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct KnownIssue {
    phase: PhaseLabel,
    row: String,
    column: String,
    expect: ExpectedOutcome,
    upstream: String,
    evidence: String,
    vendor: Option<VendorPin>,
    /// `true` when the upstream maintainer has explicitly declined to fix
    /// this defect (e.g. an Artifex bugzilla `RESOLVED WONTFIX`). Renders
    /// as `WTF` instead of `KI` in the summary, and requires the `evidence`
    /// field to cite the upstream WONTFIX record so the catalog stays
    /// auditable.
    #[serde(default)]
    wontfix: bool,
}

#[derive(Clone, Debug, Deserialize)]
enum ExpectedOutcome {
    Mismatch,
    FailContaining(String),
    FailMatchingAny(Vec<String>),
}

#[derive(Clone, Debug, Deserialize)]
enum VendorPin {
    GitSha { path: String, sha: String },
    FileSha256 { path: String, sha256: String },
}

#[derive(Clone, Debug)]
struct KnownIssues {
    entries: Vec<KnownIssue>,
    expanded: HashMap<IssueKey, usize>,
    vendor_statuses: Vec<Option<String>>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct IssueKey {
    phase: PhaseLabel,
    row: String,
    column: String,
}

impl KnownIssues {
    fn load(root: &Path) -> Result<Self, String> {
        let path = known_issues_path(root);
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("read known issues {:?}: {err}", path))?;
        let entries: Vec<KnownIssue> =
            ron::from_str(&text).map_err(|err| format!("parse known issues {:?}: {err}", path))?;
        let mut catalog = Self {
            entries,
            expanded: HashMap::new(),
            vendor_statuses: Vec::new(),
        };
        catalog.validate(root)?;
        eprintln!(
            "loaded {} known-issue entries from {}",
            catalog.entries.len(),
            path.strip_prefix(root).unwrap_or(&path).display()
        );
        Ok(catalog)
    }

    fn validate(&mut self, root: &Path) -> Result<(), String> {
        let decode_rows = decode_rows(Target::All)
            .into_iter()
            .map(|row| row.label.to_string())
            .collect::<HashSet<_>>();
        let decode_cols = decode_vectors(root)
            .into_iter()
            .map(|vector| vector.label.to_string())
            .collect::<Vec<_>>();
        let encode_rows = encode_rows(Target::All)
            .into_iter()
            .map(|row| row.label.to_string())
            .collect::<HashSet<_>>();
        let encode_cols = encode_sources(root)
            .into_iter()
            .map(|source| source.label.to_string())
            .collect::<Vec<_>>();

        let mut expanded = HashMap::new();
        let mut vendor_statuses = Vec::with_capacity(self.entries.len());

        for (idx, issue) in self.entries.iter().enumerate() {
            let entry_label = format!("entry #{idx} ({} / {})", issue.row, issue.column);
            if issue.upstream.trim().is_empty() {
                return Err(format!(
                    "known-issues {entry_label}: upstream must be non-empty"
                ));
            }
            if issue.evidence.trim().is_empty() {
                return Err(format!(
                    "known-issues {entry_label}: evidence must be non-empty"
                ));
            }
            if issue.row == "*" {
                return Err(format!(
                    "known-issues {entry_label}: row wildcard is not allowed"
                ));
            }
            if issue.evidence.contains("vendor/") && issue.vendor.is_none() {
                return Err(format!(
                    "known-issues {entry_label}: evidence cites vendor/ but no vendor pin is present"
                ));
            }
            if issue.wontfix
                && !issue.evidence.contains("http://")
                && !issue.evidence.contains("https://")
            {
                return Err(format!(
                    "known-issues {entry_label}: wontfix=true requires evidence to cite an upstream URL (e.g. a bugzilla WONTFIX record)"
                ));
            }

            let (valid_rows, valid_cols) = match issue.phase {
                PhaseLabel::Decode => (&decode_rows, &decode_cols),
                PhaseLabel::Encode => (&encode_rows, &encode_cols),
            };
            if !valid_rows.contains(&issue.row) {
                return Err(format!(
                    "known-issues {entry_label}: unknown row {:?} for {:?}",
                    issue.row, issue.phase
                ));
            }

            let concrete_cols = if issue.column == "*" {
                valid_cols.clone()
            } else {
                if !valid_cols.iter().any(|col| col == &issue.column) {
                    return Err(format!(
                        "known-issues {entry_label}: unknown column {:?} for {:?}",
                        issue.column, issue.phase
                    ));
                }
                vec![issue.column.clone()]
            };

            for column in concrete_cols {
                let key = IssueKey {
                    phase: issue.phase,
                    row: issue.row.clone(),
                    column,
                };
                if let Some(prev) = expanded.insert(key.clone(), idx) {
                    let prev_issue = &self.entries[prev];
                    return Err(format!(
                        "known-issues {entry_label}: overlaps entry #{prev} ({} / {}) after wildcard expansion at {:?} / {:?} / {:?}",
                        prev_issue.row, prev_issue.column, key.phase, key.row, key.column
                    ));
                }
            }

            vendor_statuses.push(issue.check_vendor_pin(root, idx));
        }

        self.expanded = expanded;
        self.vendor_statuses = vendor_statuses;
        Ok(())
    }

    fn lookup(&self, phase: PhaseLabel, row: &str, column: &str) -> Option<&KnownIssue> {
        let key = IssueKey {
            phase,
            row: row.to_string(),
            column: column.to_string(),
        };
        self.expanded
            .get(&key)
            .and_then(|idx| self.entries.get(*idx))
    }

    fn write_markdown_doc(&self, root: &Path, output: &Path) -> Result<(), String> {
        let output = if output.is_absolute() {
            output.to_path_buf()
        } else {
            root.join(output)
        };
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("create {parent:?}: {err}"))?;
        }

        let mut doc = String::new();
        doc.push_str("<!-- generated by conformance-matrix; do not edit by hand -->\n\n");
        doc.push_str("# Known Conformance Issues\n\n");
        doc.push_str(
            "This document is generated from `tools/conformance/known-issues.ron`. Edit the RON catalog, not this file.\n\n",
        );
        for phase in [PhaseLabel::Decode, PhaseLabel::Encode] {
            doc.push_str(&format!("## {}\n\n", phase.name()));
            for (idx, issue) in self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, issue)| issue.phase == phase)
            {
                doc.push_str(&format!("### `{}` / `{}`\n\n", issue.row, issue.column));
                let category = if issue.wontfix {
                    "WTF (upstream WONTFIX)"
                } else {
                    "KI (known issue)"
                };
                doc.push_str(&format!("- Category: {}\n", category));
                doc.push_str(&format!("- Expected: `{}`\n", issue.expect.describe()));
                doc.push_str(&format!("- Upstream: {}\n", issue.upstream));
                doc.push_str(&format!("- Evidence: {}\n", issue.evidence));
                if let Some(vendor) = &issue.vendor {
                    doc.push_str(&format!("- Vendor pin: `{}`\n", vendor.describe()));
                }
                if let Some(status) = self.vendor_statuses.get(idx).and_then(|s| s.as_ref()) {
                    doc.push_str(&format!("- Vendor pin status: {}\n", status));
                }
                doc.push('\n');
            }
        }
        fs::write(&output, doc).map_err(|err| format!("write generated doc {output:?}: {err}"))
    }
}

impl KnownIssue {
    fn matches(&self, cell: &Cell) -> bool {
        self.expect.matches(&cell.text)
    }

    fn check_vendor_pin(&self, root: &Path, idx: usize) -> Option<String> {
        let Some(vendor) = &self.vendor else {
            return None;
        };
        match vendor.check(root) {
            Ok(status) => Some(status),
            Err(err) => {
                eprintln!(
                    "warning: known-issues entry #{idx} vendor pin check failed for {} / {}: {err}",
                    self.row, self.column
                );
                Some(format!("warning: {err}"))
            }
        }
    }
}

impl ExpectedOutcome {
    fn matches(&self, actual: &str) -> bool {
        match self {
            Self::Mismatch => actual.starts_with("MISMATCH("),
            Self::FailContaining(needle) => actual.starts_with("FAIL(") && actual.contains(needle),
            Self::FailMatchingAny(needles) => {
                actual.starts_with("FAIL(") && needles.iter().any(|needle| actual.contains(needle))
            }
        }
    }

    fn describe(&self) -> String {
        match self {
            Self::Mismatch => "Mismatch".to_string(),
            Self::FailContaining(needle) => format!("FAIL containing {needle:?}"),
            Self::FailMatchingAny(needles) => {
                format!("FAIL containing any of [{}]", needles.join(", "))
            }
        }
    }
}

impl VendorPin {
    fn check(&self, root: &Path) -> Result<String, String> {
        match self {
            Self::GitSha { path, sha } => {
                let actual = git_head(root, path)?;
                if actual == *sha {
                    Ok(format!("ok ({path}@{})", short_sha(sha)))
                } else {
                    eprintln!(
                        "warning: vendor pin mismatch for {path}: catalog {}, actual {}",
                        short_sha(sha),
                        short_sha(&actual)
                    );
                    Ok(format!(
                        "mismatch (expected {}, actual {})",
                        short_sha(sha),
                        short_sha(&actual)
                    ))
                }
            }
            Self::FileSha256 { path, sha256 } => {
                let actual = file_sha256(&root.join(path))?;
                if actual == *sha256 {
                    Ok(format!("ok ({path}@{})", short_sha(sha256)))
                } else {
                    eprintln!(
                        "warning: file hash mismatch for {path}: catalog {}, actual {}",
                        short_sha(sha256),
                        short_sha(&actual)
                    );
                    Ok(format!(
                        "mismatch (expected {}, actual {})",
                        short_sha(sha256),
                        short_sha(&actual)
                    ))
                }
            }
        }
    }

    fn describe(&self) -> String {
        match self {
            Self::GitSha { path, sha } => format!("GitSha {{ path: {path}, sha: {sha} }}"),
            Self::FileSha256 { path, sha256 } => {
                format!("FileSha256 {{ path: {path}, sha256: {sha256} }}")
            }
        }
    }
}

fn known_issues_path(root: &Path) -> PathBuf {
    root.join("tools")
        .join("conformance")
        .join("known-issues.ron")
}

fn git_head(root: &Path, rel_path: &str) -> Result<String, String> {
    let path = root.join(rel_path);
    let output = Command::new("git")
        .arg("-C")
        .arg(&path)
        .arg("rev-parse")
        .arg("HEAD")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("spawn git for {rel_path}: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "git rev-parse failed for {rel_path}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let output = Command::new("shasum")
        .arg("-a")
        .arg("256")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .or_else(|_| {
            Command::new("sha256sum")
                .arg(path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        })
        .map_err(|err| format!("spawn sha256 tool for {path:?}: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "hash command failed for {path:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .split_whitespace()
        .next()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("hash command produced no digest for {path:?}"))
}

fn short_sha(sha: &str) -> String {
    sha.chars().take(12).collect()
}

struct Tools {
    system_jbig2enc: Option<PathBuf>,
    system_jbig2dec: Option<PathBuf>,
    vendor_jbig2enc: Option<PathBuf>,
    vendor_jbig2dec: Option<PathBuf>,
    itu_t88: Option<T88Tools>,
    java_cmd: Option<Vec<OsString>>,
}

#[derive(Clone)]
struct T88Tools {
    jbig2: PathBuf,
    imgcomp: PathBuf,
}

impl Tools {
    fn resolve(root: &Path, _workdir: &Path) -> Self {
        Self {
            system_jbig2enc: system_jbig2enc_bin(),
            system_jbig2dec: system_jbig2dec_bin(),
            vendor_jbig2enc: vendor_jbig2enc_bin(root),
            vendor_jbig2dec: vendor_jbig2dec_bin(root),
            itu_t88: t88_tools(root),
            java_cmd: java_cmd(root),
        }
    }
}

fn run_decode_matrix(
    root: &Path,
    workdir: &Path,
    tools: &Tools,
    target: Target,
) -> Result<Matrix, String> {
    let vectors = decode_vectors(root);
    validate_decode_oracles(root, &vectors)?;
    let rows = decode_rows(target);
    let mut oracle_cache = OracleCache::default();
    let mut matrix_rows = Vec::new();

    for row in rows {
        let mut cells = Vec::new();
        for vector in &vectors {
            let expected = oracle_cache.expected_pages(vector, workdir, tools)?;
            let cell = match expected {
                None => Cell::blank("no oracle"),
                Some(expected) => {
                    match decode_with(row.kind, vector, workdir, tools, expected.len()) {
                        DecodeAttempt::Pages(pages) => {
                            match compare_decode_pages(&pages, expected, vector.compare_prefix) {
                                Ok(()) => Cell::ok("OK"),
                                Err(err) => Cell::fail(
                                    row.is_ours,
                                    format!("MISMATCH({})", flatten_msg(&err)),
                                ),
                            }
                        }
                        DecodeAttempt::Skip(reason) => Cell::skip(reason),
                        DecodeAttempt::Fail(err) => {
                            Cell::fail(row.is_ours, format!("FAIL({})", flatten_msg(&err)))
                        }
                    }
                }
            };
            cells.push(cell);
        }
        matrix_rows.push(MatrixRow {
            label: row.label.to_string(),
            cells,
        });
    }

    Ok(Matrix {
        phase: PhaseLabel::Decode,
        subtitle: "10 TT vectors + annex-h; oracle = TT BMPs / spec-derived annex-h BMPs"
            .to_string(),
        columns: vectors.iter().map(|v| v.label.to_string()).collect(),
        rows: matrix_rows,
    })
}

fn run_encode_matrix(
    root: &Path,
    workdir: &Path,
    tools: &Tools,
    target: Target,
) -> Result<Matrix, String> {
    let Some(t88) = &tools.itu_t88 else {
        return Ok(Matrix {
            phase: PhaseLabel::Encode,
            subtitle: "oracle = itu-t88 jbig2 + imgcomp -m mse, vs source BMP".to_string(),
            columns: encode_sources(root)
                .iter()
                .map(|s| s.label.to_string())
                .collect(),
            rows: vec![MatrixRow {
                label: "encode oracle".to_string(),
                cells: encode_sources(root)
                    .iter()
                    .map(|_| Cell::skip("missing itu-t88 oracle"))
                    .collect(),
            }],
        });
    };

    let sources = encode_sources(root);
    let rows = encode_rows(target);
    let mut matrix_rows = Vec::new();

    for row in rows {
        let mut cells = Vec::new();
        for source in &sources {
            let cell = match should_skip_encode(row, source) {
                Some(reason) => Cell::blank(reason),
                None => match encode_with(row, source, workdir, tools) {
                    EncodeAttempt::Bytes(bytes) => {
                        match oracle_decode_and_compare(t88, &bytes, source, workdir, row.label) {
                            Ok(mse) if mse == 0.0 => Cell::ok("mse=0"),
                            Ok(mse) => Cell::lossy(format!("mse={}", format_mse(mse))),
                            Err(err) => {
                                Cell::fail(row.is_ours, format!("FAIL(dec: {})", flatten_msg(&err)))
                            }
                        }
                    }
                    EncodeAttempt::Skip(reason) => Cell::skip(reason),
                    EncodeAttempt::Fail(err) => {
                        Cell::fail(row.is_ours, format!("FAIL(enc: {})", flatten_msg(&err)))
                    }
                },
            };
            cells.push(cell);
        }
        matrix_rows.push(MatrixRow {
            label: row.label.to_string(),
            cells,
        });
    }

    Ok(Matrix {
        phase: PhaseLabel::Encode,
        subtitle: "oracle = itu-t88 jbig2 + imgcomp -m mse, vs source BMP".to_string(),
        columns: sources.iter().map(|s| s.label.to_string()).collect(),
        rows: matrix_rows,
    })
}

#[derive(Clone, Copy)]
struct DecodeRow {
    label: &'static str,
    kind: DecodeKind,
    is_ours: bool,
}

#[derive(Clone, Copy)]
enum DecodeKind {
    SystemBinary,
    Jbig2dec,
    ItuT88,
    Java,
    Rust,
}

fn decode_rows(target: Target) -> Vec<DecodeRow> {
    let all = [
        DecodeRow {
            label: "system-binary",
            kind: DecodeKind::SystemBinary,
            is_ours: false,
        },
        DecodeRow {
            label: "jbig2dec",
            kind: DecodeKind::Jbig2dec,
            is_ours: false,
        },
        DecodeRow {
            label: "itu-t88",
            kind: DecodeKind::ItuT88,
            is_ours: false,
        },
        DecodeRow {
            label: "java",
            kind: DecodeKind::Java,
            is_ours: false,
        },
        DecodeRow {
            label: "rust",
            kind: DecodeKind::Rust,
            is_ours: true,
        },
    ];
    all.into_iter()
        .filter(|row| match target {
            Target::All => true,
            Target::SystemBinary => matches!(row.kind, DecodeKind::SystemBinary),
            Target::Jbig2dec => matches!(row.kind, DecodeKind::Jbig2dec),
            Target::ItuT88 => matches!(row.kind, DecodeKind::ItuT88),
            Target::Java => matches!(row.kind, DecodeKind::Java),
            Target::Rust => matches!(row.kind, DecodeKind::Rust),
            Target::Jbig2enc => false,
        })
        .collect()
}

#[derive(Clone, Copy)]
struct EncodeRow {
    label: &'static str,
    kind: EncodeKind,
    is_ours: bool,
}

#[derive(Clone, Copy)]
enum EncodeKind {
    Rust(fn() -> EncoderConfig),
    SystemJbig2enc(&'static [&'static str]),
    VendorJbig2enc(&'static [&'static str]),
    ItuT88(Option<&'static str>),
}

fn encode_rows(target: Target) -> Vec<EncodeRow> {
    let rust = |label: &'static str, cfg: fn() -> EncoderConfig| EncodeRow {
        label,
        kind: EncodeKind::Rust(cfg),
        is_ours: true,
    };
    let system = |label: &'static str, args: &'static [&'static str]| EncodeRow {
        label,
        kind: EncodeKind::SystemJbig2enc(args),
        is_ours: false,
    };
    let vendor = |label: &'static str, args: &'static [&'static str]| EncodeRow {
        label,
        kind: EncodeKind::VendorJbig2enc(args),
        is_ours: false,
    };
    let t88 = |label: &'static str, ini: Option<&'static str>| EncodeRow {
        label,
        kind: EncodeKind::ItuT88(ini),
        is_ours: false,
    };
    let all = [
        rust("rust:fast", EncoderConfig::fast),
        rust("rust:balanced", EncoderConfig::balanced),
        rust("rust:max_compression", EncoderConfig::max_compression),
        rust("rust:generic_t0_no_tpgd", generic_t0_no_tpgd),
        rust("rust:generic_t0_tpgd", generic_t0_tpgd),
        rust("rust:symbol_lossy_t85", symbol_lossy_t85),
        system("system-binary:default", &[]),
        system("system-binary:-d", &["-d"]),
        system(
            "system-binary:-s -r -d -t 0.85",
            &["-s", "-r", "-d", "-t", "0.85"],
        ),
        system("system-binary:-s -d -t 0.85", &["-s", "-d", "-t", "0.85"]),
        vendor("jbig2enc:default", &[]),
        vendor("jbig2enc:-d", &["-d"]),
        vendor(
            "jbig2enc:-s -r -d -t 0.85",
            &["-s", "-r", "-d", "-t", "0.85"],
        ),
        vendor("jbig2enc:-s -d -t 0.85", &["-s", "-d", "-t", "0.85"]),
        t88("itu-t88:default", None),
        t88("itu-t88:Param2.ini", Some("jbig2_Param2.ini")),
        t88("itu-t88:Param3.ini", Some("jbig2_Param3.ini")),
        t88("itu-t88:Param4.ini", Some("jbig2_Param4.ini")),
        t88("itu-t88:Param5.ini", Some("jbig2_Param5.ini")),
        t88("itu-t88:Param6.ini", Some("jbig2_Param6.ini")),
        t88("itu-t88:Param7.ini", Some("jbig2_Param7.ini")),
        t88("itu-t88:Param8.ini", Some("jbig2_Param8.ini")),
        t88("itu-t88:Param9.ini", Some("jbig2_Param9.ini")),
    ];
    all.into_iter()
        .filter(|row| match target {
            Target::All => true,
            Target::Rust => matches!(row.kind, EncodeKind::Rust(_)),
            Target::SystemBinary => matches!(row.kind, EncodeKind::SystemJbig2enc(_)),
            Target::Jbig2enc => matches!(row.kind, EncodeKind::VendorJbig2enc(_)),
            Target::ItuT88 => matches!(row.kind, EncodeKind::ItuT88(_)),
            Target::Jbig2dec | Target::Java => false,
        })
        .collect()
}

#[derive(Clone)]
struct DecodeVector {
    label: &'static str,
    path: PathBuf,
    oracle: OracleKind,
    compare_prefix: bool,
}

#[derive(Clone)]
enum OracleKind {
    Files(Vec<&'static str>),
    SpecFiles(Vec<&'static str>),
}

fn decode_vectors(root: &Path) -> Vec<DecodeVector> {
    let conformance = conformance_dir(root);
    vec![
        DecodeVector {
            label: "TT1",
            path: conformance.join("codeStreamTest1_TT1.jb2"),
            oracle: OracleKind::Files(vec![
                "codeStreamTest1_TT1_TT00.bmp",
                "codeStreamTest1_TT1_TT01.bmp",
                "codeStreamTest1_TT1_TT02.bmp",
            ]),
            compare_prefix: false,
        },
        tt(
            "TT2",
            "codeStreamTest1_TT2.jb2",
            "codeStreamTest1_TT2_TT00.bmp",
            root,
        ),
        tt(
            "TT3",
            "codeStreamTest1_TT3.jb2",
            "codeStreamTest1_TT3_TT00.bmp",
            root,
        ),
        tt(
            "TT4",
            "codeStreamTest1_TT4.jb2",
            "codeStreamTest1_TT4_TT00.bmp",
            root,
        ),
        tt(
            "TT5",
            "codeStreamTest1_TT5.jb2",
            "codeStreamTest1_TT5_TT00.bmp",
            root,
        ),
        tt(
            "TT6",
            "codeStreamTest2_TT6.jb2",
            "codeStreamTest2_TT6_TT00.bmp",
            root,
        ),
        tt(
            "TT7",
            "codeStreamTest1_TT7.jb2",
            "codeStreamTest1_TT7_TT00.bmp",
            root,
        ),
        tt(
            "TT8",
            "codeStreamTest3_TT8.jb2",
            "codeStreamTest3_TT8_TT00.bmp",
            root,
        ),
        tt("TT9", "F01_200_TT9.jb2", "F01_200_TT9_TT00.bmp", root),
        tt("TT10", "F01_200_TT10.jb2", "F01_200_TT10_TT00.bmp", root),
        DecodeVector {
            label: "annex-h",
            path: root.join("vendor").join("jbig2dec").join("annex-h.jbig2"),
            oracle: OracleKind::SpecFiles(vec!["annex-h-page-00.bmp", "annex-h-page-01.bmp"]),
            compare_prefix: true,
        },
    ]
}

fn tt(label: &'static str, jb2: &'static str, bmp: &'static str, root: &Path) -> DecodeVector {
    DecodeVector {
        label,
        path: conformance_dir(root).join(jb2),
        oracle: OracleKind::Files(vec![bmp]),
        compare_prefix: false,
    }
}

fn oracle_file_path(root: &Path, kind: &OracleKind, name: &str) -> PathBuf {
    match kind {
        OracleKind::Files(_) => conformance_dir(root).join(name),
        OracleKind::SpecFiles(_) => t88_spec_dir().join(name),
    }
}

fn validate_decode_oracles(root: &Path, vectors: &[DecodeVector]) -> Result<(), String> {
    for vector in vectors {
        let files = match &vector.oracle {
            OracleKind::Files(files) | OracleKind::SpecFiles(files) => files,
        };
        for name in files {
            let path = oracle_file_path(root, &vector.oracle, name);
            if !path.is_file() {
                eprintln!(
                    "conformance-matrix: missing decode oracle {}",
                    path.display()
                );
                return Err(format!("missing decode oracle {}", path.display()));
            }
            validate_bmp_1bpp_fingerprint(&path)?;
        }
    }
    Ok(())
}

#[derive(Clone)]
struct EncodeSource {
    label: &'static str,
    path: PathBuf,
    is_color: bool,
}

fn encode_sources(root: &Path) -> Vec<EncodeSource> {
    let dir = conformance_dir(root);
    vec![
        source("codeStreamTest1", dir.join("codeStreamTest1.bmp"), false),
        source("codeStreamTest2", dir.join("codeStreamTest2.bmp"), false),
        source("codeStreamTest3", dir.join("codeStreamTest3.bmp"), true),
        source("F01_200", dir.join("F01_200.bmp"), false),
    ]
}

fn source(label: &'static str, path: PathBuf, is_color: bool) -> EncodeSource {
    EncodeSource {
        label,
        path,
        is_color,
    }
}

fn conformance_dir(root: &Path) -> PathBuf {
    root.join("vendor")
        .join("T-REC-T.88-201808")
        .join("Software")
        .join("JBIG2_ConformanceData-A20180829")
}

fn t88_spec_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("vendor")
        .join("T-REC-T.88-201808")
        .join("spec")
}

#[derive(Default)]
struct OracleCache;

impl OracleCache {
    fn expected_pages(
        &mut self,
        vector: &DecodeVector,
        _workdir: &Path,
        _tools: &Tools,
    ) -> Result<Option<&[PageImage]>, String> {
        match &vector.oracle {
            OracleKind::Files(files) => {
                let pages = files
                    .iter()
                    .map(|name| {
                        let path = oracle_file_path(
                            Path::new(env!("CARGO_MANIFEST_DIR")),
                            &vector.oracle,
                            name,
                        );
                        if !path.is_file() {
                            eprintln!(
                                "conformance-matrix: missing decode oracle {}",
                                path.display()
                            );
                        }
                        load_bmp_image(&path)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Some(Box::leak(pages.into_boxed_slice())))
            }
            OracleKind::SpecFiles(files) => {
                let pages = files
                    .iter()
                    .map(|name| {
                        let path = oracle_file_path(
                            Path::new(env!("CARGO_MANIFEST_DIR")),
                            &vector.oracle,
                            name,
                        );
                        if !path.is_file() {
                            eprintln!(
                                "conformance-matrix: missing decode oracle {}",
                                path.display()
                            );
                        }
                        load_bmp_image(&path)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Some(Box::leak(pages.into_boxed_slice())))
            }
        }
    }
}

fn run_self_check(workdir: &Path, tools: &Tools) -> Result<(), String> {
    let source = synthetic_self_check_bitmap()?;
    let mut bytes = Vec::new();
    let mut encoder = Jbig2Encoder::new(&mut bytes, EncoderConfig::fast());
    encoder
        .write_page(&source)
        .map_err(|err| format!("self-check encode page: {err}"))?;
    encoder
        .finish()
        .map_err(|err| format!("self-check finish: {err}"))?;

    let input = workdir.join("self-check.jb2");
    fs::write(&input, &bytes).map_err(|err| format!("write {input:?}: {err}"))?;

    let rust_pages = decode_with_rust(&input, 1)?;
    compare_page(
        rust_pages
            .first()
            .ok_or_else(|| "rust self-check produced no pages".to_string())?,
        &PageImage::Mono(source.clone()),
    )
    .map_err(|err| format!("rust self-check mismatch: {err}"))?;

    if let Some(bin) = tools
        .vendor_jbig2dec
        .as_deref()
        .or(tools.system_jbig2dec.as_deref())
    {
        let pages = decode_with_jbig2dec_bin(bin, &input, &workdir.join("self-check.pbm"))?;
        compare_page(
            pages
                .first()
                .ok_or_else(|| "jbig2dec self-check produced no pages".to_string())?,
            &PageImage::Mono(source),
        )
        .map_err(|err| format!("jbig2dec self-check mismatch: {err}"))?;
        println!("SELF-CHECK: rust and jbig2dec decoders matched synthetic page");
    } else {
        println!("SELF-CHECK: rust decoder matched synthetic page; jbig2dec unavailable");
    }

    Ok(())
}

fn synthetic_self_check_bitmap() -> Result<Bitmap, String> {
    let mut bm = Bitmap::new(64, 32).map_err(|err| err.to_string())?;
    for y in 0..32 {
        for x in 0..64 {
            if (x + y) % 11 == 0 || (x >= 7 && x < 25 && y >= 9 && y < 18) {
                bm.set_pixel(x, y, 1);
            }
        }
    }
    Ok(bm)
}

enum DecodeAttempt {
    Pages(Vec<PageImage>),
    Skip(&'static str),
    Fail(String),
}

fn decode_with(
    kind: DecodeKind,
    vector: &DecodeVector,
    workdir: &Path,
    tools: &Tools,
    expected_pages: usize,
) -> DecodeAttempt {
    match kind {
        DecodeKind::SystemBinary => {
            let Some(bin) = tools.system_jbig2dec.as_deref() else {
                return DecodeAttempt::Skip("no binary");
            };
            decode_with_jbig2dec_bin(
                bin,
                &vector.path,
                &workdir.join(format!("{}-system.pbm", vector.label)),
            )
            .map(DecodeAttempt::Pages)
            .unwrap_or_else(DecodeAttempt::Fail)
        }
        DecodeKind::Jbig2dec => {
            let Some(bin) = tools.vendor_jbig2dec.as_deref() else {
                return DecodeAttempt::Skip("no binary");
            };
            decode_with_jbig2dec_bin(
                bin,
                &vector.path,
                &workdir.join(format!("{}-jbig2dec.pbm", vector.label)),
            )
            .map(DecodeAttempt::Pages)
            .unwrap_or_else(DecodeAttempt::Fail)
        }
        DecodeKind::ItuT88 => {
            let Some(t88) = tools.itu_t88.as_ref() else {
                return DecodeAttempt::Skip("no binary");
            };
            decode_with_t88(
                t88,
                &vector.path,
                &workdir.join(format!("t88-{}", vector.label)),
                expected_pages,
            )
            .map(DecodeAttempt::Pages)
            .unwrap_or_else(DecodeAttempt::Fail)
        }
        DecodeKind::Java => {
            let Some(cmd) = tools.java_cmd.as_deref() else {
                return DecodeAttempt::Skip("no binary");
            };
            match decode_with_java(
                cmd,
                &vector.path,
                &workdir.join(format!("{}-java.pbm", vector.label)),
            ) {
                Ok(pages) => {
                    if !vector.compare_prefix && pages.len() != expected_pages {
                        DecodeAttempt::Skip("page count mismatch")
                    } else {
                        DecodeAttempt::Pages(pages.into_iter().map(PageImage::Mono).collect())
                    }
                }
                Err(err) => DecodeAttempt::Fail(err),
            }
        }
        DecodeKind::Rust => decode_with_rust(&vector.path, expected_pages)
            .map(DecodeAttempt::Pages)
            .unwrap_or_else(DecodeAttempt::Fail),
    }
}

enum EncodeAttempt {
    Bytes(Vec<u8>),
    Skip(&'static str),
    Fail(String),
}

fn should_skip_encode(row: EncodeRow, source: &EncodeSource) -> Option<&'static str> {
    match row.kind {
        EncodeKind::Rust(_) | EncodeKind::SystemJbig2enc(_) | EncodeKind::VendorJbig2enc(_) => {
            if source.is_color {
                Some("color")
            } else {
                None
            }
        }
        EncodeKind::ItuT88(Some(ini)) => ini_source(ini).and_then(|expected| {
            if expected == source.label {
                None
            } else {
                Some("ini scoped")
            }
        }),
        EncodeKind::ItuT88(None) => None,
    }
}

fn ini_source(ini: &str) -> Option<&'static str> {
    match ini {
        "jbig2_Param2.ini" | "jbig2_Param3.ini" | "jbig2_Param4.ini" | "jbig2_Param5.ini"
        | "jbig2_Param7.ini" => Some("codeStreamTest1"),
        "jbig2_Param6.ini" => Some("codeStreamTest2"),
        "jbig2_Param8.ini" => Some("codeStreamTest3"),
        "jbig2_Param9.ini" => Some("F01_200"),
        _ => None,
    }
}

fn encode_with(
    row: EncodeRow,
    source: &EncodeSource,
    workdir: &Path,
    tools: &Tools,
) -> EncodeAttempt {
    match row.kind {
        EncodeKind::Rust(config) => encode_rust(source, config()).map(EncodeAttempt::Bytes),
        EncodeKind::SystemJbig2enc(args) => {
            let Some(bin) = tools.system_jbig2enc.as_deref() else {
                return EncodeAttempt::Skip("no binary");
            };
            encode_jbig2enc(bin, args, source, &workdir.join("encode-system"))
                .map(EncodeAttempt::Bytes)
        }
        EncodeKind::VendorJbig2enc(args) => {
            let Some(bin) = tools.vendor_jbig2enc.as_deref() else {
                return EncodeAttempt::Skip("no binary");
            };
            encode_jbig2enc(bin, args, source, &workdir.join("encode-vendor"))
                .map(EncodeAttempt::Bytes)
        }
        EncodeKind::ItuT88(ini) => {
            let Some(t88) = tools.itu_t88.as_ref() else {
                return EncodeAttempt::Skip("no binary");
            };
            encode_t88(t88, ini, source, &workdir.join("encode-t88")).map(EncodeAttempt::Bytes)
        }
    }
    .unwrap_or_else(EncodeAttempt::Fail)
}

fn encode_rust(source: &EncodeSource, cfg: EncoderConfig) -> Result<Vec<u8>, String> {
    let image = load_bmp_image(&source.path)?;
    let PageImage::Mono(bitmap) = image else {
        return Err("rust encoder input is not mono".to_string());
    };
    let mut out = Vec::new();
    let mut enc = Jbig2Encoder::new(&mut out, cfg);
    enc.write_page(&bitmap)
        .map_err(|err| format!("write page: {err}"))?;
    enc.finish().map_err(|err| format!("finish: {err}"))?;
    Ok(out)
}

fn encode_jbig2enc(
    bin: &Path,
    args: &[&str],
    source: &EncodeSource,
    workdir: &Path,
) -> Result<Vec<u8>, String> {
    fs::create_dir_all(workdir).map_err(|err| format!("create {workdir:?}: {err}"))?;
    let out = workdir.join(format!("{}.jb2", safe_name(source.label)));
    let file = File::create(&out).map_err(|err| format!("create {out:?}: {err}"))?;
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .arg(&source.path)
        .stdout(Stdio::from(file))
        .stderr(Stdio::piped());
    run_output(cmd, "jbig2enc encode")?;
    fs::read(&out).map_err(|err| format!("read {out:?}: {err}"))
}

fn encode_t88(
    tools: &T88Tools,
    ini: Option<&str>,
    source: &EncodeSource,
    workdir: &Path,
) -> Result<Vec<u8>, String> {
    fs::create_dir_all(workdir).map_err(|err| format!("create {workdir:?}: {err}"))?;
    let input_stem = strip_ext(&source.path)?;
    let out_stem = workdir.join(format!(
        "{}-{}",
        safe_name(source.label),
        ini.unwrap_or("default")
    ));
    let mut cmd = Command::new(&tools.jbig2);
    cmd.arg("-i")
        .arg(input_stem)
        .arg("-f")
        .arg("bmp")
        .arg("-o")
        .arg(&out_stem)
        .arg("-F")
        .arg("jb2")
        .current_dir(workdir)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());
    if let Some(ini) = ini {
        cmd.arg("-ini").arg(t88_test_dir().join(ini));
    }
    run_output(cmd, "itu-t88 encode")?;
    let out = append_extension(&out_stem, "jb2")?;
    fs::read(&out).map_err(|err| format!("read {out:?}: {err}"))
}

fn oracle_decode_and_compare(
    tools: &T88Tools,
    bytes: &[u8],
    source: &EncodeSource,
    workdir: &Path,
    label: &str,
) -> Result<f64, String> {
    let dir = workdir.join("oracle").join(safe_name(label));
    reset_dir(&dir)?;
    let input_stem = dir.join("input");
    let input = input_stem.with_extension("jb2");
    fs::write(&input, bytes).map_err(|err| format!("write oracle input {input:?}: {err}"))?;
    let decoded_stem = dir.join("decoded");
    let mut dec = Command::new(&tools.jbig2);
    dec.arg("-i")
        .arg(&input_stem)
        .arg("-f")
        .arg("jb2")
        .arg("-o")
        .arg(&decoded_stem)
        .arg("-F")
        .arg("bmp")
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run_output(dec, "itu-t88 oracle decode")?;

    let source_stem = strip_ext(&source.path)?;
    let target_stem = decoded_stem.with_file_name("decoded00");
    let mut cmp = Command::new(&tools.imgcomp);
    cmp.arg("-t")
        .arg(source_stem)
        .arg("-f")
        .arg("bmp")
        .arg("-T")
        .arg(&target_stem)
        .arg("-F")
        .arg("bmp")
        .arg("-m")
        .arg("mse")
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = run_output(cmp, "itu-t88 imgcomp")?;
    parse_distortion(&String::from_utf8_lossy(&output.stdout))
}

fn parse_distortion(stdout: &str) -> Result<f64, String> {
    let marker = "Distortion=";
    let Some(start) = stdout.find(marker) else {
        return Err(format!("imgcomp output missing Distortion: {stdout:?}"));
    };
    let rest = &stdout[start + marker.len()..];
    let end = rest.find(',').unwrap_or(rest.len());
    rest[..end]
        .trim()
        .parse::<f64>()
        .map_err(|err| format!("parse Distortion from {stdout:?}: {err}"))
}

fn decode_with_rust(path: &Path, expected_pages: usize) -> Result<Vec<PageImage>, String> {
    let data = fs::read(path).map_err(|err| format!("read {path:?}: {err}"))?;
    let mut dec = Jbig2Decoder::new(Cursor::new(data)).map_err(|err| format!("parse: {err}"))?;
    let mut pages = Vec::new();
    for page_no in 1..=expected_pages as u32 {
        let page = dec
            .decode_page(page_no)
            .map_err(|err| format!("decode page {page_no}: {err}"))?;
        if let Some(rgb) = page.rgb_bitmap {
            pages.push(PageImage::Rgb(rgb));
        } else {
            pages.push(PageImage::Mono(page.bitmap));
        }
    }
    Ok(pages)
}

fn decode_with_jbig2dec_bin(
    bin: &Path,
    input: &Path,
    output: &Path,
) -> Result<Vec<PageImage>, String> {
    let mut cmd = Command::new(bin);
    cmd.arg("--format")
        .arg("pbm")
        .arg("-o")
        .arg(output)
        .arg(input)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = run_output(cmd, "jbig2dec decode")?;
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if diagnostics.contains("FATAL ERROR") || diagnostics.contains("failed to decode") {
        return Err(format!(
            "jbig2dec diagnostics: {}",
            diagnostics.split_whitespace().collect::<Vec<_>>().join(" ")
        ));
    }
    parse_pbm_sequence(output).map(|pages| pages.into_iter().map(PageImage::Mono).collect())
}

fn decode_with_java(cmd: &[OsString], input: &Path, output: &Path) -> Result<Vec<Bitmap>, String> {
    let Some(program) = cmd.first() else {
        return Err("empty java command".to_string());
    };
    let mut command = Command::new(program);
    command
        .args(&cmd[1..])
        .arg(input)
        .arg(output)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run_output(command, "jbig2-imageio decode")?;
    parse_pbm_sequence(output)
}

fn decode_with_t88(
    tools: &T88Tools,
    input: &Path,
    out_dir: &Path,
    expected_pages: usize,
) -> Result<Vec<PageImage>, String> {
    reset_dir(out_dir)?;
    let input_stem = copy_as_jb2_stem(input, out_dir)?;
    let out_stem = out_dir.join("page");
    let mut cmd = Command::new(&tools.jbig2);
    cmd.arg("-i")
        .arg(&input_stem)
        .arg("-f")
        .arg("jb2")
        .arg("-o")
        .arg(&out_stem)
        .arg("-F")
        .arg("bmp")
        .current_dir(out_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run_output(cmd, "itu-t88 decode")?;
    let mut pages = Vec::new();
    for idx in 0..expected_pages {
        let bmp = out_dir.join(format!("page{idx:02}.bmp"));
        pages.push(load_bmp_image(&bmp)?);
    }
    Ok(pages)
}

fn copy_as_jb2_stem(input: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    let stem = out_dir.join("input");
    fs::copy(input, stem.with_extension("jb2"))
        .map_err(|err| format!("copy {input:?} to t88 input: {err}"))?;
    Ok(stem)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PageImage {
    Mono(Bitmap),
    Rgb(RgbBitmap),
}

fn compare_decode_pages(
    actual: &[PageImage],
    expected: &[PageImage],
    compare_prefix: bool,
) -> Result<(), String> {
    if compare_prefix {
        compare_page_prefix(actual, expected)
    } else {
        compare_page_sets(actual, expected)
    }
}

fn compare_page_prefix(actual: &[PageImage], expected: &[PageImage]) -> Result<(), String> {
    if actual.len() < expected.len() {
        return Err(format!(
            "page count {} vs at least {}",
            actual.len(),
            expected.len()
        ));
    }
    for (idx, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        compare_page(actual, expected).map_err(|err| {
            if idx == 0 {
                err
            } else {
                format!("p{idx}:{err}")
            }
        })?;
    }
    Ok(())
}

fn compare_page_sets(actual: &[PageImage], expected: &[PageImage]) -> Result<(), String> {
    if actual.len() != expected.len() {
        return Err(format!("page count {} vs {}", actual.len(), expected.len()));
    }
    for (idx, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        compare_page(actual, expected).map_err(|err| {
            if idx == 0 {
                err
            } else {
                format!("p{idx}:{err}")
            }
        })?;
    }
    Ok(())
}

fn compare_page(actual: &PageImage, expected: &PageImage) -> Result<(), String> {
    match (actual, expected) {
        (PageImage::Mono(a), PageImage::Mono(e)) => compare_mono(a, e),
        (PageImage::Rgb(a), PageImage::Rgb(e)) => compare_rgb(a, e),
        (PageImage::Mono(a), PageImage::Rgb(e)) => compare_mono(a, &rgb_to_mono(e)),
        (PageImage::Rgb(a), PageImage::Mono(e)) => compare_mono(&rgb_to_mono(a), e),
    }
}

fn compare_mono(actual: &Bitmap, expected: &Bitmap) -> Result<(), String> {
    if actual.width() != expected.width() {
        return Err(format!("width {} vs {}", actual.width(), expected.width()));
    }
    if actual.height() != expected.height() {
        return Err(format!(
            "height {} vs {}",
            actual.height(),
            expected.height()
        ));
    }
    for y in 0..actual.height() as usize {
        if actual.row(y) != expected.row(y) {
            return Err(format!("row {y}"));
        }
    }
    Ok(())
}

fn compare_rgb(actual: &RgbBitmap, expected: &RgbBitmap) -> Result<(), String> {
    if actual.width() != expected.width() {
        return Err(format!("width {} vs {}", actual.width(), expected.width()));
    }
    if actual.height() != expected.height() {
        return Err(format!(
            "height {} vs {}",
            actual.height(),
            expected.height()
        ));
    }
    for y in 0..actual.height() as usize {
        if actual.row(y) != expected.row(y) {
            return Err(format!("row {y}"));
        }
    }
    Ok(())
}

fn rgb_to_mono(rgb: &RgbBitmap) -> Bitmap {
    let mut bm = Bitmap::new(rgb.width(), rgb.height()).expect("rgb dimensions valid");
    for y in 0..rgb.height() as usize {
        let row = rgb.row(y);
        for x in 0..rgb.width() as usize {
            let r = row[x * 3] as u32;
            let g = row[x * 3 + 1] as u32;
            let b = row[x * 3 + 2] as u32;
            let luma = (r * 299 + g * 587 + b * 114) / 1000;
            if luma < 128 {
                bm.set_pixel(x as i32, y as i32, 1);
            }
        }
    }
    bm
}

fn load_bmp_image(path: &Path) -> Result<PageImage, String> {
    let data = fs::read(path).map_err(|err| format!("read {path:?}: {err}"))?;
    if data.len() < 54 || &data[0..2] != b"BM" {
        return Err(format!("{path:?}: not a BMP"));
    }
    let bpp = u16::from_le_bytes(data[28..30].try_into().unwrap());
    match bpp {
        1 => Ok(PageImage::Mono(parse_bmp_1bpp(&data)?)),
        24 => Ok(PageImage::Rgb(parse_bmp_24bpp(&data)?)),
        other => Err(format!("{path:?}: unsupported BMP bpp {other}")),
    }
}

fn validate_bmp_1bpp_fingerprint(path: &Path) -> Result<(), String> {
    let data = fs::read(path).map_err(|err| format!("read {path:?}: {err}"))?;
    if data.len() < 62 || &data[0..2] != b"BM" {
        return Err(format!("{path:?}: not a BMP"));
    }
    let bpp = u16::from_le_bytes(data[28..30].try_into().unwrap());
    if bpp != 1 {
        return Ok(());
    }
    let pal0 = &data[54..58];
    let pal1 = &data[58..62];
    if pal0 != [0xff, 0xff, 0xff, 0x00].as_slice() || pal1 != [0x00, 0x00, 0x00, 0x00].as_slice() {
        return Err(format!(
            "{path:?}: unexpected 1-bpp BMP palette pal0={:02x?} pal1={:02x?}",
            pal0, pal1
        ));
    }
    let bm = parse_bmp_1bpp(&data)?;
    let top_left_is_ink = bm.row(0)[0] & 0x80 != 0;
    if top_left_is_ink {
        return Err(format!(
            "{path:?}: unexpected black sentinel pixel at (0,0); oracle polarity may have drifted"
        ));
    }
    Ok(())
}

fn parse_bmp_1bpp(data: &[u8]) -> Result<Bitmap, String> {
    let pixel_offset = u32::from_le_bytes(data[10..14].try_into().unwrap()) as usize;
    let dib_size = u32::from_le_bytes(data[14..18].try_into().unwrap()) as usize;
    if dib_size < 40 {
        return Err("not a BITMAPINFOHEADER".to_string());
    }
    let width = i32::from_le_bytes(data[18..22].try_into().unwrap());
    let height_signed = i32::from_le_bytes(data[22..26].try_into().unwrap());
    let top_down = height_signed < 0;
    let height = height_signed.unsigned_abs();
    let width_u = width as u32;
    let pal0 = &data[54..58];
    let zero_is_ink = pal0[0] <= 0x40 && pal0[1] <= 0x40 && pal0[2] <= 0x40;
    let row_bytes = width_u.div_ceil(32) as usize * 4;
    let stride = width_u.div_ceil(8) as usize;
    let mut bm = Bitmap::new(width_u, height).map_err(|err| err.to_string())?;
    for y in 0..height {
        let src_y = if top_down { y } else { height - 1 - y };
        let row_start = pixel_offset + src_y as usize * row_bytes;
        let src = &data[row_start..row_start + stride];
        let row = bm.row_mut(y as usize);
        if zero_is_ink {
            for (d, s) in row.iter_mut().zip(src) {
                *d = !*s;
            }
        } else {
            row.copy_from_slice(src);
        }
        let last_bits = width_u & 7;
        if last_bits != 0 {
            let mask = 0xFFu8 << (8 - last_bits);
            let last = row.len() - 1;
            row[last] &= mask;
        }
    }
    Ok(bm)
}

fn parse_bmp_24bpp(data: &[u8]) -> Result<RgbBitmap, String> {
    let pixel_offset = u32::from_le_bytes(data[10..14].try_into().unwrap()) as usize;
    let dib_size = u32::from_le_bytes(data[14..18].try_into().unwrap()) as usize;
    if dib_size < 40 {
        return Err("not a BITMAPINFOHEADER".to_string());
    }
    let width = i32::from_le_bytes(data[18..22].try_into().unwrap());
    let height_signed = i32::from_le_bytes(data[22..26].try_into().unwrap());
    let top_down = height_signed < 0;
    let height = height_signed.unsigned_abs();
    let width_u = width as u32;
    let row_bytes = ((width_u as usize * 3) + 3) & !3;
    let mut bm = RgbBitmap::filled(width_u, height, [255, 255, 255]).map_err(|e| e.to_string())?;
    for y in 0..height {
        let src_y = if top_down { y } else { height - 1 - y };
        let row_start = pixel_offset + src_y as usize * row_bytes;
        let src = &data[row_start..row_start + width_u as usize * 3];
        let dst = bm.row_mut(y as usize);
        for x in 0..width_u as usize {
            let b = src[x * 3];
            let g = src[x * 3 + 1];
            let r = src[x * 3 + 2];
            dst[x * 3..x * 3 + 3].copy_from_slice(&[r, g, b]);
        }
    }
    Ok(bm)
}

fn parse_pbm_sequence(path: &Path) -> Result<Vec<Bitmap>, String> {
    let data = fs::read(path).map_err(|err| format!("read {path:?}: {err}"))?;
    let mut cursor = 0usize;
    let mut pages = Vec::new();
    skip_ws_and_comments(&data, &mut cursor);
    while cursor < data.len() {
        pages.push(parse_pbm_at(&data, &mut cursor)?);
        skip_ws_and_comments(&data, &mut cursor);
    }
    if pages.is_empty() {
        return Err(format!("{path:?}: no PBM pages"));
    }
    Ok(pages)
}

fn parse_pbm_at(data: &[u8], cursor: &mut usize) -> Result<Bitmap, String> {
    if data.len().saturating_sub(*cursor) < 3 || &data[*cursor..*cursor + 2] != b"P4" {
        return Err("not a P4 PBM file".to_string());
    }
    *cursor += 2;
    let width = read_pbm_u32(data, cursor)?;
    let height = read_pbm_u32(data, cursor)?;
    if *cursor >= data.len() || !data[*cursor].is_ascii_whitespace() {
        return Err("pbm truncated before raster".to_string());
    }
    *cursor += 1;
    let stride = width.div_ceil(8) as usize;
    let needed = stride * height as usize;
    if data.len().saturating_sub(*cursor) < needed {
        return Err(format!("pbm raster shorter than declared {width}x{height}"));
    }
    let mut bm = Bitmap::new(width, height).map_err(|err| err.to_string())?;
    for y in 0..height as usize {
        let src = &data[*cursor + y * stride..*cursor + (y + 1) * stride];
        bm.row_mut(y).copy_from_slice(src);
        let last_bits = width & 7;
        if last_bits != 0 {
            let mask = 0xFFu8 << (8 - last_bits);
            let last = bm.row(y).len() - 1;
            bm.row_mut(y)[last] &= mask;
        }
    }
    *cursor += needed;
    Ok(bm)
}

fn skip_ws_and_comments(data: &[u8], cursor: &mut usize) {
    while *cursor < data.len() {
        match data[*cursor] {
            b' ' | b'\t' | b'\n' | b'\r' => *cursor += 1,
            b'#' => {
                while *cursor < data.len() && data[*cursor] != b'\n' {
                    *cursor += 1;
                }
            }
            _ => break,
        }
    }
}

fn read_pbm_u32(data: &[u8], cursor: &mut usize) -> Result<u32, String> {
    skip_ws_and_comments(data, cursor);
    let start = *cursor;
    while *cursor < data.len() && data[*cursor].is_ascii_digit() {
        *cursor += 1;
    }
    let s = std::str::from_utf8(&data[start..*cursor]).map_err(|err| err.to_string())?;
    s.parse::<u32>()
        .map_err(|err| format!("pbm dimension parse: {err}"))
}

fn generic_t0_no_tpgd() -> EncoderConfig {
    EncoderConfig {
        mode: Mode::Generic,
        template: GenericTemplate::T0,
        coding: Coding::Arithmetic,
        adaptive_templates: None,
        refinement: false,
        generic_region_duplicate_line_removal: false,
        symbol_threshold: 0.97,
        refine_after_match: false,
        ..EncoderConfig::fast()
    }
}

fn generic_t0_tpgd() -> EncoderConfig {
    EncoderConfig {
        generic_region_duplicate_line_removal: true,
        ..generic_t0_no_tpgd()
    }
}

fn symbol_lossy_t85() -> EncoderConfig {
    EncoderConfig {
        mode: Mode::SymbolLossy,
        template: GenericTemplate::T0,
        coding: Coding::Arithmetic,
        adaptive_templates: None,
        refinement: false,
        generic_region_duplicate_line_removal: true,
        symbol_threshold: 0.85,
        refine_after_match: false,
        ..EncoderConfig::fast()
    }
}

#[derive(Clone)]
struct Matrix {
    phase: PhaseLabel,
    subtitle: String,
    columns: Vec<String>,
    rows: Vec<MatrixRow>,
}

#[derive(Clone, Copy, Debug, Deserialize, Hash, PartialEq, Eq)]
enum PhaseLabel {
    Decode,
    Encode,
}

impl PhaseLabel {
    fn name(self) -> &'static str {
        match self {
            Self::Decode => "DECODE",
            Self::Encode => "ENCODE",
        }
    }
}

#[derive(Clone)]
struct MatrixRow {
    label: String,
    cells: Vec<Cell>,
}

#[derive(Clone)]
struct Cell {
    text: String,
    kind: CellKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CellKind {
    Ok,
    Lossy,
    KnownIssue,
    Wontfix,
    Resolved,
    OurError,
    ThirdPartyBreak,
    Skip,
    Blank,
}

impl Cell {
    fn ok(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: CellKind::Ok,
        }
    }

    fn lossy(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: CellKind::Lossy,
        }
    }

    fn fail(is_ours: bool, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: if is_ours {
                CellKind::OurError
            } else {
                CellKind::ThirdPartyBreak
            },
        }
    }

    fn skip(reason: impl Into<String>) -> Self {
        Self {
            text: format!("SKIP({})", reason.into()),
            kind: CellKind::Skip,
        }
    }

    fn blank(_reason: &'static str) -> Self {
        Self {
            text: String::new(),
            kind: CellKind::Blank,
        }
    }
}

fn render_matrix(
    matrix: &Matrix,
    known_issues: &KnownIssues,
    color: ColorMode,
) -> PhaseRenderStats {
    let name = matrix.phase.name();
    println!("{name} DETAILS ({})", matrix.subtitle);
    let classified = classify_matrix(matrix, known_issues);

    let row_width = matrix
        .rows
        .iter()
        .map(|row| row.label.len())
        .max()
        .unwrap_or(0)
        .max(1);
    let col_label_width = matrix
        .columns
        .iter()
        .map(|c| c.len())
        .max()
        .unwrap_or(0)
        .max(1);

    let mut emitted = false;
    for row in &matrix.rows {
        for (col, cell) in matrix.columns.iter().zip(&row.cells) {
            if cell.kind == CellKind::Blank {
                continue;
            }
            let painted = color.paint(&cell.text, cell.kind);
            println!(
                "  {label:row_width$}  {col:col_label_width$}  {painted}",
                label = row.label,
                col = col,
            );
            emitted = true;
        }
    }
    if !emitted {
        println!("  (no results)");
    }
    println!();

    println!("{name} SUMMARY");
    let summary_widths: Vec<usize> = matrix.columns.iter().map(|c| c.len().max(4)).collect();
    print!("  {:row_width$}", "");
    for (col, width) in matrix.columns.iter().zip(&summary_widths) {
        print!("  {col:>width$}");
    }
    println!();
    for (row, classified_row) in matrix.rows.iter().zip(&classified.rows) {
        print!("  {:row_width$}", row.label);
        for (classified_cell, width) in classified_row.iter().zip(&summary_widths) {
            let painted = color.paint(classified_cell.token, classified_cell.kind);
            let padding = width.saturating_sub(classified_cell.token.len());
            print!("  ");
            if padding > 0 {
                print!("{:padding$}", "");
            }
            print!("{painted}");
        }
        println!();
    }
    if !classified.footnotes.is_empty() {
        println!();
        println!("{name} KNOWN-ISSUE DRIFT");
        for note in &classified.footnotes {
            println!("  {note}");
        }
    }
    println!();
    classified.stats
}

#[derive(Clone)]
struct ClassifiedMatrix {
    rows: Vec<Vec<ClassifiedCell>>,
    footnotes: Vec<String>,
    stats: PhaseRenderStats,
}

#[derive(Clone)]
struct ClassifiedCell {
    token: &'static str,
    kind: CellKind,
}

#[derive(Clone, Default)]
struct PhaseRenderStats {
    phase: &'static str,
    ok: usize,
    known_issue: usize,
    wontfix: usize,
    err: usize,
    brkn: usize,
    skipped: usize,
    blank: usize,
    resolved: usize,
    drifted: usize,
}

impl PhaseRenderStats {
    fn strict_failed(&self) -> bool {
        self.err > 0 || self.resolved > 0 || self.drifted > 0
    }

    fn summary_text(&self) -> String {
        format!(
            "{} {} ok, {} ki, {} wtf, {} err, {} brkn, {} skip, {} blank, {} resolved, {} drifted",
            self.phase,
            self.ok,
            self.known_issue,
            self.wontfix,
            self.err,
            self.brkn,
            self.skipped,
            self.blank,
            self.resolved,
            self.drifted
        )
    }
}

fn classify_matrix(matrix: &Matrix, known_issues: &KnownIssues) -> ClassifiedMatrix {
    let mut rows = Vec::new();
    let mut footnotes = Vec::new();
    let mut stats = PhaseRenderStats {
        phase: match matrix.phase {
            PhaseLabel::Decode => "decode",
            PhaseLabel::Encode => "encode",
        },
        ..PhaseRenderStats::default()
    };

    for row in &matrix.rows {
        let mut classified_row = Vec::new();
        for (col, cell) in matrix.columns.iter().zip(&row.cells) {
            let issue = known_issues.lookup(matrix.phase, &row.label, col);
            let classified = classify_cell(cell, issue);
            match classified.token {
                "OK" => stats.ok += 1,
                "KI" => stats.known_issue += 1,
                "WTF" => stats.wontfix += 1,
                "ERR" => stats.err += 1,
                "BRKN" => stats.brkn += 1,
                "SKIP" => stats.skipped += 1,
                "" => stats.blank += 1,
                "OK*" => {
                    stats.resolved += 1;
                    if let Some(issue) = issue {
                        footnotes.push(format!(
                            "{} / {} / {}: resolved; remove from known-issues.ron ({})",
                            matrix.phase.name(),
                            row.label,
                            col,
                            issue.upstream
                        ));
                    }
                }
                "ERR!" => {
                    stats.drifted += 1;
                    if let Some(issue) = issue {
                        footnotes.push(format!(
                            "{} / {} / {}: expected {}, got {} ({})",
                            matrix.phase.name(),
                            row.label,
                            col,
                            issue.expect.describe(),
                            cell.text,
                            issue.upstream
                        ));
                    }
                }
                _ => unreachable!("unknown classified token {:?}", classified.token),
            }
            classified_row.push(classified);
        }
        rows.push(classified_row);
    }

    ClassifiedMatrix {
        rows,
        footnotes,
        stats,
    }
}

fn classify_cell(cell: &Cell, issue: Option<&KnownIssue>) -> ClassifiedCell {
    match cell.kind {
        CellKind::Blank => ClassifiedCell {
            token: "",
            kind: CellKind::Blank,
        },
        CellKind::Skip => ClassifiedCell {
            token: "SKIP",
            kind: CellKind::Skip,
        },
        CellKind::Ok | CellKind::Lossy => {
            if issue.is_some() {
                ClassifiedCell {
                    token: "OK*",
                    kind: CellKind::Resolved,
                }
            } else {
                ClassifiedCell {
                    token: "OK",
                    kind: CellKind::Ok,
                }
            }
        }
        CellKind::OurError | CellKind::ThirdPartyBreak => {
            if let Some(issue) = issue {
                if issue.matches(cell) {
                    if issue.wontfix {
                        ClassifiedCell {
                            token: "WTF",
                            kind: CellKind::Wontfix,
                        }
                    } else {
                        ClassifiedCell {
                            token: "KI",
                            kind: CellKind::KnownIssue,
                        }
                    }
                } else {
                    ClassifiedCell {
                        token: "ERR!",
                        kind: CellKind::OurError,
                    }
                }
            } else {
                match cell.kind {
                    CellKind::OurError => ClassifiedCell {
                        token: "ERR",
                        kind: CellKind::OurError,
                    },
                    CellKind::ThirdPartyBreak => ClassifiedCell {
                        token: "BRKN",
                        kind: CellKind::ThirdPartyBreak,
                    },
                    _ => unreachable!(),
                }
            }
        }
        CellKind::KnownIssue | CellKind::Wontfix | CellKind::Resolved => {
            unreachable!("cells are classified once")
        }
    }
}

fn reset_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|err| format!("remove {path:?}: {err}"))?;
    }
    fs::create_dir_all(path).map_err(|err| format!("create {path:?}: {err}"))
}

fn setup_output(mut cmd: Command, label: &str) -> Option<String> {
    match cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).output() {
        Ok(out) if out.status.success() => None,
        Ok(out) => Some(format!(
            "{label}: exited {}: {}",
            out.status,
            flatten_msg(&format!(
                "{} {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ))
        )),
        Err(err) => Some(format!("{label}: spawn failed: {err}")),
    }
}

fn report_setup_failure(tool: &str, err: Option<String>) {
    if let Some(err) = err {
        eprintln!("conformance-matrix: failed to build {tool}: {err}");
    } else {
        eprintln!("conformance-matrix: failed to resolve {tool}");
    }
}

fn system_jbig2enc_bin() -> Option<PathBuf> {
    let candidates = [
        std::env::var_os("JBIG2ENC_BIN").map(PathBuf::from),
        Some(PathBuf::from("/opt/homebrew/bin/jbig2")),
        Some(PathBuf::from("/usr/local/bin/jbig2")),
        which_on_path("jbig2enc"),
        which_on_path("jbig2"),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|path| path.is_file() && identifies_as(path, "-V", "jbig2enc"))
}

fn system_jbig2dec_bin() -> Option<PathBuf> {
    let candidates = [
        std::env::var_os("JBIG2DEC_BIN").map(PathBuf::from),
        Some(PathBuf::from("/opt/homebrew/bin/jbig2dec")),
        Some(PathBuf::from("/usr/local/bin/jbig2dec")),
        which_on_path("jbig2dec"),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|path| path.is_file() && identifies_as(path, "-V", "jbig2dec"))
}

fn vendor_jbig2enc_bin(root: &Path) -> Option<PathBuf> {
    let dir = root.join("vendor").join("jbig2enc");
    let candidates = [dir.join("jbig2enc"), dir.join("src").join("jbig2")];
    if let Some(found) = candidates.iter().find(|path| path.is_file()) {
        return Some(found.clone());
    }
    let mut last_error = None;
    let mut autogen = Command::new("./autogen.sh");
    autogen.current_dir(&dir);
    if let Some(err) = setup_output(autogen, "jbig2enc autogen") {
        last_error = Some(err);
    }
    let mut configure = Command::new("./configure");
    configure.current_dir(&dir);
    if let Some(err) = setup_output(configure, "jbig2enc configure") {
        last_error = Some(err);
    }
    let mut make = Command::new("make");
    make.current_dir(&dir);
    if let Some(err) = setup_output(make, "jbig2enc make") {
        last_error = Some(err);
    }
    let found = candidates.into_iter().find(|path| path.is_file());
    if found.is_none() {
        report_setup_failure("jbig2enc", last_error);
    }
    found
}

fn vendor_jbig2dec_bin(root: &Path) -> Option<PathBuf> {
    let dir = root.join("vendor").join("jbig2dec");
    let bin = dir.join("jbig2dec");
    if bin.is_file() {
        return Some(bin);
    }
    let mut last_error = None;
    let mut make = Command::new("make");
    make.arg("-f").arg("Makefile.unix").current_dir(&dir);
    if let Some(err) = setup_output(make, "jbig2dec make") {
        last_error = Some(err);
    }
    if !bin.is_file() && Path::new("/opt/homebrew/include/png.h").is_file() {
        let mut make = Command::new("make");
        make.arg("-f")
            .arg("Makefile.unix")
            .arg("CC=cc -I/opt/homebrew/include -L/opt/homebrew/lib")
            .current_dir(&dir);
        if let Some(err) = setup_output(make, "jbig2dec make homebrew") {
            last_error = Some(err);
        }
    }
    if bin.is_file() {
        Some(bin)
    } else {
        report_setup_failure("jbig2dec", last_error);
        None
    }
}

fn t88_tools(root: &Path) -> Option<T88Tools> {
    let dir = root
        .join("vendor")
        .join("T-REC-T.88-201808")
        .join("Software")
        .join("JBIG2_SampleSoftware-A20180829");
    let source = dir.join("source");
    let jbig2 = source.join("jbig2");
    let imgcomp = source.join("imgcomp");
    let mut build_error = None;
    if !(jbig2.is_file() && imgcomp.is_file()) {
        let mut make = Command::new("make");
        make.arg("jbig2").arg("imgcomp").current_dir(&dir);
        if let Some(err) = setup_output(make, "itu-t88 make") {
            build_error = Some(err);
        }
    }
    if jbig2.is_file() && imgcomp.is_file() {
        Some(T88Tools { jbig2, imgcomp })
    } else {
        report_setup_failure("itu-t88", build_error);
        None
    }
}

fn t88_test_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("vendor")
        .join("T-REC-T.88-201808")
        .join("Software")
        .join("JBIG2_SampleSoftware-A20180829")
        .join("test")
}

fn java_cmd(_root: &Path) -> Option<Vec<OsString>> {
    let raw = std::env::var("JBIG2_IMAGEIO_CMD").ok()?;
    if raw.trim().is_empty() {
        return None;
    }
    split_shell_like(&raw).ok()
}

fn split_shell_like(raw: &str) -> Result<Vec<OsString>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), '\\') => {
                if let Some(next) = chars.next() {
                    cur.push(next);
                }
            }
            (Some(_), c) => cur.push(c),
            (None, '\'' | '"') => quote = Some(ch),
            (None, c) if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(OsString::from(std::mem::take(&mut cur)));
                }
            }
            (None, '\\') => {
                if let Some(next) = chars.next() {
                    cur.push(next);
                }
            }
            (None, c) => cur.push(c),
        }
    }
    if quote.is_some() {
        return Err("unterminated quote".to_string());
    }
    if !cur.is_empty() {
        out.push(OsString::from(cur));
    }
    if out.is_empty() {
        Err("empty command".to_string())
    } else {
        Ok(out)
    }
}

fn identifies_as(path: &Path, arg: &str, needle: &str) -> bool {
    Command::new(path)
        .arg(arg)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map(|out| {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            combined.contains(needle)
        })
        .unwrap_or(false)
}

fn which_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn run_output(mut cmd: Command, label: &str) -> Result<std::process::Output, String> {
    let out = cmd
        .output()
        .map_err(|err| format!("{label}: spawn failed: {err}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        return Err(format!(
            "{label}: exited {}: {}{}",
            out.status,
            stdout.trim(),
            stderr.trim()
        ));
    }
    Ok(out)
}

fn strip_ext(path: &Path) -> Result<PathBuf, String> {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .ok_or_else(|| format!("{path:?}: missing file stem"))?;
    Ok(parent.join(stem))
}

fn append_extension(path: &Path, ext: &str) -> Result<PathBuf, String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("{path:?}: missing file name"))?
        .to_string_lossy();
    Ok(path.with_file_name(format!("{file_name}.{ext}")))
}

fn safe_name(s: &str) -> String {
    s.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn flatten_msg(err: &str) -> String {
    err.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn format_mse(mse: f64) -> String {
    if mse.fract() == 0.0 {
        format!("{mse:.0}")
    } else {
        format!("{mse:.6}")
    }
}
