use crate::error::Result;
use crate::fixtures::ranks_smoke::SOURCE;
use crate::scenarios::{ScenarioRun, ScenarioStepRun};
use crate::types::{InsertAfterStmtRequest, ReplaceNodeRequest};
use crate::ToolWorkspace;
use std::fs;

pub const NAME: &str = "04_ranks_agent_tool_smoke";

const PATH: &str = "fixtures/04_ranks.rs.dx";

pub fn run(workspace: &ToolWorkspace) -> Result<ScenarioRun> {
    materialize_fixture(workspace)?;
    let inspection = workspace.inspect_file(PATH)?;

    let insert_result = workspace.insert_after_stmt(InsertAfterStmtRequest {
        path: PATH,
        anchor_id: "s1",
        stmt_source: "trace();",
        expected_fingerprint: Some(&inspection.fingerprint),
        apply: false,
    })?;

    let replace_result = workspace.replace_node(ReplaceNodeRequest {
        path: PATH,
        target_id: "e2",
        fragment_source: "audit()",
        expected_fingerprint: Some(&inspection.fingerprint),
        apply: false,
    })?;

    Ok(ScenarioRun {
        case_name: NAME.to_owned(),
        inspection,
        step_runs: vec![
            ScenarioStepRun::InsertAfterStmt {
                path: PATH.to_owned(),
                anchor_id: "s1".to_owned(),
                result: insert_result,
            },
            ScenarioStepRun::ReplaceNode {
                path: PATH.to_owned(),
                target_id: "e2".to_owned(),
                result: replace_result,
            },
        ],
    })
}

fn materialize_fixture(workspace: &ToolWorkspace) -> Result<()> {
    let absolute_path = workspace.root().join(PATH);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            crate::ToolError::new(format!(
                "failed to create fixture directory {}: {err}",
                parent.display()
            ))
        })?;
    }
    fs::write(&absolute_path, SOURCE).map_err(|err| {
        crate::ToolError::new(format!(
            "failed to write fixture {}: {err}",
            absolute_path.display()
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{run, PATH};
    use crate::fixtures::ranks_smoke::SOURCE;
    use crate::scenarios::ScenarioStepRun;
    use crate::ToolWorkspace;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn ranks_smoke_case_shows_the_full_flow_in_one_place() {
        let temp_dir = tempdir().expect("temp workspace should initialize");
        let workspace = ToolWorkspace::new(temp_dir.path()).expect("workspace should initialize");

        let before_source = SOURCE;

        let expected_insert_patch = "insert @f4.body[al]: @s4 @e4 trace();\n";
        let expected_insert_preview = r#"@m1 mod pipeline {
  @f1[a] fn fetch() {
  }

  @f2[b] fn log() {
  }

  @f3[c] fn validate() {
  }

  @f4[d] fn run() {
    @s1[a] @e1 fetch();
    @s4[al] @e4 trace();
    @s2[am] @e2 log();
    @s3[b] @e3 validate();
  }
}

"#;
        let expected_replace_patch = "replace @e2: @e2 audit()\n";
        let expected_replace_preview = r#"@m1 mod pipeline {
  @f1[a] fn fetch() {
  }

  @f2[b] fn log() {
  }

  @f3[c] fn validate() {
  }

  @f4[d] fn run() {
    @s1[a] @e1 fetch();
    @s2[am] @e2 audit();
    @s3[b] @e3 validate();
  }
}

"#;

        let run = run(&workspace).expect("scenario should succeed");

        assert_eq!(run.case_name, "04_ranks_agent_tool_smoke");
        assert_eq!(run.inspection.path, PATH);
        assert_eq!(run.inspection.source, before_source);
        assert_eq!(run.inspection.node_count, 11);

        match &run.step_runs[0] {
            ScenarioStepRun::InsertAfterStmt {
                path,
                anchor_id,
                result,
            } => {
                assert_eq!(path, PATH);
                assert_eq!(anchor_id, "s1");
                assert_eq!(result.patch_text, expected_insert_patch);
                assert_eq!(result.preview_dx, expected_insert_preview);
                assert!(!result.applied);
            }
            other => panic!("expected insert step, found {other:?}"),
        }

        match &run.step_runs[1] {
            ScenarioStepRun::ReplaceNode {
                path,
                target_id,
                result,
            } => {
                assert_eq!(path, PATH);
                assert_eq!(target_id, "e2");
                assert_eq!(result.patch_text, expected_replace_patch);
                assert_eq!(result.preview_dx, expected_replace_preview);
                assert!(!result.applied);
            }
            other => panic!("expected replace step, found {other:?}"),
        }

        let after_source =
            fs::read_to_string(workspace.root().join(PATH)).expect("example should read");
        assert_eq!(
            before_source, after_source,
            "preview-only scenarios must not rewrite the source file"
        );
    }
}
