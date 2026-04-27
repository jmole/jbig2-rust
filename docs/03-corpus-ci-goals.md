# 03 — Corpus CI Goals

This document describes the CI policy we want to grow into once the
deterministic corpus harness ships. It deliberately changes no CI
configuration — there is no GitHub Actions workflow, no nightly
schedule, no `.yml`. The goal is to make the design discussion happen
once, in one place, so that the next plan that picks up CI implements
something agreed-upon rather than improvising under deadline pressure.

The reader is a maintainer about to wire the corpus into a CI provider
and trying to decide how strict each layer should be, what signal each
layer is meant to produce, and what to do when a layer exceeds its
walltime budget.

## Sequence

This is **step 3 of 5** in the corpus harness rollout.

1. [`01-corpus-drift-guards.md`](01-corpus-drift-guards.md)
2. [`02-sandbox-preflights.md`](02-sandbox-preflights.md)
3. **`03-corpus-ci-goals.md` — you are here.**
4. [`04-fuzz-strategy.md`](04-fuzz-strategy.md)
5. [`05-external-decoder-taxonomy.md`](05-external-decoder-taxonomy.md)

**Prerequisites.** Steps 1 and 2. Tier 0 (fast strict regression on
every PR) is safe to enable as soon as drift guards (step 1) are in
place. Tier 1 (full corpus including external-decoder rows) must not
enable `--with-c-decoders` until vendor-SHA preflight (step 2) is
landed; without it, baselined external-decoder verdicts can drift
silently under the next vendor bump and the regression net loses
teeth.

**Implementation cadence.** This doc is policy, not a single PR.
Implement in two phases:

- *Phase 3a — tier-0 wiring.* After step 1 lands. CI runs
  `corpus-validator --strict` on the rust row only, against a small
  hand-picked subset, on every PR.
- *Phase 3b — tier-1 expansion.* After step 2 lands. CI runs the full
  corpus including external decoders, nightly, with vendor-SHA
  preflight enforcing.

Tier 2 (fuzzing) cannot be enabled before [step
4](04-fuzz-strategy.md) is implemented — its policy is captured in
this doc only so that the cadence/budget framework is unified across
all three tiers.

**Unblocks.** A predictable CI surface that subsequent steps slot
into. Step 4's nightly fuzz cadence and step 5's matrix Bugzilla
column both reference the layering defined here.

## Why this needs a doc

The corpus has more than one purpose. It is, simultaneously:

