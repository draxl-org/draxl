use crate::error::{patch_error, PatchError};
use crate::model::PatchNode;
use draxl_ast::{Field, Item, MatchArm, Meta, Param, Stmt, Variant};

pub(super) fn stmt_id(stmt: &Stmt) -> Option<&str> {
    match stmt {
        Stmt::Let(node) => Some(node.meta.id.as_str()),
        Stmt::Expr(node) => Some(node.meta.id.as_str()),
        Stmt::Item(item) => Some(item.meta().id.as_str()),
        Stmt::Doc(node) => Some(node.meta.id.as_str()),
        Stmt::Comment(node) => Some(node.meta.id.as_str()),
    }
}

pub(super) fn stmt_slot_rank(stmt: &Stmt) -> Result<(&str, Option<String>), PatchError> {
    let meta = stmt_meta(stmt)?;
    Ok(meta_slot_rank(meta))
}

fn stmt_meta(stmt: &Stmt) -> Result<&Meta, PatchError> {
    stmt.meta()
        .ok_or_else(|| patch_error("patch operations require metadata on ranked statements"))
}

pub(super) fn meta_slot_rank(meta: &Meta) -> (&str, Option<String>) {
    (meta.slot.as_deref().unwrap_or_default(), meta.rank.clone())
}

pub(super) fn assign_item_slot_and_rank(
    item: &mut Item,
    slot: &str,
    rank: Option<&str>,
    overwrite_rank: bool,
) -> Result<(), PatchError> {
    match item {
        Item::Mod(node) => assign_meta_slot_and_rank(&mut node.meta, slot, rank, overwrite_rank),
        Item::Use(node) => assign_meta_slot_and_rank(&mut node.meta, slot, rank, overwrite_rank),
        Item::Struct(node) => assign_meta_slot_and_rank(&mut node.meta, slot, rank, overwrite_rank),
        Item::Enum(node) => assign_meta_slot_and_rank(&mut node.meta, slot, rank, overwrite_rank),
        Item::Fn(node) => assign_meta_slot_and_rank(&mut node.meta, slot, rank, overwrite_rank),
        Item::Doc(node) => assign_meta_slot_and_rank(&mut node.meta, slot, None, true),
        Item::Comment(node) => assign_meta_slot_and_rank(&mut node.meta, slot, None, true),
    }
    Ok(())
}

pub(super) fn assign_stmt_slot_and_rank(
    stmt: &mut Stmt,
    slot: &str,
    rank: Option<&str>,
    overwrite_rank: bool,
) -> Result<(), PatchError> {
    match stmt {
        Stmt::Let(node) => assign_meta_slot_and_rank(&mut node.meta, slot, rank, overwrite_rank),
        Stmt::Expr(node) => assign_meta_slot_and_rank(&mut node.meta, slot, rank, overwrite_rank),
        Stmt::Item(item) => assign_item_slot_and_rank(item, slot, rank, overwrite_rank)?,
        Stmt::Doc(node) => assign_meta_slot_and_rank(&mut node.meta, slot, None, true),
        Stmt::Comment(node) => assign_meta_slot_and_rank(&mut node.meta, slot, None, true),
    }
    Ok(())
}

pub(super) fn assign_meta_slot_and_rank(
    meta: &mut Meta,
    slot: &str,
    rank: Option<&str>,
    overwrite_rank: bool,
) {
    meta.slot = Some(slot.to_owned());
    if overwrite_rank || meta.rank.is_none() {
        meta.rank = rank.map(str::to_owned);
    }
}

pub(super) fn expect_item(node: Option<PatchNode>, slot: &str) -> Result<Item, PatchError> {
    match node {
        Some(PatchNode::Item(item)) => Ok(item),
        Some(_) => Err(patch_error(&format!(
            "slot `{}` accepts item nodes only",
            slot
        ))),
        None => Err(patch_error("patch node was consumed before insertion")),
    }
}

pub(super) fn expect_field(node: Option<PatchNode>, slot: &str) -> Result<Field, PatchError> {
    match node {
        Some(PatchNode::Field(field)) => Ok(field),
        Some(_) => Err(patch_error(&format!(
            "slot `{}` accepts field nodes only",
            slot
        ))),
        None => Err(patch_error("patch node was consumed before insertion")),
    }
}

pub(super) fn expect_variant(node: Option<PatchNode>, slot: &str) -> Result<Variant, PatchError> {
    match node {
        Some(PatchNode::Variant(variant)) => Ok(variant),
        Some(_) => Err(patch_error(&format!(
            "slot `{}` accepts variant nodes only",
            slot
        ))),
        None => Err(patch_error("patch node was consumed before insertion")),
    }
}

pub(super) fn expect_param(node: Option<PatchNode>, slot: &str) -> Result<Param, PatchError> {
    match node {
        Some(PatchNode::Param(param)) => Ok(param),
        Some(_) => Err(patch_error(&format!(
            "slot `{}` accepts parameter nodes only",
            slot
        ))),
        None => Err(patch_error("patch node was consumed before insertion")),
    }
}

pub(super) fn expect_stmt(node: Option<PatchNode>, slot: &str) -> Result<Stmt, PatchError> {
    match node {
        Some(PatchNode::Stmt(stmt)) => Ok(stmt),
        Some(_) => Err(patch_error(&format!(
            "slot `{}` accepts statement nodes only",
            slot
        ))),
        None => Err(patch_error("patch node was consumed before insertion")),
    }
}

pub(super) fn expect_match_arm(
    node: Option<PatchNode>,
    slot: &str,
) -> Result<MatchArm, PatchError> {
    match node {
        Some(PatchNode::MatchArm(arm)) => Ok(arm),
        Some(_) => Err(patch_error(&format!(
            "slot `{}` accepts match arm nodes only",
            slot
        ))),
        None => Err(patch_error("patch node was consumed before insertion")),
    }
}
