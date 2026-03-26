#![forbid(unsafe_code)]
//! Language-dispatch facade for Draxl surface parsing.
//!
//! Today the crate exposes only the Rust backend, but the public parsing
//! surface is language-aware so additional backends can be added behind the
//! same facade over time.

mod error;
mod lexer;
mod parser;
mod syntax;

use draxl_ast::{
    CommentNode, DocNode, Expr, Field, File, Item, LowerLanguage, MatchArm, Param, Pattern, Stmt,
    Type, Variant,
};

pub use error::ParseError;

mod rust_backend {
    use super::{lexer, parser, ParseError};
    use draxl_ast::{
        CommentNode, DocNode, Expr, Field, File, Item, MatchArm, Param, Pattern, Stmt, Type,
        Variant,
    };

    pub(super) fn parse_file(source: &str) -> Result<File, ParseError> {
        let tokens = lexer::lex(source)?;
        parser::Parser::new(source, tokens).parse_file()
    }

    pub(super) fn parse_item_fragment(source: &str) -> Result<Item, ParseError> {
        let tokens = lexer::lex(source)?;
        parser::Parser::new(source, tokens).parse_item_fragment()
    }

    pub(super) fn parse_field_fragment(source: &str) -> Result<Field, ParseError> {
        let tokens = lexer::lex(source)?;
        parser::Parser::new(source, tokens).parse_field_fragment()
    }

    pub(super) fn parse_variant_fragment(source: &str) -> Result<Variant, ParseError> {
        let tokens = lexer::lex(source)?;
        parser::Parser::new(source, tokens).parse_variant_fragment()
    }

    pub(super) fn parse_param_fragment(source: &str) -> Result<Param, ParseError> {
        let tokens = lexer::lex(source)?;
        parser::Parser::new(source, tokens).parse_param_fragment()
    }

    pub(super) fn parse_stmt_fragment(source: &str) -> Result<Stmt, ParseError> {
        let tokens = lexer::lex(source)?;
        parser::Parser::new(source, tokens).parse_stmt_fragment()
    }

    pub(super) fn parse_match_arm_fragment(source: &str) -> Result<MatchArm, ParseError> {
        let tokens = lexer::lex(source)?;
        parser::Parser::new(source, tokens).parse_match_arm_fragment()
    }

    pub(super) fn parse_expr_fragment(source: &str) -> Result<Expr, ParseError> {
        let tokens = lexer::lex(source)?;
        parser::Parser::new(source, tokens).parse_expr_fragment()
    }

    pub(super) fn parse_type_fragment(source: &str) -> Result<Type, ParseError> {
        let tokens = lexer::lex(source)?;
        parser::Parser::new(source, tokens).parse_type_fragment()
    }

    pub(super) fn parse_pattern_fragment(source: &str) -> Result<Pattern, ParseError> {
        let tokens = lexer::lex(source)?;
        parser::Parser::new(source, tokens).parse_pattern_fragment()
    }

    pub(super) fn parse_doc_fragment(source: &str) -> Result<DocNode, ParseError> {
        let tokens = lexer::lex(source)?;
        parser::Parser::new(source, tokens).parse_doc_fragment()
    }

    pub(super) fn parse_comment_fragment(source: &str) -> Result<CommentNode, ParseError> {
        let tokens = lexer::lex(source)?;
        parser::Parser::new(source, tokens).parse_comment_fragment()
    }
}

/// Parses Draxl source into the bootstrap AST using the selected language backend.
pub fn parse_file_for_language(language: LowerLanguage, source: &str) -> Result<File, ParseError> {
    match language {
        LowerLanguage::Rust => rust_backend::parse_file(source),
    }
}

/// Parses Draxl source into the bootstrap AST.
pub fn parse_file(source: &str) -> Result<File, ParseError> {
    parse_file_for_language(LowerLanguage::Rust, source)
}

/// Parses a single item fragment for patch resolution using the selected language backend.
pub fn parse_item_fragment_for_language(
    language: LowerLanguage,
    source: &str,
) -> Result<Item, ParseError> {
    match language {
        LowerLanguage::Rust => rust_backend::parse_item_fragment(source),
    }
}

/// Parses a single item fragment for patch resolution.
pub fn parse_item_fragment(source: &str) -> Result<Item, ParseError> {
    parse_item_fragment_for_language(LowerLanguage::Rust, source)
}

/// Parses a single struct field fragment for patch resolution using the selected language backend.
pub fn parse_field_fragment_for_language(
    language: LowerLanguage,
    source: &str,
) -> Result<Field, ParseError> {
    match language {
        LowerLanguage::Rust => rust_backend::parse_field_fragment(source),
    }
}

/// Parses a single struct field fragment for patch resolution.
pub fn parse_field_fragment(source: &str) -> Result<Field, ParseError> {
    parse_field_fragment_for_language(LowerLanguage::Rust, source)
}

