use std::error::Error as StdError;
use std::fmt;

pub type Result<T> = std::result::Result<T, ToolError>;

#[derive(Debug, Clone)]
pub struct ToolError {
    message: String,
}

impl ToolError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl StdError for ToolError {}
