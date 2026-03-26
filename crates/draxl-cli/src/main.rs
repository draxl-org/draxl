#![forbid(unsafe_code)]
//! Command-line entry point for the Draxl bootstrap prototype.
//!
//! The CLI intentionally uses the public `draxl` facade rather than re-wiring
//! parser, validator, printer, and lowering behavior itself.

use draxl::{
    apply_patch_text_for_language, check_conflicts_json, dump_json_file, format_file_for_language,
    format_source_for_language, lower_rust_source, parse_and_validate_for_language,
    parse_file_for_language, resolve_patch_ops_for_language, validate_file, LowerLanguage,
};
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
            print!("{}", check_conflicts_json(&file, &left_ops, &right_ops));
            Ok(())
        }
        _ => Err(usage()),
    }
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
  draxl lower-rust <file>
  draxl patch [--in-place] <file> <patch-file>
  draxl conflicts <file> <left-patch-file> <right-patch-file>"
        .to_owned()
}
