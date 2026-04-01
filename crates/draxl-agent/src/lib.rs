#![forbid(unsafe_code)]

mod error;
pub mod mcp;
mod support;
mod types;
mod workspace;

pub use error::{Result, ToolError};
pub use types::{
    ApplyPatchTextRequest, CheckConflictsRequest, ConflictCheckResult, FileInspection,
    InsertAfterStmtRequest, LegalInfo, NodeDetail, NodeSummary, PatchApplicationResult,
    ReplaceNodeRequest, ScalarValue, SetPathValueRequest, ValueKind,
};
pub use workspace::ToolWorkspace;