- a regression net for the decoder ("yesterday this Bugzilla fixture
  rejected with `RejectErr`; today it crashes — block the PR"),
- a confidence signal for refactors ("the validator catalog still
  classifies every fixture the same way it did last release"), and
- a research artefact ("here is the per-fixture per-implementation
  matrix; the cells where `jbig2-rust` and `jbig2dec` disagree are
  worth investigating").

These purposes have different cost profiles. Regression coverage on a
small handful of hostile inputs has to run on every PR or it is not
regression coverage. The full corpus matrix can run nightly. The fuzz
layer (described in [`docs/04-fuzz-strategy.md`](04-fuzz-strategy.md))
wants hours of walltime, not minutes. Conflating them produces either a slow
PR loop or a leaky regression net, depending on which way the
compromise leans, and we have observed both failure modes in adjacent
projects.

A CI policy that names the layers up front, sets a budget per layer,
and describes the escalation path when a layer exceeds budget is
cheaper to maintain than one that grows organically. This doc is that
policy.

## Layers

The CI layers map onto the Phase 1 harness directly:

### Tier 0 — fast strict regression

`corpus-validator --strict --filter <small subset>` against a hand-
picked subset of the harvested Bugzilla corpus, using the in-tree
`tools/jbig2-decode` binary. The baseline is the
`expected.toml::[decoder.rust].verdict` recorded under
`tests/validator-corpus/bugzilla/harvested/`.

The tier-0 contract is "this PR did not regress any decoder cell that
a maintainer has already signed off on." It runs on every PR, has to
finish in well under a minute on commodity CI, and cannot be skipped
short of an explicit maintainer override. The local `cargo test
--features validator-corpus --test corpus_validator_strict` shim
exercises the same path so a contributor can reproduce a tier-0
failure on their laptop without learning the CI surface.

### Tier 1 — full corpus matrix, default impls

`corpus-validator --strict` over the full corpus with the default
`jbig2-rust` row and the C-decoder rows wired in via
`--with-c-decoders`. This is the matrix the Phase 1 harness produces:
every fixture, every configured implementation, observed-vs-expected
verdicts in [`target/corpus-report.md`](../target/corpus-report.md).
Tier 1 catches regressions in the long tail of synthetic and
mutator-generated fixtures that tier 0 deliberately skips for
walltime.

Tier 1 runs nightly. A failure files a tracking issue rather than
blocking a PR. The reasoning: tier-1 fixtures are designed to surface
edge cases, and a breakage often takes a half-day of triage to
classify. PR-time blocking on tier 1 turns every refactor into a
forensics exercise, which trains contributors to ignore the harness.
Nightly cadence keeps the signal high without inducing that habit.

### Tier 2 — fuzzing

The fuzz layer described in
[`docs/04-fuzz-strategy.md`](04-fuzz-strategy.md). It runs on a separate
schedule (overnight, weekend, or whenever the CI provider has
spare cycles) and produces minimized reproducers, not pass/fail
signal. Tier 2 never blocks a PR. Its outputs feed manual triage,
which in turn (sometimes) produces a new tier-0 or tier-1 fixture.

## Cadence

| Layer  | Trigger                | Budget target          | Outcome on failure                               |
| ------ | ---------------------- | ---------------------- | ------------------------------------------------ |
| Tier 0 | every PR push          | < 60 s walltime        | block merge until fixed or expectation updated   |
| Tier 1 | nightly                | < 30 min walltime      | open tracking issue, do not block in-flight PRs  |
| Tier 2 | weekly / opportunistic | hours, not bounded     | new minimized reproducers in `fuzz/artifacts/`   |

The budgets are targets, not hard ceilings. The escalation procedure
below describes what happens when a layer exceeds budget.

## Escalation when a layer exceeds budget

Each layer has a documented response when its walltime grows past its
target. The order of operations is:

1. **Profile the actual cost.** `corpus-validator` already logs
   per-decoder rollups (see the "Decoder rollups" section in
   [`target/corpus-report.md`](../target/corpus-report.md)). Identify
   which fixtures or which decoder rows dominate the budget before
   reaching for a structural change.
2. **Subset, do not skip.** If a layer is over budget, split the
   fixture set into a tier-N-fast / tier-N-slow pair, with the slow
   half running on a longer cadence. Skipping fixtures wholesale is
   strictly forbidden — every fixture in `tests/validator-corpus/`
   exists because some past failure produced it, and silently
   dropping coverage is the worst possible outcome.
3. **Minimize the corpus.** Some fixtures are larger than they need
   to be (especially mutator-generated streams). The
   [`docs/01-corpus-drift-guards.md`](01-corpus-drift-guards.md) doc
   describes the seed-anchoring policy that makes regeneration safe;
   if minimization is the right answer, do it under that policy.
4. **Raise the budget with maintainer signoff.** Sometimes the right
   answer is "tier 0 is now 90 seconds." That is a maintainer
   decision, recorded in this doc with a date and a one-line
   justification, not a ratchet that creeps silently.

Each escalation must update this doc. The budgets table is the
canonical record; if the table and reality disagree, the table is
wrong and someone should fix it.

## Open questions

Things the eventual CI plan will need to decide; this doc deliberately
leaves them unresolved.

- **CI vendor.** No vendor is named here. The harness commits to
  producing pass/fail exit codes, not to running on any particular
  service.
- **Sanitizer build cadence.** A sanitizer-instrumented `jbig2dec`
  produces qualitatively different signal than a release build. The
  [`docs/02-sandbox-preflights.md`](02-sandbox-preflights.md) doc
  argues that sanitizer builds should be required at some cadence,
  but whether that cadence is "every tier-1 run" or "every tier-2
  run" is open.
- **Cross-platform.** The corpus harness has been exercised on
  Linux and macOS sandboxes. Whether tier-0 should fan out across
  both on every PR or only on nightly is a budget decision for the
  CI plan to make.
- **Vendor refresh policy.** When `vendor/jbig2dec` is bumped, the
  expected verdicts may change without the rust decoder regressing.
  The escalation step "raise budget with maintainer signoff" applies
  the same way to vendor refreshes; the open question is whether
  vendor bumps should auto-rerun tier 1 or wait for the nightly.

## Implementation status

This section records what has landed for step 3 and what remains
explicitly deferred.

- **Landed now: tier-0 PR gating and baseline hygiene CI.** The repo now
  has a PR-gating workflow with `cargo fmt --check`,
  `cargo test --workspace`, and the tier-0 strict corpus shim
  (`cargo test --release --features validator-corpus --test
  corpus_validator_strict`).
- **Landed now: tier-1 nightly wiring.** A nightly workflow now runs
  `corpus-validator --strict --with-c-decoders`, uploads
  `target/corpus-report.md`, and opens/updates a tracking issue on
  scheduled failures.
- **Deferred: sanitizer cadence policy.** This step still does not
  choose between "ASAN every tier-1 run" vs "ASAN at a slower cadence."
  That decision stays coupled to the sanitizer-canary follow-up in
  [`docs/02-sandbox-preflights.md`](02-sandbox-preflights.md).
- **Deferred: cross-platform fan-out.** Tier-0 remains Linux-first.
  Whether to fan out tier-0 across Linux/macOS on every PR is still a
  budget decision.
- **Deferred: vendor-bump trigger policy.** Whether `vendor/jbig2dec`
  bumps should auto-trigger tier-1 remains open; nightly cadence is the
  current baseline.

## Pointers

- The harness this doc gates on:
  [`tools/corpus-validator/main.rs`](../tools/corpus-validator/main.rs)
- The local sanity shim:
  [`tests/corpus_validator_strict.rs`](../tests/corpus_validator_strict.rs)
- The fuzz layer this doc references:
  [`docs/04-fuzz-strategy.md`](04-fuzz-strategy.md)
- The drift guards this doc references:
  [`docs/01-corpus-drift-guards.md`](01-corpus-drift-guards.md)
- The preflights tier-1 enforces:
  [`docs/02-sandbox-preflights.md`](02-sandbox-preflights.md)
- The architectural context:
  [`docs/conformance-matrix-decode-audit.md`](conformance-matrix-decode-audit.md)
