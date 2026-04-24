//! MQ integer coders (`IA*`) and symbol-ID coder (`IAID`) from Annex A.
//!
//! The same arithmetic coder is reused for every IA* family; the family is
//! identified by the 512-slot base index (`IADH`, `IADW`, ...) that gets added
//! to each per-bit context before touching the shared [`MqContexts`] pool.
//!
//! The encoder accepts any signed integer that fits in `i32`, plus a distinct
//! out-of-band value [`OOB`] used for end-of-strip and end-of-row markers in
//! text-region coding. Values outside `[-2^31 + 4436 + 1 .. 2^31 - 4436 - 1]`
//! are rejected by [`encode_integer`] as [`Jbig2Error::OutOfRange`].
//!
//! Port reference: `Jb2_MQLapper.cpp::MQ_EncInteger` / `MQ_DecInteger`.

use crate::coding::mq::{MqContexts, MqDecoder, MqEncoder};
use crate::error::{Jbig2Error, Jbig2Result};

/// Sentinel out-of-band value for IA* integer streams.
///
/// Chosen to match the reference C code's `OOB = 0x7FFF_FFFF` constant. The
/// encoder rejects ordinary values equal to `OOB` so the decoder can
/// unambiguously recover them.
pub const OOB: i32 = i32::MAX;

/// Result of decoding one integer. `None` means the decoder saw an OOB
/// marker.
pub type DecodedInt = Option<i32>;

/// Encode a signed integer into `enc` using the IA* tree with base context
/// `base`.
///
/// Writing out-of-band is done by passing `i32::MAX` (see [`OOB`]).
pub fn encode_integer(
    enc: &mut MqEncoder,
    cxs: &mut MqContexts,
    base: usize,
    value: i32,
) -> Jbig2Result<()> {
    // OOB has its own tiny prefix tree.
    if value == OOB {
        enc.encode(cxs, base + 0x01, 1);
        enc.encode(cxs, base + 0x03, 0);
        enc.encode(cxs, base + 0x06, 0);
        enc.encode(cxs, base + 0x0C, 0);
        return Ok(());
    }

    // Table A.1 buckets (spec Annex A.3): split magnitude into ranges
    // [0..3], [4..19], [20..83], [84..339], [340..4435], and [4436..].
    // Negative values use the symmetric tree with `S = 1`.
    let (sign, abs) = if value < 0 {
        (1u8, -(value as i64) as u64)
    } else {
        (0u8, value as i64 as u64)
    };

    enc.encode(cxs, base + 0x01, sign);

    // Walk a trie of the 6 prefix bits while simultaneously figuring out the
    // magnitude bucket and the mantissa bit count.
    let (prefix_bits, mant_bits, bias) = if abs <= 3 {
        (&[0u8][..], 2u32, 0u64)
    } else if abs <= 19 {
        (&[1, 0][..], 4, 4)
    } else if abs <= 83 {
        (&[1, 1, 0][..], 6, 20)
    } else if abs <= 339 {
        (&[1, 1, 1, 0][..], 8, 84)
    } else if abs <= 4435 {
        (&[1, 1, 1, 1, 0][..], 12, 340)
    } else if abs <= (u32::MAX as u64 + 4436) {
        (&[1, 1, 1, 1, 1][..], 32, 4436)
    } else {
        return Err(Jbig2Error::OutOfRange("IA* integer magnitude exceeds 32 bits"));
    };

    // Encode the prefix bits through the binary trie starting from CX=2 (for
    // sign=0) or CX=3 (for sign=1). Subsequent prefix bits use
    // CX = (CX<<1)|bit, capped into the 0x100..0x1ff band after hitting 0xff.
    let mut n_cx: u32 = 2 + sign as u32;
    for (k, &bit) in prefix_bits.iter().enumerate() {
        enc.encode(cxs, base + n_cx as usize, bit);
        if k + 1 < prefix_bits.len() || mant_bits > 0 {
            n_cx = advance_cx(n_cx, bit);
        }
    }

    // Encode the mantissa bits (MSB first) of `abs - bias` in `mant_bits`
    // binary digits, continuing through the trie.
    let v = abs.wrapping_sub(bias);
    for k in 0..mant_bits {
        let bit = ((v >> (mant_bits - 1 - k)) & 1) as u8;
        enc.encode(cxs, base + n_cx as usize, bit);
        n_cx = advance_cx(n_cx, bit);
    }

    Ok(())
}

#[inline]
fn advance_cx(mut n_cx: u32, bit: u8) -> u32 {
    if n_cx > 0xFF {
        n_cx = (((n_cx << 1) | bit as u32) & 0x1FF) | 0x100;
    } else {
        n_cx = (n_cx << 1) | bit as u32;
    }
    n_cx
}

