//! T.4/T.6 MH / MR / MMR line codec (Annex 7).
//!
//! JBIG2 generic-region segments with the MMR flag set carry bitmap data
//! encoded as CCITT Group 4 (T.6 / MMR). The codec below implements:
//!
//! * Modified Huffman (MH) 1-D lines (used internally by the 2-D coder for
//!   horizontal mode, and by standalone T.4 data).
//! * Modified READ (MR) 2-D lines referring back to the previous line.
//! * T.6 MMR which is MR without EOL markers and with RTC trailing markers.
//!
//! For JBIG2 we only exercise T.6 in practice, but the other two modes are
//! implemented for completeness and test coverage.
//!
//! This module is the correctness-first bit-at-a-time port of the reference
//! `decsub.cpp` / `codsub.cpp`. The production decode path for T.6 is the
//! LUT-driven, packed-row implementation in [`crate::coding::mmr_lut`]; the
//! routines here are retained as (a) the reference against which the fast
//! decoder is cross-checked line-by-line (see `tests/mmr_diag.rs` and the
//! unit tests in `mmr_lut`), and (b) the bool-oriented MMR encoder, whose
//! runs are small enough that the extra `Vec<bool>` intermediate has not
//! shown up as a hot spot in the `encode/generic/arith` benchmark. If
//! future measurements surface encode-side T.6 as a bottleneck, the
//! encoder here is the place to add a packed-row fast path mirroring the
//! decoder.

use crate::error::{Jbig2Error, Jbig2Result};

const EOL_LEN: u32 = 12;
const EOL_BITS: u32 = 0b0000_0000_0001;

// Control codes for 2-D READ mode.
const CTRL_PASS: u32 = 0b0001;
const CTRL_PASS_LEN: u8 = 4;
const CTRL_H: u32 = 0b001;
const CTRL_H_LEN: u8 = 3;
const CTRL_V0: u32 = 0b1;
const CTRL_V0_LEN: u8 = 1;
const CTRL_VR1: u32 = 0b011;
const CTRL_VR1_LEN: u8 = 3;
const CTRL_VL1: u32 = 0b010;
const CTRL_VL1_LEN: u8 = 3;
const CTRL_VR2: u32 = 0b000011;
const CTRL_VR2_LEN: u8 = 6;
const CTRL_VL2: u32 = 0b000010;
const CTRL_VL2_LEN: u8 = 6;
const CTRL_VR3: u32 = 0b0000011;
const CTRL_VR3_LEN: u8 = 7;
const CTRL_VL3: u32 = 0b0000010;
const CTRL_VL3_LEN: u8 = 7;

// Terminator tables (runs 0..63) and make-up tables (runs 64..).
// (code, length) indexed by run length.
static WHITE_TERM: &[(u32, u8)] = &[
    (0x35, 8), (0x07, 6), (0x07, 4), (0x08, 4), (0x0B, 4), (0x0C, 4), (0x0E, 4), (0x0F, 4),
    (0x13, 5), (0x14, 5), (0x07, 5), (0x08, 5), (0x08, 6), (0x03, 6), (0x34, 6), (0x35, 6),
    (0x2A, 6), (0x2B, 6), (0x27, 7), (0x0C, 7), (0x08, 7), (0x17, 7), (0x03, 7), (0x04, 7),
    (0x28, 7), (0x2B, 7), (0x13, 7), (0x24, 7), (0x18, 7), (0x02, 8), (0x03, 8), (0x1A, 8),
    (0x1B, 8), (0x12, 8), (0x13, 8), (0x14, 8), (0x15, 8), (0x16, 8), (0x17, 8), (0x28, 8),
    (0x29, 8), (0x2A, 8), (0x2B, 8), (0x2C, 8), (0x2D, 8), (0x04, 8), (0x05, 8), (0x0A, 8),
    (0x0B, 8), (0x52, 8), (0x53, 8), (0x54, 8), (0x55, 8), (0x24, 8), (0x25, 8), (0x58, 8),
    (0x59, 8), (0x5A, 8), (0x5B, 8), (0x4A, 8), (0x4B, 8), (0x32, 8), (0x33, 8), (0x34, 8),
];

