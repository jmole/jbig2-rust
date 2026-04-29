# jbig2

Pure-Rust JBIG2 (ITU-T T.88 / ISO/IEC 14492) encoder and decoder.

This crate exposes a deliberately small API built on the [`image`](https://crates.io/crates/image) crate. JBIG2 pages decode to `image::DynamicImage`, and encoding accepts `image::GrayImage` input.

## Usage

```rust
use std::fs::File;
use std::io::{BufReader, BufWriter};

use jbig2::{EncoderConfig, Jbig2Decoder, Jbig2Encoder};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut decoder = Jbig2Decoder::new(BufReader::new(File::open("input.jb2")?))?;
let page = decoder.decode_page(1)?;
let gray = page.into_luma8();

let output = File::create("output.jb2")?;
let mut encoder = Jbig2Encoder::new(BufWriter::new(output), EncoderConfig::max_compression());
encoder.write_page(&gray)?;
encoder.finish()?;
# Ok(())
# }
```

Call `jbig2::register()` at program startup to register JBIG2 with `image::open` through image-rs hooks.

## Features

- `mmr` (default): enables T.4 / T.6 line-coding paths.
- `validator`: enables the spec-cited structural validator.
- `rayon`: reserved for future parallel encoder paths.

## Encoding Convention

`Jbig2Encoder::write_page` thresholds `image::GrayImage` input at 128: pixel values below 128 become JBIG2 ink (`1`), and values 128 or above become paper (`0`). Preprocess the image first if you need a different threshold.

## Workbench

The repository also contains an unpublished `jbig2-workbench` crate with conformance tooling, external decoder sandboxes, and corpus utilities.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
