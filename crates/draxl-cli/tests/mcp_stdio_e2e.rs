use rmcp::{
    model::{CallToolRequestParams, ClientInfo},
    transport::{child_process::ConfigureCommandExt, TokioChildProcess},
    ClientHandler, ServiceExt,
};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command;

#[derive(Debug, Clone, Default)]
struct DummyClientHandler;

impl ClientHandler for DummyClientHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

#[tokio::test(flavor = "current_thread")]
async fn mcp_serve_supports_stdio_tool_round_trip() {
    let root = write_temp_dir("mcp_stdio_round_trip");
    let relative_path = "sample.rs.dx";
    fs::write(root.join(relative_path), sample_source()).expect("sample file should write");

    let transport = TokioChildProcess::new(Command::new(binary_path()).configure(|cmd| {
        cmd.args(["mcp", "serve", "--root"]);
        cmd.arg(&root);
    }))
    .expect("mcp server process should start");

    let client = DummyClientHandler::default()
        .serve(transport)
        .await
        .expect("mcp client should initialize");

    let tools = client
        .peer()
        .list_all_tools()
        .await
        .expect("tool list should succeed");
    assert!(tools.iter().any(|tool| tool.name == "draxl_inspect_file"));
    assert!(tools
        .iter()
        .any(|tool| tool.name == "draxl_insert_after_stmt"));

    let inspect_result = client
        .call_tool(
            CallToolRequestParams::new("draxl_inspect_file").with_arguments(
                json!({
                    "path": relative_path,
                })
                .as_object()
                .expect("inspect args should be an object")
                .clone(),
            ),
        )
        .await
        .expect("inspect_file should succeed");
    let inspection = parse_tool_json(inspect_result);
    assert_eq!(inspection["ok"], Value::Bool(true));
    assert_eq!(inspection["path"].as_str(), Some(relative_path));
    assert!(
        inspection["node_count"]
            .as_u64()
            .expect("inspection should include node_count")
            >= 5
    );
    let fingerprint = inspection["fingerprint"]
        .as_str()
        .expect("inspection should include a fingerprint")
        .to_owned();

    let insert_result = client
        .call_tool(
            CallToolRequestParams::new("draxl_insert_after_stmt").with_arguments(
                json!({
                    "path": relative_path,
                    "anchor_id": "s1",
                    "stmt_source": "trace();",
                    "expected_fingerprint": fingerprint,
                    "apply": true,
                })
                .as_object()
                .expect("insert args should be an object")
                .clone(),
            ),
        )
        .await
        .expect("insert_after_stmt should succeed");
    let patch = parse_tool_json(insert_result);
    assert_eq!(patch["ok"], Value::Bool(true));
    assert_eq!(patch["applied"], Value::Bool(true));
    assert_eq!(
        patch["before_fingerprint"].as_str(),
        inspection["fingerprint"].as_str()
    );
    assert_ne!(
        patch["after_fingerprint"].as_str(),
        inspection["fingerprint"].as_str()
    );

    let patch_text = patch["patch_text"]
        .as_str()
        .expect("patch result should include patch text");
    assert!(patch_text.contains("insert @f1.body["));
    assert!(patch_text.contains("trace();"));

    let preview_dx = patch["preview_dx"]
        .as_str()
        .expect("patch result should include preview source");
    assert!(preview_dx.contains("trace();"));

    let rewritten =
        fs::read_to_string(root.join(relative_path)).expect("rewritten file should read");
    assert!(rewritten.contains("trace();"));

    client
        .cancel()
        .await
        .expect("mcp client should shut down cleanly");
}

fn parse_tool_json(result: rmcp::model::CallToolResult) -> Value {
    let text = result
        .content
        .first()
        .and_then(|content| content.raw.as_text())
        .map(|content| content.text.as_str())
        .expect("tool result should contain text content");
    serde_json::from_str(text).expect("tool result text should be valid json")
}

fn sample_source() -> &'static str {
    r#"@m1 mod demo {
  @f1[a] fn run() {
    @s1[a] @e1 fetch();
    @s2[b] @e2 log();
  }
}
"#
}

fn binary_path() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_draxl") {
        let path = PathBuf::from(path);
        if path.exists() {
            return path;
        }
    }

    repo_root().join("target/debug").join(binary_name())
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[cfg(windows)]
fn binary_name() -> &'static str {
    "draxl.exe"
}

#[cfg(not(windows))]
fn binary_name() -> &'static str {
    "draxl"
}

fn write_temp_dir(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(unique_temp_name(name));
    fs::create_dir_all(&path).expect("temporary directory should be writable");
    path
}

fn unique_temp_name(name: &str) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    format!("draxl_{stamp}_{name}")
}
