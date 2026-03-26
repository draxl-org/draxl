use crate::canonicalize_file;
use draxl_ast::{
    BinaryOp, CommentNode, DocNode, Expr, File, Item, ItemFn, Literal, Meta, Param, Path, Pattern,
    Stmt, Type, UnaryOp, UseTree, Variant,
};

/// Prints a file into canonical Draxl source.
pub fn print_file(file: &File) -> String {
    let canonical = canonicalize_file(file);
    let mut out = String::new();
    write_item_list(&canonical.items, &mut out, 0);
    out.push('\n');
    out
}

fn write_item_list(items: &[Item], out: &mut String, indent: usize) {
    for (index, item) in items.iter().enumerate() {
        if index > 0 && needs_item_gap(items, index - 1, index) {
            out.push('\n');
        }
        write_item(item, out, indent);
    }
}

fn write_item(item: &Item, out: &mut String, indent: usize) {
    match item {
        Item::Mod(node) => {
            indent_line(out, indent);
            out.push_str(&format_meta_prefix(&node.meta));
            out.push_str("mod ");
            out.push_str(&node.name);
            out.push_str(" {");
            if node.items.is_empty() {
                out.push_str("}\n");
                return;
            }
            out.push('\n');
            write_item_list(&node.items, out, indent + 1);
            indent_line(out, indent);
            out.push_str("}\n");
        }
        Item::Use(node) => {
            indent_line(out, indent);
            out.push_str(&format_meta_prefix(&node.meta));
            out.push_str("use ");
            out.push_str(&format_use_tree(&node.tree));
            out.push_str(";\n");
        }
        Item::Struct(node) => {
            indent_line(out, indent);
            out.push_str(&format_meta_prefix(&node.meta));
            out.push_str("struct ");
            out.push_str(&node.name);
            out.push_str(" {\n");
            for field in &node.fields {
                indent_line(out, indent + 1);
                out.push_str(&format_meta_prefix(&field.meta));
                out.push_str(&field.name);
                out.push_str(": ");
                out.push_str(&format_type(&field.ty));
                out.push_str(",\n");
            }
            indent_line(out, indent);
            out.push_str("}\n");
        }
        Item::Enum(node) => {
            indent_line(out, indent);
            out.push_str(&format_meta_prefix(&node.meta));
            out.push_str("enum ");
            out.push_str(&node.name);
            out.push_str(" {\n");
            for variant in &node.variants {
                write_variant(variant, out, indent + 1);
            }
            indent_line(out, indent);
            out.push_str("}\n");
        }
        Item::Fn(node) => write_fn(node, out, indent),
        Item::Doc(node) => write_doc(node, out, indent),
        Item::Comment(node) => write_comment(node, out, indent),
    }
}

fn write_variant(variant: &Variant, out: &mut String, indent: usize) {
    indent_line(out, indent);
    out.push_str(&format_meta_prefix(&variant.meta));
    out.push_str(&variant.name);
    out.push_str(",\n");
}

fn write_fn(node: &ItemFn, out: &mut String, indent: usize) {
    indent_line(out, indent);
    out.push_str(&format_meta_prefix(&node.meta));
    out.push_str("fn ");
    out.push_str(&node.name);
    out.push('(');
    for (index, param) in node.params.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&format_param(param));
    }
    out.push(')');
    if let Some(ret_ty) = &node.ret_ty {
        out.push_str(" -> ");
        out.push_str(&format_type(ret_ty));
    }
    out.push_str(" {\n");
    write_stmt_list(&node.body.stmts, out, indent + 1);
    indent_line(out, indent);
    out.push_str("}\n");
}

fn format_param(param: &Param) -> String {
    let mut out = format_meta_prefix(&param.meta);
    out.push_str(&param.name);
    out.push_str(": ");
    out.push_str(&format_type(&param.ty));
    out
}

