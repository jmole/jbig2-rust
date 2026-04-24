//! MQ arithmetic coder (ITU-T T.88 Annex E).
//!
//! This is a Rust port of the JBIG2 reference MQ coder from
//! `vendor/T-REC-T.88-201808/.../MQ_codec.cpp`. It implements the JBIG2
//! byte-out / byte-in variant (no JPEG-2000 style `0xff` byte stuffing):
//!
//! * On encode, a `0xff` emitted byte is paired with the next partial byte so
//!   that the decoder can always read a full byte after each `0xff`.
//! * On decode, reading past the end of the stream synthesises a virtual
//!   `0xff 0xac` tail, which is what the spec requires (E.2.4).
//!
//! The coder keeps its registers in hot locals and only touches the [`CxState`]
//! cache on context updates, so the fast path is branch-light.
//!
//! # Bit ordering
//!
//! Following the spec, bits are emitted MSB-first. `encode(0)` codes the MPS
//! for that context, `encode(1)` the LPS; the CX's MPS bit lives in the high
//! bit of [`CxState::index`] so swapping between the two is a single XOR.

use core::ptr;

use crate::error::{Jbig2Error, Jbig2Result};

/// Number of distinct MQ context slots used by JBIG2.
///
/// The JBIG2 spec allocates a fixed block of 0x12000 entries (see
/// `Number_CX` in `Jb2Common.h`). The first 0x10000 are used by the
/// generic-region pixel contexts, the rest by the integer decoder families
/// (`IAAI`, `IADH`, ...).
pub const MQ_NUM_CONTEXTS: usize = 0x12000;

/// Per-context MQ state. Packed as a single byte where the MSB is the MPS
/// (most-probable-symbol) bit and the low 7 bits are the index into
/// [`QE_TABLE`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct CxState(pub u8);

impl CxState {
    /// Create a state with index = 0 and MPS = 0 (the JBIG2 default).
    pub const fn new() -> Self {
        Self(0)
    }
    /// Return the probability-index part (lower 7 bits).
    #[inline]
    pub fn index(self) -> u8 {
        self.0 & 0x7F
    }
    /// Return the MPS bit (high bit, already in position 0x80).
    #[inline]
    pub fn mps_bit(self) -> u8 {
        self.0 & 0x80
    }
}

/// One entry of the Qe probability state machine.
///
/// The reference `QeIndexTable` packs each entry into a 32-bit integer of the
/// form `0xSN_MM_QEHI_QELO` where `SN` is the NLPS byte with the switch bit
/// in the MSB. We unpack into a struct so the fast path uses plain field
/// reads.
#[derive(Clone, Copy, Debug)]
pub struct QeEntry {
    /// Probability of the LPS, in 16-bit fractional form.
    pub qe: u16,
    /// Next state after an MPS.
    pub nmps: u8,
    /// Next state after an LPS (low 7 bits).
    pub nlps: u8,
    /// When `true`, encoding an LPS swaps the MPS bit for this context.
    pub switch: bool,
}

/// Static Qe table, verbatim from the reference implementation and the spec.
pub const QE_TABLE: [QeEntry; 47] = build_qe_table();

const fn build_qe_table() -> [QeEntry; 47] {
    // Values mirror the `QeIndexTable` array in MQ_codec.h.
    const PACKED: [u32; 47] = [
        0x81015601, 0x06023401, 0x09031801, 0x0C040AC1, 0x1D050521, 0x21260221, 0x86075601,
        0x0E085401, 0x0E094801, 0x0E0A3801, 0x110B3001, 0x120C2401, 0x140D1C01, 0x151D1601,
        0x8E0F5601, 0x0E105401, 0x0F115101, 0x10124801, 0x11133801, 0x12143401, 0x13153001,
        0x13162801, 0x14172401, 0x15182201, 0x16191C01, 0x171A1801, 0x181B1601, 0x191C1401,
        0x1A1D1201, 0x1B1E1101, 0x1C1F0AC1, 0x1D2009C1, 0x1E2108A1, 0x1F220521, 0x20230441,
        0x212402A1, 0x22250221, 0x23260141, 0x24270111, 0x25280085, 0x26290049, 0x272A0025,
        0x282B0015, 0x292C0009, 0x2A2D0005, 0x2B2D0001, 0x2E2E5601,
    ];
    let mut out = [QeEntry {
        qe: 0,
        nmps: 0,
        nlps: 0,
        switch: false,
    }; 47];
    let mut i = 0;
    while i < 47 {
        let v = PACKED[i];
        let nlps_byte = ((v >> 24) & 0xFF) as u8;
        let nmps = ((v >> 16) & 0xFF) as u8;
        let qe = (v & 0xFFFF) as u16;
        out[i] = QeEntry {
            qe,
            nmps,
            nlps: nlps_byte & 0x7F,
            switch: (nlps_byte & 0x80) != 0,
        };
        i += 1;
    }
    out
}

