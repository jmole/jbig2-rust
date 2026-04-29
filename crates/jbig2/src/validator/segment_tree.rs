//! Parsed structural tree used by validator checks.

use crate::segments::header::SegmentType;

use super::{ParsedFileHeader, ParsedSegmentHeader};

/// Region segment information fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegionFields {
    /// Region bitmap width.
    pub width: u32,
    /// Region bitmap height.
    pub height: u32,
    /// X placement on page.
    pub x: u32,
    /// Y placement on page.
    pub y: u32,
    /// Raw region-info flags byte.
    pub flags: u8,
}

/// Parsed body summary for a segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParsedBody {
    /// Body was too short or not decoded by the structural parser.
    Unknown,
    /// Body belongs to a region segment and starts with region info.
    Region {
        /// Region info fields parsed from the body header.
        region: RegionFields,
    },
    /// Symbol dictionary header summary.
    SymbolDictionary {
        /// Raw dictionary flags.
        flags: u16,
        /// Exported symbol count.
        num_ex_syms: Option<u32>,
        /// New symbol count.
        num_new_syms: Option<u32>,
    },
    /// Text region header summary.
    TextRegion {
        /// Region info.
        region: RegionFields,
        /// Raw text flags.
        flags: u16,
        /// Raw Huffman flags, when present.
        huff_flags: Option<u16>,
        /// Instance count.
        num_instances: Option<u32>,
    },
    /// Pattern dictionary header summary.
    PatternDictionary {
        /// Raw pattern flags.
        flags: u8,
        /// Pattern width.
        width: Option<u8>,
        /// Pattern height.
        height: Option<u8>,
        /// Gray maximum.
        gray_max: Option<u32>,
    },
    /// Halftone region header summary.
    HalftoneRegion {
        /// Region info.
        region: RegionFields,
        /// Raw halftone flags.
        flags: Option<u8>,
        /// Grid width.
        grid_width: Option<u32>,
        /// Grid height.
        grid_height: Option<u32>,
    },
    /// Generic region header summary.
    GenericRegion {
        /// Region info.
        region: RegionFields,
        /// Raw generic flags.
        flags: Option<u8>,
    },
    /// Refinement region header summary.
    RefinementRegion {
        /// Region info.
        region: RegionFields,
        /// Raw refinement flags.
        flags: Option<u8>,
    },
    /// Page information segment.
    PageInformation {
        /// Page width.
        width: Option<u32>,
        /// Page height.
        height: Option<u32>,
        /// Raw page flags.
        flags: Option<u8>,
        /// Raw stripe flags and max stripe size.
        stripe: Option<u16>,
    },
    /// Profiles segment.
    Profiles {
        /// Profile bytes.
        bytes: Vec<u8>,
    },
    /// Tables segment.
    Tables {
        /// Raw table bytes.
        bytes: Vec<u8>,
    },
    /// Colour palette segment.
    ColourPalette {
        /// Raw palette bytes.
        bytes: Vec<u8>,
    },
    /// Extension segment.
    Extension {
        /// Raw extension bytes.
        bytes: Vec<u8>,
    },
}

impl ParsedBody {
    /// Return region fields when this is a region-style segment.
    pub fn region(&self) -> Option<RegionFields> {
        match self {
            Self::Region { region }
            | Self::TextRegion { region, .. }
            | Self::HalftoneRegion { region, .. }
            | Self::GenericRegion { region, .. }
            | Self::RefinementRegion { region, .. } => Some(*region),
            _ => None,
        }
    }
}

/// One parsed segment node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentNode {
    /// Header byte offset.
    pub offset: usize,
    /// Header length in bytes.
    pub header_len: usize,
    /// Parsed header.
    pub header: ParsedSegmentHeader,
    /// Segment body bytes.
    pub body: Vec<u8>,
    /// Body summary.
    pub parsed: ParsedBody,
}

impl SegmentNode {
    /// Static segment type label.
    pub fn type_label(&self) -> &'static str {
        segment_type_label(self.header.segment_type)
    }
}

/// Complete parsed JBIG2 stream tree.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SegmentTree {
    /// Input length in bytes.
    pub input_len: usize,
    /// Parsed file header, when present.
    pub file_header: Option<ParsedFileHeader>,
    /// File-level byte offset where segments begin.
    pub first_segment_offset: usize,
    /// Parsed segments.
    pub segments: Vec<SegmentNode>,
    /// Structural parse diagnostics emitted as validator findings later.
    pub diagnostics: Vec<crate::validator::Finding>,
}

/// Human-readable segment type label.
pub fn segment_type_label(segment_type: Option<SegmentType>) -> &'static str {
    match segment_type {
        Some(SegmentType::SymbolDictionary) => "SymbolDictionary",
        Some(SegmentType::IntermediateTextRegion) => "IntermediateTextRegion",
        Some(SegmentType::ImmediateTextRegion) => "ImmediateTextRegion",
        Some(SegmentType::ImmediateLosslessTextRegion) => "ImmediateLosslessTextRegion",
        Some(SegmentType::PatternDictionary) => "PatternDictionary",
        Some(SegmentType::IntermediateHalftoneRegion) => "IntermediateHalftoneRegion",
        Some(SegmentType::ImmediateHalftoneRegion) => "ImmediateHalftoneRegion",
        Some(SegmentType::ImmediateLosslessHalftoneRegion) => "ImmediateLosslessHalftoneRegion",
        Some(SegmentType::IntermediateGenericRegion) => "IntermediateGenericRegion",
        Some(SegmentType::ImmediateGenericRegion) => "ImmediateGenericRegion",
        Some(SegmentType::ImmediateLosslessGenericRegion) => "ImmediateLosslessGenericRegion",
        Some(SegmentType::IntermediateGenericRefinementRegion) => {
            "IntermediateGenericRefinementRegion"
        }
        Some(SegmentType::ImmediateGenericRefinementRegion) => "ImmediateGenericRefinementRegion",
        Some(SegmentType::ImmediateLosslessGenericRefinementRegion) => {
            "ImmediateLosslessGenericRefinementRegion"
        }
        Some(SegmentType::PageInformation) => "PageInformation",
        Some(SegmentType::EndOfPage) => "EndOfPage",
        Some(SegmentType::EndOfStripe) => "EndOfStripe",
        Some(SegmentType::EndOfFile) => "EndOfFile",
        Some(SegmentType::Profiles) => "Profiles",
        Some(SegmentType::Tables) => "Tables",
        Some(SegmentType::ColourPalette) => "ColourPalette",
        Some(SegmentType::Extension) => "Extension",
        None => "Unknown",
    }
}
