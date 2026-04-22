//! Pure-Rust JBIG2 (ITU-T T.88 / ISO/IEC 14492) encoder and decoder.
//!
//! # Scope of v0.1
//!
//! | Spec area | Decoder | Encoder |
//! |---|---|---|
//! | File header (D.4) / segment headers (7.2) | done | done |
//! | Page information + end-of-page (7.4.8, 7.4.9) | done | done |
//! | Generic region ? arithmetic (6.2.5, templates 0..3) | done | done |
//! | Generic region ? TPGD duplicate-line removal | done | done |
//! | Generic region ? extended 12-AT template (AMD2) | done | done |
//! | Generic region ? MMR (T.6) | partial? | done |
//! | Symbol dictionary (7.4.2) ? arithmetic | done | done |
//! | Symbol dictionary ? Huffman | deferred? | deferred? |
//! | Text region (7.4.3) ? arithmetic, no refinement | done | done |
//! | Text region ? Huffman / refinement | deferred? | deferred? |
//! | Connected-component extraction + identity classifier | ? | done |
//! | Lossy classifier (WXOR, size-bucket accel) | ? | done |
//! | Generic refinement (6.3, 7.4.7) | deferred? | deferred? |
//! | Pattern dictionary + halftone (7.4.4, 7.4.5) | deferred? | deferred? |
//! | Colour palette (AMD3) | deferred? | deferred? |
//!
//! Footnotes:
//! * ? Self-encoded T.6 MMR round-trips cleanly; the T.88 TT9 reference
//!   stream exposes an unresolved desync on long white make-ups at line 66.
//! * ? TT1..TT3 exercise Huffman-coded SD/TR; every real-world encoder
//!   (`jbig2enc -S` included) uses the arithmetic path which is fully
//!   implemented and round-tripped by [`Mode::SymbolLossless`].
//! * ? Generic refinement (6.3) + refined symbol/text regions unlock
//!   TT4..TT7. The arithmetic building blocks are shared with the generic
//!   region coder, so this is primarily wiring work.
//! * ? Halftone / pattern dictionary (TT8) + AMD3 colour palette are
//!   niche for patent-TIFF workflows; tracked for a later crate version.
//!
//! # Crate layout
//!
//! * [`coding`] ? MQ arithmetic coder, IA* + IAID integer coders, the 15
//!   standard Huffman tables, T.4/T.6 MH / MMR.
//! * [`segments`] ? per-segment-type parse / emit routines.
//! * [`Jbig2Decoder`] ? multi-page top-level decoder over `Read + Seek`.
//! * [`Jbig2Encoder`] ? single-page writer over `Write`, with tunable
//!   [`EncoderConfig`] presets ([`EncoderConfig::fast`],
//!   [`EncoderConfig::balanced`], [`EncoderConfig::max_compression`]).
//! * [`image_plugin`] (feature = "image") ? `image`-crate [`register`]
//!   entry point and `ImageDecoder` adapter.
//! * `bin/jbig2` (feature = "cli") ? command-line encoder / decoder / info
//!   tool.
//!
//! # Feature flags
//!
//! * `image` (default) ? `image`-crate plugin, pulls in the `image = "0.25"`
//!   dependency.
//! * `mmr` (default) ? T.4 / T.6 line-coding paths.
//! * `rayon` ? parallel classifier and multi-page encode (reserved for M7).
//! * `cli` ? builds the `jbig2` binary (pulls in `clap` + `anyhow`).
//!
//! # Benchmarks
//!
//! Criterion benches live in `benches/codec.rs`. Run:
//!
//! ```text
//! cargo bench --bench codec
//! ```
//!
//! to measure encode + decode throughput on synthetic stripe / glyph
//! bitmaps. SIMD context extraction and a Rayon-accelerated classifier
//! are gated for a future release once the functional deferrals above
//! are closed out.
//!
//! # Example
//!
//! ```no_run
//! use std::fs::File;
//! use jbig2::{Jbig2Decoder, Jbig2Encoder, EncoderConfig, Bitmap};
//! use std::io::{BufReader, BufWriter};
//!
//! // Decode
//! let mut dec = Jbig2Decoder::new(BufReader::new(File::open("input.jb2")?))?;
//! let page = dec.decode_page(1)?;
//!
//! // Encode
//! let out = File::create("output.jb2")?;
//! let mut enc = Jbig2Encoder::new(BufWriter::new(out), EncoderConfig::max_compression());
//! enc.write_page(&page.bitmap)?;
//! enc.finish()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod bitmap;
pub mod coding;
pub mod decoder;
pub mod encoder;
pub mod error;
pub mod segments;
pub mod symbol;

pub use bitmap::Bitmap;
pub use decoder::{DecodedPage, Jbig2Decoder};
pub use encoder::{Coding, EncoderConfig, GenericTemplate, Jbig2Encoder, Mode};
pub use error::{Jbig2Error, Jbig2Result};

/// JBIG2 file magic: `97 4A 42 32 0D 0A 1A 0A`.
pub const JBIG2_MAGIC: [u8; 8] = [0x97, 0x4A, 0x42, 0x32, 0x0D, 0x0A, 0x1A, 0x0A];

#[cfg(feature = "image")]
#[cfg_attr(docsrs, doc(cfg(feature = "image")))]
pub mod image_plugin;

#[cfg(feature = "image")]
#[cfg_attr(docsrs, doc(cfg(feature = "image")))]
pub use image_plugin::register;
