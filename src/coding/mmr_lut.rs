//! LUT-driven, packed-row T.6 (MMR) decode fast path.
//!
//! The decoder in [`crate::coding::mmr`] is correctness-first: it matches
//! run-length codes one bit at a time, represents each line as a
//! `Vec<bool>`, and converts between booleans and packed bytes at the
//! segment boundary. That shape is easy to validate against the T.6
//! reference, but it is not competitive for a decoder whose stated goal is
//! to be very fast.
//!
//! This module adds the fast path that the original plan called for:
//!
//! * A 13-bit lookup table for white/black run codes. Every decode pulls up
//!   to 13 bits of the stream into a register, indexes the table once, and
//!   consumes the resolved code length. Make-up codes fall through to a
//!   second terminator lookup.
//! * A small control-code LUT (up to 7 bits) that resolves Pass / Vx /
//!   Horizontal in a single table hit.
//! * Direct operation on packed JBIG2 rows, so `b1` / `b2` searches and
//!   run fills scan bytes instead of iterating boolean slices.
//!
//! The slow `Vec<bool>` implementation is retained verbatim in
//! [`crate::coding::mmr`] as a reference baseline and is exercised by the
//! existing round-trip tests and by a cross-validation test in this module.

use std::sync::OnceLock;

use crate::error::{Jbig2Error, Jbig2Result};

/// Run-length LUT entry. `len == 0` means "unresolved" (i.e. the 13-bit
/// prefix does not decode to any known code; the decoder should report an
/// invalid-Huffman error).
#[derive(Copy, Clone, Debug)]
struct RunLut {
    /// Total number of bits consumed by this code (1..=13, or 0 if invalid).
    len: u8,
    /// 0 = invalid, 1 = terminator, 2 = make-up.
    kind: u8,
    /// Run length in pixels. For terminators this is 0..=63, for make-ups
    /// a multiple of 64 up to 2560.
    run: u16,
}

const INVALID_RUN: RunLut = RunLut {
    len: 0,
    kind: 0,
    run: 0,
};

/// Control-code LUT entry for T.6 two-dimensional mode.
#[derive(Copy, Clone, Debug)]
struct CtrlLut {
    /// Total number of bits consumed (1..=7, or 0 if invalid).
    len: u8,
    /// 0 = invalid, 1 = Pass, 2 = Horizontal, 3 = V(delta).
    kind: u8,
    /// Delta for V codes (-3..=3), ignored otherwise.
    delta: i8,
}

const INVALID_CTRL: CtrlLut = CtrlLut {
    len: 0,
    kind: 0,
    delta: 0,
};

static WHITE_LUT: OnceLock<Box<[RunLut; 1 << 13]>> = OnceLock::new();
static BLACK_LUT: OnceLock<Box<[RunLut; 1 << 13]>> = OnceLock::new();
static CTRL_LUT: OnceLock<Box<[CtrlLut; 1 << 7]>> = OnceLock::new();

fn white_lut() -> &'static [RunLut; 1 << 13] {
    WHITE_LUT.get_or_init(|| build_run_lut(true))
}

fn black_lut() -> &'static [RunLut; 1 << 13] {
    BLACK_LUT.get_or_init(|| build_run_lut(false))
}

fn ctrl_lut() -> &'static [CtrlLut; 1 << 7] {
    CTRL_LUT.get_or_init(build_ctrl_lut)
}

