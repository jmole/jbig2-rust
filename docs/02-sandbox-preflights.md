# 02 — Sandbox preflights

This document motivates two guardrails the corpus harness does not yet
enforce: vendor-SHA pinning across `corpus-validator`, and runtime
verification that sandboxed binaries are sanitizer-instrumented when
the report assumes they are. It is the canonical reference for any
future plan that adds a preflight phase to `corpus-validator`.

The reader is a maintainer who has been bitten — or is about to be
bitten — by a quiet failure mode. Both failure modes already exist
in the pipeline today; the doc's job is to make them legible enough
that the next plan implementing the preflights does not have to
re-derive the motivation.

## Sequence

This is **step 2 of 5** in the corpus harness rollout.

1. [`01-corpus-drift-guards.md`](01-corpus-drift-guards.md)
2. **`02-sandbox-preflights.md` — you are here.**
3. [`03-corpus-ci-goals.md`](03-corpus-ci-goals.md)
4. [`04-fuzz-strategy.md`](04-fuzz-strategy.md)
5. [`05-external-decoder-taxonomy.md`](05-external-decoder-taxonomy.md)

**Prerequisites.** Step 1 (drift guards) does not strictly block step
2, but the two share the same underlying "vendor anchor" helper. Land
step 1 first so that helper exists; step 2 reuses it for the
submodule-SHA cross-check rather than parallel-implementing one. See
the *Relationship to corpus drift guards (step 1)* section in
[`01-corpus-drift-guards.md`](01-corpus-drift-guards.md).

**Hard sequencing rule for the wider plan.** External-decoder verdicts
must not be committed into `expected.toml` (or enforced by
`corpus-validator --strict`) until the vendor-SHA half of this
preflight is in place. Recording a `decoder.jbig2dec.verdict =
"RejectErr"` against an unpinned vendor binary is the silent
bad-baseline failure mode this doc exists to prevent. If a future PR
in this rollout baselines external-decoder expectations, that PR must
also wire the vendor-SHA preflight (or land it in the immediately
preceding PR). The sanitizer-canary half can land later — its absence
weakens the regression net but does not poison committed baselines.

**Unblocks.** Steps 3–5 all rely on `corpus-validator --strict` having
trustworthy inputs. Tier-1 CI (step 3) running external decoders, the
fuzz layer (step 4) seeded from a verified corpus, and the matrix
Bugzilla column (step 5) all assume the preflight has already removed
silent vendor drift from the failure-mode list.

## A quiet harness is a worse harness than a loud one

The corpus pipeline today has two specific places where a silent
failure produces *wrong* output that looks superficially correct.
Both are easy to miss in code review and both are concrete enough to
narrate end-to-end.

### Failure mode 1: vendor SHA drift

[`tools/conformance/known-issues.ron`](../tools/conformance/known-issues.ron)
pins vendor SHAs (the active pin is
`6ecb04980813d693234190021bd1cf874c05b1b4` for
`vendor/jbig2dec`) and the matrix renderer rejects drift loudly:
`docs/known-conformance-issues.md` shows `Vendor pin status: ok`
for every entry, and an out-of-date pin produces a hard error.

The corpus validator does not consult that pin. It calls
`vendor/jbig2dec/jbig2dec` directly and reports whatever the binary
produces. After a `git submodule update` that bumps `jbig2dec`,
the next `corpus-validator --strict` run scores against a different
binary than the one the recorded expectations were baselined on, and
the maintainer has no way to know.

The failure looks like this in slow motion:

1. A contributor runs `git submodule update --remote
   vendor/jbig2dec` for unrelated reasons (security patch, build
   fix, doc change).
2. They rebuild the `jbig2dec` binary. New binary, same path.
3. They run `corpus-validator --strict --with-c-decoders`.
   Several Bugzilla fixtures whose `decoder.jbig2dec.verdict` was
   baselined against the old SHA now produce different verdicts
   under the new SHA.
