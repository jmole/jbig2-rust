//! Colour palette checks (T.88 clause 7.4.14).

use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct ColourPaletteBody;

/// Clause 7.4.14 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(ColourPaletteBody)]
}

impl Check for ColourPaletteBody {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.14-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.14", "The colour palette segment shall describe the number and values of colours used by colour extension regions.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter_map(|node| match &node.parsed {
                ParsedBody::ColourPalette { bytes } if bytes.len() < 4 => Some(node),
                _ => None,
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "colour palette body is too short to contain a palette header",
                )
            })
            .collect()
    }
}