// Run-length code tables (mirrors of the ones in [`crate::coding::mmr`]).
// They are kept local so the fast path can be compiled independently of the
// reference decoder if that ever becomes useful.
static WHITE_TERM: &[(u32, u8)] = &[
    (0x35, 8),
    (0x07, 6),
    (0x07, 4),
    (0x08, 4),
    (0x0B, 4),
    (0x0C, 4),
    (0x0E, 4),
    (0x0F, 4),
    (0x13, 5),
    (0x14, 5),
    (0x07, 5),
    (0x08, 5),
    (0x08, 6),
    (0x03, 6),
    (0x34, 6),
    (0x35, 6),
    (0x2A, 6),
    (0x2B, 6),
    (0x27, 7),
    (0x0C, 7),
    (0x08, 7),
    (0x17, 7),
    (0x03, 7),
    (0x04, 7),
    (0x28, 7),
    (0x2B, 7),
    (0x13, 7),
    (0x24, 7),
    (0x18, 7),
    (0x02, 8),
    (0x03, 8),
    (0x1A, 8),
    (0x1B, 8),
    (0x12, 8),
    (0x13, 8),
    (0x14, 8),
    (0x15, 8),
    (0x16, 8),
    (0x17, 8),
    (0x28, 8),
    (0x29, 8),
    (0x2A, 8),
    (0x2B, 8),
    (0x2C, 8),
    (0x2D, 8),
    (0x04, 8),
    (0x05, 8),
    (0x0A, 8),
    (0x0B, 8),
    (0x52, 8),
    (0x53, 8),
    (0x54, 8),
    (0x55, 8),
    (0x24, 8),
    (0x25, 8),
    (0x58, 8),
    (0x59, 8),
    (0x5A, 8),
    (0x5B, 8),
    (0x4A, 8),
    (0x4B, 8),
    (0x32, 8),
    (0x33, 8),
    (0x34, 8),
];

static WHITE_MAKEUP: &[(u32, u8)] = &[
    (0x1B, 5),
    (0x12, 5),
    (0x17, 6),
    (0x37, 7),
    (0x36, 8),
    (0x37, 8),
    (0x64, 8),
    (0x65, 8),
    (0x68, 8),
    (0x67, 8),
    (0xCC, 9),
    (0xCD, 9),
    (0xD2, 9),
    (0xD3, 9),
    (0xD4, 9),
    (0xD5, 9),
    (0xD6, 9),
    (0xD7, 9),
    (0xD8, 9),
    (0xD9, 9),
    (0xDA, 9),
    (0xDB, 9),
    (0x98, 9),
    (0x99, 9),
    (0x9A, 9),
    (0x18, 6),
    (0x9B, 9),
    (0x08, 11),
    (0x0C, 11),
    (0x0D, 11),
    (0x12, 12),
    (0x13, 12),
    (0x14, 12),
    (0x15, 12),
    (0x16, 12),
    (0x17, 12),
    (0x1C, 12),
    (0x1D, 12),
    (0x1E, 12),
    (0x1F, 12),
];

static BLACK_TERM: &[(u32, u8)] = &[
    (0x37, 10),
    (0x02, 3),
    (0x03, 2),
    (0x02, 2),
    (0x03, 3),
    (0x03, 4),
    (0x02, 4),
    (0x03, 5),
    (0x05, 6),
    (0x04, 6),
    (0x04, 7),
    (0x05, 7),
    (0x07, 7),
    (0x04, 8),
    (0x07, 8),
    (0x18, 9),
    (0x17, 10),
    (0x18, 10),
    (0x08, 10),
    (0x67, 11),
    (0x68, 11),
    (0x6C, 11),
    (0x37, 11),
    (0x28, 11),
    (0x17, 11),
    (0x18, 11),
    (0xCA, 12),
    (0xCB, 12),
    (0xCC, 12),
    (0xCD, 12),
    (0x68, 12),
    (0x69, 12),
    (0x6A, 12),
    (0x6B, 12),
    (0xD2, 12),
    (0xD3, 12),
    (0xD4, 12),
    (0xD5, 12),
    (0xD6, 12),
    (0xD7, 12),
    (0x6C, 12),
    (0x6D, 12),
    (0xDA, 12),
    (0xDB, 12),
    (0x54, 12),
    (0x55, 12),
    (0x56, 12),
    (0x57, 12),
    (0x64, 12),
    (0x65, 12),
    (0x52, 12),
    (0x53, 12),
    (0x24, 12),
    (0x37, 12),
    (0x38, 12),
    (0x27, 12),
    (0x28, 12),
    (0x58, 12),
    (0x59, 12),
    (0x2B, 12),
    (0x2C, 12),
    (0x5A, 12),
    (0x66, 12),
    (0x67, 12),
];

