//! Halftone region checks (T.88 clause 7.4.5).

use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct HalftoneRegionHeader;

/// Clause 7.4.5 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(HalftoneRegionHeader)]
}

impl Check for HalftoneRegionHeader {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.5-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.5", "The halftone region segment data header shall contain HMMR, HTEMPLATE, HENABLESKIP, HCOMBOP, HDEFPIXEL, and grid parameters.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut out = Vec::new();
        for node in &tree.segments {
            let ParsedBody::HalftoneRegion {
                flags,
                grid_width,
                grid_height,
                ..
            } = node.parsed
            else {
                continue;
            };
            let Some(flags) = flags else {
                continue;
            };
            let hcombop = (flags >> 4) & 0x07;
            if hcombop > 4 || grid_width == Some(0) || grid_height == Some(0) {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len + 17,
                    "halftone region has invalid combination operator or zero grid dimension",
                ));
            }
        }
        out
    }
}
