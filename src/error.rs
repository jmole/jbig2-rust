//! Error types for the `jbig2` crate.

use std::io;

use thiserror::Error;

/// Errors produced by JBIG2 parse / decode / encode operations.
#[derive(Debug, Error)]
pub enum Jbig2Error {
    /// An I/O error occurred while reading or writing a stream.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// The stream does not start with the JBIG2 magic bytes.
    #[error("invalid JBIG2 magic; expected 97 4A 42 32 0D 0A 1A 0A")]
    InvalidMagic,

    /// The stream ended mid-segment.
    #[error("unexpected end of stream (need {needed} more bytes)")]
    UnexpectedEof {
        /// Number of bytes still needed from the stream.
        needed: usize,
    },

    /// A segment header field contained an invalid or unsupported value.
    #[error("invalid segment header: {0}")]
    InvalidSegmentHeader(&'static str),

    /// The decoded pixel geometry is inconsistent or exceeds a limit.
    #[error("invalid geometry: {0}")]
    InvalidGeometry(&'static str),

    /// A feature required by the bitstream is not yet implemented.
    #[error("unsupported feature: {0}")]
    Unsupported(&'static str),

    /// The arithmetic coder reached an impossible state.
    #[error("arithmetic coder desync: {0}")]
    ArithmeticDesync(&'static str),

    /// A Huffman-coded field did not match any entry in the selected table.
    #[error("invalid Huffman code: {0}")]
    InvalidHuffman(&'static str),

    /// A bitstream value was out of the range permitted by the spec.
    #[error("out-of-range value: {0}")]
    OutOfRange(&'static str),

    /// The encoder was misconfigured (conflicting options, etc.).
    #[error("invalid encoder configuration: {0}")]
    InvalidConfig(&'static str),
}

/// `Result<T, Jbig2Error>` convenience alias.
pub type Jbig2Result<T> = Result<T, Jbig2Error>;
