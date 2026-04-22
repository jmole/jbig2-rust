//! High-level JBIG2 decoder.
//!
//! The decoder supports the sequential organisation defined in spec D.1 and
//! currently handles page-information, end-of-page, and generic-region
//! segments. Additional segment families (symbol dictionary, text region,
//! refinement, halftone) are added in milestones M3..M5.
//!
//! The page is composited according to the page-info default pixel value +
//! combination operator and the per-region combination operator.

use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::bitmap::Bitmap;
use crate::coding::mq::{MqContexts, MqDecoder, MQ_NUM_CONTEXTS};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::{
    generic_region::{self, GenericRegionHeader},
    symbol_dictionary::{self, SymbolDictionaryHeader},
    text_region::{self, TextRegionHeader},
    CombinationOp, FileHeader, PageInformation, RegionInfo, SegmentHeader, SegmentType,
};

/// A decoded page from a JBIG2 file.
#[derive(Clone, Debug)]
pub struct DecodedPage {
    /// Page information header (dimensions, flags, striping).
    pub info: PageInformation,
    /// Fully-composited page bitmap.
    pub bitmap: Bitmap,
}

/// Multi-page JBIG2 decoder.
pub struct Jbig2Decoder<R: Read + Seek> {
    reader: R,
    header: FileHeader,
    /// Segment headers in stream order. Data-length `None` is only allowed
    /// for the terminal segment of an embedded / streaming file.
    segments: Vec<SegmentDescriptor>,
}

#[derive(Clone, Debug)]
struct SegmentDescriptor {
    header: SegmentHeader,
    /// Offset of the segment *data* in the stream.
    data_offset: u64,
    /// Data length. We require a known length for the v1 decoder.
    data_len: u32,
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

