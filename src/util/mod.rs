//! Internal utility modules shared by the library, binary tools, and the
//! integration test suite.
//!
//! Everything in this module is gated behind a feature flag so it never
//! ships to crate consumers that only want the encoder/decoder.

#[cfg(feature = "sandbox-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "sandbox-runtime")))]
pub mod sandbox;

#[cfg(any(feature = "conformance-tools", feature = "validator-corpus"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "conformance-tools", feature = "validator-corpus")))
)]
pub mod vendor_anchor;
