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
mod schema;
mod text;

pub use apply::{apply_op, apply_op_for_language, apply_ops, apply_ops_for_language};
pub use error::PatchError;
pub use model::{
    PatchDest, PatchNode, PatchOp, PatchPath, PatchValue, RankedDest, SlotOwner, SlotRef,
};
pub use text::{
    apply_patch_text, apply_patch_text_for_language, parse_patch_ops, resolve_patch_ops,
    resolve_patch_ops_for_language, PatchTextError, SurfaceDest, SurfaceFragment, SurfaceNodeRef,
    SurfacePatchOp, SurfacePath, SurfacePathSegment, SurfaceRankedDest, SurfaceSlotOwner,
    SurfaceSlotRef, SurfaceValue, SurfaceValueKind,
};
