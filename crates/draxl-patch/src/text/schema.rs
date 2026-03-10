use draxl_ast::File;

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

pub(crate) fn resolve_ranked_slot(owner: NodeKind, slot: &str) -> Option<FragmentKind> {
    match (owner, slot) {
        (NodeKind::File, "items") | (NodeKind::Mod, "items") => Some(FragmentKind::Item),
        (NodeKind::Struct, "fields") => Some(FragmentKind::Field),
        (NodeKind::Enum, "variants") => Some(FragmentKind::Variant),
        (NodeKind::Fn, "params") => Some(FragmentKind::Param),
        (NodeKind::Fn, "body") | (NodeKind::ExprBlock, "body") => Some(FragmentKind::Stmt),
        (NodeKind::ExprMatch, "arms") => Some(FragmentKind::MatchArm),
        _ => None,
    }
}

pub(crate) fn resolve_single_slot(owner: NodeKind, slot: &str) -> Option<FragmentKind> {
    match (owner, slot) {
        (NodeKind::Fn, "ret") => Some(FragmentKind::Type),
        (NodeKind::Field, "ty") | (NodeKind::Param, "ty") => Some(FragmentKind::Type),
        (NodeKind::LetStmt, "pat") => Some(FragmentKind::Pattern),
        (NodeKind::LetStmt, "init") => Some(FragmentKind::Expr),
        (NodeKind::ExprStmt, "expr") => Some(FragmentKind::Expr),
        (NodeKind::ExprGroup, "expr") | (NodeKind::ExprUnary, "expr") => Some(FragmentKind::Expr),
        (NodeKind::ExprBinary, "lhs") | (NodeKind::ExprBinary, "rhs") => Some(FragmentKind::Expr),
        (NodeKind::ExprCall, "callee") => Some(FragmentKind::Expr),
        (NodeKind::ExprMatch, "scrutinee") => Some(FragmentKind::Expr),
        (NodeKind::MatchArm, "pat") => Some(FragmentKind::Pattern),
        (NodeKind::MatchArm, "guard") | (NodeKind::MatchArm, "body") => Some(FragmentKind::Expr),
        _ => None,
    }
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

pub(crate) fn set_value_kind(kind: NodeKind, path: &str) -> Option<ValueKind> {
    match (kind, path) {
        (NodeKind::Mod, "name")
        | (NodeKind::Struct, "name")
        | (NodeKind::Enum, "name")
        | (NodeKind::Fn, "name")
        | (NodeKind::Field, "name")
        | (NodeKind::Variant, "name")
        | (NodeKind::Param, "name")
        | (NodeKind::PatternIdent, "name") => Some(ValueKind::Ident),
        (NodeKind::Doc, "text") | (NodeKind::Comment, "text") => Some(ValueKind::Str),
        (NodeKind::ExprBinary, "op") | (NodeKind::ExprUnary, "op") => Some(ValueKind::Ident),
        (NodeKind::ExprStmt, "semi") => Some(ValueKind::Bool),
        _ => None,
    }
}

pub(crate) fn clear_path_supported(kind: NodeKind, path: &str) -> bool {
    matches!(
        (kind, path),
        (NodeKind::Doc, "text")
            | (NodeKind::Comment, "text")
            | (NodeKind::ExprUnary, "op")
            | (NodeKind::ExprStmt, "semi")
    )
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

pub(crate) fn find_node_kind(file: &File, node_id: &str) -> Option<NodeKind> {
    for item in &file.items {
        if let Some(kind) = find_in_item(item, node_id) {
            return Some(kind);
        }
    }
    None
}

fn find_in_item(item: &draxl_ast::Item, node_id: &str) -> Option<NodeKind> {
    if item.meta().id == node_id {
        return Some(match item {
            draxl_ast::Item::Mod(_) => NodeKind::Mod,
            draxl_ast::Item::Use(_) => NodeKind::Use,
            draxl_ast::Item::Struct(_) => NodeKind::Struct,
            draxl_ast::Item::Enum(_) => NodeKind::Enum,
            draxl_ast::Item::Fn(_) => NodeKind::Fn,
            draxl_ast::Item::Doc(_) => NodeKind::Doc,
            draxl_ast::Item::Comment(_) => NodeKind::Comment,
        });
    }

    match item {
        draxl_ast::Item::Mod(module) => {
            for child in &module.items {
                if let Some(kind) = find_in_item(child, node_id) {
                    return Some(kind);
                }
            }
            None
        }
        draxl_ast::Item::Struct(strukt) => {
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
        draxl_ast::Item::Enum(enm) => {
            for variant in &enm.variants {
                if variant.meta.id == node_id {
                    return Some(NodeKind::Variant);
                }
            }
            None
        }
        draxl_ast::Item::Fn(function) => {
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
        draxl_ast::Item::Use(_) | draxl_ast::Item::Doc(_) | draxl_ast::Item::Comment(_) => None,
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

fn find_in_stmt(stmt: &draxl_ast::Stmt, node_id: &str) -> Option<NodeKind> {
    match stmt {
        draxl_ast::Stmt::Let(node) => {
            if node.meta.id == node_id {
                return Some(NodeKind::LetStmt);
            }
            find_in_pattern(&node.pat, node_id).or_else(|| find_in_expr(&node.value, node_id))
        }
        draxl_ast::Stmt::Expr(node) => {
            if node.meta.id == node_id {
                return Some(NodeKind::ExprStmt);
            }
            find_in_expr(&node.expr, node_id)
        }
        draxl_ast::Stmt::Item(item) => find_in_item(item, node_id),
        draxl_ast::Stmt::Doc(node) => (node.meta.id == node_id).then_some(NodeKind::Doc),
        draxl_ast::Stmt::Comment(node) => (node.meta.id == node_id).then_some(NodeKind::Comment),
    }
}

fn find_in_expr(expr: &draxl_ast::Expr, node_id: &str) -> Option<NodeKind> {
    if expr.meta().is_some_and(|meta| meta.id == node_id) {
        return Some(match expr {
            draxl_ast::Expr::Path(_) => NodeKind::ExprPath,
            draxl_ast::Expr::Lit(_) => NodeKind::ExprLit,
            draxl_ast::Expr::Group(_) => NodeKind::ExprGroup,
            draxl_ast::Expr::Binary(_) => NodeKind::ExprBinary,
            draxl_ast::Expr::Unary(_) => NodeKind::ExprUnary,
            draxl_ast::Expr::Call(_) => NodeKind::ExprCall,
            draxl_ast::Expr::Match(_) => NodeKind::ExprMatch,
            draxl_ast::Expr::Block(_) => NodeKind::ExprBlock,
        });
    }

    match expr {
        draxl_ast::Expr::Group(group) => find_in_expr(&group.expr, node_id),
        draxl_ast::Expr::Binary(binary) => {
            find_in_expr(&binary.lhs, node_id).or_else(|| find_in_expr(&binary.rhs, node_id))
        }
        draxl_ast::Expr::Unary(unary) => find_in_expr(&unary.expr, node_id),
        draxl_ast::Expr::Call(call) => {
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
        draxl_ast::Expr::Match(match_expr) => {
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
        draxl_ast::Expr::Block(block) => find_in_block(block, node_id),
        draxl_ast::Expr::Path(_) | draxl_ast::Expr::Lit(_) => None,
    }
}

fn find_in_pattern(pattern: &draxl_ast::Pattern, node_id: &str) -> Option<NodeKind> {
    if pattern.meta().is_some_and(|meta| meta.id == node_id) {
        return Some(match pattern {
            draxl_ast::Pattern::Ident(_) => NodeKind::PatternIdent,
            draxl_ast::Pattern::Wild(_) => NodeKind::PatternWild,
        });
    }
    None
}
