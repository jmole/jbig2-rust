# External-binary sandbox

`jbig2::util::sandbox` is the runner that every untrusted external decoder
and encoder goes through across `tools/conformance`,
`tools/corpus-validator`, `tests/common/oracles.rs`, and
`tests/jbig2dec_output_compat.rs`. Build steps (`make`, `mvn`,
`./autogen.sh`, `./configure`), version probes (`-V`), and our own Rust
binaries are intentionally **not** sandboxed.

The conformance matrix used to also drive `imgcomp` for image
comparison; that path was retired in favor of an in-tree comparator
(see [`docs/conformance-matrix-encode-audit.md`](conformance-matrix-encode-audit.md)).
The `Sandbox::for_comparator` preset described below is still provided
for any future external comparator or interop tool that wants the same
envelope as encoders.

## What it gives you

Three protective layers wrap every `Command` we hand to a C/Java tool:

1. **Wall-clock watchdog** – a thread sends `SIGTERM`, waits 750 ms, then
   `SIGKILL` if the child is still alive after `Limits::wall_seconds`.
2. **Resource limits** – on Unix we install `RLIMIT_CPU`, `RLIMIT_AS`,
   `RLIMIT_FSIZE`, `RLIMIT_NOFILE`, and `RLIMIT_CORE` via a `pre_exec`
   hook before `exec`. These apply under every backend.
3. **Filesystem / network isolation** – `bwrap` on Linux,
   `sandbox-exec` on macOS, with passthrough as the no-op fallback.
   Outbound network is denied and writes are restricted to the paths the
   caller declared via `Sandbox::rw_path`.

Output captured from stdout and stderr is truncated to
`Limits::output_bytes` per stream. The drain threads keep reading after
the cap (so the child does not block on a full pipe) and the watchdog
gets `KillReason::OutputBytes`.

## Presets

Use the named constructors – do **not** build `Limits` by hand at call
sites:

| Preset                     | Wall  | CPU  | Address space | Output cap | In use? |
|----------------------------|-------|------|---------------|------------|---------|
| `Sandbox::for_decoder`     | 10 s  | 5 s  | 512 MiB       | 4 MiB      | yes     |
| `Sandbox::for_encoder`     | 60 s  | 45 s | 1 GiB         | 8 MiB      | yes     |
| `Sandbox::for_comparator`  | 60 s  | 45 s | 1 GiB         | 8 MiB      | available, not currently wired |

Decoders get the strictest envelope because they are the targets of
hostile or fuzzed bitstreams. Encoders (ITU `jbig2 -encode`, `jbig2enc`)
are looser to accommodate slow reference implementations. The
comparator preset shares the encoder envelope and is kept as an
available preset for future external image comparators or interop
tooling; the conformance matrix performs comparisons in-process.

## Kill reasons

Every run returns a `SandboxOutcome` whose `kill_reason` is
`Some(_)` when we proactively terminated the child:

| `KillReason`        | Cause                                                 |
|---------------------|-------------------------------------------------------|
| `Timeout`           | Watchdog hit `wall_seconds`.                          |
| `AddressSpace`      | Heuristic OOM detection from stderr / `SIGABRT+malloc`. |
| `OutputBytes`       | stdout or stderr exceeded `output_bytes`.             |

The conformance matrix maps these into matrix-cell vocabulary
(`TIMEOUT`, `OOM`, `OUTCAP`); sanitizer hits in stderr surface as
`SAN(ASAN|UBSAN|LSAN|TSAN|MSAN)`; raw signal exits surface as
`SIG=<n>`.

## Backends

- `bwrap` (Linux) – `--unshare-all --die-with-parent --new-session
  --cap-drop ALL --clearenv`. Read-only binds for `/usr`, `/etc`,
  `/proc`, `/dev`, plus any caller-declared `ro_path` / `rw_path`. We
  probe `bwrap` once at startup and fall back to passthrough if the
  kernel refuses the unshare (common inside other sandboxes / CI
  containers).
- `sandbox-exec` (macOS) – `(allow default)` baseline with
  `(deny network*)` (loopback still allowed) and
  `(deny file-write*)` plus targeted writes to `/private/tmp`,
  `/private/var/folders`, and the caller's `rw_path`s. macOS's
  deny-default profiles need such an enormous allow-list (Mach lookups,
  dyld shared cache, sysctl, etc.) that allow-default plus targeted
  denies is the only practical option. Like the Linux backend, this is
  probed once and downgraded to passthrough if `sandbox_apply` is
  refused.
- `passthrough` – the bare `Command`. Watchdog and `setrlimit` still
  apply; only filesystem / network isolation is missing.

## Environment overrides

Defaults can be relaxed at runtime without rebuilding. All overrides are
parsed once per `Sandbox::detect()` call.

| Variable                          | Effect                                              |
|-----------------------------------|-----------------------------------------------------|
| `JBIG2_SANDBOX_WALL_SECS`         | Override `Limits::wall_seconds`.                    |
| `JBIG2_SANDBOX_CPU_SECS`          | Override `Limits::cpu_seconds`.                     |
| `JBIG2_SANDBOX_AS_BYTES`          | Override `Limits::address_space_bytes`.             |
| `JBIG2_SANDBOX_OUTPUT_BYTES`      | Override `Limits::output_bytes`.                    |
| `JBIG2_SANDBOX_BACKEND`           | Force a backend: `bwrap`, `sandbox-exec`, `passthrough` (`off`/`none`). |
| `JBIG2_SANDBOX_DISABLE_RLIMITS`   | `1` skips the `setrlimit` `pre_exec` hook (debug-only). |
| `JBIG2_SANDBOX_EXTRA_RO`          | Read by the test oracles to add extra `ro_path`s; colon-separated like `PATH`. |

## When the matrix shows a new failure

1. The cell label tells you which layer fired:
   `TIMEOUT` → watchdog; `OOM` → AS limit (or the OOM heuristic);
   `OUTCAP` → drain threads; `SAN(...)` → sanitizer; `SIG=11` →
   raw segfault; `exited <status>: ...` → ordinary non-zero exit.
2. Re-run the failing target with `JBIG2_SANDBOX_BACKEND=passthrough`
   to confirm the failure is intrinsic to the tool rather than the
   sandbox profile (e.g. dyld cannot read a needed library).
3. If the tool genuinely needs a wider envelope, bump the relevant
   `JBIG2_SANDBOX_*` variable for that single invocation rather than
   relaxing the preset for everyone.

## Adding a new external tool

1. Pick the right preset (decoder vs encoder; reuse `for_comparator`
   for an external image comparator if one is reintroduced).
2. Declare every input path with `ro_path` and every output / scratch
   directory with `rw_path`.
3. Route the call through `run_external` in
   `tools/conformance/main.rs`, or `Sandbox::for_decoder().run(cmd)`
   in tests, so the existing `KillReason → matrix-cell` translation
   applies.
4. Do **not** sandbox the build step that produces the binary; only
   the consumption of untrusted bitstreams needs isolation.