static WHITE_MAKEUP: &[(u32, u8)] = &[
    (0x1B, 5),  (0x12, 5),  (0x17, 6),  (0x37, 7),  (0x36, 8),  (0x37, 8),  (0x64, 8),  (0x65, 8),
    (0x68, 8),  (0x67, 8),  (0xCC, 9),  (0xCD, 9),  (0xD2, 9),  (0xD3, 9),  (0xD4, 9),  (0xD5, 9),
    (0xD6, 9),  (0xD7, 9),  (0xD8, 9),  (0xD9, 9),  (0xDA, 9),  (0xDB, 9),  (0x98, 9),  (0x99, 9),
    (0x9A, 9),  (0x18, 6),  (0x9B, 9),
    // make-up 1792..:
    (0x08, 11), (0x0C, 11), (0x0D, 11),
    (0x12, 12), (0x13, 12), (0x14, 12), (0x15, 12), (0x16, 12), (0x17, 12),
    (0x1C, 12), (0x1D, 12), (0x1E, 12), (0x1F, 12),
];

static BLACK_TERM: &[(u32, u8)] = &[
    (0x37, 10), (0x02, 3), (0x03, 2), (0x02, 2), (0x03, 3), (0x03, 4), (0x02, 4), (0x03, 5),
    (0x05, 6),  (0x04, 6), (0x04, 7), (0x05, 7), (0x07, 7), (0x04, 8), (0x07, 8), (0x18, 9),
    (0x17, 10), (0x18, 10), (0x08, 10), (0x67, 11), (0x68, 11), (0x6C, 11), (0x37, 11),
    (0x28, 11), (0x17, 11), (0x18, 11), (0xCA, 12), (0xCB, 12), (0xCC, 12), (0xCD, 12),
    (0x68, 12), (0x69, 12), (0x6A, 12), (0x6B, 12), (0xD2, 12), (0xD3, 12), (0xD4, 12),
    (0xD5, 12), (0xD6, 12), (0xD7, 12), (0x6C, 12), (0x6D, 12), (0xDA, 12), (0xDB, 12),
    (0x54, 12), (0x55, 12), (0x56, 12), (0x57, 12), (0x64, 12), (0x65, 12), (0x52, 12),
    (0x53, 12), (0x24, 12), (0x37, 12), (0x38, 12), (0x27, 12), (0x28, 12), (0x58, 12),
    (0x59, 12), (0x2B, 12), (0x2C, 12), (0x5A, 12), (0x66, 12), (0x67, 12),
];

static BLACK_MAKEUP: &[(u32, u8)] = &[
    (0x0F, 10), (0xC8, 12), (0xC9, 12), (0x5B, 12), (0x33, 12), (0x34, 12), (0x35, 12),
    (0x6C, 13), (0x6D, 13), (0x4A, 13), (0x4B, 13), (0x4C, 13), (0x4D, 13), (0x72, 13),
    (0x73, 13), (0x74, 13), (0x75, 13), (0x76, 13), (0x77, 13), (0x52, 13), (0x53, 13),
    (0x54, 13), (0x55, 13), (0x5A, 13), (0x5B, 13), (0x64, 13), (0x65, 13),
    // 1792..:
    (0x08, 11), (0x0C, 11), (0x0D, 11),
    (0x12, 12), (0x13, 12), (0x14, 12), (0x15, 12), (0x16, 12), (0x17, 12),
    (0x1C, 12), (0x1D, 12), (0x1E, 12), (0x1F, 12),
];

/// A bit-level writer that accumulates output MSB-first and flushes to
/// whole-byte boundaries on demand.
pub struct BitWriter {
    out: Vec<u8>,
    acc: u64,
    bits: u32,
}

impl BitWriter {
    /// Create a new bit writer.
    pub fn new() -> Self {
        Self { out: Vec::new(), acc: 0, bits: 0 }
    }

    /// Write `len` bits of `code`, MSB-first.
    pub fn write(&mut self, code: u32, len: u8) {
        let len = len as u32;
        self.acc = (self.acc << len) | (code as u64 & ((1u64 << len) - 1));
        self.bits += len;
        while self.bits >= 8 {
            self.bits -= 8;
            self.out.push(((self.acc >> self.bits) & 0xFF) as u8);
            self.acc &= (1u64 << self.bits) - 1;
        }
    }

