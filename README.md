# jbig2-rust

Pure-Rust JBIG2 (ITU-T T.88 / ISO/IEC 14492) encoder and decoder.

This crate is a correctness-first workbench for JBIG2 streams: parser,
decoder, encoder, validator, conformance tooling, corpus regression, and an
optional `image` crate integration point. It is pre-1.0 and not yet published
to crates.io (`publish = false`).

## Goals

- Correctness against the T.88 format, including the uncommon paths that tend
  to break decoders: refinement, Huffman text, MMR, pattern dictionaries,
  halftone regions, and AMD3 colour palettes.
- Safety on hostile inputs. The Rust decode path is safe Rust, and external C
  or Java tools used as oracles run inside the repository sandbox.
- Reproducible evidence. The repository keeps deterministic corpus tests,
  conformance tooling, and documentation for known gaps and future fuzz work.

## Status

| Spec area | Decoder | Encoder |
| --- | --- | --- |
| File header (D.4) and segment headers (7.2) | ✅ | ✅ |
| Page information and end-of-page (7.4.8, 7.4.9) | ✅ | ✅ |
| Generic region arithmetic, templates 0..3 (6.2.5) | ✅ | ✅ |
| Generic region TPGD duplicate-line removal | ✅ | ✅ |
| Generic region extended 12-AT template (AMD2) | ✅ | ✅ |
| Generic region MMR (T.6) | ✅ | ✅ |
| Symbol dictionary arithmetic (7.4.2) | ✅ | ✅ |
| Symbol dictionary Huffman | ✅ | [unsupported](#symbol-dictionary-huffman-encoder) |
| Text region arithmetic, no refinement (7.4.3) | ✅ | ✅ |
| Text region Huffman and refinement | ✅ | ✅ |
| Connected-component extraction and identity classifier | n/a | ✅ |
| Lossy classifier (WXOR, size-bucket acceleration) | n/a | ✅ |
| Generic refinement (6.3, 7.4.7) | ✅ | ✅ |
| Pattern dictionary and halftone (7.4.4, 7.4.5) | ✅ | ✅ |
| Colour palette (AMD3) | ✅ | [unsupported](#colour-palette-amd3-encoder) |

The decoder side covers the T.88 TT1..TT9 conformance streams exercised by the
repository tests. Encoding still prefers the arithmetic paths for some features
where the decoder accepts both arithmetic and Huffman variants.

### Symbol dictionary Huffman encoder

T.88 lets a symbol dictionary be coded with either MQ arithmetic or Huffman
tables. The decoder accepts both (TT1..TT3 force the Huffman path), but an
encoder only needs to pick one to produce a spec-correct stream. We emit the
arithmetic variant via `Mode::SymbolLossless` because:

- Arithmetic coding compresses better. Huffman SD coding exists in the
  standard mainly for low-power fax hardware that can't afford arithmetic
  coding, which doesn't apply here.
- The decoder side already covers Huffman SD streams produced by other
  encoders, so there is no interop gap to close.
- A Huffman SD encoder would also need to grow user-defined Huffman table
  emission and the Huffman + refinement/aggregation sub-modes (see
  `src/segments/symbol_dictionary.rs`) for a path that is strictly worse than
  what we already produce.

### Colour palette (AMD3) encoder

AMD3 is a narrow color extension to text regions: a small palette plus
per-symbol-instance color indices. The decoder consumes AMD3 color text
regions end-to-end (`src/segments/text_region.rs` returns an `RgbBitmap`),
but the encoder cannot yet produce one. Reasons it is deferred rather than
dropped:

- AMD3 is not a generic RGB page mode. Useful inputs are limited to content
  representable by a small palette; photographic or antialiased RGB is
  inherently lossy under this model.
- Encoding requires several new pipeline pieces: a palette quantizer, a
  classifier that handles "same shape, different color" without exploding
  the dictionary, and AMD3 extension emission alongside the text region.
- Downstream consumer support is patchy — for example `jbig2dec` prints
  `page segment indicates use of color segments (NYI)` (see
  `docs/known-conformance-issues.md`).
- Color decode is already protected by a regression: `Param8.ini /
  codeStreamTest3` runs ITU's encoder over color input and round-trips
  RGB-to-RGB through `Jbig2Decoder` (see
  `docs/conformance-matrix-encode-audit.md`).

The forward plan lives in `docs/color-encoder-roadmap.md`.

## Correctness Evidence

- `cargo test --workspace` runs unit, integration, conformance, corpus, and
  validator checks.
- `.github/workflows/ci.yml` enforces formatting, clippy, the full test suite,
  and the tier-0 strict corpus shim.
- `.github/workflows/nightly.yml` runs the fuller tier-1 corpus path against
  vendored external decoders.
- `tools/conformance` compares against `jbig2dec`, `jbig2enc`, the ITU sample
  codec, and `jbig2-imageio`.
- `tests/validator-corpus/bugzilla/harvested/` contains hostile streams
  harvested from real decoder bugs.
- `benchmark-corpus/patent-500/` feeds refinement regression tests.

## Safety Posture

The library parser and codec paths are safe Rust. Safe modules use
`#![forbid(unsafe_code)]`, and the crate denies unsafe operations in unsafe
functions. The remaining `unsafe` in `src/` is isolated to
`src/util/sandbox.rs`, where it wraps Unix process-control calls such as
`setrlimit`, `setsid`, and `kill`.

Every untrusted external decoder or encoder invocation used by tests and tools
goes through `jbig2::util::sandbox`. The sandbox adds wall-clock watchdogs,
CPU/address-space/file-size/open-file/core limits, output caps, and
filesystem/network isolation via `bwrap` on Linux or `sandbox-exec` on macOS
when available. See `docs/sandbox.md` for the exact contract and fallback
behavior.

For private reports, see `SECURITY.md`.

## Features

- `image` (default): install an `image = "0.25"` decoding hook for JBIG2.
- `mmr` (default): enable T.4 / T.6 line-coding paths.
- `rayon`: reserved for parallel classifier and multi-page encode work.
- `cli`: build the `jbig2` command-line encoder / decoder / info tool.
- `validator-cli`: build the structural validator CLI.
- `validator-corpus`: build corpus minting and corpus validation tools.
- `conformance-tools`: build the conformance matrix tooling.
- `sandbox-runtime` (default): enable the external-process sandbox support.

MSRV is Rust 1.75. The license is `MIT` (see `LICENSE`).

## Example

```rust
use std::fs::File;
use std::io::{BufReader, BufWriter};

use jbig2::{EncoderConfig, Jbig2Decoder, Jbig2Encoder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut decoder = Jbig2Decoder::new(BufReader::new(File::open("input.jb2")?))?;
    let page = decoder.decode_page(1)?;

    let output = File::create("output.jb2")?;
    let mut encoder = Jbig2Encoder::new(
        BufWriter::new(output),
        EncoderConfig::max_compression(),
    );
    encoder.write_page(&page.bitmap)?;
    encoder.finish()?;
    Ok(())
}
```

## Useful Docs

- `docs/sandbox.md`: external-binary sandbox contract.
- `docs/01-corpus-drift-guards.md`: corpus drift protection.
- `docs/03-corpus-ci-goals.md`: corpus CI tiers.
- `docs/04-fuzz-strategy.md`: fuzzing strategy.
- `docs/05-external-decoder-taxonomy.md`: external decoder taxonomy.
- `docs/known-conformance-issues.md`: known conformance gaps.
- `docs/validator-reference.md`: structural validator reference.

## License

Licensed under the MIT license ([LICENSE](LICENSE) or
http://opensource.org/licenses/MIT).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be licensed under the MIT license,
without any additional terms or conditions.
