//! Independent structural parser for validator checks.

use crate::segments::header::SegmentType;
use crate::validator::catalog::parse_diagnostic_cite;
use crate::validator::segment_tree::segment_type_label;
use crate::validator::{
    CheckId, Finding, ParsedBody, RegionFields, SegmentNode, SegmentTree, Severity,
};
use crate::JBIG2_MAGIC;

/// File organization declared in the JBIG2 file header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileOrganization {
    /// Sequential organization.
    Sequential,
    /// Random-access organization.
    RandomAccess,
}

/// Parsed JBIG2 file header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedFileHeader {
    /// Raw flags byte.
    pub flags: u8,
    /// File organization.
    pub organization: FileOrganization,
    /// True when the page count is not encoded.
    pub unknown_page_count: bool,
    /// Number of pages when known.
    pub num_pages: Option<u32>,
}

/// Referred-to segment count encoding form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferredCountForm {
    /// Short form with count 0..4.
    Short,
    /// Long form with extended count.
    Long,
    /// Reserved form 5 or 6.
    Reserved,
}

/// Parsed segment header, including raw fields the decoder masks away.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedSegmentHeader {
    /// Segment number.
    pub number: u32,
    /// Raw segment flags byte.
    pub flags: u8,
    /// Segment type, when known.
    pub segment_type: Option<SegmentType>,
    /// True when deferred non-retain is set.
    pub deferred_non_retain: bool,
    /// True when page association is four bytes.
    pub page_association_is_32: bool,
    /// Referred segment count form.
    pub referred_count_form: ReferredCountForm,
    /// Referred segment numbers.
    pub referred: Vec<u32>,
    /// Retain bits.
    pub retain_bits: Vec<bool>,
    /// Page association.
    pub page_association: u32,
    /// Raw data length.
    pub raw_data_length: u32,
    /// Data length, or `None` for unknown length.
    pub data_length: Option<u32>,
}

/// Parse a complete stream into a structural tree.
pub fn parse(bytes: &[u8]) -> SegmentTree {
    let mut tree = SegmentTree {
        input_len: bytes.len(),
        ..SegmentTree::default()
    };
    let mut offset = parse_file_header(bytes, &mut tree);
    tree.first_segment_offset = offset;

    while offset < bytes.len() {
        match parse_segment(bytes, offset, &mut tree) {
            Some((node, next)) => {
                let eof = node.header.segment_type == Some(SegmentType::EndOfFile);
                tree.segments.push(node);
                offset = next;
                if eof {
                    break;
                }
            }
            None => break,
        }
    }
    tree
}

fn parse_file_header(bytes: &[u8], tree: &mut SegmentTree) -> usize {
    if bytes.len() < JBIG2_MAGIC.len() {
        diagnostic(tree, 0, None, None, "file shorter than JBIG2 ID string");
        return bytes.len();
    }
    if bytes[..JBIG2_MAGIC.len()] != JBIG2_MAGIC {
        diagnostic(tree, 0, None, None, "missing JBIG2 file header ID string");
        return 0;
    }
    if bytes.len() < 9 {
        diagnostic(tree, 8, None, None, "missing file header flags byte");
        return bytes.len();
    }
    let flags = bytes[8];
    let organization = if flags & 0x01 != 0 {
        FileOrganization::Sequential
    } else {
        FileOrganization::RandomAccess
    };
    let unknown_page_count = flags & 0x02 != 0;
    if unknown_page_count {
        tree.file_header = Some(ParsedFileHeader {
            flags,
            organization,
            unknown_page_count,
            num_pages: None,
        });
        9
    } else if bytes.len() < 13 {
        diagnostic(tree, 9, None, None, "file header page count is truncated");
        tree.file_header = Some(ParsedFileHeader {
            flags,
            organization,
            unknown_page_count,
            num_pages: None,
        });
        bytes.len()
    } else {
        let num_pages = read_u32(bytes, 9);
        tree.file_header = Some(ParsedFileHeader {
            flags,
            organization,
            unknown_page_count,
            num_pages,
        });
        13
    }
}

