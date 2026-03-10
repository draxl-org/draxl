use crate::error::{patch_error, PatchError};
use crate::model::{PatchPath, PatchValue};
use crate::schema::{
    clearable_path_spec, find_node_kind, invalid_clear_path_message, invalid_set_path_message,
    path_spec, value_kind_label, ValueKind,
};
use draxl_ast::{BinaryOp, Expr, File, Item, Pattern, Stmt, Type, UnaryOp};

pub(super) fn apply_set(
    file: &mut File,
    path: PatchPath,
    value: PatchValue,
) -> Result<(), PatchError> {
    let target_kind = find_node_kind(file, &path.node_id).ok_or_else(|| {
        patch_error(&format!(
            "set target `@{}.{}` was not found",
            path.node_id,
            path.segments.join(".")
        ))
    })?;
    let segment = single_path_segment(&path)?;
    let spec = path_spec(target_kind, segment).ok_or_else(|| {
        patch_error(&invalid_set_path_message(
            &path.node_id,
            segment,
            target_kind,
        ))
    })?;
    expect_value_kind(&value, spec.value_kind, &path.node_id, segment)?;
    if apply_set_in_items(&mut file.items, &path.node_id, &path.segments, &value)? {
        Ok(())
    } else {
        Err(patch_error(&format!(
            "set target `@{}.{}` was not found",
            path.node_id,
            path.segments.join(".")
        )))
    }
}

pub(super) fn apply_clear(file: &mut File, path: PatchPath) -> Result<(), PatchError> {
    let target_kind = find_node_kind(file, &path.node_id).ok_or_else(|| {
        patch_error(&format!(
            "clear target `@{}.{}` was not found or is not clearable",
            path.node_id,
            path.segments.join(".")
        ))
    })?;
    let segment = single_path_segment(&path)?;
    clearable_path_spec(target_kind, segment).ok_or_else(|| {
        patch_error(&invalid_clear_path_message(
            &path.node_id,
            segment,
            target_kind,
        ))
    })?;
    if apply_clear_in_items(&mut file.items, &path.node_id, &path.segments)? {
        Ok(())
    } else {
        Err(patch_error(&format!(
            "clear target `@{}.{}` was not found or is not clearable",
            path.node_id,
            path.segments.join(".")
        )))
    }
}