static BLACK_MAKEUP: &[(u32, u8)] = &[
    (0x0F, 10),
    (0xC8, 12),
    (0xC9, 12),
    (0x5B, 12),
    (0x33, 12),
    (0x34, 12),
    (0x35, 12),
    (0x6C, 13),
    (0x6D, 13),
    (0x4A, 13),
    (0x4B, 13),
    (0x4C, 13),
    (0x4D, 13),
    (0x72, 13),
    (0x73, 13),
    (0x74, 13),
    (0x75, 13),
    (0x76, 13),
    (0x77, 13),
    (0x52, 13),
    (0x53, 13),
    (0x54, 13),
    (0x55, 13),
    (0x5A, 13),
    (0x5B, 13),
    (0x64, 13),
    (0x65, 13),
    (0x08, 11),
    (0x0C, 11),
    (0x0D, 11),
    (0x12, 12),
    (0x13, 12),
    (0x14, 12),
    (0x15, 12),
    (0x16, 12),
    (0x17, 12),
    (0x1C, 12),
    (0x1D, 12),
    (0x1E, 12),
    (0x1F, 12),
];

fn build_run_lut(white: bool) -> Box<[RunLut; 1 << 13]> {
    let mut table = Box::new([INVALID_RUN; 1 << 13]);
    let (term, makeup): (&[(u32, u8)], &[(u32, u8)]) = if white {
        (WHITE_TERM, WHITE_MAKEUP)
    } else {
        (BLACK_TERM, BLACK_MAKEUP)
    };
    // Populate terminator codes (run lengths 0..=63).
    for (idx, &(code, len)) in term.iter().enumerate() {
        fill_run_entries(
            &mut table,
            code,
            len,
            RunLut {
                len,
                kind: 1,
                run: idx as u16,
            },
        );
    }
    // Populate make-up codes. Indices 0..=26 cover runs 64..=1728 in
    // 64-pixel steps; indices 27..=39 cover 1792, 1856, ..., stepping by
    // 64 pixels up to 2560.
    for (idx, &(code, len)) in makeup.iter().enumerate() {
        let run = if idx < 27 {
            64 * (idx as u16 + 1)
        } else {
            1792 + 64 * (idx as u16 - 27)
        };
        fill_run_entries(&mut table, code, len, RunLut { len, kind: 2, run });
    }
    table
}

/// Populate every 13-bit index whose top `len` bits equal `code` with
/// `entry`. This is the canonical way to build a prefix-code lookup table.
fn fill_run_entries(table: &mut [RunLut; 1 << 13], code: u32, len: u8, entry: RunLut) {
    let shift = 13 - len as u32;
    let base = (code << shift) as usize;
    let span = 1usize << shift;
    for slot in table.iter_mut().skip(base).take(span) {
        *slot = entry;
    }
}

fn build_ctrl_lut() -> Box<[CtrlLut; 1 << 7]> {
    let mut table = Box::new([INVALID_CTRL; 1 << 7]);
    // (code, len, kind, delta)
    let entries: &[(u32, u8, u8, i8)] = &[
        // V0: 1
        (0b1, 1, 3, 0),
        // VR1 / VL1: 011, 010
        (0b011, 3, 3, 1),
        (0b010, 3, 3, -1),
        // H: 001
        (0b001, 3, 2, 0),
        // Pass: 0001
        (0b0001, 4, 1, 0),
        // VR2 / VL2: 000011, 000010
        (0b000011, 6, 3, 2),
        (0b000010, 6, 3, -2),
        // VR3 / VL3: 0000011, 0000010
        (0b0000011, 7, 3, 3),
        (0b0000010, 7, 3, -3),
    ];
    for &(code, len, kind, delta) in entries {
        let shift = 7 - len as u32;
        let base = (code << shift) as usize;
        let span = 1usize << shift;
        for slot in table.iter_mut().skip(base).take(span) {
            *slot = CtrlLut { len, kind, delta };
        }
    }
    table
}

