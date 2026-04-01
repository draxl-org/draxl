#![forbid(unsafe_code)]

use draxl_itest::{scenarios, ToolWorkspace};
use std::env;
use std::process::ExitCode;
use tempfile::tempdir;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return Err(usage());
    };

    match command.as_str() {
        "list" => {
            for name in scenarios::names() {
                println!("{name}");
            }
            Ok(())
        }
        "run" => {
            let Some(case_name) = args.next() else {
                return Err(format!("missing case name for `run`\n\n{}", usage()));
            };
            let temp_dir = tempdir().map_err(|err| {
                format!("failed to create temp workspace for `{case_name}`: {err}")
            })?;
            let workspace = ToolWorkspace::new(temp_dir.path()).map_err(|err| {
                format!("failed to initialize integration workspace for `{case_name}`: {err}")
            })?;
            let run = scenarios::run_named(&case_name, &workspace)
                .map_err(|err| format!("integration case `{case_name}` failed: {err}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&run)
                    .map_err(|err| format!("failed to render case output: {err}"))?
            );
            Ok(())
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage:
  draxl-itest list
  draxl-itest run <case>"
        .to_owned()
}
