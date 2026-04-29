//! Symbol extraction + dictionary construction for the symbol-based
//! encoding path.
//!
//! This module:
//!
//! * Extracts 4/8-connected components from a page bitmap ([`cc`]).
//! * Builds an identity-matching lossless dictionary ([`identity`]).
#![forbid(unsafe_code)]

pub mod cc;
pub mod classify;
pub mod identity;
