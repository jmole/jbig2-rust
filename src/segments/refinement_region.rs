//! Generic refinement region segment (spec 7.4.7 / 6.3).
//!
//! The v1 crate parses refinement region headers so every segment in a file
//! can be enumerated without errors, but does not yet run the refinement
//! coding procedure. Implementing 6.3 requires wiring a second template of
//! target + reference pixels through the MQ coder; it is tracked as a
//! follow-up milestone.

use std::io::{Read, Write};

use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::region_info::RegionInfo;

/// Parsed refinement region header.
#[derive(Clone, Debug)]
pub struct RefinementRegionHeader {
    /// Region info (width/height/page coordinates).
    pub region: RegionInfo,
    /// `GRTEMPLATE` — 0 = 13-pixel context, 1 = 10-pixel context.
    pub template: u8,
    /// `TPGRON` — typical-prediction-for-generic-refinement.
    pub tpgron: bool,
    /// Adaptive template pixels (`GRAT0x/y` target + `GRAT1x/y` reference).
    /// Only meaningful for `template = 0`.
    pub at: [(i8, i8); 2],
}

impl RefinementRegionHeader {
    /// Parse a refinement region header from a reader positioned at the
    /// region info field.
    pub fn read<R: Read>(r: &mut R) -> Jbig2Result<Self> {
        let region = RegionInfo::read(r)?;
        let mut fb = [0u8; 1];
        r.read_exact(&mut fb)?;
        let flags = fb[0];
        let template = flags & 0x01;
        let tpgron = (flags & 0x02) != 0;
        let mut at = [(0i8, 0i8); 2];
        if template == 0 {
            for i in 0..2 {
                let mut b = [0u8; 2];
                r.read_exact(&mut b)?;
                at[i] = (b[0] as i8, b[1] as i8);
            }
        }
        Ok(Self {
            region,
            template,
            tpgron,
            at,
        })
    }

    /// Emit the refinement region header.
    pub fn write<W: Write>(&self, w: &mut W) -> Jbig2Result<()> {
        self.region.write(w)?;
        let flags = (self.template & 0x01) | if self.tpgron { 0x02 } else { 0 };
        w.write_all(&[flags])?;
        if self.template == 0 {
            for i in 0..2 {
                w.write_all(&[self.at[i].0 as u8, self.at[i].1 as u8])?;
            }
        }
        Ok(())
    }
}

/// Stub for decoding a refinement region body. Returns an "unsupported"
/// error pending a full implementation of the refinement coding procedure
/// from spec 6.3.
pub fn decode_refinement_region_unimplemented() -> Jbig2Result<()> {
    Err(Jbig2Error::Unsupported(
        "generic refinement region decode (spec 6.3) — planned for a later milestone",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segments::page_information::CombinationOp;

    #[test]
    fn header_round_trip_template0() {
        let hdr = RefinementRegionHeader {
            region: RegionInfo {
                width: 50,
                height: 20,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            template: 0,
            tpgron: false,
            at: [(-1, -1), (-1, -1)],
        };
        let mut buf = Vec::new();
        hdr.write(&mut buf).unwrap();
        let back = RefinementRegionHeader::read(&mut buf.as_slice()).unwrap();
        assert_eq!(back.template, 0);
        assert_eq!(back.at, hdr.at);
    }

    #[test]
    fn header_round_trip_template1() {
        let hdr = RefinementRegionHeader {
            region: RegionInfo {
                width: 50,
                height: 20,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            template: 1,
            tpgron: true,
            at: [(0, 0); 2],
        };
        let mut buf = Vec::new();
        hdr.write(&mut buf).unwrap();
        let back = RefinementRegionHeader::read(&mut buf.as_slice()).unwrap();
        assert_eq!(back.template, 1);
        assert!(back.tpgron);
    }
}
