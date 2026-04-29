//! Pattern dictionary segment (spec 7.4.4 / 6.7).

use std::io::{Read, Write};

use crate::bitmap::Bitmap;
use crate::coding::mq::{MqContexts, MqDecoder, MqEncoder, MQ_NUM_CONTEXTS};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::generic_region::{
    decode_generic_arith, decode_generic_mmr, GenericRegionHeader,
};
use crate::segments::page_information::CombinationOp;
use crate::segments::region_info::RegionInfo;
use crate::segments::AtPixels;

/// Parsed pattern dictionary header.
#[derive(Clone, Debug)]
pub struct PatternDictionaryHeader {
    /// `HDMMR` — true = MMR coding, false = arithmetic.
    pub hdmmr: bool,
    /// `HDTEMPLATE` (0..3).
    pub hd_template: u8,
    /// Pattern width (`HDPW`) in pixels.
    pub hdpw: u8,
    /// Pattern height (`HDPH`) in pixels.
    pub hdph: u8,
    /// Greyscale maximum value (`GRAYMAX`).
    pub gray_max: u32,
}

impl PatternDictionaryHeader {
    /// Parse the header from a reader positioned at the first flag byte.
    pub fn read<R: Read>(r: &mut R) -> Jbig2Result<Self> {
        let mut flags = [0u8; 1];
        r.read_exact(&mut flags)?;
        let hdmmr = (flags[0] & 0x01) != 0;
        let hd_template = (flags[0] >> 1) & 0x3;
        let mut sz = [0u8; 2];
        r.read_exact(&mut sz[..1])?;
        let hdpw = sz[0];
        r.read_exact(&mut sz[..1])?;
        let hdph = sz[0];
        let mut gm = [0u8; 4];
        r.read_exact(&mut gm)?;
        let gray_max = u32::from_be_bytes(gm);
        Ok(Self {
            hdmmr,
            hd_template,
            hdpw,
            hdph,
            gray_max,
        })
    }

    /// Emit the header.
    pub fn write<W: Write>(&self, w: &mut W) -> Jbig2Result<()> {
        let flags = (self.hdmmr as u8) | ((self.hd_template & 0x3) << 1);
        w.write_all(&[flags])?;
        w.write_all(&[self.hdpw])?;
        w.write_all(&[self.hdph])?;
        w.write_all(&self.gray_max.to_be_bytes())?;
        Ok(())
    }
}

fn pattern_collective_header(header: &PatternDictionaryHeader) -> GenericRegionHeader {
    let mut at = [(0i8, 0i8); 12];
    at[0] = (-(header.hdpw as i8), 0);
    at[1] = (-3, -1);
    at[2] = (2, -2);
    at[3] = (-2, -2);
    GenericRegionHeader {
        region: RegionInfo {
            width: (header.gray_max + 1) * header.hdpw as u32,
            height: header.hdph as u32,
            x: 0,
            y: 0,
            external_combination_op: CombinationOp::Or,
            colour_extension: false,
        },
        mmr: header.hdmmr,
        template: header.hd_template,
        tpgdon: false,
        ext_template: false,
        at: AtPixels::new(at, 4),
    }
}

fn build_collective_bitmap(
    header: &PatternDictionaryHeader,
    patterns: &[Bitmap],
) -> Jbig2Result<Bitmap> {
    if patterns.is_empty() {
        return Err(Jbig2Error::OutOfRange(
            "pattern dictionary: need at least one pattern",
        ));
    }
    if header.gray_max as usize + 1 != patterns.len() {
        return Err(Jbig2Error::InvalidConfig(
            "pattern dictionary: GRAYMAX does not match pattern count",
        ));
    }
    if header.hdpw == 0 || header.hdph == 0 {
        return Err(Jbig2Error::OutOfRange(
            "pattern dictionary: HDPW/HDPH must be non-zero",
        ));
    }
    if patterns
        .iter()
        .any(|pat| pat.width() != header.hdpw as u32 || pat.height() != header.hdph as u32)
    {
        return Err(Jbig2Error::InvalidConfig(
            "pattern dictionary: pattern sizes do not match header",
        ));
    }
    let mut collective = Bitmap::new(
        (header.gray_max + 1) * header.hdpw as u32,
        header.hdph as u32,
    )?;
    for (idx, pat) in patterns.iter().enumerate() {
        let x_off = idx as i32 * header.hdpw as i32;
        collective.composite(pat, x_off, 0, crate::bitmap::BlitOp::Replace);
    }
    Ok(collective)
}