    /// Flush any partial byte with zero padding and return the backing vector.
    pub fn finish(mut self) -> Vec<u8> {
        if self.bits > 0 {
            self.out.push(((self.acc << (8 - self.bits)) & 0xFF) as u8);
        }
        self.out
    }
}

impl Default for BitWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Bit-level reader, MSB first within each byte.
pub struct BitReader<'a> {
    buf: &'a [u8],
    byte: usize,
    bit: u8,
}

impl<'a> BitReader<'a> {
    /// New reader starting at the beginning of `buf`.
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, byte: 0, bit: 0 }
    }

    /// Read one bit.
    #[inline]
    pub fn read(&mut self) -> Jbig2Result<u8> {
        if self.byte >= self.buf.len() {
            return Err(Jbig2Error::UnexpectedEof { needed: 1 });
        }
        let v = (self.buf[self.byte] >> (7 - self.bit)) & 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.byte += 1;
        }
        Ok(v)
    }

    /// Peek up to 16 bits without consuming them.
    pub fn peek(&self, n: u8) -> Jbig2Result<u32> {
        let mut v: u32 = 0;
        let mut b = self.byte;
        let mut bit = self.bit;
        for _ in 0..n {
            if b >= self.buf.len() {
                return Err(Jbig2Error::UnexpectedEof { needed: 1 });
            }
            v = (v << 1) | ((self.buf[b] >> (7 - bit)) & 1) as u32;
            bit += 1;
            if bit == 8 {
                bit = 0;
                b += 1;
            }
        }
        Ok(v)
    }

    /// Advance by `n` bits without reading.
    pub fn skip(&mut self, n: u8) {
        for _ in 0..n {
            self.bit += 1;
            if self.bit == 8 {
                self.bit = 0;
                self.byte += 1;
            }
        }
    }

    /// Current byte position.
    pub fn byte_pos(&self) -> usize {
        self.byte
    }
}

/// Encode one MH line (horizontal mode).
pub fn encode_mh_line(w: &mut BitWriter, runs: &[u32]) {
    // Always start with white.
    let mut white = true;
    for &r in runs {
        encode_run(w, r, white);
        white = !white;
    }
}

fn encode_run(w: &mut BitWriter, mut run: u32, white: bool) {
    let (term, makeup): (&[(u32, u8)], &[(u32, u8)]) = if white {
        (WHITE_TERM, WHITE_MAKEUP)
    } else {
        (BLACK_TERM, BLACK_MAKEUP)
    };
    // Runs >= 2560 use stacked copies of the largest extended make-up.
    while run >= 2560 {
        let (c, l) = makeup[39];
        w.write(c, l);
        run -= 2560;
    }
    if run >= 1792 {
        // Extended make-up indices 27..=39 cover 1792, 1856, 1920, 1984, ...
        let step = ((run - 1792) / 64) as usize;
        let idx = 27 + step;
        let (c, l) = makeup[idx];
        w.write(c, l);
        run -= 1792 + 64 * step as u32;
    } else if run >= 64 {
        let idx = (run / 64) as usize - 1;
        let (c, l) = makeup[idx];
        w.write(c, l);
        run %= 64;
    }
    let (c, l) = term[run as usize];
    w.write(c, l);
}

/// Decode one MH line. Returns a vector of run lengths alternating
/// white/black, starting with white.
pub fn decode_mh_line(r: &mut BitReader<'_>, width: u32) -> Jbig2Result<Vec<u32>> {
    let mut runs = Vec::new();
    let mut white = true;
    let mut total = 0u32;
    while total < width {
        let run = decode_run(r, white)?;
        runs.push(run);
        total = total.saturating_add(run);
        white = !white;
    }
    Ok(runs)
}

fn decode_run(r: &mut BitReader<'_>, white: bool) -> Jbig2Result<u32> {
    // Read 1..13 bits, matching against terminator and make-up tables; handle
    // stacked make-up codes.
    let mut run: u32 = 0;
    loop {
        let (terminator, count) = decode_code(r, white)?;
        if terminator {
            run += count;
            return Ok(run);
        } else {
            run += count;
        }
    }
}

