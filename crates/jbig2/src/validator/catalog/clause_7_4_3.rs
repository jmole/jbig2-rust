//! Text region checks (T.88 clause 7.4.3).

use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct TextRegionFlags;
struct TextInstanceCount;

/// Clause 7.4.3 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(TextRegionFlags), Box::new(TextInstanceCount)]
}

impl Check for TextRegionFlags {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.3-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.3", "The text region segment flags and text region Huffman flags shall contain only the values defined for the text region segment.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut out = Vec::new();
        for node in &tree.segments {
            let ParsedBody::TextRegion {
                flags, huff_flags, ..
            } = node.parsed
            else {
                continue;
            };
            let sbcombop = (flags >> 7) & 0x3;
            if sbcombop > 3 {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len + 17,
                    "text region combination operator is reserved",
                ));
            }
            if huff_flags.is_some_and(|value| value & 0x8000 != 0) {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len + 19,
                    "text Huffman flags contain a reserved high bit",
                ));
            }
        }
        out
    }
}

impl Check for TextInstanceCount {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.3-002")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88(
            "7.4.3",
            "SBNUMINSTANCES specifies the number of symbol instances in the text region.",
        )
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter_map(|node| match node.parsed {
                ParsedBody::TextRegion {
                    num_instances: Some(0),
                    ..
                } => Some(node),
                _ => None,
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "text region declares zero symbol instances",
                )
            })
            .collect()
    }
}
