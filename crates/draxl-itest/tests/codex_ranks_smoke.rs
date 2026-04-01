use draxl_itest::codex::CodexHarness;
use draxl_itest::fixtures::ranks_smoke::SOURCE;

const PATH: &str = "fixtures/04_ranks.rs.dx";
const MAX_ATTEMPTS: usize = 2;

const EXEC_PROMPT: &str = "\
This is a throwaway integration-test workspace and you are explicitly authorized to modify it.
Use Draxl tools only.
Start with draxl_inspect_file on fixtures/04_ranks.rs.dx to get the current fingerprint and ids.
Then in fixtures/04_ranks.rs.dx, insert trace(); after fetch(); and change log(); to audit();.
After both edits are applied, briefly confirm completion.";

const EXPECTED_AFTER: &str = "\
@m1 mod pipeline {
  @f1[a] fn fetch() {
  }

  @f2[b] fn log() {
  }

  @f3[c] fn validate() {
  }

  @f4[d] fn run() {
    @s1[a] @e1 fetch();
    @s4[al] @e4 trace();
    @s2[am] @e2 audit();
    @s3[b] @e3 validate();
  }
}

";

#[test]
#[ignore = "requires Codex auth/network for a real Codex+MCP end-to-end run"]
fn codex_04_ranks_uses_draxl_tools_end_to_end() {
    let draxl_bin =
        std::env::var("CARGO_BIN_EXE_draxl").expect("cargo should expose the draxl binary");
    let harness = CodexHarness::new(draxl_bin).expect("Codex harness should initialize");

    let mut failures = Vec::new();

    for attempt in 1..=MAX_ATTEMPTS {
        let exec_workspace = harness
            .create_workspace("codex-exec-ranks", PATH, SOURCE)
            .expect("exec workspace should initialize");
        let run = harness
            .run_exec_json(&exec_workspace, EXEC_PROMPT)
            .expect("codex exec should succeed");

        let final_source = exec_workspace
            .read_source()
            .expect("Codex workspace source should be readable");
        let mut completed_tools = run
            .events
            .iter()
            .filter_map(|event| event.item.as_ref())
            .filter(|item| {
                item.item_type == "mcp_tool_call" && item.status.as_deref() == Some("completed")
            })
            .filter_map(|item| item.tool.clone())
            .collect::<Vec<_>>();
        completed_tools.sort();

        let inspect = exec_workspace
            .inspect()
            .expect("final workspace should still be inspectable");
        let final_message = run.final_message.as_deref().unwrap_or_default().to_owned();
        let success = final_source == EXPECTED_AFTER
            && completed_tools.contains(&"draxl_inspect_file".to_owned())
            && completed_tools.contains(&"draxl_insert_after_stmt".to_owned())
            && completed_tools.contains(&"draxl_replace_node".to_owned())
            && !completed_tools.contains(&"draxl_apply_patch_text".to_owned())
            && inspect.path == PATH;

        if success {
            return;
        }

        failures.push(format!(
            "attempt {attempt} failed\nfinal_source:\n{final_source}\ncompleted_tools: {completed_tools:?}\nfinal_message:\n{final_message}\nstdout:\n{}",
            run.stdout
        ));
    }

    panic!(
        "Codex did not complete the Draxl-only edit flow after {MAX_ATTEMPTS} attempts\n\n{}",
        failures.join("\n\n---\n\n")
    );
}
