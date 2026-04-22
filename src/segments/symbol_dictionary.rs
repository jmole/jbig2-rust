//! Symbol dictionary segment (spec 7.4.2 / 6.5).
//!
//! The v1 implementation covers the arithmetic-coded, non-refinement/aggregate
//! path (`SDHUFF = 0`, `SDREFAGG = 0`). This is the default path produced by
//! `jbig2enc -S` and by this crate's lossless symbol classifier, and is what
//! every JBIG2 decoder in real-world use consumes for text regions.
//!
//! Each symbol in the dictionary is emitted as a tiny generic region using the
//! selected GBTEMPLATE and AT pixels (from the dictionary header). Symbols are
//! grouped into height classes so the widths can be delta-coded; the stream
//! is terminated on each height class by an out-of-band DW value.
//!
//! Arithmetic contexts are shared across every symbol in the dictionary, so
//! `MqContexts` is held once and threaded through every bitmap coder call.

use std::io::{Read, Write};

use crate::bitmap::Bitmap;
use crate::coding::mq::{MqContexts, MqDecoder, MqEncoder, MQ_NUM_CONTEXTS};
use crate::coding::mq_context::{IADH, IADW, IAEX};
use crate::coding::mq_integer::{decode_integer, encode_integer, OOB};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::generic_region::{
    decode_generic_bitmap, encode_generic_bitmap, nominal_at,
};

/// Symbol dictionary segment header (the 2-byte flags field + AT pixels +
/// SDNUMEXSYMS + SDNUMNEWSYMS). Refinement/aggregation fields are parsed but
/// the v1 decoder rejects them at decode time.
#[derive(Clone, Debug)]
pub struct SymbolDictionaryHeader {
    /// `true` when Huffman-encoded (`SDHUFF`).
    pub sdhuff: bool,
    /// `true` when refinement/aggregate coding is used (`SDREFAGG`).
    pub sdrefagg: bool,
    /// `SDHUFFDH` selector (0..3).
    pub sdhuff_dh: u8,
    /// `SDHUFFDW` selector (0..3).
    pub sdhuff_dw: u8,
    /// `SDHUFFBMSIZE` selector (0/1).
    pub sdhuff_bmsize: bool,
    /// `SDHUFFAGGINST` selector (0/1).
    pub sdhuff_agginst: bool,
    /// Arithmetic coding context used from a referenced dictionary.
    pub context_used: bool,
    /// Preserve arithmetic coding context for use by another dictionary.
    pub context_retained: bool,
    /// `SDTEMPLATE` (0..3).
    pub sd_template: u8,
    /// `SDRTEMPLATE` (0/1), valid only when `sdrefagg = true`.
    pub sd_rtemplate: bool,
    /// AT pixel offsets (size matches `sd_template`; unused slots are `(0, 0)`).
    pub at: [(i8, i8); 4],
    /// Refinement AT pixels (only used when `sdrefagg = true` and
    /// `sd_rtemplate = false`).
    pub rat: [(i8, i8); 2],
    /// Number of symbols exported from this dictionary.
    pub num_ex_syms: u32,
    /// Number of new symbols defined in this dictionary.
    pub num_new_syms: u32,
}

impl SymbolDictionaryHeader {
    /// Parse the dictionary header. `r` must be positioned at the first flag
    /// byte.
    pub fn read<R: Read>(r: &mut R) -> Jbig2Result<Self> {
        let mut buf = [0u8; 2];
        r.read_exact(&mut buf)?;
        let flags = u16::from_be_bytes(buf);
        let sdhuff = (flags & 0x0001) != 0;
        let sdrefagg = (flags & 0x0002) != 0;
        let sdhuff_dh = ((flags >> 2) & 0x3) as u8;
        let sdhuff_dw = ((flags >> 4) & 0x3) as u8;
        let sdhuff_bmsize = ((flags >> 6) & 0x1) != 0;
        let sdhuff_agginst = ((flags >> 7) & 0x1) != 0;
        let context_used = ((flags >> 8) & 0x1) != 0;
        let context_retained = ((flags >> 9) & 0x1) != 0;
        let sd_template = ((flags >> 10) & 0x3) as u8;
        let sd_rtemplate = ((flags >> 12) & 0x1) != 0;
        if sdhuff_dh == 2 || sdhuff_dw == 2 {
            return Err(Jbig2Error::InvalidSegmentHeader(
                "symbol dictionary: SDHUFFDH/DW == 2 is reserved",
            ));
        }

        let mut at = [(0i8, 0i8); 4];
        if !sdhuff {
            let n = if sd_template == 0 { 4 } else { 1 };
            for i in 0..n {
                let mut b = [0u8; 2];
                r.read_exact(&mut b)?;
                at[i] = (b[0] as i8, b[1] as i8);
            }
        }
        let mut rat = [(0i8, 0i8); 2];
        if sdrefagg && !sd_rtemplate {
            for i in 0..2 {
                let mut b = [0u8; 2];
                r.read_exact(&mut b)?;
                rat[i] = (b[0] as i8, b[1] as i8);
            }
        }

        let mut ne = [0u8; 4];
        r.read_exact(&mut ne)?;
        let num_ex_syms = u32::from_be_bytes(ne);
        r.read_exact(&mut ne)?;
        let num_new_syms = u32::from_be_bytes(ne);

        Ok(Self {
            sdhuff,
            sdrefagg,
            sdhuff_dh,
            sdhuff_dw,
            sdhuff_bmsize,
            sdhuff_agginst,
            context_used,
            context_retained,
            sd_template,
            sd_rtemplate,
            at,
            rat,
            num_ex_syms,
            num_new_syms,
        })
    }

