//! High-level JBIG2 decoder.
//!
//! The decoder supports the sequential organisation defined in spec D.1 and
//! currently handles page-information, end-of-page, and generic-region
//! segments. Additional segment families (symbol dictionary, text region,
//! refinement, halftone) are added in milestones M3..M5.
//!
//! The page is composited according to the page-info default pixel value +
//! combination operator and the per-region combination operator.

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::sync::Arc;

use crate::bitmap::Bitmap;
use crate::coding::mq::{MqContexts, MqDecoder, MQ_NUM_CONTEXTS};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::rgb_bitmap::RgbBitmap;
use crate::segments::{
    colour_palette::{ColourPalette, DEFAULT_COLOURS},
    generic_region::{self, GenericRegionHeader},
    halftone_region::{self, HalftoneRegionHeader},
    pattern_dictionary::{self, PatternDictionaryHeader},
    refinement_region::{self, RefinementRegionHeader},
    symbol_dictionary::{self, SymbolDictionaryHeader},
    text_region::{self, TextRegionHeader},
    CombinationOp, FileHeader, PageInformation, RegionInfo, SegmentHeader, SegmentType,
};

/// A decoded page from a JBIG2 file.
#[derive(Clone, Debug)]
pub struct DecodedPage {
    /// Page information header (dimensions, flags, striping).
    pub info: PageInformation,
    /// Fully-composited page bitmap for monochrome pages.
    pub bitmap: Bitmap,
    /// Fully-composited RGB page for colour pages (AMD3).
    pub rgb_bitmap: Option<RgbBitmap>,
}

/// Multi-page JBIG2 decoder.
pub struct Jbig2Decoder<R: Read + Seek> {
    reader: R,
    header: FileHeader,
    /// Segment headers in stream order. Data-length `None` is only allowed
    /// for the terminal segment of an embedded / streaming file.
    segments: Vec<SegmentDescriptor>,
    /// Scratch buffer reused across segment-body reads. Segments are
    /// usually modest in size but files can contain dozens of them, so
    /// paying a fresh allocation per segment is wasteful. The first segment
    /// sizes the Vec and subsequent segments just copy into it.
    body_scratch: Vec<u8>,
    /// Reusable MQ context pool reset and reused for every arithmetic-coded
    /// segment family.
    mq_cxs: MqContexts,
}

#[derive(Clone, Debug)]
struct SegmentDescriptor {
    header: SegmentHeader,
    /// Offset of the segment *data* in the stream.
    data_offset: u64,
    /// Data length. We require a known length for the v1 decoder.
    data_len: u32,
}

enum DecodedTextRegion {
    Mono(TextRegionHeader, Bitmap),
    Colour(TextRegionHeader, RgbBitmap),
}

enum T45ColourData {
    PaletteIds(Vec<u32>),
    DirectRgb(Vec<[u8; 3]>),
}

fn decode_t45_colour_data(data: &[u8]) -> Jbig2Result<T45ColourData> {
    if data.len() < 6 {
        return Err(Jbig2Error::UnexpectedEof { needed: 6 });
    }
    let num_components = data[0] as usize;
    let component_len = data[1] as usize;
    let num_vals = u32::from_be_bytes(data[2..6].try_into().unwrap()) as usize;
    if num_components == 0 {
        return Err(Jbig2Error::InvalidConfig(
            "text region: T.45 colour section has zero components",
        ));
    }
    let mut cur = 6usize;
    let mut values = vec![0u32; num_vals * num_components];
    let mut i = 0usize;
    while i < num_vals {
        if cur >= data.len() {
            return Err(Jbig2Error::UnexpectedEof { needed: 1 });
        }
        let mut run_len = data[cur] as usize;
        cur += 1;
        if run_len == 0 {
            if cur + 2 > data.len() {
                return Err(Jbig2Error::UnexpectedEof { needed: 2 });
            }
            run_len = u16::from_be_bytes(data[cur..cur + 2].try_into().unwrap()) as usize;
            cur += 2;
        }
        let mut sample = vec![0u32; num_components];
        for comp in sample.iter_mut() {
            match component_len {
                1 => {
                    if cur + 1 > data.len() {
                        return Err(Jbig2Error::UnexpectedEof { needed: 1 });
                    }
                    *comp = data[cur] as u32;
                    cur += 1;
                }
                2 => {
                    if cur + 2 > data.len() {
                        return Err(Jbig2Error::UnexpectedEof { needed: 2 });
                    }
                    *comp = u16::from_be_bytes(data[cur..cur + 2].try_into().unwrap()) as u32;
                    cur += 2;
                }
                4 => {
                    if cur + 4 > data.len() {
                        return Err(Jbig2Error::UnexpectedEof { needed: 4 });
                    }
                    *comp = u32::from_be_bytes(data[cur..cur + 4].try_into().unwrap());
                    cur += 4;
                }
                _ => {
                    return Err(Jbig2Error::Unsupported(
                        "text region: unsupported T.45 component length",
                    ));
                }
            }
        }
        if i + run_len > num_vals {
            return Err(Jbig2Error::OutOfRange(
                "text region: T.45 run exceeds colour count",
            ));
        }
        let first_dst = i * num_components;
        values[first_dst..first_dst + num_components].copy_from_slice(&sample);
        for run_idx in 1..run_len {
            let dst_start = (i + run_idx) * num_components;
            values[dst_start..dst_start + num_components].copy_from_slice(&sample);
        }
        i += run_len;
    }

    match num_components {
        1 => Ok(T45ColourData::PaletteIds(
            values
                .chunks_exact(num_components)
                .map(|sample| sample[0])
                .collect(),
        )),
        3.. => {
            let mut out = Vec::with_capacity(num_vals);
            for sample in values.chunks_exact(num_components) {
                out.push([sample[0] as u8, sample[1] as u8, sample[2] as u8]);
            }
            Ok(T45ColourData::DirectRgb(out))
        }
        _ => Err(Jbig2Error::Unsupported(
            "text region: unsupported T.45 component count",
        )),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FallbackPolicy {
    All,
    LatestOnly,
}

fn read_segment_body<'a, R: Read + Seek>(
    reader: &mut R,
    scratch: &'a mut Vec<u8>,
    seg: &SegmentDescriptor,
) -> Jbig2Result<&'a [u8]> {
    reader
        .seek(SeekFrom::Start(seg.data_offset))
        .map_err(Jbig2Error::from)?;
    scratch.clear();
    scratch.reserve(seg.data_len as usize);
    let mut limited = reader.take(seg.data_len as u64);
    limited.read_to_end(scratch).map_err(Jbig2Error::from)?;
    let expected = seg.data_len as usize;
    if scratch.len() != expected {
        return Err(Jbig2Error::UnexpectedEof {
            needed: expected - scratch.len(),
        });
    }
    Ok(scratch.as_slice())
}

