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
use crate::coding::huffman::{standard_table, HuffReader};
use crate::coding::mq::{MqContexts, MqDecoder, MqEncoder, MQ_NUM_CONTEXTS};
use crate::coding::mq_context::{IAAI, IADH, IADW, IAEX, IAID, IARDX, IARDY};
use crate::coding::mq_integer::{decode_iaid, decode_integer, encode_iaid, encode_integer, OOB};
use crate::error::{Jbig2Error, Jbig2Result};
use crate::segments::generic_region::{decode_generic_bitmap, encode_generic_bitmap, nominal_at};
use crate::segments::page_information::CombinationOp;
use crate::segments::refinement_region::{decode_refinement_region, encode_refinement_region};
use crate::segments::region_info::RegionInfo;
use crate::segments::text_region::{
    decode_text_region_body_with_code_len, sym_code_len, RefCorner, TextRegionHeader,
};
use crate::segments::AtPixels;

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
    pub at: AtPixels,
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
            at: AtPixels::from_array(at),
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
            at: AtPixels::new(
                nominal_at(template, false).as_array(),
                if template == 0 { 4 } else { 1 },
            ),
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
///
/// This is a convenience wrapper that allocates a fresh MQ context pool for
/// the duration of the call. Multi-page decoders should prefer
/// [`decode_symbol_dictionary_with_contexts`] and hand in a caller-owned
/// pool so the allocation can be amortized across segments.
pub fn decode_symbol_dictionary(
    header: &SymbolDictionaryHeader,
    body: &[u8],
    import_symbols: &[&Bitmap],
) -> Jbig2Result<DecodedSymbolDictionary> {
    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    decode_symbol_dictionary_with_contexts(header, body, import_symbols, &mut cxs)
}

