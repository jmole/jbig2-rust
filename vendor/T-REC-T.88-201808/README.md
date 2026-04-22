# ITU-T T.88 (2018) reference material

This directory holds the sample software and conformance test vectors that
ship alongside ITU-T Recommendation T.88 (2018). The `Software/` subtree is
redistributed here under the licence granted in `Software/Copyright Notice.txt`,
which explicitly permits reproduction and distribution "for the limited
purposes of ... including the Software in a conforming implementation of this
ITU Recommendation [and] determining whether your implementation conforms to
this ITU Recommendation." That is exactly how this crate uses it (see
`tests/conformance.rs`).

## Recommendation text

This folder includes a copy of ITU-T Recommendation T.88 (08/2018):

  ITU-T T.88 (08/2018)
  Information technology – Lossy/lossless coding of bi-level images
  Citation / canonical source: https://handle.itu.int/11.1002/1000/13688

Source page:
  https://www.itu.int/ITU-T/recommendations/rec.aspx?id=13688

Copyright:
  © ITU. All rights reserved.

This copy is provided for reference and attribution purposes. The
authoritative source for the Recommendation is the ITU publication page
and handle above.

The local copy lives in `spec/ITU-T_T_88__08_2018.{pdf,docx,md}`.

## Folder layout

- `Software/JBIG2_SampleSoftware-A20180829/` — reference C/C++ encoder + decoder.
- `Software/JBIG2_ConformanceData-A20180829/` — `.jb2` streams + reference
  `.bmp` outputs used by `tests/conformance.rs` (TT1..TT10).
- `Software/Copyright Notice.txt` — redistribution licence for the above.
- `spec/` (gitignored) — local copy of the Recommendation text.
- `SUMMARY.md` — project-level notes on the above.