/// Decode one signed integer from `dec`. Returns `None` if the decoder saw
/// the out-of-band marker.
pub fn decode_integer(
    dec: &mut MqDecoder<'_>,
    cxs: &mut MqContexts,
    base: usize,
) -> DecodedInt {
    let mut n_cx: u32 = 1;
    // S
    let s = dec.decode(cxs, base + n_cx as usize);
    n_cx = (n_cx << 1) | s as u32;
    // First magnitude bit
    let d0 = dec.decode(cxs, base + n_cx as usize);
    n_cx = (n_cx << 1) | d0 as u32;

    let (mant_bits, bias) = if d0 == 0 {
        (2u32, 0u64)
    } else {
        let d1 = dec.decode(cxs, base + n_cx as usize);
        n_cx = (n_cx << 1) | d1 as u32;
        if d1 == 0 {
            (4u32, 4u64)
        } else {
            let d2 = dec.decode(cxs, base + n_cx as usize);
            n_cx = (n_cx << 1) | d2 as u32;
            if d2 == 0 {
                (6u32, 20u64)
            } else {
                let d3 = dec.decode(cxs, base + n_cx as usize);
                n_cx = (n_cx << 1) | d3 as u32;
                if d3 == 0 {
                    (8u32, 84u64)
                } else {
                    let d4 = dec.decode(cxs, base + n_cx as usize);
                    n_cx = (n_cx << 1) | d4 as u32;
                    if d4 == 0 {
                        (12u32, 340u64)
                    } else {
                        (32u32, 4436u64)
                    }
                }
            }
        }
    };

    let mut v: u64 = 0;
    for _ in 0..mant_bits {
        let bit = dec.decode(cxs, base + n_cx as usize);
        n_cx = advance_cx(n_cx, bit);
        v = (v << 1) | bit as u64;
    }
    v = v.wrapping_add(bias);

    if s == 1 {
        if v == 0 {
            None
        } else {
            // Safe: v is in [1 .. 2^32 + 4436]; wrapping cast gives us back the
            // original negative value because we code absolute magnitude.
            Some((v as i64 * -1) as i32)
        }
    } else {
        Some(v as i32)
    }
}

/// Encode a fixed-length symbol ID using the IAID family.
///
/// `code_length` is the number of bits per symbol ID (usually ceil(log2(N))
/// where N is the number of symbols in the referenced dictionaries).
pub fn encode_iaid(
    enc: &mut MqEncoder,
    cxs: &mut MqContexts,
    base: usize,
    code_length: u32,
    value: u32,
) {
    let mut n_cx: u32 = 1;
    let mask: u32 = (1u64 << (code_length + 1)).saturating_sub(1) as u32;
    for k in 0..code_length {
        let bit = ((value >> (code_length - 1 - k)) & 1) as u8;
        enc.encode(cxs, base + n_cx as usize, bit);
        n_cx &= mask;
        n_cx = (n_cx << 1) | bit as u32;
    }
}

/// Decode a fixed-length symbol ID using the IAID family.
pub fn decode_iaid(
    dec: &mut MqDecoder<'_>,
    cxs: &mut MqContexts,
    base: usize,
    code_length: u32,
) -> u32 {
    let mut n_cx: u32 = 1;
    let mask: u32 = (1u64 << (code_length + 1)).saturating_sub(1) as u32;
    for _ in 0..code_length {
        let bit = dec.decode(cxs, base + n_cx as usize);
        n_cx &= mask;
        n_cx = (n_cx << 1) | bit as u32;
    }
    n_cx & ((1u32 << code_length).wrapping_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::mq::MQ_NUM_CONTEXTS;

    const IADH: usize = 0x1_0200;

    fn round_trip_values(values: &[i32]) {
        let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
        let mut enc = MqEncoder::new(32);
        for &v in values {
            encode_integer(&mut enc, &mut cxs, IADH, v).unwrap();
        }
        let buf = enc.finish();

        let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
        let mut dec = MqDecoder::new(&buf);
        for (i, &v) in values.iter().enumerate() {
            let d = decode_integer(&mut dec, &mut cxs, IADH);
            if v == OOB {
                assert!(d.is_none(), "expected OOB at index {i}, got {d:?}");
            } else {
                assert_eq!(d, Some(v), "mismatch at index {i}");
            }
        }
    }

    #[test]
    fn integers_round_trip_small() {
        round_trip_values(&[0, 1, 2, 3, -1, -2, -3]);
    }

    #[test]
    fn integers_round_trip_buckets() {
        round_trip_values(&[0, 3, 4, 19, 20, 83, 84, 339, 340, 4435, 4436, 10_000]);
        round_trip_values(&[-3, -4, -19, -20, -83, -84, -339, -340, -4435, -4436, -10_000]);
    }

    #[test]
    fn integers_round_trip_oob_mixed() {
        round_trip_values(&[0, OOB, 7, OOB, -42, OOB, OOB, 4436, -4436, 100, -100]);
    }

    #[test]
    fn iaid_round_trip() {
        let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
        let mut enc = MqEncoder::new(16);
        let length = 9;
        let values = [0u32, 1, 2, 7, 255, 256, 511];
        for &v in &values {
            encode_iaid(&mut enc, &mut cxs, 0x1_0E00, length, v);
        }
        let buf = enc.finish();

        let mut cxs = MqContexts::new(MQ_NUM_CONTEXTS);
        let mut dec = MqDecoder::new(&buf);
        for &v in &values {
            let got = decode_iaid(&mut dec, &mut cxs, 0x1_0E00, length);
            assert_eq!(got, v);
        }
    }
}