fn write_stmt_list(stmts: &[Stmt], out: &mut String, indent: usize) {
    for stmt in stmts {
        write_stmt(stmt, out, indent);
    }
}

fn write_stmt(stmt: &Stmt, out: &mut String, indent: usize) {
    match stmt {
        Stmt::Let(node) => {
            indent_line(out, indent);
            out.push_str(&format_meta_prefix(&node.meta));
            out.push_str("let ");
            out.push_str(&format_pattern(&node.pat));
            out.push_str(" = ");
            out.push_str(&format_expr(&node.value, indent));
            out.push_str(";\n");
        }
        Stmt::Expr(node) => {
            indent_line(out, indent);
            out.push_str(&format_meta_prefix(&node.meta));
            out.push_str(&format_expr(&node.expr, indent));
            if node.has_semi {
                out.push(';');
            }
            out.push('\n');
        }
        Stmt::Item(item) => write_item(item, out, indent),
        Stmt::Doc(node) => write_doc(node, out, indent),
        Stmt::Comment(node) => write_comment(node, out, indent),
    }
}

fn write_doc(node: &DocNode, out: &mut String, indent: usize) {
    indent_line(out, indent);
    out.push_str(&format_meta_prefix(&node.meta));
    out.push_str("///");
    if !node.text.is_empty() {
        out.push(' ');
        out.push_str(&node.text);
    }
    out.push('\n');
}

fn write_comment(node: &CommentNode, out: &mut String, indent: usize) {
    indent_line(out, indent);
    out.push_str(&format_meta_prefix(&node.meta));
    out.push_str("//");
    if !node.text.is_empty() {
        out.push(' ');
        out.push_str(&node.text);
    }
    out.push('\n');
}

fn indent_line(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn format_meta_prefix(meta: &Meta) -> String {
    let mut out = String::new();
    out.push('@');
    out.push_str(&meta.id);
    if let Some(rank) = &meta.rank {
        out.push('[');
        out.push_str(rank);
        out.push(']');
    }
    if let Some(anchor) = &meta.anchor {
        out.push_str("->");
        out.push_str(anchor);
    }
    out.push(' ');
    out
}

fn format_type(ty: &Type) -> String {
    match ty {
        Type::Path(node) => {
            let mut out = format_meta_prefix(&node.meta);
            out.push_str(&format_path(&node.path));
            out
        }
    }
}

fn format_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Ident(node) => {
            let mut out = String::new();
            if let Some(meta) = &node.meta {
                out.push_str(&format_meta_prefix(meta));
            }
            out.push_str(&node.name);
            out
        }
        Pattern::Wild(node) => {
            let mut out = String::new();
            if let Some(meta) = &node.meta {
                out.push_str(&format_meta_prefix(meta));
            }
            out.push('_');
            out
        }
    }
}

fn format_expr(expr: &Expr, indent: usize) -> String {
    format_expr_prec(expr, indent, 0)
}

