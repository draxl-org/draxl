#![forbid(unsafe_code)]
//! Command-line entry point for the Draxl bootstrap prototype.
//!
//! The CLI intentionally uses the public `draxl` facade rather than re-wiring
//! parser, validator, printer, and lowering behavior itself.

use draxl::{dump_json_source, format_source, lower_rust_source, parse_file, validate_file};
use std::env;
use std::fs;
use std::path::PathBuf;
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
            let source = read_source(&path)?;
            let file = parse_file(&source).map_err(|err| err.to_string())?;
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
            let source = read_source(&path)?;
            let formatted = format_source(&source).map_err(|err| err.to_string())?;
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
            let source = read_source(&path)?;
            print!(
                "{}",
                dump_json_source(&source).map_err(|err| err.to_string())?
            );
            Ok(())
        }
        "validate" => {
            let path = parse_path_arg(args.next(), "validate")?;
            let source = read_source(&path)?;
            let file = parse_file(&source).map_err(|err| err.to_string())?;
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
  draxl lower-rust <file>"
        .to_owned()
}
