use super::support::stmt_id;
use crate::error::{patch_error, PatchError};
use draxl_ast::{Block, Expr, Field, File, Item, MatchArm, Param, Stmt, Variant};

pub(super) fn apply_delete(file: &mut File, target_id: &str) -> Result<(), PatchError> {
    if delete_in_items(&mut file.items, target_id)? {
        Ok(())
    } else {
        Err(patch_error(&format!(
            "delete target `{}` was not found in ranked slots",
            target_id
        )))
    }
}

fn delete_in_items(items: &mut Vec<Item>, target_id: &str) -> Result<bool, PatchError> {
    if delete_item_vec(items, target_id) {
        return Ok(true);
    }
    for item in items {
        if delete_in_item(item, target_id)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn delete_in_item(item: &mut Item, target_id: &str) -> Result<bool, PatchError> {
    match item {
        Item::Mod(module) => delete_in_items(&mut module.items, target_id),
        Item::Struct(strukt) => Ok(delete_field_vec(&mut strukt.fields, target_id)),
        Item::Enum(enm) => Ok(delete_variant_vec(&mut enm.variants, target_id)),
        Item::Fn(function) => {
            if delete_param_vec(&mut function.params, target_id) {
                return Ok(true);
            }
            delete_in_block(&mut function.body, target_id)
        }
        Item::Use(_) | Item::Doc(_) | Item::Comment(_) => Ok(false),
    }
}

fn delete_in_block(block: &mut Block, target_id: &str) -> Result<bool, PatchError> {
    if delete_stmt_vec(&mut block.stmts, target_id) {
        return Ok(true);
    }
    for stmt in &mut block.stmts {
        if delete_in_stmt(stmt, target_id)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn delete_in_stmt(stmt: &mut Stmt, target_id: &str) -> Result<bool, PatchError> {
    match stmt {
        Stmt::Let(let_stmt) => delete_in_expr(&mut let_stmt.value, target_id),
        Stmt::Expr(expr_stmt) => delete_in_expr(&mut expr_stmt.expr, target_id),
        Stmt::Item(item) => delete_in_item(item, target_id),
        Stmt::Doc(_) | Stmt::Comment(_) => Ok(false),
    }
}

fn delete_in_expr(expr: &mut Expr, target_id: &str) -> Result<bool, PatchError> {
    match expr {
        Expr::Group(group) => delete_in_expr(&mut group.expr, target_id),
        Expr::Binary(binary) => {
            if delete_in_expr(&mut binary.lhs, target_id)? {
                return Ok(true);
            }
            delete_in_expr(&mut binary.rhs, target_id)
        }
        Expr::Unary(unary) => delete_in_expr(&mut unary.expr, target_id),
        Expr::Call(call) => {
            if delete_in_expr(&mut call.callee, target_id)? {
                return Ok(true);
            }
            for arg in &mut call.args {
                if delete_in_expr(arg, target_id)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Match(match_expr) => {
            if delete_match_arm_vec(&mut match_expr.arms, target_id) {
                return Ok(true);
            }
            if delete_in_expr(&mut match_expr.scrutinee, target_id)? {
                return Ok(true);
            }
            for arm in &mut match_expr.arms {
                if let Some(guard) = &mut arm.guard {
                    if delete_in_expr(guard, target_id)? {
                        return Ok(true);
                    }
                }
                if delete_in_expr(&mut arm.body, target_id)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Block(block) => delete_in_block(block, target_id),
        Expr::Path(_) | Expr::Lit(_) => Ok(false),
    }
}

fn delete_item_vec(items: &mut Vec<Item>, target_id: &str) -> bool {
    if let Some(index) = items.iter().position(|item| item.meta().id == target_id) {
        items.remove(index);
        true
    } else {
        false
    }
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
    if let Some(index) = stmts
        .iter()
        .position(|stmt| stmt_id(stmt).is_some_and(|id| id == target_id))
    {
        stmts.remove(index);
        true
    } else {
        false
    }
}

fn delete_match_arm_vec(arms: &mut Vec<MatchArm>, target_id: &str) -> bool {
    if let Some(index) = arms.iter().position(|arm| arm.meta.id == target_id) {
        arms.remove(index);
        true
    } else {
        false
    }
}
