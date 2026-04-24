//! Halftone region segment (spec 7.4.5 / 6.6).

use std::io::{Read, Write};

use crate::bitmap::Bitmap;
use crate::coding::mmr_lut::MmrBitBuf;
use crate::coding::mq::{MqContexts, MqDecoder, MqEncoder, MQ_NUM_CONTEXTS};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::generic_region::{
    decode_generic_bitmap, decode_generic_mmr_streaming, encode_generic_arith, encode_generic_mmr,
    GenericRegionHeader,
};
use crate::segments::page_information::CombinationOp;
use crate::segments::region_info::RegionInfo;
use crate::segments::AtPixels;

/// Parsed halftone-region segment header.
#[derive(Clone, Debug)]
pub struct HalftoneRegionHeader {
    /// Region info (size, page position, external combination op).
    pub region: RegionInfo,
    /// `HMMR`.
    pub hmmr: bool,
    /// `HTEMPLATE`.
    pub htemplate: u8,
    /// `HENABLESKIP`.
    pub enable_skip: bool,
    /// `HCOMBOP`.
    pub hcombop: CombinationOp,
    /// `HDEFPIXEL`.
    pub hdef_pixel: u8,
    /// `HGW`.
    pub hgw: u32,
    /// `HGH`.
    pub hgh: u32,
    /// `HGX` (signed, scaled by 256).
    pub hgx: i32,
    /// `HGY` (signed, scaled by 256).
    pub hgy: i32,
    /// `HRX` (scaled by 256).
    pub hrx: u16,
    /// `HRY` (scaled by 256).
    pub hry: u16,
}

impl HalftoneRegionHeader {
    /// Parse the segment data header from the start of a halftone-region body.
    pub fn read<R: Read>(r: &mut R) -> Jbig2Result<Self> {
        let region = RegionInfo::read(r)?;
        let mut flags = [0u8; 1];
        r.read_exact(&mut flags)?;
        let flags = flags[0];
        let hmmr = (flags & 0x01) != 0;
        let htemplate = (flags >> 1) & 0x03;
        let enable_skip = ((flags >> 3) & 0x01) != 0;
        let hcombop = CombinationOp::from_u8((flags >> 4) & 0x07);
        let hdef_pixel = (flags >> 7) & 0x01;

        let mut b4 = [0u8; 4];
        r.read_exact(&mut b4)?;
        let hgw = u32::from_be_bytes(b4);
        r.read_exact(&mut b4)?;
        let hgh = u32::from_be_bytes(b4);
        r.read_exact(&mut b4)?;
        let hgx = i32::from_be_bytes(b4);
        r.read_exact(&mut b4)?;
        let hgy = i32::from_be_bytes(b4);

        let mut b2 = [0u8; 2];
        r.read_exact(&mut b2)?;
        let hrx = u16::from_be_bytes(b2);
        r.read_exact(&mut b2)?;
        let hry = u16::from_be_bytes(b2);

        Ok(Self {
            region,
            hmmr,
            htemplate,
            enable_skip,
            hcombop,
            hdef_pixel,
            hgw,
            hgh,
            hgx,
            hgy,
            hrx,
            hry,
        })
    }

    /// Emit the segment data header.
    pub fn write<W: Write>(&self, w: &mut W) -> Jbig2Result<()> {
        self.region.write(w)?;
        let flags = (self.hmmr as u8)
            | ((self.htemplate & 0x03) << 1)
            | ((self.enable_skip as u8) << 3)
            | (self.hcombop.to_u8() << 4)
            | ((self.hdef_pixel & 0x01) << 7);
        w.write_all(&[flags])?;
        w.write_all(&self.hgw.to_be_bytes())?;
        w.write_all(&self.hgh.to_be_bytes())?;
        w.write_all(&self.hgx.to_be_bytes())?;
        w.write_all(&self.hgy.to_be_bytes())?;
        w.write_all(&self.hrx.to_be_bytes())?;
        w.write_all(&self.hry.to_be_bytes())?;
        Ok(())
    }
}

fn halftone_at(template: u8) -> AtPixels {
    let mut at = [(0i8, 0i8); 12];
    at[0] = (if template <= 1 { 3 } else { 2 }, -1);
    at[1] = (-3, -1);
    at[2] = (2, -2);
    at[3] = (-2, -2);
    AtPixels::new(at, 4)
}

fn halftone_bits_per_pattern(num_patterns: usize) -> u32 {
    if num_patterns <= 1 {
        0
    } else {
        usize::BITS - (num_patterns - 1).leading_zeros()
    }
}

fn halftone_plane_header(header: &HalftoneRegionHeader) -> GenericRegionHeader {
    GenericRegionHeader {
        region: RegionInfo {
            width: header.hgw,
            height: header.hgh,
            x: 0,
            y: 0,
            external_combination_op: CombinationOp::Or,
            colour_extension: false,
        },
        mmr: header.hmmr,
        template: header.htemplate,
        tpgdon: false,
        ext_template: false,
        at: halftone_at(header.htemplate),
    }
}

