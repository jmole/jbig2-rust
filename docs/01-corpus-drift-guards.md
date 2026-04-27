# 01 — Corpus drift guards

This document motivates two changes to how the synthetic and
mutator-generated corpora are validated: a `seed_sha256` cross-check
that turns silent vendor drift into a loud regeneration prompt, and a
softening of the `check_ids` strict-superset assertion into something
that does not punish orthogonal validator improvements. It is the
canonical reference for any future plan that picks up either change.

The reader is a maintainer who has not yet hit either failure mode but
is going to. Both are latent in the harness today; the doc's job is to
explain why they matter enough to fix before they do.

## Sequence

This is **step 1 of 5** in the corpus harness rollout. Land the docs
in this order:

1. **`01-corpus-drift-guards.md` — you are here.**
2. [`02-sandbox-preflights.md`](02-sandbox-preflights.md)
3. [`03-corpus-ci-goals.md`](03-corpus-ci-goals.md)
4. [`04-fuzz-strategy.md`](04-fuzz-strategy.md)
5. [`05-external-decoder-taxonomy.md`](05-external-decoder-taxonomy.md)

**Prerequisites.** None. The seed-SHA cross-check applies to existing
fixtures and the `check_ids` softening only relaxes an existing
assertion; no schema work, new binaries, or external-decoder rows are
required.

**Why first.** This is the smallest, lowest-risk change with an
immediate correctness win. Landing it before later steps means every
subsequent corpus regeneration (and there will be several as the
harness grows) is protected by the seed cross-check from day one.

**Strict ordering rule.** Step 1 can land on its own. Step 2
(vendor-SHA preflight) is the same *kind* of guarantee but for
external decoder binaries rather than corpus seed inputs; the two
checks are deliberately separated because step 1 protects fixtures
already on disk while step 2 protects baselines that have not yet been
recorded. **However: any PR that commits external-decoder verdicts
into `expected.toml` must also enforce step 2.** Recording a
`decoder.jbig2dec.verdict = "RejectErr"` against an unpinned vendor is
exactly the silent-bad-baseline failure mode step 2 exists to prevent.

**Unblocks.** Safe corpus regeneration during all later steps. Without
the seed cross-check, every later doc's "regenerate the corpus and
re-baseline" instruction risks silently re-anchoring fixtures against
a moved seed.

## The corpus is part of the contract

A regression net depends on its inputs being stable. If the inputs
change without the maintainer noticing, the net is not what it
claims to be. Today, two specific drift mechanisms violate that
property in the synthetic and mutator corpora.

The synthetic corpus
([`tools/corpus-mint/main.rs`](../tools/corpus-mint/main.rs))
generates streams from hand-built segment payloads. The mutator
corpus
([`tools/corpus-mint/mutate.rs`](../tools/corpus-mint/mutate.rs))
takes `vendor/jbig2dec/annex-h.jbig2` as a seed and applies typed
byte-level mutations. The annex-h-bitflip corpus
([`tools/corpus-mint/annex_h_bitflip.rs`](../tools/corpus-mint/annex_h_bitflip.rs))
is the same shape as the mutator corpus with a different mutation
strategy. All three are checked into
[`tests/validator-corpus/`](../tests/validator-corpus/) and exercised
by the regression test in
[`tests/validator_corpus_regression.rs`](../tests/validator_corpus_regression.rs).

Both drift modes apply to whichever of these corpora consumes a
vendored seed (today: the mutator and bitflip corpora) and to the
strict-superset assertion (today: all three).

## Failure mode 1: silent seed drift

The mutator records a `seed_sha256` per generated fixture. That hash
is cosmetic: nothing reads it back. If `vendor/jbig2dec/annex-h.jbig2`
changes content (because the submodule was bumped, or because someone
edited the file directly, or because the file was regenerated from
upstream), every mutation derived from that seed re-anchors against a
different starting point. The fixtures still parse, the regression
test still passes, but the streams persisted in
`tests/validator-corpus/synthetic-mutators/` are no longer the
mutations of the seed they were minted against.

Slow-motion narrative:

1. `vendor/jbig2dec` is bumped for a security patch. The patch
   includes a regenerated `annex-h.jbig2` (a 4-byte change in the
   header, say).
2. `tests/validator_corpus_regression.rs` runs. Every fixture under
   `synthetic-mutators/` parses cleanly because the recorded
   `expected.toml::primary_check_id` still appears in the validator
   output: the mutations remain unrelated to the seed change.
