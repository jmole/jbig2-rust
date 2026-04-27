use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use super::charting::render_comparison_chart;
use super::{DECODE_CASES, ENCODE_CASES};

/// One row of the post-run summary table. Filled in by every bench function
/// before it hands off to Criterion.
#[derive(Clone, Debug)]
pub(crate) struct ProbeRecord {
    pub(crate) side: &'static str,
    pub(crate) tool: &'static str,
    pub(crate) case: &'static str,
    pub(crate) raw_bytes: u64,
    pub(crate) compressed_bytes: u64,
}

fn probes() -> &'static Mutex<Vec<ProbeRecord>> {
    static R: OnceLock<Mutex<Vec<ProbeRecord>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) fn record(r: ProbeRecord) {
    probes().lock().unwrap().push(r);
}

/// Extract Criterion's mean point estimate (nanoseconds per iteration) from
/// `estimates.json`.
pub(crate) fn parse_mean_ns(path: &Path) -> Option<f64> {
    let data = fs::read_to_string(path).ok()?;
    let i = data.find("\"mean\":{")?;
    let rest = &data[i + "\"mean\":".len()..];
    let pe = rest.find("\"point_estimate\":")?;
    let after = &rest[pe + "\"point_estimate\":".len()..];
    let end = after.find([',', '}']).unwrap_or(after.len());
    after[..end].trim().parse().ok()
}

pub(crate) fn criterion_output_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("CRITERION_HOME") {
        return PathBuf::from(home);
    }
    if let Some(target) = std::env::var_os("CARGO_TARGET_DIR") {
        return PathBuf::from(target).join("criterion");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("criterion")
}

pub(crate) fn estimates_path(side: &str, tool: &str, case: &str) -> PathBuf {
    criterion_output_dir()
        .join(format!("{side}_reference"))
        .join(tool)
        .join(case)
        .join("new")
        .join("estimates.json")
}

fn fmt_ns(ns: f64) -> String {
    if ns >= 1e9 {
        format!("{:.3} s", ns / 1e9)
    } else if ns >= 1e6 {
        format!("{:.3} ms", ns / 1e6)
    } else if ns >= 1e3 {
        format!("{:.3} µs", ns / 1e3)
    } else {
        format!("{:.0} ns", ns)
    }
}

pub(crate) const TOOL_ORDER: &[&str] = &["rust", "jbig2enc", "jbig2dec", "t88"];

pub(crate) fn tool_rank(tool: &str) -> usize {
    TOOL_ORDER
        .iter()
        .position(|&o| o == tool)
        .unwrap_or(TOOL_ORDER.len())
}

pub(crate) fn print_summary() {
    let records = probes().lock().unwrap().clone();
    if records.is_empty() {
        return;
    }

    let mut by_case: BTreeMap<(&'static str, &'static str), Vec<ProbeRecord>> = BTreeMap::new();
    for r in &records {
        by_case.entry((r.side, r.case)).or_default().push(r.clone());
    }

    let mut stderr = std::io::stderr().lock();
    use std::io::Write;
    let _ = writeln!(stderr);
    let _ = writeln!(
        stderr,
        "================ Reference codec comparison ================"
    );

    let ordered_cases = |side: &str| -> Vec<&'static str> {
        match side {
            "decode" => DECODE_CASES.iter().map(|c| c.tag).collect(),
            "encode" => ENCODE_CASES.iter().map(|c| c.tag).collect(),
            _ => Vec::new(),
        }
    };

    for side in ["decode", "encode"] {
        for case in ordered_cases(side) {
            let Some(tools) = by_case.get(&(side, case)) else {
                continue;
            };
            let mut tools: Vec<&ProbeRecord> = tools.iter().collect();
            tools.sort_by_key(|r| tool_rank(r.tool));

            let _ = writeln!(stderr);
            let _ = writeln!(stderr, "{side}/{case}");
            let _ = writeln!(
                stderr,
                "  {:<10} {:>12} {:>14} {:>12} {:>9}",
                "tool", "mean", "throughput", "bytes", "ratio"
            );
            let _ = writeln!(stderr, "  {}", "-".repeat(61));

            for r in tools {
                let est = estimates_path(r.side, r.tool, r.case);
                let (time_str, tput_str) = match parse_mean_ns(&est) {
                    Some(ns) => {
                        let seconds = ns / 1e9;
                        let mib_s = (r.raw_bytes as f64) / seconds / (1024.0 * 1024.0);
                        (fmt_ns(ns), format!("{mib_s:.1} MiB/s"))
                    }
                    None => ("n/a".to_string(), "n/a".to_string()),
                };
                let ratio = if r.compressed_bytes == 0 {
                    "  n/a".to_string()
                } else {
                    format!("{:.2}x", r.raw_bytes as f64 / r.compressed_bytes as f64)
                };
                let _ = writeln!(
                    stderr,
                    "  {:<10} {:>12} {:>14} {:>12} {:>9}",
                    r.tool, time_str, tput_str, r.compressed_bytes, ratio
                );
            }
        }
    }

    let _ = writeln!(stderr);
    let _ = writeln!(
        stderr,
        "Columns: mean = Criterion point estimate, throughput = raw page bytes / mean,"
    );
    let _ = writeln!(
        stderr,
        "         bytes = compressed stream size (input for decode, output for encode),"
    );
    let _ = writeln!(stderr, "         ratio = raw / compressed.");
    let _ = writeln!(
        stderr,
        "Subprocess rows (t88, jbig2enc, jbig2dec) include fork/exec overhead."
    );
    let _ = writeln!(
        stderr,
        "============================================================"
    );
    print_html_link(&mut stderr);
    match render_comparison_chart(&records) {
        Ok(Some(path)) => {
            let _ = writeln!(stderr);
            let _ = writeln!(
                stderr,
                "Cross-tool grouped bar chart (MiB/s, log scale, colored by tool):"
            );
            let _ = writeln!(stderr, "  file://{}", path.display());
        }
        Ok(None) => {}
        Err(e) => {
            let _ = writeln!(stderr, "[warn] chart render failed: {e}");
        }
    }
}

fn print_html_link<W: std::io::Write>(stderr: &mut W) {
    let base = criterion_output_dir();
    let root = base.join("report").join("index.html");
    let _ = writeln!(stderr);
    if root.is_file() {
        let _ = writeln!(stderr, "Full Criterion HTML report:");
        let _ = writeln!(stderr, "  file://{}", root.display());
        for side in ["decode_reference", "encode_reference"] {
            let p = base.join(side).join("report").join("index.html");
            if p.is_file() {
                let _ = writeln!(stderr, "  file://{}", p.display());
            }
        }
    } else {
        let _ = writeln!(stderr, "HTML report not generated at {}.", root.display());
        let _ = writeln!(
            stderr,
            "If you used --quick, re-run without it; --quick skips Criterion's disk output."
        );
    }
}
