#![forbid(unsafe_code)]
//! Hand-written lexer and parser for the current Draxl Rust profile.
//!
//! The crate is intentionally small and explicit:
//!
//! - `syntax` defines the token model shared by the lexer and parser
//! - `lexer` turns source text into tokens
//! - `parser` builds a typed Draxl AST from those tokens
//! - `error` provides parse errors with stable span and line/column reporting

mod error;
mod lexer;
mod parser;
mod syntax;

use draxl_ast::File;

pub use error::ParseError;

/// Parses Draxl source into the bootstrap AST.
pub fn parse_file(source: &str) -> Result<File, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_file()
}