3. The regression net is now silently weaker. A future bug that
   would have been caught by a specific seed-anchored mutation
   (e.g. "this mutator inserts a byte at offset 12 of the seed") no
   longer hits the same code path because offset 12 of the new seed
   is different.
4. Months later, a contributor regenerates the corpus with
   `cargo run --bin corpus-mint --features validator-corpus` and
   the diff includes a hundred fixture changes. They cannot tell
   which were intentional and which were caused by the unnoticed
   seed bump.

The user-facing symptom is "regenerating the corpus produced a
mysterious diff." The actual cost is the unbounded period of weakened
coverage between the seed bump and the regeneration.

## Failure mode 2: strict-superset friction punishes validator improvements

`mint_synthetic` persists the validator's full `findings` set into
`expected.toml::check_ids`. The regression test asserts that every
persisted id is still produced by today's validator. The intent is
"this fixture was minted to test these checks; if the validator
stopped firing one, something regressed."

The implementation has a cost that the intent does not capture. The
synthetic builders hand-roll bytes that violate more than one rule —
a malformed segment header may incidentally trip a generic-region
check the builder did not know about. The validator catalog
faithfully reports both checks; `check_ids` records both. Months
later, someone improves the catalog by splitting one check into two
or by tightening the heuristics on an unrelated check, and the
incidentally-fired check id either no longer appears or is renamed.

The fixture now fails the strict-superset assertion. The contributor
who improved the catalog has to decide: regenerate the corpus
(which conflates their patch with a corpus change) or weaken the
new catalog change (which weakens the validator). Neither is the
right answer. The maintenance trap is the friction itself: a
strict-superset assertion that punishes orthogonal validator
improvements *trains contributors to weaken the regression set or
the validator*, both of which are bad outcomes.

The intent — "every persisted id should still fire" — is the wrong
contract. The persisted contract should be "the primary check id
that motivated this fixture is still firing." Other ids in the set
are useful evidence at minting time but should not be assertable.

## What success looks like

The two changes are small but pointed.

### Seed cross-check

Every `expected.toml` whose containing corpus consumed a vendored
seed records the seed's path and `sha256`. The regression test
reads the current vendored file's bytes, hashes them, and compares
against the recorded value. On mismatch, the test fails with a
remediation that is one line and unambiguous:

```
synthetic-mutators: vendor/jbig2dec/annex-h.jbig2 has changed since
this fixture was minted (was abcd1234..., now ef567890...). Either
revert the vendor bump or regenerate with
`cargo run --bin corpus-mint --features validator-corpus`.
```

The cross-check turns silent loss of regression coverage into a
loud "regenerate the corpus" message at the next test run. The
maintainer cannot regenerate without realising the seed moved,
which is the whole point.

### check_ids becomes informational, primary stays asserted

The regression test continues to assert that
`expected.toml::[validator].primary_check_id` is in the validator's
finding set. It stops asserting that every id in
`expected.toml::[validator].check_ids` is. The latter list keeps
its place as evidence — "the maintainer who minted this fixture saw
these ids fire" — but stops being an assertion target.

Concretely: the loop in
[`tests/validator_corpus_regression.rs`](../tests/validator_corpus_regression.rs)
that produces a "missing recorded check ids" failure becomes a
warning at test-output time, or moves into a separate non-failing
report, or is removed entirely. The next plan should pick which
shape based on whether the warning has any signal value at all
(initial guess: not enough to keep).

The new contract is "the primary id is still firing." Adding a
validator check that incidentally fires on a synthetic fixture no
longer breaks the test; removing or renaming an incidentally-fired
check no longer requires a corpus regeneration; the unrelated-improvement
friction goes away.

## Why both changes belong together

A reasonable reader might ask why these two unrelated-looking changes
share a doc. The reason is that both are addressing the same
underlying property: **the regression net should fail loudly when its
contract is violated and stay quiet otherwise.** Both failure modes
above invert that property. The seed-drift failure mode is silent
when it should be loud; the strict-superset failure mode is loud when
it should be quiet (or at least quieter). Fixing one without the
other still leaves the harness contributing to the wrong contributor
trainings.

The fix surface is also symmetric: both changes live in
`tests/validator_corpus_regression.rs` plus minor `corpus-mint`
work, and the doc that motivates either is the doc that motivates
both.

## Relationship to the vendor-SHA preflight (step 2)

A reasonable next question is "isn't the seed-SHA cross-check the same
thing as [`02-sandbox-preflights.md`](02-sandbox-preflights.md)'s
vendor-SHA preflight, just under a different name?" They are cousins,
not twins. Both encode the invariant *a vendored input the corpus
depends on must not move out from under a recorded baseline without
the harness saying so*, but they operate on different artefacts:

