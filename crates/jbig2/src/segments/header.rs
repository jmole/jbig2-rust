//! JBIG2 segment header (clause 7.2).
//!
//! Each JBIG2 segment starts with a header containing:
//!
//! * 4-byte segment number,
//! * 1-byte flags (retain, page-assoc-size, type),
//! * N-byte referred-segment count field (1, 5, 5, 6, or 7 bytes),
//! * `numReferSeg` referred-segment numbers (each 1, 2, or 4 bytes wide
//!   depending on the containing segment number),
//! * 1- or 4-byte page association,
//! * 4-byte data length (may be `0xFFFF_FFFF` for "unknown length").
//!
//! The encoder emits canonical headers: short retain fields and 4-byte
//! lengths unless specifically asked to emit unknown-length. The decoder
//! accepts every valid shape.

use std::io::{Read, Write};

use crate::error::{Jbig2Error, Jbig2Result};

/// Identifies one of the 22 defined JBIG2 segment types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum SegmentType {
    SymbolDictionary = 0x00,
    IntermediateTextRegion = 0x04,
    ImmediateTextRegion = 0x06,
    ImmediateLosslessTextRegion = 0x07,
    PatternDictionary = 0x10,
    IntermediateHalftoneRegion = 0x14,
    ImmediateHalftoneRegion = 0x16,
    ImmediateLosslessHalftoneRegion = 0x17,
    IntermediateGenericRegion = 0x24,
    ImmediateGenericRegion = 0x26,
    ImmediateLosslessGenericRegion = 0x27,
    IntermediateGenericRefinementRegion = 0x28,
    ImmediateGenericRefinementRegion = 0x2A,
    ImmediateLosslessGenericRefinementRegion = 0x2B,
    PageInformation = 0x30,
    EndOfPage = 0x31,
    EndOfStripe = 0x32,
    EndOfFile = 0x33,
    Profiles = 0x34,
    Tables = 0x35,
    ColourPalette = 0x36,
    Extension = 0x3E,
}

impl SegmentType {
    /// Convert from the low 6 bits of the flags byte.
    pub fn from_u8(raw: u8) -> Jbig2Result<Self> {
        use SegmentType::*;
        Ok(match raw & 0x3F {
            0x00 => SymbolDictionary,
            0x04 => IntermediateTextRegion,
            0x06 => ImmediateTextRegion,
            0x07 => ImmediateLosslessTextRegion,
            0x10 => PatternDictionary,
            0x14 => IntermediateHalftoneRegion,
            0x16 => ImmediateHalftoneRegion,
            0x17 => ImmediateLosslessHalftoneRegion,
            0x24 => IntermediateGenericRegion,
            0x26 => ImmediateGenericRegion,
            0x27 => ImmediateLosslessGenericRegion,
            0x28 => IntermediateGenericRefinementRegion,
            0x2A => ImmediateGenericRefinementRegion,
            0x2B => ImmediateLosslessGenericRefinementRegion,
            0x30 => PageInformation,
            0x31 => EndOfPage,
            0x32 => EndOfStripe,
            0x33 => EndOfFile,
            0x34 => Profiles,
            0x35 => Tables,
            0x36 => ColourPalette,
            0x3E => Extension,
            _ => return Err(Jbig2Error::InvalidSegmentHeader("unknown segment type")),
        })
    }

