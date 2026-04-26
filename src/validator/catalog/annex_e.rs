//! Annex E arithmetic-coded stream checks.

use crate::segments::header::SegmentType;
use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct ArithmeticBodyNonEmpty;

/// Annex E checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(ArithmeticBodyNonEmpty)]
}

impl Check for ArithmeticBodyNonEmpty {
    fn id(&self) -> CheckId {
        CheckId("T88-E.1.3-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("Annex E.1.3", "The arithmetic-coded byte stream shall be terminated and byte aligned as specified for the MQ coder.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter(|node| {
                matches!(
                    node.header.segment_type,
                    Some(SegmentType::SymbolDictionary)
                        | Some(SegmentType::ImmediateTextRegion)
                        | Some(SegmentType::ImmediateLosslessTextRegion)
                        | Some(SegmentType::ImmediateGenericRegion)
                        | Some(SegmentType::ImmediateLosslessGenericRegion)
                        | Some(SegmentType::ImmediateGenericRefinementRegion)
                        | Some(SegmentType::ImmediateLosslessGenericRefinementRegion)
                )
            })
            .filter(|node| !matches!(node.parsed, ParsedBody::PatternDictionary { .. }) && node.body.is_empty())
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "arithmetic-coded segment has an empty body",
                )
            })
            .collect()
    }
}
