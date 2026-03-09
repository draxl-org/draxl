use crate::error::{patch_error, PatchError};
use crate::model::{PatchNode, SlotOwner, SlotRef};
use draxl_ast::{Expr, Field, Item, MatchArm, Meta, Param, Pattern, Stmt, Type, Variant};

pub(super) fn slot_ref_label(slot: &SlotRef) -> String {
    format!("{}.{}", slot_owner_label(&slot.owner), slot.slot)
}

pub(super) fn slot_owner_label(owner: &SlotOwner) -> String {
    match owner {
        SlotOwner::File => "file".to_owned(),
        SlotOwner::Node(id) => format!("@{id}"),
    }
}

pub(super) fn patch_node_kind(node: &PatchNode) -> &'static str {
    match node {
        PatchNode::Item(_) => "item",
        PatchNode::Field(_) => "field",
        PatchNode::Variant(_) => "variant",
        PatchNode::Param(_) => "parameter",
        PatchNode::Stmt(_) => "statement",
        PatchNode::MatchArm(_) => "match arm",
        PatchNode::Expr(_) => "expression",
        PatchNode::Type(_) => "type",
        PatchNode::Pattern(_) => "pattern",
        PatchNode::Doc(_) => "doc comment",
        PatchNode::Comment(_) => "line comment",
    }
}

pub(super) fn patch_node_id(node: &PatchNode) -> Option<&str> {
    match node {
        PatchNode::Item(item) => Some(item.meta().id.as_str()),
        PatchNode::Field(field) => Some(field.meta.id.as_str()),
        PatchNode::Variant(variant) => Some(variant.meta.id.as_str()),
        PatchNode::Param(param) => Some(param.meta.id.as_str()),
        PatchNode::Stmt(stmt) => stmt_id(stmt),
        PatchNode::MatchArm(arm) => Some(arm.meta.id.as_str()),
        PatchNode::Expr(expr) => expr_id(expr),
        PatchNode::Type(ty) => Some(ty.meta().id.as_str()),
        PatchNode::Pattern(pattern) => pattern_id(pattern),
        PatchNode::Doc(node) => Some(node.meta.id.as_str()),
        PatchNode::Comment(node) => Some(node.meta.id.as_str()),
    }
}

pub(super) fn require_insert_fragment(node: &PatchNode) -> Result<(), PatchError> {
    let Some(id) = patch_node_id(node) else {
        return Err(patch_error(&format!(
            "{} fragments must carry an outer node id",
            patch_node_kind(node)
        )));
    };
    ensure_fragment_meta_is_body_only(node, id)
}

pub(super) fn require_put_fragment(node: &PatchNode) -> Result<(), PatchError> {
    let Some(id) = patch_node_id(node) else {
        return Err(patch_error(&format!(
            "{} fragments must carry an outer node id",
            patch_node_kind(node)
        )));
    };
    ensure_fragment_meta_is_body_only(node, id)
}

pub(super) fn require_replace_fragment(
    node: &PatchNode,
    target_id: &str,
) -> Result<(), PatchError> {
    ensure_fragment_meta_is_body_only(node, target_id)
}

fn ensure_fragment_meta_is_body_only(
    node: &PatchNode,
    expected_id: &str,
) -> Result<(), PatchError> {
    let kind = patch_node_kind(node);
    match node {
        PatchNode::Item(item) => validate_fragment_meta(item.meta(), expected_id, kind),
        PatchNode::Field(field) => validate_fragment_meta(&field.meta, expected_id, kind),
        PatchNode::Variant(variant) => validate_fragment_meta(&variant.meta, expected_id, kind),
        PatchNode::Param(param) => validate_fragment_meta(&param.meta, expected_id, kind),
        PatchNode::Stmt(stmt) => validate_fragment_meta(
            stmt.meta()
                .ok_or_else(|| patch_error("statement fragments must carry metadata"))?,
            expected_id,
            kind,
        ),
        PatchNode::MatchArm(arm) => validate_fragment_meta(&arm.meta, expected_id, kind),
        PatchNode::Expr(expr) => {
            if let Some(meta) = expr.meta() {
                validate_fragment_meta(meta, expected_id, kind)?;
            }
            Ok(())
        }
        PatchNode::Type(ty) => validate_fragment_meta(ty.meta(), expected_id, kind),
        PatchNode::Pattern(pattern) => {
            if let Some(meta) = pattern.meta() {
                validate_fragment_meta(meta, expected_id, kind)?;
            }
            Ok(())
        }
        PatchNode::Doc(node) => validate_fragment_meta(&node.meta, expected_id, kind),
        PatchNode::Comment(node) => validate_fragment_meta(&node.meta, expected_id, kind),
    }
}

