use super::support::{
    assign_item_slot_and_rank, assign_stmt_slot_and_rank, clear_patch_node_outer_placement,
    expr_id, is_item_trivia, is_stmt_trivia, pattern_id, resolved_item_attachment_targets,
    resolved_stmt_attachment_targets, stmt_id,
};
use super::{insert, put};
use crate::error::{patch_error, PatchError};
use crate::model::{PatchDest, PatchNode, RankedDest, SlotOwner};
use draxl_ast::{Block, Expr, File, Item, MatchArm, Stmt};

pub(super) fn apply_move(
    file: &mut File,
    target_id: &str,
    dest: PatchDest,
) -> Result<(), PatchError> {
    let mut working = file.clone();
    let mut extracted = extract_from_items(&mut working.items, target_id)?
        .ok_or_else(|| patch_error(&format!("move target `{target_id}` was not found")))?;

    clear_patch_node_outer_placement(&mut extracted.node);

    match dest.clone() {
        PatchDest::Ranked(dest) => insert::apply_insert(&mut working, dest, extracted.node)?,
        PatchDest::Slot(slot) => {
            if extracted.closure.is_some() {
                return Err(patch_error(
                    "cannot move a node with attached docs/comments into a single-child slot",
                ));
            }
            put::apply_put(&mut working, slot, extracted.node)?;
        }
    }

    if let Some(closure) = extracted.closure {
        append_attachment_closure(&mut working, &dest, target_id, closure)?;
    }

    *file = working;
    Ok(())
}

enum AttachmentClosure {
    Items(Vec<Item>),
    Stmts(Vec<Stmt>),
}

struct ExtractedNode {
    node: PatchNode,
    closure: Option<AttachmentClosure>,
}

fn extract_from_items(
    items: &mut Vec<Item>,
    target_id: &str,
) -> Result<Option<ExtractedNode>, PatchError> {
    if let Some(index) = items.iter().position(|item| item.meta().id == target_id) {
        if is_item_trivia(&items[index]) {
            return Err(patch_error(
                "move does not support doc or comment targets; use attach, detach, replace, or delete",
            ));
        }

        let attachment_targets = resolved_item_attachment_targets(items);
        let mut moved = None;
        let mut closure = Vec::new();
        let mut retained = Vec::with_capacity(items.len());
        for (current, item) in std::mem::take(items).into_iter().enumerate() {
            if current == index {
                moved = Some(PatchNode::Item(item));
            } else if is_item_trivia(&item)
                && attachment_targets[current].as_deref() == Some(target_id)
            {
                closure.push(item);
            } else {
                retained.push(item);
            }
        }
        *items = retained;
        return Ok(Some(ExtractedNode {
            node: moved.expect("moved item must exist"),
            closure: (!closure.is_empty()).then_some(AttachmentClosure::Items(closure)),
        }));
    }

    for item in items {
        if let Some(extracted) = extract_from_item(item, target_id)? {
            return Ok(Some(extracted));
        }
    }

    Ok(None)
}

fn extract_from_item(
    item: &mut Item,
    target_id: &str,
) -> Result<Option<ExtractedNode>, PatchError> {
    match item {
        Item::Mod(module) => extract_from_items(&mut module.items, target_id),
        Item::Struct(strukt) => {
            if let Some(index) = strukt
                .fields
                .iter()
                .position(|field| field.meta.id == target_id)
            {
                return Ok(Some(ExtractedNode {
                    node: PatchNode::Field(strukt.fields.remove(index)),
                    closure: None,
                }));
            }
            for field in &strukt.fields {
                if field.ty.meta().id == target_id {
                    return Err(required_source_error(target_id, "ty"));
                }
            }
            Ok(None)
        }
        Item::Enum(enm) => {
            if let Some(index) = enm
                .variants
                .iter()
                .position(|variant| variant.meta.id == target_id)
            {
                return Ok(Some(ExtractedNode {
                    node: PatchNode::Variant(enm.variants.remove(index)),
                    closure: None,
                }));
            }
            Ok(None)
        }
        Item::Fn(function) => {
            if let Some(index) = function
                .params
                .iter()
                .position(|param| param.meta.id == target_id)
            {
                return Ok(Some(ExtractedNode {
                    node: PatchNode::Param(function.params.remove(index)),
                    closure: None,
                }));
            }
            for param in &function.params {
                if param.ty.meta().id == target_id {
                    return Err(required_source_error(target_id, "ty"));
                }
            }
            if function
                .ret_ty
                .as_ref()
                .is_some_and(|ret_ty| ret_ty.meta().id == target_id)
            {
                return Ok(Some(ExtractedNode {
                    node: PatchNode::Type(
                        function
                            .ret_ty
                            .take()
                            .expect("return type must exist when matched"),
                    ),
                    closure: None,
                }));
            }
            extract_from_block(&mut function.body, target_id)
        }
        Item::Use(_) | Item::Doc(_) | Item::Comment(_) => Ok(None),
    }
}

