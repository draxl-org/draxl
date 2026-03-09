#![forbid(unsafe_code)]
//! Structured patch operations over the Draxl semantic model.
//!
//! The patch layer applies semantic edit operators directly to the Draxl AST.
//! It is intentionally slot-aware, identity-aware, and attachment-aware so
//! callers can express concurrent-friendly structural edits instead of
//! rewriting text.

mod apply;
mod error;
mod model;

pub use apply::{apply_op, apply_ops};
pub use error::PatchError;
pub use model::{
    PatchDest, PatchNode, PatchOp, PatchPath, PatchValue, RankedDest, SlotOwner, SlotRef,
};
