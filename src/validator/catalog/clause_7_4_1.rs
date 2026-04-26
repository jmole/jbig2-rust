//! Region information checks (T.88 clause 7.4.1).

use crate::validator::{Check, CheckCtx, CheckId, Finding, SegmentTree, SpecCite};

use super::finding;

struct RegionDimensions;
struct RegionFlags;

/// Clause 7.4.1 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(RegionDimensions), Box::new(RegionFlags)]
}

impl Check for RegionDimensions {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.1-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.1", "The region segment information field shall specify the width and height of the region bitmap.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter_map(|node| node.parsed.region().map(|region| (node, region)))
            .filter(|(_, region)| region.width == 0 || region.height == 0)
            .map(|(node, _)| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "region width and height must be non-zero",
                )
            })
            .collect()
    }
}

impl Check for RegionFlags {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.1-002")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.1", "Only the defined external combination operator and colour-extension bits are present in the region segment information flags.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut out = Vec::new();
        for node in &tree.segments {
            let Some(region) = node.parsed.region() else {
                continue;
            };
            let combo = region.flags & 0x07;
            if combo > 4 || region.flags & 0xF0 != 0 {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len + 16,
                    "region flags contain reserved bits or an invalid combination operator",
                ));
            }
        }
        out
    }
}
