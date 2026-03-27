#![forbid(unsafe_code)]
//! Merge analysis for Draxl patch streams.
//!
//! The initial API focuses on hard conflicts and returns structured
//! explanations suitable for both humans and agents.

mod context;
mod detect;
mod explain;
mod model;
mod render;
mod semantic;

pub use detect::{
    check_conflicts, check_conflicts_for_language, check_hard_conflicts,
    check_hard_conflicts_for_language,
};
pub use model::{
    Conflict, ConflictClass, ConflictCode, ConflictOwner, ConflictRegion, ConflictReport,
    ConflictSide, HardConflictReport, ReplayOrder, ReplayStage,
};
