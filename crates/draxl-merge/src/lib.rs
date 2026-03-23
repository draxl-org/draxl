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

pub use detect::{check_conflicts, check_hard_conflicts};
pub use model::{
    Conflict, ConflictClass, ConflictCode, ConflictReport, ConflictSide, HardConflictReport,
    ReplayOrder, ReplayStage,
};
