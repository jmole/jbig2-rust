//! Page information segment (spec 7.4.8).

use std::io::{Read, Write};

use crate::error::{Jbig2Error, Jbig2Result};

/// Combination operator for page / region composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum CombinationOp {
    Or = 0,
    And = 1,
    Xor = 2,
    XNor = 3,
    Replace = 4,
}

impl CombinationOp {
    /// Decode a 2-bit combination operator; `0..=3` are page/region-default
    /// operators, the optional `REPLACE` (4) is only valid on refinement region
    /// external combination.
    pub fn from_u8(v: u8) -> Self {
        match v & 0x07 {
            0 => Self::Or,
            1 => Self::And,
            2 => Self::Xor,
            3 => Self::XNor,
            _ => Self::Replace,
        }
    }

    /// Encode the operator into the 3-bit segment-header form.
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Decoded page information segment.
#[derive(Clone, Debug)]
pub struct PageInformation {
    /// Page bitmap width in pixels.
    pub width: u32,
    /// Page bitmap height in pixels. `0xFFFFFFFF` ⇒ unknown at encode time.
    pub height: u32,
    /// X resolution (pixels/m). `0` ⇒ unknown.
    pub x_resolution: u32,
    /// Y resolution (pixels/m). `0` ⇒ unknown.
    pub y_resolution: u32,
    /// `true` if the file is marked as eventually lossless.
    pub is_lossless: bool,
    /// `true` if refinement regions may occur on this page.
    pub may_contain_refinements: bool,
    /// Initial pixel value used to fill the page before any regions are drawn.
    pub default_pixel: u8,
    /// Default combination operator for regions without an override.
    pub default_combination_op: CombinationOp,
    /// Whether the page may need auxiliary buffers.
    pub requires_aux_buffers: bool,
    /// Whether per-region combination operator may override the default.
    pub combination_op_override: bool,
    /// Whether coloured segments are allowed on this page.
    pub may_contain_colour: bool,
    /// `true` if the page is striped (stripe size in `maximum_stripe_size`).
    pub is_striped: bool,
    /// Maximum stripe height, in pixel rows.
    pub maximum_stripe_size: u16,
}

impl PageInformation {
    /// Spec-defined on-the-wire size of this segment (19 bytes).
    pub const WIRE_SIZE: u32 = 19;

    /// Parse a page information segment body.
    pub fn read<R: Read>(r: &mut R) -> Jbig2Result<Self> {
        let mut buf = [0u8; 19];
        r.read_exact(&mut buf).map_err(Jbig2Error::from)?;
        let width = u32::from_be_bytes(buf[0..4].try_into().unwrap());
        let height = u32::from_be_bytes(buf[4..8].try_into().unwrap());
        let x_resolution = u32::from_be_bytes(buf[8..12].try_into().unwrap());
        let y_resolution = u32::from_be_bytes(buf[12..16].try_into().unwrap());
        let flags = buf[16];
        let stripe = u16::from_be_bytes(buf[17..19].try_into().unwrap());
        Ok(Self {
            width,
            height,
            x_resolution,
            y_resolution,
            is_lossless: (flags & 0x01) != 0,
            may_contain_refinements: (flags & 0x02) != 0,
            default_pixel: (flags >> 2) & 1,
            default_combination_op: CombinationOp::from_u8((flags >> 3) & 0x03),
            requires_aux_buffers: (flags & 0x20) != 0,
            combination_op_override: (flags & 0x40) != 0,
            may_contain_colour: (flags & 0x80) != 0,
            is_striped: (stripe & 0x8000) != 0,
            maximum_stripe_size: stripe & 0x7FFF,
        })
    }

    /// Emit the 19-byte page information segment body.
    pub fn write<W: Write>(&self, w: &mut W) -> Jbig2Result<()> {
        let mut buf = [0u8; 19];
        buf[0..4].copy_from_slice(&self.width.to_be_bytes());
        buf[4..8].copy_from_slice(&self.height.to_be_bytes());
        buf[8..12].copy_from_slice(&self.x_resolution.to_be_bytes());
        buf[12..16].copy_from_slice(&self.y_resolution.to_be_bytes());
        let combo = match self.default_combination_op {
            CombinationOp::Or => 0,
            CombinationOp::And => 1,
            CombinationOp::Xor => 2,
            CombinationOp::XNor => 3,
            CombinationOp::Replace => 0, // not valid here; fall back to OR
        };
        let flags = (self.is_lossless as u8)
            | ((self.may_contain_refinements as u8) << 1)
            | ((self.default_pixel & 1) << 2)
            | ((combo & 0x03) << 3)
            | ((self.requires_aux_buffers as u8) << 5)
            | ((self.combination_op_override as u8) << 6)
            | ((self.may_contain_colour as u8) << 7);
        buf[16] = flags;
        let stripe =
            (self.maximum_stripe_size & 0x7FFF) | if self.is_striped { 0x8000 } else { 0 };
        buf[17..19].copy_from_slice(&stripe.to_be_bytes());
        w.write_all(&buf).map_err(Jbig2Error::from)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn round_trip_default() {
        let pi = PageInformation {
            width: 1728,
            height: 2339,
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
        let mut buf = Vec::new();
        pi.write(&mut buf).unwrap();
        assert_eq!(buf.len(), 19);
        let back = PageInformation::read(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(back.width, pi.width);
        assert_eq!(back.height, pi.height);
        assert!(back.is_lossless);
        assert!(!back.is_striped);
    }
}