4. CI fails. The contributor reads the output: "expected RejectErr,
   observed Ok." They diff their patch — no decoder code changed.
   They spend a half-day looking for an injection point in
   `tools/corpus-validator` that does not exist.
5. Eventually someone notices the submodule moved.

The cost is wasted reviewer time, but the worse cost is erosion of
trust: a reviewer who saw that pattern once is much slower to
believe a future "expected RejectErr, observed Ok" mismatch is real.
This is exactly the failure mode the preflight is meant to prevent —
the harness should have been loud about the SHA drift in a one-line
banner at the top of the run, before any fixture executed.

### Failure mode 2: silent non-sanitizer build

[`tools/corpus-validator/main.rs`](../tools/corpus-validator/main.rs)
classifies sandbox stderr that contains
`AddressSanitizer:` markers as a sanitizer hit (the SAN
classification in the rollup). That logic is correct conditional on
the binary actually being a sanitizer build.

The corpus validator does not check that. A non-ASAN `jbig2dec`
produces zero matches even on known-bad input — the stream that
Bugzilla 696052 once segfaulted on, that an ASAN build would
flag with a heap-overflow report, runs cleanly through a release
build and the harness records it as `OK`. The rollup at the bottom
of the report says "0 sanitizer hits over 25 fixtures." This is the
worst-possible failure mode: silent confidence on a non-test.

The slow-motion narrative:

1. A new contributor sets up `vendor/jbig2dec`. The build
   instructions in their environment do not include
   `CFLAGS='-fsanitize=address'`, so they end up with a release
   build at `vendor/jbig2dec/jbig2dec`.