/// As [`decode_symbol_dictionary`] but reuses an externally-owned MQ
/// context pool. The pool is reset at entry (§7.4.3.1 requires a zero
/// initial state at the start of each symbol dictionary), so callers are
/// free to share it with other arithmetic decode families between
/// segments.
pub fn decode_symbol_dictionary_with_contexts(
    header: &SymbolDictionaryHeader,
    body: &[u8],
    import_symbols: &[&Bitmap],
    mut cxs: &mut MqContexts,
) -> Jbig2Result<DecodedSymbolDictionary> {
    if header.sdhuff {
        return decode_symbol_dictionary_huffman(header, body, import_symbols);
    }

    let at = AtPixels::new(header.at.as_array(), header.at.len() as u8);

    if cxs.len() < MQ_NUM_CONTEXTS {
        return Err(Jbig2Error::InvalidConfig(
            "symbol dictionary: external MQ context pool is too small",
        ));
    }
    cxs.reset();
    let mut dec = MqDecoder::new(body);

    let mut new_symbols: Vec<Bitmap> = Vec::with_capacity(header.num_new_syms as usize);
    let mut hc_height: i32 = 0;
    while (new_symbols.len() as u32) < header.num_new_syms {
        // Step 4 b): height-class delta height (must not be OOB).
        let hcdh = decode_integer(&mut dec, &mut cxs, IADH).ok_or(Jbig2Error::InvalidHuffman(
            "symbol dictionary: IADH returned OOB",
        ))?;
        hc_height = hc_height.saturating_add(hcdh);
        if hc_height < 0 {
            return Err(Jbig2Error::OutOfRange(
                "symbol dictionary: negative height class height",
            ));
        }

        // Step 4 c): symbols within this height class, terminated by OOB.
        let mut sym_width: i32 = 0;
        loop {
            // Reference encoder shortcut (`Jbig2ENC.cpp` line 432) skips
            // the OOB IADW when SDREFAGG=1 because the symbol count is
            // fixed by IAAI. Bail before we try to read past the body.
            if header.sdrefagg && (new_symbols.len() as u32) >= header.num_new_syms {
                break;
            }
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
            if (new_symbols.len() as u32) >= header.num_new_syms {
                return Err(Jbig2Error::OutOfRange(
                    "symbol dictionary: more symbols than SDNUMNEWSYMS",
                ));
            }
            let bm = if header.sdrefagg {
                decode_refagg_symbol(
                    &mut dec,
                    &mut cxs,
                    sym_width as u32,
                    hc_height as u32,
                    header,
                    import_symbols,
                    &new_symbols,
                )?
            } else {
                decode_generic_bitmap(
                    &mut dec,
                    &mut cxs,
                    sym_width as u32,
                    hc_height as u32,
                    header.sd_template,
                    false,
                    false,
                    &at,
                )?
            };
            new_symbols.push(bm);
        }
    }

    // Step 5 / 6.5.10: decode the export flag run-length stream.
    //
    // Note on conformance: the ITU-T T.88 reference encoder
    // (Jbig2ENC.cpp:434-435, 514-515) emits the export stream as a
    // hard-coded `IAEX(0), IAEX(0)` pair regardless of the actual
    // SBNUMSYMS, and its companion decoder reads exactly two IAEX
    // values and discards them, implicitly treating all new symbols as
    // exported. To stay interoperable with files produced by that
    // encoder (and conformance set TT4-TT7), we follow the spec
    // algorithm but fall back to the "export all new symbols"
    // convention when the encoded runs don't account for SBNUMSYMS,
    // SDNUMEXSYMS == SDNUMNEWSYMS, and the decoded body has been
    // exhausted.
    let total = import_symbols.len() as u32 + header.num_new_syms;
    let mut flags = vec![0u8; total as usize];
    let mut idx: u32 = 0;
    let mut cur: u8 = 0;
    let body_end = body.len();
    let mut exhausted_body = false;
    let mut invalid_export_stream = false;
    while idx < total {
        if dec.position() >= body_end {
            exhausted_body = true;
            break;
        }
        let run = decode_integer(&mut dec, &mut cxs, IAEX).ok_or(Jbig2Error::InvalidHuffman(
            "symbol dictionary: IAEX returned OOB",
        ))? as u32;
        let end = idx.checked_add(run).ok_or(Jbig2Error::OutOfRange(
            "symbol dictionary: export run overflows",
        ))?;
        if end > total {
            invalid_export_stream = true;
            break;
        }
        for s in idx..end {
            flags[s as usize] = cur;
        }
        idx = end;
        cur ^= 1;
    }

    if invalid_export_stream {
        // Same ITU reference-encoder shortcut as the EOF path below, but the
        // trailing bytes decoded as a formally-invalid IAEX run instead of
        // stopping cleanly at the body boundary.
        flags.fill(0);
        let n_in = import_symbols.len() as u32;
        if header.num_ex_syms == total {
            flags.fill(1);
        } else if header.num_ex_syms == header.num_new_syms {
            for s in n_in..total {
                flags[s as usize] = 1;
            }
        } else {
            return Err(Jbig2Error::OutOfRange(
                "symbol dictionary: export run exceeds total symbol count",
            ));
        }
    } else if idx < total {
        // Lenient fallback for the reference-encoder shortcut. Jbig2ENC.cpp
        // always emits `IAEX(0), IAEX(0)` regardless of the actual symbol
        // count, and Jbig2DEC.cpp just discards both values then keeps its
        // default ExportFlags table. Reproduce that behaviour by inferring
        // the export set from the header counts.
        let n_in = import_symbols.len() as u32;
        if exhausted_body && header.num_ex_syms == total {
            for s in 0..total {
                flags[s as usize] = 1;
            }
        } else if exhausted_body && header.num_ex_syms == header.num_new_syms {
            for s in n_in..total {
                flags[s as usize] = 1;
            }
        } else {
            return Err(Jbig2Error::OutOfRange(
                "symbol dictionary: export run did not cover all symbols",
            ));
        }
    }

    let n_in_usize = import_symbols.len();
    let n_in_u32 = import_symbols.len() as u32;
    let build_exported = |flags: &[u8]| {
        let mut exported = Vec::with_capacity(header.num_ex_syms as usize);
        for i in 0..total as usize {
            if flags[i] == 1 {
                if i < n_in_usize {
                    exported.push((*import_symbols[i]).clone());
                } else {
                    exported.push(new_symbols[i - n_in_usize].clone());
                }
            }
        }
        exported
    };

    let mut exported = build_exported(&flags);
    if exported.len() as u32 != header.num_ex_syms {
        // Same reference-encoder shortcut as above, but for the case where
        // the trailing zero bytes happen to parse as a formally valid B.1
        // stream that still does not match the header's export count.
        // Reconstruct the export set from the counts, matching the
        // corresponding arithmetic-path compatibility logic.
        flags.fill(0);
        if header.num_ex_syms == total {
            flags.fill(1);
        } else if header.num_ex_syms == header.num_new_syms {
            for s in n_in_u32..total {
                flags[s as usize] = 1;
            }
        } else {
            return Err(Jbig2Error::OutOfRange(
                "symbol dictionary: export count mismatch",
            ));
        }
        exported = build_exported(&flags);
    }

    Ok(DecodedSymbolDictionary {
        new_symbols,
        exported,
    })
}

