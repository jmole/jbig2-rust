//! JBIG2 Huffman codec (Annex B).
//!
//! Each table line has a `(prefix_length, range_length, range_low,
//! lower_range)` tuple. The actual binary prefix for each line is derived via
//! the canonical-Huffman algorithm in Annex B.3: sort by prefix length then
//! by input index, then assign codes incrementally. This matches how
//! user-supplied tables are transmitted on the wire, so a single mechanism
//! covers both standard tables and tables received in a `TABLES` segment.
//!
//! Tables may end with zero, one, or two "range" lines (low / high) that
//! cover `-infinity..range_low` and `range_low..infinity` respectively. An
//! optional OOB line is identified by `range_length = -1`.

use crate::error::{Jbig2Error, Jbig2Result};

/// A single table line.
#[derive(Clone, Copy, Debug)]
pub struct HuffLine {
    /// Prefix length in bits. `0` means the line is unused.
    pub prefix_len: u8,
    /// Number of range bits read after the prefix, or `-1` for OOB.
    pub range_len: i8,
    /// Low end of the range (signed).
    pub range_low: i32,
    /// `true` for the lower-range line (`range_low - raw`).
    pub lower_range: bool,
}

impl HuffLine {
    /// Convenience helper for test code.
    pub const fn new(prefix_len: u8, range_len: i8, range_low: i32) -> Self {
        Self {
            prefix_len,
            range_len,
            range_low,
            lower_range: false,
        }
    }
}

/// A compiled Huffman table: one entry per input line, plus the assigned
/// canonical prefix and a pre-computed prefix mask.
#[derive(Clone, Debug)]
pub struct HuffTable {
    /// Optional label, helpful for debug output.
    pub name: &'static str,
    entries: Vec<HuffEntry>,
}

#[derive(Clone, Copy, Debug)]
struct HuffEntry {
    prefix: u32,
    prefix_len: u8,
    range_len: i8,
    range_low: i32,
    lower_range: bool,
}

impl HuffTable {
    /// Compile `lines` into a canonical-prefix table.
    pub fn new(name: &'static str, lines: &[HuffLine]) -> Jbig2Result<Self> {
        // Assign canonical prefixes per B.3:
        //   1. Sort by (prefix_len, original_index), skipping prefix_len == 0.
        //   2. prefix starts at 0 with the first used length; for each next
        //      line, prefix = (prefix + 1); if its length is longer, shift
        //      left by the length difference.
        let mut ordered: Vec<(usize, HuffLine)> = lines
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, l)| l.prefix_len > 0)
            .collect();
        ordered.sort_by_key(|(idx, l)| (l.prefix_len, *idx));

        let mut entries = Vec::with_capacity(ordered.len());
        let mut last_len: u32 = 0;
        let mut code: u32 = 0;
        let mut first = true;
        for (_, line) in &ordered {
            let len = line.prefix_len as u32;
            if first {
                code = 0;
                first = false;
            } else {
                code += 1;
                if len > last_len {
                    code <<= len - last_len;
                }
            }
            if len > 0 && code >= (1u32 << len) {
                return Err(Jbig2Error::InvalidHuffman(
                    "over-allocated canonical Huffman table",
                ));
            }
            last_len = len;
            entries.push(HuffEntry {
                prefix: code,
                prefix_len: len as u8,
                range_len: line.range_len,
                range_low: line.range_low,
                lower_range: line.lower_range,
            });
        }
        Ok(Self { name, entries })
    }

    /// Decode one value from a [`HuffReader`]. `Ok(None)` means OOB.
    pub fn decode(&self, r: &mut HuffReader<'_>) -> Jbig2Result<Option<i32>> {
        let mut bits: u32 = 0;
        for len in 1..=32 {
            let b = r.read_bit()?;
            bits = (bits << 1) | b as u32;
            for e in &self.entries {
                if e.prefix_len as u32 == len && e.prefix == bits {
                    return decode_entry(e, r);
                }
            }
        }
        Err(Jbig2Error::InvalidHuffman("no matching prefix in 32 bits"))
    }

    /// Encode one value to a [`HuffWriter`]. Pass `None` to emit OOB.
    pub fn encode(&self, w: &mut HuffWriter, value: Option<i32>) -> Jbig2Result<()> {
        let (entry, remainder) = self.lookup_encode(value)?;
        w.write_bits(entry.prefix, entry.prefix_len as u32);
        if let Some((raw, n)) = remainder {
            w.write_bits(raw, n);
        }
        Ok(())
    }

    fn lookup_encode(
        &self,
        value: Option<i32>,
    ) -> Jbig2Result<(&HuffEntry, Option<(u32, u32)>)> {
        for e in &self.entries {
            match (value, e.range_len) {
                (None, -1) => return Ok((e, None)),
                (Some(v), 0) if v == e.range_low => return Ok((e, None)),
                (Some(v), n) if n > 0 => {
                    let n = n as i64;
                    let lo = e.range_low as i64;
                    let span = 1i64 << n;
                    let v64 = v as i64;
                    let fits = if e.lower_range {
                        v64 <= lo && v64 > lo - span
                    } else {
                        v64 >= lo && v64 < lo + span
                    };
                    if fits {
                        let raw = if e.lower_range {
                            (lo - v64) as u32
                        } else {
                            (v64 - lo) as u32
                        };
                        return Ok((e, Some((raw, n as u32))));
                    }
                }
                _ => continue,
            }
        }
        Err(Jbig2Error::InvalidHuffman(
            "value does not fit any table line",
        ))
    }
}

