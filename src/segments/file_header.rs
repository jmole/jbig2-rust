//! JBIG2 file header (spec D.4).

use std::io::{Read, Write};

use crate::error::{Jbig2Error, Jbig2Result};
use crate::JBIG2_MAGIC;

/// Decoded JBIG2 file header.
#[derive(Clone, Copy, Debug)]
pub struct FileHeader {
    /// `true` ⇒ sequential organization (bit 0 set in the flags byte).
    pub sequential: bool,
    /// `true` ⇒ "unknown number of pages" bit is set.
    pub unknown_page_count: bool,
    /// `true` ⇒ file contains at least one generic region with 12-AT
    /// extended template (AMD2).
    pub uses_extended_template: bool,
    /// `true` ⇒ file contains at least one coloured region segment.
    pub uses_colour: bool,
    /// Number of pages, if known. `None` ⇒ `unknown_page_count` is set.
    pub num_pages: Option<u32>,
}

impl FileHeader {
    /// Parse the file header. Fails with [`Jbig2Error::InvalidMagic`] if the
    /// 8-byte ID string does not match [`crate::JBIG2_MAGIC`].
    pub fn read<R: Read>(r: &mut R) -> Jbig2Result<Self> {
        let mut magic = [0u8; 8];
        r.read_exact(&mut magic).map_err(Jbig2Error::from)?;
        if magic != JBIG2_MAGIC {
            return Err(Jbig2Error::InvalidMagic);
        }
        let mut flags = [0u8; 1];
        r.read_exact(&mut flags).map_err(Jbig2Error::from)?;
        let flags = flags[0];
        let sequential = (flags & 0x01) != 0;
        let unknown_page_count = (flags & 0x02) != 0;
        let uses_extended_template = (flags & 0x04) != 0;
        let uses_colour = (flags & 0x08) != 0;
        let num_pages = if unknown_page_count {
            None
        } else {
            let mut buf = [0u8; 4];
            r.read_exact(&mut buf).map_err(Jbig2Error::from)?;
            Some(u32::from_be_bytes(buf))
        };
        Ok(Self {
            sequential,
            unknown_page_count,
            uses_extended_template,
            uses_colour,
            num_pages,
        })
    }

    /// Emit the file header.
    pub fn write<W: Write>(&self, w: &mut W) -> Jbig2Result<()> {
        w.write_all(&JBIG2_MAGIC).map_err(Jbig2Error::from)?;
        let flags = (self.sequential as u8)
            | ((self.unknown_page_count as u8) << 1)
            | ((self.uses_extended_template as u8) << 2)
            | ((self.uses_colour as u8) << 3);
        w.write_all(&[flags]).map_err(Jbig2Error::from)?;
        if !self.unknown_page_count {
            let n = self.num_pages.unwrap_or(0);
            w.write_all(&n.to_be_bytes()).map_err(Jbig2Error::from)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn round_trip_sequential_single_page() {
        let h = FileHeader {
            sequential: true,
            unknown_page_count: false,
            uses_extended_template: false,
            uses_colour: false,
            num_pages: Some(1),
        };
        let mut buf = Vec::new();
        h.write(&mut buf).unwrap();
        // 8 magic + 1 flag + 4 page count
        assert_eq!(buf.len(), 13);
        let back = FileHeader::read(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(back.num_pages, Some(1));
        assert!(back.sequential);
    }

    #[test]
    fn round_trip_unknown_pages() {
        let h = FileHeader {
            sequential: true,
            unknown_page_count: true,
            uses_extended_template: false,
            uses_colour: false,
            num_pages: None,
        };
        let mut buf = Vec::new();
        h.write(&mut buf).unwrap();
        assert_eq!(buf.len(), 9);
        let back = FileHeader::read(&mut Cursor::new(&buf)).unwrap();
        assert!(back.unknown_page_count);
        assert_eq!(back.num_pages, None);
    }
}
