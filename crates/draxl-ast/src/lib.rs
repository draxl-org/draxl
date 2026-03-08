#![forbid(unsafe_code)]
//! Typed IR for Draxl.
//!
//! This crate defines the shared syntax model for the entire workspace:
//!
//! - stable metadata on modeled nodes
//! - typed items, statements, expressions, patterns, and types
//! - deterministic JSON emission for test fixtures and tooling
//!
//! It deliberately does not own parsing, validation rules, formatting policy,
//! lowering, or patch application.

use std::fmt::Write;

/// Stable identifier attached to an Draxl node.
pub type NodeId = String;

/// Opaque ordering key for ordered list slots.
pub type Rank = String;

/// Explicit name for the slot that owns a node.
pub type SlotName = String;

/// Byte range in the original source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Shared metadata carried by Draxl nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    pub id: NodeId,
    pub rank: Option<Rank>,
    pub anchor: Option<NodeId>,
    pub slot: Option<SlotName>,
    pub span: Option<Span>,
}

impl Meta {
    /// Returns a copy with source spans removed.
    pub fn without_span(&self) -> Self {
        let mut meta = self.clone();
        meta.span = None;
        meta
    }
}

/// Parsed Draxl file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub items: Vec<Item>,
}

impl File {
    /// Returns a clone with all spans stripped for semantic comparisons.
    pub fn without_spans(&self) -> Self {
        let mut file = self.clone();
        file.clear_spans();
        file
    }

    /// Removes span data from the file in place.
    pub fn clear_spans(&mut self) {
        for item in &mut self.items {
            item.clear_spans();
        }
    }

    /// Emits deterministic JSON for the file.
    pub fn to_json_pretty(&self) -> String {
        let mut out = String::new();
        write_file_json(self, &mut out, 0);
        out.push('\n');
        out
    }
}

/// Top-level or module item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Mod(ItemMod),
    Use(ItemUse),
    Struct(ItemStruct),
    Enum(ItemEnum),
    Fn(ItemFn),
    Doc(DocNode),
    Comment(CommentNode),
}

impl Item {
    /// Returns the metadata for the item.
    pub fn meta(&self) -> &Meta {
        match self {
            Self::Mod(node) => &node.meta,
            Self::Use(node) => &node.meta,
            Self::Struct(node) => &node.meta,
            Self::Enum(node) => &node.meta,
            Self::Fn(node) => &node.meta,
            Self::Doc(node) => &node.meta,
            Self::Comment(node) => &node.meta,
        }
    }

    /// Returns mutable metadata for the item.
    pub fn meta_mut(&mut self) -> &mut Meta {
        match self {
            Self::Mod(node) => &mut node.meta,
            Self::Use(node) => &mut node.meta,
            Self::Struct(node) => &mut node.meta,
            Self::Enum(node) => &mut node.meta,
            Self::Fn(node) => &mut node.meta,
            Self::Doc(node) => &mut node.meta,
            Self::Comment(node) => &mut node.meta,
        }
    }

    /// Removes span data from the item in place.
    pub fn clear_spans(&mut self) {
        self.meta_mut().span = None;
        match self {
            Self::Mod(node) => {
                for item in &mut node.items {
                    item.clear_spans();
                }
            }
            Self::Use(_) => {}
            Self::Struct(node) => {
                for field in &mut node.fields {
                    field.clear_spans();
                }
            }
            Self::Enum(node) => {
                for variant in &mut node.variants {
                    variant.clear_spans();
                }
            }
            Self::Fn(node) => {
                for param in &mut node.params {
                    param.clear_spans();
                }
                if let Some(ret_ty) = &mut node.ret_ty {
                    ret_ty.clear_spans();
                }
                node.body.clear_spans();
            }
            Self::Doc(_) => {}
            Self::Comment(_) => {}
        }
    }
}

/// Module item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMod {
    pub meta: Meta,
    pub name: String,
    pub items: Vec<Item>,
}

