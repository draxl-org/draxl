#![forbid(unsafe_code)]
//! Language-dispatch facade for canonical Draxl rendering.
//!
//! This crate renders the AST back into compact Draxl surface syntax and
//! re-exports the shared canonicalization helper from `draxl-ast`.
//!
//! Today the crate exposes only the Rust backend, but the public rendering
//! surface is language-aware so additional backends can be added behind the
//! same facade over time.

mod render;

use draxl_ast::{File, LowerLanguage};

pub use draxl_ast::canonicalize_file;

mod rust_backend {
    use super::render;
    use draxl_ast::File;

    pub(super) fn print_file(file: &File) -> String {
        render::print_file(file)
    }
}

/// Prints a file using the selected language backend.
pub fn print_file_for_language(language: LowerLanguage, file: &File) -> String {
    match language {
        LowerLanguage::Rust => rust_backend::print_file(file),
    }
}

/// Prints a file using the default Rust backend.
pub fn print_file(file: &File) -> String {
    print_file_for_language(LowerLanguage::Rust, file)
}