fn decode_entry(e: &HuffEntry, r: &mut HuffReader<'_>) -> Jbig2Result<Option<i32>> {
    match e.range_len {
        -1 => Ok(None),
        0 => Ok(Some(e.range_low)),
        n if n > 0 => {
            let raw = r.read_bits(n as u32)? as i32;
            let val = if e.lower_range {
                e.range_low - raw
            } else {
                e.range_low + raw
            };
            Ok(Some(val))
        }
        _ => Err(Jbig2Error::InvalidHuffman("negative range length")),
    }
}

/// Bit-level MSB-first reader.
pub struct HuffReader<'a> {
    buf: &'a [u8],
    byte: usize,
    bit: u8,
}

impl<'a> HuffReader<'a> {
    /// Start reading at the beginning of `buf`.
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, byte: 0, bit: 0 }
    }

    /// Read one bit.
    #[inline]
    pub fn read_bit(&mut self) -> Jbig2Result<u8> {
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

    /// Read `n` bits (n <= 32) as a big-endian unsigned integer.
    pub fn read_bits(&mut self, n: u32) -> Jbig2Result<u32> {
        let mut v: u32 = 0;
        for _ in 0..n {
            v = (v << 1) | self.read_bit()? as u32;
        }
        Ok(v)
    }

    /// Re-align to the next byte boundary.
    pub fn byte_align(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
            self.byte += 1;
        }
    }

    /// Current byte offset (post-alignment).
    pub fn byte_pos(&self) -> usize {
        self.byte
    }
}

/// Bit-level MSB-first writer with zero padding on flush.
pub struct HuffWriter {
    out: Vec<u8>,
    acc: u32,
    bits: u8,
}

impl HuffWriter {
    /// New writer.
    pub fn new() -> Self {
        Self { out: Vec::new(), acc: 0, bits: 0 }
    }

    /// Write one bit.
    #[inline]
    pub fn write_bit(&mut self, b: u8) {
        self.acc = (self.acc << 1) | (b & 1) as u32;
        self.bits += 1;
        while self.bits >= 8 {
            self.bits -= 8;
            let v = ((self.acc >> self.bits) & 0xFF) as u8;
            self.out.push(v);
            self.acc &= (1 << self.bits) - 1;
        }
    }

    /// Write `n` bits of `v` (MSB first).
    pub fn write_bits(&mut self, v: u32, n: u32) {
        let mut i = n;
        while i > 0 {
            i -= 1;
            self.write_bit(((v >> i) & 1) as u8);
        }
    }