/// Streaming MSB-first bit reader with a 64-bit cache. Faster than the
/// bit-at-a-time reader in [`crate::coding::mmr`] because the hot path
/// only touches byte-level state once every ~56 bits.
pub struct MmrBitBuf<'a> {
    buf: &'a [u8],
    pos: usize,
    /// Bits are held in the high end of the cache (MSB-first) so peek
    /// simply shifts the top `n` bits down.
    cache: u64,
    cache_bits: u32,
}

impl<'a> MmrBitBuf<'a> {
    /// Create a new reader positioned at the start of `buf`.
    pub fn new(buf: &'a [u8]) -> Self {
        let mut s = Self {
            buf,
            pos: 0,
            cache: 0,
            cache_bits: 0,
        };
        s.fill();
        s
    }

    /// Top up the cache with whole bytes until at least 32 bits are buffered
    /// or the input is exhausted.
    #[inline]
    fn fill(&mut self) {
        while self.cache_bits <= 56 && self.pos < self.buf.len() {
            self.cache |= (self.buf[self.pos] as u64) << (56 - self.cache_bits);
            self.cache_bits += 8;
            self.pos += 1;
        }
    }

    /// Peek the top 13 bits. If fewer than 13 bits are buffered, the
    /// missing low bits are zero-extended, which is safe for LUT lookup:
    /// invalid indices map to the `INVALID_RUN` sentinel.
    #[inline]
    fn peek13(&self) -> u32 {
        (self.cache >> (64 - 13)) as u32
    }

    /// Peek the top 7 bits, zero-extended when the cache is near-empty.
    #[inline]
    fn peek7(&self) -> u32 {
        (self.cache >> (64 - 7)) as u32
    }

    /// Peek the top `n` bits (1..=32), zero-extended when the cache is
    /// near-empty.
    #[inline]
    pub fn peek_bits(&self, n: u8) -> u32 {
        debug_assert!((1..=32).contains(&n));
        (self.cache >> (64 - n as u32)) as u32
    }

    /// Drop `n` bits from the top of the cache. Must be <= 13.
    #[inline]
    fn consume(&mut self, n: u8) -> Jbig2Result<()> {
        let n_u32 = n as u32;
        if n_u32 > self.cache_bits {
            return Err(Jbig2Error::UnexpectedEof { needed: n as usize });
        }
        self.cache <<= n_u32;
        self.cache_bits -= n_u32;
        self.fill();
        Ok(())
    }

    /// Byte position suitable for diagnostics; approximates the position
    /// that a slow [`super::mmr::BitReader`] would report after consuming
    /// the same prefix.
    /// Approximate byte position after consuming the bits returned so
    /// far. Useful when the caller needs to skip past padding bits to
    /// the next byte boundary (e.g. between back-to-back MMR images
    /// inside a Huffman-coded symbol dictionary).
    pub fn byte_pos(&self) -> usize {
        self.pos - (self.cache_bits as usize / 8)
    }

    /// Total bits already consumed (whole bytes pulled into the cache
    /// minus bits still buffered).
    pub fn bit_pos(&self) -> usize {
        self.pos * 8 - self.cache_bits as usize
    }

    /// Drop `n` padding bits without performing any decode. Used when
    /// the caller knows it must skip 0..7 bits to byte-align between
    /// concatenated MMR streams (e.g. the back-to-back per-symbol
    /// bitmaps of a Huffman SD height class).
    pub fn consume_padding(&mut self, n: u8) -> Jbig2Result<()> {
        debug_assert!(n < 8, "consume_padding only handles sub-byte alignment");
        if n > 0 && self.peek_bits(n) != 0 {
            return Err(Jbig2Error::OutOfRange("MMR padding bits non-zero"));
        }
        self.consume(n)
    }