fn parse_segment(
    bytes: &[u8],
    offset: usize,
    tree: &mut SegmentTree,
) -> Option<(SegmentNode, usize)> {
    let start = offset;
    if bytes.len().saturating_sub(offset) < 6 {
        diagnostic(tree, offset, None, None, "truncated segment header");
        return None;
    }
    let number = read_u32(bytes, offset)?;
    let mut cursor = offset + 4;
    let flags = bytes[cursor];
    cursor += 1;
    let segment_type = SegmentType::from_u8(flags & 0x3F).ok();
    let deferred_non_retain = flags & 0x80 != 0;
    let page_association_is_32 = flags & 0x40 != 0;

    let count_byte = bytes[cursor];
    cursor += 1;
    let count_hi = (count_byte >> 5) & 0x07;
    let (num_ref, retain_bits, referred_count_form) = if count_hi <= 4 {
        let num_ref = count_hi as u32;
        let retain_bits = (0..=num_ref)
            .map(|i| ((count_byte >> i) & 1) != 0)
            .collect::<Vec<_>>();
        (num_ref, retain_bits, ReferredCountForm::Short)
    } else if count_hi == 7 {
        if bytes.len().saturating_sub(cursor) < 3 {
            diagnostic(
                tree,
                cursor,
                Some(number),
                Some(segment_type_label(segment_type)),
                "truncated long-form referred segment count",
            );
            return None;
        }
        let num_ref = ((count_byte as u32 & 0x1F) << 24)
            | ((bytes[cursor] as u32) << 16)
            | ((bytes[cursor + 1] as u32) << 8)
            | (bytes[cursor + 2] as u32);
        cursor += 3;
        let retain_len = (num_ref as usize + 1).div_ceil(8);
        if bytes.len().saturating_sub(cursor) < retain_len {
            diagnostic(
                tree,
                cursor,
                Some(number),
                Some(segment_type_label(segment_type)),
                "truncated long-form retain bits",
            );
            return None;
        }
        let retain_bytes = &bytes[cursor..cursor + retain_len];
        cursor += retain_len;
        let retain_bits = (0..=num_ref)
            .map(|i| {
                let byte = retain_bytes[(i as usize) / 8];
                ((byte >> (i & 7)) & 1) != 0
            })
            .collect::<Vec<_>>();
        (num_ref, retain_bits, ReferredCountForm::Long)
    } else {
        diagnostic(
            tree,
            cursor - 1,
            Some(number),
            Some(segment_type_label(segment_type)),
            "reserved referred-to segment count form",
        );
        (0, Vec::new(), ReferredCountForm::Reserved)
    };

    let ref_width = if number <= 0xFF {
        1usize
    } else if number <= 0xFFFF {
        2
    } else {
        4
    };
    let refs_len = num_ref as usize * ref_width;
    if bytes.len().saturating_sub(cursor) < refs_len {
        diagnostic(
            tree,
            cursor,
            Some(number),
            Some(segment_type_label(segment_type)),
            "truncated referred segment numbers",
        );
        return None;
    }
    let mut referred = Vec::with_capacity(num_ref as usize);
    for _ in 0..num_ref {
        let mut value = 0u32;
        for b in &bytes[cursor..cursor + ref_width] {
            value = (value << 8) | *b as u32;
        }
        referred.push(value);
        cursor += ref_width;
    }

    let page_assoc_len = if page_association_is_32 { 4 } else { 1 };
    if bytes.len().saturating_sub(cursor) < page_assoc_len + 4 {
        diagnostic(
            tree,
            cursor,
            Some(number),
            Some(segment_type_label(segment_type)),
            "truncated page association or data length",
        );
        return None;
    }
    let page_association = if page_association_is_32 {
        let v = read_u32(bytes, cursor)?;
        cursor += 4;
        v
    } else {
        let v = bytes[cursor] as u32;
        cursor += 1;
        v
    };
    let raw_data_length = read_u32(bytes, cursor)?;
    cursor += 4;
    let data_length = (raw_data_length != 0xFFFF_FFFF).then_some(raw_data_length);

    let body_end = match data_length {
        Some(len) => {
            let end = cursor.saturating_add(len as usize);
            if end > bytes.len() {
                diagnostic(
                    tree,
                    cursor,
                    Some(number),
                    Some(segment_type_label(segment_type)),
                    "segment data length exceeds remaining stream bytes",
                );
                bytes.len()
            } else {
                end
            }
        }
        None => bytes.len(),
    };
    let body = bytes[cursor..body_end].to_vec();
    let parsed = parse_body(segment_type, &body);
    let header = ParsedSegmentHeader {
        number,
        flags,
        segment_type,
        deferred_non_retain,
        page_association_is_32,
        referred_count_form,
        referred,
        retain_bits,
        page_association,
        raw_data_length,
        data_length,
    };
    Some((
        SegmentNode {
            offset: start,
            header_len: cursor - start,
            header,
            body,
            parsed,
        },
        body_end,
    ))
}

