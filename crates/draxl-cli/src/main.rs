#![forbid(unsafe_code)]
//! Command-line entry point for the Draxl bootstrap prototype.
//!
//! The CLI intentionally uses the public `draxl` facade rather than re-wiring
//! parser, validator, printer, and lowering behavior itself.

mod mcp_setup;

use draxl::{
    apply_patch_text_for_language, check_conflicts_json_for_language, dump_json_file,
    format_file_for_language, format_source_for_language, lower_rust_source,
    lower_source_for_language, parse_and_validate_for_language, parse_file_for_language,
    resolve_patch_ops_for_language, validate_file, LowerLanguage,
};
use draxl_agent::mcp;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

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
        "parse" => {
            let path = parse_path_arg(args.next(), "parse")?;
            let language = detect_lower_language(&path)?;
            let source = read_source(&path)?;
            let file = parse_file_for_language(language, &source).map_err(|err| err.to_string())?;
            println!(
                "parsed {}: {} top-level item(s)",
                path.display(),
                file.items.len()
            );
            Ok(())
        }
        "fmt" => {
            let first = args.next();
            let (in_place, path_arg) = match first.as_deref() {
                Some("--in-place") => (true, args.next()),
                _ => (false, first),
            };
            let path = parse_path_arg(path_arg, "fmt")?;
            let language = detect_lower_language(&path)?;
            let source = read_source(&path)?;
            let formatted =
                format_source_for_language(language, &source).map_err(|err| err.to_string())?;
            if in_place {
                fs::write(&path, formatted)
                    .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
            } else {
                print!("{formatted}");
            }
            Ok(())
        }
        "dump-json" => {
            let path = parse_path_arg(args.next(), "dump-json")?;
            let language = detect_lower_language(&path)?;
            let source = read_source(&path)?;
            let file = parse_and_validate_for_language(language, &source)
                .map_err(|err| err.to_string())?;
            print!("{}", dump_json_file(&file));
            Ok(())
        }
        "validate" => {
            let path = parse_path_arg(args.next(), "validate")?;
            let language = detect_lower_language(&path)?;
            let source = read_source(&path)?;
            let file = parse_file_for_language(language, &source).map_err(|err| err.to_string())?;
            validate_file(&file).map_err(format_validation_errors)?;
            println!("valid {}", path.display());
            Ok(())
        }
        "lower-rust" => {
            let path = parse_path_arg(args.next(), "lower-rust")?;
            let source = read_source(&path)?;
            print!(
                "{}",
                lower_rust_source(&source).map_err(|err| err.to_string())?
            );
            Ok(())
        }
        "lower" => {
            let path = parse_path_arg(args.next(), "lower")?;
            let language = detect_lower_language(&path)?;
            let source = read_source(&path)?;
            print!(
                "{}",
                lower_source_for_language(language, &source).map_err(|err| err.to_string())?
            );
            Ok(())
        }
        "patch" => {
            let first = args.next();
            let (in_place, file_arg, patch_arg) = match first.as_deref() {
                Some("--in-place") => (true, args.next(), args.next()),
                _ => (false, first, args.next()),
            };
            let path = parse_path_arg(file_arg, "patch")?;
            let patch_path = parse_path_arg(patch_arg, "patch")?;
            let language = detect_lower_language(&path)?;
            let source = read_source(&path)?;
            let mut file = parse_and_validate_for_language(language, &source)
                .map_err(|err| err.to_string())?;
            let patch_text = read_source(&patch_path)?;
            apply_patch_text_for_language(language, &mut file, &patch_text)
                .map_err(|err| err.to_string())?;
            validate_file(&file).map_err(format_validation_errors)?;
            let formatted = format_file_for_language(language, &file);
            if in_place {
                fs::write(&path, formatted)
                    .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
            } else {
                print!("{formatted}");
            }
            Ok(())
        }
        "conflicts" => {
            let path = parse_path_arg(args.next(), "conflicts")?;
            let left_patch_path = parse_path_arg(args.next(), "conflicts")?;
            let right_patch_path = parse_path_arg(args.next(), "conflicts")?;
            let language = detect_lower_language(&path)?;
            let source = read_source(&path)?;
            let file = parse_and_validate_for_language(language, &source)
                .map_err(|err| err.to_string())?;
            let left_patch = read_source(&left_patch_path)?;
            let right_patch = read_source(&right_patch_path)?;
            let left_ops = resolve_patch_ops_for_language(language, &file, &left_patch)
                .map_err(|err| err.to_string())?;
            let right_ops = resolve_patch_ops_for_language(language, &file, &right_patch)
                .map_err(|err| err.to_string())?;
            print!(
                "{}",
                check_conflicts_json_for_language(language, &file, &left_ops, &right_ops)
            );
            Ok(())
        }
        "mcp" => run_mcp(args),
        _ => Err(usage()),
    }
}

