#![forbid(unsafe_code)]
//! Structural validation for Draxl Source v0.
//!
//! Validation runs after parsing and checks the stronger invariants that make
//! canonical printing, lowering, and patch application predictable:
//!
//! - ids must be unique
//! - ranked slots must provide ranks
//! - anchors must refer to valid targets
//! - detached comments and docs must resolve deterministically

mod error;
mod validator;

use draxl_ast::File;

pub use error::ValidationError;

/// Runs structural validation for a parsed Draxl file.
pub fn validate_file(file: &File) -> Result<(), Vec<ValidationError>> {
    let mut validator = validator::Validator::default();
    validator.collect_file_ids(file);
    validator.validate_file(file);
    if validator.errors.is_empty() {
        Ok(())
    } else {
        Err(validator.errors)
    }
}
