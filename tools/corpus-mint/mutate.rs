//! Deterministic targeted mutator for validator regression fixtures.
//!
//! `mutate::run` walks a small set of known-good seed streams (currently the
//! Annex H sample) and applies a deterministic, seed-driven schedule of
//! single-bit, single-byte, and DLEN-perturbation mutations. Every mutation is
//! validated with `Lens::StrictT88` and persisted under
//! `tests/validator-corpus/mutated/` together with the validator's actual
//! finding set as `expected.toml`.
//!
//! This module never invokes any decoder or external tool. Sanitizer-aware
//! C-decoder runs are the responsibility of `corpus-validator`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use jbig2::validator::corpus::Expected;
use jbig2::validator::{validate, Lens};
use sha2::{Digest, Sha256};

const RNG_SEED: u64 = 0x4242_4242_4242_4242;
const PER_SCHEDULE_PERSIST_CAP: usize = 32;

const SEEDS: &[(&str, &str)] = &[("annex-h", "vendor/jbig2dec/annex-h.jbig2")];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
enum Schedule {
    BitFlip,
    ByteReplace,
    DlenPerturb,
}

impl Schedule {
    fn as_str(self) -> &'static str {
        match self {
            Self::BitFlip => "bit-flip",
            Self::ByteReplace => "byte-replace",
            Self::DlenPerturb => "dlen-perturb",
        }
    }
}

pub fn run(root: &Path) -> Result<()> {
    let out_root = root.join("tests/validator-corpus/mutated");
    if out_root.exists() {
        fs::remove_dir_all(&out_root).ok();
    }
    fs::create_dir_all(&out_root)?;

    let mut index = String::new();
    index.push_str("# Targeted validator mutator corpus\n\n");
    index.push_str(&format!(
        "Deterministic seed: 0x{RNG_SEED:016x}; per-schedule persist cap: {PER_SCHEDULE_PERSIST_CAP}\n\n"
    ));
    index.push_str("| Seed | Schedule | Generated | Persisted |\n|---|---|---|---|\n");

    for (seed_name, seed_path) in SEEDS {
        let path = root.join(seed_path);
        let bytes =
            fs::read(&path).with_context(|| format!("failed to read seed {}", path.display()))?;
        let mut sha = Sha256::new();
        sha.update(&bytes);
        let seed_sha = format!("{:x}", sha.finalize());

        let mut rng = SmallRng::new(RNG_SEED ^ stable_hash(seed_name));

        for schedule in [
            Schedule::BitFlip,
            Schedule::ByteReplace,
            Schedule::DlenPerturb,
        ] {
            let cases = generate(&bytes, schedule, &mut rng);
            let mut buckets: BTreeMap<String, usize> = BTreeMap::new();
            let mut persisted = 0;
            for case in &cases {
                let report = validate(&case.bytes, Lens::StrictT88);
                let primary_id = report
                    .findings
                    .iter()
                    .find(|f| f.severity == jbig2::validator::Severity::Error)
                    .or_else(|| report.findings.first())
                    .map(|f| f.check_id.as_str().to_string())
                    .unwrap_or_else(|| "clean".to_string());
                let entry = buckets.entry(primary_id.clone()).or_insert(0);
                if *entry >= 4 || persisted >= PER_SCHEDULE_PERSIST_CAP {
                    *entry += 1;
                    continue;
                }
                *entry += 1;
                persisted += 1;

                let dir = out_root
                    .join(seed_name)
                    .join(schedule.as_str())
                    .join(format!("{}-{:04}", sanitize(&primary_id), case.case_id));
                fs::create_dir_all(&dir)?;
                fs::write(dir.join("stream.jb2"), &case.bytes)?;
                let meta = format!(
                    "seed_name = {:?}\nseed_sha256 = {:?}\nschedule = {:?}\nrng_seed = {:#018x}\ncase_id = {}\nmutation = {:?}\n",
                    seed_name,
                    seed_sha,
                    schedule.as_str(),
                    RNG_SEED,
                    case.case_id,
                    case.mutation,
                );
                fs::write(dir.join("meta.toml"), meta)?;
                let check_ids = report
                    .findings
                    .iter()
                    .map(|f| f.check_id.as_str().to_string())
                    .collect::<Vec<_>>();
                Expected::validator(primary_id.clone(), check_ids)
                    .write(&dir.join("expected.toml"))
                    .with_context(|| {
                        format!("write expected metadata for {}", dir.display())
                    })?;
            }

            index.push_str(&format!(
                "| `{}` | `{}` | {} | {} |\n",
                seed_name,
                schedule.as_str(),
                cases.len(),
                persisted
            ));
        }
    }

    fs::write(out_root.join("INDEX.md"), index)
        .with_context(|| "failed to write mutator INDEX.md")?;

    Ok(())
}

#[derive(Clone, Debug)]
struct MutatedCase {
    case_id: usize,
    bytes: Vec<u8>,
    mutation: String,
}

fn generate(seed: &[u8], schedule: Schedule, rng: &mut SmallRng) -> Vec<MutatedCase> {
    match schedule {
        Schedule::BitFlip => sample_bit_flips(seed, rng, 64),
        Schedule::ByteReplace => sample_byte_replacements(seed, rng, 64),
        Schedule::DlenPerturb => sample_dlen_perturbations(seed, rng),
    }
}

