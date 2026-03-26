use draxl_ast::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenKind {
    At,
    LBracket,
    RBracket,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Semi,
    Eq,
    FatArrow,
    Arrow,
    Plus,
    Star,
    Minus,
    Lt,
    DoubleColon,
    Ident(String),
    Int(i64),
    Str(String),
    DocComment(String),
    LineComment(String),
    Mod,
    Use,
    Struct,
    Enum,
    Fn,
    Let,
    Match,
    If,
    Underscore,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: Span,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingMeta {
    pub(crate) id: Option<String>,
    pub(crate) rank: Option<String>,
    pub(crate) anchor: Option<String>,
    pub(crate) span: Span,
}

pub(crate) fn token(kind: TokenKind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: Span { start, end },
    }
}