/// Use item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemUse {
    /// Draxl metadata for the `use` item.
    pub meta: Meta,
    /// Structured `use` tree for the imported path set.
    pub tree: UseTree,
}

/// Tree form for `use` items in the bootstrap subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UseTree {
    /// Single imported name.
    Name(UseName),
    /// Prefix-plus-child path segment.
    Path(UsePathTree),
    /// Braced `use` tree group.
    Group(UseGroup),
    /// Glob import.
    Glob(UseGlob),
}

/// Single-name `use` tree segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseName {
    /// Imported name.
    pub name: String,
}

/// Prefix-plus-child `use` tree segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsePathTree {
    /// Leading path segment.
    pub prefix: String,
    /// Remaining `use` tree after the prefix.
    pub tree: Box<UseTree>,
}

/// Braced `use` tree group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseGroup {
    /// Children inside the braced group.
    pub items: Vec<UseTree>,
}

/// Glob `use` tree leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseGlob;

/// Struct item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStruct {
    pub meta: Meta,
    pub name: String,
    pub fields: Vec<Field>,
}

/// Enum item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemEnum {
    pub meta: Meta,
    pub name: String,
    pub variants: Vec<Variant>,
}

/// Function item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFn {
    pub meta: Meta,
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Option<Type>,
    pub body: Block,
}

/// Struct field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub meta: Meta,
    pub name: String,
    pub ty: Type,
}

impl Field {
    /// Removes span data from the field in place.
    pub fn clear_spans(&mut self) {
        self.meta.span = None;
        self.ty.clear_spans();
    }
}

/// Enum variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub meta: Meta,
    pub name: String,
}

impl Variant {
    /// Removes span data from the variant in place.
    pub fn clear_spans(&mut self) {
        self.meta.span = None;
    }
}

/// Function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub meta: Meta,
    pub name: String,
    pub ty: Type,
}

impl Param {
    /// Removes span data from the parameter in place.
    pub fn clear_spans(&mut self) {
        self.meta.span = None;
        self.ty.clear_spans();
    }
}

/// Block expression body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub meta: Option<Meta>,
    pub stmts: Vec<Stmt>,
}

impl Block {
    /// Removes span data from the block in place.
    pub fn clear_spans(&mut self) {
        if let Some(meta) = &mut self.meta {
            meta.span = None;
        }
        for stmt in &mut self.stmts {
            stmt.clear_spans();
        }
    }
}

/// Statement inside a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Let(StmtLet),
    Expr(StmtExpr),
    Item(Item),
    Doc(DocNode),
    Comment(CommentNode),
}

impl Stmt {
    /// Returns the metadata for ranked block children when present.
    pub fn meta(&self) -> Option<&Meta> {
        match self {
            Self::Let(node) => Some(&node.meta),
            Self::Expr(node) => Some(&node.meta),
            Self::Item(node) => Some(node.meta()),
            Self::Doc(node) => Some(&node.meta),
            Self::Comment(node) => Some(&node.meta),
        }
    }

    /// Removes span data from the statement in place.
    pub fn clear_spans(&mut self) {
        match self {
            Self::Let(node) => node.clear_spans(),
            Self::Expr(node) => node.clear_spans(),
            Self::Item(node) => node.clear_spans(),
            Self::Doc(node) => node.meta.span = None,
            Self::Comment(node) => node.meta.span = None,
        }
    }
}

/// Let statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StmtLet {
    pub meta: Meta,
    pub pat: Pattern,
    pub value: Expr,
}

impl StmtLet {
    /// Removes span data from the statement in place.
    pub fn clear_spans(&mut self) {
        self.meta.span = None;
        self.pat.clear_spans();
        self.value.clear_spans();
    }
}

/// Expression statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StmtExpr {
    /// Draxl metadata for the statement node.
    pub meta: Meta,
    /// Expression carried by the statement.
    pub expr: Expr,
    /// Whether the statement ends with a semicolon.
    pub has_semi: bool,
}