/// Parses a single enum variant fragment for patch resolution using the selected language backend.
pub fn parse_variant_fragment_for_language(
    language: LowerLanguage,
    source: &str,
) -> Result<Variant, ParseError> {
    match language {
        LowerLanguage::Rust => rust_backend::parse_variant_fragment(source),
    }
}

/// Parses a single enum variant fragment for patch resolution.
pub fn parse_variant_fragment(source: &str) -> Result<Variant, ParseError> {
    parse_variant_fragment_for_language(LowerLanguage::Rust, source)
}

/// Parses a single function parameter fragment for patch resolution using the selected language backend.
pub fn parse_param_fragment_for_language(
    language: LowerLanguage,
    source: &str,
) -> Result<Param, ParseError> {
    match language {
        LowerLanguage::Rust => rust_backend::parse_param_fragment(source),
    }
}

/// Parses a single function parameter fragment for patch resolution.
pub fn parse_param_fragment(source: &str) -> Result<Param, ParseError> {
    parse_param_fragment_for_language(LowerLanguage::Rust, source)
}

/// Parses a single statement fragment for patch resolution using the selected language backend.
pub fn parse_stmt_fragment_for_language(
    language: LowerLanguage,
    source: &str,
) -> Result<Stmt, ParseError> {
    match language {
        LowerLanguage::Rust => rust_backend::parse_stmt_fragment(source),
    }
}

/// Parses a single statement fragment for patch resolution.
pub fn parse_stmt_fragment(source: &str) -> Result<Stmt, ParseError> {
    parse_stmt_fragment_for_language(LowerLanguage::Rust, source)
}

/// Parses a single match arm fragment for patch resolution using the selected language backend.
pub fn parse_match_arm_fragment_for_language(
    language: LowerLanguage,
    source: &str,
) -> Result<MatchArm, ParseError> {
    match language {
        LowerLanguage::Rust => rust_backend::parse_match_arm_fragment(source),
    }
}

/// Parses a single match arm fragment for patch resolution.
pub fn parse_match_arm_fragment(source: &str) -> Result<MatchArm, ParseError> {
    parse_match_arm_fragment_for_language(LowerLanguage::Rust, source)
}

/// Parses a single expression fragment for patch resolution using the selected language backend.
pub fn parse_expr_fragment_for_language(
    language: LowerLanguage,
    source: &str,
) -> Result<Expr, ParseError> {
    match language {
        LowerLanguage::Rust => rust_backend::parse_expr_fragment(source),
    }
}

/// Parses a single expression fragment for patch resolution.
pub fn parse_expr_fragment(source: &str) -> Result<Expr, ParseError> {
    parse_expr_fragment_for_language(LowerLanguage::Rust, source)
}

/// Parses a single type fragment for patch resolution using the selected language backend.
pub fn parse_type_fragment_for_language(
    language: LowerLanguage,
    source: &str,
) -> Result<Type, ParseError> {
    match language {
        LowerLanguage::Rust => rust_backend::parse_type_fragment(source),
    }
}

/// Parses a single type fragment for patch resolution.
pub fn parse_type_fragment(source: &str) -> Result<Type, ParseError> {
    parse_type_fragment_for_language(LowerLanguage::Rust, source)
}

/// Parses a single pattern fragment for patch resolution using the selected language backend.
pub fn parse_pattern_fragment_for_language(
    language: LowerLanguage,
    source: &str,
) -> Result<Pattern, ParseError> {
    match language {
        LowerLanguage::Rust => rust_backend::parse_pattern_fragment(source),
    }
}

/// Parses a single pattern fragment for patch resolution.
pub fn parse_pattern_fragment(source: &str) -> Result<Pattern, ParseError> {
    parse_pattern_fragment_for_language(LowerLanguage::Rust, source)
}

/// Parses a single doc comment fragment for patch resolution using the selected language backend.
pub fn parse_doc_fragment_for_language(
    language: LowerLanguage,
    source: &str,
) -> Result<DocNode, ParseError> {
    match language {
        LowerLanguage::Rust => rust_backend::parse_doc_fragment(source),
    }
}

/// Parses a single doc comment fragment for patch resolution.
pub fn parse_doc_fragment(source: &str) -> Result<DocNode, ParseError> {
    parse_doc_fragment_for_language(LowerLanguage::Rust, source)
}

/// Parses a single line comment fragment for patch resolution using the selected language backend.
pub fn parse_comment_fragment_for_language(
    language: LowerLanguage,
    source: &str,
) -> Result<CommentNode, ParseError> {
    match language {
        LowerLanguage::Rust => rust_backend::parse_comment_fragment(source),
    }
}

/// Parses a single line comment fragment for patch resolution.
pub fn parse_comment_fragment(source: &str) -> Result<CommentNode, ParseError> {
    parse_comment_fragment_for_language(LowerLanguage::Rust, source)
}
