use super::support::{
    is_item_trivia, is_stmt_trivia, semantic_item_target_ids, semantic_stmt_target_ids,
};
use crate::error::{patch_error, PatchError};
use crate::schema::{
    attach_target_not_sibling_message, detach_requires_following_sibling_message, find_node_kind,
    is_attachable_kind,
};
use draxl_ast::{Block, Expr, File, Item, LowerLanguage, Stmt};

pub(super) fn apply_attach(
    language: LowerLanguage,
    file: &mut File,
    node_id: &str,
    target_id: &str,
) -> Result<(), PatchError> {
    if !find_node_kind(language, file, node_id)
        .is_some_and(|kind| is_attachable_kind(language, kind))
    {
        return Err(patch_error(&format!(
            "attach source `{node_id}` was not found or is not attachable"
        )));
    }

    if attach_in_items(language, &mut file.items, node_id, target_id)? {
        Ok(())
    } else {
        Err(patch_error(&format!(
            "attach source `{node_id}` was not found or is not attachable"
        )))
    }
}

pub(super) fn apply_detach(
    language: LowerLanguage,
    file: &mut File,
    node_id: &str,
) -> Result<(), PatchError> {
    if !find_node_kind(language, file, node_id)
        .is_some_and(|kind| is_attachable_kind(language, kind))
    {
        return Err(patch_error(&format!(
            "detach source `{node_id}` was not found or is not attachable"
        )));
    }

    if detach_in_items(language, &mut file.items, node_id)? {
        Ok(())
    } else {
        Err(patch_error(&format!(
            "detach source `{node_id}` was not found or is not attachable"
        )))
    }
}

fn attach_in_items(
    language: LowerLanguage,
    items: &mut Vec<Item>,
    node_id: &str,
    target_id: &str,
) -> Result<bool, PatchError> {
    let local_targets = semantic_item_target_ids(items)
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for item in items.iter_mut() {
        match item {
            Item::Doc(node) if node.meta.id == node_id => {
                if !local_targets.iter().any(|candidate| candidate == target_id) {
                    return Err(patch_error(&attach_target_not_sibling_message(
                        language, target_id, node_id,
                    )));
                }
                node.meta.anchor = Some(target_id.to_owned());
                return Ok(true);
            }
            Item::Comment(node) if node.meta.id == node_id => {
                if !local_targets.iter().any(|candidate| candidate == target_id) {
                    return Err(patch_error(&attach_target_not_sibling_message(
                        language, target_id, node_id,
                    )));
                }
                node.meta.anchor = Some(target_id.to_owned());
                return Ok(true);
            }
            _ => {}
        }
    }

    for item in items {
        if let Some(found) = recurse_item_for_attach(language, item, node_id, target_id)? {
            return Ok(found);
        }
    }

    Ok(false)
}

fn recurse_item_for_attach(
    language: LowerLanguage,
    item: &mut Item,
    node_id: &str,
    target_id: &str,
) -> Result<Option<bool>, PatchError> {
    match item {
        Item::Mod(module) => {
            attach_in_items(language, &mut module.items, node_id, target_id).map(Some)
        }
        Item::Fn(function) => {
            attach_in_stmts(language, &mut function.body.stmts, node_id, target_id).map(Some)
        }
        Item::Use(_) | Item::Struct(_) | Item::Enum(_) | Item::Doc(_) | Item::Comment(_) => {
            Ok(None)
        }
    }
}