fn decode_code(r: &mut BitReader<'_>, white: bool) -> Jbig2Result<(bool, u32)> {
    // Brute-force match against the tables. This is the reference decoder
    // path; production decode goes through the 13-bit LUT in
    // `crate::coding::mmr_lut`, which this function is cross-checked
    // against line-by-line in the MMR diag harness.
    let (term, makeup): (&[(u32, u8)], &[(u32, u8)]) = if white {
        (WHITE_TERM, WHITE_MAKEUP)
    } else {
        (BLACK_TERM, BLACK_MAKEUP)
    };
    let mut bits: u32 = 0;
    for len in 1..=13u8 {
        bits = (bits << 1) | r.read()? as u32;
        for (i, &(c, l)) in term.iter().enumerate() {
            if l == len && c == bits {
                return Ok((true, i as u32));
            }
        }
        for (i, &(c, l)) in makeup.iter().enumerate() {
            if l == len && c == bits {
                let count = if i < 27 {
                    64 * (i as u32 + 1)
                } else {
                    1792 + 64 * (i as u32 - 27)
                };
                return Ok((false, count));
            }
        }
    }
    Err(Jbig2Error::InvalidHuffman("mmr/mh run code"))
}

/// Encode one T.6 (MMR) line referring back to `ref_line`.
///
/// `cur_line` and `ref_line` are `width`-long bool vectors (`true` = black).
pub fn encode_t6_line(w: &mut BitWriter, cur_line: &[bool], ref_line: &[bool]) {
    let width = cur_line.len();
    let mut a0: isize = -1;
    loop {
        let a0_color = if a0 < 0 { false } else { cur_line[a0 as usize] };
        let a1 = next_changing(cur_line, a0, a0_color);
        let b1 = next_changing_ref(ref_line, a0, a0_color);
        let b2 = next_changing(ref_line, b1 as isize, color_at(ref_line, b1 as isize));

        if (b2 as isize) < (a1 as isize) {
            // Pass mode
            w.write(CTRL_PASS, CTRL_PASS_LEN);
            a0 = b2 as isize;
        } else if (a1 as isize - b1 as isize).abs() <= 3 {
            let delta = a1 as isize - b1 as isize;
            match delta {
                0 => w.write(CTRL_V0, CTRL_V0_LEN),
                1 => w.write(CTRL_VR1, CTRL_VR1_LEN),
                -1 => w.write(CTRL_VL1, CTRL_VL1_LEN),
                2 => w.write(CTRL_VR2, CTRL_VR2_LEN),
                -2 => w.write(CTRL_VL2, CTRL_VL2_LEN),
                3 => w.write(CTRL_VR3, CTRL_VR3_LEN),
                -3 => w.write(CTRL_VL3, CTRL_VL3_LEN),
                _ => unreachable!(),
            }
            a0 = a1 as isize;
        } else {
            // Horizontal mode
            w.write(CTRL_H, CTRL_H_LEN);
            let a2 = next_changing(cur_line, a1 as isize, color_at(cur_line, a1 as isize));
            let (r1, r2) = (a1.saturating_sub(a0.max(0) as usize), a2.saturating_sub(a1));
            let (c1_white, _c2_white) = (a0_color == false, a0_color == true);
            let _ = _c2_white;
            encode_run(w, r1 as u32, c1_white);
            encode_run(w, r2 as u32, !c1_white);
            a0 = a2 as isize;
        }
        if (a0 as usize) >= width {
            break;
        }
    }
}

/// Decode one T.6 (MMR) line referring back to `ref_line`, producing a
/// `width`-long bool vector.
pub fn decode_t6_line(r: &mut BitReader<'_>, ref_line: &[bool], width: u32) -> Jbig2Result<Vec<bool>> {
    let width = width as usize;
    let mut cur = vec![false; width];
    let mut a0: isize = -1;
    let mut a0_color = false;
    while (a0 as isize) < width as isize {
        if (a0 as isize) >= width as isize {
            break;
        }
        let mode = read_ctrl(r)?;
        match mode {
            Ctrl::Pass => {
                let b1 = next_changing_ref(ref_line, a0, a0_color);
                let b2 = next_changing(ref_line, b1 as isize, color_at(ref_line, b1 as isize));
                fill(&mut cur, a0.max(0) as usize, b2, a0_color);
                a0 = b2 as isize;
            }
            Ctrl::V(delta) => {
                let b1 = next_changing_ref(ref_line, a0, a0_color) as isize;
                let a1 = (b1 + delta) as isize;
                fill(&mut cur, a0.max(0) as usize, a1 as usize, a0_color);
                a0 = a1;
                a0_color = !a0_color;
            }
            Ctrl::Horizontal => {
                let (r1_color, r2_color) = (a0_color, !a0_color);
                let r1 = decode_run(r, !r1_color)?;
                let r2 = decode_run(r, !r2_color)?;
                let a1 = (a0.max(0) as usize) + r1 as usize;
                let a2 = a1 + r2 as usize;
                fill(&mut cur, a0.max(0) as usize, a1, a0_color);
                fill(&mut cur, a1, a2, !a0_color);
                a0 = a2 as isize;
            }
        }
    }
    Ok(cur)
}

