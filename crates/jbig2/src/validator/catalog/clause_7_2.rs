//! Segment header checks (T.88 clause 7.2).

use crate::segments::header::SegmentType;
use crate::validator::{
    Check, CheckCtx, CheckId, Finding, ReferredCountForm, SegmentTree, SpecCite,
};

use super::finding;

struct SegmentNumberOrder;
struct SegmentTypeDefined;
struct ReferredCountEncoding;
struct RetainBitsLength;
struct DataLengthTightness;
struct UnknownLengthAllowed;

/// Clause 7.2 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(SegmentNumberOrder),
        Box::new(SegmentTypeDefined),
        Box::new(ReferredCountEncoding),
        Box::new(RetainBitsLength),
        Box::new(DataLengthTightness),
        Box::new(UnknownLengthAllowed),
    ]
}

impl Check for SegmentNumberOrder {
    fn id(&self) -> CheckId {
        CheckId("T88-7.2.1-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88(
            "7.2.1",
            "Segment numbers shall be assigned in ascending order.",
        )
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut out = Vec::new();
        let mut prev = None;
        for node in &tree.segments {
            if let Some(prev) = prev {
                if node.header.number <= prev {
                    out.push(finding(
                        self.id(),
                        self.cite(),
                        Some(node),
                        node.offset,
                        "segment number is not greater than the previous segment number",
                    ));
                }
            }
            prev = Some(node.header.number);
        }
        out
    }
}

impl Check for SegmentTypeDefined {
    fn id(&self) -> CheckId {
        CheckId("T88-7.2.2-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.2.2", "The segment type field shall contain one of the segment type values defined in Table 2.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter(|node| node.header.segment_type.is_none())
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + 4,
                    "unknown or reserved segment type",
                )
            })
            .collect()
    }
}

impl Check for ReferredCountEncoding {
    fn id(&self) -> CheckId {
        CheckId("T88-7.2.3-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88(
            "7.2.3",
            "Values 5 and 6 of the three-bit referred-to segment count field are reserved.",
        )
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter(|node| node.header.referred_count_form == ReferredCountForm::Reserved)
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + 5,
                    "reserved referred-to segment count form",
                )
            })
            .collect()
    }
}

impl Check for RetainBitsLength {
    fn id(&self) -> CheckId {
        CheckId("T88-7.2.4-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.2.4", "The retain flags shall contain one flag for each referred-to segment and one flag for the current segment.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter(|node| node.header.retain_bits.len() != node.header.referred.len() + 1)
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + 5,
                    "retain-bit count does not equal referred count plus one",
                )
            })
            .collect()
    }
}

impl Check for DataLengthTightness {
    fn id(&self) -> CheckId {
        CheckId("T88-7.2.7-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.2.7", "The segment data length field shall give the number of octets in the segment data field.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter(|node| {
                node.header
                    .data_length
                    .is_some_and(|len| len as usize != node.body.len())
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len.saturating_sub(4),
                    format!(
                        "declared data length {} but parsed {} bytes",
                        node.header.data_length.unwrap_or(0),
                        node.body.len()
                    ),
                )
            })
            .collect()
    }
}

impl Check for UnknownLengthAllowed {
    fn id(&self) -> CheckId {
        CheckId("T88-7.2.7-002")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.2.7", "The value 0xffffffff may be used only where the standard permits an unknown segment data length.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter(|node| node.header.data_length.is_none())
            .filter(|node| {
                !matches!(
                    node.header.segment_type,
                    Some(SegmentType::ImmediateGenericRegion)
                        | Some(SegmentType::ImmediateLosslessGenericRegion)
                        | Some(SegmentType::ImmediateTextRegion)
                        | Some(SegmentType::ImmediateLosslessTextRegion)
                        | Some(SegmentType::ImmediateHalftoneRegion)
                        | Some(SegmentType::ImmediateLosslessHalftoneRegion)
                )
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len.saturating_sub(4),
                    "unknown segment data length is not permitted for this segment type",
                )
            })
            .collect()
    }
}
