# ITU-T T.88 (2018) reference material

This directory holds the sample software and conformance test vectors that
ship alongside ITU-T Recommendation T.88 (2018). The `Software/` subtree is
redistributed here under the licence granted in `Software/Copyright Notice.txt`,
which explicitly permits reproduction and distribution "for the limited
purposes of ... including the Software in a conforming implementation of this
ITU Recommendation [and] determining whether your implementation conforms to
this ITU Recommendation." That is exactly how this crate uses it (see
`tests/conformance.rs`).

## Spec text — not redistributed

The Recommendation text itself (`spec/ITU-T_T_88__08_2018.{pdf,docx,md}`) is
**not** committed to this repository. Its redistribution terms are not
granted by the Software copyright notice above, so contributors should
download a personal copy directly from ITU-T:

  https://www.itu.int/rec/T-REC-T.88

and drop it into `vendor/T-REC-T.88-201808/spec/` locally. That path is
`.gitignore`d on purpose.

## Folder layout

- `Software/JBIG2_SampleSoftware-A20180829/` — reference C/C++ encoder + decoder.
- `Software/JBIG2_ConformanceData-A20180829/` — `.jb2` streams + reference
  `.bmp` outputs used by `tests/conformance.rs` (TT1..TT10).
- `Software/Copyright Notice.txt` — redistribution licence for the above.
- `spec/` (gitignored) — local copy of the Recommendation text.
- `SUMMARY.md` — project-level notes on the above.
