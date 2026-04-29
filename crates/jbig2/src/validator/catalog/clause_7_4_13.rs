//! Tables segment checks (T.88 clause 7.4.13).

use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct TablesBody;

/// Clause 7.4.13 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(TablesBody)]
}

impl Check for TablesBody {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.13-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.13", "A tables segment shall contain a well-formed table definition used by later Huffman-coded segments.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter_map(|node| match &node.parsed {
                ParsedBody::Tables { bytes } if bytes.is_empty() => Some(node),
                _ => None,
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "tables segment is empty",
                )
            })
            .collect()
    }
}