    /// Emit as the low 6 bits of the flags byte.
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Decoded segment header (without the attached data body).
#[derive(Clone, Debug)]
pub struct SegmentHeader {
    /// Segment number.
    pub number: u32,
    /// Segment type.
    pub segment_type: SegmentType,
    /// `true` when "deferred non-retain" is set.
    pub deferred_non_retain: bool,
    /// Segment numbers this one refers to.
    pub referred: Vec<u32>,
    /// One retain flag per referred segment, plus one for this segment itself
    /// (LSB first in the spec's bit-packed layout).
    pub retain_bits: Vec<bool>,
    /// Page association (0 = global / not associated).
    pub page_association: u32,
    /// Data-length field. `None` means unknown length (`0xFFFF_FFFF`).
    pub data_length: Option<u32>,
}

impl SegmentHeader {
    /// Parse a header. `r` must be positioned at the first byte of the header.
    pub fn read<R: Read>(r: &mut R) -> Jbig2Result<Self> {
        let mut buf4 = [0u8; 4];
        read_exact(r, &mut buf4)?;
        let number = u32::from_be_bytes(buf4);

        let mut flags = [0u8; 1];
        read_exact(r, &mut flags)?;
        let flags = flags[0];
        let deferred_non_retain = (flags & 0x80) != 0;
        let page_assoc_size = (flags & 0x40) != 0; // true ⇒ 4-byte page assoc
        let segment_type = SegmentType::from_u8(flags)?;

        // Referred-to segment count: top 3 bits of next byte.
        let mut b = [0u8; 1];
        read_exact(r, &mut b)?;
        let count_hi = (b[0] >> 5) & 0x07;
        let (num_ref, retain_bytes_consumed) = if count_hi <= 4 {
            (count_hi as u32, 0usize)
        } else if count_hi == 7 {
            // Long form: 4 extra bytes of count, and ceil((N+1)/8) retain bytes.
            let mut rest = [0u8; 3];
            read_exact(r, &mut rest)?;
            let num = ((b[0] as u32) & 0x1F) << 24
                | (rest[0] as u32) << 16
                | (rest[1] as u32) << 8
                | (rest[2] as u32);
            let n_retain_bytes = ((num + 1) as usize).div_ceil(8);
            let mut retain = vec![0u8; n_retain_bytes];
            read_exact(r, &mut retain)?;
            // We have not yet parsed retain_bits as booleans; do that below
            // after we know N. Store for now by embedding into the closure.
            // To avoid duplicating parsing, re-enter a branch below; track via
            // a temporary vector.
            return continue_long_form(
                r,
                number,
                segment_type,
                deferred_non_retain,
                num,
                retain,
                page_assoc_size,
            );
        } else {
            return Err(Jbig2Error::InvalidSegmentHeader(
                "invalid referred-to segment count field",
            ));
        };
        let _ = retain_bytes_consumed; // short form has only the low-5 retain bits in `b`.

        // Short form: one byte encodes NumRef (top 3 bits) + low 5 retain bits.
        // Retain bits are LSB-first within the byte for the first 5 of N+1.
        let mut retain_bits_packed = b[0] & 0x1F;

        // Referred segment numbers: 1, 2, or 4 bytes each, determined by the
        // current segment number.
        let ref_width = if number <= 0xFF {
            1usize
        } else if number <= 0xFFFF {
            2
        } else {
            4
        };
        let mut referred = Vec::with_capacity(num_ref as usize);
        for _ in 0..num_ref {
            let mut rb = [0u8; 4];
            read_exact(r, &mut rb[..ref_width])?;
            let mut v: u32 = 0;
            for k in 0..ref_width {
                v = (v << 8) | rb[k] as u32;
            }
            referred.push(v);
        }

        let page_association = if page_assoc_size {
            read_exact(r, &mut buf4)?;
            u32::from_be_bytes(buf4)
        } else {
            let mut pb = [0u8; 1];
            read_exact(r, &mut pb)?;
            pb[0] as u32
        };

        read_exact(r, &mut buf4)?;
        let raw_len = u32::from_be_bytes(buf4);
        let data_length = if raw_len == 0xFFFF_FFFF {
            None
        } else {
            Some(raw_len)
        };

        let mut retain_bits = Vec::with_capacity((num_ref + 1) as usize);
        for i in 0..=num_ref {
            let bit = if (i as usize) < 5 {
                (retain_bits_packed >> i) & 1
            } else {
                0 // short form only ever carries 5 retain bits; extras default false
            };
            retain_bits.push(bit != 0);
            if i < 5 {
                retain_bits_packed &= !(1u8 << i);
            }
        }

        Ok(Self {
            number,
            segment_type,
            deferred_non_retain,
            referred,
            retain_bits,
            page_association,
            data_length,
        })
    }

