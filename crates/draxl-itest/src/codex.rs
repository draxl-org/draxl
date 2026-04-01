use crate::{ToolError, ToolWorkspace};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{Builder, TempDir};

const DEFAULT_DEVELOPER_INSTRUCTIONS: &str = r#"This is a Draxl Codex end-to-end test.
This workspace is a throwaway integration-test repo.
You are explicitly authorized to modify files inside this workspace through the Draxl MCP tools.

Read files and inspect code normally.
Use the `draxl_*` MCP tools for every code change.
Start by calling `draxl_inspect_file` on the target `.rs.dx` file.
Prefer `draxl_insert_after_stmt`, `draxl_replace_node`, and `draxl_set_path`.
Do not use `draxl_apply_patch_text` unless a higher-level Draxl tool fails.
Do not modify files directly.
Pass `expected_fingerprint` on mutating tool calls when it is available.
If a Draxl tool fails, report the failure instead of falling back to direct file edits."#;

const APPROVED_DRAXL_TOOLS: &[&str] = &[
    "draxl_inspect_file",
    "draxl_get_node",
    "draxl_replace_node",
    "draxl_insert_after_stmt",
    "draxl_set_path",
    "draxl_apply_patch_text",
    "draxl_check_conflicts",
];

pub struct CodexHarness {
    codex_home: TempDir,
    mcp_server_bin: PathBuf,
    model: String,
    developer_instructions: String,
}

impl CodexHarness {
    pub fn new(mcp_server_bin: impl Into<PathBuf>) -> crate::Result<Self> {
        let codex_home = Builder::new()
            .prefix("codex-home-")
            .tempdir_in(target_temp_root()?)
            .map_err(io_error("failed to create isolated CODEX_HOME"))?;
        let harness = Self {
            codex_home,
            mcp_server_bin: canonicalize_path(
                mcp_server_bin.into(),
                "failed to canonicalize MCP server binary",
            )?,
            model: std::env::var("DRAXL_ITEST_CODEX_MODEL")
                .unwrap_or_else(|_| "gpt-5.4".to_owned()),
            developer_instructions: DEFAULT_DEVELOPER_INSTRUCTIONS.to_owned(),
        };
        harness.install_auth()?;
        Ok(harness)
    }

    pub fn codex_home(&self) -> &Path {
        self.codex_home.path()
    }

    pub fn create_workspace(
        &self,
        name: &str,
        relative_path: &str,
        source: &str,
    ) -> crate::Result<CodexWorkspace> {
        self.create_workspace_with_files(name, relative_path, source, &[])
    }

    pub fn create_workspace_with_files(
        &self,
        name: &str,
        relative_path: &str,
        source: &str,
        extra_files: &[(&str, &str)],
    ) -> crate::Result<CodexWorkspace> {
        let tempdir = Builder::new()
            .prefix(&format!("{name}-"))
            .tempdir_in(target_temp_root()?)
            .map_err(io_error("failed to create Codex workspace"))?;
        let root = canonicalize_path(
            tempdir.path(),
            "failed to canonicalize Codex workspace root",
        )?;

        write_workspace_file(&root, relative_path, source)?;
        for (extra_path, extra_source) in extra_files {
            write_workspace_file(&root, extra_path, extra_source)?;
        }
        fs::write(root.join("AGENTS.md"), agents_md())
            .map_err(io_error("failed to write AGENTS.md"))?;
        let git_init = Command::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(&root)
            .output()
            .map_err(io_error("failed to initialize throwaway git repo"))?;
        if !git_init.status.success() {
            let stderr = String::from_utf8_lossy(&git_init.stderr);
            return Err(ToolError::new(format!(
                "failed to initialize throwaway git repo in {}: {}",
                root.display(),
                stderr
            )));
        }

        Ok(CodexWorkspace {
            tempdir,
            root,
            relative_path: relative_path.to_owned(),
        })
    }

    pub fn run_exec_json(
        &self,
        workspace: &CodexWorkspace,
        prompt: &str,
    ) -> crate::Result<CodexExecRun> {
        self.write_config(workspace.root())?;

        let output = Command::new("codex")
            .env("CODEX_HOME", self.codex_home())
            .arg("exec")
            .arg("--ephemeral")
            .arg("--json")
            .arg("--color")
            .arg("never")
            .arg("--skip-git-repo-check")
            .arg("-C")
            .arg(workspace.root())
            .arg("-s")
            .arg("read-only")
            .arg("-c")
            .arg(r#"approval_policy="never""#)
            .arg(prompt)
            .output()
            .map_err(io_error("failed to run codex exec"))?;

        let stdout = String::from_utf8(output.stdout)
            .map_err(|err| ToolError::new(format!("codex exec stdout was not utf-8: {err}")))?;
        let stderr = String::from_utf8(output.stderr)
            .map_err(|err| ToolError::new(format!("codex exec stderr was not utf-8: {err}")))?;

        if !output.status.success() {
            return Err(ToolError::new(format!(
                "codex exec failed with status {}{}\n{}",
                output.status,
                if stderr.is_empty() {
                    ""
                } else {
                    " and stderr:"
                },
                stderr
            )));
        }

        let mut events = Vec::new();
        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let event = serde_json::from_str::<CodexEvent>(line).map_err(|err| {
                ToolError::new(format!("failed to decode codex exec event `{line}`: {err}"))
            })?;
            events.push(event);
        }

        let final_message = events.iter().rev().find_map(|event| {
            event.item.as_ref().and_then(|item| {
                if item.item_type == "agent_message" {
                    item.text.clone()
                } else {
                    None
                }
            })
        });

        Ok(CodexExecRun {
            stdout,
            stderr,
            events,
            final_message,
        })
    }