    /// Skip an arbitrary number of bits. Used by callers that need to
    /// consume protocol-level markers such as the 24-bit T.6 EOFB found
    /// after each MMR-coded halftone bitplane in the reference streams.
    pub fn skip_bits(&mut self, n: u32) -> Jbig2Result<()> {
        let mut left = n;
        while left > 0 {
            let step = left.min(13) as u8;
            self.consume(step)?;
            left -= step as u32;
        }
        Ok(())
    }
}

/// Decode a single MMR run, pulling bits via the fast LUT path. Handles
/// stacked make-up codes internally.
#[inline]
fn decode_run_fast(r: &mut MmrBitBuf<'_>, white: bool) -> Jbig2Result<u32> {
    let table = if white { white_lut() } else { black_lut() };
    let mut run: u32 = 0;
    loop {
        let idx = r.peek13() as usize;
        let entry = table[idx];
        if entry.kind == 0 {
            return Err(Jbig2Error::InvalidHuffman("mmr/mh run code"));
        }
        r.consume(entry.len)?;
        run += entry.run as u32;
        if entry.kind == 1 {
            return Ok(run);
        }
    }
}

/// Control-code result, mirroring the private `Ctrl` enum in
/// [`crate::coding::mmr`] but suitable for LUT dispatch.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum FastCtrl {
    Pass,
    V(i8),
    Horizontal,
}

#[inline]
fn decode_ctrl_fast(r: &mut MmrBitBuf<'_>) -> Jbig2Result<FastCtrl> {
    let entry = ctrl_lut()[r.peek7() as usize];
    if entry.kind == 0 {
        return Err(Jbig2Error::InvalidHuffman("mmr control code"));
    }
    r.consume(entry.len)?;
    Ok(match entry.kind {
        1 => FastCtrl::Pass,
        2 => FastCtrl::Horizontal,
        _ => FastCtrl::V(entry.delta),
    })
}

/// Return the pixel value at bit index `x` in a packed row. `true`
/// represents black (1), matching the `Vec<bool>` convention used by the
/// reference implementation.
#[inline]
fn color_at_packed(row: &[u8], x: isize, width: usize) -> bool {
    if x < 0 || (x as usize) >= width {
        return false;
    }
    let x = x as usize;
    (row[x >> 3] >> (7 - (x & 7))) & 1 != 0
}

/// Locate b1 on the reference row: the first *changing* picture element
/// to the right of `start` whose colour is opposite to `color`. A
/// changing element is a pixel whose colour differs from the pixel
/// immediately to its left; the virtual pixel preceding position 0 is
/// treated as white, matching the CCITT convention used at the start of
/// each line. Scans byte-wise so we examine at most one byte per 8
/// pixels for typical content.
#[inline]
fn next_changing_packed(row: &[u8], start: isize, color: bool, width: usize) -> usize {
    let i = if start < 0 { 0 } else { start as usize + 1 };
    if i >= width {
        return width;
    }
    let start_byte = i >> 3;
    let end_byte = (width + 7) >> 3;
    // `carry` holds the bit immediately to the left of the next byte's
    // MSB, placed in the LSB of this u8. Virtual pixel at -1 is white
    // (zero) for the first byte of the row.
    let mut carry: u8 = if start_byte == 0 {
        0
    } else {
        row[start_byte - 1] & 1
    };
    for b in start_byte..end_byte {
        let byte = row[b];
        // Each bit in `shifted` equals the pixel to the left of the
        // matching bit in `byte`. XORing them surfaces transitions as
        // 1-bits; ANDing with the bits of the opposite colour keeps only
        // transitions that land on the colour we care about.
        let shifted = (byte >> 1) | (carry << 7);
        let transitions = byte ^ shifted;
        let opposite_bits = if color { !byte } else { byte };
        let mut candidates = transitions & opposite_bits;
        if b == start_byte {
            let bit_off = i & 7;
            candidates &= 0xFFu8 >> bit_off;
        }
        if candidates != 0 {
            let bit_idx = (candidates as u32).leading_zeros() as usize - 24;
            let pos = (b << 3) + bit_idx;
            return pos.min(width);
        }
        carry = byte & 1;
    }
    width
}

