# Color Encoder Roadmap

## Status today

`jbig2-rust` can decode AMD3 color text regions, but it cannot encode color.
The decoder path returns an `RgbBitmap` for color text regions in
`src/segments/text_region.rs`, and `DecodedPage` exposes that as
`rgb_bitmap`.

The encoder path in `src/encoder.rs` is mono-only. It accepts `Bitmap`,
constructs generic regions or symbol/text-region streams, and has no
`RgbBitmap` ingestion path or color-extension emission path.

That is why the encode matrix treats `codeStreamTest3` carefully:

- `rust:*`, `jbig2enc:*`, and `system-binary:*` remain blank on
  `codeStreamTest3`.
- `itu-t88:default / codeStreamTest3` is blank because the default ITU profile
  emits mono from a color source.
- `itu-t88:Param8.ini / codeStreamTest3` stays live because it is the ITU
  color profile. It proves our decoder can read freshly encoded AMD3 color
  reference output, but it does not prove our encoder can produce color.

## Why this is worth doing

Color encoding closes the symmetry gap with the decoder. Today we can consume
AMD3 color text-region streams but cannot produce one from an RGB input. That
means there is no `rust:* / codeStreamTest3` encode row and no self-roundtrip
test for color content.

It would also be a real product differentiator. `jbig2enc` is effectively a
mono document encoder, while the ITU reference software is academic sample
code. A small, well-scoped color encoder would make `jbig2-rust` more useful
for archival and PDF workflows that need colored symbol overlays, stamps, or
limited-palette document content.

## Scope of color encoding

JBIG2 color is not a generic RGB page encoding mode. The color slice we care
about is T.88 AMD3: a color extension applied to text regions. The stream
describes a small palette and assigns color information to symbol instances.

The reference profile is `jbig2_Param8.ini`:

```text
-txt -Param -numInst 5 -ID 0 5 0 -ID 1 5 8 -RefID 0 Sym000.bmp 7 16 0 -1 -ID 1 5 23 -ID 0 5 31
-txt -Param -ColorExt 5 255 0 0 0 0 255 0 255 0 128 0 255 255 0 255
```

That is the target shape for a first implementation: text-region color over a
small palette. Generic color regions are out of scope; JBIG2 does not have a
separate generic RGB region segment equivalent to its 1-bpp generic region.

## Encoder work breakdown

1. **Add an explicit color mode.** Introduce something like
   `Mode::SymbolColor` or a separate encoder entry point so existing mono
   presets keep their current contracts.
2. **Accept RGB input.** Add an `RgbBitmap` ingestion path that can find
   symbol-shaped components and associate each instance with a palette color.
3. **Derive or accept a palette.** The first pass can require a caller-supplied
   palette or derive a fixed small palette from the source. More ambitious
   palette learning can wait.
4. **Classify symbols by shape and color.** Extend the symbol classifier so
   "same shape, different color" does not explode the dictionary unless the
   mode asks for color-separated symbols.
5. **Emit AMD3 color extension data.** Write the color palette and per-instance
   color data alongside the text region segment, mirroring the shape exercised
   by `Param8.ini`.
6. **Roundtrip strictly.** The encode matrix should compare RGB-to-RGB exactly
   for lossless color modes and report pixel diffs for explicitly lossy color
   modes.

## Test surface this unlocks

- A `rust:symbol_color / codeStreamTest3` row in the encode matrix with
  `v88=OK rt=0/N` for the first lossless color target.
- Encoder unit tests in `src/encoder.rs` that construct a small RGB symbol
  page and roundtrip it through `Jbig2Decoder`.
- Future interop rows in `docs/encoder-decoder-interop.md` once color decoder
  support is measured across ITU, `jbig2dec`, Java, and PDF renderers.

## Hard parts and open questions

- **Palette derivation.** Should the encoder require a user-supplied palette,
  use the exact unique colors in the source when small enough, or learn a
  palette with a quantizer such as k-means? `Param8.ini` uses a hand-authored
  five-color palette.
- **Color-aware classification.** The current classifier is shape-oriented.
  We need a policy for same shape / different color: one dictionary symbol
  with colored instances, or separate dictionary symbols when color carries
  semantic meaning.
- **Lossless contract.** Bit-exact RGB roundtrip is only feasible when the
  source can be represented by the chosen palette and text-region model.
  Larger photographic or antialiased RGB inputs are inherently lossy under
  this approach.
- **Interop reality.** Some decoders ignore or reject AMD3 color. The encoder
  can still be spec-correct, but product docs should be explicit about which
  downstream stacks preserve color.

## Out of scope

This document does not implement color encoding. It records the scope and
tradeoffs so the current encode-matrix work can keep mono-only rows blank
without losing the future color plan.
