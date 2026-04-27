# 05 — External decoder taxonomy

This document motivates consolidating the project's three vocabularies
for "what a decoder did with a stream" into a single shared
classification. It is the canonical reference for any future plan that
extracts an `external_decoders` module shared by `corpus-validator`
and the conformance matrix, and that adds a Bugzilla column to the
matrix.

The intended reader is a maintainer who has just spent twenty minutes
translating between three legends to figure out whether a single
upstream change broke something we care about. The point of this doc
is to argue that they should not have to.

## Sequence

This is **step 5 of 5** in the corpus harness rollout — the last one.

1. [`01-corpus-drift-guards.md`](01-corpus-drift-guards.md)
2. [`02-sandbox-preflights.md`](02-sandbox-preflights.md)
3. [`03-corpus-ci-goals.md`](03-corpus-ci-goals.md)
4. [`04-fuzz-strategy.md`](04-fuzz-strategy.md)
5. **`05-external-decoder-taxonomy.md` — you are here.**

**Prerequisites.** Step 2 at minimum. The consolidation extracts a
shared `Verdict` / `Disposition` module across `corpus-validator` and
the conformance matrix; doing it before step 2's preflights are in
place would force the new module to either embed or paper over the
silent-vendor-drift failure mode. Step 4 is not strictly required,
but the taxonomy should anticipate fuzz-derived fixtures (verdicts
like `RejectErr` against minimized inputs) so the eventual fuzz
triage tooling slots in cleanly.

**Why this comes last.** Consolidation is a refactor whose value is
proportional to how much behaviour each consumer already has. Doing
it before steps 1–4 is premature abstraction: the consolidated enum
would be guessing at variants the consumers had not yet generated
real evidence for. Doing it after the consumers are exercised in
anger means the consolidation is a mechanical extraction rather than
a design exercise.

**Strict ordering rule.** This step delivers the matrix Bugzilla
column block. That block must reuse the same legend as the existing
TT* block — adding a parallel legend is the failure mode this doc
exists to prevent. Any PR adding the Bugzilla column without first
landing the shared `Verdict` module is doing the work in the wrong
order.

**Unblocks.** Nothing further in this rollout. Step 5 is the
finishing-line consolidation; future plans extending the harness
(Java decoder verdicts, differential fuzzing, additional matrix
columns) build on top of the consolidated taxonomy rather than
extending it as part of the rollout.

## The fragmentation, today

Three artefacts in this repo each describe decoder outcomes in their
own vocabulary:

1. The conformance matrix. `OK` / `KI` / `WTF` / `BRKN` / `ERR` /
   `SKIP` / `OK*` / `ERR!`. The legend lives at the top of
   [`docs/conformance-matrix-decode-audit.md`](conformance-matrix-decode-audit.md)
   §2.1 and is hand-classified by maintainers reading
   [`tools/conformance/known-issues.ron`](../tools/conformance/known-issues.ron).
2. The corpus validator. `OK` / `EXIT` / `SAN` / `CRASH` / `TIMEOUT`
   / `SPAWN`, plus the higher-level `Verdict` (`Ok` / `RejectErr` /
   `RejectOk` / `Crash` / `Timeout` / `Unknown`) introduced in
   Phase 1 and defined in
   [`src/validator/corpus.rs`](../src/validator/corpus.rs).
3. The sandbox. `KillReason::Timeout` / `AddressSpace` /
   `OutputBytes` (see
   [`src/util/sandbox.rs`](../src/util/sandbox.rs)).

The three are not unrelated. A `KillReason::Timeout` from the sandbox
maps cleanly to a `corpus-validator` `TIMEOUT` classification, which
in turn corresponds to a matrix `ERR!` cell. But each artefact does
that mapping in its own code, sometimes inconsistently, and a reader
crossing artefact boundaries has to translate every time.

## Why the fragmentation hurts

The cost is concrete enough that we have already paid it. A reviewer
investigating a `vendor/jbig2dec` bump today opens
[`tools/conformance/known-issues.ron`](../tools/conformance/known-issues.ron)
to see whether matrix cells changed category, then opens
`target/corpus-report.md` to see whether sandbox rollups changed, and
mentally has to keep three legends straight: was a `BRKN` cell that
turned `OK` actually fixed by the bump, or did the matrix's "expected
failure substring" coincidentally fall out of the new build's stderr?
The answer is in two different reports under two different vocabularies
and the reviewer is the only translator.

