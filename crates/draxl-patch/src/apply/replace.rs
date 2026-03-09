use super::support::{
    apply_shell_to_expr, apply_shell_to_field, apply_shell_to_item, apply_shell_to_match_arm,
    apply_shell_to_param, apply_shell_to_pattern, apply_shell_to_stmt, apply_shell_to_type,
    apply_shell_to_variant, expect_expr, expect_field, expect_item, expect_match_arm, expect_param,
    expect_pattern, expect_stmt, expect_type, expect_variant, expr_id, patch_node_kind,
    require_replace_fragment, stmt_id,
};
use crate::error::{patch_error, PatchError};
use crate::model::PatchNode;
use draxl_ast::{Block, Expr, Field, File, Item, MatchArm, Param, Pattern, Stmt, Type, Variant};

pub(super) fn apply_replace(
    file: &mut File,
    target_id: &str,
    replacement: PatchNode,
) -> Result<(), PatchError> {
    require_replace_fragment(&replacement, target_id)?;

    let mut replacement = Some(replacement);
    if replace_in_items(&mut file.items, target_id, &mut replacement)? {
        Ok(())
    } else {
        Err(patch_error(&format!(
            "replace target `{target_id}` was not found"
        )))
    }
}

fn replace_in_items(
    items: &mut Vec<Item>,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if replace_item_vec(items, target_id, replacement)? {
        return Ok(true);
    }

    for item in items {
        if replace_in_item(item, target_id, replacement)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn replace_in_item(
    item: &mut Item,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    match item {
        Item::Mod(module) => replace_in_items(&mut module.items, target_id, replacement),
        Item::Struct(strukt) => {
            if replace_field_vec(&mut strukt.fields, target_id, replacement)? {
                return Ok(true);
            }
            for field in &mut strukt.fields {
                if replace_in_type(&mut field.ty, target_id, replacement)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Item::Enum(enm) => replace_variant_vec(&mut enm.variants, target_id, replacement),
        Item::Fn(function) => {
            if replace_param_vec(&mut function.params, target_id, replacement)? {
                return Ok(true);
            }
            for param in &mut function.params {
                if replace_in_type(&mut param.ty, target_id, replacement)? {
                    return Ok(true);
                }
            }
            if let Some(ret_ty) = &mut function.ret_ty {
                if replace_in_type(ret_ty, target_id, replacement)? {
                    return Ok(true);
                }
            }
            replace_in_block(&mut function.body, target_id, replacement)
        }
        Item::Use(_) | Item::Doc(_) | Item::Comment(_) => Ok(false),
    }
}

fn replace_in_block(
    block: &mut Block,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if replace_stmt_vec(&mut block.stmts, target_id, replacement)? {
        return Ok(true);
    }
    for stmt in &mut block.stmts {
        if replace_in_stmt(stmt, target_id, replacement)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn replace_in_stmt(
    stmt: &mut Stmt,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    match stmt {
        Stmt::Let(let_stmt) => {
            if replace_in_pattern(&mut let_stmt.pat, target_id, replacement)? {
                return Ok(true);
            }
            replace_in_expr(&mut let_stmt.value, target_id, replacement)
        }
        Stmt::Expr(expr_stmt) => replace_in_expr(&mut expr_stmt.expr, target_id, replacement),
        Stmt::Item(item) => replace_in_item(item, target_id, replacement),
        Stmt::Doc(_) | Stmt::Comment(_) => Ok(false),
    }
}

fn replace_in_expr(
    expr: &mut Expr,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if expr_id(expr).is_some_and(|id| id == target_id) {
        let shell = expr
            .meta()
            .cloned()
            .ok_or_else(|| patch_error("replace target expression is missing metadata"))?;
        let mut replacement_expr =
            expect_expr(replacement.take(), shell.slot.as_deref().unwrap_or(""))?;
        apply_shell_to_expr(&mut replacement_expr, &shell);
        *expr = replacement_expr;
        return Ok(true);
    }

    match expr {
        Expr::Group(group) => replace_in_expr(&mut group.expr, target_id, replacement),
        Expr::Binary(binary) => {
            if replace_in_expr(&mut binary.lhs, target_id, replacement)? {
                return Ok(true);
            }
            replace_in_expr(&mut binary.rhs, target_id, replacement)
        }
        Expr::Unary(unary) => replace_in_expr(&mut unary.expr, target_id, replacement),
        Expr::Call(call) => {
            if replace_in_expr(&mut call.callee, target_id, replacement)? {
                return Ok(true);
            }
            for arg in &mut call.args {
                if replace_in_expr(arg, target_id, replacement)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Match(match_expr) => {
            if replace_match_arm_vec(&mut match_expr.arms, target_id, replacement)? {
                return Ok(true);
            }
            if replace_in_expr(&mut match_expr.scrutinee, target_id, replacement)? {
                return Ok(true);
            }
            for arm in &mut match_expr.arms {
                if replace_in_match_arm(arm, target_id, replacement)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Block(block) => replace_in_block(block, target_id, replacement),
        Expr::Path(_) | Expr::Lit(_) => Ok(false),
    }
}

fn replace_in_match_arm(
    arm: &mut MatchArm,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if replace_in_pattern(&mut arm.pat, target_id, replacement)? {
        return Ok(true);
    }
    if let Some(guard) = &mut arm.guard {
        if replace_in_expr(guard, target_id, replacement)? {
            return Ok(true);
        }
    }
    replace_in_expr(&mut arm.body, target_id, replacement)
}

fn replace_in_pattern(
    pattern: &mut Pattern,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if super::support::pattern_id(pattern).is_some_and(|id| id == target_id) {
        let shell = pattern
            .meta()
            .cloned()
            .ok_or_else(|| patch_error("replace target pattern is missing metadata"))?;
        let mut replacement_pattern =
            expect_pattern(replacement.take(), shell.slot.as_deref().unwrap_or(""))?;
        apply_shell_to_pattern(&mut replacement_pattern, &shell);
        *pattern = replacement_pattern;
        return Ok(true);
    }
    Ok(false)
}

fn replace_in_type(
    ty: &mut Type,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if ty.meta().id == target_id {
        let shell = ty.meta().clone();
        let mut replacement_ty =
            expect_type(replacement.take(), shell.slot.as_deref().unwrap_or(""))?;
        apply_shell_to_type(&mut replacement_ty, &shell);
        *ty = replacement_ty;
        return Ok(true);
    }
    Ok(false)
}

fn replace_item_vec(
    items: &mut Vec<Item>,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    let Some(index) = items.iter().position(|item| item.meta().id == target_id) else {
        return Ok(false);
    };

    let shell = items[index].meta().clone();
    match &items[index] {
        Item::Doc(_) => {
            let slot = shell.slot.as_deref().unwrap_or("");
            let node = replacement
                .take()
                .ok_or_else(|| patch_error("patch node was consumed before use"))?;
            let mut item = match node {
                PatchNode::Doc(doc) => Item::Doc(doc),
                PatchNode::Comment(comment) => Item::Comment(comment),
                other => {
                    return Err(patch_error(&format!(
                        "replace target `{target_id}` expects an attachment fragment, found {}",
                        patch_node_kind(&other)
                    )))
                }
            };
            apply_shell_to_item(&mut item, &shell);
            items[index] = item;
            let _ = slot;
        }
        Item::Comment(_) => {
            let node = replacement
                .take()
                .ok_or_else(|| patch_error("patch node was consumed before use"))?;
            let mut item = match node {
                PatchNode::Comment(comment) => Item::Comment(comment),
                PatchNode::Doc(doc) => Item::Doc(doc),
                other => {
                    return Err(patch_error(&format!(
                        "replace target `{target_id}` expects an attachment fragment, found {}",
                        patch_node_kind(&other)
                    )))
                }
            };
            apply_shell_to_item(&mut item, &shell);
            items[index] = item;
        }
        _ => {
            let mut item = expect_item(replacement.take(), shell.slot.as_deref().unwrap_or(""))?;
            apply_shell_to_item(&mut item, &shell);
            items[index] = item;
        }
    }

    Ok(true)
}

fn replace_field_vec(
    fields: &mut Vec<Field>,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    let Some(index) = fields.iter().position(|field| field.meta.id == target_id) else {
        return Ok(false);
    };
    let shell = fields[index].meta.clone();
    let mut field = expect_field(replacement.take(), shell.slot.as_deref().unwrap_or(""))?;
    apply_shell_to_field(&mut field, &shell);
    fields[index] = field;
    Ok(true)
}

fn replace_variant_vec(
    variants: &mut Vec<Variant>,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    let Some(index) = variants
        .iter()
        .position(|variant| variant.meta.id == target_id)
    else {
        return Ok(false);
    };
    let shell = variants[index].meta.clone();
    let mut variant = expect_variant(replacement.take(), shell.slot.as_deref().unwrap_or(""))?;
    apply_shell_to_variant(&mut variant, &shell);
    variants[index] = variant;
    Ok(true)
}

fn replace_param_vec(
    params: &mut Vec<Param>,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    let Some(index) = params.iter().position(|param| param.meta.id == target_id) else {
        return Ok(false);
    };
    let shell = params[index].meta.clone();
    let mut param = expect_param(replacement.take(), shell.slot.as_deref().unwrap_or(""))?;
    apply_shell_to_param(&mut param, &shell);
    params[index] = param;
    Ok(true)
}

fn replace_stmt_vec(
    stmts: &mut Vec<Stmt>,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    let Some(index) = stmts
        .iter()
        .position(|stmt| stmt_id(stmt).is_some_and(|id| id == target_id))
    else {
        return Ok(false);
    };

    let shell = stmts[index]
        .meta()
        .cloned()
        .ok_or_else(|| patch_error("replace target statement is missing metadata"))?;
    match &stmts[index] {
        Stmt::Doc(_) => {
            let node = replacement
                .take()
                .ok_or_else(|| patch_error("patch node was consumed before use"))?;
            let mut stmt = match node {
                PatchNode::Doc(doc) => Stmt::Doc(doc),
                PatchNode::Comment(comment) => Stmt::Comment(comment),
                other => {
                    return Err(patch_error(&format!(
                        "replace target `{target_id}` expects an attachment fragment, found {}",
                        patch_node_kind(&other)
                    )))
                }
            };
            apply_shell_to_stmt(&mut stmt, &shell);
            stmts[index] = stmt;
        }
        Stmt::Comment(_) => {
            let node = replacement
                .take()
                .ok_or_else(|| patch_error("patch node was consumed before use"))?;
            let mut stmt = match node {
                PatchNode::Comment(comment) => Stmt::Comment(comment),
                PatchNode::Doc(doc) => Stmt::Doc(doc),
                other => {
                    return Err(patch_error(&format!(
                        "replace target `{target_id}` expects an attachment fragment, found {}",
                        patch_node_kind(&other)
                    )))
                }
            };
            apply_shell_to_stmt(&mut stmt, &shell);
            stmts[index] = stmt;
        }
        _ => {
            let mut stmt = expect_stmt(replacement.take(), shell.slot.as_deref().unwrap_or(""))?;
            apply_shell_to_stmt(&mut stmt, &shell);
            stmts[index] = stmt;
        }
    }

    Ok(true)
}

fn replace_match_arm_vec(
    arms: &mut Vec<MatchArm>,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    let Some(index) = arms.iter().position(|arm| arm.meta.id == target_id) else {
        return Ok(false);
    };
    let shell = arms[index].meta.clone();
    let mut arm = expect_match_arm(replacement.take(), shell.slot.as_deref().unwrap_or(""))?;
    apply_shell_to_match_arm(&mut arm, &shell);
    arms[index] = arm;
    Ok(true)
}
