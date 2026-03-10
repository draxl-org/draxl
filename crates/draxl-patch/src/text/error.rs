use draxl_ast::Span;
use draxl_parser::ParseError;
use std::fmt;

/// Error produced while parsing, resolving, or applying textual patch ops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchTextError {
    /// Human-readable description of the failure.
    pub message: String,
    /// Source span that triggered the failure.
    pub span: Span,
    /// One-based line number for the span start.
    pub line: usize,
    /// One-based column number for the span start.
    pub column: usize,
}

impl fmt::Display for PatchTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {}, column {}",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for PatchTextError {}

pub(crate) fn patch_text_error(source: &str, span: Span, message: &str) -> PatchTextError {
    let (line, column) = line_col(source, span.start);
    PatchTextError {
        message: message.to_owned(),
        span,
        line,
        column,
    }
}

pub(crate) fn map_fragment_parse_error(
    source: &str,
    fragment_start: usize,
    error: ParseError,
) -> PatchTextError {
    let span = Span {
        start: fragment_start + error.span.start,
        end: fragment_start + error.span.end,
    };
    patch_text_error(source, span, &error.message)
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    for (index, ch) in source.char_indices() {
        if index >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}