/// Decode a Huffman-coded symbol dictionary body (spec 6.5 with
/// `SDHUFF = 1`, `SDREFAGG = 0`).
///
/// New symbols are emitted in height classes whose bitmaps are packed
/// horizontally into a single "collective" bitmap (spec 6.5.9), which is
/// either uncompressed or MMR-coded depending on the per-class BMSIZE.
/// User-defined Huffman tables are not yet supported; selectors that ask
/// for them return [`Jbig2Error::Unsupported`].
fn decode_symbol_dictionary_huffman(
    header: &SymbolDictionaryHeader,
    body: &[u8],
    import_symbols: &[&Bitmap],
) -> Jbig2Result<DecodedSymbolDictionary> {
    if header.sdrefagg {
        return Err(Jbig2Error::Unsupported(
            "symbol dictionary: Huffman + refagg path not yet implemented",
        ));
    }
    if header.sdhuff_dh > 1 || header.sdhuff_dw > 1 {
        return Err(Jbig2Error::Unsupported(
            "symbol dictionary: user-defined Huffman tables not yet implemented",
        ));
    }
    if header.sdhuff_bmsize || header.sdhuff_agginst {
        return Err(Jbig2Error::Unsupported(
            "symbol dictionary: user-defined SDHUFFBMSIZE/AGGINST not yet implemented",
        ));
    }

    // Spec 7.4.2.1.1: SDHUFFDH selects B.4 (0) or B.5 (1).
    let dh_table = standard_table(if header.sdhuff_dh == 0 { 4 } else { 5 })?;
    // SDHUFFDW selects B.2 (0) or B.3 (1); both encode OOB so the
    // height-class loop terminates.
    let dw_table = standard_table(if header.sdhuff_dw == 0 { 2 } else { 3 })?;
    // Standard Table B.1 covers BMSIZE and the export-flag run lengths.
    let b1_table = standard_table(1)?;

    let mut r = HuffReader::new(body);
    let mut new_symbols: Vec<Bitmap> = Vec::with_capacity(header.num_new_syms as usize);
    let mut hc_height: i32 = 0;
    while (new_symbols.len() as u32) < header.num_new_syms {
        // Step 4 b): height-class delta height (no OOB on B.4/B.5).
        let hcdh = dh_table.decode(&mut r)?.ok_or(Jbig2Error::InvalidHuffman(
            "symbol dictionary: Huffman IADH returned OOB",
        ))?;
        hc_height = hc_height.saturating_add(hcdh);
        if hc_height < 0 {
            return Err(Jbig2Error::OutOfRange(
                "symbol dictionary: negative height class height",
            ));
        }

        // Step 4 c): symbols in this height class, terminated by OOB DW.
        let mut widths: Vec<i32> = Vec::new();
        let mut sym_width: i32 = 0;
        loop {
            let dw = dw_table.decode(&mut r)?;
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
            widths.push(sym_width);
            if (new_symbols.len() + widths.len()) as u32 > header.num_new_syms {
                return Err(Jbig2Error::OutOfRange(
                    "symbol dictionary: more symbols than SDNUMNEWSYMS",
                ));
            }
        }
        if widths.is_empty() {
            // Empty height class — spec allows it; just continue.
            continue;
        }

        // Step 4 d): collective bitmap (spec 6.5.9).
        let bmsize = b1_table.decode(&mut r)?.ok_or(Jbig2Error::InvalidHuffman(
            "symbol dictionary: BMSIZE decoded as OOB",
        ))? as i64;
        r.byte_align();

        let total_width: u32 = widths.iter().sum::<i32>() as u32;
        let bytes_per_row = ((total_width + 7) / 8) as usize;
        if bmsize == 0 {
            // Uncompressed: HCHEIGHT × ⌈TOTWIDTH/8⌉ bytes packed MSB-first
            // with 0..7 padding bits per row. We decode into a single
            // collective bitmap and then slice it horizontally into the
            // per-symbol bitmaps.
            let total_bytes = bytes_per_row.saturating_mul(hc_height as usize);
            let pos = r.byte_pos();
            if pos + total_bytes > body.len() {
                return Err(Jbig2Error::UnexpectedEof {
                    needed: pos + total_bytes - body.len(),
                });
            }
            let mut bhc = Bitmap::new(total_width, hc_height as u32)?;
            for y in 0..hc_height as usize {
                let row = bhc.row_mut(y);
                let src = &body[pos + y * bytes_per_row..pos + (y + 1) * bytes_per_row];
                let n = row.len().min(src.len());
                row[..n].copy_from_slice(&src[..n]);
            }
            r.skip_bytes(total_bytes);
            let mut x_off: i32 = 0;
            for &w in &widths {
                let mut sym = Bitmap::new(w as u32, hc_height as u32)?;
                sym.copy_from(&bhc, x_off, 0, w as u32, hc_height as u32, 0, 0);
                new_symbols.push(sym);
                x_off += w;
            }
        } else {
            #[cfg(not(feature = "mmr"))]
            {
                let _ = bytes_per_row;
                return Err(Jbig2Error::Unsupported(
                    "symbol dictionary: Huffman + MMR collective bitmap requires the `mmr` feature",
                ));
            }
            #[cfg(feature = "mmr")]
            {
                // The reference T.88 sample encoder (Jbig2ENC.cpp) does
                // NOT produce a single MMR-coded collective bitmap as
                // section 6.5.9 might suggest. Instead, when
                // `BMSIZE > 0`, it emits one MMR T6 stream **per
                // symbol** of size `Width[k] x HCHEIGHT`, byte-aligned
                // between symbols and without EOFB. The reference
                // decoder mirrors this layout (T4T6DecMain is invoked
                // once per symbol). Conformance vectors TT1/TT2 are
                // produced this way, so we follow the same convention
                // — `BMSIZE` is treated as a flag (non-zero =
                // per-symbol MMR, zero = uncompressed collective
                // bitmap) and its absolute value is ignored, matching
                // the reference implementations.
                let _ = (bytes_per_row, bmsize);
                let pos = r.byte_pos();
                let mut buf = crate::coding::mmr_lut::MmrBitBuf::new(&body[pos..]);
                for &w in &widths {
                    let sym = crate::segments::generic_region::decode_generic_mmr_streaming(
                        &mut buf,
                        w as u32,
                        hc_height as u32,
                    )?;
                    new_symbols.push(sym);
                    // Byte-align between back-to-back MMR streams to
                    // mirror the encoder's `ByteStuffOutJXR` step
                    // following each `T4T6Encmain` call.
                    let bp = buf.bit_pos();
                    let aligned = (bp + 7) & !7;
                    let pad = aligned - bp;
                    if pad > 0 {
                        buf.consume_padding(pad as u8)?;
                    }
                }
                let consumed = buf.byte_pos();
                r.skip_bytes(consumed);
            }
        }
    }

    // Step 5 / 6.5.10: export-flag run-length stream using B.1.
    //
    // The reference T.88 sample encoder (Jbig2ENC.cpp) does NOT write
    // a Huffman-coded export-flag stream in the SDHUFF=1 path; it just
    // appends two trailing 0x00 bytes after the last height class and
    // the matching decoder simply discards two bytes (Jbig2DEC.cpp,
    // `Ref_1Byte; Ref_1Byte`). All conformance vectors that exercise
    // SDHUFF=1 (TT1, TT2) follow this convention. We therefore try a
    // best-effort B.1 decode first, and fall back to "all new symbols
    // exported" if the run-length stream cannot be parsed — matching
    // the arithmetic-path behaviour above.
    let total = import_symbols.len() as u32 + header.num_new_syms;
    let mut flags = vec![0u8; total as usize];
    let n_in = import_symbols.len() as u32;
    let recovered = (|| -> Jbig2Result<bool> {
        let mut idx: u32 = 0;
        let mut cur: u8 = 0;
        while idx < total {
            let run = b1_table.decode(&mut r)?.ok_or(Jbig2Error::InvalidHuffman(
                "symbol dictionary: Huffman export run decoded as OOB",
            ))? as u32;
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
        Ok(true)
    })();
    if recovered.is_err() {
        if header.num_ex_syms == total {
            for s in 0..total {
                flags[s as usize] = 1;
            }
        } else if header.num_ex_syms == header.num_new_syms {
            for s in n_in..total {
                flags[s as usize] = 1;
            }
        } else {
            return Err(recovered.unwrap_err());
        }
    }

    let n_in_usize = import_symbols.len();
    let build_exported = |flags: &[u8]| {
        let mut exported = Vec::with_capacity(header.num_ex_syms as usize);
        for i in 0..total as usize {
            if flags[i] == 1 {
                if i < n_in_usize {
                    exported.push((*import_symbols[i]).clone());
                } else {
                    exported.push(new_symbols[i - n_in_usize].clone());
                }
            }
        }
        exported
    };

    let mut exported = build_exported(&flags);
    if exported.len() as u32 != header.num_ex_syms {
        // The trailing zero bytes emitted by the reference encoder can also
        // decode into a formally valid, but semantically wrong, B.1 run
        // stream. If that happens, reconstruct the export set directly from
        // the header counts, matching the arithmetic-path compatibility
        // behaviour above.
        flags.fill(0);
        if header.num_ex_syms == total {
            flags.fill(1);
        } else if header.num_ex_syms == header.num_new_syms {
            for s in n_in..total {
                flags[s as usize] = 1;
            }
        } else {
            return Err(Jbig2Error::OutOfRange(
                "symbol dictionary: Huffman export count mismatch",
            ));
        }
        exported = build_exported(&flags);
    }

    Ok(DecodedSymbolDictionary {
        new_symbols,
        exported,
    })
}

/// Decode a single refinement/aggregate symbol (spec 6.5.8.2).
///
/// Shares the SD's MQ decoder and context pool with the caller so the bit
/// stream stays aligned. For `REFAGGNINST = 1` we emit a plain refinement
/// region against the referenced symbol; for `REFAGGNINST > 1` we nest a
/// sub text region that composites multiple (optionally refined)
/// instances into the new symbol's canvas.
#[allow(clippy::too_many_arguments)]
fn decode_refagg_symbol(
    dec: &mut MqDecoder<'_>,
    cxs: &mut MqContexts,
    width: u32,
    height: u32,
    header: &SymbolDictionaryHeader,
    import_symbols: &[&Bitmap],
    new_so_far: &[Bitmap],
) -> Jbig2Result<Bitmap> {
    let refaggn = decode_integer(dec, cxs, IAAI).ok_or(Jbig2Error::InvalidHuffman(
        "symbol dictionary: IAAI (REFAGGNINST) returned OOB",
    ))?;
    if refaggn <= 0 {
        return Err(Jbig2Error::OutOfRange(
            "symbol dictionary: REFAGGNINST must be positive",
        ));
    }

    // Build the combined symbol table: imports followed by the
    // previously-decoded new symbols.  This is the SBSYMS / referable set
    // when decoding the aggregate body.
    let total_syms = import_symbols.len() + new_so_far.len();
    if total_syms == 0 {
        return Err(Jbig2Error::OutOfRange(
            "symbol dictionary: aggregate symbol with empty reference table",
        ));
    }

    // Spec 6.5.8.2.2: the IAID code length is fixed for the whole SD as
    // SBSYMCODELEN = ceil(log2(NUMINSYMS + NUMNEWSYMS)) — *not* the running
    // count of decoded symbols. The reference encoder
    // (`Jbig2ENC.cpp::SymbolAggregate_MQEnc`) emits IDs at this width.
    let sb_num_syms = import_symbols.len() as u32 + header.num_new_syms;
    let code_len = sym_code_len(sb_num_syms);

    if refaggn == 1 {
        // Single refinement body, spec 6.5.8.2.2.
        let id = decode_iaid(dec, cxs, IAID, code_len);
        if (id as usize) >= total_syms {
            return Err(Jbig2Error::OutOfRange(
                "symbol dictionary: aggregate ID exceeds symbol table",
            ));
        }
        let rdx = decode_integer(dec, cxs, IARDX).ok_or(Jbig2Error::InvalidHuffman(
            "symbol dictionary: IARDX returned OOB",
        ))?;
        let rdy = decode_integer(dec, cxs, IARDY).ok_or(Jbig2Error::InvalidHuffman(
            "symbol dictionary: IARDY returned OOB",
        ))?;
        let reference = symbol_by_index(import_symbols, new_so_far, id as usize);
        decode_refinement_region(
            dec,
            cxs,
            width,
            height,
            if header.sd_rtemplate { 1 } else { 0 },
            false,
            &header.rat,
            reference,
            rdx,
            rdy,
        )
    } else {
        // Aggregate: nested text region of size WIDTH × HCHEIGHT with
        // REFAGGNINST instances composited onto a zero background.
        // Spec 6.5.8.2 fixes SBSTRIPS=1, SBCOMBOP=OR, TL corner, TPGRON
        // disabled, no dsoffset.
        let sbsyms: Vec<&Bitmap> = import_symbols
            .iter()
            .copied()
            .chain(new_so_far.iter())
            .collect();
        let hdr = TextRegionHeader {
            region: RegionInfo {
                width,
                height,
                x: 0,
                y: 0,
                external_combination_op: CombinationOp::Or,
                colour_extension: false,
            },
            sbhuff: false,
            sbrefine: true,
            log_sbstrips: 0,
            ref_corner: RefCorner::TL,
            transposed: false,
            sbcombop: CombinationOp::Or,
            default_pixel: 0,
            sbds_offset: 0,
            sbr_template: header.sd_rtemplate,
            sbhuff_fs: 0,
            sbhuff_ds: 0,
            sbhuff_dt: 0,
            sbhuff_rdw: 0,
            sbhuff_rdh: 0,
            sbhuff_rdx: 0,
            sbhuff_rdy: 0,
            sbhuff_rsize: false,
            rat: header.rat,
            num_instances: refaggn as u32,
        };
        decode_text_region_body_with_code_len(dec, cxs, &hdr, &sbsyms, code_len)
    }
}

#[inline]
fn symbol_by_index<'a>(
    imports: &'a [&'a Bitmap],
    new_so_far: &'a [Bitmap],
    idx: usize,
) -> &'a Bitmap {
    if idx < imports.len() {
        imports[idx]
    } else {
        &new_so_far[idx - imports.len()]
    }
}

