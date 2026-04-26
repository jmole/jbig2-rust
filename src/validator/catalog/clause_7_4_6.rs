//! Generic region checks (T.88 clause 7.4.6).

use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct GenericRegionFlags;

/// Clause 7.4.6 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(GenericRegionFlags)]
}

impl Check for GenericRegionFlags {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.6-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.6", "The generic region segment flags field shall define MMR, GBTEMPLATE, TPGDON, EXTTEMPLATE, and reserved bits.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut out = Vec::new();
        for node in &tree.segments {
            let ParsedBody::GenericRegion { flags, .. } = node.parsed else {
                continue;
            };
            let Some(flags) = flags else {
                continue;
            };
            let mmr = flags & 0x01 != 0;
            let template = (flags >> 1) & 0x03;
            let tpgdon = flags & 0x08 != 0;
            let ext_template = flags & 0x10 != 0;
            if flags & 0xE0 != 0
                || (mmr && template != 0)
                || (ext_template && template != 0)
                || (tpgdon && template > 1)
            {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len + 17,
                    "generic region flags contain reserved or incoherent values",
                ));
            }
        }
        out
    }
}