impl StmtExpr {
    /// Removes span data from the statement in place.
    pub fn clear_spans(&mut self) {
        self.meta.span = None;
        self.expr.clear_spans();
    }
}

/// Expression in the bootstrap subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Path(ExprPath),
    Lit(ExprLit),
    Group(ExprGroup),
    Binary(ExprBinary),
    Unary(ExprUnary),
    Call(ExprCall),
    Match(ExprMatch),
    Block(Block),
}

impl Expr {
    /// Returns metadata when the expression carries explicit Draxl metadata.
    pub fn meta(&self) -> Option<&Meta> {
        match self {
            Self::Path(node) => node.meta.as_ref(),
            Self::Lit(node) => node.meta.as_ref(),
            Self::Group(node) => node.meta.as_ref(),
            Self::Binary(node) => node.meta.as_ref(),
            Self::Unary(node) => node.meta.as_ref(),
            Self::Call(node) => node.meta.as_ref(),
            Self::Match(node) => node.meta.as_ref(),
            Self::Block(node) => node.meta.as_ref(),
        }
    }

    /// Returns mutable metadata when the expression carries explicit Draxl metadata.
    pub fn meta_mut(&mut self) -> Option<&mut Meta> {
        match self {
            Self::Path(node) => node.meta.as_mut(),
            Self::Lit(node) => node.meta.as_mut(),
            Self::Group(node) => node.meta.as_mut(),
            Self::Binary(node) => node.meta.as_mut(),
            Self::Unary(node) => node.meta.as_mut(),
            Self::Call(node) => node.meta.as_mut(),
            Self::Match(node) => node.meta.as_mut(),
            Self::Block(node) => node.meta.as_mut(),
        }
    }

    /// Removes span data from the expression in place.
    pub fn clear_spans(&mut self) {
        if let Some(meta) = self.meta_mut() {
            meta.span = None;
        }
        match self {
            Self::Path(_) => {}
            Self::Lit(_) => {}
            Self::Group(node) => {
                node.expr.clear_spans();
            }
            Self::Binary(node) => {
                node.lhs.clear_spans();
                node.rhs.clear_spans();
            }
            Self::Unary(node) => {
                node.expr.clear_spans();
            }
            Self::Call(node) => {
                node.callee.clear_spans();
                for arg in &mut node.args {
                    arg.clear_spans();
                }
            }
            Self::Match(node) => {
                node.scrutinee.clear_spans();
                for arm in &mut node.arms {
                    arm.clear_spans();
                }
            }
            Self::Block(node) => node.clear_spans(),
        }
    }
}

/// Path expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprPath {
    pub meta: Option<Meta>,
    pub path: Path,
}

/// Literal expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprLit {
    pub meta: Option<Meta>,
    pub value: Literal,
}

/// Grouped expression that preserves source parentheses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprGroup {
    /// Optional Draxl metadata attached to the grouped expression.
    pub meta: Option<Meta>,
    /// Inner expression wrapped by the source parentheses.
    pub expr: Box<Expr>,
}

/// Binary expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprBinary {
    pub meta: Option<Meta>,
    pub lhs: Box<Expr>,
    pub op: BinaryOp,
    pub rhs: Box<Expr>,
}

/// Unary expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprUnary {
    pub meta: Option<Meta>,
    pub op: UnaryOp,
    pub expr: Box<Expr>,
}

/// Call expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprCall {
    pub meta: Option<Meta>,
    pub callee: Box<Expr>,
    pub args: Vec<Expr>,
}

/// Match expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprMatch {
    pub meta: Option<Meta>,
    pub scrutinee: Box<Expr>,
    pub arms: Vec<MatchArm>,
}

/// Match arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub meta: Meta,
    pub pat: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

impl MatchArm {
    /// Removes span data from the match arm in place.
    pub fn clear_spans(&mut self) {
        self.meta.span = None;
        self.pat.clear_spans();
        if let Some(guard) = &mut self.guard {
            guard.clear_spans();
        }
        self.body.clear_spans();
    }
}

