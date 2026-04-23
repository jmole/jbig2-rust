# Benchmarks

This directory hosts two Criterion bench targets:

| Target | File | Purpose |
|--------|------|---------|
| `codec` | [`codec.rs`](codec.rs) | In-process Rust microbenchmarks — the authoritative signal for internal hot-path work. |
| `reference_codec` | [`reference_codec.rs`](reference_codec.rs) | Wall-clock comparison of this crate against external reference JBIG2 encoders and decoders. Includes subprocess startup. |

The two targets are deliberately separate so subprocess overhead from the reference comparison never leaks into `codec`'s headline numbers.

## Running

```sh
CARGO_HOME=./.cargo cargo bench --bench codec
CARGO_HOME=./.cargo cargo bench --bench reference_codec
```

Criterion's usual knobs apply, e.g. `-- --noplot` if you don't need the HTML reports. **Avoid `--quick` on `reference_codec`** — in Criterion 0.5 that flag skips persisting `estimates.json` and the HTML report entirely, which means the Rust rows in the cross-tool summary lose their Criterion timings. Use `-- --sample-size 10 --measurement-time 2 --warm-up-time 1` for a similarly fast run that still produces disk output.

## `reference_codec` — what it measures

The bench runs the same inputs through three codecs per side:

- **`rust`** — this crate, in-process (`Jbig2Decoder` / `Jbig2Encoder`).
- **`t88`** — the normative ITU-T T.88 sample software bundled under `vendor/T-REC-T.88-201808`.
- **`jbig2dec`** / **`jbig2enc`** — Ghostscript `jbig2dec` (decoder) and Agl's `jbig2enc` (encoder), both available via Homebrew.

Groups and cases:

| Group | Case | Rust path | Reference paths |
|-------|------|-----------|-----------------|
| `decode/reference` | `tt9_mmr` (`F01_200_TT9.jb2`) | `Jbig2Decoder::decode_page` | T.88 `jbig2` decode, `jbig2dec` |
| `decode/reference` | `tt10_arith` (`F01_200_TT10.jb2`) | same | same |
| `encode/reference` | `tt9_page` (`F01_200_TT9_TT00.bmp`) | `Jbig2Encoder::fast()` | T.88 `jbig2` encode, `jbig2enc` |
| `encode/reference` | `tt10_page` (`F01_200_TT10_TT00.bmp`) | same | same |

Throughput is reported in **page bytes** (`width * height / 8`) so decode and encode are directly comparable even when compressed sizes differ.

Before timing, each reference tool is run once and its output is round-tripped (reference BMP for decode, source bitmap for encode) against the pristine bitmap. A silently-wrong tool therefore fails loudly instead of showing up as "fast".

### Summary table and HTML report

After Criterion finishes the bench writes a cross-tool summary to stderr grouped by `(side, case)`, followed by a `file://` link to Criterion's HTML report:

```
================ Reference codec comparison ================

decode/tt9_mmr
  tool               mean     throughput        bytes     ratio
  -------------------------------------------------------------
  rust           1.151 ms    418.7 MiB/s        16699    30.25x
  jbig2dec       3.416 ms    141.1 MiB/s        16699    30.25x
  t88           43.832 ms     11.0 MiB/s        16699    30.25x
...
============================================================

Full Criterion HTML report:
  file:///.../target/criterion/report/index.html
  file:///.../target/criterion/decode_reference/report/index.html
  file:///.../target/criterion/encode_reference/report/index.html
```

The table's timing column is populated from two sources:

- **`rust` rows** parse Criterion's own `target/criterion/<group>/rust/<case>/new/estimates.json` files, so they stay aligned with Criterion's regression tracking.
- **External-tool rows** (`t88`, `jbig2enc`, `jbig2dec`) are sampled directly by the harness as informational wall-clock measurements and are not registered as Criterion benchmark IDs.

Byte sizes and compression ratios are captured synchronously during each tool's correctness probe, so they appear even for tools whose timing source is missing.

Resolution of the Criterion output directory mirrors Criterion's own logic: `$CRITERION_HOME`, then `$CARGO_TARGET_DIR/criterion`, then `./target/criterion`. This matters in sandboxed development environments where `CARGO_TARGET_DIR` is redirected outside the workspace.

### Cross-tool grouped bar chart

Criterion renders one chart per `(group, function)` node, so comparing rust vs `jbig2enc` vs T.88 at a glance means opening three tabs and eyeballing three different axes. The bench harness also emits a single SVG that overlays all tools per case:

```
<criterion_out>/reference_codec_chart.svg
```

Layout:

- Two stacked subcharts, decode on top, encode below.
- X-axis: one slot per case (`tt9_mmr`, `tt10_arith` / `tt9_page`, `tt10_page`).
- Y-axis: throughput in MiB/s on a **log scale** — the dynamic range between rust (~400 MiB/s) and T.88 (~3 MiB/s) is ~140×, which a linear scale flattens into invisible slivers.
- Bars grouped by case, colored by tool (rust = blue, `jbig2{enc,dec}` = orange, T.88 = purple), with absolute MiB/s values printed above each bar so log-scale distortion doesn't hide the number.
- Tools that were skipped this run (missing binary, filtered out) don't appear in the legend — the chart surface matches what the harness actually measured.

The SVG is plotted with `plotters` (a dev-dep we already pay for transitively via Criterion's plotting backend), so no extra tooling is required. Open the path printed to stderr in any browser.

## External binary discovery

| Variable | Default | Notes |
|----------|---------|-------|
| `JBIG2_T88_BIN` | `vendor/T-REC-T.88-201808/Software/JBIG2_SampleSoftware-A20180829/source/jbig2` | Built by the T.88 sample `Makefile`. The vendored tree ships a prebuilt macOS arm64 binary; on other hosts run `make` there first. |
| `JBIG2ENC_BIN` | `/opt/homebrew/bin/jbig2` (or `/usr/local/bin/jbig2`, or `jbig2enc` on `PATH`) | The Homebrew package is called `jbig2enc` but installs itself as `jbig2`, colliding with the T.88 binary name. The discovery probe runs `-V` and accepts the candidate only if its output identifies as `jbig2enc`. |
| `JBIG2DEC_BIN` | `/opt/homebrew/bin/jbig2dec` (or `/usr/local/bin/jbig2dec`, or `jbig2dec` on `PATH`) | Probed the same way. |

If a tool is missing, its bench cases are cleanly skipped with a `decode/reference: ... not found; ...` line on stderr and the run continues. Similarly, cases whose corpus files are absent are skipped instead of panicking.

Install the Homebrew tools with:

```sh
brew install jbig2enc jbig2dec
```

Build the T.88 reference from the vendor tree:

```sh
cd vendor/T-REC-T.88-201808/Software/JBIG2_SampleSoftware-A20180829
make
```

## Authoritative metrics

- `codec` end-to-end and micro groups — in-process decoder/encoder throughput for this crate.
- `decode/reference/rust/*` and `encode/reference/rust/*` — direct apples-to-apples for the crate's public `Jbig2Decoder` / `Jbig2Encoder` API under the same page bytes throughput as the reference tools.
- `decode/reference/{t88,jbig2dec}` and `encode/reference/{t88,jbig2enc}` — process-level times including startup. Useful as a sanity ceiling and wall-clock comparison context, not as a stopwatch of the reference's internal codec loop and not as tracked project regressions.

## Scratch layout

External tools write into `target/bench_tmp/<case>/`. The directory is recreated clean at setup so stale outputs cannot survive between runs.