/// Fill bits `[start, end)` of a packed row with `color`. Processes
/// whole bytes in the interior and handles head/tail fragments
/// individually; this is the packed counterpart of the slow-path
/// `fill(line, start, end, color)`.
#[inline]
fn fill_row_range(row: &mut [u8], start: usize, end: usize, color: bool) {
    if start >= end {
        return;
    }
    let end = end.min(row.len() * 8);
    if start >= end {
        return;
    }
    let byte_fill: u8 = if color { 0xFF } else { 0x00 };
    let sb = start >> 3;
    let eb = (end - 1) >> 3; // inclusive last byte
    if sb == eb {
        let head_mask = 0xFFu8 >> (start & 7);
        let tail_mask = 0xFFu8 << (7 - ((end - 1) & 7));
        let mask = head_mask & tail_mask;
        if color {
            row[sb] |= mask;
        } else {
            row[sb] &= !mask;
        }
        return;
    }
    // Head
    let head_mask = 0xFFu8 >> (start & 7);
    if color {
        row[sb] |= head_mask;
    } else {
        row[sb] &= !head_mask;
    }
    // Interior
    if sb + 1 < eb {
        for b in &mut row[sb + 1..eb] {
            *b = byte_fill;
        }
    }
    // Tail
    let tail_mask = 0xFFu8 << (7 - ((end - 1) & 7));
    if color {
        row[eb] |= tail_mask;
    } else {
        row[eb] &= !tail_mask;
    }
}