fn parse_segment_header<T, F>(body: &[u8], parse: F) -> Jbig2Result<(T, &[u8])>
where
    F: for<'a> FnOnce(&mut Cursor<&'a [u8]>) -> Jbig2Result<T>,
{
    let mut cur = Cursor::new(body);
    let header = parse(&mut cur)?;
    let header_len = cur.position() as usize;
    Ok((header, &body[header_len..]))
}

fn fallback_referred_numbers<F>(
    segments: &[SegmentDescriptor],
    idx: usize,
    mut matches_kind: F,
    policy: FallbackPolicy,
) -> Vec<u32>
where
    F: FnMut(SegmentType) -> bool,
{
    let page_assoc = segments[idx].header.page_association;
    let mut out = Vec::new();
    for prior in &segments[..idx] {
        if !matches_kind(prior.header.segment_type) {
            continue;
        }
        let p = prior.header.page_association;
        if p != 0 && p != page_assoc {
            continue;
        }
        match policy {
            FallbackPolicy::All => out.push(prior.header.number),
            FallbackPolicy::LatestOnly => {
                out.clear();
                out.push(prior.header.number);
            }
        }
    }
    out
}

impl<R: Read + Seek> Jbig2Decoder<R> {
    /// Parse the file header and enumerate every segment header in the
    /// stream. The segment data is not read eagerly; [`Jbig2Decoder::decode_page`]
    /// seeks back to the stored offsets on demand.
    pub fn new(mut reader: R) -> Jbig2Result<Self> {
        let header = FileHeader::read(&mut reader)?;
        let mut segments = Vec::new();
        loop {
            let seg_header = match SegmentHeader::read(&mut reader) {
                Ok(h) => h,
                Err(Jbig2Error::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            };
            let data_offset = reader.stream_position().map_err(Jbig2Error::from)?;
            let data_len = seg_header
                .data_length
                .ok_or(Jbig2Error::Unsupported("segment with unknown data length"))?;
            let next_offset = data_offset + data_len as u64;
            reader
                .seek(SeekFrom::Start(next_offset))
                .map_err(Jbig2Error::from)?;
            let end_of_file = matches!(seg_header.segment_type, SegmentType::EndOfFile);
            segments.push(SegmentDescriptor {
                header: seg_header,
                data_offset,
                data_len,
            });
            if end_of_file {
                break;
            }
        }
        Ok(Self {
            reader,
            header,
            segments,
            body_scratch: Vec::new(),
            mq_cxs: MqContexts::new(MQ_NUM_CONTEXTS),
        })
    }

    /// Return the parsed file header.
    pub fn file_header(&self) -> &FileHeader {
        &self.header
    }

    /// Return the parsed segment headers.
    pub fn segment_headers(&self) -> impl Iterator<Item = &SegmentHeader> {
        self.segments.iter().map(|s| &s.header)
    }

    /// Number of distinct pages referenced by the file.
    pub fn num_pages(&self) -> u32 {
        let mut max = 0u32;
        for s in &self.segments {
            if matches!(s.header.segment_type, SegmentType::PageInformation) {
                max = max.max(s.header.page_association);
            }
        }
        max
    }

    /// Decode a single page. `page_number` is 1-based to match the spec.
    pub fn decode_page(&mut self, page_number: u32) -> Jbig2Result<DecodedPage> {
        // Locate the PageInformation segment.
        let info_idx = self
            .segments
            .iter()
            .position(|s| {
                matches!(s.header.segment_type, SegmentType::PageInformation)
                    && s.header.page_association == page_number
            })
            .ok_or(Jbig2Error::Unsupported("page not found"))?;
        let info = self.read_page_information(info_idx)?;

        // Allocate the page bitmap; if the height is unknown at encode time
        // we grow it as we encounter regions.
        let (page_w, mut page_h) = (info.width, info.height);
        let height_is_unknown = info.height == 0xFFFF_FFFF;
        if height_is_unknown {
            page_h = 0;
        }
        let mut page = Bitmap::filled(page_w, page_h, info.default_pixel)?;
        let mut rgb_page = if info.may_contain_colour {
            Some(RgbBitmap::filled(page_w, page_h, [255, 255, 255])?)
        } else {
            None
        };

        // Symbol dictionaries accumulate across segments; each key is the
        // global segment number of the dictionary segment, value is its
        // exported symbols. Dictionaries may reference other dictionaries
        // via the segment header's referred-to list.
        let mut sym_dicts: HashMap<u32, Vec<Arc<Bitmap>>> = HashMap::new();
        // Pattern dictionaries accumulate similarly for later halftone
        // region segments.
        let mut pattern_dicts: HashMap<u32, Vec<Arc<Bitmap>>> = HashMap::new();
        // Colour palette segments contribute additional RGB entries that
        // colour-extended text regions may refer to by palette ID.
        let mut colour_palettes: HashMap<u32, Vec<[u8; 3]>> = HashMap::new();
        // Region bitmaps emitted so far on this page, keyed by segment
        // number. Refinement region segments may name one of these as
        // their reference bitmap via the referred-to list (spec 7.4.7.1);
        // we keep the raw region bitmap rather than the composited page
        // because the reference pixels must be exactly the decoder output
        // of the producing segment.
        let mut region_bitmaps: HashMap<u32, Bitmap> = HashMap::new();

        // Iterate every segment associated with this page in stream order.
        // Symbol dictionaries referenced by later text regions may live on
        // page 0 (global) — we decode those eagerly too.
        for idx in 0..self.segments.len() {
            let seg = &self.segments[idx];
            let on_this_page = seg.header.page_association == page_number;
            let is_global = seg.header.page_association == 0;
            match seg.header.segment_type {
                SegmentType::SymbolDictionary => {
                    if !on_this_page && !is_global {
                        continue;
                    }
                    let (number, syms) = self.decode_symbol_dictionary(idx, &sym_dicts)?;
                    sym_dicts.insert(number, syms);
                }
                SegmentType::PatternDictionary => {
                    if !on_this_page && !is_global {
                        continue;
                    }
                    let (number, pats) = self.decode_pattern_dictionary(idx)?;
                    pattern_dicts.insert(number, pats);
                }
                SegmentType::ColourPalette => {
                    if !on_this_page && !is_global {
                        continue;
                    }
                    let (number, cols) = self.decode_colour_palette(idx)?;
                    colour_palettes.insert(number, cols);
                }
                _ if !on_this_page => continue,
                SegmentType::PageInformation | SegmentType::EndOfPage => continue,
                SegmentType::ImmediateGenericRegion
                | SegmentType::ImmediateLosslessGenericRegion
                | SegmentType::IntermediateGenericRegion => {
                    let seg_no = self.segments[idx].header.number;
                    let (hdr, region_bitmap) = self.decode_generic_region(idx)?;
                    if rgb_page.is_none() {
                        composite_region(
                            &mut page,
                            &region_bitmap,
                            &hdr.region,
                            hdr.region.external_combination_op,
                            &info,
                            height_is_unknown,
                        )?;
                    }
                    region_bitmaps.insert(seg_no, region_bitmap);
                }
                SegmentType::ImmediateTextRegion
                | SegmentType::ImmediateLosslessTextRegion
                | SegmentType::IntermediateTextRegion => {
                    let seg_no = self.segments[idx].header.number;
                    match self.decode_text_region(idx, &sym_dicts, &colour_palettes)? {
                        DecodedTextRegion::Mono(hdr, region_bitmap) => {
                            if rgb_page.is_none() {
                                composite_region(
                                    &mut page,
                                    &region_bitmap,
                                    &hdr.region,
                                    hdr.region.external_combination_op,
                                    &info,
                                    height_is_unknown,
                                )?;
                            }
                            region_bitmaps.insert(seg_no, region_bitmap);
                        }
                        DecodedTextRegion::Colour(hdr, region_bitmap) => {
                            if let Some(page_rgb) = rgb_page.as_mut() {
                                page_rgb.composite_replace(
                                    &region_bitmap,
                                    hdr.region.x as i32,
                                    hdr.region.y as i32,
                                );
                            } else {
                                return Err(Jbig2Error::InvalidConfig(
                                    "text region: colour region on monochrome page",
                                ));
                            }
                        }
                    }
                }
                SegmentType::ImmediateGenericRefinementRegion
                | SegmentType::ImmediateLosslessGenericRefinementRegion
                | SegmentType::IntermediateGenericRefinementRegion => {
                    let seg_no = self.segments[idx].header.number;
                    let (hdr, region_bitmap) =
                        self.decode_refinement_region(idx, &region_bitmaps, &page)?;
                    composite_region(
                        &mut page,
                        &region_bitmap,
                        &hdr.region,
                        hdr.region.external_combination_op,
                        &info,
                        height_is_unknown,
                    )?;
                    region_bitmaps.insert(seg_no, region_bitmap);
                }
                SegmentType::IntermediateHalftoneRegion
                | SegmentType::ImmediateHalftoneRegion
                | SegmentType::ImmediateLosslessHalftoneRegion => {
                    let seg_no = self.segments[idx].header.number;
                    let (hdr, region_bitmap) = self.decode_halftone_region(idx, &pattern_dicts)?;
                    if rgb_page.is_none() {
                        composite_region(
                            &mut page,
                            &region_bitmap,
                            &hdr.region,
                            hdr.region.external_combination_op,
                            &info,
                            height_is_unknown,
                        )?;
                    }
                    region_bitmaps.insert(seg_no, region_bitmap);
                }
                SegmentType::EndOfFile | SegmentType::EndOfStripe | SegmentType::Tables => continue,
                other => {
                    return Err(Jbig2Error::Unsupported(segment_type_name(other)));
                }
            }
        }
        Ok(DecodedPage {
            info,
            bitmap: page,
            rgb_bitmap: rgb_page,
        })
    }

    fn read_page_information(&mut self, idx: usize) -> Jbig2Result<PageInformation> {
        let seg = &self.segments[idx];
        self.reader
            .seek(SeekFrom::Start(seg.data_offset))
            .map_err(Jbig2Error::from)?;
        PageInformation::read(&mut self.reader)
    }

    fn decode_symbol_dictionary(
        &mut self,
        idx: usize,
        sym_dicts: &HashMap<u32, Vec<Arc<Bitmap>>>,
    ) -> Jbig2Result<(u32, Vec<Arc<Bitmap>>)> {
        let seg_no = self.segments[idx].header.number;
        // Borrow imports by reference so we never clone upstream
        // dictionaries just to pass them into the decoder. This is
        // especially important for large lossless dictionaries whose
        // bitmaps can dominate decoder memory traffic.
        //
        // Iterating the referred list directly (instead of a local
        // `referred.clone()`) also drops a small but avoidable
        // per-segment allocation: the scan only needs a shared borrow
        // of the segment header, which is released before the later
        // split-borrow of `self`.
        let mut import: Vec<&Bitmap> = Vec::new();
        for ref_no in &self.segments[idx].header.referred {
            if let Some(syms) = sym_dicts.get(ref_no) {
                import.extend(syms.iter().map(Arc::as_ref));
            }
        }
        if import.is_empty() {
            // Lenient fallback (see `decode_text_region`): the
            // reference encoder leaves the referred-to list empty and
            // expects every prior in-scope symbol dictionary to be
            // imported implicitly. Conformance set TT5 needs this for
            // its second symbol dictionary.
            for ref_no in fallback_referred_numbers(
                &self.segments,
                idx,
                |kind| matches!(kind, SegmentType::SymbolDictionary),
                FallbackPolicy::All,
            ) {
                if let Some(syms) = sym_dicts.get(&ref_no) {
                    import.extend(syms.iter().map(Arc::as_ref));
                }
            }
        }
        let Self {
            reader,
            segments,
            body_scratch,
            mq_cxs,
            ..
        } = self;
        let seg = &segments[idx];
        let body = read_segment_body(reader, body_scratch, seg)?;
        let (header, coded) = parse_segment_header(body, |cur| SymbolDictionaryHeader::read(cur))?;

        let decoded = symbol_dictionary::decode_symbol_dictionary_with_contexts(
            &header, coded, &import, mq_cxs,
        )?;
        Ok((seg_no, decoded.exported.into_iter().map(Arc::new).collect()))
    }

    fn decode_text_region(
        &mut self,
        idx: usize,
        sym_dicts: &HashMap<u32, Vec<Arc<Bitmap>>>,
        colour_palettes: &HashMap<u32, Vec<[u8; 3]>>,
    ) -> Jbig2Result<DecodedTextRegion> {
        // SBSYMS is the concatenation of every referred-to symbol
        // dictionary's exported list, in referred-to order. The `&Bitmap`
        // references avoid cloning every dictionary bitmap per text
        // region; on symbol-heavy pages this was previously a major
        // allocation driver.
        //
        // The referred list is iterated by shared borrow (no local
        // `Vec<u32>` clone) so we avoid a small per-region allocation;
        // the borrow of the segment header is released before the
        // split-borrow of `self` below.
        let mut sbsyms: Vec<&Bitmap> = Vec::new();
        for ref_no in &self.segments[idx].header.referred {
            if let Some(syms) = sym_dicts.get(ref_no) {
                sbsyms.extend(syms.iter().map(Arc::as_ref));
            }
        }
        if sbsyms.is_empty() {
            // Lenient fallback: the ITU-T T.88 reference encoder
            // (Jbig2ENC.cpp) emits text-region segment headers with an
            // empty referred-to list and relies on positional ordering.
            // Its decoder maintains a single global symbol chain that
            // each SD pushes its new symbols onto, then a TR walks the
            // entire chain.
            //
            // Because each SD already re-exports the symbols it
            // imported from earlier SDs (TT5 SD#2 has SDNUMEXSYMS = 3
            // = 2 imports + 1 new), simply using the most recent SD on
            // the page reproduces the full chain without duplicating
            // imported symbols.
            for ref_no in fallback_referred_numbers(
                &self.segments,
                idx,
                |kind| matches!(kind, SegmentType::SymbolDictionary),
                FallbackPolicy::LatestOnly,
            ) {
                if let Some(syms) = sym_dicts.get(&ref_no) {
                    sbsyms.extend(syms.iter().map(Arc::as_ref));
                }
            }
            if sbsyms.is_empty() {
                return Err(Jbig2Error::OutOfRange(
                    "text region: no referred symbol dictionary",
                ));
            }
        }
        let Self {
            reader,
            segments,
            body_scratch,
            mq_cxs,
            ..
        } = self;
        let seg = &segments[idx];
        let body = read_segment_body(reader, body_scratch, seg)?;
        let (header, rest) = parse_segment_header(body, |cur| TextRegionHeader::read(cur))?;
        let header_len = body.len() - rest.len();
        let data_len = body.len();
        if header.region.colour_extension {
            let colour_total_len =
                u32::from_be_bytes(body[data_len - 4..data_len].try_into().unwrap()) as usize;
            if colour_total_len < 4 || colour_total_len > data_len - header_len {
                return Err(Jbig2Error::OutOfRange(
                    "text region: invalid colour-section length",
                ));
            }
            let colour_start = data_len - colour_total_len;
            let coded = &body[header_len..colour_start];
            let mut palette = DEFAULT_COLOURS.to_vec();
            for ref_no in &self.segments[idx].header.referred {
                if let Some(cols) = colour_palettes.get(ref_no) {
                    palette.extend_from_slice(cols);
                }
            }
            let colours = match decode_t45_colour_data(&body[colour_start..data_len - 4])? {
                T45ColourData::DirectRgb(cols) => cols,
                T45ColourData::PaletteIds(ids) => ids
                    .into_iter()
                    .map(|id| {
                        palette
                            .get(id as usize)
                            .copied()
                            .ok_or(Jbig2Error::OutOfRange(
                                "text region: colour palette ID out of range",
                            ))
                    })
                    .collect::<Jbig2Result<Vec<_>>>()?,
            };
            let region = text_region::decode_text_region_colour_with_contexts(
                &header, coded, &sbsyms, &colours, mq_cxs,
            )?;
            Ok(DecodedTextRegion::Colour(header, region))
        } else {
            let coded = &body[header_len..];
            let region =
                text_region::decode_text_region_with_contexts(&header, coded, &sbsyms, mq_cxs)?;
            Ok(DecodedTextRegion::Mono(header, region))
        }
    }

    fn decode_pattern_dictionary(&mut self, idx: usize) -> Jbig2Result<(u32, Vec<Arc<Bitmap>>)> {
        let seg_no = self.segments[idx].header.number;
        let Self {
            reader,
            segments,
            body_scratch,
            mq_cxs,
            ..
        } = self;
        let seg = &segments[idx];
        let body = read_segment_body(reader, body_scratch, seg)?;
        let (header, coded) = parse_segment_header(body, |cur| PatternDictionaryHeader::read(cur))?;
        let patterns =
            pattern_dictionary::decode_pattern_dictionary_with_contexts(&header, coded, mq_cxs)?;
        Ok((seg_no, patterns.into_iter().map(Arc::new).collect()))
    }

    fn decode_colour_palette(&mut self, idx: usize) -> Jbig2Result<(u32, Vec<[u8; 3]>)> {
        let seg_no = self.segments[idx].header.number;
        let Self {
            reader,
            segments,
            body_scratch,
            ..
        } = self;
        let seg = &segments[idx];
        let body = read_segment_body(reader, body_scratch, seg)?;
        let palette = ColourPalette::decode(body)?;
        Ok((seg_no, palette.rgb_values()?))
    }

    fn decode_halftone_region(
        &mut self,
        idx: usize,
        pattern_dicts: &HashMap<u32, Vec<Arc<Bitmap>>>,
    ) -> Jbig2Result<(HalftoneRegionHeader, Bitmap)> {
        let mut patterns: Vec<&Bitmap> = Vec::new();
        for ref_no in &self.segments[idx].header.referred {
            if let Some(pats) = pattern_dicts.get(ref_no) {
                patterns.extend(pats.iter().map(Arc::as_ref));
            }
        }
        if patterns.is_empty() {
            for ref_no in fallback_referred_numbers(
                &self.segments,
                idx,
                |kind| matches!(kind, SegmentType::PatternDictionary),
                FallbackPolicy::LatestOnly,
            ) {
                if let Some(pats) = pattern_dicts.get(&ref_no) {
                    patterns.extend(pats.iter().map(Arc::as_ref));
                }
            }
            if patterns.is_empty() {
                return Err(Jbig2Error::OutOfRange(
                    "halftone region: no referred pattern dictionary",
                ));
            }
        }

        let Self {
            reader,
            segments,
            body_scratch,
            mq_cxs,
            ..
        } = self;
        let seg = &segments[idx];
        let body = read_segment_body(reader, body_scratch, seg)?;
        let (header, coded) = parse_segment_header(body, |cur| HalftoneRegionHeader::read(cur))?;
        let region = halftone_region::decode_halftone_region_with_contexts(
            &header, coded, &patterns, mq_cxs,
        )?;
        Ok((header, region))
    }

    fn decode_generic_region(&mut self, idx: usize) -> Jbig2Result<(GenericRegionHeader, Bitmap)> {
        let Self {
            reader,
            segments,
            body_scratch,
            mq_cxs,
            ..
        } = self;
        let seg = &segments[idx];
        let body = read_segment_body(reader, body_scratch, seg)?;
        let (header, coded) = parse_segment_header(body, |cur| GenericRegionHeader::read(cur))?;
        let bitmap = if header.mmr {
            #[cfg(feature = "mmr")]
            {
                generic_region::decode_generic_mmr(
                    coded,
                    header.region.width,
                    header.region.height,
                )?
            }
            #[cfg(not(feature = "mmr"))]
            {
                return Err(Jbig2Error::Unsupported("mmr feature disabled"));
            }
        } else {
            // The context pool is per-region per spec §7.4.2.1, so we
            // reset the decoder-owned pool here instead of allocating a
            // fresh [`MqContexts`] every time.
            mq_cxs.reset();
            let mut dec = MqDecoder::new(coded);
            generic_region::decode_generic_arith(&mut dec, mq_cxs, &header)?
        };
        Ok((header, bitmap))
    }

    /// Decode a standalone generic refinement region segment.
    ///
    /// Per spec 7.4.7.1 the reference bitmap is either the single
    /// region bitmap named in the segment's referred-to list, or (when
    /// the referred-to list is empty) the current page bitmap cropped to
    /// the refinement region's bounding box. We use the page bitmap as
    /// it stands at this point in the stream so that overlapping regions
    /// emitted earlier on the page contribute to the reference.
    fn decode_refinement_region(
        &mut self,
        idx: usize,
        region_bitmaps: &HashMap<u32, Bitmap>,
        page: &Bitmap,
    ) -> Jbig2Result<(RefinementRegionHeader, Bitmap)> {
        // Decide which bitmap to use as the reference *before* we take
        // the split-borrow on `self` for the MQ decode. If the segment
        // names a referred-to region we clone the match (refinement is
        // rare enough that cloning the packed bytes once per segment is
        // cheaper than carrying extra lifetime machinery through the
        // decode path).
        let referred: Vec<u32> = self.segments[idx].header.referred.clone();

        let Self {
            reader,
            segments,
            body_scratch,
            mq_cxs,
            ..
        } = self;
        let seg = &segments[idx];
        let body = read_segment_body(reader, body_scratch, seg)?;
        let (header, coded) = parse_segment_header(body, |cur| RefinementRegionHeader::read(cur))?;

        // Locate the reference bitmap + its placement on the page so we
        // can crop/pad it to the refinement-region coordinate system.
        let (reference, reference_dx, reference_dy) =
            refinement_reference_for(&referred, region_bitmaps, page, &header)?;

        // Spec §7.4.2.1: contexts reset per region, so we reuse the
        // decoder-owned pool rather than allocating a fresh one.
        mq_cxs.reset();
        let mut dec = MqDecoder::new(coded);
        let region = refinement_region::decode_refinement_region(
            &mut dec,
            mq_cxs,
            header.region.width,
            header.region.height,
            header.template,
            header.tpgron,
            &header.at.to_array_2(),
            &reference,
            reference_dx,
            reference_dy,
        )?;
        Ok((header, region))
    }
}

/// Resolve the reference bitmap for a refinement region segment.
///
/// When the segment's referred-to list names exactly one region bitmap
/// we take it verbatim (with zero translation; refinement segments do
/// not carry a GRREFERENCEDX/DY field on the wire — any offset lives in
/// the placement of the referred-to region). Otherwise we fall back to
/// the page bitmap masked to the refinement region's box.
fn refinement_reference_for<'a>(
    referred: &'a [u32],
    region_bitmaps: &'a HashMap<u32, Bitmap>,
    page: &'a Bitmap,
    header: &'a RefinementRegionHeader,
) -> Jbig2Result<(Cow<'a, Bitmap>, i32, i32)> {
    for seg_no in referred {
        if let Some(bm) = region_bitmaps.get(seg_no) {
            return Ok((Cow::Borrowed(bm), 0, 0));
        }
    }
    // No decoded region reference was available; snapshot the current page
    // under this region's bounds so refinement can use local coordinates.
    let w = header.region.width;
    let h = header.region.height;
    let mut bm = Bitmap::new(w, h)?;
    bm.composite(
        page,
        -(header.region.x as i32),
        -(header.region.y as i32),
        crate::bitmap::BlitOp::Replace,
    );
    Ok((Cow::Owned(bm), 0, 0))
}

