# Security Policy

`jbig2-rust` parses JBIG2 streams, including streams embedded in PDFs and
other document formats. Treat those inputs as attacker-controlled.

## Scope

Security reports are in scope for:

- `Jbig2Decoder` and all segment parsing, arithmetic coding, Huffman coding,
  MMR, refinement, text, halftone, and colour-palette decode paths.
- The optional `image` feature, including the `image` crate decoding hook.
- The command-line binaries in this repository when they process untrusted
  JBIG2 streams.
- Resource-exhaustion bugs that can cause unbounded CPU, memory, stack, or
  output growth on hostile input.

The encoder is in scope for panics, memory safety, and unbounded resource use,
but it does not currently claim confidentiality or integrity properties over
maliciously chosen source images.

Vendored third-party tools under `vendor/` are out of scope for this project.
Please report vulnerabilities in those tools to their upstream maintainers.
This repository does sandbox those tools when using them as differential
oracles.

## Reporting

Use GitHub Security Advisories for private reports:

https://github.com/jmole/jbig2-rust/security/advisories/new

Please include:

- A reproducer stream or enough detail to construct one.
- The command or API call that triggers the issue.
- Expected versus observed behavior.
- Whether the issue is a crash, panic, infinite loop, excessive allocation, or
  semantic correctness problem.

The maintainer will try to acknowledge reports within 7 days. Fix timing
depends on severity and release status; for high-severity issues, the target is
to land a fix or documented workaround within 90 days.

## Current Mitigations

- The Rust decode path is written in safe Rust. The library denies unsafe
  operations, and safe modules forbid unsafe code. The remaining `unsafe` in
  `src/` is limited to Unix process-control calls in `src/util/sandbox.rs`.
- External decoders and encoders used by tests and conformance tooling run
  through `jbig2::util::sandbox`, which applies wall-clock timeouts, resource
  limits, output caps, and filesystem/network isolation where the platform
  supports it.
- The test suite includes deterministic conformance, corpus, and differential
  checks against external JBIG2 implementations.

## Non-Goals

This project is pre-1.0 and not published to crates.io yet. The current goal is
to make correctness and safety evidence visible before publishing a stable API.