/// A shared pool of MQ context states.
///
/// The context pool is owned separately from the coder because the same block
/// of contexts is reused across many `Enc`/`Dec` sessions (in particular the
/// MQ integer coders share these slots with the image coder).
#[derive(Clone, Debug)]
pub struct MqContexts {
    states: Box<[CxState]>,
}

impl MqContexts {
    /// Allocate a fresh pool with every context reset to `(index=0, mps=0)`.
    pub fn new(num: usize) -> Self {
        Self {
            states: vec![CxState::new(); num].into_boxed_slice(),
        }
    }

    /// Reset every context to the default.
    pub fn reset(&mut self) {
        for s in self.states.iter_mut() {
            *s = CxState::new();
        }
    }

    /// Number of contexts in the pool.
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// `true` if the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Borrow a single context.
    #[inline]
    pub fn get(&self, cx: usize) -> CxState {
        self.states[cx]
    }

    /// Mutably borrow the backing slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [CxState] {
        &mut self.states
    }

    /// Mutably borrow one context.
    #[inline]
    pub fn get_mut(&mut self, cx: usize) -> &mut CxState {
        &mut self.states[cx]
    }

    /// Borrow one context without a bounds check in release builds.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `cx < self.len()`.
    #[inline]
    pub(crate) unsafe fn get_unchecked_mut(&mut self, cx: usize) -> &mut CxState {
        debug_assert!(cx < self.states.len());
        // SAFETY: upheld by the caller; see contract above.
        unsafe { self.states.get_unchecked_mut(cx) }
    }
}

/// MQ arithmetic encoder.
///
/// Writes encoded bytes to an internal [`Vec<u8>`]; call [`MqEncoder::finish`]
/// to obtain the final stream.
pub struct MqEncoder {
    a_reg: u32,
    c_reg: u32,
    ct: i32,
    b: u8,
    first: bool,
    out: Vec<u8>,
    write_pos: usize,
}

impl MqEncoder {
    /// Create a fresh encoder. `capacity_hint` is used to pre-size the output
    /// buffer — pass something close to the expected compressed size to avoid
    /// reallocation on the hot path.
    pub fn new(capacity_hint: usize) -> Self {
        Self {
            a_reg: 0x8000,
            c_reg: 0,
            ct: 12,
            b: 0,
            first: true,
            out: Vec::with_capacity(capacity_hint),
            write_pos: 0,
        }
    }

    /// Encode one binary decision in context `cx`.
    #[inline]
    pub fn encode(&mut self, cxs: &mut MqContexts, cx: usize, d: u8) {
        // SAFETY: Segment encoders validate the context range before entering
        // their per-bit MQ loops, so repeated bounds checks are redundant here.
        let state = unsafe { cxs.get_unchecked_mut(cx) };
        let index = state.index() as usize;
        let mps_bit = state.mps_bit();
        let lps_bit = mps_bit ^ 0x80;
        debug_assert!(index < QE_TABLE.len());
        // SAFETY: `CxState::index()` is a 7-bit QE-table index. All states come
        // from `QE_TABLE` transitions, which stay in-range for all 47 entries.
        let entry = unsafe { *QE_TABLE.get_unchecked(index) };
        let qe = entry.qe as u32;
        let lps_state = if entry.switch {
            CxState(entry.nlps | lps_bit)
        } else {
            CxState(entry.nlps | mps_bit)
        };
        let mps_state = CxState(entry.nmps | mps_bit);
        let is_lps = (((d & 1) << 7) ^ mps_bit) != 0;

        self.a_reg = self.a_reg.wrapping_sub(qe);
        let swap = self.a_reg < qe;
        if is_lps == swap {
            self.c_reg = self.c_reg.wrapping_add(qe);
        } else {
            self.a_reg = qe;
        }

        if is_lps {
            *state = lps_state;
        } else if swap || (self.a_reg & 0x8000 == 0) {
            *state = mps_state;
        }

        self.renorm();
    }