/// One refinement-coded new symbol in an `SDREFAGG = 1` dictionary.
///
/// Each entry is rendered against `reference_id` (an index into the
/// concatenation of imports + previously-decoded new symbols) using the
/// refinement coder. `target` must equal the rendered symbol bitmap.
#[derive(Clone, Debug)]
pub struct NewSymbol {
    /// The fully-rendered symbol bitmap.
    pub target: Bitmap,
    /// Index into imports ++ previously-decoded new symbols.
    pub reference_id: u32,
    /// `RDX`: refinement-region reference offset on the X axis.
    pub rdx: i32,
    /// `RDY`: refinement-region reference offset on the Y axis.
    pub rdy: i32,
}

/// Encode a refinement/aggregate (`SDREFAGG = 1`) symbol dictionary body.
///
/// Mirrors [`encode_symbol_dictionary`] but emits each new symbol as a
/// single-instance refinement (`REFAGGNINST = 1`) of a previously-known
/// symbol drawn from `imports ++ previous-new-symbols`. The first symbol
/// therefore requires at least one `imports` entry to refer to.
pub fn encode_symbol_dictionary_refagg(
    header: &SymbolDictionaryHeader,
    symbols: &[NewSymbol],
    imports: &[&Bitmap],
) -> Jbig2Result<Vec<u8>> {
    if header.sdhuff {
        return Err(Jbig2Error::Unsupported(
            "symbol dictionary encoder: Huffman coding not yet implemented",
        ));
    }
    if !header.sdrefagg {
        return Err(Jbig2Error::InvalidConfig(
            "symbol dictionary refagg encoder: header must have sdrefagg = true",
        ));
    }
    if symbols.len() as u32 != header.num_new_syms {
        return Err(Jbig2Error::InvalidConfig(
            "symbol dictionary: symbol count != num_new_syms",
        ));
    }

    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    let mut enc = MqEncoder::new(symbols.iter().map(|s| s.target.data().len()).sum::<usize>() + 64);

    let mut i = 0;
    let mut prev_height: i32 = 0;
    while i < symbols.len() {
        let h = symbols[i].target.height() as i32;
        encode_integer(&mut enc, &mut cxs, IADH, h - prev_height)?;
        prev_height = h;

        let mut prev_width: i32 = 0;
        while i < symbols.len() && symbols[i].target.height() as i32 == h {
            let s = &symbols[i];
            let w = s.target.width() as i32;
            encode_integer(&mut enc, &mut cxs, IADW, w - prev_width)?;
            prev_width = w;
            encode_integer(&mut enc, &mut cxs, IAAI, 1)?;
            let total_syms = imports.len() + i;
            if total_syms == 0 {
                return Err(Jbig2Error::InvalidConfig(
                    "symbol dictionary refagg: first symbol needs an import to refine against",
                ));
            }
            if (s.reference_id as usize) >= total_syms {
                return Err(Jbig2Error::InvalidConfig(
                    "symbol dictionary: reference_id exceeds known symbol table",
                ));
            }
            // Spec 6.5.8.2.2: SBSYMCODELEN is fixed for the SD using the
            // total possible referable count (NUMINSYMS + NUMNEWSYMS).
            let sb_num_syms = imports.len() as u32 + header.num_new_syms;
            let code_len = sym_code_len(sb_num_syms);
            let reference: &Bitmap = if (s.reference_id as usize) < imports.len() {
                imports[s.reference_id as usize]
            } else {
                &symbols[s.reference_id as usize - imports.len()].target
            };
            encode_iaid(&mut enc, &mut cxs, IAID, code_len, s.reference_id);
            encode_integer(&mut enc, &mut cxs, IARDX, s.rdx)?;
            encode_integer(&mut enc, &mut cxs, IARDY, s.rdy)?;
            encode_refinement_region(
                &mut enc,
                &mut cxs,
                &s.target,
                if header.sd_rtemplate { 1 } else { 0 },
                false,
                &header.rat,
                reference,
                s.rdx,
                s.rdy,
            )?;
            i += 1;
        }
        // The refinement/aggregate path has a fixed symbol count from
        // SDNUMNEWSYMS/IAAI, and the decoder follows the reference encoder by
        // stopping without consuming a height-class OOB terminator.
        if !header.sdrefagg {
            encode_integer(&mut enc, &mut cxs, IADW, OOB)?;
        }
    }

    // Emit the export flags exactly as 6.5.10 specifies
    encode_integer(&mut enc, &mut cxs, IAEX, imports.len() as i32)?;
    encode_integer(&mut enc, &mut cxs, IAEX, header.num_new_syms as i32)?;

    Ok(enc.finish())
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
    let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
    encode_symbol_dictionary_with_contexts(header, symbols, import_count, &mut cxs)
}

