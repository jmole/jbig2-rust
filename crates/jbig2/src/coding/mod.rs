//! Low-level coding primitives used by JBIG2 segments.
#![forbid(unsafe_code)]

pub mod huffman;
pub mod mmr;
#[cfg(feature = "mmr")]
pub mod mmr_lut;
pub mod mq;
pub mod mq_context;
pub mod mq_integer;
