use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct FileInspection {
    pub ok: bool,
    pub path: String,
    pub fingerprint: String,
    pub source: String,
    pub node_count: usize,
    pub nodes: Vec<NodeSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeSummary {
    pub id: String,
    pub kind: String,
    pub parent_id: Option<String>,
    pub slot: Option<String>,
    pub rank: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeDetail {
    pub ok: bool,
    pub path: String,
    pub fingerprint: String,
    pub node_id: String,
    pub kind: String,
    pub parent_id: Option<String>,
    pub slot: Option<String>,
    pub rank: Option<String>,
    pub legal: LegalInfo,
    pub node: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct LegalInfo {
    pub ranked_slots: Vec<&'static str>,
    pub single_slots: Vec<&'static str>,
    pub set_paths: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatchApplicationResult {
    pub ok: bool,
    pub path: String,
    pub applied: bool,
    pub before_fingerprint: String,
    pub after_fingerprint: String,
    pub patch_text: String,
    pub preview_dx: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConflictCheckResult {
    pub ok: bool,
    pub path: String,
    pub conflicts: Value,
}

#[derive(Debug, Clone, Copy)]
pub enum ValueKind {
    Ident,
    String,
    Bool,
    Int,
}

#[derive(Debug, Clone, Copy)]
pub enum ScalarValue<'a> {
    Ident(&'a str),
    String(&'a str),
    Bool(bool),
    Int(i64),
}

pub struct ReplaceNodeRequest<'a> {
    pub path: &'a str,
    pub target_id: &'a str,
    pub fragment_source: &'a str,
    pub expected_fingerprint: Option<&'a str>,
    pub apply: bool,
}

pub struct InsertAfterStmtRequest<'a> {
    pub path: &'a str,
    pub anchor_id: &'a str,
    pub stmt_source: &'a str,
    pub expected_fingerprint: Option<&'a str>,
    pub apply: bool,
}

pub struct SetPathValueRequest<'a> {
    pub path: &'a str,
    pub node_id: &'a str,
    pub field: &'a str,
    pub value_kind: ValueKind,
    pub value: ScalarValue<'a>,
    pub expected_fingerprint: Option<&'a str>,
    pub apply: bool,
}

pub struct ApplyPatchTextRequest<'a> {
    pub path: &'a str,
    pub patch_text: &'a str,
    pub expected_fingerprint: Option<&'a str>,
    pub apply: bool,
}

pub struct CheckConflictsRequest<'a> {
    pub path: &'a str,
    pub left_patch_text: &'a str,
    pub right_patch_text: &'a str,
}
