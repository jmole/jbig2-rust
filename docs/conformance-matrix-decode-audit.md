# Conformance Matrix Decode Audit and Decisions

This document is a complete, evidence-first review of every cell in the
**decode phase** of the parallel conformance matrix produced by
`cargo run --bin conformance-matrix`. The encode phase has a separate audit
in `docs/conformance-matrix-encode-audit.md`; the two phases are split because
their workflows and the implications for what the matrix can prove are quite
different.

The intended reader is a future contributor or reviewer who needs to decide
whether a red cell in the decode row of the matrix is a release blocker, a
known limitation, or noise.

## 1. Why the decode phase exists, and what it can prove

JBIG2 (ITU-T T.88 / ISO/IEC 14492) is a small standard with a long tail of
implementation incompatibilities. The dominant production decoder is
Artifex's `jbig2dec`. The ITU reference C++ decoder shipped with T.88 is
old and fragile but is the only tool guaranteed to produce the BMPs that
the spec actually specifies. The Java decoder (`jbig2-imageio`) is widely
used in PDF pipelines. Consumers of `jbig2-rust` are typically reading
JBIG2 streams that were produced by some other tool, and they need our
decoder to behave the same way as those tools on real input.

The decode workflow is straightforward:

1. The harness takes a JBIG2 stream from the conformance corpus.
2. It runs that stream through every available decoder.
3. It compares the decoded pixels against a per-vector reference bitmap.

The oracle for the decode phase is a **set of BMP files** (one per page)
that ship with the T.88 conformance package, plus spec-derived BMP files for
the Artifex `annex-h` vector. These oracles are static, direct, and
independent of any running decoder. That is the key asymmetry with the encode
phase: in decode, the oracle is just a pile of bytes on disk, not another
implementation we have to trust. When a decoder disagrees with the oracle, we
know exactly what the right answer was supposed to be.

A green `OK` cell here proves "this decoder reads this stream and produces
the spec-defined pixels." A failure cell proves "this decoder either crashes
on this stream or produces different pixels than the spec." Triangulation
across decoders tells us whether the disagreement is a property of the
decoder or of the stream, which then determines whether the cell renders as
`ERR` (our `jbig2-rust` is at fault) or `BRKN` (a third-party decoder is at
fault); see the legend in 2.1.

## 2. Classification algorithm for decode cells

Every decode cell is classified using the same evidence-first procedure.
The decode-specific short form:

1. **Inventory.** Record row, column, decoder command path, oracle BMP path,
   tool version or vendor SHA, and rendered state
   (`OK`, `KI`, `WTF`, `BRKN`, `ERR`, `SKIP`, blank, `OK*`, `ERR!`; see 2.1).
2. **Validate the signal.** Confirm the cell actually compared decoded pixels
   to the oracle, not just "the decoder exited 0."
3. **Identify the JBIG2 feature under test.** Generic region, symbol
   dictionary, text region, refinement, multi-page, color segment, halftone,
   pattern dictionary, etc. Use the vector documentation, not pixel guesses.
4. **Triangulate.** Compare ITU reference, `jbig2dec`, `jbig2-rust`, and Java
   on the same column. If the ITU reference and at least one independent
   decoder agree against an outlier, the outlier is suspect.
5. **Decide consumer relevance.** Will real downstream consumers of
   `jbig2-rust` encounter streams of this shape? Streams shaped like
   `annex-h` are far more relevant than streams shaped like obscure
   conformance vectors.
6. **Assign a bucket.**
7. **Decide a repo action.**

The buckets, framed for decode:

- **Meaningful coverage.** Decoder reads the stream and produces oracle
  pixels. Cell renders as `OK`.
- **Bucket 1 - third-party decoder broken, not our problem.** A non-Rust
  decoder fails on a stream we can decode and the spec defines, but the
  failing decoder is not in any realistic consumer path for our library.
  Cell renders as `BRKN`. Useful as a third-party-quality signal; not
  catalogued as KI.