    /// Flush with zero padding and return bytes.
    pub fn finish(mut self) -> Vec<u8> {
        if self.bits > 0 {
            let v = ((self.acc << (8 - self.bits)) & 0xFF) as u8;
            self.out.push(v);
        }
        self.out
    }
}

impl Default for HuffWriter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Standard tables B.1 .. B.15 (Annex B.5).
//
// Each macro entry corresponds to one row in the spec table, in the order the
// spec lists them. The canonical-code algorithm assigns prefixes based on the
// tuple `(prefix_length, input_index)` — the order of rows within a table is
// significant.
// ---------------------------------------------------------------------------

macro_rules! line {
    ($pl:expr, $rl:expr, $rv:expr) => {
        HuffLine {
            prefix_len: $pl,
            range_len: $rl,
            range_low: $rv,
            lower_range: false,
        }
    };
    ($pl:expr, $rl:expr, $rv:expr, low) => {
        HuffLine {
            prefix_len: $pl,
            range_len: $rl,
            range_low: $rv,
            lower_range: true,
        }
    };
}

fn table_b1() -> HuffTable {
    let lines = [
        line!(1, 4, 0),
        line!(2, 8, 16),
        line!(3, 16, 272),
        line!(3, 32, 65808),
    ];
    HuffTable::new("B1", &lines).expect("valid canonical table")
}

fn table_b2() -> HuffTable {
    let lines = [
        line!(1, 0, 0),
        line!(2, 0, 1),
        line!(3, 0, 2),
        line!(4, 3, 3),
        line!(5, 6, 11),
        line!(6, 32, 75),
        line!(6, -1, 0), // OOB
    ];
    HuffTable::new("B2", &lines).unwrap()
}

fn table_b3() -> HuffTable {
    let lines = [
        line!(8, 8, -256),
        line!(1, 0, 0),
        line!(2, 0, 1),
        line!(3, 0, 2),
        line!(4, 3, 3),
        line!(5, 6, 11),
        line!(8, 32, -257, low),
        line!(7, 32, 75),
        line!(6, -1, 0),
    ];
    HuffTable::new("B3", &lines).unwrap()
}

fn table_b4() -> HuffTable {
    let lines = [
        line!(1, 0, 1),
        line!(2, 0, 2),
        line!(3, 0, 3),
        line!(4, 3, 4),
        line!(5, 6, 12),
        line!(5, 32, 76),
    ];
    HuffTable::new("B4", &lines).unwrap()
}

fn table_b5() -> HuffTable {
    let lines = [
        line!(7, 8, -255),
        line!(1, 0, 1),
        line!(2, 0, 2),
        line!(3, 0, 3),
        line!(4, 3, 4),
        line!(5, 6, 12),
        line!(7, 32, -256, low),
        line!(6, 32, 76),
    ];
    HuffTable::new("B5", &lines).unwrap()
}

fn table_b6() -> HuffTable {
    let lines = [
        line!(5, 10, -2048),
        line!(4, 9, -1024),
        line!(4, 8, -512),
        line!(4, 7, -256),
        line!(5, 6, -128),
        line!(5, 5, -64),
        line!(4, 5, -32),
        line!(2, 7, 0),
        line!(3, 7, 128),
        line!(3, 8, 256),
        line!(4, 9, 512),
        line!(4, 10, 1024),
        line!(6, 32, -2049, low),
        line!(6, 32, 2048),
    ];
    HuffTable::new("B6", &lines).unwrap()
}

fn table_b7() -> HuffTable {
    let lines = [
        line!(4, 9, -1024),
        line!(3, 8, -512),
        line!(4, 7, -256),
        line!(5, 6, -128),
        line!(5, 5, -64),
        line!(4, 5, -32),
        line!(4, 5, 0),
        line!(5, 5, 32),
        line!(5, 6, 64),
        line!(4, 7, 128),
        line!(3, 8, 256),
        line!(3, 9, 512),
        line!(3, 10, 1024),
        line!(5, 32, -1025, low),
        line!(5, 32, 2048),
    ];
    HuffTable::new("B7", &lines).unwrap()
}