fn attach_in_stmts(
    language: LowerLanguage,
    stmts: &mut Vec<Stmt>,
    node_id: &str,
    target_id: &str,
) -> Result<bool, PatchError> {
    let local_targets = semantic_stmt_target_ids(stmts)
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for stmt in stmts.iter_mut() {
        match stmt {
            Stmt::Doc(node) if node.meta.id == node_id => {
                if !local_targets.iter().any(|candidate| candidate == target_id) {
                    return Err(patch_error(&attach_target_not_sibling_message(
                        language, target_id, node_id,
                    )));
                }
                node.meta.anchor = Some(target_id.to_owned());
                return Ok(true);
            }
            Stmt::Comment(node) if node.meta.id == node_id => {
                if !local_targets.iter().any(|candidate| candidate == target_id) {
                    return Err(patch_error(&attach_target_not_sibling_message(
                        language, target_id, node_id,
                    )));
                }
                node.meta.anchor = Some(target_id.to_owned());
                return Ok(true);
            }
            _ => {}
        }
    }

    for stmt in stmts {
        if let Some(found) = recurse_stmt_for_attach(language, stmt, node_id, target_id)? {
            return Ok(found);
        }
    }

    Ok(false)
}

fn recurse_stmt_for_attach(
    language: LowerLanguage,
    stmt: &mut Stmt,
    node_id: &str,
    target_id: &str,
) -> Result<Option<bool>, PatchError> {
    match stmt {
        Stmt::Let(let_stmt) => {
            attach_in_expr(language, &mut let_stmt.value, node_id, target_id).map(Some)
        }
        Stmt::Expr(expr_stmt) => {
            attach_in_expr(language, &mut expr_stmt.expr, node_id, target_id).map(Some)
        }
        Stmt::Item(item) => recurse_item_for_attach(language, item, node_id, target_id),
        Stmt::Doc(_) | Stmt::Comment(_) => Ok(None),
    }
}

