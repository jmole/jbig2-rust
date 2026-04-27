//! Spec-ordered validator check catalog.

pub mod annex_a;
pub mod annex_e;
pub mod clause_6_2_5_3;
pub mod clause_7_2;
pub mod clause_7_3;
pub mod clause_7_4_1;
pub mod clause_7_4_12;
pub mod clause_7_4_13;
pub mod clause_7_4_14;
pub mod clause_7_4_15;
pub mod clause_7_4_2;
pub mod clause_7_4_3;
pub mod clause_7_4_4;
pub mod clause_7_4_5;
pub mod clause_7_4_6;
pub mod clause_7_4_7;
pub mod clause_7_4_8_through_11;
pub mod file_header;

use crate::validator::{Check, Finding, SegmentNode, Severity, SpecCite};
use crate::validator::{CheckId, SegmentTree};

/// Return all catalog checks in spec order.
pub fn checks() -> Vec<Box<dyn Check>> {
    let mut out: Vec<Box<dyn Check>> = Vec::new();
    out.extend(file_header::checks());
    out.extend(clause_7_2::checks());
    out.extend(clause_7_3::checks());
    out.extend(clause_7_4_1::checks());
    out.extend(clause_7_4_2::checks());
    out.extend(clause_7_4_3::checks());
    out.extend(clause_7_4_4::checks());
    out.extend(clause_7_4_5::checks());
    out.extend(clause_7_4_6::checks());
    out.extend(clause_7_4_7::checks());
    out.extend(clause_7_4_8_through_11::checks());
    out.extend(clause_7_4_12::checks());
    out.extend(clause_7_4_13::checks());
    out.extend(clause_7_4_14::checks());
    out.extend(clause_7_4_15::checks());
    out.extend(annex_e::checks());
    out.extend(annex_a::checks());
    out.extend(clause_6_2_5_3::checks());
    out
}

/// Citation used for structural parse diagnostics.
pub fn parse_diagnostic_cite() -> SpecCite {
    SpecCite::t88(
        "7.2",
        "Each segment shall have a segment header followed by the segment data bytes indicated by the segment data length field.",
    )
}

pub(crate) fn finding(
    id: CheckId,
    cite: SpecCite,
    node: Option<&SegmentNode>,
    offset: usize,
    message: impl Into<String>,
) -> Finding {
    Finding::new(
        id,
        Severity::Error,
        offset,
        node.map(|node| node.header.number),
        node.map(SegmentNode::type_label),
        message,
        cite,
    )
}

pub(crate) fn file_finding(
    id: CheckId,
    cite: SpecCite,
    offset: usize,
    message: impl Into<String>,
) -> Finding {
    Finding::new(id, Severity::Error, offset, None, None, message, cite)
}

pub(crate) fn has_segment_number(tree: &SegmentTree, number: u32) -> bool {
    tree.segments
        .iter()
        .any(|node| node.header.number == number)
}