fn validate_fragment_meta(meta: &Meta, expected_id: &str, kind: &str) -> Result<(), PatchError> {
    if meta.id != expected_id {
        return Err(patch_error(&format!(
            "{kind} fragment id `{}` does not match the target id `{expected_id}`",
            meta.id
        )));
    }
    if meta.rank.is_some() {
        return Err(patch_error(&format!(
            "{kind} fragment for `{expected_id}` must omit outer rank metadata"
        )));
    }
    if meta.slot.is_some() {
        return Err(patch_error(&format!(
            "{kind} fragment for `{expected_id}` must omit outer slot metadata"
        )));
    }
    if meta.anchor.is_some() {
        return Err(patch_error(&format!(
            "{kind} fragment for `{expected_id}` must omit outer anchor metadata"
        )));
    }
    Ok(())
}

pub(super) fn stmt_id(stmt: &Stmt) -> Option<&str> {
    match stmt {
        Stmt::Let(node) => Some(node.meta.id.as_str()),
        Stmt::Expr(node) => Some(node.meta.id.as_str()),
        Stmt::Item(item) => Some(item.meta().id.as_str()),
        Stmt::Doc(node) => Some(node.meta.id.as_str()),
        Stmt::Comment(node) => Some(node.meta.id.as_str()),
    }
}

pub(super) fn expr_id(expr: &Expr) -> Option<&str> {
    expr.meta().map(|meta| meta.id.as_str())
}