- **Bucket 1a - upstream Won't Fix.** A specialization of bucket 1: the
  third-party decoder fails *and* the upstream maintainer has explicitly
  declined to fix the defect (the canonical case here is a Ghostscript /
  Artifex bugzilla closed `RESOLVED WONTFIX`). Cataloging this as `WTF`
  with the bug URL in `evidence` keeps the matrix honest by attributing
  the persistent red cell to a maintenance posture rather than to
  unspecified breakage. Use `WTF` only when there is a clickable upstream
  citation; otherwise the cell stays `BRKN`.
- **Bucket 2 - third-party decoder broken, interop matters.** A non-Rust
  decoder fails on a stream consumers will plausibly hand to `jbig2dec`,
  Java, or another decoder downstream. Cell renders as `KI` once cataloged;
  uncataloged it shows as `BRKN`. Treat as important context for users
  picking a decoder.
- **Bucket 3 - our bug.** `jbig2-rust` crashes or mis-renders a stream the
  ITU reference and at least one third-party decoder accept. Cell renders
  as `ERR`. Must be fixed; never KI.
- **Low-value or invalid cell.** The cell does not actually test what the
  row/column suggests, or it duplicates another cell with no added evidence.

### 2.1 Cell legend

The summary table uses these states. The detailed per-cell line above the
summary always shows the underlying decoder message; the summary token is
just the rolled-up classification.

| Token  | Color         | Meaning                                                                  |
|--------|---------------|--------------------------------------------------------------------------|
| `OK`   | green         | Decoder produced the oracle pixels.                                      |
| `KI`   | yellow        | Cataloged third-party known issue (matches `tools/conformance/known-issues.ron`). |
| `WTF`  | bold magenta  | Won't Fix: cataloged third-party defect that the upstream maintainer has explicitly declined to fix (e.g. a Ghostscript/Artifex bugzilla `RESOLVED WONTFIX`). Strictly stronger than `BRKN`/`KI`: it requires both that the failure pattern matches the catalog *and* that `evidence` cites an upstream WONTFIX URL. |
| `BRKN` | orange        | Third-party decoder broke on this cell; not (yet) a cataloged KI or WTF. |
| `ERR`  | red           | `jbig2-rust` (or the harness) failed; ours to fix.                       |
| `SKIP` | gray          | Meaningful cell, but the decoder/feature could not be invoked (missing tool, unsupported adapter mode, etc.). |
| blank  | -             | No meaningful cell exists for this row/column (no oracle, structurally not applicable). |
| `OK*`  | cyan          | Cataloged KI/WTF unexpectedly passed; review the catalog.                |
| `ERR!` | red           | Cataloged KI/WTF failed differently than expected (drift); review.       |

**`KI` vs `WTF`.** `KI` says "we have triaged this and chosen to live with
it"; `WTF` says "the upstream maintainer has triaged it and chosen *not to
fix it*." Mechanically, both are produced by the same matcher in
`tools/conformance/known-issues.ron`; the difference is the `wontfix: true`
flag on the catalog entry, which the validator only accepts when `evidence`
contains an `http(s)://` link to an upstream WONTFIX record (or equivalent
maintainer statement of declined-to-fix). The intent is that `WTF` is never
a soft signal: every `WTF` cell must be backed by a citation a future
contributor can click on.

Throughout this document, when a cell is classified as a known third-party
defect, the upstream evidence is cited inline and is mirrored in
`tools/conformance/known-issues.ron`.

## 3. The shape of the decode matrix

**Columns.** Ten ITU-T T.88 conformance vectors (`TT1`-`TT10`) plus the
`annex-h.jbig2` vector vendored from Artifex. The TT vectors come from
`vendor/T-REC-T.88-201808/Software/JBIG2_ConformanceData-A20180829/`; each TT
vector has a per-page BMP oracle (`<vector>_TT00.bmp`, `_TT01.bmp`, ...) that
acts as the spec-defined ground truth. `annex-h.jbig2` uses the spec-folder
BMP oracle generated from Figure H.1 rather than live decoder output.

**Rows.**