fn apply_set_in_items(
    items: &mut Vec<Item>,
    node_id: &str,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    for item in items {
        if item.meta().id == node_id {
            return apply_set_in_item(item, segments, value);
        }
        if recurse_set_in_item(item, node_id, segments, value)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn recurse_set_in_item(
    item: &mut Item,
    node_id: &str,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    match item {
        Item::Mod(module) => apply_set_in_items(&mut module.items, node_id, segments, value),
        Item::Struct(strukt) => {
            for field in &mut strukt.fields {
                if field.meta.id == node_id {
                    return apply_set_in_field(field, segments, value);
                }
                if field.ty.meta().id == node_id {
                    return apply_set_in_type(&mut field.ty, segments, value);
                }
            }
            Ok(false)
        }
        Item::Enum(enm) => {
            for variant in &mut enm.variants {
                if variant.meta.id == node_id {
                    return apply_set_in_variant(variant, segments, value);
                }
            }
            Ok(false)
        }
        Item::Fn(function) => {
            for param in &mut function.params {
                if param.meta.id == node_id {
                    return apply_set_in_param(param, segments, value);
                }
                if param.ty.meta().id == node_id {
                    return apply_set_in_type(&mut param.ty, segments, value);
                }
            }
            if function
                .ret_ty
                .as_ref()
                .is_some_and(|ret_ty| ret_ty.meta().id == node_id)
            {
                return apply_set_in_type(
                    function
                        .ret_ty
                        .as_mut()
                        .expect("return type must exist when matched"),
                    segments,
                    value,
                );
            }
            apply_set_in_block(&mut function.body, node_id, segments, value)
        }
        Item::Use(_) | Item::Doc(_) | Item::Comment(_) => Ok(false),
    }
}

fn apply_set_in_block(
    block: &mut draxl_ast::Block,
    node_id: &str,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    for stmt in &mut block.stmts {
        if let Some(id) = stmt.meta().map(|meta| meta.id.as_str()) {
            if id == node_id {
                return apply_set_in_stmt(stmt, segments, value);
            }
        }
        if recurse_set_in_stmt(stmt, node_id, segments, value)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn recurse_set_in_stmt(
    stmt: &mut Stmt,
    node_id: &str,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    match stmt {
        Stmt::Let(let_stmt) => {
            if let_stmt.pat.meta().is_some_and(|meta| meta.id == node_id) {
                return apply_set_in_pattern(&mut let_stmt.pat, segments, value);
            }
            if let_stmt.value.meta().is_some_and(|meta| meta.id == node_id) {
                return apply_set_expr_field(&mut let_stmt.value, segments, value);
            }
            apply_set_in_expr(&mut let_stmt.value, node_id, segments, value)
        }
        Stmt::Expr(expr_stmt) => {
            if expr_stmt.expr.meta().is_some_and(|meta| meta.id == node_id) {
                return apply_set_expr_field(&mut expr_stmt.expr, segments, value);
            }
            apply_set_in_expr(&mut expr_stmt.expr, node_id, segments, value)
        }
        Stmt::Item(item) => recurse_set_in_item(item, node_id, segments, value),
        Stmt::Doc(node) if node.meta.id == node_id => apply_set_in_doc(node, segments, value),
        Stmt::Comment(node) if node.meta.id == node_id => {
            apply_set_in_comment(node, segments, value)
        }
        Stmt::Doc(_) | Stmt::Comment(_) => Ok(false),
    }
}

fn apply_set_in_expr(
    expr: &mut Expr,
    node_id: &str,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    if expr.meta().is_some_and(|meta| meta.id == node_id) {
        return apply_set_expr_field(expr, segments, value);
    }

    match expr {
        Expr::Group(group) => apply_set_in_expr(&mut group.expr, node_id, segments, value),
        Expr::Binary(binary) => {
            if apply_set_in_expr(&mut binary.lhs, node_id, segments, value)? {
                return Ok(true);
            }
            apply_set_in_expr(&mut binary.rhs, node_id, segments, value)
        }
        Expr::Unary(unary) => apply_set_in_expr(&mut unary.expr, node_id, segments, value),
        Expr::Call(call) => {
            if apply_set_in_expr(&mut call.callee, node_id, segments, value)? {
                return Ok(true);
            }
            for arg in &mut call.args {
                if apply_set_in_expr(arg, node_id, segments, value)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Match(match_expr) => {
            if apply_set_in_expr(&mut match_expr.scrutinee, node_id, segments, value)? {
                return Ok(true);
            }
            for arm in &mut match_expr.arms {
                if arm.meta.id == node_id {
                    return apply_set_in_match_arm(arm, segments, value);
                }
                if arm.pat.meta().is_some_and(|meta| meta.id == node_id) {
                    return apply_set_in_pattern(&mut arm.pat, segments, value);
                }
                if let Some(guard) = &mut arm.guard {
                    if guard.meta().is_some_and(|meta| meta.id == node_id) {
                        return apply_set_expr_field(guard, segments, value);
                    }
                    if apply_set_in_expr(guard, node_id, segments, value)? {
                        return Ok(true);
                    }
                }
                if arm.body.meta().is_some_and(|meta| meta.id == node_id) {
                    return apply_set_expr_field(&mut arm.body, segments, value);
                }
                if apply_set_in_expr(&mut arm.body, node_id, segments, value)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Block(block) => apply_set_in_block(block, node_id, segments, value),
        Expr::Path(_) | Expr::Lit(_) => Ok(false),
    }
}

fn apply_set_in_item(
    item: &mut Item,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    match item {
        Item::Mod(node) => set_name_field(&mut node.name, segments, value),
        Item::Struct(node) => set_name_field(&mut node.name, segments, value),
        Item::Enum(node) => set_name_field(&mut node.name, segments, value),
        Item::Fn(node) => set_name_field(&mut node.name, segments, value),
        Item::Doc(node) => apply_set_in_doc(node, segments, value),
        Item::Comment(node) => apply_set_in_comment(node, segments, value),
        Item::Use(_) => Ok(false),
    }
}

fn apply_set_in_stmt(
    stmt: &mut Stmt,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    match stmt {
        Stmt::Expr(node) if segments == ["semi"] => {
            node.has_semi = expect_bool(value, "semi")?;
            Ok(true)
        }
        Stmt::Doc(node) => apply_set_in_doc(node, segments, value),
        Stmt::Comment(node) => apply_set_in_comment(node, segments, value),
        Stmt::Let(_) | Stmt::Expr(_) | Stmt::Item(_) => Ok(false),
    }
}

fn apply_set_in_field(
    field: &mut draxl_ast::Field,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    set_name_field(&mut field.name, segments, value)
}

fn apply_set_in_variant(
    variant: &mut draxl_ast::Variant,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    set_name_field(&mut variant.name, segments, value)
}

fn apply_set_in_param(
    param: &mut draxl_ast::Param,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    set_name_field(&mut param.name, segments, value)
}

fn apply_set_in_match_arm(
    _arm: &mut draxl_ast::MatchArm,
    _segments: &[String],
    _value: &PatchValue,
) -> Result<bool, PatchError> {
    Ok(false)
}

fn apply_set_in_pattern(
    pattern: &mut Pattern,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    match pattern {
        Pattern::Ident(node) => set_name_field(&mut node.name, segments, value),
        Pattern::Wild(_) => Ok(false),
    }
}

fn apply_set_in_type(
    ty: &mut Type,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    let _ = (ty, segments, value);
    Ok(false)
}

fn apply_set_in_doc(
    node: &mut draxl_ast::DocNode,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    set_text_field(&mut node.text, segments, value)
}

fn apply_set_in_comment(
    node: &mut draxl_ast::CommentNode,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    set_text_field(&mut node.text, segments, value)
}

fn apply_set_expr_field(
    expr: &mut Expr,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    if segments.len() != 1 {
        return Ok(false);
    }

    match (expr, segments[0].as_str()) {
        (Expr::Binary(node), "op") => {
            node.op = parse_binary_op(value)?;
            Ok(true)
        }
        (Expr::Unary(node), "op") => {
            node.op = parse_unary_op(value)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn set_name_field(
    target: &mut String,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    if segments == ["name"] {
        *target = expect_ident(value, "name")?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn set_text_field(
    target: &mut String,
    segments: &[String],
    value: &PatchValue,
) -> Result<bool, PatchError> {
    if segments == ["text"] {
        *target = expect_string(value, "text")?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn apply_clear_in_items(
    items: &mut Vec<Item>,
    node_id: &str,
    segments: &[String],
) -> Result<bool, PatchError> {
    for item in items {
        if item.meta().id == node_id {
            return apply_clear_in_item(item, segments);
        }
        if recurse_clear_in_item(item, node_id, segments)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn recurse_clear_in_item(
    item: &mut Item,
    node_id: &str,
    segments: &[String],
) -> Result<bool, PatchError> {
    match item {
        Item::Mod(module) => apply_clear_in_items(&mut module.items, node_id, segments),
        Item::Struct(strukt) => {
            for field in &mut strukt.fields {
                if field.meta.id == node_id {
                    return apply_clear_in_field(field, segments);
                }
                if field.ty.meta().id == node_id {
                    return apply_clear_in_type(&mut field.ty, segments);
                }
            }
            Ok(false)
        }
        Item::Enum(enm) => {
            for variant in &mut enm.variants {
                if variant.meta.id == node_id {
                    return apply_clear_in_variant(variant, segments);
                }
            }
            Ok(false)
        }
        Item::Fn(function) => {
            for param in &mut function.params {
                if param.meta.id == node_id {
                    return apply_clear_in_param(param, segments);
                }
                if param.ty.meta().id == node_id {
                    return apply_clear_in_type(&mut param.ty, segments);
                }
            }
            if function
                .ret_ty
                .as_ref()
                .is_some_and(|ret_ty| ret_ty.meta().id == node_id)
            {
                return apply_clear_in_type(
                    function
                        .ret_ty
                        .as_mut()
                        .expect("return type must exist when matched"),
                    segments,
                );
            }
            apply_clear_in_block(&mut function.body, node_id, segments)
        }
        Item::Use(_) | Item::Doc(_) | Item::Comment(_) => Ok(false),
    }
}

fn apply_clear_in_block(
    block: &mut draxl_ast::Block,
    node_id: &str,
    segments: &[String],
) -> Result<bool, PatchError> {
    for stmt in &mut block.stmts {
        if let Some(id) = stmt.meta().map(|meta| meta.id.as_str()) {
            if id == node_id {
                return apply_clear_in_stmt(stmt, segments);
            }
        }
        if recurse_clear_in_stmt(stmt, node_id, segments)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn recurse_clear_in_stmt(
    stmt: &mut Stmt,
    node_id: &str,
    segments: &[String],
) -> Result<bool, PatchError> {
    match stmt {
        Stmt::Let(let_stmt) => {
            if let_stmt.pat.meta().is_some_and(|meta| meta.id == node_id) {
                return apply_clear_in_pattern(&mut let_stmt.pat, segments);
            }
            if let_stmt.value.meta().is_some_and(|meta| meta.id == node_id) {
                return apply_clear_expr_field(&mut let_stmt.value, segments);
            }
            apply_clear_in_expr(&mut let_stmt.value, node_id, segments)
        }
        Stmt::Expr(expr_stmt) => {
            if expr_stmt.expr.meta().is_some_and(|meta| meta.id == node_id) {
                return apply_clear_expr_field(&mut expr_stmt.expr, segments);
            }
            apply_clear_in_expr(&mut expr_stmt.expr, node_id, segments)
        }
        Stmt::Item(item) => recurse_clear_in_item(item, node_id, segments),
        Stmt::Doc(node) if node.meta.id == node_id => apply_clear_in_doc(node, segments),
        Stmt::Comment(node) if node.meta.id == node_id => apply_clear_in_comment(node, segments),
        Stmt::Doc(_) | Stmt::Comment(_) => Ok(false),
    }
}

fn apply_clear_in_expr(
    expr: &mut Expr,
    node_id: &str,
    segments: &[String],
) -> Result<bool, PatchError> {
    if expr.meta().is_some_and(|meta| meta.id == node_id) {
        return apply_clear_expr_field(expr, segments);
    }

    match expr {
        Expr::Group(group) => apply_clear_in_expr(&mut group.expr, node_id, segments),
        Expr::Binary(binary) => {
            if apply_clear_in_expr(&mut binary.lhs, node_id, segments)? {
                return Ok(true);
            }
            apply_clear_in_expr(&mut binary.rhs, node_id, segments)
        }
        Expr::Unary(unary) => apply_clear_in_expr(&mut unary.expr, node_id, segments),
        Expr::Call(call) => {
            if apply_clear_in_expr(&mut call.callee, node_id, segments)? {
                return Ok(true);
            }
            for arg in &mut call.args {
                if apply_clear_in_expr(arg, node_id, segments)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Match(match_expr) => {
            if apply_clear_in_expr(&mut match_expr.scrutinee, node_id, segments)? {
                return Ok(true);
            }
            for arm in &mut match_expr.arms {
                if arm.meta.id == node_id {
                    return Ok(false);
                }
                if arm.pat.meta().is_some_and(|meta| meta.id == node_id) {
                    return apply_clear_in_pattern(&mut arm.pat, segments);
                }
                if let Some(guard) = &mut arm.guard {
                    if guard.meta().is_some_and(|meta| meta.id == node_id) {
                        return apply_clear_expr_field(guard, segments);
                    }
                    if apply_clear_in_expr(guard, node_id, segments)? {
                        return Ok(true);
                    }
                }
                if arm.body.meta().is_some_and(|meta| meta.id == node_id) {
                    return apply_clear_expr_field(&mut arm.body, segments);
                }
                if apply_clear_in_expr(&mut arm.body, node_id, segments)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expr::Block(block) => apply_clear_in_block(block, node_id, segments),
        Expr::Path(_) | Expr::Lit(_) => Ok(false),
    }
}

fn apply_clear_in_item(item: &mut Item, segments: &[String]) -> Result<bool, PatchError> {
    match item {
        Item::Doc(node) => apply_clear_in_doc(node, segments),
        Item::Comment(node) => apply_clear_in_comment(node, segments),
        Item::Mod(_) | Item::Use(_) | Item::Struct(_) | Item::Enum(_) | Item::Fn(_) => Ok(false),
    }
}

fn apply_clear_in_stmt(stmt: &mut Stmt, segments: &[String]) -> Result<bool, PatchError> {
    match stmt {
        Stmt::Expr(node) if segments == ["semi"] => {
            node.has_semi = false;
            Ok(true)
        }
        Stmt::Doc(node) => apply_clear_in_doc(node, segments),
        Stmt::Comment(node) => apply_clear_in_comment(node, segments),
        Stmt::Let(_) | Stmt::Expr(_) | Stmt::Item(_) => Ok(false),
    }
}

fn apply_clear_in_field(
    _field: &mut draxl_ast::Field,
    _segments: &[String],
) -> Result<bool, PatchError> {
    Ok(false)
}

fn apply_clear_in_variant(
    _variant: &mut draxl_ast::Variant,
    _segments: &[String],
) -> Result<bool, PatchError> {
    Ok(false)
}

fn apply_clear_in_param(
    _param: &mut draxl_ast::Param,
    _segments: &[String],
) -> Result<bool, PatchError> {
    Ok(false)
}

fn apply_clear_in_pattern(
    _pattern: &mut Pattern,
    _segments: &[String],
) -> Result<bool, PatchError> {
    Ok(false)
}

fn apply_clear_in_type(_ty: &mut Type, _segments: &[String]) -> Result<bool, PatchError> {
    Ok(false)
}

fn apply_clear_in_doc(
    node: &mut draxl_ast::DocNode,
    segments: &[String],
) -> Result<bool, PatchError> {
    if segments == ["text"] {
        node.text.clear();
        Ok(true)
    } else {
        Ok(false)
    }
}

fn apply_clear_in_comment(
    node: &mut draxl_ast::CommentNode,
    segments: &[String],
) -> Result<bool, PatchError> {
    if segments == ["text"] {
        node.text.clear();
        Ok(true)
    } else {
        Ok(false)
    }
}

fn apply_clear_expr_field(expr: &mut Expr, segments: &[String]) -> Result<bool, PatchError> {
    if segments.len() != 1 {
        return Ok(false);
    }

    match (expr, segments[0].as_str()) {
        (Expr::Unary(node), "op") => {
            node.op = UnaryOp::Neg;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn expect_ident(value: &PatchValue, field: &str) -> Result<String, PatchError> {
    match value {
        PatchValue::Ident(value) => Ok(value.clone()),
        _ => Err(patch_error(&format!(
            "field `{field}` expects an identifier value"
        ))),
    }
}

fn expect_string(value: &PatchValue, field: &str) -> Result<String, PatchError> {
    match value {
        PatchValue::Str(value) => Ok(value.clone()),
        _ => Err(patch_error(&format!(
            "field `{field}` expects a string value"
        ))),
    }
}

fn expect_bool(value: &PatchValue, field: &str) -> Result<bool, PatchError> {
    match value {
        PatchValue::Bool(value) => Ok(*value),
        _ => Err(patch_error(&format!(
            "field `{field}` expects a boolean value"
        ))),
    }
}

fn parse_binary_op(value: &PatchValue) -> Result<BinaryOp, PatchError> {
    match value {
        PatchValue::Ident(value) if value == "add" => Ok(BinaryOp::Add),
        PatchValue::Ident(value) if value == "sub" => Ok(BinaryOp::Sub),
        PatchValue::Ident(value) if value == "lt" => Ok(BinaryOp::Lt),
        _ => Err(patch_error(
            "binary `op` expects one of `add`, `sub`, or `lt`",
        )),
    }
}

fn parse_unary_op(value: &PatchValue) -> Result<UnaryOp, PatchError> {
    match value {
        PatchValue::Ident(value) if value == "neg" => Ok(UnaryOp::Neg),
        _ => Err(patch_error("unary `op` expects `neg`")),
    }
}

fn single_path_segment(path: &PatchPath) -> Result<&str, PatchError> {
    if path.segments.len() != 1 {
        return Err(patch_error(
            "only single-segment scalar patch paths are supported in the current Rust profile",
        ));
    }
    Ok(path.segments[0].as_str())
}

fn expect_value_kind(
    value: &PatchValue,
    expected: ValueKind,
    node_id: &str,
    segment: &str,
) -> Result<(), PatchError> {
    match (value, expected) {
        (PatchValue::Ident(_), ValueKind::Ident)
        | (PatchValue::Str(_), ValueKind::Str)
        | (PatchValue::Bool(_), ValueKind::Bool) => Ok(()),
        _ => Err(patch_error(&format!(
            "path `@{node_id}.{segment}` expects {}",
            value_kind_label(expected)
        ))),
    }
}
