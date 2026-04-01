mod ranks_smoke;

use crate::error::{Result, ToolError};
use crate::types::{FileInspection, PatchApplicationResult};
use crate::ToolWorkspace;
use serde::Serialize;

pub fn names() -> &'static [&'static str] {
    &[ranks_smoke::NAME]
}

pub fn run_named(name: &str, workspace: &ToolWorkspace) -> Result<ScenarioRun> {
    match name {
        ranks_smoke::NAME => ranks_smoke::run(workspace),
        _ => Err(ToolError::new(format!("unknown itest case `{name}`"))),
    }
}

#[derive(Debug, Serialize)]
pub struct ScenarioRun {
    pub case_name: String,
    pub inspection: FileInspection,
    pub step_runs: Vec<ScenarioStepRun>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScenarioStepRun {
    InsertAfterStmt {
        path: String,
        anchor_id: String,
        result: PatchApplicationResult,
    },
    ReplaceNode {
        path: String,
        target_id: String,
        result: PatchApplicationResult,
    },
    SetPathValue {
        path: String,
        node_id: String,
        field: String,
        result: PatchApplicationResult,
    },
}