That cost compounds. As Phase 1 lands, the corpus matrix grows a
per-fixture per-impl table on top of the existing per-impl rollup. The
matrix audit grows a Bugzilla column, separate from but visually
parallel to the existing TT* columns. The reviewer count of legends to
juggle goes up, not down, unless we deliberately consolidate.

The user-facing failure mode is not a bug — the harness still works —
it is **erosion of trust**: a reviewer who has to translate between
vocabularies for every report stops fully reading any of them. That is
the worst possible outcome for a regression net whose value depends on
maintainers actually looking at the output.

## What success looks like

A reviewer reading the conformance matrix and a reviewer reading
`target/corpus-report.md` see the same legend, the same cataloging,
and the same vendor-pin annotations. They do not have to translate.
Concretely:

- **One `Verdict` enum.** Lives in a single module, used by every
  artefact that classifies decoder behaviour. The Phase 1
  `Verdict` (`Ok` / `RejectErr` / `RejectOk` / `Crash` / `Timeout`
  / `Unknown`) is a reasonable starting point; the consolidated
  enum may grow variants for matrix-only states (`KI`, `WTF`,
  `BRKN`) or it may keep `Verdict` as the runtime classification
  and add a separate `Disposition` enum for the
  human-classified-after-the-fact bucket. Both shapes are sensible;
  the next plan should pick one and stick with it.
- **One known-issues catalog.** `tools/conformance/known-issues.ron`
  becomes available to `corpus-validator`. A Bugzilla fixture that
  the matrix already classifies `WTF` (upstream WONTFIX) is the
  same `WTF` in the corpus report; a `KI` is the same `KI`. The
  reviewer doing decoder triage sees one entry, not two.
- **A matrix Bugzilla column.** The conformance matrix grows a
  block that uses the same legend as the existing TT* block, so a
  reviewer doing decoder triage looks at *one* matrix and sees
  both trusted-input regressions (red TT* cells) and hostile-input
  regressions (red Bugzilla cells) under one legend. This is the
  downstream payoff of the consolidation: the matrix is the single
  view, and the corpus-validator report becomes the per-cell
  evidence backing it.

The reviewer workflow becomes: open the matrix, look for red cells,
click into the cell to see the per-fixture evidence (corpus-report
rollup, sanitizer markers, vendor pin). One legend, one starting
point.

## Why this is not a refactor for its own sake

A natural pushback is "the three vocabularies work; consolidating them
is a lot of code change for not much functional gain." The answer is
that the cost is not the code change, it is the reviewer-time leak the
fragmentation produces every time someone reads the reports. That cost
is hard to measure directly but is visible in slower vendor bumps,
slower triage on flaky cells, and reviewer comments asking "what does
ERR! mean here?" that should not need to be asked at this point in the
project's life.

The consolidation also unblocks the matrix Bugzilla column. Today the
matrix renderer assumes its inputs come from
`tools/conformance/known-issues.ron`; adding a Bugzilla column without
the consolidation means duplicating either the renderer or the catalog,
both of which are worse than doing the extraction.

## Open questions

Things the eventual plan should pick up; this doc deliberately does
not pre-decide them.

- **Module location.** `src/util/external_decoders.rs` keeps the
  enum near the sandbox and the corpus-validator;
  `src/conformance/external.rs` keeps it near the matrix and the
  RON catalog. Both have good arguments; the next plan should pick
  one and document why.
- **Layering.** Whether the consolidated enum should be a strict
  superset of `Verdict` (every Verdict variant is also a
  Disposition variant) or a layered relationship (the enum
  describes runtime outcomes, a separate type describes
  human-classified buckets) is open.
- **Matrix layout for the Bugzilla block.** Sibling matrix vs
  embedded block in the same matrix. The audit workflow probably
  prefers embedded; the rendering width budget probably prefers
  sibling. The next plan should do this with data, not aesthetics.
- **Java decoder verdicts.** When the Java decoder
  (`jbig2-imageio`) is wired in, it may produce verdicts the C
  decoders do not (e.g. `JVMCrash`). The legend extension policy
  should be in the next plan, not assumed today.

## Pointers

- The Phase 1 `Verdict` enum that this doc would extend or replace:
  [`src/validator/corpus.rs`](../src/validator/corpus.rs)
- The classified catalog this doc would consolidate with:
  [`tools/conformance/known-issues.ron`](../tools/conformance/known-issues.ron)
- The matrix audit that defines the existing legend:
  [`docs/conformance-matrix-decode-audit.md`](conformance-matrix-decode-audit.md)
- The corpus harness whose rollup this doc would unify with the
  matrix: [`tools/corpus-validator/main.rs`](../tools/corpus-validator/main.rs)