/// Pattern in the bootstrap subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Ident(PatIdent),
    Wild(PatWild),
}

impl Pattern {
    /// Returns metadata when the pattern carries explicit Draxl metadata.
    pub fn meta(&self) -> Option<&Meta> {
        match self {
            Self::Ident(node) => node.meta.as_ref(),
            Self::Wild(node) => node.meta.as_ref(),
        }
    }

    /// Removes span data from the pattern in place.
    pub fn clear_spans(&mut self) {
        match self {
            Self::Ident(node) => {
                if let Some(meta) = &mut node.meta {
                    meta.span = None;
                }
            }
            Self::Wild(node) => {
                if let Some(meta) = &mut node.meta {
                    meta.span = None;
                }
            }
        }
    }
}

/// Identifier pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatIdent {
    pub meta: Option<Meta>,
    pub name: String,
}

/// Wildcard pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatWild {
    pub meta: Option<Meta>,
}

/// Type in the bootstrap subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Path(TypePath),
}

impl Type {
    /// Returns the metadata attached to the type.
    pub fn meta(&self) -> &Meta {
        match self {
            Self::Path(node) => &node.meta,
        }
    }

    /// Removes span data from the type in place.
    pub fn clear_spans(&mut self) {
        match self {
            Self::Path(node) => node.meta.span = None,
        }
    }
}

/// Path type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypePath {
    pub meta: Meta,
    pub path: Path,
}

/// Path value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    pub segments: Vec<String>,
}

/// Literal value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Int(i64),
    Str(String),
}

/// Binary operator in the bootstrap subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Lt,
}

/// Unary operator in the bootstrap subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
}

/// Doc comment node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocNode {
    pub meta: Meta,
    pub text: String,
}

/// Line comment node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentNode {
    pub meta: Meta,
    pub text: String,
}

fn write_indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn write_json_string(value: &str, out: &mut String) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
}

fn write_json_meta(meta: &Meta, out: &mut String, level: usize) {
    out.push_str("{\n");
    write_indent(out, level + 1);
    out.push_str("\"id\": ");
    write_json_string(&meta.id, out);
    out.push_str(",\n");
    write_indent(out, level + 1);
    out.push_str("\"rank\": ");
    match &meta.rank {
        Some(rank) => write_json_string(rank, out),
        None => out.push_str("null"),
    }
    out.push_str(",\n");
    write_indent(out, level + 1);
    out.push_str("\"anchor\": ");
    match &meta.anchor {
        Some(anchor) => write_json_string(anchor, out),
        None => out.push_str("null"),
    }
    out.push_str(",\n");
    write_indent(out, level + 1);
    out.push_str("\"slot\": ");
    match &meta.slot {
        Some(slot) => write_json_string(slot, out),
        None => out.push_str("null"),
    }
    out.push_str(",\n");
    write_indent(out, level + 1);
    out.push_str("\"span\": ");
    match meta.span {
        Some(span) => {
            out.push_str("{\n");
            write_indent(out, level + 2);
            let _ = write!(out, "\"start\": {},\n", span.start);
            write_indent(out, level + 2);
            let _ = write!(out, "\"end\": {}\n", span.end);
            write_indent(out, level + 1);
            out.push('}');
        }
        None => out.push_str("null"),
    }
    out.push('\n');
    write_indent(out, level);
    out.push('}');
}

fn write_json_path(path: &Path, out: &mut String, level: usize) {
    out.push_str("{\n");
    write_indent(out, level + 1);
    out.push_str("\"segments\": [");
    for (index, segment) in path.segments.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write_json_string(segment, out);
    }
    out.push_str("]\n");
    write_indent(out, level);
    out.push('}');
}

