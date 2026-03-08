use super::support::{
    assign_item_slot_and_rank, assign_meta_slot_and_rank, assign_stmt_slot_and_rank, expect_field,
    expect_item, expect_match_arm, expect_param, expect_stmt, expect_variant,
};
use crate::error::{patch_error, PatchError};
use crate::model::{PatchNode, PatchParent};
use draxl_ast::{Block, Expr, File, Item, Stmt};

pub(super) fn apply_insert(
    file: &mut File,
    parent: PatchParent,
    slot: &str,
    rank: &str,
    node: PatchNode,
) -> Result<(), PatchError> {
    let parent_label = match &parent {
        PatchParent::File => "file".to_owned(),
        PatchParent::Node { id } => id.clone(),
    };
    let mut node = Some(node);
    let found = match parent {
        PatchParent::File => {
            insert_into_file_slot(&mut file.items, slot, rank, &mut node)?;
            true
        }
        PatchParent::Node { id } => {
            let mut found = false;
            for item in &mut file.items {
                if insert_into_item(item, &id, slot, rank, &mut node)? {
                    found = true;
                    break;
                }
            }
            found
        }
    };

    if !found {
        return Err(patch_error(&format!(
            "insert target for parent `{}` and slot `{}` was not found",
            parent_label, slot
        )));
    }

    Ok(())
}

fn insert_into_file_slot(
    items: &mut Vec<Item>,
    slot: &str,
    rank: &str,
    node: &mut Option<PatchNode>,
) -> Result<(), PatchError> {
    if slot != "file_items" {
        return Err(patch_error(&format!(
            "slot `{}` is invalid for the file root, expected `file_items`",
            slot
        )));
    }
    let mut item = expect_item(node.take(), slot)?;
    assign_item_slot_and_rank(&mut item, slot, Some(rank), true)?;
    items.push(item);
    Ok(())
}

fn insert_into_item(
    item: &mut Item,
    parent_id: &str,
    slot: &str,
    rank: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if item.meta().id == parent_id {
        match item {
            Item::Mod(module) if slot == "items" => {
                let mut child = expect_item(node.take(), slot)?;
                assign_item_slot_and_rank(&mut child, slot, Some(rank), true)?;
                module.items.push(child);
                return Ok(true);
            }
            Item::Struct(strukt) if slot == "fields" => {
                let mut field = expect_field(node.take(), slot)?;
                assign_meta_slot_and_rank(&mut field.meta, slot, Some(rank), true);
                strukt.fields.push(field);
                return Ok(true);
            }
            Item::Enum(enm) if slot == "variants" => {
                let mut variant = expect_variant(node.take(), slot)?;
                assign_meta_slot_and_rank(&mut variant.meta, slot, Some(rank), true);
                enm.variants.push(variant);
                return Ok(true);
            }
            Item::Fn(function) if slot == "params" => {
                let mut param = expect_param(node.take(), slot)?;
                assign_meta_slot_and_rank(&mut param.meta, slot, Some(rank), true);
                function.params.push(param);
                return Ok(true);
            }
            Item::Fn(function) if slot == "body" => {
                let mut stmt = expect_stmt(node.take(), slot)?;
                assign_stmt_slot_and_rank(&mut stmt, slot, Some(rank), true)?;
                function.body.stmts.push(stmt);
                return Ok(true);
            }
            _ => {
                return Err(patch_error(&format!(
                    "slot `{}` is not available on node `{}`",
                    slot, parent_id
                )));
            }
        }
    }

    match item {
        Item::Mod(module) => {
            for child in &mut module.items {
                if insert_into_item(child, parent_id, slot, rank, node)? {
                    return Ok(true);
                }
            }
        }
        Item::Fn(function) => {
            if insert_into_block(&mut function.body, parent_id, slot, rank, node)? {
                return Ok(true);
            }
        }
        Item::Use(_) | Item::Struct(_) | Item::Enum(_) | Item::Doc(_) | Item::Comment(_) => {}
    }

    Ok(false)
}

fn insert_into_block(
    block: &mut Block,
    parent_id: &str,
    slot: &str,
    rank: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if let Some(meta) = &block.meta {
        if meta.id == parent_id {
            if slot != "body" {
                return Err(patch_error(&format!(
                    "slot `{}` is not available on node `{}`",
                    slot, parent_id
                )));
            }
            let mut stmt = expect_stmt(node.take(), slot)?;
            assign_stmt_slot_and_rank(&mut stmt, slot, Some(rank), true)?;
            block.stmts.push(stmt);
            return Ok(true);
        }
    }

    for stmt in &mut block.stmts {
        if insert_into_stmt(stmt, parent_id, slot, rank, node)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn insert_into_stmt(
    stmt: &mut Stmt,
    parent_id: &str,
    slot: &str,
    rank: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    match stmt {
        Stmt::Let(let_stmt) => insert_into_expr(&mut let_stmt.value, parent_id, slot, rank, node),
        Stmt::Expr(expr_stmt) => insert_into_expr(&mut expr_stmt.expr, parent_id, slot, rank, node),
        Stmt::Item(item) => insert_into_item(item, parent_id, slot, rank, node),
        Stmt::Doc(_) | Stmt::Comment(_) => Ok(false),
    }
}

fn insert_into_expr(
    expr: &mut Expr,
    parent_id: &str,
    slot: &str,
    rank: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if let Some(meta) = expr.meta() {
        if meta.id == parent_id {
            match expr {
                Expr::Match(match_expr) if slot == "arms" => {
                    let mut arm = expect_match_arm(node.take(), slot)?;
                    assign_meta_slot_and_rank(&mut arm.meta, slot, Some(rank), true);
                    match_expr.arms.push(arm);
                    return Ok(true);
                }
                _ => {
                    return Err(patch_error(&format!(
                        "slot `{}` is not available on node `{}`",
                        slot, parent_id
                    )));
                }
            }
        }
    }

    match expr {
        Expr::Group(group) => insert_into_expr(&mut group.expr, parent_id, slot, rank, node),
        Expr::Binary(binary) => {
            if insert_into_expr(&mut binary.lhs, parent_id, slot, rank, node)? {
                return Ok(true);
            }
            insert_into_expr(&mut binary.rhs, parent_id, slot, rank, node)
        }
        Expr::Unary(unary) => insert_into_expr(&mut unary.expr, parent_id, slot, rank, node),
        Expr::Call(call) => {
            if insert_into_expr(&mut call.callee, parent_id, slot, rank, node)? {
                return Ok(true);
            }
            for arg in &mut call.args {
                if insert_into_expr(arg, parent_id, slot, rank, node)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Match(match_expr) => {
            if insert_into_expr(&mut match_expr.scrutinee, parent_id, slot, rank, node)? {
                return Ok(true);
            }
            for arm in &mut match_expr.arms {
                if let Some(guard) = &mut arm.guard {
                    if insert_into_expr(guard, parent_id, slot, rank, node)? {
                        return Ok(true);
                    }
                }
                if insert_into_expr(&mut arm.body, parent_id, slot, rank, node)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Block(block) => insert_into_block(block, parent_id, slot, rank, node),
        Expr::Path(_) | Expr::Lit(_) => Ok(false),
    }
}
