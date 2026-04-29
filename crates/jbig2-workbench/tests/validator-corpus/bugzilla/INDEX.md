# Ghostscript Bugzilla JBIG2 corpus

Bugs harvested from `bugs.ghostscript.com` (`product=jbig2dec` plus `Ghostscript` bugs whose summary contains `jbig2`).
Selection prioritises crashes, sanitizer reports, and integer-overflow / out-of-bounds bugs that document a concrete spec-violation vector.
All attachments were downloaded via the Bugzilla REST API and were never decoded by any JBIG2 implementation in this workspace.

## Layout

The corpus is split by whether a real JBIG2 stream is present on disk:

- `harvested/<id>/` — directory contains `stream.jb2`, `meta.toml`, and `expected.toml`. These fixtures get full regression coverage (validator + decoder rows in `corpus-validator`).
- `tracked/<id>/` — directory contains `meta.toml` only. The original attachment was a PDF, zip, or other container and we deliberately do not extract embedded streams in this harvest. The bug is tracked here as documentation; consult the Bugzilla URL for the original.

The regression test in [`tests/validator_corpus_regression.rs`](../../validator_corpus_regression.rs) asserts that every directory under `harvested/` contains a `stream.jb2`, so the directory shape and the actual coverage cannot drift apart.

## Catalog