fn write_json_use_tree(tree: &UseTree, out: &mut String, level: usize) {
    match tree {
        UseTree::Name(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Name\",\n");
            write_indent(out, level + 1);
            out.push_str("\"name\": ");
            write_json_string(&node.name, out);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        UseTree::Path(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Path\",\n");
            write_indent(out, level + 1);
            out.push_str("\"prefix\": ");
            write_json_string(&node.prefix, out);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"tree\": ");
            write_json_use_tree(&node.tree, out, level + 1);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        UseTree::Group(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Group\",\n");
            write_indent(out, level + 1);
            out.push_str("\"items\": [\n");
            for (index, item) in node.items.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                write_indent(out, level + 2);
                write_json_use_tree(item, out, level + 2);
            }
            out.push('\n');
            write_indent(out, level + 1);
            out.push_str("]\n");
            write_indent(out, level);
            out.push('}');
        }
        UseTree::Glob(_) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Glob\"\n");
            write_indent(out, level);
            out.push('}');
        }
    }
}

fn write_json_literal(literal: &Literal, out: &mut String, level: usize) {
    out.push_str("{\n");
    match literal {
        Literal::Int(value) => {
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Int\",\n");
            write_indent(out, level + 1);
            let _ = write!(out, "\"value\": {}\n", value);
        }
        Literal::Str(value) => {
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Str\",\n");
            write_indent(out, level + 1);
            out.push_str("\"value\": ");
            write_json_string(value, out);
            out.push('\n');
        }
    }
    write_indent(out, level);
    out.push('}');
}

fn write_json_type(ty: &Type, out: &mut String, level: usize) {
    match ty {
        Type::Path(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Path\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            write_json_meta(&node.meta, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"path\": ");
            write_json_path(&node.path, out, level + 1);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
    }
}

fn write_json_pattern(pattern: &Pattern, out: &mut String, level: usize) {
    match pattern {
        Pattern::Ident(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Ident\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            match &node.meta {
                Some(meta) => write_json_meta(meta, out, level + 1),
                None => out.push_str("null"),
            }
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"name\": ");
            write_json_string(&node.name, out);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Pattern::Wild(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Wild\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            match &node.meta {
                Some(meta) => write_json_meta(meta, out, level + 1),
                None => out.push_str("null"),
            }
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
    }
}

fn write_json_expr(expr: &Expr, out: &mut String, level: usize) {
    match expr {
        Expr::Path(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Path\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            match &node.meta {
                Some(meta) => write_json_meta(meta, out, level + 1),
                None => out.push_str("null"),
            }
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"path\": ");
            write_json_path(&node.path, out, level + 1);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Expr::Lit(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Lit\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            match &node.meta {
                Some(meta) => write_json_meta(meta, out, level + 1),
                None => out.push_str("null"),
            }
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"value\": ");
            write_json_literal(&node.value, out, level + 1);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Expr::Group(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Group\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            match &node.meta {
                Some(meta) => write_json_meta(meta, out, level + 1),
                None => out.push_str("null"),
            }
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"expr\": ");
            write_json_expr(&node.expr, out, level + 1);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Expr::Binary(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Binary\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            match &node.meta {
                Some(meta) => write_json_meta(meta, out, level + 1),
                None => out.push_str("null"),
            }
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"lhs\": ");
            write_json_expr(&node.lhs, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"op\": ");
            write_json_string(
                match node.op {
                    BinaryOp::Add => "Add",
                    BinaryOp::Sub => "Sub",
                    BinaryOp::Lt => "Lt",
                },
                out,
            );
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"rhs\": ");
            write_json_expr(&node.rhs, out, level + 1);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Expr::Unary(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Unary\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            match &node.meta {
                Some(meta) => write_json_meta(meta, out, level + 1),
                None => out.push_str("null"),
            }
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"op\": ");
            write_json_string("Neg", out);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"expr\": ");
            write_json_expr(&node.expr, out, level + 1);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Expr::Call(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Call\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            match &node.meta {
                Some(meta) => write_json_meta(meta, out, level + 1),
                None => out.push_str("null"),
            }
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"callee\": ");
            write_json_expr(&node.callee, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"args\": [\n");
            for (index, arg) in node.args.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                write_indent(out, level + 2);
                write_json_expr(arg, out, level + 2);
            }
            out.push('\n');
            write_indent(out, level + 1);
            out.push_str("]\n");
            write_indent(out, level);
            out.push('}');
        }
        Expr::Match(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Match\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            match &node.meta {
                Some(meta) => write_json_meta(meta, out, level + 1),
                None => out.push_str("null"),
            }
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"scrutinee\": ");
            write_json_expr(&node.scrutinee, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"arms\": [\n");
            for (index, arm) in node.arms.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                write_indent(out, level + 2);
                write_json_match_arm(arm, out, level + 2);
            }
            out.push('\n');
            write_indent(out, level + 1);
            out.push_str("]\n");
            write_indent(out, level);
            out.push('}');
        }
        Expr::Block(node) => write_json_block(node, out, level),
    }
}