        // Symbol dictionaries accumulate across segments; each key is the
        // global segment number of the dictionary segment, value is its
        // exported symbols. Dictionaries may reference other dictionaries
        // via the segment header's referred-to list.
        let mut sym_dicts: HashMap<u32, Vec<Bitmap>> = HashMap::new();

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
                _ if !on_this_page => continue,
                SegmentType::PageInformation | SegmentType::EndOfPage => continue,
                SegmentType::ImmediateGenericRegion
                | SegmentType::ImmediateLosslessGenericRegion
                | SegmentType::IntermediateGenericRegion => {
                    let (hdr, region_bitmap) = self.decode_generic_region(idx)?;
                    composite_region(
                        &mut page,
                        &region_bitmap,
                        &hdr.region,
                        hdr.region.external_combination_op,
                        &info,
                        height_is_unknown,
                    )?;
                }
                SegmentType::ImmediateTextRegion
                | SegmentType::ImmediateLosslessTextRegion
                | SegmentType::IntermediateTextRegion => {
                    let (hdr, region_bitmap) = self.decode_text_region(idx, &sym_dicts)?;
                    composite_region(
                        &mut page,
                        &region_bitmap,
                        &hdr.region,
                        hdr.region.external_combination_op,
                        &info,
                        height_is_unknown,
                    )?;
                }
                SegmentType::EndOfFile | SegmentType::EndOfStripe | SegmentType::Tables => continue,
                other => {
                    return Err(Jbig2Error::Unsupported(segment_type_name(other)));
                }
            }
        }
        Ok(DecodedPage { info, bitmap: page })
    }

    fn read_page_information(&mut self, idx: usize) -> Jbig2Result<PageInformation> {
        let seg = &self.segments[idx];
        self.reader
            .seek(SeekFrom::Start(seg.data_offset))
            .map_err(Jbig2Error::from)?;
        PageInformation::read(&mut self.reader)
    }

    fn read_segment_body(&mut self, idx: usize) -> Jbig2Result<Vec<u8>> {
        let seg = &self.segments[idx];
        let data_len = seg.data_len as usize;
        self.reader
            .seek(SeekFrom::Start(seg.data_offset))
            .map_err(Jbig2Error::from)?;
        let mut body = vec![0u8; data_len];
        self.reader.read_exact(&mut body).map_err(Jbig2Error::from)?;
        Ok(body)
    }

    fn decode_symbol_dictionary(
        &mut self,
        idx: usize,
        sym_dicts: &HashMap<u32, Vec<Bitmap>>,
    ) -> Jbig2Result<(u32, Vec<Bitmap>)> {
        let referred = self.segments[idx].header.referred.clone();
        let seg_no = self.segments[idx].header.number;
        let body = self.read_segment_body(idx)?;
        let mut cur = Cursor::new(&body);
        let header = SymbolDictionaryHeader::read(&mut cur)?;
        let header_len = cur.position() as usize;
        let coded = &body[header_len..];

        // Import symbols from referred symbol dictionaries in referred-to
        // order.
        let mut import: Vec<Bitmap> = Vec::new();
        for ref_no in &referred {
            if let Some(syms) = sym_dicts.get(ref_no) {
                import.extend(syms.iter().cloned());
            }
        }
        let decoded = symbol_dictionary::decode_symbol_dictionary(&header, coded, &import)?;
        Ok((seg_no, decoded.exported))
    }

    fn decode_text_region(
        &mut self,
        idx: usize,
        sym_dicts: &HashMap<u32, Vec<Bitmap>>,
    ) -> Jbig2Result<(TextRegionHeader, Bitmap)> {
        let referred = self.segments[idx].header.referred.clone();
        let body = self.read_segment_body(idx)?;
        let mut cur = Cursor::new(&body);
        let header = TextRegionHeader::read(&mut cur)?;
        let header_len = cur.position() as usize;
        let coded = &body[header_len..];

        // SBSYMS is the concatenation of every referred-to symbol
        // dictionary's exported list, in referred-to order.
        let mut sbsyms: Vec<Bitmap> = Vec::new();
        for ref_no in &referred {
            if let Some(syms) = sym_dicts.get(ref_no) {
                sbsyms.extend(syms.iter().cloned());
            }
        }
        if sbsyms.is_empty() {
            return Err(Jbig2Error::OutOfRange(
                "text region: no referred symbol dictionary",
            ));
        }
        let region = text_region::decode_text_region(&header, coded, &sbsyms)?;
        Ok((header, region))
    }

    fn decode_generic_region(
        &mut self,
        idx: usize,
    ) -> Jbig2Result<(GenericRegionHeader, Bitmap)> {
        let seg = &self.segments[idx];
        let data_len = seg.data_len as usize;
        self.reader
            .seek(SeekFrom::Start(seg.data_offset))
            .map_err(Jbig2Error::from)?;
        let mut body = vec![0u8; data_len];
        self.reader.read_exact(&mut body).map_err(Jbig2Error::from)?;
        let mut cur = Cursor::new(&body);
        let header = GenericRegionHeader::read(&mut cur)?;
        let header_len = cur.position() as usize;
        let coded = &body[header_len..];
        let bitmap = if header.mmr {
            #[cfg(feature = "mmr")]
            {
                generic_region::decode_generic_mmr(coded, header.region.width, header.region.height)?
            }
            #[cfg(not(feature = "mmr"))]
            {
                return Err(Jbig2Error::Unsupported("mmr feature disabled"));
            }
        } else {
            let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
            let mut dec = MqDecoder::new(coded);
            generic_region::decode_generic_arith(&mut dec, &mut cxs, &header)?
        };
        Ok((header, bitmap))
    }
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
    let rw = region.width();
    let rh = region.height();

    if grow {
        let needed = y0 + rh;
        if needed > page.height() {
            grow_bitmap(page, needed, page_info.default_pixel)?;
        }
    }
    let page_w = page.width();
    let page_h = page.height();

    for yy in 0..rh {
        let py = y0 + yy;
        if py >= page_h {
            break;
        }
        for xx in 0..rw {
            let px = x0 + xx;
            if px >= page_w {
                break;
            }
            let src = region.get_pixel(xx as i32, yy as i32);
            let dst = page.get_pixel(px as i32, py as i32);
            let out = match op {
                CombinationOp::Or => src | dst,
                CombinationOp::And => src & dst,
                CombinationOp::Xor => src ^ dst,
                CombinationOp::XNor => 1 ^ (src ^ dst),
                CombinationOp::Replace => src,
            };
            page.set_pixel(px as i32, py as i32, out);
        }
    }
    Ok(())
}

fn grow_bitmap(bm: &mut Bitmap, new_height: u32, fill: u8) -> Jbig2Result<()> {
    let old = std::mem::replace(bm, Bitmap::filled(bm.width(), new_height, fill)?);
    // Copy old contents into the top portion of the new bitmap.
    for y in 0..old.height() {
        let src = old.row(y as usize).to_vec();
        bm.row_mut(y as usize).copy_from_slice(&src);
    }
    Ok(())
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
}