#[cfg(feature = "mmr")]
fn consume_t6_rtc_and_align(buf: &mut MmrBitBuf<'_>) -> Jbig2Result<()> {
    if buf.peek_bits(12) == 1 {
        buf.skip_bits(12)?;
        if buf.peek_bits(12) == 1 {
            buf.skip_bits(12)?;
        }
    }
    let bp = buf.bit_pos();
    let aligned = (bp + 7) & !7;
    let pad = aligned - bp;
    if pad > 0 {
        buf.consume_padding(pad as u8)?;
    }
    Ok(())
}

/// Decode a halftone region body using a caller-owned MQ context pool for
/// arithmetic-coded bitplanes.
pub fn decode_halftone_region_with_contexts(
    header: &HalftoneRegionHeader,
    body: &[u8],
    patterns: &[&Bitmap],
    cxs: &mut MqContexts,
) -> Jbig2Result<Bitmap> {
    if patterns.is_empty() {
        return Err(Jbig2Error::OutOfRange(
            "halftone region: no referred pattern dictionary",
        ));
    }
    let pattern_w = patterns[0].width() as i32;
    let pattern_h = patterns[0].height() as i32;
    if patterns
        .iter()
        .any(|pat| pat.width() as i32 != pattern_w || pat.height() as i32 != pattern_h)
    {
        return Err(Jbig2Error::InvalidConfig(
            "halftone region: mixed pattern sizes in referred dictionary",
        ));
    }

    let hbpp = halftone_bits_per_pattern(patterns.len());
    let mut planes = Vec::with_capacity(hbpp as usize);
    if header.hmmr {
        #[cfg(not(feature = "mmr"))]
        {
            return Err(Jbig2Error::Unsupported(
                "halftone region: MMR decoding requires the `mmr` feature",
            ));
        }
        #[cfg(feature = "mmr")]
        {
            let mut buf = MmrBitBuf::new(body);
            for _ in 0..hbpp {
                let plane = decode_generic_mmr_streaming(&mut buf, header.hgw, header.hgh)?;
                consume_t6_rtc_and_align(&mut buf)?;
                planes.push(plane);
            }
        }
    } else {
        cxs.reset();
        let mut dec = MqDecoder::new(body);
        let at = halftone_at(header.htemplate);
        for _ in 0..hbpp {
            let plane = decode_generic_bitmap(
                &mut dec,
                cxs,
                header.hgw,
                header.hgh,
                header.htemplate,
                false,
                false,
                &at,
            )?;
            planes.push(plane);
        }
    }

    let mut region = Bitmap::filled(header.region.width, header.region.height, header.hdef_pixel)?;
    for mg in 0..header.hgh {
        for ng in 0..header.hgw {
            let x = ((header.hgx as i64)
                + mg as i64 * header.hry as i64
                + ng as i64 * header.hrx as i64)
                >> 8;
            let y = ((header.hgy as i64)
                + mg as i64 * header.hrx as i64
                - ng as i64 * header.hry as i64)
                >> 8;
            if header.enable_skip
                && (x + pattern_w as i64 <= 0
                    || x >= header.region.width as i64
                    || y + pattern_h as i64 <= 0
                    || y >= header.region.height as i64)
            {
                continue;
            }

            let mut gray = if hbpp == 0 {
                0u32
            } else {
                planes[0].get_pixel(ng as i32, mg as i32) as u32
            };
            for plane in planes.iter().skip(1) {
                let bit = plane.get_pixel(ng as i32, mg as i32) as u32;
                gray = (gray << 1) | ((gray & 1) ^ bit);
            }
            let pat = patterns.get(gray as usize).ok_or(Jbig2Error::OutOfRange(
                "halftone region: gray value exceeds pattern dictionary",
            ))?;
            region.composite(pat, x as i32, y as i32, header.hcombop.into());
        }
    }
    Ok(region)
}

/// Decode a halftone region body using a fresh MQ context pool.
pub fn decode_halftone_region(
    header: &HalftoneRegionHeader,
    body: &[u8],
    patterns: &[&Bitmap],
) -> Jbig2Result<Bitmap> {
    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    decode_halftone_region_with_contexts(header, body, patterns, &mut cxs)
}

