#![forbid(unsafe_code)]
//! Structured patch operations over ranked Draxl slot children.
//!
//! The patch layer applies typed edit operators directly to the Draxl AST. It
//! is intentionally slot-aware and rank-aware so callers can express
//! concurrent-friendly structural edits instead of rewriting text.

mod apply;
mod error;
mod model;

pub use apply::{apply_op, apply_ops};
pub use error::PatchError;
pub use model::{PatchNode, PatchOp, PatchParent};