| Bug | Title | Attachment | sha256(8) | Clause guess | Layout | Bug status |
|---|---|---|---|---|---|---|
| [690596](https://bugs.ghostscript.com/show_bug.cgi?id=690596) | memleak with jbig2 images | `jbig-decode-error.pdf` | `bbda4f6e` | unknown | tracked | RESOLVED FIXED |
| [690723](https://bugs.ghostscript.com/show_bug.cgi?id=690723) | Using unallocated memory when parsing image... | `globals.jbig2` | `222df4a9` | 7.3.1 | harvested | RESOLVED FIXED |
| [690923](https://bugs.ghostscript.com/show_bug.cgi?id=690923) | Wrong text positioning when symbol instance coordinates are transposed | `042_19.jb2` | `a445f751` | 7.4.3.1.4 | harvested | RESOLVED FIXED |
| [691958](https://bugs.ghostscript.com/show_bug.cgi?id=691958) | jbig2_complete_page doesn't check if there's a page at all | `jbig2dec crash.pdf` | `cb11a97f` | 7.4.8 | tracked | RESOLVED FIXED |
| [693025](https://bugs.ghostscript.com/show_bug.cgi?id=693025) | various crashes and leaks | `missing JBIG2Globals.pdf` | `ed6aeb5e` | 7.3.1 | tracked | RESOLVED FIXED |
| [693285](https://bugs.ghostscript.com/show_bug.cgi?id=693285) | NULL pointer dereferences | `jbig2 null segments and glyphs.pdf` | `83331ae7` | unknown | tracked | RESOLVED FIXED |
| [695225](https://bugs.ghostscript.com/show_bug.cgi?id=695225) | Heap-buffer-overflow in jbig2_decode_symbol_dict | `090514_jbig2.pdf` | `4ca3b2ad` | 7.4.2 | tracked | RESOLVED FIXED |
| [695315](https://bugs.ghostscript.com/show_bug.cgi?id=695315) | SEGV in jbig2_decode_symbol_dict | `malformed.pdf` | `eae36cc7` | 7.4.2 | tracked | RESOLVED FIXED |
| [696052](https://bugs.ghostscript.com/show_bug.cgi?id=696052) | Segfault (null pointer access) when trying to open malformed inputs | `jbig2_decode_refinement_template0_unopt-jbig2_image_get_pixel-nullptr.jb2` | `9c48911f` | 7.4.7 | harvested | RESOLVED FIXED |
| [696453](https://bugs.ghostscript.com/show_bug.cgi?id=696453) | Out of bounds heap read on malformed input in jbig2_image_compose() | `jbig2dec-oob-heap-read-jbig2_image_compose.jbig2` | `d1d7cce7` | 6.2.5.3 | harvested | RESOLVED WONTFIX |
| [697595](https://bugs.ghostscript.com/show_bug.cgi?id=697595) | Fuzzing. | `jbig2dec-oob-heap-read-jbig2_image_compose.jbig2` | `d1d7cce7` | 6.2.5.3 | harvested | RESOLVED WORKSFORME |
| [698776](https://bugs.ghostscript.com/show_bug.cgi?id=698776) | jbig2dec fails to parse "immediate generic region" segment with unspecified length | `bug5200_p1_I1.jbig2` | `accb5a65` | 7.4.6.4 | harvested | RESOLVED FIXED |
| [699083](https://bugs.ghostscript.com/show_bug.cgi?id=699083) | Leak in symbol dictionary parsing upon error | `6434-80b59185030368fecf38d9abe13ffb0302a60c2a.pdf` | `f75f297c` | 7.4.2 | tracked | RESOLVED FIXED |
| [705238](https://bugs.ghostscript.com/show_bug.cgi?id=705238) | jbig2dec complete system hang when used in mupdf | `MuPDF_system_hang` | `939a2b19` | Annex E | tracked | UNCONFIRMED |
| [707438](https://bugs.ghostscript.com/show_bug.cgi?id=707438) | jbig2_image_new integer overflow causing high memory using | `jbig2_high_memory_using.zip` | `b8abe94e` | 6.2 | tracked | UNCONFIRMED |
| [708722](https://bugs.ghostscript.com/show_bug.cgi?id=708722) | Vulnerability Report: Uncontrolled Memory Allocation in jbig2_hd_new | `oom_min` | `212b548c` | 7.4.4 | harvested | UNCONFIRMED |
| [708791](https://bugs.ghostscript.com/show_bug.cgi?id=708791) | integer overflow vulnerability | `poc.jb2` | `a9dee794` | 7.4 | harvested | RESOLVED DUPLICATE |
| [709025](https://bugs.ghostscript.com/show_bug.cgi?id=709025) | Integer overflow in jbig2_find_changing_element of jbig2dec v0.20. | `poc` | `ce659369` | Annex G | harvested | UNCONFIRMED |
| [709026](https://bugs.ghostscript.com/show_bug.cgi?id=709026) | Integer overflow in jbig2_decode_pattern_dict leads to unbounded memory allocation of jbig2dec v0.20. | `poc` | `6a1f539d` | 7.4.4 | harvested | UNCONFIRMED |
| [709027](https://bugs.ghostscript.com/show_bug.cgi?id=709027) | Integer overflow in jbig2_get_int32 of jbig2dec v0.20. | `poc` | `11a86879` | 7.2.4 | harvested | UNCONFIRMED |
| [709028](https://bugs.ghostscript.com/show_bug.cgi?id=709028) | Integer overflow in jbig2_arith_int_decode of jbig2dec v0.20. | `poc` | `1de6bcb1` | Annex A.2 | harvested | UNCONFIRMED |
| [709207](https://bugs.ghostscript.com/show_bug.cgi?id=709207) | jbig2dec: heap-buffer-overflow read + signed integer overflow in jbig2_mmr.c | `jbig2_mmr_poc.zip` | `cd2d94a7` | Annex G | tracked | UNCONFIRMED |
| [709213](https://bugs.ghostscript.com/show_bug.cgi?id=709213) | jbig2dec: overflow check uses wrong variable in jbig2_image_resize() (jbig2_image.c:112) — DoS/heap overflow | `jbig2_image_poc.zip` | `6c22f0fe` | 6.2.5 | tracked | UNCONFIRMED |
| [709214](https://bugs.ghostscript.com/show_bug.cgi?id=709214) | jbig2_text.c: off-by-one in NINSTANCES check allows extra symbol instance (line 294) | `poc_jbig2_text.pdf` | `88599a83` | 7.4.3.1.7 | tracked | UNCONFIRMED |
| [709222](https://bugs.ghostscript.com/show_bug.cgi?id=709222) | jbig2dec: signed integer overflow in refinement region coordinate arithmetic (jbig2_refinement.c) | `jbig2_refinement_poc.zip` | `44005712` | 7.4.7 | tracked | UNCONFIRMED |

## Layout legend

- `harvested` — directory under `harvested/<id>/` contains `stream.jb2`, `meta.toml`, and `expected.toml`. Stream may still be a malformed/embedded JBIG2 fragment; check `embedded` and `attachment_format` in `meta.toml`.
- `tracked` — directory under `tracked/<id>/` contains `meta.toml` only. The attachment is a PDF, zip, image, or other container. The original bytes are not stored in the corpus directory; consult the Bugzilla URL for the original. (We deliberately do not extract embedded JBIG2 streams from PDFs in this harvest because the safe extraction path requires running PDF tooling we are not authorised to invoke here.)
- `attachment-missing` — selected bug has no usable attachment in Bugzilla. (None in this batch.)
