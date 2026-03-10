use super::error::{patch_text_error, PatchTextError};
use super::surface::{
    SurfaceDest, SurfaceFragment, SurfaceNodeRef, SurfacePatchOp, SurfacePath, SurfacePathSegment,
    SurfaceRankedDest, SurfaceSlotOwner, SurfaceSlotRef, SurfaceValue, SurfaceValueKind,
};
use draxl_ast::Span;

pub(super) fn parse_patch_ops(source: &str) -> Result<Vec<SurfacePatchOp>, PatchTextError> {
    PatchTextParser::new(source).parse_ops()
}

struct PatchTextParser<'a> {
    source: &'a str,
    bytes: &'a [u8],
    index: usize,
}

impl<'a> PatchTextParser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            index: 0,
        }
    }

    fn parse_ops(&mut self) -> Result<Vec<SurfacePatchOp>, PatchTextError> {
        let mut ops = Vec::new();
        self.skip_whitespace();
        while !self.at_eof() {
            ops.push(self.parse_op()?);
            self.skip_whitespace();
        }
        Ok(ops)
    }

    fn parse_op(&mut self) -> Result<SurfacePatchOp, PatchTextError> {
        let start = self.index;
        let (verb, _) = self.parse_ident()?;
        match verb.as_str() {
            "insert" => self.parse_insert(start),
            "put" => self.parse_put(start),
            "replace" => self.parse_replace(start),
            "delete" => self.parse_delete(start),
            "move" => self.parse_move(start),
            "set" => self.parse_set(start),
            "clear" => self.parse_clear(start),
            "attach" => self.parse_attach(start),
            "detach" => self.parse_detach(start),
            _ => Err(self.error_at_span(
                Span {
                    start,
                    end: self.index,
                },
                "expected a patch op such as `insert`, `replace`, or `set`",
            )),
        }
    }

    fn parse_insert(&mut self, start: usize) -> Result<SurfacePatchOp, PatchTextError> {
        self.skip_inline_ws_required("expected a destination after `insert`")?;
        let dest = self.parse_ranked_dest()?;
        self.skip_inline_ws();
        self.expect_char(b':', "expected `:` after insert destination")?;
        let fragment = self.parse_fragment()?;
        Ok(SurfacePatchOp::Insert {
            dest,
            span: Span {
                start,
                end: fragment.span.end,
            },
            fragment,
        })
    }

    fn parse_put(&mut self, start: usize) -> Result<SurfacePatchOp, PatchTextError> {
        self.skip_inline_ws_required("expected a destination after `put`")?;
        let slot = self.parse_slot_ref()?;
        self.skip_inline_ws();
        self.expect_char(b':', "expected `:` after put destination")?;
        let fragment = self.parse_fragment()?;
        Ok(SurfacePatchOp::Put {
            span: Span {
                start,
                end: fragment.span.end,
            },
            slot,
            fragment,
        })
    }

    fn parse_replace(&mut self, start: usize) -> Result<SurfacePatchOp, PatchTextError> {
        self.skip_inline_ws_required("expected a target after `replace`")?;
        let target = self.parse_node_ref()?;
        self.skip_inline_ws();
        self.expect_char(b':', "expected `:` after replace target")?;
        let fragment = self.parse_fragment()?;
        Ok(SurfacePatchOp::Replace {
            span: Span {
                start,
                end: fragment.span.end,
            },
            target,
            fragment,
        })
    }

    fn parse_delete(&mut self, start: usize) -> Result<SurfacePatchOp, PatchTextError> {
        self.skip_inline_ws_required("expected a target after `delete`")?;
        let target = self.parse_node_ref()?;
        let end = self.expect_op_end()?;
        Ok(SurfacePatchOp::Delete {
            target,
            span: Span { start, end },
        })
    }

    fn parse_move(&mut self, start: usize) -> Result<SurfacePatchOp, PatchTextError> {
        self.skip_inline_ws_required("expected a target after `move`")?;
        let target = self.parse_node_ref()?;
        self.skip_inline_ws();
        self.expect_arrow("expected `->` after move target")?;
        self.skip_inline_ws();
        let dest = self.parse_dest()?;
        let end = self.expect_op_end()?;
        Ok(SurfacePatchOp::Move {
            target,
            dest,
            span: Span { start, end },
        })
    }

    fn parse_set(&mut self, start: usize) -> Result<SurfacePatchOp, PatchTextError> {
        self.skip_inline_ws_required("expected a path after `set`")?;
        let path = self.parse_path()?;
        self.skip_inline_ws();
        self.expect_char(b'=', "expected `=` after set path")?;
        self.skip_inline_ws();
        let value = self.parse_value()?;
        let end = self.expect_op_end()?;
        Ok(SurfacePatchOp::Set {
            path,
            value,
            span: Span { start, end },
        })
    }

    fn parse_clear(&mut self, start: usize) -> Result<SurfacePatchOp, PatchTextError> {
        self.skip_inline_ws_required("expected a path after `clear`")?;
        let path = self.parse_path()?;
        let end = self.expect_op_end()?;
        Ok(SurfacePatchOp::Clear {
            path,
            span: Span { start, end },
        })
    }

    fn parse_attach(&mut self, start: usize) -> Result<SurfacePatchOp, PatchTextError> {
        self.skip_inline_ws_required("expected a source node after `attach`")?;
        let node = self.parse_node_ref()?;
        self.skip_inline_ws();
        self.expect_arrow("expected `->` after attach source")?;
        self.skip_inline_ws();
        let target = self.parse_node_ref()?;
        let end = self.expect_op_end()?;
        Ok(SurfacePatchOp::Attach {
            node,
            target,
            span: Span { start, end },
        })
    }

    fn parse_detach(&mut self, start: usize) -> Result<SurfacePatchOp, PatchTextError> {
        self.skip_inline_ws_required("expected a source node after `detach`")?;
        let node = self.parse_node_ref()?;
        let end = self.expect_op_end()?;
        Ok(SurfacePatchOp::Detach {
            node,
            span: Span { start, end },
        })
    }

    fn parse_dest(&mut self) -> Result<SurfaceDest, PatchTextError> {
        let slot = self.parse_slot_ref()?;
        if self.peek() == Some(b'[') {
            let start = slot.span.start;
            self.bump();
            let (rank, rank_span) = self.parse_ident()?;
            self.expect_char(b']', "expected `]` after rank")?;
            let span = Span {
                start,
                end: self.index,
            };
            Ok(SurfaceDest::Ranked(SurfaceRankedDest {
                slot,
                rank,
                rank_span,
                span,
            }))
        } else {
            Ok(SurfaceDest::Slot(slot))
        }
    }

    fn parse_ranked_dest(&mut self) -> Result<SurfaceRankedDest, PatchTextError> {
        let slot = self.parse_slot_ref()?;
        let start = slot.span.start;
        self.expect_char(b'[', "expected `[` after ranked destination slot")?;
        let (rank, rank_span) = self.parse_ident()?;
        self.expect_char(b']', "expected `]` after rank")?;
        Ok(SurfaceRankedDest {
            slot,
            rank,
            rank_span,
            span: Span {
                start,
                end: self.index,
            },
        })
    }

    fn parse_slot_ref(&mut self) -> Result<SurfaceSlotRef, PatchTextError> {
        let start = self.index;
        let owner = if self.starts_with_word("file") {
            let span = Span {
                start,
                end: start + 4,
            };
            self.index += 4;
            SurfaceSlotOwner::File { span }
        } else {
            SurfaceSlotOwner::Node(self.parse_node_ref()?)
        };
        self.expect_char(b'.', "expected `.` in slot reference")?;
        let (slot, slot_span) = self.parse_ident()?;
        Ok(SurfaceSlotRef {
            owner,
            slot,
            slot_span,
            span: Span {
                start,
                end: slot_span.end,
            },
        })
    }

    fn parse_path(&mut self) -> Result<SurfacePath, PatchTextError> {
        let node = self.parse_node_ref()?;
        let start = node.span.start;
        let mut segments = Vec::new();
        while self.peek() == Some(b'.') {
            self.bump();
            let (name, span) = self.parse_ident()?;
            segments.push(SurfacePathSegment { name, span });
        }
        if segments.is_empty() {
            return Err(self.error_current("expected a scalar path like `@f1.name`"));
        }
        Ok(SurfacePath {
            node,
            span: Span {
                start,
                end: segments.last().expect("path segment must exist").span.end,
            },
            segments,
        })
    }

    fn parse_node_ref(&mut self) -> Result<SurfaceNodeRef, PatchTextError> {
        let start = self.index;
        self.expect_char(b'@', "expected a node ref such as `@f1`")?;
        let (id, span) = self.parse_ident()?;
        Ok(SurfaceNodeRef {
            id,
            span: Span {
                start,
                end: span.end,
            },
        })
    }

    fn parse_value(&mut self) -> Result<SurfaceValue, PatchTextError> {
        if self.peek() == Some(b'"') {
            let (value, span) = self.parse_string()?;
            return Ok(SurfaceValue {
                kind: SurfaceValueKind::Str(value),
                span,
            });
        }

        if self.peek() == Some(b'-') && self.peek_next().is_some_and(|byte| byte.is_ascii_digit()) {
            let start = self.index;
            self.bump();
            let (digits, _) = self.parse_digits()?;
            let value = format!("-{digits}").parse::<i64>().map_err(|_| {
                self.error_at_span(
                    Span {
                        start,
                        end: self.index,
                    },
                    "integer literal is outside the supported i64 range",
                )
            })?;
            return Ok(SurfaceValue {
                kind: SurfaceValueKind::Int(value),
                span: Span {
                    start,
                    end: self.index,
                },
            });
        }

        if self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            let (digits, span) = self.parse_digits()?;
            let value = digits.parse::<i64>().map_err(|_| {
                self.error_at_span(span, "integer literal is outside the supported i64 range")
            })?;
            return Ok(SurfaceValue {
                kind: SurfaceValueKind::Int(value),
                span,
            });
        }

        let (ident, span) = self.parse_ident()?;
        let kind = match ident.as_str() {
            "true" => SurfaceValueKind::Bool(true),
            "false" => SurfaceValueKind::Bool(false),
            _ => SurfaceValueKind::Ident(ident),
        };
        Ok(SurfaceValue { kind, span })
    }

    fn parse_fragment(&mut self) -> Result<SurfaceFragment, PatchTextError> {
        self.skip_whitespace();
        let start = self.index;
        if self.at_eof() {
            return Err(self.error_current("expected a Draxl fragment after `:`"));
        }

        let mut index = self.index;
        let mut parens = 0usize;
        let mut braces = 0usize;
        let mut brackets = 0usize;

        while index < self.bytes.len() {
            match self.bytes[index] {
                b'"' => index = self.scan_string(index)?,
                b'/' if self.bytes.get(index + 1).is_some_and(|byte| *byte == b'/') => {
                    index += 2;
                    while index < self.bytes.len() && self.bytes[index] != b'\n' {
                        index += 1;
                    }
                }
                b'(' => {
                    parens += 1;
                    index += 1;
                }
                b')' => {
                    parens = parens.saturating_sub(1);
                    index += 1;
                }
                b'{' => {
                    braces += 1;
                    index += 1;
                }
                b'}' => {
                    braces = braces.saturating_sub(1);
                    index += 1;
                }
                b'[' => {
                    brackets += 1;
                    index += 1;
                }
                b']' => {
                    brackets = brackets.saturating_sub(1);
                    index += 1;
                }
                b'\n' if parens == 0 && braces == 0 && brackets == 0 => {
                    let next = index + 1;
                    let next_non_space = self.skip_horizontal_from(next);
                    if next_non_space >= self.bytes.len()
                        || self.bytes[next_non_space] == b'\n'
                        || self.starts_with_patch_keyword(next_non_space)
                    {
                        let end = trim_horizontal_end(self.source, start, index);
                        if end == start {
                            return Err(self.error_at_span(
                                Span { start, end: index },
                                "expected a Draxl fragment after `:`",
                            ));
                        }
                        self.index = index;
                        return Ok(SurfaceFragment {
                            source: self.source[start..end].to_owned(),
                            span: Span { start, end },
                        });
                    }
                    index += 1;
                }
                _ => {
                    index += 1;
                }
            }
        }

        let end = trim_horizontal_end(self.source, start, self.bytes.len());
        if end == start {
            return Err(
                self.error_at_span(Span { start, end }, "expected a Draxl fragment after `:`")
            );
        }
        self.index = self.bytes.len();
        Ok(SurfaceFragment {
            source: self.source[start..end].to_owned(),
            span: Span { start, end },
        })
    }

    fn scan_string(&self, start: usize) -> Result<usize, PatchTextError> {
        let mut index = start + 1;
        while index < self.bytes.len() {
            match self.bytes[index] {
                b'"' => return Ok(index + 1),
                b'\\' => {
                    index += 1;
                    if index >= self.bytes.len() {
                        return Err(self.error_at_span(
                            Span { start, end: index },
                            "unterminated string literal",
                        ));
                    }
                    index += 1;
                }
                _ => index += 1,
            }
        }
        Err(self.error_at_span(
            Span {
                start,
                end: self.bytes.len(),
            },
            "unterminated string literal",
        ))
    }

    fn parse_string(&mut self) -> Result<(String, Span), PatchTextError> {
        let start = self.index;
        self.expect_char(b'"', "expected string literal")?;
        let mut value = String::new();
        while let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    self.bump();
                    return Ok((
                        value,
                        Span {
                            start,
                            end: self.index,
                        },
                    ));
                }
                b'\\' => {
                    self.bump();
                    let escaped = match self.peek() {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'n') => '\n',
                        Some(b'r') => '\r',
                        Some(b't') => '\t',
                        Some(other) => {
                            return Err(self.error_at_span(
                                Span {
                                    start: self.index,
                                    end: self.index + 1,
                                },
                                &format!("unsupported string escape `\\{}`", other as char),
                            ));
                        }
                        None => {
                            return Err(self.error_at_span(
                                Span {
                                    start,
                                    end: self.index,
                                },
                                "unterminated string literal",
                            ));
                        }
                    };
                    self.bump();
                    value.push(escaped);
                }
                other => {
                    self.bump();
                    value.push(other as char);
                }
            }
        }
        Err(self.error_at_span(
            Span {
                start,
                end: self.index,
            },
            "unterminated string literal",
        ))
    }

    fn parse_digits(&mut self) -> Result<(String, Span), PatchTextError> {
        let start = self.index;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.bump();
        }
        if start == self.index {
            return Err(self.error_current("expected integer literal"));
        }
        Ok((
            self.source[start..self.index].to_owned(),
            Span {
                start,
                end: self.index,
            },
        ))
    }

    fn parse_ident(&mut self) -> Result<(String, Span), PatchTextError> {
        let start = self.index;
        let Some(first) = self.peek() else {
            return Err(self.error_current("expected identifier"));
        };
        if !is_ident_start(first) {
            return Err(self.error_current("expected identifier"));
        }
        self.bump();
        while self.peek().is_some_and(is_ident_continue) {
            self.bump();
        }
        Ok((
            self.source[start..self.index].to_owned(),
            Span {
                start,
                end: self.index,
            },
        ))
    }

    fn expect_op_end(&mut self) -> Result<usize, PatchTextError> {
        self.skip_inline_ws();
        if self.at_eof() || self.peek() == Some(b'\n') {
            Ok(self.index)
        } else {
            Err(self.error_current("unexpected trailing tokens after patch op"))
        }
    }

    fn skip_whitespace(&mut self) {
        while self
            .peek()
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        {
            self.bump();
        }
    }

    fn skip_inline_ws(&mut self) {
        while self
            .peek()
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r'))
        {
            self.bump();
        }
    }

    fn skip_inline_ws_required(&mut self, message: &str) -> Result<(), PatchTextError> {
        let start = self.index;
        self.skip_inline_ws();
        if self.index == start {
            Err(self.error_current(message))
        } else {
            Ok(())
        }
    }

    fn skip_horizontal_from(&self, mut index: usize) -> usize {
        while index < self.bytes.len() && matches!(self.bytes[index], b' ' | b'\t' | b'\r') {
            index += 1;
        }
        index
    }

    fn starts_with_patch_keyword(&self, index: usize) -> bool {
        PATCH_KEYWORDS
            .iter()
            .any(|keyword| self.starts_with_word_at(index, keyword))
    }

    fn starts_with_word(&self, word: &str) -> bool {
        self.starts_with_word_at(self.index, word)
    }

    fn starts_with_word_at(&self, index: usize, word: &str) -> bool {
        let bytes = word.as_bytes();
        let end = index + bytes.len();
        if end > self.bytes.len() || &self.bytes[index..end] != bytes {
            return false;
        }
        if index > 0 && self.bytes[index - 1].is_ascii_alphanumeric() {
            return false;
        }
        self.bytes
            .get(end)
            .is_none_or(|byte| !is_ident_continue(*byte))
    }

    fn expect_char(&mut self, expected: u8, message: &str) -> Result<(), PatchTextError> {
        if self.peek() == Some(expected) {
            self.bump();
            Ok(())
        } else {
            Err(self.error_current(message))
        }
    }

    fn expect_arrow(&mut self, message: &str) -> Result<(), PatchTextError> {
        if self.peek() == Some(b'-') && self.peek_next() == Some(b'>') {
            self.bump();
            self.bump();
            Ok(())
        } else {
            Err(self.error_current(message))
        }
    }

    fn error_current(&self, message: &str) -> PatchTextError {
        self.error_at_span(
            Span {
                start: self.index,
                end: self.index.min(self.bytes.len()),
            },
            message,
        )
    }

    fn error_at_span(&self, span: Span, message: &str) -> PatchTextError {
        patch_text_error(self.source, span, message)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.bytes.get(self.index + 1).copied()
    }

    fn bump(&mut self) {
        self.index += 1;
    }

    fn at_eof(&self) -> bool {
        self.index >= self.bytes.len()
    }
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn trim_horizontal_end(source: &str, start: usize, mut end: usize) -> usize {
    while end > start {
        match source.as_bytes()[end - 1] {
            b' ' | b'\t' | b'\r' => end -= 1,
            _ => break,
        }
    }
    end
}

const PATCH_KEYWORDS: [&str; 9] = [
    "insert", "put", "replace", "delete", "move", "set", "clear", "attach", "detach",
];