fn extract_from_block(
    block: &mut Block,
    target_id: &str,
) -> Result<Option<ExtractedNode>, PatchError> {
    if let Some(index) = block
        .stmts
        .iter()
        .position(|stmt| stmt_id(stmt).is_some_and(|id| id == target_id))
    {
        if is_stmt_trivia(&block.stmts[index]) {
            return Err(patch_error(
                "move does not support doc or comment targets; use attach, detach, replace, or delete",
            ));
        }

        let attachment_targets = resolved_stmt_attachment_targets(&block.stmts);
        let mut moved = None;
        let mut closure = Vec::new();
        let mut retained = Vec::with_capacity(block.stmts.len());
        for (current, stmt) in std::mem::take(&mut block.stmts).into_iter().enumerate() {
            if current == index {
                moved = Some(PatchNode::Stmt(stmt));
            } else if is_stmt_trivia(&stmt)
                && attachment_targets[current].as_deref() == Some(target_id)
            {
                closure.push(stmt);
            } else {
                retained.push(stmt);
            }
        }
        block.stmts = retained;
        return Ok(Some(ExtractedNode {
            node: moved.expect("moved statement must exist"),
            closure: (!closure.is_empty()).then_some(AttachmentClosure::Stmts(closure)),
        }));
    }

    for stmt in &mut block.stmts {
        if let Some(extracted) = extract_from_stmt(stmt, target_id)? {
            return Ok(Some(extracted));
        }
    }

    Ok(None)
}

fn extract_from_stmt(
    stmt: &mut Stmt,
    target_id: &str,
) -> Result<Option<ExtractedNode>, PatchError> {
    match stmt {
        Stmt::Let(let_stmt) => {
            if pattern_id(&let_stmt.pat).is_some_and(|id| id == target_id) {
                return Err(required_source_error(target_id, "pat"));
            }
            if expr_id(&let_stmt.value).is_some_and(|id| id == target_id) {
                return Err(required_source_error(target_id, "init"));
            }
            extract_from_expr(&mut let_stmt.value, target_id)
        }
        Stmt::Expr(expr_stmt) => {
            if expr_id(&expr_stmt.expr).is_some_and(|id| id == target_id) {
                return Err(required_source_error(target_id, "expr"));
            }
            extract_from_expr(&mut expr_stmt.expr, target_id)
        }
        Stmt::Item(item) => extract_from_item(item, target_id),
        Stmt::Doc(_) | Stmt::Comment(_) => Ok(None),
    }
}

