# Fuzz strategy

This document motivates layering coverage-guided fuzzing on top of the
deterministic corpus regression that ships in Phase 1. It is the
canonical reference for any future plan that picks up fuzz integration.
It explicitly does not pin the fuzzer (libFuzzer, AFL++, both),
the target shape (byte-level, structure-aware, differential), or the
nightly schedule. Those are implementation details the next plan
should figure out on its own evidence.

The intended reader is a maintainer or contributor weighing whether a
fuzz layer is worth the cost. Today, the answer is "the deterministic
corpus is the floor; without a fuzz layer, the project is leaving
signal on the table." The rest of the doc explains why.

## What we already get from the deterministic corpus

[`tests/validator-corpus/bugzilla/harvested/`](../tests/validator-corpus/bugzilla/harvested/)
contains 12 hostile inputs harvested from Ghostscript Bugzilla. Each
fixture is a stream that broke `jbig2dec` in some way (segfault,
infinite loop, bounded but pathological allocation, OOB read). Phase 1
turns those fixtures into a per-impl regression net via
`corpus-validator --strict`: today's `jbig2-rust` rejects every one of
the 10 hostile streams that should be rejected, accepts the two that
look structurally valid, and the contract is a one-line `verdict =`
field per `(fixture, impl)` cell. If a refactor regresses any cell, CI
is loud about it.

That coverage is real and load-bearing, and it is the wrong conclusion
to stop there. The deterministic corpus tells us about the bugs we
already know about. It cannot tell us about the bugs we do not.

## Why fuzz is the right next layer

The Bugzilla corpus is itself the fossil record of decoder bugs found
by fuzzing somewhere else. Several of its entries cite
`https://oss-fuzz.com/` directly; others reference upstream bug-bot
emails and security-researcher reports. Every fixture in
`harvested/` is evidence that fuzzing produces value on this codebase
shape — it is just that today the value flows from someone else's
fuzzer (Artifex's OSS-Fuzz, security researchers, jbig2dec's own
historical CI) into our regression set.

That asymmetry is uncomfortable to leave in place. Three reasons:

1. **The upstream fuzzers do not test our decoder.** OSS-Fuzz runs
   `jbig2dec`. Crashes there sometimes produce streams that also
   trip `jbig2-rust`, but we have no way to systematically check
   that. The bugs that affect our decoder but not jbig2dec — and
   those exist; the implementations are independent — never reach
   the harvested corpus.
2. **We are downstream of upstream priorities.** Several historical
   Bugzilla entries reach Artifex closed `RESOLVED WONTFIX` for
   reasons that are sensible inside the Ghostscript codebase but do
   not bind us. We should not depend on Artifex deciding to fuzz a
   particular code path for our regression set to grow there.
3. **Coverage decay is asymptotic, not abrupt.** A fuzzer pointed at
   `jbig2-rust` keeps finding bugs the same way an OSS-Fuzz instance
   does. The interesting question is not "will it find anything"
   but "how long until it pays for itself." The answer for a
   parser-heavy codebase with a documented bit-level grammar is "not
   long."

Phase 1's deterministic regression is the floor. A fuzz layer raises
the ceiling.

## What success looks like, as outcomes

This section is deliberately about outcomes rather than architecture.
The implementation plan should pick whichever fuzzer and harness shape
gets these outcomes cheapest.

- **Nightly fuzz runs over the existing corpus as seeds.** Every
  fixture under `tests/validator-corpus/bugzilla/harvested/` is a
  high-quality starting point. The fuzzer should bias mutation
  around them rather than generating random bytes from scratch.
- **New crashes land in `fuzz/artifacts/` and do not auto-promote.**
  The reviewer concern in the original architectural plan was that
  auto-promoting fuzz crashes into the regression corpus creates
  failing tests before fixes exist. We took that concern seriously
  and the policy is: a fuzz crash is an artefact, not a fixture. A
  maintainer triages it (steps below) and decides what to do.
- **Manual triage is supported by tooling, not heroics.** A
  `tools/corpus-mint fuzz-triage <reproducer>` subcommand should
  minimize the input, produce a `meta.toml` skeleton, and stage
  the fixture under `tests/validator-corpus/fuzz-derived/`. The
  maintainer reviews the verdict per impl, decides whether to flip
  any cell from observed to desired, and commits.
