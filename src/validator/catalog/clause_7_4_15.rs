//! Extension segment checks (T.88 clause 7.4.15).

use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct ExtensionBody;

/// Clause 7.4.15 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(ExtensionBody)]
}

impl Check for ExtensionBody {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.15-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88(
            "7.4.15",
            "The extension segment shall begin with an extension type field and extension flags.",
        )
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter_map(|node| match &node.parsed {
                ParsedBody::Extension { bytes } if bytes.len() < 4 => Some(node),
                _ => None,
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "extension segment body is too short",
                )
            })
            .collect()
    }
}