/// LUT-driven packed-row T.6 decoder.
///
/// `ref_row` and `cur_row` must already be sized to the row stride of the
/// bitmap. `cur_row` is cleared before decoding; callers that need the
/// previous contents should snapshot first. Pixels past `width` within the
/// final byte are left at zero.
pub fn decode_t6_line_packed(
    r: &mut MmrBitBuf<'_>,
    ref_row: &[u8],
    cur_row: &mut [u8],
    width: u32,
) -> Jbig2Result<()> {
    let width = width as usize;
    // Clear only the bits we actually produce; we leave padding bits in
    // the tail byte untouched because the caller guarantees a zeroed row.
    for b in cur_row.iter_mut() {
        *b = 0;
    }
    let mut a0: isize = -1;
    let mut a0_color = false;
    while (a0) < width as isize {
        match decode_ctrl_fast(r)? {
            FastCtrl::Pass => {
                let b1 = next_changing_packed(ref_row, a0, a0_color, width);
                let b2 = next_changing_packed(
                    ref_row,
                    b1 as isize,
                    color_at_packed(ref_row, b1 as isize, width),
                    width,
                );
                let start = if a0 < 0 { 0 } else { a0 as usize };
                fill_row_range(cur_row, start, b2, a0_color);
                a0 = b2 as isize;
            }
            FastCtrl::V(delta) => {
                let b1 = next_changing_packed(ref_row, a0, a0_color, width) as isize;
                let a1 = b1 + delta as isize;
                let start = if a0 < 0 { 0 } else { a0 as usize };
                let end = a1.max(0) as usize;
                fill_row_range(cur_row, start, end, a0_color);
                a0 = a1;
                a0_color = !a0_color;
            }
            FastCtrl::Horizontal => {
                let r1_color = a0_color;
                let r1 = decode_run_fast(r, !r1_color)?;
                let r2 = decode_run_fast(r, r1_color)?;
                let start = if a0 < 0 { 0 } else { a0 as usize };
                let a1 = start + r1 as usize;
                let a2 = a1 + r2 as usize;
                fill_row_range(cur_row, start, a1, a0_color);
                fill_row_range(cur_row, a1, a2, !a0_color);
                a0 = a2 as isize;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::mmr::{
        bools_to_row, decode_t6_line, encode_t6_line, row_to_bools, BitReader, BitWriter,
    };

    fn packed_from_bools(line: &[bool]) -> Vec<u8> {
        bools_to_row(line, (line.len() + 7) / 8)
    }

    /// Decode the same stream with both the slow and fast paths and assert
    /// they produce identical packed rows.
    fn cross_check_lines(lines: &[Vec<bool>]) {
        let width = lines[0].len();
        let stride = (width + 7) / 8;
        let mut w = BitWriter::new();
        let mut prev = vec![false; width];
        for line in lines {
            encode_t6_line(&mut w, line, &prev);
            prev = line.clone();
        }
        let bytes = w.finish();

        // Reference (slow) decode.
        let mut slow_rows: Vec<Vec<u8>> = Vec::with_capacity(lines.len());
        {
            let mut reader = BitReader::new(&bytes);
            let mut prev_bools = vec![false; width];
            for _ in 0..lines.len() {
                let got = decode_t6_line(&mut reader, &prev_bools, width as u32).unwrap();
                slow_rows.push(packed_from_bools(&got));
                prev_bools = got;
            }
        }

        // Fast (LUT + packed) decode.
        let mut fast_rows: Vec<Vec<u8>> = Vec::with_capacity(lines.len());
        {
            let mut buf = MmrBitBuf::new(&bytes);
            let mut prev_row = vec![0u8; stride];
            for _ in 0..lines.len() {
                let mut cur_row = vec![0u8; stride];
                decode_t6_line_packed(&mut buf, &prev_row, &mut cur_row, width as u32).unwrap();
                fast_rows.push(cur_row.clone());
                prev_row = cur_row;
            }
        }

        assert_eq!(slow_rows.len(), fast_rows.len());
        for (i, (s, f)) in slow_rows.iter().zip(fast_rows.iter()).enumerate() {
            assert_eq!(s, f, "mismatch on line {i}");
        }

        // Also confirm the round-trip decodes match the input lines.
        for (i, line) in lines.iter().enumerate() {
            let expected = row_to_bools(&slow_rows[i], width as u32);
            assert_eq!(
                &expected, line,
                "slow path disagrees with source on line {i}"
            );
        }
    }

    #[test]
    fn lut_matches_slow_single_line() {
        let line: Vec<bool> = (0..64).map(|i| (i % 5) == 0).collect();
        cross_check_lines(&[line]);
    }

    #[test]
    fn lut_matches_slow_multi_line() {
        let lines: Vec<Vec<bool>> = (0..16)
            .map(|y| (0..80).map(|x| ((x + y) % 7) == 0).collect())
            .collect();
        cross_check_lines(&lines);
    }

    #[test]
    fn lut_matches_slow_wide_runs() {
        let mut line = vec![false; 1728];
        for x in 200..800 {
            line[x] = true;
        }
        for x in 1000..1050 {
            line[x] = true;
        }
        cross_check_lines(&[line]);
    }

    /// Minimal repro for the TT9-class bug: a single line with a long
    /// white run that requires stacked make-up codes (runs ≥ 1792 pixels
    /// need the 1792+64·k extension), plus a short black run so the
    /// decoder also has to resync a BLACK code immediately afterwards.
    /// This used to desync because the slow reader bit-stacker and the
    /// fast LUT stacker disagreed on which bits were consumed when a
    /// make-up code wrapped into a terminator on the next byte.
    #[test]
    fn lut_matches_slow_stacked_white_makeup() {
        let width = 3456usize;
        let mut line = vec![false; width];
        for x in 2800..2810 {
            line[x] = true;
        }
        cross_check_lines(&[line]);
    }

    /// Same idea, but the long run is black. Exercises stacked BLACK
    /// make-ups and the WHITE → BLACK → WHITE colour toggle between
    /// terminator and make-up codes on a single row.
    #[test]
    fn lut_matches_slow_stacked_black_makeup() {
        let width = 3456usize;
        let mut line = vec![false; width];
        for x in 10..2900 {
            line[x] = true;
        }
        cross_check_lines(&[line]);
    }

    /// Cross-row variant: the previous row has a changing element far
    /// to the right of the current a0, so a V(0) emitted in the tail
    /// of the row has to resolve b1/b2 beyond the last column. This is
    /// the pattern that caused the original TT9 line-66 desync before
    /// `next_changing_ref` was fixed, so it lives here as a cheap,
    /// stable regression separate from the full-page test.
    #[test]
    fn lut_matches_slow_trailing_reference_transition() {
        let width = 256usize;
        let mut prev = vec![false; width];
        for x in 200..220 {
            prev[x] = true;
        }
        let mut cur = vec![false; width];
        for x in 50..60 {
            cur[x] = true;
        }
        cross_check_lines(&[prev, cur]);
    }

    #[test]
    fn lut_matches_slow_conformance_prefix() {
        // Structure mirroring the top of F01_200_TT9.jb2: 27 all-white
        // lines, then a short black run. This exercises a long chain of
        // V(0) codes plus Horizontal mode.
        let width = 1728usize;
        let mut lines: Vec<Vec<bool>> = (0..27).map(|_| vec![false; width]).collect();
        let mut line = vec![false; width];
        for x in 100..110 {
            line[x] = true;
        }
        lines.push(line);
        cross_check_lines(&lines);
    }

    #[test]
    fn packed_fill_range_spans_byte_boundaries() {
        let mut row = vec![0u8; 4];
        fill_row_range(&mut row, 3, 29, true);
        assert_eq!(row[0], 0b0001_1111);
        assert_eq!(row[1], 0xFF);
        assert_eq!(row[2], 0xFF);
        assert_eq!(row[3], 0b1111_1000);
        fill_row_range(&mut row, 5, 26, false);
        assert_eq!(row[0], 0b0001_1000);
        assert_eq!(row[1], 0x00);
        assert_eq!(row[2], 0x00);
        // Bits 26..=28 remain set (we only cleared 5..26), plus trailing zero padding.
        assert_eq!(row[3], 0b0011_1000);
    }

    #[test]
    fn next_changing_packed_finds_transition() {
        // row: 1111_0000 1111_0000 ... (bit-level 11110000 pattern)
        let row = vec![0xF0, 0xF0, 0xF0];
        assert_eq!(next_changing_packed(&row, -1, false, 24), 0);
        assert_eq!(next_changing_packed(&row, 0, true, 24), 4);
        assert_eq!(next_changing_packed(&row, 4, false, 24), 8);
        assert_eq!(next_changing_packed(&row, 7, false, 24), 8);
    }

    #[test]
    fn next_changing_packed_requires_an_actual_transition() {
        // When a0 lies inside a run of opposite-colour pixels on the ref
        // row, the next b1 must be the transition back to opposite colour
        // *after* the interposed same-colour run, not the current pixel
        // under a0+1 (which shares a colour with the run we are in).
        //
        // Layout: WWWWWWWW BBBBBBBB WWWWWWWW BBBBBBBB -> 0x00 0xFF 0x00 0xFF
        // Bit 8..15 are black, bit 16..23 white, bit 24..31 black.
        let row = vec![0x00, 0xFF, 0x00, 0xFF];
        // Starting at a0=10 with a0_color=white, position 11 already sits
        // in the black run. The "first pixel of opposite colour" heuristic
        // would incorrectly return 11; the transition-aware rule must
        // return 24, the next rising edge back into black after the
        // intervening white run.
        assert_eq!(next_changing_packed(&row, 10, false, 32), 24);
        // Starting at a0=2 with a0_color=white, the first transition to
        // black is bit 8.
        assert_eq!(next_changing_packed(&row, 2, false, 32), 8);
        // Starting at a0=10 with a0_color=black, the first transition to
        // white is bit 16 (the leading white run at 0..7 does not count
        // because it is to the left of a0).
        assert_eq!(next_changing_packed(&row, 10, true, 32), 16);
    }
}
