//! Pattern dictionary checks (T.88 clause 7.4.4).

use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct PatternDictionaryHeader;

/// Clause 7.4.4 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(PatternDictionaryHeader)]
}

impl Check for PatternDictionaryHeader {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.4-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.4", "The pattern dictionary segment shall define HDMMR, HDTEMPLATE, HDPW, HDPH, and GRAYMAX in the segment data header.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut out = Vec::new();
        for node in &tree.segments {
            let ParsedBody::PatternDictionary {
                flags,
                width,
                height,
                ..
            } = node.parsed
            else {
                continue;
            };
            if flags & 0xF8 != 0 {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "pattern dictionary flags contain reserved bits",
                ));
            }
            if width == Some(0) || height == Some(0) {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "pattern width and height must be non-zero",
                ));
            }
        }
        out
    }
}
