#![forbid(unsafe_code)]
//! Public facade for Draxl, an agent-native source language with
//! explicit syntax identity.
//!
//! Draxl makes syntax identity explicit with stable node ids, ranks, anchors,
//! and semantic patch operators so tools can edit the program tree directly
//! instead of patching text spans.
//!
//! This crate is the intended Rust integration surface for the workspace. It
//! keeps the common workflows small and explicit while still re-exporting the
//! lower-level crates for callers that need finer control.
//!
//! ```rust
//! let source = "@m1 mod demo { @f1[a] fn run() { @s1[a] @e1 work(); } }";
//! let file = draxl::parse_and_validate(source)?;
//! let formatted = draxl::format_source(source)?;
//! let lowered = draxl::lower_rust_source(source)?;
//! assert_eq!(file.items.len(), 1);
//! assert!(formatted.contains("@f1[a] fn run()"));
//! assert!(lowered.contains("fn run()"));
//! # Ok::<(), draxl::Error>(())
//! ```

use std::error::Error as StdError;
use std::fmt;

pub use draxl_ast as ast;
pub use draxl_ast::LowerLanguage;
pub use draxl_merge as merge;
pub use draxl_parser as parser;
pub use draxl_patch as patch;
pub use draxl_printer as printer;
pub use draxl_rust as rust;
pub use draxl_rust as lower_rust;
pub use draxl_validate as validate;

/// Convenience result type for `draxl` workflows.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level error for public Draxl workflows.
#[derive(Debug)]
pub enum Error {
    /// The source could not be parsed into the Draxl AST.
    Parse(parser::ParseError),
    /// The source parsed, but failed structural validation.
    Validation(Vec<validate::ValidationError>),
    /// Ordinary Rust source could not be imported into Draxl.
    RustImport(rust::ImportError),
}

impl Error {
    /// Returns validation errors when the failure happened after parsing.
    pub fn validation_errors(&self) -> Option<&[validate::ValidationError]> {
        match self {
            Self::Validation(errors) => Some(errors),
            Self::Parse(_) | Self::RustImport(_) => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => error.fmt(f),
            Self::Validation(errors) => {
                f.write_str("validation failed:")?;
                for error in errors {
                    f.write_str("\n- ")?;
                    f.write_str(&error.message)?;
                }
                Ok(())
            }
            Self::RustImport(error) => error.fmt(f),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::RustImport(error) => Some(error),
            Self::Validation(_) => None,
        }
    }
}

impl From<parser::ParseError> for Error {
    fn from(error: parser::ParseError) -> Self {
        Self::Parse(error)
    }
}

impl From<rust::ImportError> for Error {
    fn from(error: rust::ImportError) -> Self {
        Self::RustImport(error)
    }
}

/// Parses a Draxl source string into the typed AST.
pub fn parse_file(source: &str) -> std::result::Result<ast::File, parser::ParseError> {
    parser::parse_file(source)
}

/// Parses a Draxl source string into the typed AST using the selected lower language.
pub fn parse_file_for_language(
    language: LowerLanguage,
    source: &str,
) -> std::result::Result<ast::File, parser::ParseError> {
    parser::parse_file_for_language(language, source)
}

/// Validates a parsed Draxl file.
pub fn validate_file(file: &ast::File) -> std::result::Result<(), Vec<validate::ValidationError>> {
    validate::validate_file(file)
}

/// Parses and validates a Draxl source string in one step.
pub fn parse_and_validate(source: &str) -> Result<ast::File> {
    let file = parse_file(source)?;
    validate_file(&file).map_err(Error::Validation)?;
    Ok(file)
}

/// Parses and validates a Draxl source string using the selected lower language.
pub fn parse_and_validate_for_language(language: LowerLanguage, source: &str) -> Result<ast::File> {
    let file = parse_file_for_language(language, source)?;
    validate_file(&file).map_err(Error::Validation)?;
    Ok(file)
}

/// Canonically formats a parsed Draxl file.
pub fn format_file(file: &ast::File) -> String {
    printer::print_file(file)
}

/// Canonically formats a parsed Draxl file using the selected lower language.
pub fn format_file_for_language(language: LowerLanguage, file: &ast::File) -> String {
    printer::print_file_for_language(language, file)
}

