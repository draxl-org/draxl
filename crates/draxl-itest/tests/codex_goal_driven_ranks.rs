use draxl_itest::codex::CodexHarness;

const PATH: &str = "fixtures/04_ranks.rs.dx";
const TASK_PATH: &str = "TASK.md";
const MAX_ATTEMPTS: usize = 2;

const SOURCE: &str = "\
@m1 mod pipeline {
  @f1[a] fn fetch() {
  }

  @f2[b] fn audit() {
  }

  @f3[c] fn trace() {
  }

  @f4[d] fn validate() {
  }

  @f5[e] fn run() {
    @s1[a] @e1 fetch();
    @s2[am] @e2 log();
    @s3[b] @e3 validate();
  }
}
";

const TASK: &str = "\
# Requested Behavior Change

The pipeline runner is out of date.

Update the code so that:
- `run()` emits `trace();` immediately after `fetch();`
- `run()` uses the current `audit();` API instead of the legacy `log();`

The helper APIs already exist in the module, so only `run()` should change.
Make the smallest code change that satisfies this request.
";

const EXEC_PROMPT: &str = "\
This is a throwaway integration-test workspace and you are explicitly authorized to modify it.
Read TASK.md and inspect the repo.
Use Draxl tools only for any code changes.
Implement the requested behavior with the minimal code change.
After the code satisfies TASK.md, briefly confirm completion.";

const EXPECTED_AFTER: &str = "\
@m1 mod pipeline {
  @f1[a] fn fetch() {
  }

  @f2[b] fn audit() {
  }

  @f3[c] fn trace() {
  }

  @f4[d] fn validate() {
  }

  @f5[e] fn run() {
    @s1[a] @e1 fetch();
    @s4[al] @e4 trace();
    @s2[am] @e2 audit();
    @s3[b] @e3 validate();
  }
}

";

#[test]
#[ignore = "requires Codex auth/network for a real Codex+MCP end-to-end run"]
fn codex_goal_driven_ranks_uses_draxl_tools_end_to_end() {
    let mcp_server_bin = std::env::var("CARGO_BIN_EXE_draxl-itest-mcp")
        .expect("cargo should expose the draxl-itest-mcp binary to integration tests");
    let harness = CodexHarness::new(mcp_server_bin).expect("Codex harness should initialize");

    let mut failures = Vec::new();

    for attempt in 1..=MAX_ATTEMPTS {
        let exec_workspace = harness
            .create_workspace_with_files("codex-goal-ranks", PATH, SOURCE, &[(TASK_PATH, TASK)])
            .expect("goal-driven exec workspace should initialize");
        let run = harness
            .run_exec_json(&exec_workspace, EXEC_PROMPT)
            .expect("codex exec should succeed");

        let final_source = exec_workspace
            .read_source()
            .expect("Codex workspace source should be readable");
        let completed_tools_in_order = run
            .events
            .iter()
            .filter_map(|event| event.item.as_ref())
            .filter(|item| {
                item.item_type == "mcp_tool_call" && item.status.as_deref() == Some("completed")
            })
            .filter_map(|item| item.tool.clone())
            .collect::<Vec<_>>();
        let mut completed_tools_sorted = completed_tools_in_order.clone();
        completed_tools_sorted.sort();

        let inspect = exec_workspace
            .inspect()
            .expect("final workspace should still be inspectable");
        let first_inspect_index = completed_tools_in_order
            .iter()
            .position(|tool| tool == "draxl_inspect_file");
        let first_mutation_index = completed_tools_in_order.iter().position(|tool| {
            tool == "draxl_insert_after_stmt"
                || tool == "draxl_replace_node"
                || tool == "draxl_set_path"
        });
        let success = final_source == EXPECTED_AFTER
            && completed_tools_sorted.contains(&"draxl_inspect_file".to_owned())
            && completed_tools_sorted.contains(&"draxl_insert_after_stmt".to_owned())
            && completed_tools_sorted.contains(&"draxl_replace_node".to_owned())
            && !completed_tools_sorted.contains(&"draxl_apply_patch_text".to_owned())
            && inspect.path == PATH
            && matches!(
                (first_inspect_index, first_mutation_index),
                (Some(inspect_index), Some(mutation_index)) if inspect_index < mutation_index
            );

        if success {
            return;
        }

        let final_message = run.final_message.as_deref().unwrap_or_default().to_owned();
        failures.push(format!(
            "attempt {attempt} failed\nfinal_source:\n{final_source}\ncompleted_tools_in_order: {completed_tools_in_order:?}\nfinal_message:\n{final_message}\nstdout:\n{}",
            run.stdout
        ));
    }

    panic!(
        "Codex did not complete the goal-driven Draxl edit flow after {MAX_ATTEMPTS} attempts\n\n{}",
        failures.join("\n\n---\n\n")
    );
}