    /// Emit the dictionary header.
    pub fn write<W: Write>(&self, w: &mut W) -> Jbig2Result<()> {
        let flags: u16 = (self.sdhuff as u16)
            | ((self.sdrefagg as u16) << 1)
            | ((self.sdhuff_dh as u16 & 0x3) << 2)
            | ((self.sdhuff_dw as u16 & 0x3) << 4)
            | ((self.sdhuff_bmsize as u16) << 6)
            | ((self.sdhuff_agginst as u16) << 7)
            | ((self.context_used as u16) << 8)
            | ((self.context_retained as u16) << 9)
            | ((self.sd_template as u16 & 0x3) << 10)
            | ((self.sd_rtemplate as u16) << 12);
        w.write_all(&flags.to_be_bytes())?;
        if !self.sdhuff {
            let n = if self.sd_template == 0 { 4 } else { 1 };
            for i in 0..n {
                w.write_all(&[self.at[i].0 as u8, self.at[i].1 as u8])?;
            }
        }
        if self.sdrefagg && !self.sd_rtemplate {
            for i in 0..2 {
                w.write_all(&[self.rat[i].0 as u8, self.rat[i].1 as u8])?;
            }
        }
        w.write_all(&self.num_ex_syms.to_be_bytes())?;
        w.write_all(&self.num_new_syms.to_be_bytes())?;
        Ok(())
    }

    /// Size in bytes for a typical (non-refagg) header plus the 2×2 AT
    /// bytes. Useful for length calculations.
    pub fn wire_size(&self) -> usize {
        let mut sz = 2;
        if !self.sdhuff {
            sz += if self.sd_template == 0 { 8 } else { 2 };
        }
        if self.sdrefagg && !self.sd_rtemplate {
            sz += 4;
        }
        sz + 8
    }

    /// Default header for the arithmetic-only, no-refagg path.
    pub fn default_arithmetic(template: u8, num_ex: u32, num_new: u32) -> Self {
        let at_full = nominal_at(template, false);
        let mut at = [(0i8, 0i8); 4];
        for (i, slot) in at.iter_mut().enumerate() {
            *slot = at_full[i];
        }
        Self {
            sdhuff: false,
            sdrefagg: false,
            sdhuff_dh: 0,
            sdhuff_dw: 0,
            sdhuff_bmsize: false,
            sdhuff_agginst: false,
            context_used: false,
            context_retained: false,
            sd_template: template,
            sd_rtemplate: false,
            at,
            rat: [(0, 0); 2],
            num_ex_syms: num_ex,
            num_new_syms: num_new,
        }
    }
}

/// Output of decoding a symbol dictionary segment.
#[derive(Clone, Debug)]
pub struct DecodedSymbolDictionary {
    /// Newly defined symbols in dictionary order (length = `num_new_syms`).
    pub new_symbols: Vec<Bitmap>,
    /// Exported symbols (length = `num_ex_syms`). This is the array exposed
    /// to downstream text regions.
    pub exported: Vec<Bitmap>,
}