fn run_mcp<I>(args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let mut args = args;
    let Some(subcommand) = args.next() else {
        return Err(mcp_usage());
    };

    match subcommand.as_str() {
        "serve" => {
            let usage = mcp_usage();
            let root = parse_optional_root_arg(args, "serve", &usage)?;
            run_mcp_serve(root)
        }
        "setup" => mcp_setup::run_setup(args),
        _ => Err(mcp_usage()),
    }
}

fn run_mcp_serve(root: Option<PathBuf>) -> Result<(), String> {
    let root = match root {
        Some(root) => root,
        None => env::current_dir()
            .map_err(|err| format!("failed to determine current directory: {err}"))?,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|err| format!("failed to initialize tokio runtime: {err}"))?;
    runtime
        .block_on(mcp::serve_stdio(root))
        .map_err(|err| err.to_string())
}

fn parse_optional_root_arg<I>(
    mut args: I,
    command: &str,
    usage: &str,
) -> Result<Option<PathBuf>, String>
where
    I: Iterator<Item = String>,
{
    let Some(first) = args.next() else {
        return Ok(None);
    };
    if first != "--root" {
        return Err(format!(
            "unknown argument `{first}` for `draxl mcp {command}`\n\n{usage}"
        ));
    }
    let Some(path) = args.next() else {
        return Err(format!(
            "missing value for `--root` in `draxl mcp {command}`\n\n{usage}"
        ));
    };
    if let Some(extra) = args.next() {
        return Err(format!(
            "unexpected extra argument `{extra}` for `draxl mcp {command}`\n\n{usage}"
        ));
    }
    Ok(Some(PathBuf::from(path)))
}

fn read_source(path: &PathBuf) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn parse_path_arg(arg: Option<String>, command: &str) -> Result<PathBuf, String> {
    arg.map(PathBuf::from)
        .ok_or_else(|| format!("missing file path for `{command}`\n\n{}", usage()))
}

fn detect_lower_language(path: &Path) -> Result<LowerLanguage, String> {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return Err(format!(
            "could not infer lower language from {}: path is not valid utf-8",
            path.display()
        ));
    };

    if name.ends_with(".rs.dx") {
        Ok(LowerLanguage::Rust)
    } else {
        Err(format!(
            "could not infer lower language from {}: expected a supported source extension like `.rs.dx`",
            path.display()
        ))
    }
}

fn format_validation_errors(errors: Vec<draxl::validate::ValidationError>) -> String {
    let mut out = String::from("validation failed:");
    for error in errors {
        out.push('\n');
        out.push_str("- ");
        out.push_str(&error.message);
    }
    out
}

fn usage() -> String {
    "usage:
  draxl parse <file>
  draxl fmt [--in-place] <file>
  draxl dump-json <file>
  draxl validate <file>
  draxl lower <file>
  draxl lower-rust <file>
  draxl patch [--in-place] <file> <patch-file>
  draxl conflicts <file> <left-patch-file> <right-patch-file>
  draxl mcp serve [--root <workspace>]
  draxl mcp setup --client codex [--root <workspace>] [--print] [--force]"
        .to_owned()
}

fn mcp_usage() -> String {
    format!(
        "usage:
  draxl mcp serve [--root <workspace>]
  {}",
        mcp_setup::setup_usage()
    )
}
