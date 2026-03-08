use super::support::{
    assign_item_slot_and_rank, assign_meta_slot_and_rank, assign_stmt_slot_and_rank, expect_field,
    expect_item, expect_match_arm, expect_param, expect_stmt, expect_variant, meta_slot_rank,
    stmt_id, stmt_slot_rank,
};
use crate::error::{patch_error, PatchError};
use crate::model::PatchNode;
use draxl_ast::{Block, Expr, Field, File, Item, MatchArm, Param, Stmt, Variant};

pub(super) fn apply_replace(
    file: &mut File,
    target_id: &str,
    replacement: PatchNode,
) -> Result<(), PatchError> {
    let mut replacement = Some(replacement);
    if replace_in_items(&mut file.items, target_id, &mut replacement)? {
        Ok(())
    } else {
        Err(patch_error(&format!(
            "replace target `{}` was not found in ranked slots",
            target_id
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
        Item::Struct(strukt) => replace_field_vec(&mut strukt.fields, target_id, replacement),
        Item::Enum(enm) => replace_variant_vec(&mut enm.variants, target_id, replacement),
        Item::Fn(function) => {
            if replace_param_vec(&mut function.params, target_id, replacement)? {
                return Ok(true);
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
        Stmt::Let(let_stmt) => replace_in_expr(&mut let_stmt.value, target_id, replacement),
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
                if let Some(guard) = &mut arm.guard {
                    if replace_in_expr(guard, target_id, replacement)? {
                        return Ok(true);
                    }
                }
                if replace_in_expr(&mut arm.body, target_id, replacement)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Block(block) => replace_in_block(block, target_id, replacement),
        Expr::Path(_) | Expr::Lit(_) => Ok(false),
    }
}

fn replace_item_vec(
    items: &mut Vec<Item>,
    target_id: &str,
    replacement: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    let Some(index) = items.iter().position(|item| item.meta().id == target_id) else {
        return Ok(false);
    };
    let (slot, rank) = meta_slot_rank(items[index].meta());
    let mut item = expect_item(replacement.take(), slot)?;
    assign_item_slot_and_rank(&mut item, slot, rank.as_deref(), false)?;
    items[index] = item;
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
    let (slot, rank) = meta_slot_rank(&fields[index].meta);
    let mut field = expect_field(replacement.take(), slot)?;
    assign_meta_slot_and_rank(&mut field.meta, slot, rank.as_deref(), false);
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
    let (slot, rank) = meta_slot_rank(&variants[index].meta);
    let mut variant = expect_variant(replacement.take(), slot)?;
    assign_meta_slot_and_rank(&mut variant.meta, slot, rank.as_deref(), false);
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
    let (slot, rank) = meta_slot_rank(&params[index].meta);
    let mut param = expect_param(replacement.take(), slot)?;
    assign_meta_slot_and_rank(&mut param.meta, slot, rank.as_deref(), false);
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
    let (slot, rank) = stmt_slot_rank(&stmts[index])?;
    let mut stmt = expect_stmt(replacement.take(), slot)?;
    assign_stmt_slot_and_rank(&mut stmt, slot, rank.as_deref(), false)?;
    stmts[index] = stmt;
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
    let (slot, rank) = meta_slot_rank(&arms[index].meta);
    let mut arm = expect_match_arm(replacement.take(), slot)?;
    assign_meta_slot_and_rank(&mut arm.meta, slot, rank.as_deref(), false);
    arms[index] = arm;
    Ok(true)
}