fn write_json_match_arm(arm: &MatchArm, out: &mut String, level: usize) {
    out.push_str("{\n");
    write_indent(out, level + 1);
    out.push_str("\"meta\": ");
    write_json_meta(&arm.meta, out, level + 1);
    out.push_str(",\n");
    write_indent(out, level + 1);
    out.push_str("\"pat\": ");
    write_json_pattern(&arm.pat, out, level + 1);
    out.push_str(",\n");
    write_indent(out, level + 1);
    out.push_str("\"guard\": ");
    match &arm.guard {
        Some(guard) => write_json_expr(guard, out, level + 1),
        None => out.push_str("null"),
    }
    out.push_str(",\n");
    write_indent(out, level + 1);
    out.push_str("\"body\": ");
    write_json_expr(&arm.body, out, level + 1);
    out.push('\n');
    write_indent(out, level);
    out.push('}');
}

fn write_json_block(block: &Block, out: &mut String, level: usize) {
    out.push_str("{\n");
    write_indent(out, level + 1);
    out.push_str("\"kind\": \"Block\",\n");
    write_indent(out, level + 1);
    out.push_str("\"meta\": ");
    match &block.meta {
        Some(meta) => write_json_meta(meta, out, level + 1),
        None => out.push_str("null"),
    }
    out.push_str(",\n");
    write_indent(out, level + 1);
    out.push_str("\"stmts\": [\n");
    for (index, stmt) in block.stmts.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        write_indent(out, level + 2);
        write_json_stmt(stmt, out, level + 2);
    }
    out.push('\n');
    write_indent(out, level + 1);
    out.push_str("]\n");
    write_indent(out, level);
    out.push('}');
}