/// Parses, validates, and canonically formats a Draxl source string.
pub fn format_source(source: &str) -> Result<String> {
    let file = parse_and_validate(source)?;
    Ok(format_file(&file))
}

/// Parses, validates, and canonically formats a Draxl source string using the selected lower language.
pub fn format_source_for_language(language: LowerLanguage, source: &str) -> Result<String> {
    let file = parse_and_validate_for_language(language, source)?;
    Ok(format_file_for_language(language, &file))
}

/// Emits deterministic JSON for a parsed Draxl file.
pub fn dump_json_file(file: &ast::File) -> String {
    printer::canonicalize_file(file).to_json_pretty()
}

/// Parses, validates, and emits deterministic JSON for a Draxl source string.
pub fn dump_json_source(source: &str) -> Result<String> {
    let file = parse_and_validate(source)?;
    Ok(dump_json_file(&file))
}

/// Lowers a validated Draxl file to ordinary Rust source.
pub fn lower_rust_file(file: &ast::File) -> String {
    lower_rust::lower_file(file)
}

/// Parses, validates, and lowers Draxl source to ordinary Rust.
pub fn lower_rust_source(source: &str) -> Result<String> {
    let file = parse_and_validate(source)?;
    Ok(lower_rust_file(&file))
}

/// Imports ordinary Rust source into canonical Draxl Rust-profile source.
pub fn import_rust_source(source: &str) -> Result<String> {
    let imported = rust::import_source(source)?;
    let printed = format_file_for_language(LowerLanguage::Rust, &imported);
    let file = parse_and_validate_for_language(LowerLanguage::Rust, &printed)?;
    Ok(format_file_for_language(LowerLanguage::Rust, &file))
}

/// Applies a single structured patch operation.
pub fn apply_patch(
    file: &mut ast::File,
    op: patch::PatchOp,
) -> std::result::Result<(), patch::PatchError> {
    patch::apply_op(file, op)
}

/// Applies multiple structured patch operations in order.
pub fn apply_patches(
    file: &mut ast::File,
    ops: impl IntoIterator<Item = patch::PatchOp>,
) -> std::result::Result<(), patch::PatchError> {
    patch::apply_ops(file, ops)
}

/// Parses canonical textual patch ops into unresolved surface ops.
pub fn parse_patch_ops(
    source: &str,
) -> std::result::Result<Vec<patch::SurfacePatchOp>, patch::PatchTextError> {
    patch::parse_patch_ops(source)
}

/// Resolves textual patch ops against the current file.
pub fn resolve_patch_ops(
    file: &ast::File,
    source: &str,
) -> std::result::Result<Vec<patch::PatchOp>, patch::PatchTextError> {
    patch::resolve_patch_ops(file, source)
}

/// Parses, resolves, and applies textual patch ops in order.
pub fn apply_patch_text(
    file: &mut ast::File,
    source: &str,
) -> std::result::Result<(), patch::PatchTextError> {
    patch::apply_patch_text(file, source)
}

/// Checks whether two patch streams have hard conflicts against the same base.
pub fn check_hard_conflicts(
    base: &ast::File,
    left: &[patch::PatchOp],
    right: &[patch::PatchOp],
) -> merge::HardConflictReport {
    merge::check_hard_conflicts(base, left, right)
}

/// Checks whether two patch streams have hard conflicts and emits JSON.
pub fn check_hard_conflicts_json(
    base: &ast::File,
    left: &[patch::PatchOp],
    right: &[patch::PatchOp],
) -> String {
    check_hard_conflicts(base, left, right).to_json_pretty()
}

/// Checks both hard and semantic conflicts against the same base.
pub fn check_conflicts(
    base: &ast::File,
    left: &[patch::PatchOp],
    right: &[patch::PatchOp],
) -> merge::ConflictReport {
    merge::check_conflicts(base, left, right)
}

/// Checks both hard and semantic conflicts and emits JSON.
pub fn check_conflicts_json(
    base: &ast::File,
    left: &[patch::PatchOp],
    right: &[patch::PatchOp],
) -> String {
    check_conflicts(base, left, right).to_json_pretty()
}
