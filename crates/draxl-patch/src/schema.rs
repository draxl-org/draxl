use draxl_ast::{Expr, File, Item, Pattern, Stmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeKind {
    File,
    Mod,
    Use,
    Struct,
    Enum,
    Fn,
    Field,
    Variant,
    Param,
    LetStmt,
    ExprStmt,
    MatchArm,
    PatternIdent,
    PatternWild,
    Type,
    ExprPath,
    ExprLit,
    ExprGroup,
    ExprBinary,
    ExprUnary,
    ExprCall,
    ExprMatch,
    ExprBlock,
    Doc,
    Comment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FragmentKind {
    Item,
    Field,
    Variant,
    Param,
    Stmt,
    MatchArm,
    Expr,
    Type,
    Pattern,
    Doc,
    Comment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueKind {
    Ident,
    Str,
    Bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotArity {
    Ranked,
    Single,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SlotSpec {
    pub public_name: &'static str,
    pub meta_slot_name: &'static str,
    pub fragment_kind: FragmentKind,
    pub arity: SlotArity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PathSpec {
    pub public_name: &'static str,
    pub value_kind: ValueKind,
    pub clearable: bool,
}

pub(crate) fn slot_spec(owner: NodeKind, slot: &str) -> Option<SlotSpec> {
    match (owner, slot) {
        (NodeKind::File, "items") => Some(SlotSpec {
            public_name: "items",
            meta_slot_name: "file_items",
            fragment_kind: FragmentKind::Item,
            arity: SlotArity::Ranked,
        }),
        (NodeKind::Mod, "items") => Some(SlotSpec {
            public_name: "items",
            meta_slot_name: "items",
            fragment_kind: FragmentKind::Item,
            arity: SlotArity::Ranked,
        }),
        (NodeKind::Struct, "fields") => Some(SlotSpec {
            public_name: "fields",
            meta_slot_name: "fields",
            fragment_kind: FragmentKind::Field,
            arity: SlotArity::Ranked,
        }),
        (NodeKind::Enum, "variants") => Some(SlotSpec {
            public_name: "variants",
            meta_slot_name: "variants",
            fragment_kind: FragmentKind::Variant,
            arity: SlotArity::Ranked,
        }),
        (NodeKind::Fn, "params") => Some(SlotSpec {
            public_name: "params",
            meta_slot_name: "params",
            fragment_kind: FragmentKind::Param,
            arity: SlotArity::Ranked,
        }),
        (NodeKind::Fn, "body") | (NodeKind::ExprBlock, "body") => Some(SlotSpec {
            public_name: "body",
            meta_slot_name: "body",
            fragment_kind: FragmentKind::Stmt,
            arity: SlotArity::Ranked,
        }),
        (NodeKind::ExprMatch, "arms") => Some(SlotSpec {
            public_name: "arms",
            meta_slot_name: "arms",
            fragment_kind: FragmentKind::MatchArm,
            arity: SlotArity::Ranked,
        }),
        (NodeKind::Fn, "ret") => Some(SlotSpec {
            public_name: "ret",
            meta_slot_name: "ret",
            fragment_kind: FragmentKind::Type,
            arity: SlotArity::Single,
        }),
        (NodeKind::Field, "ty") | (NodeKind::Param, "ty") => Some(SlotSpec {
            public_name: "ty",
            meta_slot_name: "ty",
            fragment_kind: FragmentKind::Type,
            arity: SlotArity::Single,
        }),
        (NodeKind::LetStmt, "pat") => Some(SlotSpec {
            public_name: "pat",
            meta_slot_name: "pat",
            fragment_kind: FragmentKind::Pattern,
            arity: SlotArity::Single,
        }),
        (NodeKind::LetStmt, "init") => Some(SlotSpec {
            public_name: "init",
            meta_slot_name: "init",
            fragment_kind: FragmentKind::Expr,
            arity: SlotArity::Single,
        }),
        (NodeKind::ExprStmt, "expr") => Some(SlotSpec {
            public_name: "expr",
            meta_slot_name: "expr",
            fragment_kind: FragmentKind::Expr,
            arity: SlotArity::Single,
        }),
        (NodeKind::ExprGroup, "expr") | (NodeKind::ExprUnary, "expr") => Some(SlotSpec {
            public_name: "expr",
            meta_slot_name: "expr",
            fragment_kind: FragmentKind::Expr,
            arity: SlotArity::Single,
        }),
        (NodeKind::ExprBinary, "lhs") => Some(SlotSpec {
            public_name: "lhs",
            meta_slot_name: "lhs",
            fragment_kind: FragmentKind::Expr,
            arity: SlotArity::Single,
        }),
        (NodeKind::ExprBinary, "rhs") => Some(SlotSpec {
            public_name: "rhs",
            meta_slot_name: "rhs",
            fragment_kind: FragmentKind::Expr,
            arity: SlotArity::Single,
        }),
        (NodeKind::ExprCall, "callee") => Some(SlotSpec {
            public_name: "callee",
            meta_slot_name: "callee",
            fragment_kind: FragmentKind::Expr,
            arity: SlotArity::Single,
        }),
        (NodeKind::ExprMatch, "scrutinee") => Some(SlotSpec {
            public_name: "scrutinee",
            meta_slot_name: "scrutinee",
            fragment_kind: FragmentKind::Expr,
            arity: SlotArity::Single,
        }),
        (NodeKind::MatchArm, "pat") => Some(SlotSpec {
            public_name: "pat",
            meta_slot_name: "pat",
            fragment_kind: FragmentKind::Pattern,
            arity: SlotArity::Single,
        }),
        (NodeKind::MatchArm, "guard") => Some(SlotSpec {
            public_name: "guard",
            meta_slot_name: "guard",
            fragment_kind: FragmentKind::Expr,
            arity: SlotArity::Single,
        }),
        (NodeKind::MatchArm, "body") => Some(SlotSpec {
            public_name: "body",
            meta_slot_name: "body",
            fragment_kind: FragmentKind::Expr,
            arity: SlotArity::Single,
        }),
        _ => None,
    }
}

pub(crate) fn ranked_slot_spec(owner: NodeKind, slot: &str) -> Option<SlotSpec> {
    slot_spec(owner, slot).filter(|spec| spec.arity == SlotArity::Ranked)
}

pub(crate) fn single_slot_spec(owner: NodeKind, slot: &str) -> Option<SlotSpec> {
    slot_spec(owner, slot).filter(|spec| spec.arity == SlotArity::Single)
}

pub(crate) fn path_spec(kind: NodeKind, path: &str) -> Option<PathSpec> {
    match (kind, path) {
        (NodeKind::Mod, "name")
        | (NodeKind::Struct, "name")
        | (NodeKind::Enum, "name")
        | (NodeKind::Fn, "name")
        | (NodeKind::Field, "name")
        | (NodeKind::Variant, "name")
        | (NodeKind::Param, "name")
        | (NodeKind::PatternIdent, "name") => Some(PathSpec {
            public_name: "name",
            value_kind: ValueKind::Ident,
            clearable: false,
        }),
        (NodeKind::Doc, "text") | (NodeKind::Comment, "text") => Some(PathSpec {
            public_name: "text",
            value_kind: ValueKind::Str,
            clearable: true,
        }),
        (NodeKind::ExprBinary, "op") => Some(PathSpec {
            public_name: "op",
            value_kind: ValueKind::Ident,
            clearable: false,
        }),
        (NodeKind::ExprUnary, "op") => Some(PathSpec {
            public_name: "op",
            value_kind: ValueKind::Ident,
            clearable: true,
        }),
        (NodeKind::ExprStmt, "semi") => Some(PathSpec {
            public_name: "semi",
            value_kind: ValueKind::Bool,
            clearable: true,
        }),
        _ => None,
    }
}

pub(crate) fn clearable_path_spec(kind: NodeKind, path: &str) -> Option<PathSpec> {
    path_spec(kind, path).filter(|spec| spec.clearable)
}

pub(crate) fn replace_fragment_kind(kind: NodeKind) -> FragmentKind {
    match kind {
        NodeKind::Mod | NodeKind::Use | NodeKind::Struct | NodeKind::Enum | NodeKind::Fn => {
            FragmentKind::Item
        }
        NodeKind::Field => FragmentKind::Field,
        NodeKind::Variant => FragmentKind::Variant,
        NodeKind::Param => FragmentKind::Param,
        NodeKind::LetStmt | NodeKind::ExprStmt => FragmentKind::Stmt,
        NodeKind::MatchArm => FragmentKind::MatchArm,
        NodeKind::PatternIdent | NodeKind::PatternWild => FragmentKind::Pattern,
        NodeKind::Type => FragmentKind::Type,
        NodeKind::ExprPath
        | NodeKind::ExprLit
        | NodeKind::ExprGroup
        | NodeKind::ExprBinary
        | NodeKind::ExprUnary
        | NodeKind::ExprCall
        | NodeKind::ExprMatch
        | NodeKind::ExprBlock => FragmentKind::Expr,
        NodeKind::Doc => FragmentKind::Doc,
        NodeKind::Comment => FragmentKind::Comment,
        NodeKind::File => FragmentKind::Item,
    }
}

pub(crate) fn item_kind(item: &Item) -> NodeKind {
    match item {
        Item::Mod(_) => NodeKind::Mod,
        Item::Use(_) => NodeKind::Use,
        Item::Struct(_) => NodeKind::Struct,
        Item::Enum(_) => NodeKind::Enum,
        Item::Fn(_) => NodeKind::Fn,
        Item::Doc(_) => NodeKind::Doc,
        Item::Comment(_) => NodeKind::Comment,
    }
}

pub(crate) fn stmt_kind(stmt: &Stmt) -> NodeKind {
    match stmt {
        Stmt::Let(_) => NodeKind::LetStmt,
        Stmt::Expr(_) => NodeKind::ExprStmt,
        Stmt::Item(item) => item_kind(item),
        Stmt::Doc(_) => NodeKind::Doc,
        Stmt::Comment(_) => NodeKind::Comment,
    }
}

pub(crate) fn expr_kind(expr: &Expr) -> NodeKind {
    match expr {
        Expr::Path(_) => NodeKind::ExprPath,
        Expr::Lit(_) => NodeKind::ExprLit,
        Expr::Group(_) => NodeKind::ExprGroup,
        Expr::Binary(_) => NodeKind::ExprBinary,
        Expr::Unary(_) => NodeKind::ExprUnary,
        Expr::Call(_) => NodeKind::ExprCall,
        Expr::Match(_) => NodeKind::ExprMatch,
        Expr::Block(_) => NodeKind::ExprBlock,
    }
}

pub(crate) fn pattern_kind(pattern: &Pattern) -> NodeKind {
    match pattern {
        Pattern::Ident(_) => NodeKind::PatternIdent,
        Pattern::Wild(_) => NodeKind::PatternWild,
    }
}

pub(crate) fn node_kind_label(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::File => "file",
        NodeKind::Mod => "module",
        NodeKind::Use => "use item",
        NodeKind::Struct => "struct",
        NodeKind::Enum => "enum",
        NodeKind::Fn => "function",
        NodeKind::Field => "field",
        NodeKind::Variant => "variant",
        NodeKind::Param => "parameter",
        NodeKind::LetStmt => "let statement",
        NodeKind::ExprStmt => "expression statement",
        NodeKind::MatchArm => "match arm",
        NodeKind::PatternIdent => "identifier pattern",
        NodeKind::PatternWild => "wildcard pattern",
        NodeKind::Type => "type",
        NodeKind::ExprPath => "path expression",
        NodeKind::ExprLit => "literal expression",
        NodeKind::ExprGroup => "grouped expression",
        NodeKind::ExprBinary => "binary expression",
        NodeKind::ExprUnary => "unary expression",
        NodeKind::ExprCall => "call expression",
        NodeKind::ExprMatch => "match expression",
        NodeKind::ExprBlock => "block expression",
        NodeKind::Doc => "doc comment",
        NodeKind::Comment => "line comment",
    }
}

pub(crate) fn value_kind_label(value_kind: ValueKind) -> &'static str {
    match value_kind {
        ValueKind::Ident => "an identifier value",
        ValueKind::Str => "a string value",
        ValueKind::Bool => "a boolean value",
    }
}

pub(crate) fn invalid_ranked_slot_message(owner_label: &str, slot: &str) -> String {
    format!("slot `{owner_label}.{slot}` is not available for ranked insertion")
}

pub(crate) fn invalid_single_slot_message(owner_label: &str, slot: &str) -> String {
    format!("slot `{owner_label}.{slot}` is not available for `put`")
}

pub(crate) fn invalid_set_path_message(node_id: &str, path: &str, kind: NodeKind) -> String {
    format!(
        "path `@{node_id}.{path}` is not settable on {}",
        node_kind_label(kind)
    )
}

pub(crate) fn invalid_clear_path_message(node_id: &str, path: &str, kind: NodeKind) -> String {
    format!(
        "path `@{node_id}.{path}` is not clearable on {}",
        node_kind_label(kind)
    )
}

pub(crate) fn find_node_kind(file: &File, node_id: &str) -> Option<NodeKind> {
    for item in &file.items {
        if let Some(kind) = find_in_item(item, node_id) {
            return Some(kind);
        }
    }
    None
}

fn find_in_item(item: &Item, node_id: &str) -> Option<NodeKind> {
    if item.meta().id == node_id {
        return Some(item_kind(item));
    }

    match item {
        Item::Mod(module) => {
            for child in &module.items {
                if let Some(kind) = find_in_item(child, node_id) {
                    return Some(kind);
                }
            }
            None
        }
        Item::Struct(strukt) => {
            for field in &strukt.fields {
                if field.meta.id == node_id {
                    return Some(NodeKind::Field);
                }
                if field.ty.meta().id == node_id {
                    return Some(NodeKind::Type);
                }
            }
            None
        }
        Item::Enum(enm) => {
            for variant in &enm.variants {
                if variant.meta.id == node_id {
                    return Some(NodeKind::Variant);
                }
            }
            None
        }
        Item::Fn(function) => {
            for param in &function.params {
                if param.meta.id == node_id {
                    return Some(NodeKind::Param);
                }
                if param.ty.meta().id == node_id {
                    return Some(NodeKind::Type);
                }
            }
            if function
                .ret_ty
                .as_ref()
                .is_some_and(|ret_ty| ret_ty.meta().id == node_id)
            {
                return Some(NodeKind::Type);
            }
            find_in_block(&function.body, node_id)
        }
        Item::Use(_) | Item::Doc(_) | Item::Comment(_) => None,
    }
}

fn find_in_block(block: &draxl_ast::Block, node_id: &str) -> Option<NodeKind> {
    if block.meta.as_ref().is_some_and(|meta| meta.id == node_id) {
        return Some(NodeKind::ExprBlock);
    }

    for stmt in &block.stmts {
        if let Some(kind) = find_in_stmt(stmt, node_id) {
            return Some(kind);
        }
    }
    None
}

fn find_in_stmt(stmt: &Stmt, node_id: &str) -> Option<NodeKind> {
    match stmt {
        Stmt::Let(node) => {
            if node.meta.id == node_id {
                return Some(NodeKind::LetStmt);
            }
            find_in_pattern(&node.pat, node_id).or_else(|| find_in_expr(&node.value, node_id))
        }
        Stmt::Expr(node) => {
            if node.meta.id == node_id {
                return Some(NodeKind::ExprStmt);
            }
            find_in_expr(&node.expr, node_id)
        }
        Stmt::Item(item) => find_in_item(item, node_id),
        Stmt::Doc(node) => (node.meta.id == node_id).then_some(NodeKind::Doc),
        Stmt::Comment(node) => (node.meta.id == node_id).then_some(NodeKind::Comment),
    }
}

fn find_in_expr(expr: &Expr, node_id: &str) -> Option<NodeKind> {
    if expr.meta().is_some_and(|meta| meta.id == node_id) {
        return Some(expr_kind(expr));
    }

    match expr {
        Expr::Group(group) => find_in_expr(&group.expr, node_id),
        Expr::Binary(binary) => {
            find_in_expr(&binary.lhs, node_id).or_else(|| find_in_expr(&binary.rhs, node_id))
        }
        Expr::Unary(unary) => find_in_expr(&unary.expr, node_id),
        Expr::Call(call) => {
            if let Some(kind) = find_in_expr(&call.callee, node_id) {
                return Some(kind);
            }
            for arg in &call.args {
                if let Some(kind) = find_in_expr(arg, node_id) {
                    return Some(kind);
                }
            }
            None
        }
        Expr::Match(match_expr) => {
            if let Some(kind) = find_in_expr(&match_expr.scrutinee, node_id) {
                return Some(kind);
            }
            for arm in &match_expr.arms {
                if arm.meta.id == node_id {
                    return Some(NodeKind::MatchArm);
                }
                if let Some(kind) = find_in_pattern(&arm.pat, node_id) {
                    return Some(kind);
                }
                if let Some(guard) = &arm.guard {
                    if let Some(kind) = find_in_expr(guard, node_id) {
                        return Some(kind);
                    }
                }
                if let Some(kind) = find_in_expr(&arm.body, node_id) {
                    return Some(kind);
                }
            }
            None
        }
        Expr::Block(block) => find_in_block(block, node_id),
        Expr::Path(_) | Expr::Lit(_) => None,
    }
}

fn find_in_pattern(pattern: &Pattern, node_id: &str) -> Option<NodeKind> {
    if pattern.meta().is_some_and(|meta| meta.id == node_id) {
        return Some(pattern_kind(pattern));
    }
    None
}
