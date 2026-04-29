//! Profiles segment checks (T.88 clause 7.4.12).

use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct ProfilesBody;

/// Clause 7.4.12 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(ProfilesBody)]
}

impl Check for ProfilesBody {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.12-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.12", "The profiles segment shall list the profiles to which the coded representation conforms.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter_map(|node| match &node.parsed {
                ParsedBody::Profiles { bytes } if bytes.is_empty() || bytes.len() % 4 != 0 => {
                    Some(node)
                }
                _ => None,
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "profiles segment body length is not a non-zero multiple of four bytes",
                )
            })
            .collect()
    }
}
