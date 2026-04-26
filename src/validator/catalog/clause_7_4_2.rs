//! Symbol dictionary checks (T.88 clause 7.4.2).

use crate::segments::header::SegmentType;
use crate::validator::{Check, CheckCtx, CheckId, Finding, ParsedBody, SegmentTree, SpecCite};

use super::finding;

struct SymbolDictionaryFlags;
struct SymbolCounts;
struct RefAggForwardSafety;

/// Clause 7.4.2 checks.
pub fn checks() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(SymbolDictionaryFlags),
        Box::new(SymbolCounts),
        Box::new(RefAggForwardSafety),
    ]
}

impl Check for SymbolDictionaryFlags {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.2-001")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.2", "The symbol dictionary segment flags field contains the SDHUFF, SDREFAGG, template, table selection, and reserved bits defined by Table 18.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        let mut out = Vec::new();
        for node in &tree.segments {
            let ParsedBody::SymbolDictionary { flags, .. } = node.parsed else {
                continue;
            };
            let sdhuff = flags & 0x0001 != 0;
            let sdhuff_dh = (flags >> 2) & 0x3;
            let sdhuff_dw = (flags >> 4) & 0x3;
            let sd_rtemplate = flags & 0x1000 != 0;
            if flags & 0xE000 != 0 || sdhuff_dh == 2 || sdhuff_dw == 2 {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "symbol dictionary flags contain reserved values",
                ));
            }
            if sdhuff && ((flags >> 10) & 0x3) != 0 {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "SDTEMPLATE must be zero when SDHUFF is set",
                ));
            }
            if sd_rtemplate && flags & 0x0002 == 0 {
                out.push(finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "SDRTEMPLATE is meaningful only when SDREFAGG is set",
                ));
            }
        }
        out
    }
}

impl Check for SymbolCounts {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.2-002")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.2", "SDNUMEXSYMS and SDNUMNEWSYMS shall describe the number of exported and newly decoded symbols in the dictionary.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter_map(|node| match node.parsed {
                ParsedBody::SymbolDictionary {
                    num_ex_syms,
                    num_new_syms,
                    ..
                } => Some((node, num_ex_syms, num_new_syms)),
                _ => None,
            })
            .filter(|(_, ex, new)| ex.zip(*new).is_some_and(|(ex, new)| ex == 0 || new == 0))
            .map(|(node, _, _)| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset + node.header_len,
                    "symbol dictionary declares zero exported or new symbols",
                )
            })
            .collect()
    }
}

impl Check for RefAggForwardSafety {
    fn id(&self) -> CheckId {
        CheckId("T88-7.4.2-009")
    }

    fn cite(&self) -> SpecCite {
        SpecCite::t88("7.4.2", "When SDREFAGG is one, refinement aggregate instances shall refer only to symbols already available from the imported and previously decoded symbol sets.")
    }

    fn run(&self, _ctx: &CheckCtx, tree: &SegmentTree) -> Vec<Finding> {
        tree.segments
            .iter()
            .filter(|node| node.header.segment_type == Some(SegmentType::SymbolDictionary))
            .filter_map(|node| match node.parsed {
                ParsedBody::SymbolDictionary { flags, .. } if flags & 0x0002 != 0 => Some(node),
                _ => None,
            })
            .filter(|node| {
                node.header
                    .referred
                    .iter()
                    .any(|&r| r >= node.header.number)
            })
            .map(|node| {
                finding(
                    self.id(),
                    self.cite(),
                    Some(node),
                    node.offset,
                    "SDREFAGG dictionary has a non-preceding referred symbol dictionary; refinement aggregates with REFAGGNINST > 1 can become forward references",
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::validator::{validate, CheckId, Lens};

    fn header() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&crate::JBIG2_MAGIC);
        out.push(0x03);
        out
    }

    fn symbol_segment(number: u32, sd_flags: u16) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&sd_flags.to_be_bytes());
        body.extend_from_slice(&[0u8; 8]);
        body.extend_from_slice(&1u32.to_be_bytes());
        body.extend_from_slice(&1u32.to_be_bytes());
        body.push(0xFF);
        let mut header_bytes = Vec::new();
        header_bytes.extend_from_slice(&number.to_be_bytes());
        header_bytes.push(0x00);
        header_bytes.push(0x00);
        header_bytes.push(0x00);
        header_bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
        header_bytes.extend_from_slice(&body);
        header_bytes
    }

    fn ids(bytes: &[u8]) -> Vec<&'static str> {
        validate(bytes, Lens::StrictT88)
            .findings
            .into_iter()
            .map(|f| f.check_id.as_str())
            .collect()
    }

    #[test]
    fn symbol_dictionary_flags_negative_zero_flags_passes() {
        let mut bytes = header();
        bytes.extend(symbol_segment(1, 0x0000));
        assert!(!ids(&bytes).contains(&CheckId("T88-7.4.2-001").as_str()));
    }

    #[test]
    fn symbol_dictionary_flags_positive_reserved_high_bits() {
        let mut bytes = header();
        bytes.extend(symbol_segment(1, 0x8000));
        assert!(ids(&bytes).contains(&CheckId("T88-7.4.2-001").as_str()));
    }

    #[test]
    fn symbol_dictionary_flags_positive_sdrtemplate_without_sdrefagg() {
        let mut bytes = header();
        bytes.extend(symbol_segment(1, 0x1000));
        assert!(ids(&bytes).contains(&CheckId("T88-7.4.2-001").as_str()));
    }
}
