#![forbid(unsafe_code)]
//! Rust-profile support for Draxl.
//!
//! The crate owns Rust-profile parsing, rendering, lowering, and import
//! helpers so the generic surface crates can dispatch by lower language.

pub mod import;
pub mod lower;
pub mod parse;
pub mod render;

pub use import::{import_source, ImportError};
pub use lower::lower_file;
pub use parse::{
    parse_comment_fragment, parse_doc_fragment, parse_expr_fragment, parse_field_fragment,
    parse_file, parse_item_fragment, parse_match_arm_fragment, parse_param_fragment,
    parse_pattern_fragment, parse_stmt_fragment, parse_type_fragment, parse_variant_fragment,
    ParseError,
};
pub use render::print_file;
