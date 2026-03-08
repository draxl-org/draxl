use crate::error::{parse_error, ParseError};
use crate::syntax::{token, Token, TokenKind};
use draxl_ast::Span;

pub(crate) fn lex(source: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut tokens = Vec::new();

    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b' ' | b'\t' | b'\r' | b'\n' => {
                index += 1;
            }
            b'@' => {
                tokens.push(token(TokenKind::At, index, index + 1));
                index += 1;
            }
            b'[' => {
                tokens.push(token(TokenKind::LBracket, index, index + 1));
                index += 1;
            }
            b']' => {
                tokens.push(token(TokenKind::RBracket, index, index + 1));
                index += 1;
            }
            b'(' => {
                tokens.push(token(TokenKind::LParen, index, index + 1));
                index += 1;
            }
            b')' => {
                tokens.push(token(TokenKind::RParen, index, index + 1));
                index += 1;
            }
            b'{' => {
                tokens.push(token(TokenKind::LBrace, index, index + 1));
                index += 1;
            }
            b'}' => {
                tokens.push(token(TokenKind::RBrace, index, index + 1));
                index += 1;
            }
            b',' => {
                tokens.push(token(TokenKind::Comma, index, index + 1));
                index += 1;
            }
            b':' => {
                if index + 1 < bytes.len() && bytes[index + 1] == b':' {
                    tokens.push(token(TokenKind::DoubleColon, index, index + 2));
                    index += 2;
                } else {
                    tokens.push(token(TokenKind::Colon, index, index + 1));
                    index += 1;
                }
            }
            b';' => {
                tokens.push(token(TokenKind::Semi, index, index + 1));
                index += 1;
            }
            b'=' => {
                if index + 1 < bytes.len() && bytes[index + 1] == b'>' {
                    tokens.push(token(TokenKind::FatArrow, index, index + 2));
                    index += 2;
                } else {
                    tokens.push(token(TokenKind::Eq, index, index + 1));
                    index += 1;
                }
            }
            b'-' => {
                if index + 1 < bytes.len() && bytes[index + 1] == b'>' {
                    tokens.push(token(TokenKind::Arrow, index, index + 2));
                    index += 2;
                } else {
                    tokens.push(token(TokenKind::Minus, index, index + 1));
                    index += 1;
                }
            }
            b'+' => {
                tokens.push(token(TokenKind::Plus, index, index + 1));
                index += 1;
            }
            b'*' => {
                tokens.push(token(TokenKind::Star, index, index + 1));
                index += 1;
            }
            b'<' => {
                tokens.push(token(TokenKind::Lt, index, index + 1));
                index += 1;
            }
            b'/' => {
                if index + 2 < bytes.len() && bytes[index + 1] == b'/' && bytes[index + 2] == b'/' {
                    let start = index;
                    index += 3;
                    while index < bytes.len() && bytes[index] != b'\n' {
                        index += 1;
                    }
                    let mut text = source[start + 3..index].to_owned();
                    if let Some(stripped) = text.strip_prefix(' ') {
                        text = stripped.to_owned();
                    }
                    tokens.push(token(TokenKind::DocComment(text), start, index));
                } else if index + 1 < bytes.len() && bytes[index + 1] == b'/' {
                    let start = index;
                    index += 2;
                    while index < bytes.len() && bytes[index] != b'\n' {
                        index += 1;
                    }
                    let mut text = source[start + 2..index].to_owned();
                    if let Some(stripped) = text.strip_prefix(' ') {
                        text = stripped.to_owned();
                    }
                    tokens.push(token(TokenKind::LineComment(text), start, index));
                } else {
                    return Err(lex_error(
                        source,
                        Span {
                            start: index,
                            end: index + 1,
                        },
                        "only `//` and `///` comments are supported in the current Draxl Rust profile",
                    ));
                }
            }
            b'"' => {
                let start = index;
                index += 1;
                let mut value = String::new();
                while index < bytes.len() {
                    match bytes[index] {
                        b'"' => {
                            index += 1;
                            break;
                        }
                        b'\\' => {
                            index += 1;
                            if index >= bytes.len() {
                                return Err(lex_error(
                                    source,
                                    Span { start, end: index },
                                    "unterminated string literal",
                                ));
                            }
                            let escaped = match bytes[index] {
                                b'"' => '"',
                                b'\\' => '\\',
                                b'n' => '\n',
                                b'r' => '\r',
                                b't' => '\t',
                                other => {
                                    return Err(lex_error(
                                        source,
                                        Span {
                                            start: index,
                                            end: index + 1,
                                        },
                                        &format!(
                                            "unsupported string escape `\\{}` in the current Draxl Rust profile",
                                            other as char
                                        ),
                                    ))
                                }
                            };
                            value.push(escaped);
                            index += 1;
                        }
                        other => {
                            value.push(other as char);
                            index += 1;
                        }
                    }
                }
                if index > bytes.len() || !source[start..index].ends_with('"') {
                    return Err(lex_error(
                        source,
                        Span { start, end: index },
                        "unterminated string literal",
                    ));
                }
                tokens.push(token(TokenKind::Str(value), start, index));
            }
            b'0'..=b'9' => {
                let start = index;
                while index < bytes.len() && bytes[index].is_ascii_digit() {
                    index += 1;
                }
                let text = &source[start..index];
                let value = text.parse::<i64>().map_err(|_| {
                    lex_error(
                        source,
                        Span { start, end: index },
                        "integer literal is outside the supported i64 range",
                    )
                })?;
                tokens.push(token(TokenKind::Int(value), start, index));
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let start = index;
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                let text = &source[start..index];
                let kind = match text {
                    "mod" => TokenKind::Mod,
                    "use" => TokenKind::Use,
                    "struct" => TokenKind::Struct,
                    "enum" => TokenKind::Enum,
                    "fn" => TokenKind::Fn,
                    "let" => TokenKind::Let,
                    "match" => TokenKind::Match,
                    "if" => TokenKind::If,
                    "_" => TokenKind::Underscore,
                    _ => TokenKind::Ident(text.to_owned()),
                };
                tokens.push(token(kind, start, index));
            }
            _ => {
                return Err(lex_error(
                    source,
                    Span {
                        start: index,
                        end: index + 1,
                    },
                    "unsupported token in the current Draxl Rust profile",
                ));
            }
        }
    }

    tokens.push(token(TokenKind::Eof, source.len(), source.len()));
    Ok(tokens)
}

fn lex_error(source: &str, span: Span, message: &str) -> ParseError {
    parse_error(source, span, message)
}