- **Structure-aware fuzzing kicks in when the byte-level fuzzer
  stops finding new bugs.** Coverage trace plateaus are the signal:
  while libFuzzer keeps discovering new edges, the byte-level
  target is paying its keep. When it plateaus, an `Arbitrary`-based
  structure-aware target that emits real `SegmentHeader` /
  `RegionHeader` tuples will start finding bugs the byte-level
  target cannot reach.

The fuzz layer is *not* a replacement for the deterministic corpus.
It is a different kind of evidence: the deterministic corpus enforces
known invariants, the fuzz layer searches for unknown ones. Both run.

## Why not just rely on existing fuzzers

Two related questions a reader might ask: "OSS-Fuzz is open source —
can we just upload our crate?" and "There are existing JBIG2 fuzzing
projects — can we point at them?" Both are reasonable starting points;
neither replaces a local fuzz layer.

The difference is feedback latency. An upstream fuzzer that finds a
crash filing it as a bug somewhere takes days to weeks to reach our
regression set. A local fuzz layer fails the next build. The cost of
that latency is exactly the bugs that reach a release without us
noticing — which, given the history of this corpus, has happened
before and will happen again unless we do something about it.

The cost-of-ownership argument is also weaker than it looks. Cargo's
`cargo-fuzz` integration plus a single fuzz target is roughly a
day's work, and the running cost is "one nightly job that produces
artefacts you can ignore until they're interesting." That is well
under the cost of one un-noticed shipped regression.

## Manual triage as a guardrail

The reviewer concern about auto-promotion is the one detail this doc
is explicit about. Auto-promotion of fuzz crashes into a regression
corpus creates a failure mode where:

1. The fuzzer finds a crash.
2. The CI auto-promotes it into a fixture with `verdict =
   "RejectErr"` (or whatever heuristic).
3. The fixture starts failing every PR.
4. The crash is real, but the fix is not in scope for the PR that
   triggered the auto-promotion, so the failing test is a permanent
   blocker until someone happens to fix the underlying bug.

That pattern trains contributors to disable failing tests, which is
worse than not having the test at all. The policy here is the
opposite: a fuzz crash is *evidence*, and the reviewer decides what
to do with it. Sometimes the right action is "minimize and add as a
fixture with verdict = RejectErr to track the bug" (the bug is real
but the fix is scoped); sometimes it is "minimize and add as a
fixture with the observed verdict, accept that the fix is out of
scope for now" (the bug is a known limitation we are documenting);
sometimes it is "this is a duplicate of an existing fixture, drop
it." Tooling should make all three cheap; nothing should be
automatic.

## Open questions

Things this doc deliberately leaves on the table.

- **Fuzzer choice.** libFuzzer integration via `cargo-fuzz` is the
  obvious starting point on Linux. AFL++ is a serious contender for
  longer-running runs. Whether the project should run both is open.
- **Differential fuzzing.** Comparing `Jbig2Decoder` output against
  `jbig2dec` on the same input produces a different kind of signal
  (semantic divergence) than crash-finding. Whether the cost of a
  differential harness — both decoders running on every input,
  pixel comparisons in the loop — pays off relative to single-impl
  fuzzing is an open question that the next plan should answer with
  data.
- **Structure-aware target shape.** The right `Arbitrary` impl for
  JBIG2 streams is non-trivial. A first cut emits valid
  `SegmentHeader` records and lets the data section be free bytes;
  a more aggressive cut produces structurally-valid arithmetic-coded
  payloads. The right choice depends on what the byte-level fuzzer
  has already found.
- **Triage interaction with the bugzilla corpus.** When a fuzz crash
  reproduces a known-but-unharvested upstream Bugzilla bug, the
  triage workflow needs to recognize that and link the new fixture
  to the upstream entry rather than creating a parallel record.

## Pointers

- The deterministic regression layer this layers on top of:
  [`docs/corpus-ci-goals.md`](corpus-ci-goals.md)
- The fixture authoring tooling that the triage subcommand will
  extend: [`tools/corpus-mint/main.rs`](../tools/corpus-mint/main.rs)
- The harvested corpus that seeds the fuzzer:
  [`tests/validator-corpus/bugzilla/harvested/`](../tests/validator-corpus/bugzilla/harvested/)
- The drift guards that make fixture regeneration safe:
  [`docs/corpus-drift-guards.md`](corpus-drift-guards.md)