fn format_expr_prec(expr: &Expr, indent: usize, outer_prec: u8) -> String {
    match expr {
        Expr::Path(node) => format_wrapped_expr(node.meta.as_ref(), format_path(&node.path), false),
        Expr::Lit(node) => {
            let core = match &node.value {
                Literal::Int(value) => value.to_string(),
                Literal::Str(value) => format!("\"{}\"", escape_string(value)),
            };
            format_wrapped_expr(node.meta.as_ref(), core, false)
        }
        Expr::Group(node) => {
            let core = format!("({})", format_expr(&node.expr, indent));
            format_wrapped_expr(node.meta.as_ref(), core, false)
        }
        Expr::Binary(node) => {
            let prec = binary_precedence(node.op);
            let core = format!(
                "{} {} {}",
                format_expr_prec(&node.lhs, indent, prec),
                match node.op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Lt => "<",
                },
                format_expr_prec(&node.rhs, indent, prec + 1)
            );
            format_wrapped_expr(node.meta.as_ref(), core, prec < outer_prec)
        }
        Expr::Unary(node) => {
            let prec = 30;
            let core = format!(
                "{}{}",
                match node.op {
                    UnaryOp::Neg => "-",
                },
                format_expr_prec(&node.expr, indent, prec)
            );
            format_wrapped_expr(node.meta.as_ref(), core, prec < outer_prec)
        }
        Expr::Call(node) => {
            let prec = 40;
            let mut core = format_expr_prec(&node.callee, indent, prec);
            core.push('(');
            for (index, arg) in node.args.iter().enumerate() {
                if index > 0 {
                    core.push_str(", ");
                }
                core.push_str(&format_expr(arg, indent));
            }
            core.push(')');
            format_wrapped_expr(node.meta.as_ref(), core, prec < outer_prec)
        }
        Expr::Match(node) => {
            let mut core = String::new();
            core.push_str("match ");
            core.push_str(&format_expr(&node.scrutinee, indent));
            core.push_str(" {\n");
            for arm in &node.arms {
                indent_line(&mut core, indent + 1);
                core.push_str(&format_meta_prefix(&arm.meta));
                core.push_str(&format_pattern(&arm.pat));
                if let Some(guard) = &arm.guard {
                    core.push_str(" if ");
                    core.push_str(&format_expr(guard, indent + 1));
                }
                core.push_str(" => ");
                core.push_str(&format_expr(&arm.body, indent + 1));
                core.push_str(",\n");
            }
            indent_line(&mut core, indent);
            core.push('}');
            format_wrapped_expr(node.meta.as_ref(), core, false)
        }
        Expr::Block(block) => {
            let mut core = String::new();
            core.push_str("{\n");
            write_stmt_list(&block.stmts, &mut core, indent + 1);
            indent_line(&mut core, indent);
            core.push('}');
            format_wrapped_expr(block.meta.as_ref(), core, false)
        }
    }
}

fn format_wrapped_expr(meta: Option<&Meta>, mut core: String, wrap: bool) -> String {
    if wrap {
        core = format!("({core})");
    }
    if let Some(meta) = meta {
        let mut out = format_meta_prefix(meta);
        out.push_str(&core);
        out
    } else {
        core
    }
}

fn binary_precedence(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Lt => 10,
        BinaryOp::Add | BinaryOp::Sub => 20,
    }
}

fn needs_item_gap(items: &[Item], current: usize, next: usize) -> bool {
    let current = &items[current];
    let next = &items[next];
    match current {
        Item::Doc(_) | Item::Comment(_) => {
            resolved_item_target(items, current) != resolved_item_target(items, next)
        }
        _ => true,
    }
}

fn resolved_item_target<'a>(items: &'a [Item], item: &'a Item) -> Option<&'a str> {
    if !matches!(item, Item::Doc(_) | Item::Comment(_)) {
        return Some(item.meta().id.as_str());
    }
    if let Some(anchor) = item.meta().anchor.as_deref() {
        return Some(anchor);
    }
    let index = items
        .iter()
        .position(|candidate| std::ptr::eq(candidate, item))?;
    for candidate in &items[index + 1..] {
        if !matches!(candidate, Item::Doc(_) | Item::Comment(_)) {
            return Some(candidate.meta().id.as_str());
        }
    }
    None
}

fn format_path(path: &Path) -> String {
    path.segments.join("::")
}

fn format_use_tree(tree: &UseTree) -> String {
    match tree {
        UseTree::Name(node) => node.name.clone(),
        UseTree::Path(node) => format!("{}::{}", node.prefix, format_use_tree(&node.tree)),
        UseTree::Group(node) => {
            let mut out = String::from("{");
            for (index, item) in node.items.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format_use_tree(item));
            }
            out.push('}');
            out
        }
        UseTree::Glob(_) => "*".to_owned(),
    }
}

fn escape_string(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}