enum Ctrl {
    Pass,
    V(isize),
    Horizontal,
}

fn read_ctrl(r: &mut BitReader<'_>) -> Jbig2Result<Ctrl> {
    // V0 (1) has highest priority, followed by 3-bit codes, then 4-bit, etc.
    let b1 = r.read()?;
    if b1 == 1 {
        return Ok(Ctrl::V(0));
    }
    // bit is 0; read two more
    let b2 = r.read()?;
    let b3 = r.read()?;
    let first3 = (b2 << 1) | b3;
    match first3 {
        0b10 /* originally 010 */ => return Ok(Ctrl::V(-1)),
        0b11 /* originally 011 */ => return Ok(Ctrl::V(1)),
        0b01 /* originally 001 */ => return Ok(Ctrl::Horizontal),
        _ => {}
    }
    // We've consumed 3 bits "000", need to discriminate 0001 (pass), or
    // 0000 1x/1x/... for V2/V3.
    let b4 = r.read()?;
    if b4 == 1 {
        return Ok(Ctrl::Pass);
    }
    // 0000_ ... deeper
    let b5 = r.read()?;
    let b6 = r.read()?;
    // 000001x (VR2/VL2)
    if b5 == 1 {
        return Ok(Ctrl::V(if b6 == 1 { 2 } else { -2 }));
    }
    // 0000001x (VR3/VL3)
    let b7 = r.read()?;
    if b6 == 1 {
        return Ok(Ctrl::V(if b7 == 1 { 3 } else { -3 }));
    }
    Err(Jbig2Error::InvalidHuffman("mmr control code"))
}

fn color_at(line: &[bool], x: isize) -> bool {
    if x < 0 || (x as usize) >= line.len() {
        false
    } else {
        line[x as usize]
    }
}

fn next_changing(line: &[bool], start: isize, color: bool) -> usize {
    let mut i = if start < 0 { 0 } else { start as usize + 1 };
    while i < line.len() && line[i] == color {
        i += 1;
    }
    i
}

fn next_changing_ref(ref_line: &[bool], a0: isize, a0_color: bool) -> usize {
    // Per T.6 definitions, b1 is the first *changing* picture element on the
    // reference line to the right of a0 whose colour is opposite to a0's
    // colour. "Changing" means the pixel differs from the one immediately to
    // its left. The virtual pixel preceding position 0 is treated as white,
    // matching the CCITT convention for the imaginary first reference line.
    //
    // A naive "first pixel of opposite colour" search is wrong when a0 sits
    // inside a run of opposite-colour pixels on the reference line: that run
    // has no transition to opposite colour within it, so we must keep
    // scanning until we pass through a same-colour run and then back across
    // to the opposite colour.
    let len = ref_line.len();
    let start = if a0 < 0 { 0 } else { a0 as usize + 1 };
    let mut i = start;
    while i < len {
        let prev = if i == 0 { false } else { ref_line[i - 1] };
        if ref_line[i] != prev && ref_line[i] != a0_color {
            return i;
        }
        i += 1;
    }
    len
}

fn fill(line: &mut [bool], start: usize, end: usize, color: bool) {
    let end = end.min(line.len());
    for x in start..end {
        line[x] = color;
    }
}

/// Convert a packed bitmap row into a bool vector (`true` = 1 = black).
pub fn row_to_bools(row: &[u8], width: u32) -> Vec<bool> {
    let mut v = Vec::with_capacity(width as usize);
    for x in 0..width {
        let b = row[x as usize / 8];
        v.push(((b >> (7 - (x & 7))) & 1) != 0);
    }
    v
}

