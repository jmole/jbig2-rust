//! Spec citation types used by validator checks.

/// Source document containing the cited normative text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpecSource {
    /// Path to the repository-local Markdown copy of the spec.
    pub path: &'static str,
}

impl SpecSource {
    /// Repository-local ITU-T T.88 Markdown document.
    pub const T88_2018: Self = Self {
        path: "vendor/T-REC-T.88-201808/spec/ITU-T_T_88__08_2018.md",
    };
}

/// Normative citation for a validator check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpecCite {
    /// Spec section, annex, or table identifier.
    pub section: &'static str,
    /// Verbatim or tightly quoted normative sentence for review.
    pub quote: &'static str,
    /// Source file containing the cited passage.
    pub source: SpecSource,
}

impl SpecCite {
    /// Construct a T.88 citation.
    pub const fn t88(section: &'static str, quote: &'static str) -> Self {
        Self {
            section,
            quote,
            source: SpecSource::T88_2018,
        }
    }
}