fn attach_in_expr(
    language: LowerLanguage,
    expr: &mut Expr,
    node_id: &str,
    target_id: &str,
) -> Result<bool, PatchError> {
    match expr {
        Expr::Group(group) => attach_in_expr(language, &mut group.expr, node_id, target_id),
        Expr::Binary(binary) => {
            if attach_in_expr(language, &mut binary.lhs, node_id, target_id)? {
                return Ok(true);
            }
            attach_in_expr(language, &mut binary.rhs, node_id, target_id)
        }
        Expr::Unary(unary) => attach_in_expr(language, &mut unary.expr, node_id, target_id),
        Expr::Call(call) => {
            if attach_in_expr(language, &mut call.callee, node_id, target_id)? {
                return Ok(true);
            }
            for arg in &mut call.args {
                if attach_in_expr(language, arg, node_id, target_id)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Match(match_expr) => {
            if attach_in_expr(language, &mut match_expr.scrutinee, node_id, target_id)? {
                return Ok(true);
            }
            for arm in &mut match_expr.arms {
                if let Some(guard) = &mut arm.guard {
                    if attach_in_expr(language, guard, node_id, target_id)? {
                        return Ok(true);
                    }
                }
                if attach_in_expr(language, &mut arm.body, node_id, target_id)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Block(Block { stmts, .. }) => attach_in_stmts(language, stmts, node_id, target_id),
        Expr::Path(_) | Expr::Lit(_) => Ok(false),
    }
}

fn detach_in_items(
    language: LowerLanguage,
    items: &mut Vec<Item>,
    node_id: &str,
) -> Result<bool, PatchError> {
    for index in 0..items.len() {
        let has_following = has_following_semantic_item(items, index);
        match &mut items[index] {
            Item::Doc(node) if node.meta.id == node_id => {
                if !has_following {
                    return Err(patch_error(&detach_requires_following_sibling_message(
                        language, node_id,
                    )));
                }
                node.meta.anchor = None;
                return Ok(true);
            }
            Item::Comment(node) if node.meta.id == node_id => {
                if !has_following {
                    return Err(patch_error(&detach_requires_following_sibling_message(
                        language, node_id,
                    )));
                }
                node.meta.anchor = None;
                return Ok(true);
            }
            _ => {}
        }
    }

    for item in items {
        if let Some(found) = recurse_item_for_detach(language, item, node_id)? {
            return Ok(found);
        }
    }

    Ok(false)
}

fn recurse_item_for_detach(
    language: LowerLanguage,
    item: &mut Item,
    node_id: &str,
) -> Result<Option<bool>, PatchError> {
    match item {
        Item::Mod(module) => detach_in_items(language, &mut module.items, node_id).map(Some),
        Item::Fn(function) => {
            detach_in_stmts(language, &mut function.body.stmts, node_id).map(Some)
        }
        Item::Use(_) | Item::Struct(_) | Item::Enum(_) | Item::Doc(_) | Item::Comment(_) => {
            Ok(None)
        }
    }
}

fn detach_in_stmts(
    language: LowerLanguage,
    stmts: &mut Vec<Stmt>,
    node_id: &str,
) -> Result<bool, PatchError> {
    for index in 0..stmts.len() {
        let has_following = has_following_semantic_stmt(stmts, index);
        match &mut stmts[index] {
            Stmt::Doc(node) if node.meta.id == node_id => {
                if !has_following {
                    return Err(patch_error(&detach_requires_following_sibling_message(
                        language, node_id,
                    )));
                }
                node.meta.anchor = None;
                return Ok(true);
            }
            Stmt::Comment(node) if node.meta.id == node_id => {
                if !has_following {
                    return Err(patch_error(&detach_requires_following_sibling_message(
                        language, node_id,
                    )));
                }
                node.meta.anchor = None;
                return Ok(true);
            }
            _ => {}
        }
    }

    for stmt in stmts {
        if let Some(found) = recurse_stmt_for_detach(language, stmt, node_id)? {
            return Ok(found);
        }
    }

    Ok(false)
}

fn recurse_stmt_for_detach(
    language: LowerLanguage,
    stmt: &mut Stmt,
    node_id: &str,
) -> Result<Option<bool>, PatchError> {
    match stmt {
        Stmt::Let(let_stmt) => detach_in_expr(language, &mut let_stmt.value, node_id).map(Some),
        Stmt::Expr(expr_stmt) => detach_in_expr(language, &mut expr_stmt.expr, node_id).map(Some),
        Stmt::Item(item) => recurse_item_for_detach(language, item, node_id),
        Stmt::Doc(_) | Stmt::Comment(_) => Ok(None),
    }
}

fn detach_in_expr(
    language: LowerLanguage,
    expr: &mut Expr,
    node_id: &str,
) -> Result<bool, PatchError> {
    match expr {
        Expr::Group(group) => detach_in_expr(language, &mut group.expr, node_id),
        Expr::Binary(binary) => {
            if detach_in_expr(language, &mut binary.lhs, node_id)? {
                return Ok(true);
            }
            detach_in_expr(language, &mut binary.rhs, node_id)
        }
        Expr::Unary(unary) => detach_in_expr(language, &mut unary.expr, node_id),
        Expr::Call(call) => {
            if detach_in_expr(language, &mut call.callee, node_id)? {
                return Ok(true);
            }
            for arg in &mut call.args {
                if detach_in_expr(language, arg, node_id)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Match(match_expr) => {
            if detach_in_expr(language, &mut match_expr.scrutinee, node_id)? {
                return Ok(true);
            }
            for arm in &mut match_expr.arms {
                if let Some(guard) = &mut arm.guard {
                    if detach_in_expr(language, guard, node_id)? {
                        return Ok(true);
                    }
                }
                if detach_in_expr(language, &mut arm.body, node_id)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Block(Block { stmts, .. }) => detach_in_stmts(language, stmts, node_id),
        Expr::Path(_) | Expr::Lit(_) => Ok(false),
    }
}

fn has_following_semantic_item(items: &[Item], index: usize) -> bool {
    items[index + 1..].iter().any(|item| !is_item_trivia(item))
}

fn has_following_semantic_stmt(stmts: &[Stmt], index: usize) -> bool {
    stmts[index + 1..].iter().any(|stmt| !is_stmt_trivia(stmt))
}
