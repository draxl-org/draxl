use draxl_ast::{Block, Expr, File, Item, MatchArm, Pattern, Stmt, StmtLet, Type};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub(crate) struct TreeContext {
    nodes: HashMap<String, NodeContext>,
}

impl TreeContext {
    pub(crate) fn build(file: &File) -> Self {
        let mut context = Self::default();
        for item in &file.items {
            visit_item(
                &mut context,
                item,
                None,
                None,
                false,
                None,
                false,
                None,
                None,
                false,
            );
        }
        context
    }

    pub(crate) fn node(&self, node_id: &str) -> Option<&NodeContext> {
        self.nodes.get(node_id)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NodeContext {
    #[allow(dead_code)]
    pub parent_id: Option<String>,
    pub enclosing_fn: Option<String>,
    pub in_fn_body: bool,
    pub enclosing_param: Option<String>,
    pub param_type_region: bool,
    pub param_name: Option<String>,
    pub enclosing_let: Option<String>,
    pub let_region: Option<LetRegion>,
    pub is_let_binding: bool,
    pub is_let_stmt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LetRegion {
    Pattern,
    Init,
}

fn visit_item(
    context: &mut TreeContext,
    item: &Item,
    parent_id: Option<&str>,
    enclosing_fn: Option<&str>,
    in_fn_body: bool,
    enclosing_param: Option<&str>,
    param_type_region: bool,
    enclosing_let: Option<&str>,
    let_region: Option<LetRegion>,
    is_let_binding: bool,
) {
    let item_id = item.meta().id.as_str();
    register_node(
        context,
        item_id,
        parent_id,
        enclosing_fn,
        in_fn_body,
        enclosing_param,
        param_type_region,
        None,
        enclosing_let,
        let_region,
        is_let_binding,
        false,
    );

    match item {
        Item::Mod(node) => {
            for child in &node.items {
                visit_item(
                    context,
                    child,
                    Some(item_id),
                    enclosing_fn,
                    in_fn_body,
                    enclosing_param,
                    param_type_region,
                    enclosing_let,
                    let_region,
                    false,
                );
            }
        }
        Item::Struct(node) => {
            for field in &node.fields {
                visit_type(
                    context,
                    &field.ty,
                    Some(field.meta.id.as_str()),
                    enclosing_fn,
                    in_fn_body,
                    enclosing_param,
                    param_type_region,
                    enclosing_let,
                    let_region,
                );
            }
        }
        Item::Enum(_) | Item::Use(_) | Item::Doc(_) | Item::Comment(_) => {}
        Item::Fn(node) => {
            for param in &node.params {
                register_node(
                    context,
                    &param.meta.id,
                    Some(item_id),
                    Some(item_id),
                    false,
                    Some(param.meta.id.as_str()),
                    false,
                    Some(param.name.as_str()),
                    None,
                    None,
                    false,
                    false,
                );
                visit_type(
                    context,
                    &param.ty,
                    Some(param.meta.id.as_str()),
                    Some(item_id),
                    false,
                    Some(param.meta.id.as_str()),
                    true,
                    None,
                    None,
                );
            }

            if let Some(ret_ty) = &node.ret_ty {
                visit_type(
                    context,
                    ret_ty,
                    Some(item_id),
                    Some(item_id),
                    false,
                    None,
                    false,
                    None,
                    None,
                );
            }

            visit_block(
                context,
                &node.body,
                Some(item_id),
                Some(item_id),
                true,
                None,
                false,
                None,
                None,
            );
        }
    }
}

fn visit_block(
    context: &mut TreeContext,
    block: &Block,
    parent_id: Option<&str>,
    enclosing_fn: Option<&str>,
    in_fn_body: bool,
    enclosing_param: Option<&str>,
    param_type_region: bool,
    enclosing_let: Option<&str>,
    let_region: Option<LetRegion>,
) {
    let block_parent = match &block.meta {
        Some(meta) => {
            register_node(
                context,
                &meta.id,
                parent_id,
                enclosing_fn,
                in_fn_body,
                enclosing_param,
                param_type_region,
                None,
                enclosing_let,
                let_region,
                false,
                false,
            );
            Some(meta.id.as_str())
        }
        None => parent_id,
    };

    for stmt in &block.stmts {
        visit_stmt(
            context,
            stmt,
            block_parent,
            enclosing_fn,
            in_fn_body,
            enclosing_param,
            param_type_region,
            enclosing_let,
            let_region,
        );
    }
}

fn visit_stmt(
    context: &mut TreeContext,
    stmt: &Stmt,
    parent_id: Option<&str>,
    enclosing_fn: Option<&str>,
    in_fn_body: bool,
    enclosing_param: Option<&str>,
    param_type_region: bool,
    enclosing_let: Option<&str>,
    let_region: Option<LetRegion>,
) {
    match stmt {
        Stmt::Let(node) => visit_let_stmt(
            context,
            node,
            parent_id,
            enclosing_fn,
            in_fn_body,
            enclosing_param,
            param_type_region,
            enclosing_let,
            let_region,
        ),
        Stmt::Expr(node) => {
            register_node(
                context,
                &node.meta.id,
                parent_id,
                enclosing_fn,
                in_fn_body,
                enclosing_param,
                param_type_region,
                None,
                enclosing_let,
                let_region,
                false,
                false,
            );
            visit_expr(
                context,
                &node.expr,
                Some(node.meta.id.as_str()),
                enclosing_fn,
                in_fn_body,
                enclosing_param,
                param_type_region,
                enclosing_let,
                let_region,
            );
        }
        Stmt::Item(item) => visit_item(
            context,
            item,
            parent_id,
            enclosing_fn,
            in_fn_body,
            enclosing_param,
            param_type_region,
            enclosing_let,
            let_region,
            false,
        ),
        Stmt::Doc(node) => register_node(
            context,
            &node.meta.id,
            parent_id,
            enclosing_fn,
            in_fn_body,
            enclosing_param,
            param_type_region,
            None,
            enclosing_let,
            let_region,
            false,
            false,
        ),
        Stmt::Comment(node) => register_node(
            context,
            &node.meta.id,
            parent_id,
            enclosing_fn,
            in_fn_body,
            enclosing_param,
            param_type_region,
            None,
            enclosing_let,
            let_region,
            false,
            false,
        ),
    }
}

fn visit_let_stmt(
    context: &mut TreeContext,
    node: &StmtLet,
    parent_id: Option<&str>,
    enclosing_fn: Option<&str>,
    in_fn_body: bool,
    enclosing_param: Option<&str>,
    param_type_region: bool,
    enclosing_let: Option<&str>,
    let_region: Option<LetRegion>,
) {
    let let_id = node.meta.id.as_str();
    register_node(
        context,
        let_id,
        parent_id,
        enclosing_fn,
        in_fn_body,
        enclosing_param,
        param_type_region,
        None,
        enclosing_let,
        let_region,
        false,
        true,
    );
    visit_pattern(
        context,
        &node.pat,
        Some(let_id),
        enclosing_fn,
        in_fn_body,
        enclosing_param,
        param_type_region,
        Some(let_id),
        Some(LetRegion::Pattern),
        true,
    );
    visit_expr(
        context,
        &node.value,
        Some(let_id),
        enclosing_fn,
        in_fn_body,
        enclosing_param,
        param_type_region,
        Some(let_id),
        Some(LetRegion::Init),
    );
}

fn visit_pattern(
    context: &mut TreeContext,
    pattern: &Pattern,
    parent_id: Option<&str>,
    enclosing_fn: Option<&str>,
    in_fn_body: bool,
    enclosing_param: Option<&str>,
    param_type_region: bool,
    enclosing_let: Option<&str>,
    let_region: Option<LetRegion>,
    is_let_binding: bool,
) {
    match pattern {
        Pattern::Ident(node) => {
            if let Some(meta) = &node.meta {
                register_node(
                    context,
                    &meta.id,
                    parent_id,
                    enclosing_fn,
                    in_fn_body,
                    enclosing_param,
                    param_type_region,
                    None,
                    enclosing_let,
                    let_region,
                    is_let_binding,
                    false,
                );
            }
        }
        Pattern::Wild(node) => {
            if let Some(meta) = &node.meta {
                register_node(
                    context,
                    &meta.id,
                    parent_id,
                    enclosing_fn,
                    in_fn_body,
                    enclosing_param,
                    param_type_region,
                    None,
                    enclosing_let,
                    let_region,
                    false,
                    false,
                );
            }
        }
    }
}

fn visit_expr(
    context: &mut TreeContext,
    expr: &Expr,
    parent_id: Option<&str>,
    enclosing_fn: Option<&str>,
    in_fn_body: bool,
    enclosing_param: Option<&str>,
    param_type_region: bool,
    enclosing_let: Option<&str>,
    let_region: Option<LetRegion>,
) {
    let expr_parent = match expr.meta() {
        Some(meta) => {
            register_node(
                context,
                &meta.id,
                parent_id,
                enclosing_fn,
                in_fn_body,
                enclosing_param,
                param_type_region,
                None,
                enclosing_let,
                let_region,
                false,
                false,
            );
            Some(meta.id.as_str())
        }
        None => parent_id,
    };

    match expr {
        Expr::Path(_) | Expr::Lit(_) => {}
        Expr::Group(node) => {
            visit_expr(
                context,
                &node.expr,
                expr_parent,
                enclosing_fn,
                in_fn_body,
                enclosing_param,
                param_type_region,
                enclosing_let,
                let_region,
            );
        }
        Expr::Binary(node) => {
            visit_expr(
                context,
                &node.lhs,
                expr_parent,
                enclosing_fn,
                in_fn_body,
                enclosing_param,
                param_type_region,
                enclosing_let,
                let_region,
            );
            visit_expr(
                context,
                &node.rhs,
                expr_parent,
                enclosing_fn,
                in_fn_body,
                enclosing_param,
                param_type_region,
                enclosing_let,
                let_region,
            );
        }
        Expr::Unary(node) => {
            visit_expr(
                context,
                &node.expr,
                expr_parent,
                enclosing_fn,
                in_fn_body,
                enclosing_param,
                param_type_region,
                enclosing_let,
                let_region,
            );
        }
        Expr::Call(node) => {
            visit_expr(
                context,
                &node.callee,
                expr_parent,
                enclosing_fn,
                in_fn_body,
                enclosing_param,
                param_type_region,
                enclosing_let,
                let_region,
            );
            for arg in &node.args {
                visit_expr(
                    context,
                    arg,
                    expr_parent,
                    enclosing_fn,
                    in_fn_body,
                    enclosing_param,
                    param_type_region,
                    enclosing_let,
                    let_region,
                );
            }
        }
        Expr::Match(node) => {
            visit_expr(
                context,
                &node.scrutinee,
                expr_parent,
                enclosing_fn,
                in_fn_body,
                enclosing_param,
                param_type_region,
                enclosing_let,
                let_region,
            );
            for arm in &node.arms {
                visit_match_arm(
                    context,
                    arm,
                    expr_parent,
                    enclosing_fn,
                    in_fn_body,
                    enclosing_param,
                    param_type_region,
                    enclosing_let,
                    let_region,
                );
            }
        }
        Expr::Block(block) => visit_block(
            context,
            block,
            expr_parent,
            enclosing_fn,
            in_fn_body,
            enclosing_param,
            param_type_region,
            enclosing_let,
            let_region,
        ),
    }
}

fn visit_match_arm(
    context: &mut TreeContext,
    arm: &MatchArm,
    parent_id: Option<&str>,
    enclosing_fn: Option<&str>,
    in_fn_body: bool,
    enclosing_param: Option<&str>,
    param_type_region: bool,
    enclosing_let: Option<&str>,
    let_region: Option<LetRegion>,
) {
    let arm_id = arm.meta.id.as_str();
    register_node(
        context,
        arm_id,
        parent_id,
        enclosing_fn,
        in_fn_body,
        enclosing_param,
        param_type_region,
        None,
        enclosing_let,
        let_region,
        false,
        false,
    );
    visit_pattern(
        context,
        &arm.pat,
        Some(arm_id),
        enclosing_fn,
        in_fn_body,
        enclosing_param,
        param_type_region,
        enclosing_let,
        let_region,
        false,
    );
    if let Some(guard) = &arm.guard {
        visit_expr(
            context,
            guard,
            Some(arm_id),
            enclosing_fn,
            in_fn_body,
            enclosing_param,
            param_type_region,
            enclosing_let,
            let_region,
        );
    }
    visit_expr(
        context,
        &arm.body,
        Some(arm_id),
        enclosing_fn,
        in_fn_body,
        enclosing_param,
        param_type_region,
        enclosing_let,
        let_region,
    );
}

fn visit_type(
    context: &mut TreeContext,
    ty: &Type,
    parent_id: Option<&str>,
    enclosing_fn: Option<&str>,
    in_fn_body: bool,
    enclosing_param: Option<&str>,
    param_type_region: bool,
    enclosing_let: Option<&str>,
    let_region: Option<LetRegion>,
) {
    register_node(
        context,
        &ty.meta().id,
        parent_id,
        enclosing_fn,
        in_fn_body,
        enclosing_param,
        param_type_region,
        None,
        enclosing_let,
        let_region,
        false,
        false,
    );
}

fn register_node(
    context: &mut TreeContext,
    node_id: &str,
    parent_id: Option<&str>,
    enclosing_fn: Option<&str>,
    in_fn_body: bool,
    enclosing_param: Option<&str>,
    param_type_region: bool,
    param_name: Option<&str>,
    enclosing_let: Option<&str>,
    let_region: Option<LetRegion>,
    is_let_binding: bool,
    is_let_stmt: bool,
) {
    context.nodes.insert(
        node_id.to_owned(),
        NodeContext {
            parent_id: parent_id.map(str::to_owned),
            enclosing_fn: enclosing_fn.map(str::to_owned),
            in_fn_body,
            enclosing_param: enclosing_param.map(str::to_owned),
            param_type_region,
            param_name: param_name.map(str::to_owned),
            enclosing_let: enclosing_let.map(str::to_owned),
            let_region,
            is_let_binding,
            is_let_stmt,
        },
    );
}
