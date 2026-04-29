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

    /// Check identifier as a stable string.
    pub fn check_id(&self) -> &str {
        self.check_id.as_str()
    }

    /// Finding severity.
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// Byte offset most closely associated with the finding.
    pub fn byte_offset(&self) -> Option<u64> {
        Some(self.byte_offset as u64)
    }

    /// Human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Referenced specification clause.
    pub fn spec_clause(&self) -> Option<&str> {
        Some(self.cite.section)
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
    /// Iterate over findings in sorted order.
    pub fn iter_findings(&self) -> impl Iterator<Item = &Finding> {
        self.findings.iter()
    }

    /// True when there are no error-severity findings.
    pub fn ok(&self) -> bool {
        !self.is_invalid()
    }

    /// True when the report has at least one error-severity finding.
    pub fn is_invalid(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == Severity::Error)
    }

    /// Highest severity in the report.
    pub fn highest_severity(&self) -> Option<Severity> {
        self.findings.iter().map(|finding| finding.severity).max()
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

    /// JSON rendering placeholder. The unpublished workbench owns structured
    /// JSON output so the published crate does not depend on `serde_json`.
    pub fn render_json(&self) -> String {
        self.render_text()
    }

    /// Number of error-severity findings.
    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == Severity::Error)
            .count()
    }

    /// Number of warning-severity findings.
    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == Severity::Warning)
            .count()
    }

    /// Short cell token for conformance matrix integration.
    pub fn render_matrix_cell(&self) -> String {
        let errors = self.error_count();
        if errors > 0 {
            return format!("BAD({errors})");
        }
        let warnings = self.warning_count();
        if warnings > 0 {
            format!("WARN({warnings})")
        } else {
            "OK".to_string()
        }
    }

    /// Same as [`Report::render_matrix_cell`], but when the report has
    /// any error-severity findings the token is suffixed with up to
    /// `id_limit` deduplicated `:CHECK-ID,CHECK-ID,...` entries so the
    /// conformance matrix can show which checks fired without parsing
    /// the full report.
    ///
    /// Consumers should prefer this over re-walking [`Report::findings`]
    /// so the matrix vocabulary stays in one place.
    pub fn render_matrix_cell_with_error_ids(&self, id_limit: usize) -> String {
        let mut token = self.render_matrix_cell();
        if id_limit == 0 || self.error_count() == 0 {
            return token;
        }
        let mut seen = Vec::with_capacity(id_limit);
        for finding in &self.findings {
            if finding.severity != Severity::Error {
                continue;
            }
            let id = finding.check_id.as_str();
            if seen.contains(&id) {
                continue;
            }
            seen.push(id);
            if seen.len() == id_limit {
                break;
            }
        }
        if !seen.is_empty() {
            token.push(':');
            token.push_str(&seen.join(","));
        }
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::{CheckId, SpecCite};

    fn finding(check: &'static str, severity: Severity) -> Finding {
        Finding::new(
            CheckId(check),
            severity,
            0,
            None,
            None,
            "synthetic",
            SpecCite::t88("0.0", "synthetic"),
        )
    }

    #[test]
    fn matrix_cell_ok_when_empty() {
        let report = Report::default();
        assert_eq!(report.render_matrix_cell(), "OK");
        assert_eq!(report.render_matrix_cell_with_error_ids(3), "OK");
        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 0);
    }

    #[test]
    fn matrix_cell_warns_when_only_warnings() {
        let report = Report {
            findings: vec![
                finding("T88-X-001", Severity::Warning),
                finding("T88-X-002", Severity::Warning),
            ],
        };
        assert_eq!(report.render_matrix_cell(), "WARN(2)");
        assert_eq!(report.render_matrix_cell_with_error_ids(3), "WARN(2)");
    }

    #[test]
    fn matrix_cell_with_error_ids_appends_unique_ids_up_to_limit() {
        let report = Report {
            findings: vec![
                finding("T88-A-001", Severity::Error),
                finding("T88-A-001", Severity::Error),
                finding("T88-B-001", Severity::Warning),
                finding("T88-C-001", Severity::Error),
                finding("T88-D-001", Severity::Error),
            ],
        };
        assert_eq!(report.error_count(), 4);
        assert_eq!(
            report.render_matrix_cell_with_error_ids(2),
            "BAD(4):T88-A-001,T88-C-001",
        );
        assert_eq!(
            report.render_matrix_cell_with_error_ids(10),
            "BAD(4):T88-A-001,T88-C-001,T88-D-001",
        );
    }

    #[test]
    fn matrix_cell_with_error_ids_zero_limit_drops_suffix() {
        let report = Report {
            findings: vec![finding("T88-A-001", Severity::Error)],
        };
        assert_eq!(report.render_matrix_cell_with_error_ids(0), "BAD(1)");
    }
}
