use crate::types::{
    ApplyPatchTextRequest, CheckConflictsRequest, InsertAfterStmtRequest, ReplaceNodeRequest,
    ScalarValue, SetPathValueRequest, ValueKind,
};
use crate::{ToolError, ToolWorkspace};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DraxlMcpServer {
    workspace: ToolWorkspace,
    tool_router: ToolRouter<Self>,
}

impl DraxlMcpServer {
    pub fn new(root: impl Into<PathBuf>) -> crate::Result<Self> {
        let workspace = ToolWorkspace::new(root)?;
        Ok(Self {
            workspace,
            tool_router: Self::tool_router(),
        })
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DraxlMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Draxl edit tools. Inspect files normally and use draxl_* tools for semantic edits.",
        )
    }
}

#[tool_router(router = tool_router)]
impl DraxlMcpServer {
    #[tool(
        name = "draxl_inspect_file",
        description = "Read a .rs.dx file and return source, fingerprint, and node summaries."
    )]
    async fn inspect_file(
        &self,
        Parameters(InspectFileParams { path }): Parameters<InspectFileParams>,
    ) -> Result<String, String> {
        json_text(self.workspace.inspect_file(&path))
    }

    #[tool(
        name = "draxl_get_node",
        description = "Read one node by stable id and return its subtree plus legal edit affordances."
    )]
    async fn get_node(
        &self,
        Parameters(GetNodeParams { path, node_id }): Parameters<GetNodeParams>,
    ) -> Result<String, String> {
        json_text(self.workspace.get_node(&path, &node_id))
    }

    #[tool(
        name = "draxl_replace_node",
        description = "Replace an existing node with a source fragment like `audit()`."
    )]
    async fn replace_node(
        &self,
        Parameters(params): Parameters<ReplaceNodeParams>,
    ) -> Result<String, String> {
        json_text(self.workspace.replace_node(ReplaceNodeRequest {
            path: &params.path,
            target_id: &params.target_id,
            fragment_source: &params.fragment_source,
            expected_fingerprint: params.expected_fingerprint.as_deref(),
            apply: params.apply,
        }))
    }

    #[tool(
        name = "draxl_insert_after_stmt",
        description = "Insert a statement after an existing statement id using plain source like `trace();`."
    )]
    async fn insert_after_stmt(
        &self,
        Parameters(params): Parameters<InsertAfterStmtParams>,
    ) -> Result<String, String> {
        json_text(self.workspace.insert_after_stmt(InsertAfterStmtRequest {
            path: &params.path,
            anchor_id: &params.anchor_id,
            stmt_source: &params.stmt_source,
            expected_fingerprint: params.expected_fingerprint.as_deref(),
            apply: params.apply,
        }))
    }

    #[tool(
        name = "draxl_set_path",
        description = "Set a simple scalar path such as a name, text, op, or semi flag."
    )]
    async fn set_path(
        &self,
        Parameters(params): Parameters<SetPathParams>,
    ) -> Result<String, String> {
        let value_kind = match params.value_kind.as_str() {
            "ident" => ValueKind::Ident,
            "string" => ValueKind::String,
            "bool" => ValueKind::Bool,
            "int" => ValueKind::Int,
            other => return Err(format!("unsupported value_kind `{other}`")),
        };

        let value = match (value_kind, &params.value) {
            (ValueKind::Ident, Value::String(text)) => ScalarValue::Ident(text),
            (ValueKind::String, Value::String(text)) => ScalarValue::String(text),
            (ValueKind::Bool, Value::Bool(flag)) => ScalarValue::Bool(*flag),
            (ValueKind::Int, Value::Number(number)) => {
                let Some(value) = number.as_i64() else {
                    return Err("int value must fit in i64".to_owned());
                };
                ScalarValue::Int(value)
            }
            _ => {
                return Err(
                    "value does not match value_kind; expected string/bool/int payload".to_owned(),
                )
            }
        };

        json_text(self.workspace.set_path_value(SetPathValueRequest {
            path: &params.path,
            node_id: &params.node_id,
            field: &params.field,
            value_kind,
            value,
            expected_fingerprint: params.expected_fingerprint.as_deref(),
            apply: params.apply,
        }))
    }

    #[tool(
        name = "draxl_apply_patch_text",
        description = "Apply raw Draxl patch text. Prefer higher-level draxl_* edit tools first."
    )]
    async fn apply_patch_text(
        &self,
        Parameters(params): Parameters<ApplyPatchTextParams>,
    ) -> Result<String, String> {
        json_text(self.workspace.apply_patch_text(ApplyPatchTextRequest {
            path: &params.path,
            patch_text: &params.patch_text,
            expected_fingerprint: params.expected_fingerprint.as_deref(),
            apply: params.apply,
        }))
    }

    #[tool(
        name = "draxl_check_conflicts",
        description = "Compare two patch bundles against the same file and return Draxl conflict JSON."
    )]
    async fn check_conflicts(
        &self,
        Parameters(params): Parameters<CheckConflictsParams>,
    ) -> Result<String, String> {
        json_text(self.workspace.check_conflicts(CheckConflictsRequest {
            path: &params.path,
            left_patch_text: &params.left_patch_text,
            right_patch_text: &params.right_patch_text,
        }))
    }
}

pub async fn serve_stdio(root: impl Into<PathBuf>) -> crate::Result<()> {
    let server = DraxlMcpServer::new(root)?;
    let running = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(rmcp_error)?;
    let _ = running
        .waiting()
        .await
        .map_err(|err| ToolError::new(format!("mcp server join failed: {err}")))?;
    Ok(())
}

fn json_text<T>(result: crate::Result<T>) -> Result<String, String>
where
    T: serde::Serialize,
{
    let value = result.map_err(|err| err.to_string())?;
    serde_json::to_string_pretty(&value).map_err(|err| err.to_string())
}

fn rmcp_error(err: impl std::fmt::Display) -> ToolError {
    ToolError::new(format!("mcp server error: {err}"))
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct InspectFileParams {
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetNodeParams {
    path: String,
    node_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ReplaceNodeParams {
    path: String,
    target_id: String,
    fragment_source: String,
    expected_fingerprint: Option<String>,
    #[serde(default = "default_true")]
    apply: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct InsertAfterStmtParams {
    path: String,
    anchor_id: String,
    stmt_source: String,
    expected_fingerprint: Option<String>,
    #[serde(default = "default_true")]
    apply: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SetPathParams {
    path: String,
    node_id: String,
    field: String,
    value_kind: String,
    value: Value,
    expected_fingerprint: Option<String>,
    #[serde(default = "default_true")]
    apply: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ApplyPatchTextParams {
    path: String,
    patch_text: String,
    expected_fingerprint: Option<String>,
    #[serde(default = "default_true")]
    apply: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CheckConflictsParams {
    path: String,
    left_patch_text: String,
    right_patch_text: String,
}