fn composite_region(
    page: &mut Bitmap,
    region: &Bitmap,
    region_info: &RegionInfo,
    external_op: CombinationOp,
    page_info: &PageInformation,
    grow: bool,
) -> Jbig2Result<()> {
    let op = if page_info.combination_op_override {
        external_op
    } else {
        page_info.default_combination_op
    };
    let x0 = region_info.x;
    let y0 = region_info.y;
    let rh = region.height();

    if grow {
        let needed = y0 + rh;
        if needed > page.height() {
            grow_bitmap(page, needed, page_info.default_pixel)?;
        }
    }
    // Shared packed-row blit: clipping, alignment, and op dispatch all live
    // inside [`Bitmap::composite`], so this replaces the old pixel-by-pixel
    // composite loop with a single byte-level pass per row.
    page.composite(region, x0 as i32, y0 as i32, op.into());
    Ok(())
}

fn grow_bitmap(bm: &mut Bitmap, new_height: u32, fill: u8) -> Jbig2Result<()> {
    // Extend the bitmap in place (no per-row temporaries, no double-copy
    // through a scratch `Bitmap`). The previous implementation allocated a
    // full replacement page and then copied every existing row byte-by-byte
    // through a per-row `Vec`; `grow_height` reuses the backing `Vec<u8>`.
    bm.grow_height(new_height, fill)
}