    fn install_auth(&self) -> crate::Result<()> {
        let auth_source = std::env::var_os("DRAXL_ITEST_CODEX_AUTH_JSON")
            .map(PathBuf::from)
            .or_else(default_auth_path)
            .ok_or_else(|| {
                ToolError::new("could not locate Codex auth.json; set DRAXL_ITEST_CODEX_AUTH_JSON")
            })?;

        let auth_target = self.codex_home().join("auth.json");
        fs::copy(&auth_source, &auth_target).map_err(|err| {
            ToolError::new(format!(
                "failed to copy Codex auth from {} to {}: {err}",
                auth_source.display(),
                auth_target.display()
            ))
        })?;
        Ok(())
    }

    fn write_config(&self, workspace_root: &Path) -> crate::Result<()> {
        let tool_sections = APPROVED_DRAXL_TOOLS
            .iter()
            .map(|tool| format!("[mcp_servers.draxl.tools.{tool}]\napproval_mode = \"approve\"\n"))
            .collect::<Vec<_>>()
            .join("\n");
        let config = format!(
            r#"model = "{model}"
sandbox_mode = "read-only"
approval_policy = "on-request"
developer_instructions = '''
{developer_instructions}
'''

[projects."{workspace_root}"]
trust_level = "trusted"

[mcp_servers.draxl]
command = "{server_bin}"
args = ["--root", "{workspace_root}"]
cwd = "{workspace_root}"
default_tools_approval_mode = "approve"

{tool_sections}
"#,
            model = self.model,
            developer_instructions = self.developer_instructions,
            workspace_root = workspace_root.display(),
            server_bin = self.mcp_server_bin.display(),
            tool_sections = tool_sections,
        );

        fs::write(self.codex_home().join("config.toml"), config)
            .map_err(io_error("failed to write isolated Codex config"))
    }
}

pub struct CodexWorkspace {
    tempdir: TempDir,
    root: PathBuf,
    relative_path: String,
}

impl CodexWorkspace {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn read_source(&self) -> crate::Result<String> {
        fs::read_to_string(self.root.join(&self.relative_path))
            .map_err(io_error("failed to read Codex workspace source"))
    }

    pub fn inspect(&self) -> crate::Result<crate::FileInspection> {
        ToolWorkspace::new(&self.root)?.inspect_file(&self.relative_path)
    }

    #[allow(dead_code)]
    pub fn keep_alive(&self) -> &TempDir {
        &self.tempdir
    }
}

pub struct CodexExecRun {
    pub stdout: String,
    pub stderr: String,
    pub events: Vec<CodexEvent>,
    pub final_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodexEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub item: Option<CodexEventItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodexEventItem {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub item_type: String,
    pub server: Option<String>,
    pub tool: Option<String>,
    pub arguments: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<CodexEventError>,
    pub status: Option<String>,
    pub text: Option<String>,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodexEventError {
    pub message: String,
}

fn default_auth_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let path = PathBuf::from(home).join(".codex/auth.json");
    path.exists().then_some(path)
}

fn agents_md() -> &'static str {
    "This is a throwaway integration-test workspace.\nYou are explicitly authorized to modify files here through the draxl_* MCP tools.\nDo not edit files directly.\nPrefer the higher-level Draxl edit tools over raw patch text.\n"
}

fn target_temp_root() -> crate::Result<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("target/draxl-itest");
    fs::create_dir_all(&path)
        .map_err(|err| ToolError::new(format!("failed to create {}: {err}", path.display())))?;
    canonicalize_path(path, "failed to canonicalize draxl-itest target directory")
}

fn canonicalize_path(path: impl AsRef<Path>, context: &'static str) -> crate::Result<PathBuf> {
    path.as_ref()
        .canonicalize()
        .map_err(|err| ToolError::new(format!("{context}: {err}")))
}

fn write_workspace_file(root: &Path, relative_path: &str, source: &str) -> crate::Result<()> {
    let path = Path::new(relative_path);
    if relative_path.is_empty() || path.is_absolute() {
        return Err(ToolError::new(format!(
            "workspace file path must be a non-empty relative path, got `{relative_path}`"
        )));
    }

    let absolute_path = root.join(path);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)
            .map_err(io_error("failed to create Codex workspace directories"))?;
    }
    fs::write(&absolute_path, source).map_err(io_error("failed to write fixture source"))
}

fn io_error(context: &'static str) -> impl FnOnce(std::io::Error) -> ToolError {
    move |err| ToolError::new(format!("{context}: {err}"))
}
