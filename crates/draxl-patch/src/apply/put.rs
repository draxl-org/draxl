use super::support::{
    assign_expr_slot_and_rank, assign_pattern_slot_and_rank, assign_type_slot_and_rank,
    expect_expr, expect_pattern, expect_type, require_put_fragment, slot_ref_label,
};
use crate::error::{patch_error, PatchError};
use crate::model::{PatchNode, SlotOwner, SlotRef};
use draxl_ast::{Block, Expr, File, Item, MatchArm, Stmt};

pub(super) fn apply_put(file: &mut File, slot: SlotRef, node: PatchNode) -> Result<(), PatchError> {
    require_put_fragment(&node)?;

    let mut node = Some(node);
    let found = match &slot.owner {
        SlotOwner::File => {
            return Err(patch_error("the file root has no single-child patch slots"));
        }
        SlotOwner::Node(id) => {
            let mut found = false;
            for item in &mut file.items {
                if put_in_item(item, id, &slot.slot, &mut node)? {
                    found = true;
                    break;
                }
            }
            found
        }
    };

    if !found {
        return Err(patch_error(&format!(
            "put destination `{}` was not found",
            slot_ref_label(&slot)
        )));
    }

    Ok(())
}

fn put_in_item(
    item: &mut Item,
    owner_id: &str,
    public_slot: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if item.meta().id == owner_id {
        match item {
            Item::Fn(function) if public_slot == "ret" => {
                let mut ty = expect_type(node.take(), public_slot)?;
                assign_type_slot_and_rank(&mut ty, "ret", None, true);
                function.ret_ty = Some(ty);
                return Ok(true);
            }
            Item::Mod(_) | Item::Use(_) | Item::Struct(_) | Item::Enum(_) | Item::Fn(_) => {
                return Err(patch_error(&format!(
                    "slot `@{owner_id}.{public_slot}` is not available for `put`"
                )));
            }
            Item::Doc(_) | Item::Comment(_) => {
                return Err(patch_error(
                    "doc and comment nodes do not own single-child patch slots",
                ));
            }
        }
    }

    match item {
        Item::Mod(module) => {
            for child in &mut module.items {
                if put_in_item(child, owner_id, public_slot, node)? {
                    return Ok(true);
                }
            }
        }
        Item::Struct(strukt) => {
            for field in &mut strukt.fields {
                if field.meta.id == owner_id {
                    if public_slot != "ty" {
                        return Err(patch_error(&format!(
                            "slot `@{owner_id}.{public_slot}` is not available for `put`"
                        )));
                    }
                    let mut ty = expect_type(node.take(), public_slot)?;
                    assign_type_slot_and_rank(&mut ty, "ty", None, true);
                    field.ty = ty;
                    return Ok(true);
                }
            }
        }
        Item::Fn(function) => {
            for param in &mut function.params {
                if param.meta.id == owner_id {
                    if public_slot != "ty" {
                        return Err(patch_error(&format!(
                            "slot `@{owner_id}.{public_slot}` is not available for `put`"
                        )));
                    }
                    let mut ty = expect_type(node.take(), public_slot)?;
                    assign_type_slot_and_rank(&mut ty, "ty", None, true);
                    param.ty = ty;
                    return Ok(true);
                }
            }
            if let Some(ret_ty) = &mut function.ret_ty {
                if ret_ty.meta().id == owner_id {
                    return Err(patch_error(
                        "use `replace` to preserve the existing return type identity",
                    ));
                }
            }
            if put_in_block(&mut function.body, owner_id, public_slot, node)? {
                return Ok(true);
            }
        }
        Item::Enum(_) | Item::Use(_) | Item::Doc(_) | Item::Comment(_) => {}
    }

    Ok(false)
}

