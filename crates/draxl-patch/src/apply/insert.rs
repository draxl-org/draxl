use super::support::{
    assign_item_slot_and_rank, assign_meta_slot_and_rank, assign_stmt_slot_and_rank, expect_field,
    expect_item, expect_match_arm, expect_param, expect_stmt, expect_variant,
    require_insert_fragment, slot_ref_label,
};
use crate::error::{patch_error, PatchError};
use crate::model::{PatchNode, RankedDest, SlotOwner};
use crate::schema::{
    expr_kind, invalid_ranked_slot_message, item_kind, ranked_slot_spec, NodeKind,
};
use draxl_ast::{Block, Expr, File, Item, Stmt};

pub(super) fn apply_insert(
    file: &mut File,
    dest: RankedDest,
    node: PatchNode,
) -> Result<(), PatchError> {
    require_insert_fragment(&node)?;

    let mut node = Some(node);
    let found = match &dest.slot.owner {
        SlotOwner::File => {
            insert_into_file_slot(&mut file.items, &dest.slot.slot, &dest.rank, &mut node)?;
            true
        }
        SlotOwner::Node(id) => {
            let mut found = false;
            for item in &mut file.items {
                if insert_into_item(item, id, &dest.slot.slot, &dest.rank, &mut node)? {
                    found = true;
                    break;
                }
            }
            found
        }
    };

    if !found {
        return Err(patch_error(&format!(
            "insert destination `{}` was not found",
            slot_ref_label(&dest.slot)
        )));
    }

    Ok(())
}

fn insert_into_file_slot(
    items: &mut Vec<Item>,
    public_slot: &str,
    rank: &str,
    node: &mut Option<PatchNode>,
) -> Result<(), PatchError> {
    let spec = ranked_slot_spec(NodeKind::File, public_slot)
        .ok_or_else(|| patch_error(&invalid_ranked_slot_message("file", public_slot)))?;
    let mut item = expect_item(node.take(), spec.public_name)?;
    assign_item_slot_and_rank(&mut item, spec.meta_slot_name, Some(rank), true)?;
    items.push(item);
    Ok(())
}

fn insert_into_item(
    item: &mut Item,
    owner_id: &str,
    public_slot: &str,
    rank: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if item.meta().id == owner_id {
        let spec = ranked_slot_spec(item_kind(item), public_slot).ok_or_else(|| {
            patch_error(&invalid_ranked_slot_message(
                &format!("@{owner_id}"),
                public_slot,
            ))
        })?;
        match item {
            Item::Mod(module) => {
                let mut child = expect_item(node.take(), spec.public_name)?;
                assign_item_slot_and_rank(&mut child, spec.meta_slot_name, Some(rank), true)?;
                module.items.push(child);
                return Ok(true);
            }
            Item::Struct(strukt) => {
                let mut field = expect_field(node.take(), spec.public_name)?;
                assign_meta_slot_and_rank(&mut field.meta, spec.meta_slot_name, Some(rank), true);
                strukt.fields.push(field);
                return Ok(true);
            }
            Item::Enum(enm) => {
                let mut variant = expect_variant(node.take(), spec.public_name)?;
                assign_meta_slot_and_rank(&mut variant.meta, spec.meta_slot_name, Some(rank), true);
                enm.variants.push(variant);
                return Ok(true);
            }
            Item::Fn(function) => match spec.fragment_kind {
                crate::schema::FragmentKind::Param => {
                    let mut param = expect_param(node.take(), spec.public_name)?;
                    assign_meta_slot_and_rank(
                        &mut param.meta,
                        spec.meta_slot_name,
                        Some(rank),
                        true,
                    );
                    function.params.push(param);
                    return Ok(true);
                }
                crate::schema::FragmentKind::Stmt => {
                    let mut stmt = expect_stmt(node.take(), spec.public_name)?;
                    assign_stmt_slot_and_rank(&mut stmt, spec.meta_slot_name, Some(rank), true)?;
                    function.body.stmts.push(stmt);
                    return Ok(true);
                }
                _ => {
                    return Err(patch_error(
                        "ranked function slot expected parameter or statement",
                    ))
                }
            },
            Item::Use(_) | Item::Doc(_) | Item::Comment(_) => unreachable!(),
        }
    }

    match item {
        Item::Mod(module) => {
            for child in &mut module.items {
                if insert_into_item(child, owner_id, public_slot, rank, node)? {
                    return Ok(true);
                }
            }
        }
        Item::Fn(function) => {
            if insert_into_block(&mut function.body, owner_id, public_slot, rank, node)? {
                return Ok(true);
            }
        }
        Item::Use(_) | Item::Struct(_) | Item::Enum(_) | Item::Doc(_) | Item::Comment(_) => {}
    }

    Ok(false)
}

