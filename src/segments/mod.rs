//! JBIG2 segment types and header parsing (spec clause 7).

use std::ops::Index;

pub mod colour_palette;
pub mod end_of_page;
pub mod file_header;
pub mod generic_region;
pub mod halftone_region;
pub mod header;
pub mod page_information;
pub mod pattern_dictionary;
pub mod refinement_region;
pub mod region_info;
pub mod symbol_dictionary;
pub mod text_region;

/// AT-pixel storage with an explicit active length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtPixels {
    data: [(i8, i8); 12],
    len: u8,
}

impl AtPixels {
    /// Create a new AT-pixel set from a packed 12-slot buffer and an active
    /// length.
    pub const fn new(data: [(i8, i8); 12], len: u8) -> Self {
        Self { data, len }
    }

    /// Number of active AT pixels stored in this value.
    pub const fn len(self) -> usize {
        self.len as usize
    }

    /// Return the zero-padded 12-slot backing array.
    pub const fn as_array(self) -> [(i8, i8); 12] {
        self.data
    }

    /// Borrow the active AT pixels as a slice.
    pub fn as_slice(&self) -> &[(i8, i8)] {
        &self.data[..self.len()]
    }

    /// Build an AT-pixel set from an array of up to 12 active entries.
    pub fn from_array<const N: usize>(data: [(i8, i8); N]) -> Self {
        let mut full = [(0i8, 0i8); 12];
        let mut i = 0;
        while i < N {
            full[i] = data[i];
            i += 1;
        }
        Self::new(full, N as u8)
    }

    /// Return the first four AT pixels as a fixed-size array.
    pub fn to_array_4(self) -> [(i8, i8); 4] {
        [self.data[0], self.data[1], self.data[2], self.data[3]]
    }

    /// Return the first two AT pixels as a fixed-size array.
    pub fn to_array_2(self) -> [(i8, i8); 2] {
        [self.data[0], self.data[1]]
    }
}

impl Default for AtPixels {
    fn default() -> Self {
        Self::new([(0, 0); 12], 0)
    }
}

impl Index<usize> for AtPixels {
    type Output = (i8, i8);

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

pub use colour_palette::{ColourPalette, ColourPaletteHeader, DEFAULT_COLOURS};
pub use end_of_page::EndOfPage;
pub use file_header::FileHeader;
pub use generic_region::{GenericRegionHeader, nominal_at};
pub use halftone_region::{
    HalftoneRegionHeader, decode_halftone_region, encode_halftone_region,
};
pub use header::{SegmentHeader, SegmentType};
pub use page_information::{CombinationOp, PageInformation};
pub use pattern_dictionary::{
    PatternDictionaryHeader, decode_pattern_dictionary, encode_pattern_dictionary,
};
pub use region_info::RegionInfo;
pub use refinement_region::{
    decode_refinement_region, encode_refinement_region, RefinementRegionHeader,
    NOMINAL_REFINEMENT_AT,
};
pub use symbol_dictionary::{
    DecodedSymbolDictionary, SymbolDictionaryHeader, decode_symbol_dictionary,
    encode_symbol_dictionary,
};
pub use text_region::{
    RefCorner, SymbolInstance, TextRegionHeader, decode_text_region, encode_text_region,
    sym_code_len,
};
