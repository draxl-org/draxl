mod error;
mod lexer;
mod parser;
mod syntax;

use draxl_ast::{
    CommentNode, DocNode, Expr, Field, File, Item, MatchArm, Param, Pattern, Stmt, Type, Variant,
};

pub use error::ParseError;

/// Parses Draxl Rust-profile source into the bootstrap AST.
pub fn parse_file(source: &str) -> Result<File, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_file()
}

/// Parses a single item fragment for patch resolution.
pub fn parse_item_fragment(source: &str) -> Result<Item, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_item_fragment()
}

/// Parses a single struct field fragment for patch resolution.
pub fn parse_field_fragment(source: &str) -> Result<Field, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_field_fragment()
}

/// Parses a single enum variant fragment for patch resolution.
pub fn parse_variant_fragment(source: &str) -> Result<Variant, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_variant_fragment()
}

/// Parses a single function parameter fragment for patch resolution.
pub fn parse_param_fragment(source: &str) -> Result<Param, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_param_fragment()
}

/// Parses a single statement fragment for patch resolution.
pub fn parse_stmt_fragment(source: &str) -> Result<Stmt, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_stmt_fragment()
}

/// Parses a single match arm fragment for patch resolution.
pub fn parse_match_arm_fragment(source: &str) -> Result<MatchArm, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_match_arm_fragment()
}

/// Parses a single expression fragment for patch resolution.
pub fn parse_expr_fragment(source: &str) -> Result<Expr, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_expr_fragment()
}

/// Parses a single type fragment for patch resolution.
pub fn parse_type_fragment(source: &str) -> Result<Type, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_type_fragment()
}

/// Parses a single pattern fragment for patch resolution.
pub fn parse_pattern_fragment(source: &str) -> Result<Pattern, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_pattern_fragment()
}

/// Parses a single doc comment fragment for patch resolution.
pub fn parse_doc_fragment(source: &str) -> Result<DocNode, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_doc_fragment()
}

/// Parses a single line comment fragment for patch resolution.
pub fn parse_comment_fragment(source: &str) -> Result<CommentNode, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(source, tokens).parse_comment_fragment()
}