/// Encode a halftone region body from pattern indices using a caller-owned MQ
/// context pool for the arithmetic-coded bitplane variant.
pub fn encode_halftone_region_with_contexts(
    header: &HalftoneRegionHeader,
    gray_values: &[u32],
    num_patterns: usize,
    cxs: &mut MqContexts,
) -> Jbig2Result<Vec<u8>> {
    if header.hgw == 0 || header.hgh == 0 {
        return Err(Jbig2Error::OutOfRange(
            "halftone region: HGW/HGH must be non-zero",
        ));
    }
    if gray_values.len() != (header.hgw as usize) * (header.hgh as usize) {
        return Err(Jbig2Error::InvalidConfig(
            "halftone region: gray-value count does not match HGW*HGH",
        ));
    }
    let hbpp = halftone_bits_per_pattern(num_patterns);
    if hbpp == 0 && gray_values.iter().any(|&v| v != 0) {
        return Err(Jbig2Error::OutOfRange(
            "halftone region: gray value exceeds single-pattern dictionary",
        ));
    }
    if hbpp > 0 && gray_values.iter().any(|&v| v >= num_patterns as u32) {
        return Err(Jbig2Error::OutOfRange(
            "halftone region: gray value exceeds pattern dictionary",
        ));
    }

    let mut planes = Vec::with_capacity(hbpp as usize);
    for _ in 0..hbpp {
        planes.push(Bitmap::new(header.hgw, header.hgh)?);
    }
    for mg in 0..header.hgh as usize {
        for ng in 0..header.hgw as usize {
            let idx = mg * header.hgw as usize + ng;
            let g = gray_values[idx];
            let gray = g ^ (g >> 1);
            for plane_idx in 0..hbpp as usize {
                let shift = hbpp as usize - 1 - plane_idx;
                let bit = ((gray >> shift) & 1) as u8;
                planes[plane_idx].set_pixel(ng as i32, mg as i32, bit);
            }
        }
    }

    if header.hmmr {
        #[cfg(feature = "mmr")]
        {
            let mut body = Vec::new();
            for plane in &planes {
                body.extend_from_slice(&encode_generic_mmr(plane)?);
            }
            Ok(body)
        }
        #[cfg(not(feature = "mmr"))]
        {
            Err(Jbig2Error::Unsupported(
                "halftone region: MMR encoding requires the `mmr` feature",
            ))
        }
    } else {
        let plane_hdr = halftone_plane_header(header);
        cxs.reset();
        let cap = (header.hgw as usize * header.hgh as usize * hbpp as usize).max(16);
        let mut enc = MqEncoder::new(cap);
        for plane in &planes {
            encode_generic_arith(&mut enc, cxs, &plane_hdr, plane)?;
        }
        Ok(enc.finish())
    }
}

/// Encode a halftone region body using a fresh MQ context pool.
pub fn encode_halftone_region(
    header: &HalftoneRegionHeader,
    gray_values: &[u32],
    num_patterns: usize,
) -> Jbig2Result<Vec<u8>> {
    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    encode_halftone_region_with_contexts(header, gray_values, num_patterns, &mut cxs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitmap::BlitOp;

    fn sample_patterns() -> Vec<Bitmap> {
        let mut p0 = Bitmap::new(3, 3).unwrap();
        p0.set_pixel(1, 0, 1);
        p0.set_pixel(0, 1, 1);
        p0.set_pixel(1, 1, 1);
        p0.set_pixel(2, 1, 1);
        p0.set_pixel(1, 2, 1);

        let mut p1 = Bitmap::new(3, 3).unwrap();
        for i in 0..3 {
            p1.set_pixel(i, i, 1);
        }

        vec![p0, p1]
    }

    fn sample_header(hmmr: bool) -> HalftoneRegionHeader {
        HalftoneRegionHeader {
            region: RegionInfo {
                width: 10,
                height: 6,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            hmmr,
            htemplate: 0,
            enable_skip: false,
            hcombop: CombinationOp::Or,
            hdef_pixel: 0,
            hgw: 2,
            hgh: 1,
            hgx: 0,
            hgy: 0,
            hrx: 4 << 8,
            hry: 0,
        }
    }

    #[test]
    fn header_round_trip() {
        let hdr = sample_header(false);
        let mut buf = Vec::new();
        hdr.write(&mut buf).unwrap();
        let back = HalftoneRegionHeader::read(&mut buf.as_slice()).unwrap();
        assert_eq!(back.hgw, hdr.hgw);
        assert_eq!(back.hgh, hdr.hgh);
        assert_eq!(back.hrx, hdr.hrx);
        assert_eq!(back.hry, hdr.hry);
    }

    #[test]
    fn arithmetic_round_trip() {
        let hdr = sample_header(false);
        let gray = [0u32, 1u32];
        let pats = sample_patterns();
        let refs: Vec<&Bitmap> = pats.iter().collect();
        let body = encode_halftone_region(&hdr, &gray, pats.len()).unwrap();
        let out = decode_halftone_region(&hdr, &body, &refs).unwrap();

        let mut expect = Bitmap::new(10, 6).unwrap();
        expect.composite(&pats[0], 0, 0, BlitOp::Or);
        expect.composite(&pats[1], 4, 0, BlitOp::Or);
        assert_eq!(out, expect);
    }

    #[cfg(feature = "mmr")]
    #[test]
    fn mmr_round_trip() {
        let hdr = sample_header(true);
        let gray = [1u32, 0u32];
        let pats = sample_patterns();
        let refs: Vec<&Bitmap> = pats.iter().collect();
        let body = encode_halftone_region(&hdr, &gray, pats.len()).unwrap();
        let out = decode_halftone_region(&hdr, &body, &refs).unwrap();

        let mut expect = Bitmap::new(10, 6).unwrap();
        expect.composite(&pats[1], 0, 0, BlitOp::Or);
        expect.composite(&pats[0], 4, 0, BlitOp::Or);
        assert_eq!(out, expect);
    }
}