fn put_in_block(
    block: &mut Block,
    owner_id: &str,
    public_slot: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    for stmt in &mut block.stmts {
        if put_in_stmt(stmt, owner_id, public_slot, node)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn put_in_stmt(
    stmt: &mut Stmt,
    owner_id: &str,
    public_slot: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if super::support::stmt_id(stmt).is_some_and(|id| id == owner_id) {
        match stmt {
            Stmt::Let(let_stmt) if public_slot == "pat" => {
                let mut pattern = expect_pattern(node.take(), public_slot)?;
                assign_pattern_slot_and_rank(&mut pattern, "pat", None, true);
                let_stmt.pat = pattern;
                return Ok(true);
            }
            Stmt::Let(let_stmt) if public_slot == "init" => {
                let mut expr = expect_expr(node.take(), public_slot)?;
                assign_expr_slot_and_rank(&mut expr, "init", None, true);
                let_stmt.value = expr;
                return Ok(true);
            }
            Stmt::Expr(expr_stmt) if public_slot == "expr" => {
                let mut expr = expect_expr(node.take(), public_slot)?;
                assign_expr_slot_and_rank(&mut expr, "expr", None, true);
                expr_stmt.expr = expr;
                return Ok(true);
            }
            Stmt::Item(_) | Stmt::Doc(_) | Stmt::Comment(_) | Stmt::Let(_) | Stmt::Expr(_) => {
                return Err(patch_error(&format!(
                    "slot `@{owner_id}.{public_slot}` is not available for `put`"
                )));
            }
        }
    }

    match stmt {
        Stmt::Let(let_stmt) => put_in_expr(&mut let_stmt.value, owner_id, public_slot, node),
        Stmt::Expr(expr_stmt) => put_in_expr(&mut expr_stmt.expr, owner_id, public_slot, node),
        Stmt::Item(item) => put_in_item(item, owner_id, public_slot, node),
        Stmt::Doc(_) | Stmt::Comment(_) => Ok(false),
    }
}

fn put_in_expr(
    expr: &mut Expr,
    owner_id: &str,
    public_slot: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if super::support::expr_id(expr).is_some_and(|id| id == owner_id) {
        match expr {
            Expr::Group(group) if public_slot == "expr" => {
                let mut inner = expect_expr(node.take(), public_slot)?;
                assign_expr_slot_and_rank(&mut inner, "expr", None, true);
                group.expr = Box::new(inner);
                return Ok(true);
            }
            Expr::Binary(binary) if public_slot == "lhs" => {
                let mut inner = expect_expr(node.take(), public_slot)?;
                assign_expr_slot_and_rank(&mut inner, "lhs", None, true);
                binary.lhs = Box::new(inner);
                return Ok(true);
            }
            Expr::Binary(binary) if public_slot == "rhs" => {
                let mut inner = expect_expr(node.take(), public_slot)?;
                assign_expr_slot_and_rank(&mut inner, "rhs", None, true);
                binary.rhs = Box::new(inner);
                return Ok(true);
            }
            Expr::Unary(unary) if public_slot == "expr" => {
                let mut inner = expect_expr(node.take(), public_slot)?;
                assign_expr_slot_and_rank(&mut inner, "expr", None, true);
                unary.expr = Box::new(inner);
                return Ok(true);
            }
            Expr::Call(call) if public_slot == "callee" => {
                let mut inner = expect_expr(node.take(), public_slot)?;
                assign_expr_slot_and_rank(&mut inner, "callee", None, true);
                call.callee = Box::new(inner);
                return Ok(true);
            }
            Expr::Match(match_expr) if public_slot == "scrutinee" => {
                let mut inner = expect_expr(node.take(), public_slot)?;
                assign_expr_slot_and_rank(&mut inner, "scrutinee", None, true);
                match_expr.scrutinee = Box::new(inner);
                return Ok(true);
            }
            Expr::Block(_)
            | Expr::Path(_)
            | Expr::Lit(_)
            | Expr::Group(_)
            | Expr::Binary(_)
            | Expr::Unary(_)
            | Expr::Call(_)
            | Expr::Match(_) => {
                return Err(patch_error(&format!(
                    "slot `@{owner_id}.{public_slot}` is not available for `put`"
                )));
            }
        }
    }

    match expr {
        Expr::Group(group) => put_in_expr(&mut group.expr, owner_id, public_slot, node),
        Expr::Binary(binary) => {
            if put_in_expr(&mut binary.lhs, owner_id, public_slot, node)? {
                return Ok(true);
            }
            put_in_expr(&mut binary.rhs, owner_id, public_slot, node)
        }
        Expr::Unary(unary) => put_in_expr(&mut unary.expr, owner_id, public_slot, node),
        Expr::Call(call) => {
            if put_in_expr(&mut call.callee, owner_id, public_slot, node)? {
                return Ok(true);
            }
            for arg in &mut call.args {
                if put_in_expr(arg, owner_id, public_slot, node)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Match(match_expr) => {
            if put_in_expr(&mut match_expr.scrutinee, owner_id, public_slot, node)? {
                return Ok(true);
            }
            for arm in &mut match_expr.arms {
                if put_in_match_arm(arm, owner_id, public_slot, node)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Block(block) => put_in_block(block, owner_id, public_slot, node),
        Expr::Path(_) | Expr::Lit(_) => Ok(false),
    }
}

fn put_in_match_arm(
    arm: &mut MatchArm,
    owner_id: &str,
    public_slot: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if arm.meta.id == owner_id {
        match public_slot {
            "pat" => {
                let mut pattern = expect_pattern(node.take(), public_slot)?;
                assign_pattern_slot_and_rank(&mut pattern, "pat", None, true);
                arm.pat = pattern;
                return Ok(true);
            }
            "guard" => {
                let mut expr = expect_expr(node.take(), public_slot)?;
                assign_expr_slot_and_rank(&mut expr, "guard", None, true);
                arm.guard = Some(expr);
                return Ok(true);
            }
            "body" => {
                let mut expr = expect_expr(node.take(), public_slot)?;
                assign_expr_slot_and_rank(&mut expr, "body", None, true);
                arm.body = expr;
                return Ok(true);
            }
            _ => {
                return Err(patch_error(&format!(
                    "slot `@{owner_id}.{public_slot}` is not available for `put`"
                )));
            }
        }
    }

    if let Some(guard) = &mut arm.guard {
        if put_in_expr(guard, owner_id, public_slot, node)? {
            return Ok(true);
        }
    }
    if put_in_expr(&mut arm.body, owner_id, public_slot, node)? {
        return Ok(true);
    }
    Ok(false)
}