/// Convert a bool vector back into a packed bitmap row.
pub fn bools_to_row(bits: &[bool], stride: usize) -> Vec<u8> {
    let mut out = vec![0u8; stride];
    for (x, &b) in bits.iter().enumerate() {
        if b {
            out[x / 8] |= 1 << (7 - (x & 7));
        }
    }
    out
}

/// Emit the T.6 end-of-block marker (`0000_0000_0001 0000_0000_0001`).
pub fn write_t6_eofb(w: &mut BitWriter) {
    w.write(EOL_BITS, EOL_LEN as u8);
    w.write(EOL_BITS, EOL_LEN as u8);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip_runs(runs: &[u32]) {
        let mut w = BitWriter::new();
        encode_mh_line(&mut w, runs);
        let buf = w.finish();
        let width: u32 = runs.iter().sum();
        let mut r = BitReader::new(&buf);
        let got = decode_mh_line(&mut r, width).unwrap();
        assert_eq!(got, runs);
    }

    #[test]
    fn mh_short_runs() {
        round_trip_runs(&[3, 2, 5, 4]);
        round_trip_runs(&[7, 8, 15, 16]);
        round_trip_runs(&[63, 1, 63, 1, 64, 1, 65, 1, 128]);
    }

    #[test]
    fn mh_long_run_whites() {
        round_trip_runs(&[1000, 100, 500, 700]);
        round_trip_runs(&[2560, 1, 2560]);
        round_trip_runs(&[1792, 1, 1856, 1, 1920, 1, 2048, 1]);
    }

    #[test]
    fn t6_single_line_round_trip() {
        let cur: Vec<bool> = (0..32).map(|i| (i % 5) == 0).collect();
        let ref_line: Vec<bool> = vec![false; 32];
        let mut w = BitWriter::new();
        encode_t6_line(&mut w, &cur, &ref_line);
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        let back = decode_t6_line(&mut r, &ref_line, cur.len() as u32).unwrap();
        assert_eq!(back, cur);
    }

    #[test]
    fn t6_mimic_conformance_top_of_page() {
        // 27 all-white lines followed by a single line with a narrow black run:
        // this mirrors the start of the F01_200_TT9.jb2 data (27 V0 codes, then
        // an H-mode line). The test exists purely to produce a diagnostic
        // output we can inspect; it does not assert a specific byte pattern.
        let width = 1728usize;
        let mut w = BitWriter::new();
        let prev: Vec<bool> = vec![false; width];
        for _ in 0..27 {
            encode_t6_line(&mut w, &prev, &prev);
        }
        let mut line = vec![false; width];
        for x in 100..110 {
            line[x] = true;
        }
        encode_t6_line(&mut w, &line, &prev);
        let bytes = w.finish();
        eprintln!(
            "t6 encoder produced first 8 bytes: {:02x?}",
            &bytes[..8.min(bytes.len())]
        );
        // Self-decode to verify internal consistency
        let mut r = BitReader::new(&bytes);
        let mut ref_line = vec![false; width];
        for _ in 0..27 {
            let back = decode_t6_line(&mut r, &ref_line, width as u32).unwrap();
            assert_eq!(back, ref_line);
            ref_line = back;
        }
        let back = decode_t6_line(&mut r, &ref_line, width as u32).unwrap();
        assert_eq!(back, line);
    }

    #[test]
    fn t6_multi_line_round_trip() {
        let lines: Vec<Vec<bool>> = (0..8)
            .map(|y| (0..40).map(|x| ((x + y) % 7) == 0).collect())
            .collect();
        let mut w = BitWriter::new();
        let mut prev: Vec<bool> = vec![false; 40];
        for line in &lines {
            encode_t6_line(&mut w, line, &prev);
            prev = line.clone();
        }
        write_t6_eofb(&mut w);
        let buf = w.finish();

        let mut r = BitReader::new(&buf);
        let mut prev: Vec<bool> = vec![false; 40];
        for line in &lines {
            let got = decode_t6_line(&mut r, &prev, 40).unwrap();
            assert_eq!(&got, line);
            prev = got;
        }
    }
}
