//! JBIG2 segment types and header parsing (spec clause 7).

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
