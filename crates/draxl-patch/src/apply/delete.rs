use super::support::{
    expr_id, is_item_trivia, is_stmt_trivia, pattern_id, resolved_item_attachment_targets,
    resolved_stmt_attachment_targets, stmt_id,
};
use crate::error::{patch_error, PatchError};
use crate::schema::{
    removable_slot_spec, required_slot_error_message, unsupported_slot_error_message, NodeKind,
};
use draxl_ast::{Block, Expr, Field, File, Item, LowerLanguage, MatchArm, Param, Stmt, Variant};

pub(super) fn apply_delete(
    language: LowerLanguage,
    file: &mut File,
    target_id: &str,
) -> Result<(), PatchError> {
    if delete_in_items(language, &mut file.items, target_id)? {
        Ok(())
    } else {
        Err(patch_error(&format!(
            "delete target `{target_id}` was not found"
        )))
    }
}

fn delete_in_items(
    language: LowerLanguage,
    items: &mut Vec<Item>,
    target_id: &str,
) -> Result<bool, PatchError> {
    if delete_item_vec(items, target_id) {
        return Ok(true);
    }
    for item in items {
        if delete_in_item(language, item, target_id)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn delete_in_item(
    language: LowerLanguage,
    item: &mut Item,
    target_id: &str,
) -> Result<bool, PatchError> {
    match item {
        Item::Mod(module) => delete_in_items(language, &mut module.items, target_id),
        Item::Struct(strukt) => {
            if delete_field_vec(&mut strukt.fields, target_id) {
                return Ok(true);
            }
            for field in &mut strukt.fields {
                if field.ty.meta().id == target_id {
                    return removable_child_error(
                        language,
                        "delete",
                        target_id,
                        NodeKind::Field,
                        "ty",
                    );
                }
            }
            Ok(false)
        }
        Item::Enum(enm) => Ok(delete_variant_vec(&mut enm.variants, target_id)),
        Item::Fn(function) => {
            if delete_param_vec(&mut function.params, target_id) {
                return Ok(true);
            }
            for param in &mut function.params {
                if param.ty.meta().id == target_id {
                    return removable_child_error(
                        language,
                        "delete",
                        target_id,
                        NodeKind::Param,
                        "ty",
                    );
                }
            }
            if function
                .ret_ty
                .as_ref()
                .is_some_and(|ret_ty| ret_ty.meta().id == target_id)
            {
                removable_child_error(language, "delete", target_id, NodeKind::Fn, "ret")?;
                function.ret_ty = None;
                return Ok(true);
            }
            delete_in_block(language, &mut function.body, target_id)
        }
        Item::Use(_) | Item::Doc(_) | Item::Comment(_) => Ok(false),
    }
}

fn delete_in_block(
    language: LowerLanguage,
    block: &mut Block,
    target_id: &str,
) -> Result<bool, PatchError> {
    if delete_stmt_vec(&mut block.stmts, target_id) {
        return Ok(true);
    }
    for stmt in &mut block.stmts {
        if delete_in_stmt(language, stmt, target_id)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn delete_in_stmt(
    language: LowerLanguage,
    stmt: &mut Stmt,
    target_id: &str,
) -> Result<bool, PatchError> {
    match stmt {
        Stmt::Let(let_stmt) => {
            if pattern_id(&let_stmt.pat).is_some_and(|id| id == target_id) {
                return removable_child_error(
                    language,
                    "delete",
                    target_id,
                    NodeKind::LetStmt,
                    "pat",
                );
            }
            if expr_id(&let_stmt.value).is_some_and(|id| id == target_id) {
                return removable_child_error(
                    language,
                    "delete",
                    target_id,
                    NodeKind::LetStmt,
                    "init",
                );
            }
            delete_in_expr(language, &mut let_stmt.value, target_id)
        }
        Stmt::Expr(expr_stmt) => {
            if expr_id(&expr_stmt.expr).is_some_and(|id| id == target_id) {
                return removable_child_error(
                    language,
                    "delete",
                    target_id,
                    NodeKind::ExprStmt,
                    "expr",
                );
            }
            delete_in_expr(language, &mut expr_stmt.expr, target_id)
        }
        Stmt::Item(item) => delete_in_item(language, item, target_id),
        Stmt::Doc(_) | Stmt::Comment(_) => Ok(false),
    }
}

fn delete_in_expr(
    language: LowerLanguage,
    expr: &mut Expr,
    target_id: &str,
) -> Result<bool, PatchError> {
    match expr {
        Expr::Group(group) => {
            if expr_id(&group.expr).is_some_and(|id| id == target_id) {
                return removable_child_error(
                    language,
                    "delete",
                    target_id,
                    NodeKind::ExprGroup,
                    "expr",
                );
            }
            delete_in_expr(language, &mut group.expr, target_id)
        }
        Expr::Binary(binary) => {
            if expr_id(&binary.lhs).is_some_and(|id| id == target_id) {
                return removable_child_error(
                    language,
                    "delete",
                    target_id,
                    NodeKind::ExprBinary,
                    "lhs",
                );
            }
            if expr_id(&binary.rhs).is_some_and(|id| id == target_id) {
                return removable_child_error(
                    language,
                    "delete",
                    target_id,
                    NodeKind::ExprBinary,
                    "rhs",
                );
            }
            if delete_in_expr(language, &mut binary.lhs, target_id)? {
                return Ok(true);
            }
            delete_in_expr(language, &mut binary.rhs, target_id)
        }
        Expr::Unary(unary) => {
            if expr_id(&unary.expr).is_some_and(|id| id == target_id) {
                return removable_child_error(
                    language,
                    "delete",
                    target_id,
                    NodeKind::ExprUnary,
                    "expr",
                );
            }
            delete_in_expr(language, &mut unary.expr, target_id)
        }
        Expr::Call(call) => {
            if expr_id(&call.callee).is_some_and(|id| id == target_id) {
                return removable_child_error(
                    language,
                    "delete",
                    target_id,
                    NodeKind::ExprCall,
                    "callee",
                );
            }
            if delete_in_expr(language, &mut call.callee, target_id)? {
                return Ok(true);
            }
            for arg in &mut call.args {
                if expr_id(arg).is_some_and(|id| id == target_id) {
                    return removable_child_error(
                        language,
                        "delete",
                        target_id,
                        NodeKind::ExprCall,
                        "args",
                    );
                }
                if delete_in_expr(language, arg, target_id)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Match(match_expr) => {
            if delete_match_arm_vec(&mut match_expr.arms, target_id) {
                return Ok(true);
            }
            if expr_id(&match_expr.scrutinee).is_some_and(|id| id == target_id) {
                return removable_child_error(
                    language,
                    "delete",
                    target_id,
                    NodeKind::ExprMatch,
                    "scrutinee",
                );
            }
            if delete_in_expr(language, &mut match_expr.scrutinee, target_id)? {
                return Ok(true);
            }
            for arm in &mut match_expr.arms {
                if delete_in_match_arm(language, arm, target_id)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Block(block) => delete_in_block(language, block, target_id),
        Expr::Path(_) | Expr::Lit(_) => Ok(false),
    }
}

fn delete_in_match_arm(
    language: LowerLanguage,
    arm: &mut MatchArm,
    target_id: &str,
) -> Result<bool, PatchError> {
    if pattern_id(&arm.pat).is_some_and(|id| id == target_id) {
        return removable_child_error(language, "delete", target_id, NodeKind::MatchArm, "pat");
    }
    if arm
        .guard
        .as_ref()
        .is_some_and(|guard| expr_id(guard).is_some_and(|id| id == target_id))
    {
        removable_child_error(language, "delete", target_id, NodeKind::MatchArm, "guard")?;
        arm.guard = None;
        return Ok(true);
    }
    if let Some(guard) = &mut arm.guard {
        if delete_in_expr(language, guard, target_id)? {
            return Ok(true);
        }
    }
    if expr_id(&arm.body).is_some_and(|id| id == target_id) {
        return removable_child_error(language, "delete", target_id, NodeKind::MatchArm, "body");
    }
    delete_in_expr(language, &mut arm.body, target_id)
}

fn delete_item_vec(items: &mut Vec<Item>, target_id: &str) -> bool {
    let Some(index) = items.iter().position(|item| item.meta().id == target_id) else {
        return false;
    };

    let attachment_targets = resolved_item_attachment_targets(items);
    let remove_closure = !is_item_trivia(&items[index]);
    let mut retained = Vec::with_capacity(items.len());
    for (current, item) in std::mem::take(items).into_iter().enumerate() {
        let remove = current == index
            || (remove_closure
                && is_item_trivia(&item)
                && attachment_targets[current].as_deref() == Some(target_id));
        if !remove {
            retained.push(item);
        }
    }
    *items = retained;
    true
}

fn delete_field_vec(fields: &mut Vec<Field>, target_id: &str) -> bool {
    if let Some(index) = fields.iter().position(|field| field.meta.id == target_id) {
        fields.remove(index);
        true
    } else {
        false
    }
}

fn delete_variant_vec(variants: &mut Vec<Variant>, target_id: &str) -> bool {
    if let Some(index) = variants
        .iter()
        .position(|variant| variant.meta.id == target_id)
    {
        variants.remove(index);
        true
    } else {
        false
    }
}

fn delete_param_vec(params: &mut Vec<Param>, target_id: &str) -> bool {
    if let Some(index) = params.iter().position(|param| param.meta.id == target_id) {
        params.remove(index);
        true
    } else {
        false
    }
}

fn delete_stmt_vec(stmts: &mut Vec<Stmt>, target_id: &str) -> bool {
    let Some(index) = stmts
        .iter()
        .position(|stmt| stmt_id(stmt).is_some_and(|id| id == target_id))
    else {
        return false;
    };

    let attachment_targets = resolved_stmt_attachment_targets(stmts);
    let remove_closure = !is_stmt_trivia(&stmts[index]);
    let mut retained = Vec::with_capacity(stmts.len());
    for (current, stmt) in std::mem::take(stmts).into_iter().enumerate() {
        let remove = current == index
            || (remove_closure
                && is_stmt_trivia(&stmt)
                && attachment_targets[current].as_deref() == Some(target_id));
        if !remove {
            retained.push(stmt);
        }
    }
    *stmts = retained;
    true
}

fn delete_match_arm_vec(arms: &mut Vec<MatchArm>, target_id: &str) -> bool {
    if let Some(index) = arms.iter().position(|arm| arm.meta.id == target_id) {
        arms.remove(index);
        true
    } else {
        false
    }
}

fn removable_child_error(
    language: LowerLanguage,
    action: &str,
    target_id: &str,
    owner_kind: NodeKind,
    slot: &str,
) -> Result<bool, PatchError> {
    match removable_slot_spec(language, owner_kind, slot) {
        Some(_) => Ok(true),
        None => {
            let message = if crate::schema::slot_spec(language, owner_kind, slot).is_some() {
                required_slot_error_message(language, action, target_id, slot)
            } else {
                unsupported_slot_error_message(language, action, target_id, slot)
            };
            Err(patch_error(&message))
        }
    }
}
