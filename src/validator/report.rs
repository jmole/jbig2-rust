//! Validator report and finding rendering.

use crate::validator::{CheckId, SpecCite};

/// Severity attached to a validator finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Informational note.
    Info,
    /// Interop warning that does not make the strict verdict invalid.
    Warning,
    /// Strict T.88 violation.
    Error,
}

impl Severity {
    /// Stable lowercase label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// One validator finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// Check that emitted the finding.
    pub check_id: CheckId,
    /// Severity after lens application.
    pub severity: Severity,
    /// Byte offset most closely associated with the finding.
    pub byte_offset: usize,
    /// Segment number, if the finding is segment-local.
    pub segment_number: Option<u32>,
    /// Segment type label, if known.
    pub segment_type: Option<&'static str>,
    /// Human-readable finding message.
    pub message: String,
    /// Normative citation.
    pub cite: SpecCite,
}

impl Finding {
    /// Construct a finding.
    pub fn new(
        check_id: CheckId,
        severity: Severity,
        byte_offset: usize,
        segment_number: Option<u32>,
        segment_type: Option<&'static str>,
        message: impl Into<String>,
        cite: SpecCite,
    ) -> Self {
        Self {
            check_id,
            severity,
            byte_offset,
            segment_number,
            segment_type,
            message: message.into(),
            cite,
        }
    }
}

/// Non-finding validator failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatorError {
    /// Error message.
    pub message: String,
}

/// Complete validator report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Findings sorted by `(byte_offset, check_id)`.
    pub findings: Vec<Finding>,
}

impl Report {
    /// True when the report has at least one error-severity finding.
    pub fn is_invalid(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == Severity::Error)
    }

    /// Highest severity in the report.
    pub fn highest_severity(&self) -> Option<Severity> {
        self.findings
            .iter()
            .map(|finding| finding.severity)
            .max()
    }

    /// True when findings are sorted by the public invariant.
    pub fn is_sorted(&self) -> bool {
        self.findings.windows(2).all(|pair| {
            let a = (&pair[0].byte_offset, pair[0].check_id);
            let b = (&pair[1].byte_offset, pair[1].check_id);
            a <= b
        })
    }

    /// Text rendering for CLI output.
    pub fn render_text(&self) -> String {
        if self.findings.is_empty() {
            return "OK: no validator findings\n".to_string();
        }
        let mut out = String::new();
        for finding in &self.findings {
            let segment = finding
                .segment_number
                .map(|number| format!(" segment={number}"))
                .unwrap_or_default();
            out.push_str(&format!(
                "{} {} offset={}{}: {}\n  cite: {} ({})\n",
                finding.severity.as_str(),
                finding.check_id,
                finding.byte_offset,
                segment,
                finding.message,
                finding.cite.section,
                finding.cite.source.path
            ));
        }
        out
    }

    /// JSON rendering for CLI output.
    #[cfg(any(feature = "validator-cli", feature = "validator-corpus"))]
    pub fn render_json(&self) -> String {
        use serde_json::json;

        let findings = self
            .findings
            .iter()
            .map(|finding| {
                json!({
                    "check_id": finding.check_id.as_str(),
                    "severity": finding.severity.as_str(),
                    "byte_offset": finding.byte_offset,
                    "segment_number": finding.segment_number,
                    "segment_type": finding.segment_type,
                    "message": finding.message,
                    "cite": {
                        "section": finding.cite.section,
                        "quote": finding.cite.quote,
                        "source": finding.cite.source.path,
                    },
                })
            })
            .collect::<Vec<_>>();
        json!({
            "invalid": self.is_invalid(),
            "highest_severity": self.highest_severity().map(Severity::as_str),
            "findings": findings,
        })
        .to_string()
    }

    /// JSON rendering placeholder when `serde_json` is unavailable.
    #[cfg(not(any(feature = "validator-cli", feature = "validator-corpus")))]
    pub fn render_json(&self) -> String {
        self.render_text()
    }

    /// Short cell token for conformance matrix integration.
    pub fn render_matrix_cell(&self) -> String {
        let errors = self
            .findings
            .iter()
            .filter(|finding| finding.severity == Severity::Error)
            .count();
        if errors > 0 {
            return format!("BAD({errors})");
        }
        let warnings = self
            .findings
            .iter()
            .filter(|finding| finding.severity == Severity::Warning)
            .count();
        if warnings > 0 {
            format!("WARN({warnings})")
        } else {
            "OK".to_string()
        }
    }
}