pub(super) fn pattern_id(pattern: &Pattern) -> Option<&str> {
    pattern.meta().map(|meta| meta.id.as_str())
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

pub(super) fn assign_expr_slot_and_rank(
    expr: &mut Expr,
    slot: &str,
    rank: Option<&str>,
    overwrite_rank: bool,
) {
    if let Some(meta) = expr.meta_mut() {
        assign_meta_slot_and_rank(meta, slot, rank, overwrite_rank);
    }
}

pub(super) fn assign_pattern_slot_and_rank(
    pattern: &mut Pattern,
    slot: &str,
    rank: Option<&str>,
    overwrite_rank: bool,
) {
    match pattern {
        Pattern::Ident(node) => {
            if let Some(meta) = &mut node.meta {
                assign_meta_slot_and_rank(meta, slot, rank, overwrite_rank);
            }
        }
        Pattern::Wild(node) => {
            if let Some(meta) = &mut node.meta {
                assign_meta_slot_and_rank(meta, slot, rank, overwrite_rank);
            }
        }
    }
}

pub(super) fn assign_type_slot_and_rank(
    ty: &mut Type,
    slot: &str,
    rank: Option<&str>,
    overwrite_rank: bool,
) {
    match ty {
        Type::Path(node) => assign_meta_slot_and_rank(&mut node.meta, slot, rank, overwrite_rank),
    }
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
    meta.span = None;
}

pub(super) fn apply_shell_to_item(item: &mut Item, shell: &Meta) {
    apply_shell_to_meta(item.meta_mut(), shell);
}

pub(super) fn apply_shell_to_field(field: &mut Field, shell: &Meta) {
    apply_shell_to_meta(&mut field.meta, shell);
}

pub(super) fn apply_shell_to_variant(variant: &mut Variant, shell: &Meta) {
    apply_shell_to_meta(&mut variant.meta, shell);
}

pub(super) fn apply_shell_to_param(param: &mut Param, shell: &Meta) {
    apply_shell_to_meta(&mut param.meta, shell);
}

pub(super) fn apply_shell_to_stmt(stmt: &mut Stmt, shell: &Meta) {
    match stmt {
        Stmt::Let(node) => apply_shell_to_meta(&mut node.meta, shell),
        Stmt::Expr(node) => apply_shell_to_meta(&mut node.meta, shell),
        Stmt::Item(item) => apply_shell_to_item(item, shell),
        Stmt::Doc(node) => apply_shell_to_meta(&mut node.meta, shell),
        Stmt::Comment(node) => apply_shell_to_meta(&mut node.meta, shell),
    }
}

pub(super) fn apply_shell_to_expr(expr: &mut Expr, shell: &Meta) {
    ensure_expr_meta(expr, shell);
    apply_shell_to_meta(
        expr.meta_mut()
            .expect("expression shell application must leave metadata present"),
        shell,
    );
}

pub(super) fn apply_shell_to_pattern(pattern: &mut Pattern, shell: &Meta) {
    ensure_pattern_meta(pattern, shell);
    match pattern {
        Pattern::Ident(node) => apply_shell_to_meta(
            node.meta
                .as_mut()
                .expect("pattern shell application must leave metadata present"),
            shell,
        ),
        Pattern::Wild(node) => apply_shell_to_meta(
            node.meta
                .as_mut()
                .expect("pattern shell application must leave metadata present"),
            shell,
        ),
    }
}

pub(super) fn apply_shell_to_type(ty: &mut Type, shell: &Meta) {
    match ty {
        Type::Path(node) => apply_shell_to_meta(&mut node.meta, shell),
    }
}

pub(super) fn apply_shell_to_match_arm(arm: &mut MatchArm, shell: &Meta) {
    apply_shell_to_meta(&mut arm.meta, shell);
}

pub(super) fn clear_patch_node_outer_placement(node: &mut PatchNode) {
    match node {
        PatchNode::Item(item) => clear_meta_placement(item.meta_mut()),
        PatchNode::Field(field) => clear_meta_placement(&mut field.meta),
        PatchNode::Variant(variant) => clear_meta_placement(&mut variant.meta),
        PatchNode::Param(param) => clear_meta_placement(&mut param.meta),
        PatchNode::Stmt(stmt) => clear_stmt_outer_placement(stmt),
        PatchNode::MatchArm(arm) => clear_meta_placement(&mut arm.meta),
        PatchNode::Expr(expr) => {
            if let Some(meta) = expr.meta_mut() {
                clear_meta_placement(meta);
            }
        }
        PatchNode::Type(ty) => match ty {
            Type::Path(node) => clear_meta_placement(&mut node.meta),
        },
        PatchNode::Pattern(pattern) => clear_pattern_outer_placement(pattern),
        PatchNode::Doc(node) => clear_meta_placement(&mut node.meta),
        PatchNode::Comment(node) => clear_meta_placement(&mut node.meta),
    }
}

fn apply_shell_to_meta(meta: &mut Meta, shell: &Meta) {
    meta.id = shell.id.clone();
    meta.rank = shell.rank.clone();
    meta.anchor = shell.anchor.clone();
    meta.slot = shell.slot.clone();
    meta.span = None;
}

fn clear_stmt_outer_placement(stmt: &mut Stmt) {
    match stmt {
        Stmt::Let(node) => clear_meta_placement(&mut node.meta),
        Stmt::Expr(node) => clear_meta_placement(&mut node.meta),
        Stmt::Item(item) => clear_meta_placement(item.meta_mut()),
        Stmt::Doc(node) => clear_meta_placement(&mut node.meta),
        Stmt::Comment(node) => clear_meta_placement(&mut node.meta),
    }
}

fn clear_pattern_outer_placement(pattern: &mut Pattern) {
    match pattern {
        Pattern::Ident(node) => {
            if let Some(meta) = &mut node.meta {
                clear_meta_placement(meta);
            }
        }
        Pattern::Wild(node) => {
            if let Some(meta) = &mut node.meta {
                clear_meta_placement(meta);
            }
        }
    }
}

fn clear_meta_placement(meta: &mut Meta) {
    meta.rank = None;
    meta.anchor = None;
    meta.slot = None;
    meta.span = None;
}

fn ensure_expr_meta(expr: &mut Expr, shell: &Meta) {
    match expr {
        Expr::Path(node) => ensure_optional_meta(&mut node.meta, shell),
        Expr::Lit(node) => ensure_optional_meta(&mut node.meta, shell),
        Expr::Group(node) => ensure_optional_meta(&mut node.meta, shell),
        Expr::Binary(node) => ensure_optional_meta(&mut node.meta, shell),
        Expr::Unary(node) => ensure_optional_meta(&mut node.meta, shell),
        Expr::Call(node) => ensure_optional_meta(&mut node.meta, shell),
        Expr::Match(node) => ensure_optional_meta(&mut node.meta, shell),
        Expr::Block(node) => ensure_optional_meta(&mut node.meta, shell),
    }
}

fn ensure_pattern_meta(pattern: &mut Pattern, shell: &Meta) {
    match pattern {
        Pattern::Ident(node) => ensure_optional_meta(&mut node.meta, shell),
        Pattern::Wild(node) => ensure_optional_meta(&mut node.meta, shell),
    }
}

fn ensure_optional_meta(slot: &mut Option<Meta>, shell: &Meta) {
    if slot.is_none() {
        *slot = Some(Meta {
            id: shell.id.clone(),
            rank: None,
            anchor: None,
            slot: None,
            span: None,
        });
    }
}

pub(super) fn expect_item(node: Option<PatchNode>, slot: &str) -> Result<Item, PatchError> {
    match node {
        Some(PatchNode::Item(item)) => Ok(item),
        Some(other) => Err(slot_kind_error(slot, "item", Some(&other))),
        None => Err(patch_error("patch node was consumed before use")),
    }
}

pub(super) fn expect_field(node: Option<PatchNode>, slot: &str) -> Result<Field, PatchError> {
    match node {
        Some(PatchNode::Field(field)) => Ok(field),
        Some(other) => Err(slot_kind_error(slot, "field", Some(&other))),
        None => Err(patch_error("patch node was consumed before use")),
    }
}

pub(super) fn expect_variant(node: Option<PatchNode>, slot: &str) -> Result<Variant, PatchError> {
    match node {
        Some(PatchNode::Variant(variant)) => Ok(variant),
        Some(other) => Err(slot_kind_error(slot, "variant", Some(&other))),
        None => Err(patch_error("patch node was consumed before use")),
    }
}

pub(super) fn expect_param(node: Option<PatchNode>, slot: &str) -> Result<Param, PatchError> {
    match node {
        Some(PatchNode::Param(param)) => Ok(param),
        Some(other) => Err(slot_kind_error(slot, "parameter", Some(&other))),
        None => Err(patch_error("patch node was consumed before use")),
    }
}

pub(super) fn expect_stmt(node: Option<PatchNode>, slot: &str) -> Result<Stmt, PatchError> {
    match node {
        Some(PatchNode::Stmt(stmt)) => Ok(stmt),
        Some(other) => Err(slot_kind_error(slot, "statement", Some(&other))),
        None => Err(patch_error("patch node was consumed before use")),
    }
}

pub(super) fn expect_match_arm(
    node: Option<PatchNode>,
    slot: &str,
) -> Result<MatchArm, PatchError> {
    match node {
        Some(PatchNode::MatchArm(arm)) => Ok(arm),
        Some(other) => Err(slot_kind_error(slot, "match arm", Some(&other))),
        None => Err(patch_error("patch node was consumed before use")),
    }
}

pub(super) fn expect_expr(node: Option<PatchNode>, slot: &str) -> Result<Expr, PatchError> {
    match node {
        Some(PatchNode::Expr(expr)) => Ok(expr),
        Some(other) => Err(slot_kind_error(slot, "expression", Some(&other))),
        None => Err(patch_error("patch node was consumed before use")),
    }
}

pub(super) fn expect_type(node: Option<PatchNode>, slot: &str) -> Result<Type, PatchError> {
    match node {
        Some(PatchNode::Type(ty)) => Ok(ty),
        Some(other) => Err(slot_kind_error(slot, "type", Some(&other))),
        None => Err(patch_error("patch node was consumed before use")),
    }
}

pub(super) fn expect_pattern(node: Option<PatchNode>, slot: &str) -> Result<Pattern, PatchError> {
    match node {
        Some(PatchNode::Pattern(pattern)) => Ok(pattern),
        Some(other) => Err(slot_kind_error(slot, "pattern", Some(&other))),
        None => Err(patch_error("patch node was consumed before use")),
    }
}

fn slot_kind_error(slot: &str, expected: &str, found: Option<&PatchNode>) -> PatchError {
    let found = found.map(patch_node_kind).unwrap_or("unknown fragment");
    patch_error(&format!(
        "slot `{slot}` accepts {expected} nodes only, found {found}"
    ))
}

pub(super) fn is_item_trivia(item: &Item) -> bool {
    matches!(item, Item::Doc(_) | Item::Comment(_))
}

pub(super) fn is_stmt_trivia(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Doc(_) | Stmt::Comment(_))
}