    #[inline]
    fn renorm(&mut self) {
        let shift = self.a_reg.leading_zeros().saturating_sub(16);
        if shift == 0 {
            return;
        }

        let mut remaining = shift;
        while remaining != 0 {
            if self.ct == 0 {
                self.byte_out();
            }

            let step = remaining.min(self.ct as u32);
            self.a_reg <<= step;
            self.c_reg <<= step;
            self.ct -= step as i32;
            remaining -= step;

            if self.ct == 0 {
                self.byte_out();
            }
        }
    }

    #[inline]
    fn push_byte(&mut self, byte: u8) {
        if self.write_pos == self.out.capacity() {
            self.grow_out();
        }

        // SAFETY: `grow_out` ensures spare capacity for `write_pos`, and we
        // immediately extend the logical length to cover the initialized byte.
        unsafe {
            ptr::write(self.out.as_mut_ptr().add(self.write_pos), byte);
            self.write_pos += 1;
            self.out.set_len(self.write_pos);
        }
    }

    #[cold]
    fn grow_out(&mut self) {
        let additional = self.out.capacity().max(64);
        self.out.reserve(additional);
    }

    #[cold]
    #[inline]
    fn byte_out(&mut self) {
        if self.first {
            self.b = (self.c_reg >> 19) as u8;
            self.c_reg &= 0x7_FFFF;
            self.ct = 8;
            self.first = false;
            return;
        }
        let mut ff_flag = self.b == 0xFF;
        if !ff_flag {
            if (self.c_reg & 0x0800_0000) != 0 {
                self.b = self.b.wrapping_add(1);
            }
            self.c_reg &= 0x07FF_FFFF;
            if self.b == 0xFF {
                ff_flag = true;
            }
        }
        self.push_byte(self.b);
        if ff_flag {
            self.b = (self.c_reg >> 20) as u8;
            self.c_reg &= 0x000F_FFFF;
            self.ct = 7;
        } else {
            self.b = (self.c_reg >> 19) as u8;
            self.c_reg &= 0x0007_FFFF;
            self.ct = 8;
        }
    }

    #[cold]
    fn set_bits(&mut self) {
        let t = self.c_reg.wrapping_add(self.a_reg);
        self.c_reg |= 0xFFFF;
        if self.c_reg >= t {
            self.c_reg = self.c_reg.wrapping_sub(0x8000);
        }
    }

    /// Flush the encoder and return the encoded byte vector. The trailing
    /// state is emitted according to the JBIG2 termination procedure
    /// (E.2.9 / `MQ_flush`).
    pub fn finish(mut self) -> Vec<u8> {
        self.set_bits();
        self.c_reg <<= self.ct;
        self.byte_out();
        self.c_reg <<= self.ct;
        self.byte_out();
        if self.b != 0xFF {
            self.c_reg <<= self.ct;
            self.byte_out();
        }
        self.c_reg <<= self.ct;
        self.byte_out();
        // Note: the final buffered B is intentionally *not* pushed; the
        // JBIG2 reference MQ_flush (Annex E / MQ_codec.cpp) terminates with
        // B still sitting in the register. The decoder synthesises a
        // `0xFF00` tail for the remaining bit pump, so preserving the
        // exact byte count from the reference encoder is important for
        // conformance streams to land in the same MQ state at decode time.
        self.out.truncate(self.write_pos);
        self.out
    }

    /// Bytes emitted so far (not counting the buffered `b`).
    pub fn bytes_written(&self) -> usize {
        self.write_pos
    }
}

/// MQ arithmetic decoder. Reads from an immutable byte slice, tracking the
/// current read position inside.
pub struct MqDecoder<'a> {
    a_reg: u32,
    c_reg: u32,
    ct: i32,
    b: u8,
    buf: &'a [u8],
    pos: usize,
}

impl<'a> MqDecoder<'a> {
    /// Start a new decoder on `buf`. The initial C register is primed from the
    /// first two bytes as specified by E.3.5.
    pub fn new(buf: &'a [u8]) -> Self {
        let mut d = Self {
            a_reg: 0x8000,
            c_reg: 0,
            ct: 0,
            b: 0,
            buf,
            pos: 0,
        };
        // Spec E.3.5 says "B = first byte of coded data; C = B << 16" and then
        // BYTEIN. Setting B to buf[0] means the first BYTEIN takes the FF
        // branch whenever buf[0] is 0xFF, effectively consuming 7 bits of
        // buf[1] instead of 8. This matches what our encoder emits (the
        // encoder's FF_flag is set by the second MQ_ByteOut when the first
        // buffered byte happens to be 0xFF), so the round-trip stays
        // self-consistent.
        d.b = d.read_byte();
        d.c_reg = (d.b as u32) << 16;
        d.byte_in();
        d.c_reg <<= 7;
        d.ct -= 7;
        d
    }

