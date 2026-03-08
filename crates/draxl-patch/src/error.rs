use std::fmt;

/// Error produced while applying a patch operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchError {
    /// Human-readable description of the patch failure.
    pub message: String,
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PatchError {}

pub(crate) fn patch_error(message: &str) -> PatchError {
    PatchError {
        message: message.to_owned(),
    }
}