/// Decode a pattern dictionary body using a caller-owned MQ context pool
/// for the arithmetic-coded collective bitmap variant.
pub fn decode_pattern_dictionary_with_contexts(
    header: &PatternDictionaryHeader,
    body: &[u8],
    cxs: &mut MqContexts,
) -> Jbig2Result<Vec<Bitmap>> {
    if header.hdpw == 0 || header.hdph == 0 {
        return Err(Jbig2Error::OutOfRange(
            "pattern dictionary: HDPW/HDPH must be non-zero",
        ));
    }
    let collective_hdr = pattern_collective_header(header);
    let collective = if header.hdmmr {
        #[cfg(feature = "mmr")]
        {
            decode_generic_mmr(
                body,
                collective_hdr.region.width,
                collective_hdr.region.height,
            )?
        }
        #[cfg(not(feature = "mmr"))]
        {
            return Err(Jbig2Error::Unsupported(
                "pattern dictionary: MMR decoding requires the `mmr` feature",
            ));
        }
    } else {
        cxs.reset();
        let mut dec = MqDecoder::new(body);
        decode_generic_arith(&mut dec, cxs, &collective_hdr)?
    };

    let num_patterns = header.gray_max as usize + 1;
    let pw = header.hdpw as i32;
    let ph = header.hdph as u32;
    let mut pats = Vec::with_capacity(num_patterns);
    for idx in 0..num_patterns {
        let mut pat = Bitmap::new(header.hdpw as u32, ph)?;
        let x_off = idx as i32 * pw;
        pat.copy_from(&collective, x_off, 0, header.hdpw as u32, ph, 0, 0);
        pats.push(pat);
    }
    Ok(pats)
}

/// Decode a pattern dictionary body using a fresh MQ context pool.
pub fn decode_pattern_dictionary(
    header: &PatternDictionaryHeader,
    body: &[u8],
) -> Jbig2Result<Vec<Bitmap>> {
    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    decode_pattern_dictionary_with_contexts(header, body, &mut cxs)
}

/// Encode a pattern dictionary body using a caller-owned MQ context pool for
/// the arithmetic-coded collective bitmap variant.
pub fn encode_pattern_dictionary_with_contexts(
    header: &PatternDictionaryHeader,
    patterns: &[Bitmap],
    cxs: &mut MqContexts,
) -> Jbig2Result<Vec<u8>> {
    let collective_hdr = pattern_collective_header(header);
    let collective = build_collective_bitmap(header, patterns)?;
    if header.hdmmr {
        #[cfg(feature = "mmr")]
        {
            crate::segments::generic_region::encode_generic_mmr(&collective)
        }
        #[cfg(not(feature = "mmr"))]
        {
            Err(Jbig2Error::Unsupported(
                "pattern dictionary: MMR encoding requires the `mmr` feature",
            ))
        }
    } else {
        cxs.reset();
        let mut enc = MqEncoder::new(collective.data().len() + 16);
        crate::segments::generic_region::encode_generic_arith(
            &mut enc,
            cxs,
            &collective_hdr,
            &collective,
        )?;
        Ok(enc.finish())
    }
}

/// Encode a pattern dictionary body using a fresh MQ context pool.
pub fn encode_pattern_dictionary(
    header: &PatternDictionaryHeader,
    patterns: &[Bitmap],
) -> Jbig2Result<Vec<u8>> {
    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    encode_pattern_dictionary_with_contexts(header, patterns, &mut cxs)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn header_round_trip() {
        let hdr = PatternDictionaryHeader {
            hdmmr: false,
            hd_template: 0,
            hdpw: 8,
            hdph: 8,
            gray_max: 15,
        };
        let mut buf = Vec::new();
        hdr.write(&mut buf).unwrap();
        let back = PatternDictionaryHeader::read(&mut buf.as_slice()).unwrap();
        assert_eq!(back.hdpw, 8);
        assert_eq!(back.hdph, 8);
        assert_eq!(back.gray_max, 15);
    }

    #[test]
    fn arithmetic_round_trip() {
        let pats = sample_patterns();
        let hdr = PatternDictionaryHeader {
            hdmmr: false,
            hd_template: 0,
            hdpw: 3,
            hdph: 3,
            gray_max: 1,
        };
        let body = encode_pattern_dictionary(&hdr, &pats).unwrap();
        let back = decode_pattern_dictionary(&hdr, &body).unwrap();
        assert_eq!(back, pats);
    }

    #[cfg(feature = "mmr")]
    #[test]
    fn mmr_round_trip() {
        let pats = sample_patterns();
        let hdr = PatternDictionaryHeader {
            hdmmr: true,
            hd_template: 0,
            hdpw: 3,
            hdph: 3,
            gray_max: 1,
        };
        let body = encode_pattern_dictionary(&hdr, &pats).unwrap();
        let back = decode_pattern_dictionary(&hdr, &body).unwrap();
        assert_eq!(back, pats);
    }
}