fn write_json_stmt(stmt: &Stmt, out: &mut String, level: usize) {
    match stmt {
        Stmt::Let(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Let\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            write_json_meta(&node.meta, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"pat\": ");
            write_json_pattern(&node.pat, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"value\": ");
            write_json_expr(&node.value, out, level + 1);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Stmt::Expr(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Expr\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            write_json_meta(&node.meta, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            let _ = write!(out, "\"has_semi\": {},\n", node.has_semi);
            write_indent(out, level + 1);
            out.push_str("\"expr\": ");
            write_json_expr(&node.expr, out, level + 1);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Stmt::Item(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Item\",\n");
            write_indent(out, level + 1);
            out.push_str("\"item\": ");
            write_json_item(node, out, level + 1);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Stmt::Doc(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Doc\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            write_json_meta(&node.meta, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"text\": ");
            write_json_string(&node.text, out);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Stmt::Comment(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Comment\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            write_json_meta(&node.meta, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"text\": ");
            write_json_string(&node.text, out);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
    }
}

fn write_json_item(item: &Item, out: &mut String, level: usize) {
    match item {
        Item::Mod(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Mod\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            write_json_meta(&node.meta, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"name\": ");
            write_json_string(&node.name, out);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"items\": [\n");
            for (index, child) in node.items.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                write_indent(out, level + 2);
                write_json_item(child, out, level + 2);
            }
            out.push('\n');
            write_indent(out, level + 1);
            out.push_str("]\n");
            write_indent(out, level);
            out.push('}');
        }
        Item::Use(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Use\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            write_json_meta(&node.meta, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"tree\": ");
            write_json_use_tree(&node.tree, out, level + 1);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Item::Struct(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Struct\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            write_json_meta(&node.meta, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"name\": ");
            write_json_string(&node.name, out);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"fields\": [\n");
            for (index, field) in node.fields.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                write_indent(out, level + 2);
                out.push_str("{\n");
                write_indent(out, level + 3);
                out.push_str("\"meta\": ");
                write_json_meta(&field.meta, out, level + 3);
                out.push_str(",\n");
                write_indent(out, level + 3);
                out.push_str("\"name\": ");
                write_json_string(&field.name, out);
                out.push_str(",\n");
                write_indent(out, level + 3);
                out.push_str("\"ty\": ");
                write_json_type(&field.ty, out, level + 3);
                out.push('\n');
                write_indent(out, level + 2);
                out.push('}');
            }
            out.push('\n');
            write_indent(out, level + 1);
            out.push_str("]\n");
            write_indent(out, level);
            out.push('}');
        }
        Item::Enum(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Enum\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            write_json_meta(&node.meta, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"name\": ");
            write_json_string(&node.name, out);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"variants\": [\n");
            for (index, variant) in node.variants.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                write_indent(out, level + 2);
                out.push_str("{\n");
                write_indent(out, level + 3);
                out.push_str("\"meta\": ");
                write_json_meta(&variant.meta, out, level + 3);
                out.push_str(",\n");
                write_indent(out, level + 3);
                out.push_str("\"name\": ");
                write_json_string(&variant.name, out);
                out.push('\n');
                write_indent(out, level + 2);
                out.push('}');
            }
            out.push('\n');
            write_indent(out, level + 1);
            out.push_str("]\n");
            write_indent(out, level);
            out.push('}');
        }
        Item::Fn(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Fn\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            write_json_meta(&node.meta, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"name\": ");
            write_json_string(&node.name, out);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"params\": [\n");
            for (index, param) in node.params.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                write_indent(out, level + 2);
                out.push_str("{\n");
                write_indent(out, level + 3);
                out.push_str("\"meta\": ");
                write_json_meta(&param.meta, out, level + 3);
                out.push_str(",\n");
                write_indent(out, level + 3);
                out.push_str("\"name\": ");
                write_json_string(&param.name, out);
                out.push_str(",\n");
                write_indent(out, level + 3);
                out.push_str("\"ty\": ");
                write_json_type(&param.ty, out, level + 3);
                out.push('\n');
                write_indent(out, level + 2);
                out.push('}');
            }
            out.push('\n');
            write_indent(out, level + 1);
            out.push_str("],\n");
            write_indent(out, level + 1);
            out.push_str("\"ret_ty\": ");
            match &node.ret_ty {
                Some(ret_ty) => write_json_type(ret_ty, out, level + 1),
                None => out.push_str("null"),
            }
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"body\": ");
            write_json_block(&node.body, out, level + 1);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Item::Doc(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Doc\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            write_json_meta(&node.meta, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"text\": ");
            write_json_string(&node.text, out);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
        Item::Comment(node) => {
            out.push_str("{\n");
            write_indent(out, level + 1);
            out.push_str("\"kind\": \"Comment\",\n");
            write_indent(out, level + 1);
            out.push_str("\"meta\": ");
            write_json_meta(&node.meta, out, level + 1);
            out.push_str(",\n");
            write_indent(out, level + 1);
            out.push_str("\"text\": ");
            write_json_string(&node.text, out);
            out.push('\n');
            write_indent(out, level);
            out.push('}');
        }
    }
}

fn write_file_json(file: &File, out: &mut String, level: usize) {
    out.push_str("{\n");
    write_indent(out, level + 1);
    out.push_str("\"items\": [\n");
    for (index, item) in file.items.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        write_indent(out, level + 2);
        write_json_item(item, out, level + 2);
    }
    out.push('\n');
    write_indent(out, level + 1);
    out.push_str("]\n");
    write_indent(out, level);
    out.push('}');
}