/// Decode the body of a symbol dictionary segment, with any imported symbols
/// from previously-decoded dictionaries. `body` is the segment's data bytes
/// *after* the dictionary header.
pub fn decode_symbol_dictionary(
    header: &SymbolDictionaryHeader,
    body: &[u8],
    import_symbols: &[Bitmap],
) -> Jbig2Result<DecodedSymbolDictionary> {
    if header.sdhuff {
        return Err(Jbig2Error::Unsupported(
            "symbol dictionary: Huffman coding not yet implemented",
        ));
    }
    if header.sdrefagg {
        return Err(Jbig2Error::Unsupported(
            "symbol dictionary: refinement/aggregation not yet implemented",
        ));
    }

    let mut at = [(0i8, 0i8); 12];
    for (i, slot) in at.iter_mut().enumerate().take(4) {
        *slot = header.at[i];
    }

    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    let mut dec = MqDecoder::new(body);

    let mut new_symbols: Vec<Bitmap> = Vec::with_capacity(header.num_new_syms as usize);
    let mut hc_height: i32 = 0;
    while (new_symbols.len() as u32) < header.num_new_syms {
        // Step 4 b): height-class delta height (must not be OOB).
        let hcdh = decode_integer(&mut dec, &mut cxs, IADH).ok_or(
            Jbig2Error::InvalidHuffman("symbol dictionary: IADH returned OOB"),
        )?;
        hc_height = hc_height.saturating_add(hcdh);
        if hc_height < 0 {
            return Err(Jbig2Error::OutOfRange(
                "symbol dictionary: negative height class height",
            ));
        }

        // Step 4 c): symbols within this height class, terminated by OOB.
        let mut sym_width: i32 = 0;
        loop {
            let dw = decode_integer(&mut dec, &mut cxs, IADW);
            let dw = match dw {
                Some(v) => v,
                None => break,
            };
            sym_width = sym_width.saturating_add(dw);
            if sym_width < 0 {
                return Err(Jbig2Error::OutOfRange(
                    "symbol dictionary: negative symbol width",
                ));
            }
            if new_symbols.len() as u32 >= header.num_new_syms {
                return Err(Jbig2Error::OutOfRange(
                    "symbol dictionary: more symbols than SDNUMNEWSYMS",
                ));
            }
            let bm = decode_generic_bitmap(
                &mut dec,
                &mut cxs,
                sym_width as u32,
                hc_height as u32,
                header.sd_template,
                false,
                false,
                &at,
            )?;
            new_symbols.push(bm);
        }
    }

    // Step 5 / 6.5.10: decode the export flag run-length stream.
    let total = import_symbols.len() as u32 + header.num_new_syms;
    let mut flags = vec![0u8; total as usize];
    let mut idx: u32 = 0;
    let mut cur: u8 = 0;
    while idx < total {
        let run = decode_integer(&mut dec, &mut cxs, IAEX).ok_or(
            Jbig2Error::InvalidHuffman("symbol dictionary: IAEX returned OOB"),
        )? as u32;
        let end = idx.checked_add(run).ok_or(Jbig2Error::OutOfRange(
            "symbol dictionary: export run overflows",
        ))?;
        if end > total {
            return Err(Jbig2Error::OutOfRange(
                "symbol dictionary: export run exceeds total symbol count",
            ));
        }
        for s in idx..end {
            flags[s as usize] = cur;
        }
        idx = end;
        cur ^= 1;
    }

    let mut exported = Vec::with_capacity(header.num_ex_syms as usize);
    let n_in = import_symbols.len();
    for i in 0..total as usize {
        if flags[i] == 1 {
            if i < n_in {
                exported.push(import_symbols[i].clone());
            } else {
                exported.push(new_symbols[i - n_in].clone());
            }
        }
    }
    if exported.len() as u32 != header.num_ex_syms {
        return Err(Jbig2Error::OutOfRange(
            "symbol dictionary: export count mismatch",
        ));
    }

    Ok(DecodedSymbolDictionary {
        new_symbols,
        exported,
    })
}

