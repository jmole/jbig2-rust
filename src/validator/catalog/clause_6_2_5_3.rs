//! Shared AT-pixel legality checks (T.88 clause 6.2.5.3).

use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct AtPixelBodyLength;

/// Clause 6.2.5.3 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(AtPixelBodyLength)]
}

impl Check for AtPixelBodyLength {
    fn id(&self) -> CheckId {
        CheckId("T88-6.2.5.3-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("6.2.5.3", "Adaptive template pixel positions shall be present and legal for the selected generic-region template.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter_map(|node| match node.parsed {
                ParsedBody::GenericRegion {
                    flags: Some(flags), ..
                } => Some((node, flags)),
                _ => None,
            })
            .filter(|(node, flags)| {
                let mmr = flags & 0x01 != 0;
                let template = (flags >> 1) & 0x03;
                let ext = flags & 0x10 != 0;
                let needed = if mmr {
                    18
                } else if template == 0 && ext {
                    42
                } else if template == 0 {
                    26
                } else {
                    20
                };
                node.body.len() < needed
            })
            .map(|(node, _)| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "generic region body is too short for its declared AT pixels",
                )
            })
            .collect()
    }
}
