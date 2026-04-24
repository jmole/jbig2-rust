//! End-of-page segment (spec 7.4.9). Carries no data.

use crate::error::Jbig2Result;

/// Placeholder type: end-of-page segments have zero data bytes.
#[derive(Clone, Copy, Debug, Default)]
pub struct EndOfPage;

impl EndOfPage {
    /// Data length (always zero).
    pub const DATA_LENGTH: u32 = 0;

    /// No-op reader (no payload to consume).
    pub fn read() -> Jbig2Result<Self> {
        Ok(Self)
    }

    /// No-op writer.
    pub fn write<W: std::io::Write>(&self, _w: &mut W) -> Jbig2Result<()> {
        Ok(())
    }
}