    #[inline]
    fn read_byte(&mut self) -> u8 {
        if self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            self.pos += 1;
            b
        } else {
            // Past end: virtual 0xff 0xac tail (spec E.2.4). We return 0xff
            // repeatedly; the `byte_in` logic keeps the coder happy.
            0xFF
        }
    }

    #[inline]
    fn byte_in(&mut self) {
        if self.pos < self.buf.len() {
            if self.b == 0xFF {
                if self.buf[self.pos] > 0x8F {
                    // Marker / terminator: the reference MQ_ByteIn advances
                    // past the marker byte and leaves B as 0xFF while the
                    // renormaliser keeps pulling synthetic bits. Mirror that so
                    // `pos` is not sticky (which would trap subsequent
                    // byte_in calls re-reading the same marker byte forever).
                    self.pos += 1;
                    self.ct = 8;
                } else {
                    let nb = self.buf[self.pos];
                    self.pos += 1;
                    self.b = nb;
                    self.c_reg = self.c_reg.wrapping_add((nb as u32) << 9);
                    self.ct = 7;
                }
            } else {
                let nb = self.buf[self.pos];
                self.pos += 1;
                self.b = nb;
                self.c_reg = self.c_reg.wrapping_add((nb as u32) << 8);
                self.ct = 8;
            }
        } else {
            // Past end of stream: match the reference MQ_ByteIn behaviour,
            // which synthesises a 0xFF data byte (not a stuff byte) regardless
            // of the previous B value. This keeps the renormaliser pulling in
            // `0xFF00` units of Creg per ByteIn, exactly mirroring the spec's
            // virtual end-of-stream semantics (E.2.4).
            self.c_reg = self.c_reg.wrapping_add(0xFF00);
            self.ct = 8;
        }
    }

    /// Decode one binary decision in context `cx`.
    #[inline]
    pub fn decode(&mut self, cxs: &mut MqContexts, cx: usize) -> u8 {
        let state = cxs.get_mut(cx);
        let index = state.index() as usize;
        let mps_bit = state.mps_bit();
        let entry = QE_TABLE[index];
        let qe = entry.qe as u32;

        let c_high = (self.c_reg >> 16) & 0xFFFF;
        self.a_reg = self.a_reg.wrapping_sub(qe);

        let d: u8;
        if c_high < qe {
            if self.a_reg >= qe {
                // D1 — LPS
                self.a_reg = qe;
                d = mps_bit ^ 0x80;
                let mut ns = entry.nlps;
                if entry.switch {
                    ns |= mps_bit ^ 0x80;
                } else {
                    ns |= mps_bit;
                }
                *state = CxState(ns);
            } else {
                // D5 — MPS (conditional exchange)
                self.a_reg = qe;
                d = mps_bit;
                *state = CxState(entry.nmps | mps_bit);
            }
        } else {
            self.c_reg = self.c_reg.wrapping_sub(qe << 16);
            if self.a_reg & 0x8000 == 0 {
                if self.a_reg < qe {
                    // D2 — LPS
                    d = mps_bit ^ 0x80;
                    let mut ns = entry.nlps;
                    if entry.switch {
                        ns |= mps_bit ^ 0x80;
                    } else {
                        ns |= mps_bit;
                    }
                    *state = CxState(ns);
                } else {
                    // D4 — MPS
                    d = mps_bit;
                    *state = CxState(entry.nmps | mps_bit);
                }
            } else {
                // D3 — MPS
                d = mps_bit;
                // State unchanged.
            }
        }

        while self.a_reg & 0x8000 == 0 {
            if self.ct == 0 {
                self.byte_in();
            }
            self.a_reg <<= 1;
            self.c_reg <<= 1;
            self.ct -= 1;
        }

        if d == 0 {
            0
        } else {
            1
        }
    }

    /// Returns the position within the source buffer.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Debug helper: returns `(a_reg, c_reg, ct, b, pos)` for external
    /// tracing. Intended for diagnostics only; the shape of the tuple is
    /// not part of the stable API surface.
    #[doc(hidden)]
    pub fn state(&self) -> (u32, u32, i32, u8, usize) {
        (self.a_reg, self.c_reg, self.ct, self.b, self.pos)
    }

    /// Check that the decoder did not run far past the end of the buffer.
    /// Intended as a post-hoc sanity check.
    pub fn check_eof(&self, tolerance: usize) -> Jbig2Result<()> {
        if self.pos > self.buf.len() + tolerance {
            Err(Jbig2Error::ArithmeticDesync("decoder read past end"))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256PlusPlus;

    #[test]
    fn qe_table_has_47_entries_and_expected_shape() {
        assert_eq!(QE_TABLE.len(), 47);
        // Spot-check the reference-table-row translation.
        assert_eq!(QE_TABLE[0].qe, 0x5601);
        assert_eq!(QE_TABLE[0].nmps, 1);
        assert_eq!(QE_TABLE[0].nlps, 1);
        assert!(QE_TABLE[0].switch);
        assert_eq!(QE_TABLE[6].qe, 0x5601);
        assert!(QE_TABLE[6].switch);
        assert_eq!(QE_TABLE[46].qe, 0x5601);
        assert_eq!(QE_TABLE[46].nmps, 46);
        assert_eq!(QE_TABLE[46].nlps, 46);
        assert!(!QE_TABLE[46].switch);
    }

    fn round_trip(bits: &[u8]) {
        let mut enc_cx = MqContexts::new(16);
        let mut enc = MqEncoder::new(bits.len() / 8 + 4);
        for &b in bits {
            let cx = (b as usize) & 0xF; // spread across contexts
            enc.encode(&mut enc_cx, cx, b);
        }
        let out = enc.finish();

        let mut dec_cx = MqContexts::new(16);
        let mut dec = MqDecoder::new(&out);
        for (i, &b) in bits.iter().enumerate() {
            let cx = (b as usize) & 0xF;
            let d = dec.decode(&mut dec_cx, cx);
            assert_eq!(d, b, "mismatch at bit {i}");
        }
    }

    fn stable_bits() -> Vec<u8> {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(0x1234_5678_9ABC_DEF0);
        (0..256).map(|_| rng.gen_range(0..2u8)).collect()
    }

    fn random_decisions(seed: u64, len: usize, contexts: usize) -> Vec<(usize, u8)> {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);
        (0..len)
            .map(|_| (rng.gen_range(0..contexts), rng.gen_range(0..2u8)))
            .collect()
    }

    fn encode_decisions(decisions: &[(usize, u8)], capacity_hint: usize) -> Vec<u8> {
        let mut enc_cx = MqContexts::new(16);
        let mut enc = MqEncoder::new(capacity_hint);
        for &(cx, bit) in decisions {
            enc.encode(&mut enc_cx, cx, bit);
        }
        enc.finish()
    }

    fn round_trip_decisions(decisions: &[(usize, u8)], capacity_hint: usize) {
        let out = encode_decisions(decisions, capacity_hint);
        let mut dec_cx = MqContexts::new(16);
        let mut dec = MqDecoder::new(&out);
        for (i, &(cx, bit)) in decisions.iter().enumerate() {
            let d = dec.decode(&mut dec_cx, cx);
            assert_eq!(d, bit, "mismatch at decision {i}");
        }
    }

    #[test]
    fn round_trip_alternating_bits() {
        let v: Vec<u8> = (0..512u32).map(|i| (i & 1) as u8).collect();
        round_trip(&v);
    }

    #[test]
    fn round_trip_all_zeros() {
        round_trip(&vec![0u8; 4096]);
    }

    #[test]
    fn round_trip_all_ones() {
        round_trip(&vec![1u8; 4096]);
    }

    #[test]
    fn round_trip_random() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(0xDEAD_BEEF);
        let v: Vec<u8> = (0..8192).map(|_| rng.gen_range(0..2u8)).collect();
        round_trip(&v);
    }

    #[test]
    fn round_trip_biased() {
        // 95% zero, 5% one — exercises MPS/LPS state machine.
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(0xF00D_BABE);
        let v: Vec<u8> = (0..16384)
            .map(|_| if rng.gen_range(0..100u32) < 5 { 1 } else { 0 })
            .collect();
        round_trip(&v);
    }

    #[test]
    fn mq_termination_byte_count_is_stable() {
        let bits = stable_bits();
        let mut cxs = MqContexts::new(16);
        let mut enc = MqEncoder::new(bits.len() / 8 + 4);
        for &b in &bits {
            let cx = (b as usize) & 0xF;
            enc.encode(&mut cxs, cx, b);
        }
        let out = enc.finish();
        assert_eq!(out, vec![85, 255, 127, 240]);
    }

    #[test]
    fn round_trip_random_context_mix() {
        let decisions = random_decisions(0xA11C_E55E, 12_345, 16);
        round_trip_decisions(&decisions, 0);
    }
}
