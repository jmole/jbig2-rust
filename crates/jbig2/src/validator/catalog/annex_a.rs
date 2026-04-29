//! Annex A integer arithmetic range checks.

use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct IntegerRanges;

/// Annex A checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(IntegerRanges)]
}

impl Check for IntegerRanges {
    fn id(&self) -> CheckId {
        CheckId("T88-A-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("Annex A", "Integer arithmetic coding procedures shall decode integers within the range required by the associated coding context.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter_map(|node| match node.parsed {
                ParsedBody::SymbolDictionary {
                    num_ex_syms: Some(ex),
                    num_new_syms: Some(new),
                    ..
                } if ex > 1_000_000 || new > 1_000_000 => Some(node),
                ParsedBody::TextRegion {
                    num_instances: Some(instances),
                    ..
                } if instances > 10_000_000 => Some(node),
                _ => None,
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "declared arithmetic-coded count is implausibly large for validator resource limits",
                )
            })
            .collect()
    }
}
