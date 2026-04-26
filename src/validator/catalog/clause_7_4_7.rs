//! Generic refinement region checks (T.88 clause 7.4.7).

use crate::segments::header::SegmentType;
use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct RefinementRegionFlags;
struct RefinementReferenceType;

/// Clause 7.4.7 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(RefinementRegionFlags),
        Box::new(RefinementReferenceType),
    ]
}

impl Check for RefinementRegionFlags {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.7-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.7", "The generic refinement region segment flags field shall contain GRTEMPLATE, TPGRON, and reserved bits.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter_map(|node| match node.parsed {
                ParsedBody::RefinementRegion { flags: Some(flags), .. } if flags & 0xFC != 0 => {
                    Some(node)
                }
                _ => None,
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len + 17,
                    "refinement region flags contain reserved bits",
                )
            })
            .collect()
    }
}

impl Check for RefinementReferenceType {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.7-002")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.7", "Generic refinement regions shall refer to a bitmap source segment to be refined.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let bitmap_sources = tree
            .segments
            .iter()
            .filter(|node| {
                matches!(
                    node.header.segment_type,
                    Some(SegmentType::SymbolDictionary)
                        | Some(SegmentType::IntermediateGenericRegion)
                        | Some(SegmentType::ImmediateGenericRegion)
                        | Some(SegmentType::ImmediateLosslessGenericRegion)
                        | Some(SegmentType::IntermediateTextRegion)
                        | Some(SegmentType::ImmediateTextRegion)
                        | Some(SegmentType::ImmediateLosslessTextRegion)
                )
            })
            .map(|node| node.header.number)
            .collect::<std::collections::HashSet<_>>();
        tree.segments
            .iter()
            .filter(|node| matches!(node.parsed, ParsedBody::RefinementRegion { .. }))
            .filter(|node| {
                !node
                    .header
                    .referred
                    .iter()
                    .any(|r| bitmap_sources.contains(r))
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset,
                    "refinement region does not refer to a bitmap source segment",
                )
            })
            .collect()
    }
}