fn table_b8() -> HuffTable {
    let lines = [
        line!(8, 3, -15),
        line!(9, 1, -7),
        line!(8, 1, -5),
        line!(9, 0, -3),
        line!(7, 0, -2),
        line!(4, 0, -1),
        line!(2, 1, 0),
        line!(5, 0, 2),
        line!(6, 0, 3),
        line!(3, 4, 4),
        line!(6, 1, 20),
        line!(4, 4, 22),
        line!(4, 5, 38),
        line!(5, 6, 70),
        line!(5, 7, 134),
        line!(6, 7, 262),
        line!(7, 8, 390),
        line!(6, 10, 646),
        line!(9, 32, -16, low),
        line!(9, 32, 1670),
        line!(2, -1, 0),
    ];
    HuffTable::new("B8", &lines).unwrap()
}

fn table_b9() -> HuffTable {
    let lines = [
        line!(8, 4, -31),
        line!(9, 2, -15),
        line!(8, 2, -11),
        line!(9, 1, -7),
        line!(7, 1, -5),
        line!(4, 1, -3),
        line!(3, 1, -1),
        line!(3, 1, 1),
        line!(5, 1, 3),
        line!(6, 1, 5),
        line!(3, 5, 7),
        line!(6, 2, 39),
        line!(4, 5, 43),
        line!(4, 6, 75),
        line!(5, 7, 139),
        line!(5, 8, 267),
        line!(6, 8, 523),
        line!(7, 9, 779),
        line!(6, 11, 1291),
        line!(9, 32, -32, low),
        line!(9, 32, 3339),
        line!(2, -1, 0),
    ];
    HuffTable::new("B9", &lines).unwrap()
}

fn table_b10() -> HuffTable {
    let lines = [
        line!(7, 4, -21),
        line!(8, 0, -5),
        line!(7, 0, -4),
        line!(5, 0, -3),
        line!(2, 2, -2),
        line!(5, 0, 2),
        line!(6, 0, 3),
        line!(7, 0, 4),
        line!(8, 0, 5),
        line!(2, 6, 6),
        line!(5, 5, 70),
        line!(6, 5, 102),
        line!(6, 6, 134),
        line!(6, 7, 198),
        line!(6, 8, 326),
        line!(6, 9, 582),
        line!(6, 10, 1094),
        line!(7, 11, 2118),
        line!(8, 32, -22, low),
        line!(8, 32, 4166),
        line!(2, -1, 0),
    ];
    HuffTable::new("B10", &lines).unwrap()
}

fn table_b11() -> HuffTable {
    let lines = [
        line!(1, 0, 1),
        line!(2, 1, 2),
        line!(4, 0, 4),
        line!(4, 1, 5),
        line!(5, 1, 7),
        line!(5, 2, 9),
        line!(6, 2, 13),
        line!(7, 2, 17),
        line!(7, 3, 21),
        line!(7, 4, 29),
        line!(7, 5, 45),
        line!(7, 6, 77),
        line!(7, 32, 141),
    ];
    HuffTable::new("B11", &lines).unwrap()
}

fn table_b12() -> HuffTable {
    let lines = [
        line!(1, 0, 1),
        line!(2, 0, 2),
        line!(3, 1, 3),
        line!(5, 0, 5),
        line!(5, 1, 6),
        line!(6, 1, 8),
        line!(7, 0, 10),
        line!(7, 1, 11),
        line!(7, 2, 13),
        line!(7, 3, 17),
        line!(7, 4, 25),
        line!(8, 5, 41),
        line!(8, 32, 73),
    ];
    HuffTable::new("B12", &lines).unwrap()
}

