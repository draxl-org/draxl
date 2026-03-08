#![forbid(unsafe_code)]
//! Canonical printer for Draxl Source v0.
//!
//! This crate has two responsibilities:
//!
//! - canonicalize AST containers so ranked children and attached trivia are in
//!   deterministic order
//! - render the canonical tree back into compact Draxl surface syntax
//!
//! Keeping those steps separate makes it easier to reason about whether a
//! change affects semantic ordering, textual formatting, or both.

mod canonical;
mod render;

pub use canonical::canonicalize_file;
pub use render::print_file;
