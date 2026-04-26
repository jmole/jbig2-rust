//! Spec-cited structural validator for JBIG2 / ITU-T T.88 streams.
//!
//! The validator is deliberately separate from the decoder: it walks the
//! wire format, builds a small structural tree, and runs clause-scoped checks
//! without trying to reconstruct pixels.

pub mod catalog;
mod check;
mod citation;
mod lens;
mod parse;
mod report;
mod runner;
mod segment_tree;

pub use check::{Check, CheckCtx};
pub use citation::{SpecCite, SpecSource};
pub use lens::{Lens, LensDecision};
pub use parse::{FileOrganization, ParsedFileHeader, ParsedSegmentHeader, ReferredCountForm};
pub use report::{Finding, Report, Severity, ValidatorError};
pub use segment_tree::{ParsedBody, RegionFields, SegmentNode, SegmentTree};

/// Stable identifier for a validator check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckId(pub &'static str);

impl CheckId {
    /// Return the stable string representation.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for CheckId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// Validate `bytes` against the selected conformance lens.
pub fn validate(bytes: &[u8], lens: Lens) -> Report {
    let tree = parse::parse(bytes);
    runner::run(tree, lens)
}

/// Parse `bytes` into a [`SegmentTree`] without running any checks.
///
/// Intended for `examples/` and ad-hoc inspection tools; production callers
/// should use [`validate`] instead.
pub fn parse_for_dump(bytes: &[u8]) -> SegmentTree {
    parse::parse(bytes)
}
