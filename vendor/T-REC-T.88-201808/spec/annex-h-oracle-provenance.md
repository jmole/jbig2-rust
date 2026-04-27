# Annex H Figure H.1 Oracle Provenance

- Source image: `vendor/T-REC-T.88-201808/spec/annex-h-oracle-source.png` (relative to repo root)
- Spec figure: ITU-T T.88 Annex H, Figure H.1 (`Test datastream page bitmap`)
- Output pages: `annex-h-page-00.bmp`, `annex-h-page-01.bmp`
- Page dimensions: `64 x 56`
- Crop bounds: `top=1`, `bottom=1680`
- Grid bounds: `left=1`, `right=1946`, `top=1`, `bottom=1680`
- Cell spacing: `x=30.390625`, `y=29.982143`
- Sampling: median luminance over `9 x 9` windows at logical cell centers
- Classification: `median_luma < 128` maps to black/ink (`1`)
- BMP palette: index 0 = white, index 1 = black
- Packed logical raster SHA-256: `975e63be32f6dd9c4367dd25ae268cd5701b888717656236c98c31ee8bb35db4`
- Note: Annex H states that pages 1 and 2 decode to identical bitmaps; `annex-h-page-01.bmp` is intentionally duplicated from `annex-h-page-00.bmp`.

The generated BMPs are spec-derived from the Figure H.1 grid. `jbig2dec` output may be used as a diagnostic cross-check, but it is not the source of this oracle.