fn insert_into_block(
    block: &mut Block,
    owner_id: &str,
    public_slot: &str,
    rank: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if let Some(meta) = &block.meta {
        if meta.id == owner_id {
            let spec = ranked_slot_spec(NodeKind::ExprBlock, public_slot).ok_or_else(|| {
                patch_error(&invalid_ranked_slot_message(
                    &format!("@{owner_id}"),
                    public_slot,
                ))
            })?;
            let mut stmt = expect_stmt(node.take(), spec.public_name)?;
            assign_stmt_slot_and_rank(&mut stmt, spec.meta_slot_name, Some(rank), true)?;
            block.stmts.push(stmt);
            return Ok(true);
        }
    }

    for stmt in &mut block.stmts {
        if insert_into_stmt(stmt, owner_id, public_slot, rank, node)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn insert_into_stmt(
    stmt: &mut Stmt,
    owner_id: &str,
    public_slot: &str,
    rank: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    match stmt {
        Stmt::Let(let_stmt) => {
            insert_into_expr(&mut let_stmt.value, owner_id, public_slot, rank, node)
        }
        Stmt::Expr(expr_stmt) => {
            insert_into_expr(&mut expr_stmt.expr, owner_id, public_slot, rank, node)
        }
        Stmt::Item(item) => insert_into_item(item, owner_id, public_slot, rank, node),
        Stmt::Doc(_) | Stmt::Comment(_) => Ok(false),
    }
}

fn insert_into_expr(
    expr: &mut Expr,
    owner_id: &str,
    public_slot: &str,
    rank: &str,
    node: &mut Option<PatchNode>,
) -> Result<bool, PatchError> {
    if let Some(meta) = expr.meta() {
        if meta.id == owner_id {
            let spec = ranked_slot_spec(expr_kind(expr), public_slot).ok_or_else(|| {
                patch_error(&invalid_ranked_slot_message(
                    &format!("@{owner_id}"),
                    public_slot,
                ))
            })?;
            match expr {
                Expr::Match(match_expr) => {
                    let mut arm = expect_match_arm(node.take(), spec.public_name)?;
                    assign_meta_slot_and_rank(&mut arm.meta, spec.meta_slot_name, Some(rank), true);
                    match_expr.arms.push(arm);
                    return Ok(true);
                }
                Expr::Block(block) => {
                    let mut stmt = expect_stmt(node.take(), spec.public_name)?;
                    assign_stmt_slot_and_rank(&mut stmt, spec.meta_slot_name, Some(rank), true)?;
                    block.stmts.push(stmt);
                    return Ok(true);
                }
                Expr::Path(_)
                | Expr::Lit(_)
                | Expr::Group(_)
                | Expr::Binary(_)
                | Expr::Unary(_)
                | Expr::Call(_) => unreachable!(),
            }
        }
    }

    match expr {
        Expr::Group(group) => insert_into_expr(&mut group.expr, owner_id, public_slot, rank, node),
        Expr::Binary(binary) => {
            if insert_into_expr(&mut binary.lhs, owner_id, public_slot, rank, node)? {
                return Ok(true);
            }
            insert_into_expr(&mut binary.rhs, owner_id, public_slot, rank, node)
        }
        Expr::Unary(unary) => insert_into_expr(&mut unary.expr, owner_id, public_slot, rank, node),
        Expr::Call(call) => {
            if insert_into_expr(&mut call.callee, owner_id, public_slot, rank, node)? {
                return Ok(true);
            }
            for arg in &mut call.args {
                if insert_into_expr(arg, owner_id, public_slot, rank, node)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Match(match_expr) => {
            if insert_into_expr(&mut match_expr.scrutinee, owner_id, public_slot, rank, node)? {
                return Ok(true);
            }
            for arm in &mut match_expr.arms {
                if let Some(guard) = &mut arm.guard {
                    if insert_into_expr(guard, owner_id, public_slot, rank, node)? {
                        return Ok(true);
                    }
                }
                if insert_into_expr(&mut arm.body, owner_id, public_slot, rank, node)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Block(block) => insert_into_block(block, owner_id, public_slot, rank, node),
        Expr::Path(_) | Expr::Lit(_) => Ok(false),
    }
}
