//! Pattern dictionary segment (spec 7.4.4).
//!
//! Header parsing only for the v1 crate; the pattern/halftone coding path
//! (spec 6.7) is a substantial standalone feature and is tracked as a
//! follow-up milestone. Every JBIG2 reader needs to at least walk past
//! these segments without failing the enumeration, which the header
//! definition below enables.

use std::io::{Read, Write};

use crate::error::{Jbig2Error, Jbig2Result};

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

/// Stub decoder — refuses pattern dictionary decoding until spec 6.7 has
/// been fully wired.
pub fn decode_pattern_dictionary_unimplemented() -> Jbig2Result<()> {
    Err(Jbig2Error::Unsupported(
        "pattern dictionary decode (spec 6.7) — planned for a later milestone",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