fn extract_from_expr(
    expr: &mut Expr,
    target_id: &str,
) -> Result<Option<ExtractedNode>, PatchError> {
    match expr {
        Expr::Group(group) => {
            if expr_id(&group.expr).is_some_and(|id| id == target_id) {
                return Err(required_source_error(target_id, "expr"));
            }
            extract_from_expr(&mut group.expr, target_id)
        }
        Expr::Binary(binary) => {
            if expr_id(&binary.lhs).is_some_and(|id| id == target_id) {
                return Err(required_source_error(target_id, "lhs"));
            }
            if expr_id(&binary.rhs).is_some_and(|id| id == target_id) {
                return Err(required_source_error(target_id, "rhs"));
            }
            if let Some(extracted) = extract_from_expr(&mut binary.lhs, target_id)? {
                return Ok(Some(extracted));
            }
            extract_from_expr(&mut binary.rhs, target_id)
        }
        Expr::Unary(unary) => {
            if expr_id(&unary.expr).is_some_and(|id| id == target_id) {
                return Err(required_source_error(target_id, "expr"));
            }
            extract_from_expr(&mut unary.expr, target_id)
        }
        Expr::Call(call) => {
            if expr_id(&call.callee).is_some_and(|id| id == target_id) {
                return Err(required_source_error(target_id, "callee"));
            }
            if let Some(extracted) = extract_from_expr(&mut call.callee, target_id)? {
                return Ok(Some(extracted));
            }
            for arg in &mut call.args {
                if expr_id(arg).is_some_and(|id| id == target_id) {
                    return Err(patch_error(&format!(
                        "move target `{target_id}` is in unsupported slot `args`"
                    )));
                }
                if let Some(extracted) = extract_from_expr(arg, target_id)? {
                    return Ok(Some(extracted));
                }
            }
            Ok(None)
        }
        Expr::Match(match_expr) => {
            if let Some(index) = match_expr
                .arms
                .iter()
                .position(|arm| arm.meta.id == target_id)
            {
                return Ok(Some(ExtractedNode {
                    node: PatchNode::MatchArm(match_expr.arms.remove(index)),
                    closure: None,
                }));
            }
            if expr_id(&match_expr.scrutinee).is_some_and(|id| id == target_id) {
                return Err(required_source_error(target_id, "scrutinee"));
            }
            if let Some(extracted) = extract_from_expr(&mut match_expr.scrutinee, target_id)? {
                return Ok(Some(extracted));
            }
            for arm in &mut match_expr.arms {
                if let Some(extracted) = extract_from_match_arm(arm, target_id)? {
                    return Ok(Some(extracted));
                }
            }
            Ok(None)
        }
        Expr::Block(block) => extract_from_block(block, target_id),
        Expr::Path(_) | Expr::Lit(_) => Ok(None),
    }
}

fn extract_from_match_arm(
    arm: &mut MatchArm,
    target_id: &str,
) -> Result<Option<ExtractedNode>, PatchError> {
    if pattern_id(&arm.pat).is_some_and(|id| id == target_id) {
        return Err(required_source_error(target_id, "pat"));
    }
    if arm
        .guard
        .as_ref()
        .is_some_and(|guard| expr_id(guard).is_some_and(|id| id == target_id))
    {
        return Ok(Some(ExtractedNode {
            node: PatchNode::Expr(
                arm.guard
                    .take()
                    .expect("guard expression must exist when matched"),
            ),
            closure: None,
        }));
    }
    if let Some(guard) = &mut arm.guard {
        if let Some(extracted) = extract_from_expr(guard, target_id)? {
            return Ok(Some(extracted));
        }
    }
    if expr_id(&arm.body).is_some_and(|id| id == target_id) {
        return Err(required_source_error(target_id, "body"));
    }
    extract_from_expr(&mut arm.body, target_id)
}

fn append_attachment_closure(
    file: &mut File,
    dest: &PatchDest,
    target_id: &str,
    closure: AttachmentClosure,
) -> Result<(), PatchError> {
    match (dest, closure) {
        (PatchDest::Ranked(dest), AttachmentClosure::Items(items)) => {
            append_item_closure(file, dest, target_id, items)
        }
        (PatchDest::Ranked(dest), AttachmentClosure::Stmts(stmts)) => {
            append_stmt_closure(file, dest, target_id, stmts)
        }
        (PatchDest::Slot(_), AttachmentClosure::Items(_))
        | (PatchDest::Slot(_), AttachmentClosure::Stmts(_)) => Err(patch_error(
            "cannot move a node with attached docs/comments into a single-child slot",
        )),
    }
}