fn sample_bit_flips(seed: &[u8], rng: &mut SmallRng, count: usize) -> Vec<MutatedCase> {
    let mut out = Vec::with_capacity(count);
    for case_id in 0..count {
        let byte_offset = (rng.next_u64() as usize) % seed.len();
        let bit_index = (rng.next_u64() & 0b111) as u8;
        let mut bytes = seed.to_vec();
        bytes[byte_offset] ^= 1 << bit_index;
        out.push(MutatedCase {
            case_id,
            bytes,
            mutation: format!("bit-flip byte={byte_offset} bit={bit_index}"),
        });
    }
    out
}

fn sample_byte_replacements(seed: &[u8], rng: &mut SmallRng, count: usize) -> Vec<MutatedCase> {
    let mut out = Vec::with_capacity(count);
    for case_id in 0..count {
        let byte_offset = (rng.next_u64() as usize) % seed.len();
        let new_value = (rng.next_u64() & 0xFF) as u8;
        let mut bytes = seed.to_vec();
        if bytes[byte_offset] == new_value {
            bytes[byte_offset] = new_value.wrapping_add(1);
        } else {
            bytes[byte_offset] = new_value;
        }
        out.push(MutatedCase {
            case_id,
            bytes,
            mutation: format!("byte-replace byte={byte_offset} value=0x{new_value:02x}"),
        });
    }
    out
}

/// Find every segment-header DLEN field (the 4 bytes that immediately precede
/// the segment data) and produce one perturbation per segment.
fn sample_dlen_perturbations(seed: &[u8], rng: &mut SmallRng) -> Vec<MutatedCase> {
    let dlen_offsets = locate_dlen_offsets(seed);
    let mut out = Vec::with_capacity(dlen_offsets.len() * 2);
    let mut case_id = 0usize;
    for offset in dlen_offsets {
        for delta in [
            (rng.next_u64() & 0xF) as i32 + 1,
            -((rng.next_u64() & 0xF) as i32 + 1),
        ] {
            if offset + 4 > seed.len() {
                continue;
            }
            let original = u32::from_be_bytes([
                seed[offset],
                seed[offset + 1],
                seed[offset + 2],
                seed[offset + 3],
            ]);
            let new_value = original.wrapping_add(delta as u32);
            if new_value == original {
                continue;
            }
            let mut bytes = seed.to_vec();
            bytes[offset..offset + 4].copy_from_slice(&new_value.to_be_bytes());
            out.push(MutatedCase {
                case_id,
                bytes,
                mutation: format!(
                    "dlen-perturb offset={offset} delta={delta} new=0x{new_value:08x}"
                ),
            });
            case_id += 1;
        }
    }
    out
}

/// Walk the file structure to enumerate the byte offset of each DLEN field.
///
/// This duplicates a thin slice of the validator parser (rather than re-using
/// it) on purpose: the mutator needs to know the locations even when the
/// validator would otherwise complain about the surrounding context.
fn locate_dlen_offsets(seed: &[u8]) -> Vec<usize> {
    let mut offsets = Vec::new();
    if seed.len() < 13 {
        return offsets;
    }
    let flags = seed[8];
    let unknown_pages = flags & 0b10 != 0;
    let mut cursor = if unknown_pages { 9 } else { 13 };

    while cursor + 6 < seed.len() {
        let header_start = cursor;
        cursor += 4;
        let _seg_flags = seed[cursor];
        cursor += 1;
        let ref_byte = seed[cursor];
        cursor += 1;
        let count_high = ref_byte >> 5;
        let ref_count = if count_high <= 4 {
            count_high as u32
        } else if count_high == 7 {
            if cursor + 4 > seed.len() {
                return offsets;
            }
            let n = u32::from_be_bytes([
                seed[cursor],
                seed[cursor + 1],
                seed[cursor + 2],
                seed[cursor + 3],
            ]);
            cursor += 4;
            let retain_bytes = ((n + 8) / 8) as usize;
            cursor += retain_bytes;
            n
        } else {
            return offsets;
        };

        let ref_field_size = if ref_count > 0 {
            let segment_number = u32::from_be_bytes([
                seed[header_start],
                seed[header_start + 1],
                seed[header_start + 2],
                seed[header_start + 3],
            ]);
            if segment_number <= 0xFF {
                1
            } else if segment_number <= 0xFFFF {
                2
            } else {
                4
            }
        } else {
            0
        };
        cursor += (ref_count as usize) * ref_field_size;

        cursor += if header_start + 4 < seed.len() && seed[header_start + 4] & 0x40 != 0 {
            4
        } else {
            1
        };
        if cursor + 4 > seed.len() {
            break;
        }
        offsets.push(cursor);
        let dlen = u32::from_be_bytes([
            seed[cursor],
            seed[cursor + 1],
            seed[cursor + 2],
            seed[cursor + 3],
        ]);
        cursor += 4;
        if dlen == 0xFFFF_FFFF {
            break;
        }
        cursor += dlen as usize;
    }
    offsets
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn stable_hash(s: &str) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for byte in s.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Tiny xorshift64 RNG to keep the binary reproducible without dragging in a
/// dependency just for fixture minting.
struct SmallRng {
    state: u64,
}

impl SmallRng {
    fn new(seed: u64) -> Self {
        let state = if seed == 0 {
            0xDEAD_BEEF_DEAD_BEEF
        } else {
            seed
        };
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}