    /// Emit the header. Chooses short-form encoding when possible.
    pub fn write<W: Write>(&self, w: &mut W) -> Jbig2Result<()> {
        w.write_all(&self.number.to_be_bytes())?;
        let flags = self.segment_type.to_u8()
            | if self.page_association > 0xFF {
                0x40
            } else {
                0
            }
            | if self.deferred_non_retain { 0x80 } else { 0 };
        w.write_all(&[flags])?;

        let num_ref = self.referred.len() as u32;
        if num_ref <= 4 {
            // Short form
            let mut b = (num_ref as u8) << 5;
            for (i, r) in self.retain_bits.iter().take(5).enumerate() {
                if *r {
                    b |= 1 << i;
                }
            }
            w.write_all(&[b])?;
        } else {
            // Long form
            let mut b = 0x07u8 << 5;
            b |= ((num_ref >> 24) & 0x1F) as u8;
            w.write_all(&[b])?;
            w.write_all(&[((num_ref >> 16) & 0xFF) as u8])?;
            w.write_all(&[((num_ref >> 8) & 0xFF) as u8])?;
            w.write_all(&[(num_ref & 0xFF) as u8])?;
            // (N+1) retain bits packed LSB-first into ceil((N+1)/8) bytes.
            let n_bits = (num_ref + 1) as usize;
            let n_bytes = n_bits.div_ceil(8);
            let mut packed = vec![0u8; n_bytes];
            for (i, r) in self.retain_bits.iter().take(n_bits).enumerate() {
                if *r {
                    packed[i / 8] |= 1 << (i & 7);
                }
            }
            w.write_all(&packed)?;
        }

        let ref_width = if self.number <= 0xFF {
            1
        } else if self.number <= 0xFFFF {
            2
        } else {
            4
        };
        for &r in &self.referred {
            let bytes = r.to_be_bytes();
            w.write_all(&bytes[4 - ref_width..])?;
        }

        if self.page_association > 0xFF {
            w.write_all(&self.page_association.to_be_bytes())?;
        } else {
            w.write_all(&[self.page_association as u8])?;
        }

        let len = self.data_length.unwrap_or(0xFFFF_FFFF);
        w.write_all(&len.to_be_bytes())?;
        Ok(())
    }
}

fn continue_long_form<R: Read>(
    r: &mut R,
    number: u32,
    segment_type: SegmentType,
    deferred_non_retain: bool,
    num_ref: u32,
    retain: Vec<u8>,
    page_assoc_size: bool,
) -> Jbig2Result<SegmentHeader> {
    let ref_width = if number <= 0xFF {
        1usize
    } else if number <= 0xFFFF {
        2
    } else {
        4
    };
    let mut referred = Vec::with_capacity(num_ref as usize);
    for _ in 0..num_ref {
        let mut rb = [0u8; 4];
        read_exact(r, &mut rb[..ref_width])?;
        let mut v: u32 = 0;
        for k in 0..ref_width {
            v = (v << 8) | rb[k] as u32;
        }
        referred.push(v);
    }

    let mut buf4 = [0u8; 4];
    let page_association = if page_assoc_size {
        read_exact(r, &mut buf4)?;
        u32::from_be_bytes(buf4)
    } else {
        let mut pb = [0u8; 1];
        read_exact(r, &mut pb)?;
        pb[0] as u32
    };

    read_exact(r, &mut buf4)?;
    let raw_len = u32::from_be_bytes(buf4);
    let data_length = if raw_len == 0xFFFF_FFFF {
        None
    } else {
        Some(raw_len)
    };

    let mut retain_bits = Vec::with_capacity((num_ref + 1) as usize);
    for i in 0..=num_ref {
        let byte = retain.get((i as usize) / 8).copied().unwrap_or(0);
        retain_bits.push((byte >> (i & 7)) & 1 != 0);
    }

    Ok(SegmentHeader {
        number,
        segment_type,
        deferred_non_retain,
        referred,
        retain_bits,
        page_association,
        data_length,
    })
}

fn read_exact<R: Read>(r: &mut R, buf: &mut [u8]) -> Jbig2Result<()> {
    r.read_exact(buf).map_err(Jbig2Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn round_trip_short_form() {
        let hdr = SegmentHeader {
            number: 12,
            segment_type: SegmentType::PageInformation,
            deferred_non_retain: false,
            referred: vec![],
            retain_bits: vec![false],
            page_association: 1,
            data_length: Some(19),
        };
        let mut buf = Vec::new();
        hdr.write(&mut buf).unwrap();
        let mut cur = Cursor::new(&buf);
        let back = SegmentHeader::read(&mut cur).unwrap();
        assert_eq!(back.number, 12);
        assert_eq!(back.segment_type, SegmentType::PageInformation);
        assert_eq!(back.page_association, 1);
        assert_eq!(back.data_length, Some(19));
        assert_eq!(back.referred.len(), 0);
    }

    #[test]
    fn round_trip_with_referred() {
        let hdr = SegmentHeader {
            number: 300,
            segment_type: SegmentType::ImmediateTextRegion,
            deferred_non_retain: false,
            referred: vec![1, 2, 3],
            retain_bits: vec![true, false, true, false],
            page_association: 2,
            data_length: None,
        };
        let mut buf = Vec::new();
        hdr.write(&mut buf).unwrap();
        let mut cur = Cursor::new(&buf);
        let back = SegmentHeader::read(&mut cur).unwrap();
        assert_eq!(back.number, 300);
        assert_eq!(back.segment_type, SegmentType::ImmediateTextRegion);
        assert_eq!(back.referred, vec![1u32, 2, 3]);
        assert_eq!(back.page_association, 2);
        assert_eq!(back.data_length, None);
    }
}
