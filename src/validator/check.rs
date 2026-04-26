//! Validator check trait and execution context.

use crate::validator::{Finding, SegmentTree, Severity, SpecCite};

use super::CheckId;

/// Immutable context passed to every check.
#[derive(Clone, Copy, Debug)]
pub struct CheckCtx {
    /// Active conformance lens.
    pub lens: crate::validator::Lens,
}

/// One spec-scoped validator check.
pub trait Check: Send + Sync {
    /// Stable check id.
    fn id(&self) -> CheckId;

    /// Default severity before lens application.
    fn severity(&self) -> Severity {
        Severity::Error
    }

    /// Normative citation.
    fn cite(&self) -> SpecCite;

    /// Execute the check.
    fn run(&self, ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding>;
}