fn table_b13() -> HuffTable {
    let lines = [
        line!(1, 0, 1),
        line!(3, 0, 2),
        line!(4, 0, 3),
        line!(5, 0, 4),
        line!(4, 1, 5),
        line!(3, 3, 7),
        line!(6, 1, 15),
        line!(6, 2, 17),
        line!(6, 3, 21),
        line!(6, 4, 29),
        line!(6, 5, 45),
        line!(7, 6, 77),
        line!(7, 32, 141),
    ];
    HuffTable::new("B13", &lines).unwrap()
}

fn table_b14() -> HuffTable {
    let lines = [
        line!(3, 0, -2),
        line!(3, 0, -1),
        line!(1, 0, 0),
        line!(3, 0, 1),
        line!(3, 0, 2),
    ];
    HuffTable::new("B14", &lines).unwrap()
}

fn table_b15() -> HuffTable {
    let lines = [
        line!(7, 4, -24),
        line!(6, 2, -8),
        line!(5, 1, -4),
        line!(4, 0, -2),
        line!(3, 0, -1),
        line!(1, 0, 0),
        line!(3, 0, 1),
        line!(4, 0, 2),
        line!(5, 1, 3),
        line!(6, 2, 5),
        line!(7, 4, 9),
        line!(7, 32, -25, low),
        line!(7, 32, 25),
    ];
    HuffTable::new("B15", &lines).unwrap()
}

/// Get a built-in table by number `1..=15`.
pub fn standard_table(number: u32) -> Jbig2Result<HuffTable> {
    Ok(match number {
        1 => table_b1(),
        2 => table_b2(),
        3 => table_b3(),
        4 => table_b4(),
        5 => table_b5(),
        6 => table_b6(),
        7 => table_b7(),
        8 => table_b8(),
        9 => table_b9(),
        10 => table_b10(),
        11 => table_b11(),
        12 => table_b12(),
        13 => table_b13(),
        14 => table_b14(),
        15 => table_b15(),
        _ => return Err(Jbig2Error::InvalidHuffman("unknown standard table number")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(t: &HuffTable, values: &[Option<i32>]) {
        let mut w = HuffWriter::new();
        for &v in values {
            t.encode(&mut w, v).unwrap_or_else(|e| panic!("table {} encode {v:?}: {e}", t.name));
        }
        let buf = w.finish();
        let mut r = HuffReader::new(&buf);
        for (i, &v) in values.iter().enumerate() {
            let got = t.decode(&mut r).unwrap();
            assert_eq!(got, v, "table {} mismatch at {i}: {v:?} vs {got:?}", t.name);
        }
    }

    #[test]
    fn b1_round_trip() {
        round_trip(&table_b1(), &[
            Some(0), Some(15), Some(16), Some(271), Some(272), Some(65_807), Some(65_808), Some(123_456),
        ]);
    }

    #[test]
    fn b2_round_trip_including_oob() {
        round_trip(&table_b2(), &[Some(0), Some(1), Some(2), Some(3), Some(10), Some(11), Some(74), Some(75), None]);
    }

    #[test]
    fn b3_negative_ranges() {
        round_trip(&table_b3(), &[Some(-256), Some(-1), Some(0), Some(74), Some(75), Some(-257), Some(-1000), None]);
    }

    #[test]
    fn b6_signed_delta() {
        round_trip(&table_b6(), &[Some(-2048), Some(-1), Some(0), Some(127), Some(2047), Some(2048), Some(-2049), Some(100_000), Some(-100_000)]);
    }

    #[test]
    fn b7_wide() {
        round_trip(&table_b7(), &[Some(-1024), Some(-1), Some(0), Some(31), Some(2047), Some(-1025), Some(50_000)]);
    }

    #[test]
    fn b14_three_values() {
        round_trip(&table_b14(), &[Some(-2), Some(-1), Some(0), Some(1), Some(2)]);
    }

    #[test]
    fn b15_sweep() {
        let mut vals: Vec<Option<i32>> = (-30..=30).map(Some).collect();
        vals.push(Some(100));
        vals.push(Some(-100));
        round_trip(&table_b15(), &vals);
    }
}
