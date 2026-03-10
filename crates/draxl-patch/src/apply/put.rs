use super::support::{
    assign_expr_slot_and_rank, assign_pattern_slot_and_rank, assign_type_slot_and_rank,
    expect_expr, expect_pattern, expect_type, require_put_fragment, slot_ref_label,
};
use crate::error::{patch_error, PatchError};
use crate::model::{PatchNode, SlotOwner, SlotRef};
use crate::schema::{expr_kind, invalid_single_slot_message, item_kind, single_slot_spec};
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
        let spec = single_slot_spec(item_kind(item), public_slot).ok_or_else(|| {
            patch_error(&invalid_single_slot_message(
                &format!("@{owner_id}"),
                public_slot,
            ))
        })?;
        match item {
            Item::Fn(function) => {
                let mut ty = expect_type(node.take(), spec.public_name)?;
                assign_type_slot_and_rank(&mut ty, spec.meta_slot_name, None, true);
                function.ret_ty = Some(ty);
                return Ok(true);
            }
            Item::Mod(_) | Item::Use(_) | Item::Struct(_) | Item::Enum(_) => unreachable!(),
            Item::Doc(_) | Item::Comment(_) => unreachable!(),
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
                    let spec = single_slot_spec(crate::schema::NodeKind::Field, public_slot)
                        .ok_or_else(|| {
                            patch_error(&invalid_single_slot_message(
                                &format!("@{owner_id}"),
                                public_slot,
                            ))
                        })?;
                    let mut ty = expect_type(node.take(), spec.public_name)?;
                    assign_type_slot_and_rank(&mut ty, spec.meta_slot_name, None, true);
                    field.ty = ty;
                    return Ok(true);
                }
            }
        }
        Item::Fn(function) => {
            for param in &mut function.params {
                if param.meta.id == owner_id {
                    let spec = single_slot_spec(crate::schema::NodeKind::Param, public_slot)
                        .ok_or_else(|| {
                            patch_error(&invalid_single_slot_message(
                                &format!("@{owner_id}"),
                                public_slot,
                            ))
                        })?;
                    let mut ty = expect_type(node.take(), spec.public_name)?;
                    assign_type_slot_and_rank(&mut ty, spec.meta_slot_name, None, true);
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
        let spec =
            single_slot_spec(crate::schema::stmt_kind(stmt), public_slot).ok_or_else(|| {
                patch_error(&invalid_single_slot_message(
                    &format!("@{owner_id}"),
                    public_slot,
                ))
            })?;
        match stmt {
            Stmt::Let(let_stmt) => match spec.fragment_kind {
                crate::schema::FragmentKind::Pattern => {
                    let mut pattern = expect_pattern(node.take(), spec.public_name)?;
                    assign_pattern_slot_and_rank(&mut pattern, spec.meta_slot_name, None, true);
                    let_stmt.pat = pattern;
                    return Ok(true);
                }
                crate::schema::FragmentKind::Expr => {
                    let mut expr = expect_expr(node.take(), spec.public_name)?;
                    assign_expr_slot_and_rank(&mut expr, spec.meta_slot_name, None, true);
                    let_stmt.value = expr;
                    return Ok(true);
                }
                _ => unreachable!(),
            },
            Stmt::Expr(expr_stmt) => {
                let mut expr = expect_expr(node.take(), spec.public_name)?;
                assign_expr_slot_and_rank(&mut expr, spec.meta_slot_name, None, true);
                expr_stmt.expr = expr;
                return Ok(true);
            }
            Stmt::Item(_) | Stmt::Doc(_) | Stmt::Comment(_) => unreachable!(),
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
        let spec = single_slot_spec(expr_kind(expr), public_slot).ok_or_else(|| {
            patch_error(&invalid_single_slot_message(
                &format!("@{owner_id}"),
                public_slot,
            ))
        })?;
        match expr {
            Expr::Group(group) => {
                let mut inner = expect_expr(node.take(), spec.public_name)?;
                assign_expr_slot_and_rank(&mut inner, spec.meta_slot_name, None, true);
                group.expr = Box::new(inner);
                return Ok(true);
            }
            Expr::Binary(binary) => {
                let mut inner = expect_expr(node.take(), spec.public_name)?;
                assign_expr_slot_and_rank(&mut inner, spec.meta_slot_name, None, true);
                if spec.public_name == "lhs" {
                    binary.lhs = Box::new(inner);
                } else {
                    binary.rhs = Box::new(inner);
                }
                return Ok(true);
            }
            Expr::Unary(unary) => {
                let mut inner = expect_expr(node.take(), spec.public_name)?;
                assign_expr_slot_and_rank(&mut inner, spec.meta_slot_name, None, true);
                unary.expr = Box::new(inner);
                return Ok(true);
            }
            Expr::Call(call) => {
                let mut inner = expect_expr(node.take(), spec.public_name)?;
                assign_expr_slot_and_rank(&mut inner, spec.meta_slot_name, None, true);
                call.callee = Box::new(inner);
                return Ok(true);
            }
            Expr::Match(match_expr) => {
                let mut inner = expect_expr(node.take(), spec.public_name)?;
                assign_expr_slot_and_rank(&mut inner, spec.meta_slot_name, None, true);
                match_expr.scrutinee = Box::new(inner);
                return Ok(true);
            }
            Expr::Block(_) | Expr::Path(_) | Expr::Lit(_) => unreachable!(),
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
        let spec =
            single_slot_spec(crate::schema::NodeKind::MatchArm, public_slot).ok_or_else(|| {
                patch_error(&invalid_single_slot_message(
                    &format!("@{owner_id}"),
                    public_slot,
                ))
            })?;
        match spec.fragment_kind {
            crate::schema::FragmentKind::Pattern => {
                let mut pattern = expect_pattern(node.take(), spec.public_name)?;
                assign_pattern_slot_and_rank(&mut pattern, spec.meta_slot_name, None, true);
                arm.pat = pattern;
                return Ok(true);
            }
            crate::schema::FragmentKind::Expr if spec.public_name == "guard" => {
                let mut expr = expect_expr(node.take(), spec.public_name)?;
                assign_expr_slot_and_rank(&mut expr, spec.meta_slot_name, None, true);
                arm.guard = Some(expr);
                return Ok(true);
            }
            crate::schema::FragmentKind::Expr => {
                let mut expr = expect_expr(node.take(), spec.public_name)?;
                assign_expr_slot_and_rank(&mut expr, spec.meta_slot_name, None, true);
                arm.body = expr;
                return Ok(true);
            }
            _ => unreachable!(),
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
