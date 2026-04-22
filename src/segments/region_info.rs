//! Region segment information field (spec 7.4.1). All region-style
//! segments (generic, refinement, text, halftone) start with this 17-byte
//! preamble.

use std::io::{Read, Write};

use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::page_information::CombinationOp;

/// Parsed region-segment information field.
#[derive(Clone, Copy, Debug)]
pub struct RegionInfo {
    /// Region bitmap width.
    pub width: u32,
    /// Region bitmap height.
    pub height: u32,
    /// X coordinate of the region on the page bitmap.
    pub x: u32,
    /// Y coordinate of the region on the page bitmap.
    pub y: u32,
    /// External combination operator. When the page flag
    /// `combination_op_override` is clear the decoder must use the page
    /// default.
    pub external_combination_op: CombinationOp,
    /// Whether the region is extended to colour (AMD3).
    pub colour_extension: bool,
}

impl RegionInfo {
    /// On-the-wire size (17 bytes).
    pub const WIRE_SIZE: u32 = 17;

    /// Parse the 17-byte region info.
    pub fn read<R: Read>(r: &mut R) -> Jbig2Result<Self> {
        let mut buf = [0u8; 17];
        r.read_exact(&mut buf).map_err(Jbig2Error::from)?;
        let width = u32::from_be_bytes(buf[0..4].try_into().unwrap());
        let height = u32::from_be_bytes(buf[4..8].try_into().unwrap());
        let x = u32::from_be_bytes(buf[8..12].try_into().unwrap());
        let y = u32::from_be_bytes(buf[12..16].try_into().unwrap());
        let flags = buf[16];
        Ok(Self {
            width,
            height,
            x,
            y,
            external_combination_op: CombinationOp::from_u8(flags & 0x07),
            colour_extension: (flags & 0x08) != 0,
        })
    }

    /// Emit the 17-byte region info.
    pub fn write<W: Write>(&self, w: &mut W) -> Jbig2Result<()> {
        let mut buf = [0u8; 17];
        buf[0..4].copy_from_slice(&self.width.to_be_bytes());
        buf[4..8].copy_from_slice(&self.height.to_be_bytes());
        buf[8..12].copy_from_slice(&self.x.to_be_bytes());
        buf[12..16].copy_from_slice(&self.y.to_be_bytes());
        let flags = (match self.external_combination_op {
            CombinationOp::Or => 0u8,
            CombinationOp::And => 1,
            CombinationOp::Xor => 2,
            CombinationOp::XNor => 3,
            CombinationOp::Replace => 4,
        }) | if self.colour_extension { 0x08 } else { 0 };
        buf[16] = flags;
        w.write_all(&buf).map_err(Jbig2Error::from)?;
        Ok(())
    }
}