fn append_item_closure(
    file: &mut File,
    dest: &RankedDest,
    target_id: &str,
    mut closure: Vec<Item>,
) -> Result<(), PatchError> {
    if dest.slot.slot != "items" {
        return Err(patch_error(
            "cannot move item attachments into a non-item ranked slot",
        ));
    }

    let internal_slot = match dest.slot.owner {
        SlotOwner::File => "file_items",
        SlotOwner::Node(_) => "items",
    };

    for item in &mut closure {
        assign_item_slot_and_rank(item, internal_slot, None, true)?;
        item.meta_mut().anchor = Some(target_id.to_owned());
    }

    match &dest.slot.owner {
        SlotOwner::File => {
            file.items.extend(closure);
            Ok(())
        }
        SlotOwner::Node(owner_id) => {
            let mut closure = Some(closure);
            if append_items_to_owner(&mut file.items, owner_id, &mut closure)? {
                Ok(())
            } else {
                Err(patch_error(&format!(
                    "item attachment destination owner `@{owner_id}` was not found"
                )))
            }
        }
    }
}

fn append_items_to_owner(
    items: &mut Vec<Item>,
    owner_id: &str,
    closure: &mut Option<Vec<Item>>,
) -> Result<bool, PatchError> {
    for item in items {
        if item.meta().id == owner_id {
            match item {
                Item::Mod(module) => {
                    module
                        .items
                        .extend(closure.take().expect("item closure should only move once"));
                    return Ok(true);
                }
                _ => {
                    return Err(patch_error(&format!(
                        "owner `@{owner_id}` does not expose an item attachment container"
                    )));
                }
            }
        }
        if let Item::Mod(module) = item {
            if append_items_to_owner(&mut module.items, owner_id, closure)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn append_stmt_closure(
    file: &mut File,
    dest: &RankedDest,
    target_id: &str,
    mut closure: Vec<Stmt>,
) -> Result<(), PatchError> {
    if dest.slot.slot != "body" {
        return Err(patch_error(
            "cannot move statement attachments into a non-body ranked slot",
        ));
    }

    for stmt in &mut closure {
        assign_stmt_slot_and_rank(stmt, "body", None, true)?;
        match stmt {
            Stmt::Doc(node) => node.meta.anchor = Some(target_id.to_owned()),
            Stmt::Comment(node) => node.meta.anchor = Some(target_id.to_owned()),
            _ => {}
        }
    }

    match &dest.slot.owner {
        SlotOwner::File => Err(patch_error(
            "the file root does not expose a statement body slot",
        )),
        SlotOwner::Node(owner_id) => {
            let mut closure = Some(closure);
            if append_stmts_to_owner(&mut file.items, owner_id, &mut closure)? {
                Ok(())
            } else {
                Err(patch_error(&format!(
                    "statement attachment destination owner `@{owner_id}` was not found"
                )))
            }
        }
    }
}

fn append_stmts_to_owner(
    items: &mut Vec<Item>,
    owner_id: &str,
    closure: &mut Option<Vec<Stmt>>,
) -> Result<bool, PatchError> {
    for item in items {
        if item.meta().id == owner_id {
            match item {
                Item::Fn(function) => {
                    function.body.stmts.extend(
                        closure
                            .take()
                            .expect("statement closure should only move once"),
                    );
                    return Ok(true);
                }
                _ => {
                    return Err(patch_error(&format!(
                        "owner `@{owner_id}` does not expose a statement body slot"
                    )));
                }
            }
        }
        match item {
            Item::Mod(module) => {
                if append_stmts_to_owner(&mut module.items, owner_id, closure)? {
                    return Ok(true);
                }
            }
            Item::Fn(function) => {
                if append_stmts_to_block(&mut function.body, owner_id, closure)? {
                    return Ok(true);
                }
            }
            Item::Use(_) | Item::Struct(_) | Item::Enum(_) | Item::Doc(_) | Item::Comment(_) => {}
        }
    }
    Ok(false)
}

fn append_stmts_to_block(
    block: &mut Block,
    owner_id: &str,
    closure: &mut Option<Vec<Stmt>>,
) -> Result<bool, PatchError> {
    if block.meta.as_ref().is_some_and(|meta| meta.id == owner_id) {
        block.stmts.extend(
            closure
                .take()
                .expect("statement closure should only move once"),
        );
        return Ok(true);
    }
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Let(let_stmt) => {
                if append_stmts_to_expr(&mut let_stmt.value, owner_id, closure)? {
                    return Ok(true);
                }
            }
            Stmt::Expr(expr_stmt) => {
                if append_stmts_to_expr(&mut expr_stmt.expr, owner_id, closure)? {
                    return Ok(true);
                }
            }
            Stmt::Item(item) => {
                if append_stmts_to_item(item, owner_id, closure)? {
                    return Ok(true);
                }
            }
            Stmt::Doc(_) | Stmt::Comment(_) => {}
        }
    }
    Ok(false)
}

fn append_stmts_to_expr(
    expr: &mut Expr,
    owner_id: &str,
    closure: &mut Option<Vec<Stmt>>,
) -> Result<bool, PatchError> {
    if expr_id(expr).is_some_and(|id| id == owner_id) {
        match expr {
            Expr::Block(block) => {
                block.stmts.extend(
                    closure
                        .take()
                        .expect("statement closure should only move once"),
                );
                return Ok(true);
            }
            _ => {
                return Err(patch_error(&format!(
                    "owner `@{owner_id}` does not expose a statement body slot"
                )));
            }
        }
    }

    match expr {
        Expr::Group(group) => append_stmts_to_expr(&mut group.expr, owner_id, closure),
        Expr::Binary(binary) => {
            if append_stmts_to_expr(&mut binary.lhs, owner_id, closure)? {
                return Ok(true);
            }
            append_stmts_to_expr(&mut binary.rhs, owner_id, closure)
        }
        Expr::Unary(unary) => append_stmts_to_expr(&mut unary.expr, owner_id, closure),
        Expr::Call(call) => {
            if append_stmts_to_expr(&mut call.callee, owner_id, closure)? {
                return Ok(true);
            }
            for arg in &mut call.args {
                if append_stmts_to_expr(arg, owner_id, closure)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Match(match_expr) => {
            if append_stmts_to_expr(&mut match_expr.scrutinee, owner_id, closure)? {
                return Ok(true);
            }
            for arm in &mut match_expr.arms {
                if let Some(guard) = &mut arm.guard {
                    if append_stmts_to_expr(guard, owner_id, closure)? {
                        return Ok(true);
                    }
                }
                if append_stmts_to_expr(&mut arm.body, owner_id, closure)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Block(block) => append_stmts_to_block(block, owner_id, closure),
        Expr::Path(_) | Expr::Lit(_) => Ok(false),
    }
}

fn append_stmts_to_item(
    item: &mut Item,
    owner_id: &str,
    closure: &mut Option<Vec<Stmt>>,
) -> Result<bool, PatchError> {
    if item.meta().id == owner_id {
        match item {
            Item::Fn(function) => {
                function.body.stmts.extend(
                    closure
                        .take()
                        .expect("statement closure should only move once"),
                );
                return Ok(true);
            }
            _ => {
                return Err(patch_error(&format!(
                    "owner `@{owner_id}` does not expose a statement body slot"
                )));
            }
        }
    }

    match item {
        Item::Mod(module) => append_stmts_to_owner(&mut module.items, owner_id, closure),
        Item::Fn(function) => append_stmts_to_block(&mut function.body, owner_id, closure),
        Item::Use(_) | Item::Struct(_) | Item::Enum(_) | Item::Doc(_) | Item::Comment(_) => {
            Ok(false)
        }
    }
}

fn required_source_error(target_id: &str, slot: &str) -> PatchError {
    patch_error(&format!(
        "move target `{target_id}` cannot be removed from required slot `{slot}`"
    ))
}