2. `corpus-validator --with-c-decoders` runs. Every C-decoder
   row that should crash under ASAN simply rejects the stream with
   exit 1 (a "ParseError" in jbig2dec's terms). The classification
   is `EXIT`, not `SAN`, but `EXIT` looks indistinguishable from
   "the rust decoder also rejected this" in the rollup.
3. The contributor commits an updated `expected.toml` with
   `decoder.jbig2dec.verdict = "RejectErr"` baselined off this
   non-sanitizer run. CI accepts it.
4. A future PR that *re-introduces* a heap overflow in jbig2dec
   would have been caught by an ASAN build. With the silent
   non-ASAN baseline, the regression has to manifest as something
   visible at exit code level — which, in the case of "writes one
   byte past a buffer," it does not.

This one is harder to narrate than the SHA drift case because
nothing visibly fails; the harness runs to completion and the
maintainer never finds out the regression net never had teeth.

## What success looks like

A preflight phase at the top of every `corpus-validator` run
produces a one-line banner per configured impl:

```
corpus-validator: preflight: rust    OK (binary @ target/release/jbig2-decode)
corpus-validator: preflight: jbig2dec    OK (vendor/jbig2dec @ 6ecb04980813, ASAN build detected)
corpus-validator: preflight: itu_t88    OK (binary @ vendor/T-REC-T.88-201808/.../jbig2)
```

The exact wording is the next plan's call. The shape is what matters:

- **Vendor SHA cross-check.** For impls whose binary lives under
  `vendor/<submodule>/`, the preflight reads
  [`tools/conformance/known-issues.ron`](../tools/conformance/known-issues.ron),
  computes the current submodule SHA, and refuses to score against a
  drifted vendor with a one-line remediation:
  `git -C vendor/jbig2dec checkout 6ecb04980813`. Local debugging
  gets an explicit env-var opt-out (`JBIG2_CORPUS_NO_VENDOR_CHECK=1`)
  so a contributor explicitly working on a vendor bump is not
  blocked.
- **Sanitizer canary.** For impls that the rollup is going to
  classify ASAN markers from, the preflight runs the binary against
  a small, known-bad canary stream and verifies the binary actually
  produces an `AddressSanitizer:` line. A non-ASAN build fails the
  preflight loudly, again with an env-var opt-out
  (`JBIG2_CORPUS_NO_SANITIZER_CHECK=1`).
- **Failure mode is loud, not silent.** The preflight is a
  hard refusal by default, not a warning. A reviewer reading the
  output cannot mistake a missed signal for a real outcome. Local
  debugging has explicit overrides; CI does not enable them.

The preflight is cheap (under a second per impl), runs at the top of
the run, and reduces the cost of the two failure modes from
"half-day of forensics" to "one-line banner."

## Open questions

Things the next plan should pick up. The preflight philosophy is
clear; the specifics are not.

- **Canary fixture per decoder.** The sanitizer canary must be a
  known-bad stream that always trips ASAN on a sanitizer build of
  the target binary. The Bugzilla corpus has plausible candidates;
  the next plan should pick one per impl and document the choice.
- **ASAN detection on macOS.** `otool -L | grep
  libclang_rt.asan` is the conventional probe and works on Apple
  Clang. It does not always work on Homebrew LLVM or on cross-built
  binaries. The probe needs to fall back to "run the canary and
  see what comes out" gracefully, which is what the canary already
  does — but the preflight order has to be "structural probe first,
  canary second" to keep latency down.
- **Cadence.** The fast preflight (vendor SHA cross-check) costs
  milliseconds and should run on every invocation. The sanitizer
  canary costs ~100 ms per impl and is borderline; the next plan
  should decide whether that is fine for tier-0 PR runs or whether
  it should run on tier-1 only. (See
  [`docs/03-corpus-ci-goals.md`](03-corpus-ci-goals.md) for the tier
  layering.)
- **What happens when a contributor updates the SHA pin
  intentionally.** The preflight should refuse to run, but the
  remediation needs to mention the pin file as well as the
  submodule. A contributor doing a deliberate vendor bump should
  update both atomically.

## Implementation status

This section records what has landed in the narrowed vendor-SHA PR
and what remains intentionally deferred.

- **Landed now: vendor-SHA preflight.** This is the load-bearing half
  for baseline safety: `corpus-validator` now refuses to score
  `--with-c-decoders` against a drifted vendored decoder binary,
  instead of silently re-scoring `expected.toml` against a moved
  submodule.
- **Deferred: sanitizer canary.** Current
  `[decoder.jbig2dec].verdict` cells in the corpus are still
  `Unknown`, and `corpus-validator --strict` skips `Unknown` expected
  cells (see `collect_strict_mismatches` in
  [`tools/corpus-validator/main.rs`](../tools/corpus-validator/main.rs)).
  So deferring the canary does not weaken an active, committed crash
  quality contract today.
- **Hard sequencing rule.** Before any PR commits concrete
  `jbig2dec` crash-quality expectations (for example a non-`Unknown`
  verdict whose correctness depends on ASAN/SAN behavior), the
  sanitizer canary becomes non-optional and must land in that PR or
  the immediately preceding one. Vendor-SHA preflight protects
  baseline identity; sanitizer canary protects crash-quality claims.
- **Motivation still stands.** The "Failure mode 2: silent
  non-sanitizer build" narrative above remains the canonical reason
  for the canary; this status note only records why it is deferred in
  the current step.

## Pointers

- The pin catalog the SHA cross-check would read:
  [`tools/conformance/known-issues.ron`](../tools/conformance/known-issues.ron)
- The corpus harness this doc would extend:
  [`tools/corpus-validator/main.rs`](../tools/corpus-validator/main.rs)
- The sandbox layer that runs the canary:
  [`src/util/sandbox.rs`](../src/util/sandbox.rs) and
  [`docs/sandbox.md`](sandbox.md)
- The CI policy this preflight would run inside:
  [`docs/03-corpus-ci-goals.md`](03-corpus-ci-goals.md)
- The drift guard whose helper this preflight reuses:
  [`docs/01-corpus-drift-guards.md`](01-corpus-drift-guards.md)
