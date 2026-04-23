//! Colour palette segment (spec 7.4.16 / 6.8).

use std::io::{Read, Write};

use crate::error::{Jbig2Error, Jbig2Result};

/// Parsed colour-palette segment data header.
#[derive(Clone, Debug)]
pub struct ColourPaletteHeader {
    /// Whether the flags field continues beyond the first byte.
    pub flagfield_continues: bool,
    /// Colour-space selector from the first flags byte.
    pub colour_space: u8,
    /// `CPNCOMP`.
    pub num_components: u8,
    /// `CPCOMPLEN`.
    pub component_len: u8,
    /// `CPNVALS`.
    pub num_values: u32,
    /// Number of bytes consumed by the flags field.
    pub flags_len: u8,
}

/// Decoded colour-palette segment contents.
#[derive(Clone, Debug)]
pub struct ColourPalette {
    /// Parsed header.
    pub header: ColourPaletteHeader,
    /// Raw decoded colour values, one entry per palette value.
    pub values: Vec<Vec<u32>>,
}

impl ColourPaletteHeader {
    /// Parse the colour-palette header from the start of a segment body.
    pub fn read<R: Read>(r: &mut R) -> Jbig2Result<Self> {
        let mut flags_len = 0u8;
        let mut first = 0u8;
        let mut continue_flag = true;
        while continue_flag {
            let mut b = [0u8; 1];
            r.read_exact(&mut b)?;
            if flags_len == 0 {
                first = b[0];
            }
            flags_len = flags_len.saturating_add(1);
            continue_flag = (b[0] & 0x01) != 0;
        }
        let mut b = [0u8; 1];
        r.read_exact(&mut b)?;
        let num_components = b[0];
        r.read_exact(&mut b)?;
        let component_len = b[0];
        let mut n = [0u8; 4];
        r.read_exact(&mut n)?;
        Ok(Self {
            flagfield_continues: (first & 0x01) != 0,
            colour_space: (first >> 1) & 0x0f,
            num_components,
            component_len,
            num_values: u32::from_be_bytes(n),
            flags_len,
        })
    }

    /// Emit the colour-palette header.
    pub fn write<W: Write>(&self, w: &mut W) -> Jbig2Result<()> {
        // We only emit the non-extended one-byte flags form.
        let flags = ((self.colour_space & 0x0f) << 1) | (self.flagfield_continues as u8);
        w.write_all(&[flags])?;
        w.write_all(&[self.num_components])?;
        w.write_all(&[self.component_len])?;
        w.write_all(&self.num_values.to_be_bytes())?;
        Ok(())
    }
}

impl ColourPalette {
    /// Decode a complete colour-palette segment body.
    pub fn decode(body: &[u8]) -> Jbig2Result<Self> {
        let mut cur = std::io::Cursor::new(body);
        let header = ColourPaletteHeader::read(&mut cur)?;
        let mut values = Vec::with_capacity(header.num_values as usize);
        for _ in 0..header.num_values {
            let mut comps = Vec::with_capacity(header.num_components as usize);
            for _ in 0..header.num_components {
                let v = match header.component_len {
                    1 => {
                        let mut b = [0u8; 1];
                        cur.read_exact(&mut b)?;
                        b[0] as u32
                    }
                    2 => {
                        let mut b = [0u8; 2];
                        cur.read_exact(&mut b)?;
                        u16::from_be_bytes(b) as u32
                    }
                    4 => {
                        let mut b = [0u8; 4];
                        cur.read_exact(&mut b)?;
                        u32::from_be_bytes(b)
                    }
                    _ => {
                        return Err(Jbig2Error::Unsupported(
                            "colour palette: unsupported component length",
                        ));
                    }
                };
                comps.push(v);
            }
            values.push(comps);
        }
        Ok(Self { header, values })
    }

    /// Convert the decoded palette values into RGB triples.
    pub fn rgb_values(&self) -> Jbig2Result<Vec<[u8; 3]>> {
        let mut out = Vec::with_capacity(self.values.len());
        for value in &self.values {
            if value.len() < 3 {
                return Err(Jbig2Error::Unsupported(
                    "colour palette: need at least 3 components for RGB output",
                ));
            }
            out.push([value[0] as u8, value[1] as u8, value[2] as u8]);
        }
        Ok(out)
    }
}

/// The 32 default colours from spec Table 30.
pub const DEFAULT_COLOURS: [[u8; 3]; 32] = [
    [0, 0, 0],
    [128, 128, 128],
    [192, 192, 192],
    [255, 255, 255],
    [255, 0, 0],
    [0, 255, 0],
    [0, 0, 255],
    [255, 255, 0],
    [0, 255, 255],
    [255, 0, 255],
    [128, 0, 0],
    [0, 128, 0],
    [0, 0, 128],
    [128, 128, 0],
    [0, 128, 128],
    [128, 0, 128],
    [255, 165, 0],
    [204, 204, 0],
    [153, 0, 0],
    [0, 204, 0],
    [0, 153, 0],
    [204, 204, 0],
    [153, 153, 0],
    [102, 0, 0],
    [0, 0, 204],
    [0, 0, 153],
    [204, 0, 204],
    [153, 0, 153],
    [0, 204, 204],
    [0, 153, 153],
    [102, 102, 102],
    [153, 153, 153],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip() {
        let hdr = ColourPaletteHeader {
            flagfield_continues: false,
            colour_space: 0,
            num_components: 3,
            component_len: 1,
            num_values: 2,
            flags_len: 1,
        };
        let mut buf = Vec::new();
        hdr.write(&mut buf).unwrap();
        let back = ColourPaletteHeader::read(&mut buf.as_slice()).unwrap();
        assert_eq!(back.colour_space, 0);
        assert_eq!(back.num_components, 3);
        assert_eq!(back.component_len, 1);
        assert_eq!(back.num_values, 2);
    }

    #[test]
    fn decode_palette_body() {
        let body = [
            0x00, // flags
            0x03, // components
            0x01, // component length
            0x00, 0x00, 0x00, 0x02, // values
            0x01, 0x02, 0x03,
            0x04, 0x05, 0x06,
        ];
        let pal = ColourPalette::decode(&body).unwrap();
        assert_eq!(pal.rgb_values().unwrap(), vec![[1, 2, 3], [4, 5, 6]]);
    }
}