| Aspect            | Step 1 (this doc)                              | Step 2 (preflights)                                          |
| ----------------- | ---------------------------------------------- | ------------------------------------------------------------ |
| What is hashed    | A file's content (`vendor/jbig2dec/annex-h.jbig2`) | A submodule SHA (`git -C vendor/jbig2dec rev-parse HEAD`)    |
| Protects          | Synthetic / mutator / bitflip fixtures on disk | External-decoder verdicts in `expected.toml`                 |
| Lives in          | `tests/validator_corpus_regression.rs`         | `tools/corpus-validator/main.rs` (preflight phase)           |
| Required to land  | Now (step 1)                                   | Before any external-decoder verdict is committed (step 2)    |

The two can land in separate PRs. They should share the underlying
"vendor anchor" helper — a single module that hashes a path or reads a
submodule SHA, compares against a recorded value, and produces a
unified remediation message — so step 2's preflight reuses step 1's
infrastructure rather than parallel-implementing it.

## Open questions

Things the next plan should pick up; this doc does not pre-decide.

- **Removing `check_ids` entirely.** A reasonable alternative to
  "demote check_ids to a warning" is "remove check_ids from
  `expected.toml` entirely." The argument for removal: anything not
  asserted is overhead. The argument for keeping it as evidence: a
  maintainer regenerating a corpus 18 months from now benefits from
  the historical record of what fired. The next plan should decide
  with explicit weight on the maintainer-time cost; we suspect
  removal is fine, but we want the next plan to verify on the
  regenerated corpora.
- **Auto-regen on drift.** When the seed cross-check fails, should
  CI auto-regenerate the corpus and produce a PR, or should it
  hard-fail and require a human? Auto-regen is a tempting
  ergonomics win and a serious correctness loss; we err toward
  hard-fail unless the next plan can produce a credible auto-regen
  workflow that does not silently update the contract.
- **Bitflip corpus coverage.** The `annex-h-bitflip` corpus has
  the same shape as the mutator corpus and is subject to the same
  seed-drift failure mode. The cross-check should extend to it
  identically; the next plan should not treat it as a separate
  case.
- **Cross-impl drift.** The seed cross-check assumes the seed file
  is the only relevant input. If a future plan adds mutator
  corpora seeded from `vendor/T-REC-T.88-201808/...` conformance
  streams, the same logic applies but with a different seed path.
  The cross-check shape should be "list of (path, sha) pairs," not
  "the one annex-h path."

## Implementation status

This section records what has landed from step 1 and what remains
explicitly deferred.

- **Landed: seed drift cross-check.** Commit `201d38c5` added regression
  checks that read `meta.toml` seed anchors and fail loudly when a
  fixture's vendored seed has moved.
- **Landed: shared vendor-anchor infrastructure.** Commit `03fc124b`
  introduced [`src/util/vendor_anchor.rs`](../src/util/vendor_anchor.rs)
  and wired step 2 to reuse it, satisfying this doc's requirement that
  drift checks and preflights share one anchor helper rather than
  parallel implementations.
- **Still open: `check_ids` strict-superset softening.** The regression
  contract still persists and asserts historical `check_ids`; the
  proposed shift to "assert primary id only, treat extra ids as
  informational" remains follow-up work.
- **Still open: policy questions in this doc's open-questions list.**
  Auto-regeneration policy and future multi-seed cross-impl coverage are
  still unresolved by design.

## Pointers

- The mutator that records `seed_sha256`:
  [`tools/corpus-mint/mutate.rs`](../tools/corpus-mint/mutate.rs)
- The synthetic minter that persists `check_ids`:
  [`tools/corpus-mint/main.rs`](../tools/corpus-mint/main.rs)
- The bitflip corpus that has the same shape:
  [`tools/corpus-mint/annex_h_bitflip.rs`](../tools/corpus-mint/annex_h_bitflip.rs)
- The regression test where both changes land:
  [`tests/validator_corpus_regression.rs`](../tests/validator_corpus_regression.rs)
- The schema where `check_ids` lives:
  [`src/validator/corpus.rs`](../src/validator/corpus.rs)
- The CI cadence the changes will run inside:
  [`docs/03-corpus-ci-goals.md`](03-corpus-ci-goals.md)
- The preflight that is the closest cousin to this doc:
  [`docs/02-sandbox-preflights.md`](02-sandbox-preflights.md)
