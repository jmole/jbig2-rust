//! Page information, EOP, EOS, and EOF checks (T.88 clauses 7.4.8-7.4.11).

use std::collections::HashSet;

use crate::segments::header::SegmentType;
use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct PageInformationFields;
struct PageTerminators;
struct EofPlacement;

/// Clause 7.4.8 through 7.4.11 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(PageInformationFields),
        Box::new(PageTerminators),
        Box::new(EofPlacement),
    ]
}

impl Check for PageInformationFields {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.8-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.8", "The page information segment shall specify page dimensions, resolution, page flags, and striping information.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut out = Vec::new();
        for node in &tree.segments {
            let ParsedBody::PageInformation {
                width,
                height,
                flags,
                ..
            } = node.parsed
            else {
                continue;
            };
            if width == Some(0) || height == Some(0) || flags.is_none() {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "page information is truncated or has zero dimensions",
                ));
            }
        }
        out
    }
}

impl Check for PageTerminators {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.9-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88(
            "7.4.9",
            "An end-of-page segment shall indicate the end of a page.",
        )
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let pages = tree
            .segments
            .iter()
            .filter(|node| node.header.segment_type == Some(SegmentType::PageInformation))
            .map(|node| node.header.page_association)
            .collect::<HashSet<_>>();
        let ended = tree
            .segments
            .iter()
            .filter(|node| node.header.segment_type == Some(SegmentType::EndOfPage))
            .map(|node| node.header.page_association)
            .collect::<HashSet<_>>();
        pages
            .difference(&ended)
            .filter_map(|page| {
                tree.segments.iter().find(|node| {
                    node.header.segment_type == Some(SegmentType::PageInformation)
                        && node.header.page_association == *page
                })
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset,
                    "page information has no matching end-of-page segment",
                )
            })
            .collect()
    }
}

impl Check for EofPlacement {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.11-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88(
            "7.4.11",
            "The end-of-file segment shall occur after all other segments in the JBIG2 file.",
        )
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut out = Vec::new();
        for (idx, node) in tree.segments.iter().enumerate() {
            if node.header.segment_type == Some(SegmentType::EndOfFile)
                && idx + 1 != tree.segments.len()
            {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset,
                    "end-of-file segment is not the last parsed segment",
                ));
            }
        }
        out
    }
}