fn parse_body(segment_type: Option<SegmentType>, body: &[u8]) -> ParsedBody {
    match segment_type {
        Some(SegmentType::SymbolDictionary) => parse_symbol_dictionary(body),
        Some(SegmentType::IntermediateTextRegion)
        | Some(SegmentType::ImmediateTextRegion)
        | Some(SegmentType::ImmediateLosslessTextRegion) => parse_text_region(body),
        Some(SegmentType::PatternDictionary) => parse_pattern_dictionary(body),
        Some(SegmentType::IntermediateHalftoneRegion)
        | Some(SegmentType::ImmediateHalftoneRegion)
        | Some(SegmentType::ImmediateLosslessHalftoneRegion) => parse_halftone_region(body),
        Some(SegmentType::IntermediateGenericRegion)
        | Some(SegmentType::ImmediateGenericRegion)
        | Some(SegmentType::ImmediateLosslessGenericRegion) => parse_generic_region(body),
        Some(SegmentType::IntermediateGenericRefinementRegion)
        | Some(SegmentType::ImmediateGenericRefinementRegion)
        | Some(SegmentType::ImmediateLosslessGenericRefinementRegion) => {
            parse_refinement_region(body)
        }
        Some(SegmentType::PageInformation) => ParsedBody::PageInformation {
            width: read_u32(body, 0),
            height: read_u32(body, 4),
            flags: body.get(16).copied(),
            stripe: read_u16(body, 17),
        },
        Some(SegmentType::Profiles) => ParsedBody::Profiles {
            bytes: body.to_vec(),
        },
        Some(SegmentType::Tables) => ParsedBody::Tables {
            bytes: body.to_vec(),
        },
        Some(SegmentType::ColourPalette) => ParsedBody::ColourPalette {
            bytes: body.to_vec(),
        },
        Some(SegmentType::Extension) => ParsedBody::Extension {
            bytes: body.to_vec(),
        },
        _ => {
            if let Some(region) = parse_region_fields(body) {
                ParsedBody::Region { region }
            } else {
                ParsedBody::Unknown
            }
        }
    }
}

fn parse_symbol_dictionary(body: &[u8]) -> ParsedBody {
    let flags = match read_u16(body, 0) {
        Some(flags) => flags,
        None => return ParsedBody::Unknown,
    };
    let sdhuff = flags & 0x0001 != 0;
    let sdrefagg = flags & 0x0002 != 0;
    let sd_template = ((flags >> 10) & 0x03) as usize;
    let sd_rtemplate = flags & 0x1000 != 0;
    let mut cursor = 2usize;
    if !sdhuff {
        cursor += if sd_template == 0 { 8 } else { 2 };
    }
    if sdrefagg && !sd_rtemplate {
        cursor += 4;
    }
    ParsedBody::SymbolDictionary {
        flags,
        num_ex_syms: read_u32(body, cursor),
        num_new_syms: read_u32(body, cursor + 4),
    }
}

fn parse_text_region(body: &[u8]) -> ParsedBody {
    let Some(region) = parse_region_fields(body) else {
        return ParsedBody::Unknown;
    };
    let flags_offset = 17;
    let Some(flags) = read_u16(body, flags_offset) else {
        return ParsedBody::TextRegion {
            region,
            flags: 0,
            huff_flags: None,
            num_instances: None,
        };
    };
    let mut cursor = flags_offset + 2;
    let sbhuff = flags & 0x0001 != 0;
    let huff_flags = if sbhuff {
        let value = read_u16(body, cursor);
        cursor += 2;
        value
    } else {
        None
    };
    let sbrefine = flags & 0x0002 != 0;
    let sbr_template = flags & 0x8000 != 0;
    if sbrefine && !sbr_template {
        cursor += 4;
    }
    ParsedBody::TextRegion {
        region,
        flags,
        huff_flags,
        num_instances: read_u32(body, cursor),
    }
}

fn parse_pattern_dictionary(body: &[u8]) -> ParsedBody {
    ParsedBody::PatternDictionary {
        flags: body.first().copied().unwrap_or(0),
        width: body.get(1).copied(),
        height: body.get(2).copied(),
        gray_max: read_u32(body, 3),
    }
}

fn parse_halftone_region(body: &[u8]) -> ParsedBody {
    let Some(region) = parse_region_fields(body) else {
        return ParsedBody::Unknown;
    };
    ParsedBody::HalftoneRegion {
        region,
        flags: body.get(17).copied(),
        grid_width: read_u32(body, 18),
        grid_height: read_u32(body, 22),
    }
}

fn parse_generic_region(body: &[u8]) -> ParsedBody {
    let Some(region) = parse_region_fields(body) else {
        return ParsedBody::Unknown;
    };
    ParsedBody::GenericRegion {
        region,
        flags: body.get(17).copied(),
    }
}

fn parse_refinement_region(body: &[u8]) -> ParsedBody {
    let Some(region) = parse_region_fields(body) else {
        return ParsedBody::Unknown;
    };
    ParsedBody::RefinementRegion {
        region,
        flags: body.get(17).copied(),
    }
}

fn parse_region_fields(body: &[u8]) -> Option<RegionFields> {
    Some(RegionFields {
        width: read_u32(body, 0)?,
        height: read_u32(body, 4)?,
        x: read_u32(body, 8)?,
        y: read_u32(body, 12)?,
        flags: *body.get(16)?,
    })
}

fn diagnostic(
    tree: &mut SegmentTree,
    offset: usize,
    segment_number: Option<u32>,
    segment_type: Option<&'static str>,
    message: impl Into<String>,
) {
    tree.diagnostics.push(Finding::new(
        CheckId("T88-PARSE-001"),
        Severity::Error,
        offset,
        segment_number,
        segment_type,
        message,
        parse_diagnostic_cite(),
    ));
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset + 2)?;
    Some(u16::from_be_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}