- `system-binary` - the system-installed `jbig2dec` (version 0.20 from
  Homebrew on the developer's machine).
- `jbig2dec` - the vendored Artifex `jbig2dec` built from
  `vendor/jbig2dec` (a proper submodule pinned to
  `6ecb04980813d693234190021bd1cf874c05b1b4`).
- `itu-t88` - the reference C++ decoder built from
  `vendor/T-REC-T.88-201808`.
- `java` - `jbig2-imageio`, invoked through `JBIG2_IMAGEIO_CMD` when it is set.
- `rust` - our crate, called through `Jbig2Decoder` directly.

The redundancy between `system-binary` and `jbig2dec` exists for a specific
reason: it lets us catch the case where a developer's machine has a different
`jbig2dec` than the vendored submodule. Drift between the two would itself be
a finding. As of the current matrix, both rows agree exactly, so the cells
are duplicates of one another in terms of pixel results, but the duplication
is load-bearing and intentional.

```mermaid
flowchart TD
    tt["TT1-TT10 .jb2 vectors"] --> expected["T.88 BMP page oracles"]
    annex["annex-h.jbig2"] --> annexOracle["spec-derived Annex H BMP oracle"]

    tt --> sysDec["system-binary jbig2dec"]
    tt --> vendDec["vendored jbig2dec"]
    tt --> t88Dec["ITU-T.88 reference decoder"]
    tt --> javaDec["jbig2-imageio CLI"]
    tt --> rustDec["jbig2-rust decoder"]

    annex --> sysDec
    annex --> vendDec
    annex --> t88Dec
    annex --> javaDec
    annex --> rustDec

    sysDec --> compare["Compare decoded pages to oracle pixels"]
    vendDec --> compare
    t88Dec --> compare
    javaDec --> compare
    rustDec --> compare

    expected --> compare
    annexOracle --> compare
    compare --> decodeCell["Decode matrix cell: OK / KI / WTF / BRKN / ERR / SKIP / blank"]
```

## 4. Decode matrix, cell by cell

The current rendered state, copied verbatim from the harness:

```
                 TT1  TT2  TT3  TT4  TT5  TT6  TT7  TT8  TT9  TT10  annex-h
  system-binary  BRKN BRKN BRKN BRKN  WTF  WTF BRKN  KI    OK    OK       OK
  jbig2dec       BRKN BRKN BRKN BRKN  WTF  WTF BRKN  KI    OK    OK       OK
  itu-t88          OK   OK   OK   OK   OK   OK   OK  OK    OK    OK     BRKN
  java           BRKN BRKN BRKN BRKN BRKN BRKN BRKN BRKN   OK    OK       OK
  rust            ERR   OK   OK   OK  ERR   OK   OK  OK    OK    OK      ERR
```

`OK` means the decoder produced the oracle pixels. `BRKN` is a third-party
decoder breakage we have not (yet) cataloged. `KI` is a cataloged known
issue we are choosing to live with. `WTF` is a cataloged third-party defect
that the upstream maintainer has explicitly closed `RESOLVED WONTFIX` (or
equivalent) - mechanically the same matcher as `KI`, but with a stronger
attribution. `ERR` is `jbig2-rust` (or the harness) failing on its own.
`SKIP` means the decoder's wrapper could not run that cell at all. See the
legend in 2.1.

### 4.1 The ITU reference row is the ground truth

`itu-t88` passes every TT vector. That is by construction: the BMP oracles
shipped with the conformance data were generated by this same reference
decoder, so they will agree. The single failure in this row, `annex-h`,
is discussed in 4.5.

The value of the `itu-t88` row in the matrix is not "did the standard pass."
It is "did anyone build the reference decoder out of T.88 sources today, and
does it still produce the bitmaps the spec promises." That is a regression
guard against vendor drift in `vendor/T-REC-T.88-201808`. As long as this row
is green on TT1-TT10, we trust the BMP oracles for every other row.

**Classification:** Meaningful coverage for TT1-TT10. The `annex-h` cell is
discussed in section 4.5.

### 4.2 The two `jbig2dec` rows agree, and stderr failures are now honest

`system-binary` and `jbig2dec` produce identical pixel output for every
column. That is the redundancy paying off: the system Homebrew build and the
vendored submodule build of `jbig2dec` agree on these vectors. The two rows
exist to detect the day when they stop agreeing.

Within those identical results, there are three groups of failures. The
harness now preserves `jbig2dec` stderr diagnostics, so fatal upstream decode
errors render as `FAIL(...)` instead of misleading row mismatches when
`jbig2dec` exits 0 after writing a partial PBM.

**TT1, TT3, TT4, TT7: pixel mismatches after nominal decode.** TT1, TT3, and
TT4 all show first-row pixel disagreement; TT7 disagrees at row 0. These
remain real pixel comparisons, not setup failures. The ITU reference decoder
passes all four TT vectors, while `jbig2dec` is the outlier in the
cross-decoder comparison. `rust / TT1` now agrees with the oracle, so TT1 is
no longer evidence against the Rust decoder.

**TT2, TT5, TT6: fatal upstream decode diagnostics.** Each of these prints a
stable `jbig2dec` diagnostic that the harness now reports directly:

- **TT2** prints `FATAL ERROR zeroes code in MMR-coded data` - `jbig2dec`'s
  MMR Huffman path miscounts a code at the start of a run. The output PBM
  is mostly blank because the coder gives up early. We have not found an
  upstream record explicitly declining to fix this; an active, currently
  unconfirmed report covers a related MMR memory-safety bug
  (https://bugs.ghostscript.com/show_bug.cgi?id=709207, "jbig2dec:
  heap-buffer-overflow read + signed integer overflow in jbig2_mmr.c").
  Until upstream takes a position, this stays in `BRKN`, not `WTF`.
- **TT5** prints `FATAL ERROR refinement references unknown symbol 0`
  (preceded by the giveaway warning `exporting more symbols than available
  (4294967280 > 2), capping`). `jbig2dec`'s refinement-aggregate
  symbol-dictionary path drops the symbols it just decoded when
  `REFAGGNINST > 1`, so the next refinement reference resolves to symbol id
  0 and the decoder bails. Artifex closed this report
  `RESOLVED WONTFIX` in 2017 and the defect has remained in shipping
  releases ever since (https://bugs.ghostscript.com/show_bug.cgi?id=695737,
  "jbig2_decode_symbol_dict: newly decoded symbols missing in dictionary
  used for REFAGGNINST > 1 decoding"). Cell renders as `WTF` on both rows.
- **TT6** prints `FATAL ERROR OOB obtained when decoding symbol instance
  refinement data`, reached through a text region with symbol refinement.
  This is the same upstream defect as TT5, exposed via the text-region
  path (see Bug 695737, plus the related symbol/text-region tickets
  https://bugs.ghostscript.com/show_bug.cgi?id=695736 and
  https://bugs.ghostscript.com/show_bug.cgi?id=695741, both also closed
  `WONTFIX`). Cell renders as `WTF` on both rows.

In each case the ITU reference decoder agrees pixel-for-pixel with the BMP
oracle, and `jbig2-rust` also agrees with the BMP oracle. So `jbig2dec` is the
outlier under triangulation, and for TT5/TT6 the outlier is "outlier by
upstream policy."

**TT8: known limitation, already a strict KI.** `jbig2dec` does not implement
AMD3 colour-palette page segments and prints
`page segment indicates use of color segments (NYI)`. This is a documented
upstream limitation, not a regression. It already lives in
`tools/conformance/known-issues.ron` with a vendor pin and a citation to
`vendor/jbig2dec/jbig2_page.c:124`. The system row carries the same KI for
the same reason.

**TT9, TT10:** clean pass. These are the simpler vectors that exercise only
the parts of the standard `jbig2dec` actually implements correctly.

**Classification.**

- TT8 (both rows): Bucket 2 KI - documented upstream NYI; the cell is real
  evidence that consumers using `jbig2dec` for color streams will be unable
  to decode them, which matters for our interop story. Already cataloged.
- TT5, TT6 (both rows): Bucket 1a `WTF` - upstream Won't Fix.
  Bug 695737 (and the companion tickets 695736 / 695741) was closed
  `RESOLVED WONTFIX` by Artifex; the defect manifests on every shipping
  `jbig2dec` we test against. Cataloging as `WTF` rather than `BRKN`
  attributes the persistent red cell to maintainer policy and pins the
  bugzilla URL in `tools/conformance/known-issues.ron` so the citation
  cannot quietly rot.
- TT1, TT3, TT4, TT7 (both rows): Bucket 1 - reproducible upstream defect, but our
  encoder does not emit these specific text-region constructs by default and
  no realistic consumer pipeline depends on `jbig2dec` decoding the ITU
  conformance vectors specifically. Not a release blocker. Worth keeping in
  the matrix as a third-party-quality signal and as a regression detector for
  any future `jbig2dec` upgrade.
- TT2 (both rows): Bucket 1 - reproducible upstream defect. We have not yet
  found an explicit upstream WONTFIX statement attached to this exact
  diagnostic, only a closely related currently-unconfirmed report
  (Bug 709207). Stays `BRKN` until we have either a citation worth the
  upgrade to `WTF` or evidence that upstream will fix it.

We deliberately do not catalog TT1-TT4 / TT7 jbig2dec failures as KIs. The
strict bar for KI requires that the failure be in code we cannot fix and
that its absence would be a meaningful interop signal. Here, the failures
are noise from third-party decoders on contrived spec vectors, not
consumer-facing breakages. TT5/TT6 *do* clear a different bar - the `WTF`
bar - because we have direct upstream evidence that the failures will
persist indefinitely, and that fact is itself worth surfacing in the
matrix.

### 4.3 The `rust` row is now green

`rust` is green on TT1-TT10 and `annex-h`. The three former product bugs were
real decoder defects and are fixed:

- **`rust / TT1`** failed on page 3 while decoding segment 17, an
  `SDREFAGG=1` symbol dictionary. The nested aggregate text region used the
  running symbol-table length as `SBSYMCODELEN`; T.88 6.5.8.2.3 fixes that
  code width to `ceil(log2(NUMINSYMS + SDNUMNEWSYMS))` for the whole
  dictionary. After fixing that, the stream exposed the ITU reference encoder's
  already-documented malformed `IAEX(0), IAEX(0)` export shortcut, so the
  existing export fallback now also handles malformed IAEX runs rather than
  only physical end-of-body.
- **`rust / TT5`** was a 24-pixel mismatch in the two instances of the refined
  symbol, not a full-page polarity inversion. The root cause was the generic
  refinement context order: our decoder used the `jbig2dec`/ImageIO private
  context layout, but the T.88 conformance streams were authored by the ITU
  sample encoder. `src/segments/refinement_region.rs` now mirrors
  `Jb2_MQLapper.cpp::CX_RefEncode`.
- **`rust / annex-h`** failed in page-1 segment 3, the Huffman text region. The
  Artifex fixture uses the normative 7.4.3.1.5 symbol-ID Huffman table
  expansion, while the old decoder only implemented the ITU sample-decoder
  shortcut needed by TT1. The text-region decoder now tries the normative table
  first and falls back to the sample-decoder shortcut.

**Classification:** all three are now Bucket 0 (`OK`) for `jbig2-rust`; the
third-party rows remain classified independently.

### 4.4 The `java` row is real cross-decoder evidence

The Java decoder row resolves through `JBIG2_IMAGEIO_CMD`. When the
configured wrapper only decodes the first page, multi-page streams render
as `SKIP` as a deliberate adapter limitation, not an unconfigured-tool
condition.

`TT9`, `TT10`, and `annex-h` pass, so the Java path is not a dead harness
path and now provides independent positive evidence for the spec-derived Annex
H oracle. `TT1` fails with an `EOFException` while reading the third page.
`TT2` through `TT8` fail inside `org.apache.pdfbox.jbig2.segments.TextRegion`
with `IndexOutOfBoundsException: Index 0 out of bounds for length 0` from
`TextRegion.decodeIb` and render as `BRKN`. That is useful independent signal:
Java agrees that the simpler vectors are readable, but its text-region path
fails on the same family of conformance vectors where `jbig2dec` also shows
upstream weaknesses. It is not evidence against `jbig2-rust`, because the ITU
reference and Rust decoder both successfully decode most of those vectors.

**Classification:** Meaningful coverage for `TT9`, `TT10`, and `annex-h`.
Bucket 1 (`BRKN`) for `TT1` through `TT8`: diagnosed third-party decoder
limitations, useful as an interop/weather-vane row, but not a `jbig2-rust`
release blocker.

### 4.5 `itu-t88 / annex-h`: process crash

The ITU reference C++ decoder crashes on the Artifex `annex-h.jbig2` vector.
That vector uses pattern dictionaries, halftone regions, immediate-lossless
variants, and a multi-segment structure that the 2018-vintage reference C++
decoder was never built to handle robustly. This is a known fragility of the
ITU reference codebase rather than a property of `annex-h`: `jbig2dec` and
`system-binary` decode the same file without complaint, and the Java ImageIO
adapter now also decodes the first two oracle pages successfully.

**Classification:** Bucket 1. Reproducible third-party limitation in code that
nobody ships, in a vector authored by Artifex specifically. Keep visible as a
guard against ever recommending the ITU reference decoder for production.
Strict KI is justifiable but optional; the failure mode is a stable process
crash (`SIGBUS` in the latest full matrix run).

### 4.6 Decode summary table

The "State" column is the rendered token from the harness summary; the
"Action" column is what we plan to do with that cell.

| Cell                              | State  | Action                          |
|-----------------------------------|--------|---------------------------------|
| `itu-t88` / TT1-TT10              | `OK`   | Keep. Oracle ground truth.      |
| `system-binary` / TT9, TT10       | `OK`   | Keep. Cross-decoder evidence.   |
| `jbig2dec` / TT9, TT10            | `OK`   | Keep. Vendor-pinned baseline.   |
| `system-binary` / `annex-h`       | `OK`   | Keep.                           |
| `jbig2dec` / `annex-h`            | `OK`   | Keep. Vendor-pinned baseline.   |
| `system-binary` / TT8             | `KI`   | Already cataloged.              |
| `jbig2dec` / TT8                  | `KI`   | Already cataloged.              |
| `system-binary` / TT1, TT3, TT4   | `BRKN` | Keep visible; do not KI.        |
| `jbig2dec` / TT1, TT3, TT4        | `BRKN` | Keep visible; do not KI.        |
| `system-binary` / TT2             | `BRKN` | Keep visible; do not KI.        |
| `jbig2dec` / TT2                  | `BRKN` | Keep visible; do not KI.        |
| `system-binary` / TT5             | `WTF`  | Cataloged: upstream WONTFIX (Bug 695737). |
| `jbig2dec` / TT5                  | `WTF`  | Cataloged: upstream WONTFIX (Bug 695737). |
| `system-binary` / TT6             | `WTF`  | Cataloged: upstream WONTFIX (Bug 695737 / 695736 / 695741). |
| `jbig2dec` / TT6                  | `WTF`  | Cataloged: upstream WONTFIX (Bug 695737 / 695736 / 695741). |
| `system-binary` / TT7             | `BRKN` | Keep visible; do not KI.        |
| `jbig2dec` / TT7                  | `BRKN` | Keep visible; do not KI.        |
| `itu-t88` / `annex-h`             | `BRKN` | Keep; KI optional.              |
| `rust` / TT1-TT10, `annex-h`      | `OK`   | Keep. Real spec + interop coverage. |
| `java` / TT9, TT10, `annex-h`     | `OK`   | Keep. Independent decoder pass. |
| `java` / TT1-TT8                  | `BRKN` | Keep visible; do not KI yet.    |

## 5. Final groupings

### 5.1 Meaningful tests we keep as-is

- `itu-t88` decode row TT1-TT10 (oracle continuity).
- `system-binary` decode TT9, TT10, `annex-h` (cross-decoder evidence).
- `jbig2dec` decode TT9, TT10, `annex-h` (vendor-pinned baseline).
- `java` decode TT9, TT10, and `annex-h` (independent ImageIO decoder evidence).
- `rust` decode TT1-TT10 and `annex-h` (real spec + interop coverage).

### 5.2 Cataloged known issues (Bucket 2 / `KI`)

- `system-binary` / `TT8` and `jbig2dec` / `TT8`: AMD3 colour-palette segments
  unimplemented in `jbig2dec`. Vendor pin: `vendor/jbig2dec@6ecb04980813`.
  Already in `tools/conformance/known-issues.ron`.

### 5.2a Cataloged upstream Won't-Fix (Bucket 1a / `WTF`)

- `system-binary` / `TT5` and `jbig2dec` / `TT5`: refinement-aggregate
  symbol-dictionary path drops the symbols it just decoded when
  `REFAGGNINST > 1`, surfacing as `FATAL ERROR refinement references unknown
  symbol 0`. Closed `RESOLVED WONTFIX` upstream
  (https://bugs.ghostscript.com/show_bug.cgi?id=695737). Vendor pin:
  `vendor/jbig2dec@6ecb04980813`.
- `system-binary` / `TT6` and `jbig2dec` / `TT6`: same upstream defect as
  TT5, exposed via a text region with symbol refinement
  (`FATAL ERROR OOB obtained when decoding symbol instance refinement
  data`). Companion tickets
  https://bugs.ghostscript.com/show_bug.cgi?id=695736 and
  https://bugs.ghostscript.com/show_bug.cgi?id=695741 are also closed
  WONTFIX. Vendor pin: `vendor/jbig2dec@6ecb04980813`.

### 5.3 Third-party noise we keep visible without cataloging (Bucket 1)

- `system-binary` and `jbig2dec` on TT1, TT2, TT3, TT4, TT7 (five distinct
  upstream defects in `jbig2dec`'s arithmetic, MMR, and extended-template
  paths). Worth keeping in the matrix as a third-party-quality signal and as
  a regression detector for any future `jbig2dec` upgrade. Not a release
  blocker, not catalogued as KI under the strict bar; TT2 may be eligible
  for promotion to `WTF` once Bug 709207 (or a sibling MMR ticket) reaches
  a definitive upstream resolution.
- `java` on TT1-TT8. TT1 fails with `EOFException`; TT2-TT8 fail inside
  `TextRegion.decodeIb` with the same `IndexOutOfBoundsException` shape.
  Useful cross-decoder evidence, but not a release blocker for `jbig2-rust`
  because the ITU reference and Rust agree on most of the same vectors.
- `itu-t88 / annex-h` process crash on Artifex multi-feature stream.
  Optional KI on the crash fingerprint.

### 5.4 Product bugs in `jbig2-rust` to fix (Bucket 3)

No decode product bugs remain in the current matrix. The former TT1, TT5, and
`annex-h` cells are documented in the fix log below and covered by regression
tests.

### 5.5 Harness/oracle bugs to fix

No remaining decode-harness/oracle issue is identified in this audit pass. The
Java adapter now covers multi-page streams, and the Annex H oracle is a
spec-derived BMP snapshot rather than live decoder output.

### 5.6 Low-value or invalid cells to consider trimming

- The `system-binary` and `jbig2dec` decode rows are pixel-identical for
  every cell. They are not redundant in intent (one is a vendor pin, the
  other is "what your machine has"), but the duplication doubles the
  rendered output without doubling the information. Worth keeping for
  reproducibility, but consider collapsing the rendered table when both
  rows agree (e.g. show the system row with a footnote when it diverges
  from the vendored row).

## 6. Rust decoder fix log

1. **TT1 page 3 / RefAGG:** segment 17 (`SDREFAGG=1`, body `757..789`) uses a
   nested aggregate text region inside the symbol dictionary. The fix uses the
   dictionary-wide `SBSYMCODELEN` from T.88 6.5.8.2.3 and extends the
   reference-encoder IAEX fallback to malformed export runs.
2. **TT5 / refined symbol:** the final diff was 24 pixels across the two
   placements of the refined symbol. The fix changes refinement context
   construction to match the ITU sample encoder/decoder context order in
   `Jb2_MQLapper.cpp::CX_RefEncode`.
3. **annex-h page 1 / Huffman text:** page-1 segment 3 is the failing segment.
   The fix implements the normative 7.4.3.1.5 symbol-ID Huffman table
   expansion and keeps the ITU sample shortcut as a compatibility fallback.

## 7. Repo actions

### 7.1 Open as tracked bugs in `jbig2-rust`

No open decode-conformance product bugs remain.

### 7.2 Fix in the harness

No immediate harness fix remains from this pass.

### 7.3 Catalog or trim

1. (Optional) add a strict KI for `itu-t88 / annex-h` based on the stable
   process-crash fingerprint.
2. (Optional) collapse the duplicated `system-binary` / `jbig2dec` rendered
   rows when they agree, keeping the underlying drift check intact.

### 7.4 Keep as-is

Everything in section 5.1 and the existing entry in section 5.2 stay where
they are. The decode matrix is doing the job it was designed for: it makes
silent decoder bugs loud and lets us classify them with evidence rather
than guesses.
