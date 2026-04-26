//! Segment reference graph checks (T.88 clause 7.3).

use std::collections::HashSet;

use crate::segments::header::SegmentType;
use crate::validator::{Check, CheckCtx, CheckId, Finding, SegmentTree, SpecCite};

use super::{finding, has_segment_number};

struct ReferredSegmentsExist;
struct ReferredSegmentsPrecede;
struct RegionPageHasPageInfo;
struct TextReferencesSymbolDictionary;

/// Clause 7.3 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(ReferredSegmentsExist),
        Box::new(ReferredSegmentsPrecede),
        Box::new(RegionPageHasPageInfo),
        Box::new(TextReferencesSymbolDictionary),
    ]
}

impl Check for ReferredSegmentsExist {
    fn id(&self) -> CheckId {
        CheckId("T88-7.3.1-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.3.1", "A referred-to segment number shall identify a segment in the current JBIG2 stream.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut out = Vec::new();
        for node in &tree.segments {
            for &r in &node.header.referred {
                if !has_segment_number(tree, r) {
                    out.push(finding(
                        self.id(),
                        self.cite(),
                        Some(node),
                        node.offset,
                        format!("referred segment {r} does not exist"),
                    ));
                }
            }
        }
        out
    }
}

impl Check for ReferredSegmentsPrecede {
    fn id(&self) -> CheckId {
        CheckId("T88-7.3.1-002")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.3.1", "A segment shall not refer to a segment that follows it in the sequential organization.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut out = Vec::new();
        for node in &tree.segments {
            for &r in &node.header.referred {
                if r >= node.header.number {
                    out.push(finding(
                        self.id(),
                        self.cite(),
                        Some(node),
                        node.offset,
                        format!("referred segment {r} does not precede current segment"),
                    ));
                }
            }
        }
        out
    }
}

impl Check for RegionPageHasPageInfo {
    fn id(&self) -> CheckId {
        CheckId("T88-7.3.2-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.3.2", "A page-associated region segment shall be associated with a page for which page information is present.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut seen_pages = HashSet::new();
        let mut out = Vec::new();
        for node in &tree.segments {
            if node.header.segment_type == Some(SegmentType::PageInformation) {
                seen_pages.insert(node.header.page_association);
                continue;
            }
            if node.parsed.region().is_some()
                && node.header.page_association != 0
                && !seen_pages.contains(&node.header.page_association)
            {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset,
                    format!(
                        "region segment associated with page {} before page information",
                        node.header.page_association
                    ),
                ));
            }
        }
        out
    }
}

impl Check for TextReferencesSymbolDictionary {
    fn id(&self) -> CheckId {
        CheckId("T88-7.3.2-002")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.3.2", "Text region segments shall refer to the symbol dictionaries from which symbol IDs are decoded.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let symbol_numbers = tree
            .segments
            .iter()
            .filter(|node| node.header.segment_type == Some(SegmentType::SymbolDictionary))
            .map(|node| node.header.number)
            .collect::<HashSet<_>>();
        tree.segments
            .iter()
            .filter(|node| {
                matches!(
                    node.header.segment_type,
                    Some(SegmentType::IntermediateTextRegion)
                        | Some(SegmentType::ImmediateTextRegion)
                        | Some(SegmentType::ImmediateLosslessTextRegion)
                )
            })
            .filter(|node| {
                !node
                    .header
                    .referred
                    .iter()
                    .any(|r| symbol_numbers.contains(r))
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset,
                    "text region does not refer to any symbol dictionary",
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::validator::{validate, CheckId, Lens};

    fn header(flags: u8) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&crate::JBIG2_MAGIC);
        out.push(flags);
        out.extend_from_slice(&1u32.to_be_bytes());
        out
    }

    fn seg(num: u32, flags: u8, ref_count_byte: u8, referred: &[u32], page: u8, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&num.to_be_bytes());
        out.push(flags);
        out.push(ref_count_byte);
        for &r in referred {
            out.push(r as u8);
        }
        out.push(page);
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
        out
    }

    fn region_body(width: u32, height: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&width.to_be_bytes());
        body.extend_from_slice(&height.to_be_bytes());
        body.extend_from_slice(&0u32.to_be_bytes());
        body.extend_from_slice(&0u32.to_be_bytes());
        body.push(0);
        body
    }

    fn ids(bytes: &[u8]) -> Vec<&'static str> {
        validate(bytes, Lens::StrictT88)
            .findings
            .into_iter()
            .map(|f| f.check_id.as_str())
            .collect()
    }

    #[test]
    fn referred_segments_exist_negative_no_refs() {
        let mut bytes = header(0x01);
        bytes.extend(seg(1, 0x33, 0x00, &[], 0, &[]));
        assert!(!ids(&bytes).contains(&CheckId("T88-7.3.1-001").as_str()));
    }

    #[test]
    fn referred_segments_exist_positive_dangling_ref() {
        let mut bytes = header(0x01);
        bytes.extend(seg(1, 0x33, 0x20, &[0x09], 0, &[]));
        assert!(ids(&bytes).contains(&CheckId("T88-7.3.1-001").as_str()));
    }

    #[test]
    fn referred_segments_precede_positive_forward_ref() {
        let mut bytes = header(0x01);
        bytes.extend(seg(1, 0x33, 0x20, &[0x09], 0, &[]));
        assert!(ids(&bytes).contains(&CheckId("T88-7.3.1-002").as_str()));
    }

    #[test]
    fn region_page_has_page_info_positive_orphaned_region() {
        let mut bytes = header(0x01);
        let body = {
            let mut b = region_body(1, 1);
            b.extend_from_slice(&[0xFF]);
            b
        };
        bytes.extend(seg(1, 0x26, 0x00, &[], 1, &body));
        assert!(ids(&bytes).contains(&CheckId("T88-7.3.2-001").as_str()));
    }
}
