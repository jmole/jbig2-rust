//! Pure-Rust JBIG2 (ITU-T T.88 / ISO/IEC 14492) encoder and decoder.
//!
//! The public API intentionally uses [`image`] crate types: decoding returns
//! [`image::DynamicImage`], and encoding accepts [`image::GrayImage`].
//!
//! ```no_run
//! use std::fs::File;
//! use std::io::{BufReader, BufWriter};
//! use jbig2::{EncoderConfig, Jbig2Decoder, Jbig2Encoder};
//!
//! let mut dec = Jbig2Decoder::new(BufReader::new(File::open("input.jb2")?))?;
//! let page = dec.decode_page(1)?;
//! let gray = page.into_luma8();
//!
//! let out = File::create("output.jb2")?;
//! let mut enc = Jbig2Encoder::new(BufWriter::new(out), EncoderConfig::max_compression());
//! enc.write_page(&gray)?;
//! enc.finish()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
#![warn(missing_docs)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::all)]
#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "__workbench")]
pub mod bitmap;
#[cfg(not(feature = "__workbench"))]
pub(crate) mod bitmap;

#[cfg(feature = "__workbench")]
pub mod coding;
#[cfg(not(feature = "__workbench"))]
pub(crate) mod coding;

#[cfg(feature = "__workbench")]
pub mod decoder;
#[cfg(not(feature = "__workbench"))]
pub(crate) mod decoder;

#[cfg(feature = "__workbench")]
pub mod encoder;
#[cfg(not(feature = "__workbench"))]
pub(crate) mod encoder;

pub(crate) mod error;

#[cfg(feature = "__workbench")]
pub mod image_plugin;
#[cfg(not(feature = "__workbench"))]
pub(crate) mod image_plugin;

#[cfg(feature = "__workbench")]
pub mod rgb_bitmap;
#[cfg(not(feature = "__workbench"))]
pub(crate) mod rgb_bitmap;

#[cfg(feature = "__workbench")]
pub mod segments;
#[cfg(not(feature = "__workbench"))]
pub(crate) mod segments;

#[cfg(feature = "__workbench")]
pub mod symbol;
#[cfg(not(feature = "__workbench"))]
pub(crate) mod symbol;

pub use decoder::Jbig2Decoder;
pub use encoder::{EncoderConfig, Jbig2Encoder};
pub use error::{Jbig2Error, Jbig2Result};
pub use image_plugin::register;

#[cfg(feature = "__workbench")]
pub use bitmap::Bitmap;
#[cfg(feature = "__workbench")]
pub use decoder::DecodedPage;
#[cfg(feature = "__workbench")]
pub use encoder::{
    Coding, GenericTemplate, HalftonePageOptions, Mode, RefinementGate, SymbolCoding,
};
#[cfg(feature = "__workbench")]
pub use rgb_bitmap::RgbBitmap;

/// JBIG2 file magic: `97 4A 42 32 0D 0A 1A 0A`.
pub const JBIG2_MAGIC: [u8; 8] = [0x97, 0x4A, 0x42, 0x32, 0x0D, 0x0A, 0x1A, 0x0A];

#[cfg(feature = "validator")]
#[cfg_attr(docsrs, doc(cfg(feature = "validator")))]
pub mod validator;