fn segment_type_name(t: SegmentType) -> &'static str {
    match t {
        SegmentType::SymbolDictionary => "symbol dictionary (M3)",
        SegmentType::IntermediateTextRegion
        | SegmentType::ImmediateTextRegion
        | SegmentType::ImmediateLosslessTextRegion => "text region (M3)",
        SegmentType::PatternDictionary => "pattern dictionary (M5)",
        SegmentType::IntermediateHalftoneRegion
        | SegmentType::ImmediateHalftoneRegion
        | SegmentType::ImmediateLosslessHalftoneRegion => "halftone region (M5)",
        SegmentType::IntermediateGenericRefinementRegion
        | SegmentType::ImmediateGenericRefinementRegion
        | SegmentType::ImmediateLosslessGenericRefinementRegion => "generic refinement (M4)",
        SegmentType::ColourPalette => "colour palette (M5)",
        SegmentType::Profiles => "profiles segment",
        SegmentType::Extension => "extension segment",
        _ => "unsupported segment",
    }
}

impl Jbig2Decoder<Cursor<Vec<u8>>> {
    /// Convenience constructor for in-memory streams.
    pub fn from_bytes(data: Vec<u8>) -> Jbig2Result<Self> {
        Self::new(Cursor::new(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::mq::MqEncoder;
    use crate::segments::file_header::FileHeader;
    use crate::segments::generic_region::{encode_generic_arith, nominal_at, GenericRegionHeader};
    use crate::segments::region_info::RegionInfo;
    use std::io::Cursor;

    fn encode_minimal_file(bm: &Bitmap) -> Vec<u8> {
        let mut out = Vec::new();
        let fh = FileHeader {
            sequential: true,
            unknown_page_count: false,
            uses_extended_template: false,
            uses_colour: false,
            num_pages: Some(1),
        };
        fh.write(&mut out).unwrap();

        // Page information segment
        let page_info = PageInformation {
            width: bm.width(),
            height: bm.height(),
            x_resolution: 0,
            y_resolution: 0,
            is_lossless: true,
            may_contain_refinements: false,
            default_pixel: 0,
            default_combination_op: CombinationOp::Or,
            requires_aux_buffers: false,
            combination_op_override: false,
            may_contain_colour: false,
            is_striped: false,
            maximum_stripe_size: 0,
        };
        let seg_hdr = SegmentHeader {
            number: 0,
            segment_type: SegmentType::PageInformation,
            deferred_non_retain: false,
            referred: vec![],
            retain_bits: vec![false],
            page_association: 1,
            data_length: Some(PageInformation::WIRE_SIZE),
        };
        seg_hdr.write(&mut out).unwrap();
        page_info.write(&mut out).unwrap();

        // Generic region segment
        let region_info = RegionInfo {
            width: bm.width(),
            height: bm.height(),
            x: 0,
            y: 0,
            external_combination_op: CombinationOp::Or,
            colour_extension: false,
        };
        let hdr = GenericRegionHeader {
            region: region_info,
            mmr: false,
            template: 0,
            tpgdon: false,
            ext_template: false,
            at: nominal_at(0, false),
        };

        // Encode generic body into a buffer to measure length.
        let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
        let mut enc = MqEncoder::new(bm.data().len());
        encode_generic_arith(&mut enc, &mut cxs, &hdr, bm).unwrap();
        let coded = enc.finish();

        let mut hdr_bytes = Vec::new();
        hdr.write(&mut hdr_bytes).unwrap();
        let data_len = hdr_bytes.len() + coded.len();

        let seg_hdr = SegmentHeader {
            number: 1,
            segment_type: SegmentType::ImmediateLosslessGenericRegion,
            deferred_non_retain: false,
            referred: vec![],
            retain_bits: vec![false],
            page_association: 1,
            data_length: Some(data_len as u32),
        };
        seg_hdr.write(&mut out).unwrap();
        out.extend_from_slice(&hdr_bytes);
        out.extend_from_slice(&coded);

        // End-of-page segment
        let seg_hdr = SegmentHeader {
            number: 2,
            segment_type: SegmentType::EndOfPage,
            deferred_non_retain: false,
            referred: vec![],
            retain_bits: vec![false],
            page_association: 1,
            data_length: Some(0),
        };
        seg_hdr.write(&mut out).unwrap();

        out
    }

    #[test]
    fn round_trip_single_page_generic() {
        let mut bm = Bitmap::new(40, 16).unwrap();
        for y in 0..16 {
            for x in 0..40 {
                if (x + y) % 7 == 0 {
                    bm.set_pixel(x, y, 1);
                }
            }
        }
        let buf = encode_minimal_file(&bm);
        let mut dec = Jbig2Decoder::new(Cursor::new(buf)).unwrap();
        assert_eq!(dec.num_pages(), 1);
        let page = dec.decode_page(1).unwrap();
        assert_eq!(page.bitmap, bm);
    }

    /// Encode a two-segment page: an intermediate generic region with a
    /// "reference" bitmap and an immediate-lossless generic refinement
    /// region that reconstructs the target from the reference. The
    /// refinement segment's referred-to list names the generic region.
    #[test]
    fn round_trip_single_page_refinement() {
        use crate::segments::refinement_region::{
            encode_refinement_region, RefinementRegionHeader, NOMINAL_REFINEMENT_AT,
        };

        let mut reference = Bitmap::new(40, 16).unwrap();
        let mut target = Bitmap::new(40, 16).unwrap();
        for y in 0..16 {
            for x in 0..40 {
                let r = ((x * 3 + y) & 1) as u8;
                let t = ((x + y) & 1) as u8;
                reference.set_pixel(x, y, r);
                target.set_pixel(x, y, t);
            }
        }

        let mut out = Vec::new();
        let fh = FileHeader {
            sequential: true,
            unknown_page_count: false,
            uses_extended_template: false,
            uses_colour: false,
            num_pages: Some(1),
        };
        fh.write(&mut out).unwrap();

        let page_info = PageInformation {
            width: target.width(),
            height: target.height(),
            x_resolution: 0,
            y_resolution: 0,
            is_lossless: true,
            may_contain_refinements: true,
            default_pixel: 0,
            default_combination_op: CombinationOp::Replace,
            requires_aux_buffers: false,
            combination_op_override: false,
            may_contain_colour: false,
            is_striped: false,
            maximum_stripe_size: 0,
        };
        let seg = SegmentHeader {
            number: 0,
            segment_type: SegmentType::PageInformation,
            deferred_non_retain: false,
            referred: vec![],
            retain_bits: vec![false],
            page_association: 1,
            data_length: Some(PageInformation::WIRE_SIZE),
        };
        seg.write(&mut out).unwrap();
        page_info.write(&mut out).unwrap();

        // Intermediate generic region — supplies the reference bitmap
        // without compositing onto the page.
        let region_info = RegionInfo {
            width: reference.width(),
            height: reference.height(),
            x: 0,
            y: 0,
            external_combination_op: CombinationOp::Replace,
            colour_extension: false,
        };
        let gr_hdr = GenericRegionHeader {
            region: region_info,
            mmr: false,
            template: 0,
            tpgdon: false,
            ext_template: false,
            at: nominal_at(0, false),
        };
        let mut gr_cxs = MqContexts::new(MQ_NUM_CONTEXTS);
        let mut gr_enc = MqEncoder::new(reference.data().len());
        encode_generic_arith(&mut gr_enc, &mut gr_cxs, &gr_hdr, &reference).unwrap();
        let gr_coded = gr_enc.finish();
        let mut gr_hdr_bytes = Vec::new();
        gr_hdr.write(&mut gr_hdr_bytes).unwrap();
        let gr_data_len = gr_hdr_bytes.len() + gr_coded.len();
        let gr_seg = SegmentHeader {
            number: 1,
            segment_type: SegmentType::IntermediateGenericRegion,
            deferred_non_retain: false,
            referred: vec![],
            retain_bits: vec![false],
            page_association: 1,
            data_length: Some(gr_data_len as u32),
        };
        gr_seg.write(&mut out).unwrap();
        out.extend_from_slice(&gr_hdr_bytes);
        out.extend_from_slice(&gr_coded);

        // Refinement region: reconstructs `target` against the reference
        // region above.
        let rr_hdr = RefinementRegionHeader {
            region: region_info,
            template: 0,
            tpgron: false,
            at: NOMINAL_REFINEMENT_AT,
        };
        let mut rr_cxs = MqContexts::new(MQ_NUM_CONTEXTS);
        let mut rr_enc = MqEncoder::new(target.data().len());
        encode_refinement_region(
            &mut rr_enc,
            &mut rr_cxs,
            &target,
            rr_hdr.template,
            rr_hdr.tpgron,
            &rr_hdr.at.to_array_2(),
            &reference,
            0,
            0,
        )
        .unwrap();
        let rr_coded = rr_enc.finish();
        let mut rr_hdr_bytes = Vec::new();
        rr_hdr.write(&mut rr_hdr_bytes).unwrap();
        let rr_data_len = rr_hdr_bytes.len() + rr_coded.len();
        let rr_seg = SegmentHeader {
            number: 2,
            segment_type: SegmentType::ImmediateLosslessGenericRefinementRegion,
            deferred_non_retain: false,
            referred: vec![1],
            retain_bits: vec![false, false],
            page_association: 1,
            data_length: Some(rr_data_len as u32),
        };
        rr_seg.write(&mut out).unwrap();
        out.extend_from_slice(&rr_hdr_bytes);
        out.extend_from_slice(&rr_coded);

        let eop = SegmentHeader {
            number: 3,
            segment_type: SegmentType::EndOfPage,
            deferred_non_retain: false,
            referred: vec![],
            retain_bits: vec![false],
            page_association: 1,
            data_length: Some(0),
        };
        eop.write(&mut out).unwrap();

        let mut dec = Jbig2Decoder::new(Cursor::new(out)).unwrap();
        assert_eq!(dec.num_pages(), 1);
        let page = dec.decode_page(1).unwrap();
        assert_eq!(page.bitmap, target);
    }
}
