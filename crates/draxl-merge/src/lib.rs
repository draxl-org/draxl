#![forbid(unsafe_code)]
//! Merge analysis for Draxl patch streams.
//!
//! The initial API focuses on hard conflicts and returns structured
//! explanations suitable for both humans and agents.

mod detect;
mod explain;
mod model;
mod render;

pub use detect::check_hard_conflicts;
pub use model::{
    Conflict, ConflictClass, ConflictCode, ConflictSide, HardConflictReport, ReplayOrder,
    ReplayStage,
};