pub(super) fn resolved_item_attachment_targets(items: &[Item]) -> Vec<Option<String>> {
    let mut next_semantic = None;
    let mut targets = vec![None; items.len()];
    for index in (0..items.len()).rev() {
        match &items[index] {
            Item::Doc(node) => {
                targets[index] = node.meta.anchor.clone().or_else(|| next_semantic.clone());
            }
            Item::Comment(node) => {
                targets[index] = node.meta.anchor.clone().or_else(|| next_semantic.clone());
            }
            item => {
                next_semantic = Some(item.meta().id.clone());
            }
        }
    }
    targets
}

pub(super) fn resolved_stmt_attachment_targets(stmts: &[Stmt]) -> Vec<Option<String>> {
    let mut next_semantic = None;
    let mut targets = vec![None; stmts.len()];
    for index in (0..stmts.len()).rev() {
        match &stmts[index] {
            Stmt::Doc(node) => {
                targets[index] = node.meta.anchor.clone().or_else(|| next_semantic.clone());
            }
            Stmt::Comment(node) => {
                targets[index] = node.meta.anchor.clone().or_else(|| next_semantic.clone());
            }
            stmt => {
                next_semantic = stmt_id(stmt).map(str::to_owned);
            }
        }
    }
    targets
}

pub(super) fn semantic_item_target_ids(items: &[Item]) -> Vec<&str> {
    items
        .iter()
        .filter(|item| !is_item_trivia(item))
        .map(|item| item.meta().id.as_str())
        .collect()
}

pub(super) fn semantic_stmt_target_ids(stmts: &[Stmt]) -> Vec<&str> {
    stmts
        .iter()
        .filter(|stmt| !is_stmt_trivia(stmt))
        .filter_map(stmt_id)
        .collect()
}
