//! File header checks (clause 9 / Annex D).

use crate::validator::{Check, CheckCtx, CheckId, Finding, SegmentTree, SpecCite};

use super::file_finding;

struct MagicAndFlags;
struct PageCountCoherence;

/// Checks for the file header.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![Box::new(MagicAndFlags), Box::new(PageCountCoherence)]
}

impl Check for MagicAndFlags {
    fn id(&self) -> CheckId {
        CheckId("T88-D.4-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88(
            "Annex D.4",
            "The JBIG2 file header shall begin with the 8-octet ID string and the following file header flags byte.",
        )
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut out = Vec::new();
        let Some(header) = &tree.file_header else {
            return out;
        };
        if header.flags & 0xF0 != 0 {
            out.push(file_finding(
                self.id(),
                self.cite(),
                8,
                "file header uses reserved flag bits 4..7",
            ));
        }
        out
    }
}

impl Check for PageCountCoherence {
    fn id(&self) -> CheckId {
        CheckId("T88-D.4-002")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88(
            "Annex D.4",
            "If the number of pages is unknown then the number-of-pages field shall not be present; otherwise it shall contain the number of pages in the file.",
        )
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let Some(header) = &tree.file_header else {
            return Vec::new();
        };
        if !header.unknown_page_count && header.num_pages == Some(0) {
            return vec![file_finding(
                self.id(),
                self.cite(),
                9,
                "known page count is zero",
            )];
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::validator::{validate, CheckId, Lens};

    fn header_with(flags: u8, num_pages: Option<u32>) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&crate::JBIG2_MAGIC);
        out.push(flags);
        if let Some(n) = num_pages {
            out.extend_from_slice(&n.to_be_bytes());
        }
        out
    }

    fn ids(bytes: &[u8]) -> Vec<&'static str> {
        validate(bytes, Lens::StrictT88)
            .findings
            .into_iter()
            .map(|f| f.check_id.as_str())
            .collect()
    }

    #[test]
    fn magic_and_flags_negative_clean_header_passes() {
        let bytes = header_with(0x01, Some(1));
        assert!(!ids(&bytes).contains(&CheckId("T88-D.4-001").as_str()));
    }

    #[test]
    fn magic_and_flags_positive_reserved_bits_set() {
        let bytes = header_with(0x80 | 0x01, Some(1));
        assert!(ids(&bytes).contains(&CheckId("T88-D.4-001").as_str()));
    }

    #[test]
    fn page_count_coherence_negative_known_count_passes() {
        let bytes = header_with(0x01, Some(2));
        assert!(!ids(&bytes).contains(&CheckId("T88-D.4-002").as_str()));
    }

    #[test]
    fn page_count_coherence_positive_zero_count_flagged() {
        let bytes = header_with(0x01, Some(0));
        assert!(ids(&bytes).contains(&CheckId("T88-D.4-002").as_str()));
    }
}