/// Build a symbol dictionary body using a caller-owned MQ context pool.
pub fn encode_symbol_dictionary_with_contexts(
    header: &SymbolDictionaryHeader,
    symbols: &[Bitmap],
    import_count: u32,
    cxs: &mut MqContexts,
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
    let at = AtPixels::new(header.at.as_array(), header.at.len() as u8);

    let mut enc = MqEncoder::new(symbols.iter().map(|s| s.data().len()).sum::<usize>() + 64);
    cxs.reset();

    let mut i = 0;
    let mut prev_height: i32 = 0;
    while i < symbols.len() {
        let h = symbols[i].height() as i32;
        encode_integer(&mut enc, cxs, IADH, h - prev_height)?;
        prev_height = h;

        // Emit every symbol with the same height.
        let mut prev_width: i32 = 0;
        while i < symbols.len() && symbols[i].height() as i32 == h {
            let w = symbols[i].width() as i32;
            encode_integer(&mut enc, cxs, IADW, w - prev_width)?;
            prev_width = w;
            encode_generic_bitmap(
                &mut enc,
                cxs,
                &symbols[i],
                header.sd_template,
                false,
                false,
                &at,
            )?;
            i += 1;
        }
        // Terminate the height class with OOB.
        encode_integer(&mut enc, cxs, IADW, OOB)?;
    }

    // Emit the standards-compliant export-flag run stream so external
    // decoders do not depend on the T.88 sample encoder's shortcut.
    encode_integer(&mut enc, cxs, IAEX, import_count as i32)?;
    encode_integer(&mut enc, cxs, IAEX, header.num_new_syms as i32)?;

    Ok(enc.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mkglyph(w: u32, h: u32, pattern: u32) -> Bitmap {
        let mut bm = Bitmap::new(w, h).unwrap();
        for y in 0..h {
            for x in 0..w {
                if ((x
                    .wrapping_mul(1103515245)
                    .wrapping_add(y.wrapping_mul(12345).wrapping_add(pattern)))
                    & 0x8)
                    != 0
                {
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
        let hdr =
            SymbolDictionaryHeader::default_arithmetic(0, syms.len() as u32, syms.len() as u32);
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
        let hdr =
            SymbolDictionaryHeader::default_arithmetic(0, syms.len() as u32, syms.len() as u32);
        let body = encode_symbol_dictionary(&hdr, &syms, 0).unwrap();
        let dec = decode_symbol_dictionary(&hdr, &body, &[]).unwrap();
        for (i, s) in syms.iter().enumerate() {
            assert_eq!(&dec.new_symbols[i], s);
        }
    }

    #[test]
    fn round_trip_template3() {
        let syms = vec![mkglyph(7, 9, 11), mkglyph(11, 9, 22)];
        let hdr =
            SymbolDictionaryHeader::default_arithmetic(3, syms.len() as u32, syms.len() as u32);
        let body = encode_symbol_dictionary(&hdr, &syms, 0).unwrap();
        let dec = decode_symbol_dictionary(&hdr, &body, &[]).unwrap();
        assert_eq!(dec.new_symbols, syms);
    }

    fn refagg_header(num_new: u32) -> SymbolDictionaryHeader {
        let mut h = SymbolDictionaryHeader::default_arithmetic(0, num_new, num_new);
        h.sdrefagg = true;
        h.sd_rtemplate = false;
        h.rat = [(-1, -1), (-1, -1)];
        h
    }

    #[test]
    fn round_trip_refagg_refined_against_import() {
        let import = mkglyph(9, 11, 5);
        let mut variant = import.clone();
        variant.set_pixel(0, 0, variant.get_pixel(0, 0) ^ 1);
        let imports_owned = vec![import.clone()];
        let imports: Vec<&Bitmap> = imports_owned.iter().collect();
        let syms = vec![NewSymbol {
            target: variant.clone(),
            reference_id: 0,
            rdx: 0,
            rdy: 0,
        }];
        let hdr = refagg_header(syms.len() as u32);
        let body = encode_symbol_dictionary_refagg(&hdr, &syms, &imports).unwrap();
        let dec = decode_symbol_dictionary(&hdr, &body, &imports).unwrap();
        assert_eq!(dec.new_symbols[0], variant);
    }

    #[test]
    fn round_trip_refagg_refined_against_previous() {
        let import = mkglyph(10, 12, 5);
        let mut variant = import.clone();
        variant.set_pixel(2, 3, variant.get_pixel(2, 3) ^ 1);
        variant.set_pixel(7, 9, variant.get_pixel(7, 9) ^ 1);
        let imports_owned = vec![import.clone()];
        let imports: Vec<&Bitmap> = imports_owned.iter().collect();
        // First new symbol refines against the import; second refines
        // against the first new symbol (reference_id = 1 = imports.len() + 0).
        let syms = vec![
            NewSymbol {
                target: import.clone(),
                reference_id: 0,
                rdx: 0,
                rdy: 0,
            },
            NewSymbol {
                target: variant.clone(),
                reference_id: 1,
                rdx: 0,
                rdy: 0,
            },
        ];
        let hdr = refagg_header(syms.len() as u32);
        let body = encode_symbol_dictionary_refagg(&hdr, &syms, &imports).unwrap();
        let dec = decode_symbol_dictionary(&hdr, &body, &imports).unwrap();
        assert_eq!(dec.new_symbols[0], import);
        assert_eq!(dec.new_symbols[1], variant);
    }
}