/// Build a symbol dictionary body (without the segment header or the
/// dictionary header). Groups `symbols` into height classes, delta-codes
/// widths, and emits the export-flag run-length stream.
///
/// `import_count` is the number of previously-imported symbols that sit in
/// the logical `SBSYMS` array before the new ones; for a fresh dictionary
/// that's zero.
pub fn encode_symbol_dictionary(
    header: &SymbolDictionaryHeader,
    symbols: &[Bitmap],
    import_count: u32,
) -> Jbig2Result<Vec<u8>> {
    if header.sdhuff || header.sdrefagg {
        return Err(Jbig2Error::Unsupported(
            "symbol dictionary encoder: only arithmetic non-refagg path supported",
        ));
    }
    if symbols.len() as u32 != header.num_new_syms {
        return Err(Jbig2Error::InvalidConfig(
            "symbol dictionary: symbol count != num_new_syms",
        ));
    }

    // Order: symbols are already in dictionary order, grouped by height.
    // We expect the caller to have sorted them by height ascending; if not,
    // we group them as given and emit arbitrary HCDH deltas.
    let mut at = [(0i8, 0i8); 12];
    for (i, slot) in at.iter_mut().enumerate().take(4) {
        *slot = header.at[i];
    }

    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    let mut enc = MqEncoder::new(symbols.iter().map(|s| s.data().len()).sum::<usize>() + 64);

    let mut i = 0;
    let mut prev_height: i32 = 0;
    while i < symbols.len() {
        let h = symbols[i].height() as i32;
        encode_integer(&mut enc, &mut cxs, IADH, h - prev_height)?;
        prev_height = h;

        // Emit every symbol with the same height.
        let mut prev_width: i32 = 0;
        while i < symbols.len() && symbols[i].height() as i32 == h {
            let w = symbols[i].width() as i32;
            encode_integer(&mut enc, &mut cxs, IADW, w - prev_width)?;
            prev_width = w;
            encode_generic_bitmap(
                &mut enc,
                &mut cxs,
                &symbols[i],
                header.sd_template,
                false,
                false,
                &at,
            )?;
            i += 1;
        }
        // Terminate the height class with OOB.
        encode_integer(&mut enc, &mut cxs, IADW, OOB)?;
    }

    // Export run stream: classify the (import_count + num_new_syms) symbols
    // as "not exported" then "exported" runs. With the simple convention we
    // use (exports == new symbols, imports are not re-exported), that's a
    // single non-export run of length `import_count` followed by a single
    // export run of length `num_new_syms`.
    let mut cur: u8 = 0;
    if import_count > 0 {
        encode_integer(&mut enc, &mut cxs, IAEX, import_count as i32)?;
        cur ^= 1;
    } else {
        // Must emit at least one run before the first export.
        encode_integer(&mut enc, &mut cxs, IAEX, 0)?;
        cur ^= 1;
    }
    let _ = cur; // next run is the new-symbol run
    encode_integer(&mut enc, &mut cxs, IAEX, header.num_new_syms as i32)?;

    Ok(enc.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mkglyph(w: u32, h: u32, pattern: u32) -> Bitmap {
        let mut bm = Bitmap::new(w, h).unwrap();
        for y in 0..h {
            for x in 0..w {
                if ((x.wrapping_mul(1103515245).wrapping_add(y.wrapping_mul(12345).wrapping_add(pattern))) & 0x8) != 0 {
                    bm.set_pixel(x as i32, y as i32, 1);
                }
            }
        }
        bm
    }

    #[test]
    fn header_round_trip_arith() {
        let hdr = SymbolDictionaryHeader::default_arithmetic(0, 3, 3);
        let mut buf = Vec::new();
        hdr.write(&mut buf).unwrap();
        assert_eq!(buf.len(), hdr.wire_size());
        let back = SymbolDictionaryHeader::read(&mut buf.as_slice()).unwrap();
        assert_eq!(back.sd_template, 0);
        assert_eq!(back.num_ex_syms, 3);
        assert_eq!(back.num_new_syms, 3);
        assert!(!back.sdhuff);
    }

    #[test]
    fn round_trip_three_symbols_same_height() {
        let syms = vec![mkglyph(10, 12, 1), mkglyph(14, 12, 2), mkglyph(8, 12, 3)];
        let hdr = SymbolDictionaryHeader::default_arithmetic(0, syms.len() as u32, syms.len() as u32);
        let body = encode_symbol_dictionary(&hdr, &syms, 0).unwrap();
        let dec = decode_symbol_dictionary(&hdr, &body, &[]).unwrap();
        assert_eq!(dec.new_symbols.len(), syms.len());
        assert_eq!(dec.exported.len(), syms.len());
        for (i, s) in syms.iter().enumerate() {
            assert_eq!(&dec.new_symbols[i], s, "symbol {i} mismatch");
            assert_eq!(&dec.exported[i], s, "exported {i} mismatch");
        }
    }

    #[test]
    fn round_trip_multi_height() {
        let syms = vec![
            mkglyph(10, 8, 1),
            mkglyph(12, 8, 2),
            mkglyph(6, 10, 3),
            mkglyph(14, 10, 4),
            mkglyph(20, 15, 5),
        ];
        let hdr = SymbolDictionaryHeader::default_arithmetic(0, syms.len() as u32, syms.len() as u32);
        let body = encode_symbol_dictionary(&hdr, &syms, 0).unwrap();
        let dec = decode_symbol_dictionary(&hdr, &body, &[]).unwrap();
        for (i, s) in syms.iter().enumerate() {
            assert_eq!(&dec.new_symbols[i], s);
        }
    }

    #[test]
    fn round_trip_template3() {
        let syms = vec![mkglyph(7, 9, 11), mkglyph(11, 9, 22)];
        let hdr = SymbolDictionaryHeader::default_arithmetic(3, syms.len() as u32, syms.len() as u32);
        let body = encode_symbol_dictionary(&hdr, &syms, 0).unwrap();
        let dec = decode_symbol_dictionary(&hdr, &body, &[]).unwrap();
        assert_eq!(dec.new_symbols, syms);
    }
}
