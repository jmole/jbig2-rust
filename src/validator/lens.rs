//! Conformance lenses for validator findings.

use crate::validator::{CheckId, Severity};

/// Validator lens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Lens {
    /// Strict ITU-T T.88.
    #[default]
    StrictT88,
    /// Re-weight for Artifex `jbig2dec` interop.
    Jbig2decInterop,
    /// Re-weight for the ITU T.88 reference codec.
    ItuT88Interop,
    /// Re-weight for `jbig2-imageio` interop.
    ImageioInterop,
}

/// Per-check decision produced by a lens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LensDecision {
    /// Keep the finding with this severity.
    Emit(Severity),
    /// Suppress the finding for this lens.
    Disable,
}

impl Lens {
    /// Apply the lens to a check's default severity.
    pub fn decide(self, id: CheckId, severity: Severity) -> LensDecision {
        match self {
            Self::StrictT88 => LensDecision::Emit(severity),
            Self::Jbig2decInterop => match id.as_str() {
                "T88-7.4.14-001" => LensDecision::Emit(Severity::Warning),
                _ => LensDecision::Emit(severity),
            },
            Self::ItuT88Interop => match id.as_str() {
                // The ITU reference codec resolves symbol-dictionary refs
                // implicitly from the per-page segment graph. The codeStreamTest*
                // conformance streams emit text regions with zero explicit
                // references; downgrade so we still surface this divergence
                // without poisoning interop.
                "T88-7.3.2-002" => LensDecision::Emit(Severity::Warning),
                "T88-7.4.2-009" => LensDecision::Emit(Severity::Warning),
                _ => LensDecision::Emit(severity),
            },
            Self::ImageioInterop => match id.as_str() {
                "T88-7.4.14-001" => LensDecision::Emit(Severity::Warning),
                _ => LensDecision::Emit(severity),
            },
        }
    }
}
