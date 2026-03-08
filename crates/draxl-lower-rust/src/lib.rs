#![forbid(unsafe_code)]
//! Lowering from Draxl Source v0 to ordinary Rust source.
//!
//! Lowering assumes the input already passed structural validation. The output
//! strips Draxl metadata and preserves only the modeled Rust subset.

use draxl_ast::{
    BinaryOp, Block, CommentNode, DocNode, Expr, File, Item, ItemFn, Literal, Path, Pattern, Stmt,
    Type, UnaryOp, UseTree,
};
use draxl_printer::canonicalize_file;

/// Lowers a validated Draxl file to ordinary Rust.
pub fn lower_file(file: &File) -> String {
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
            out.push_str("use ");
            out.push_str(&format_use_tree(&node.tree));
            out.push_str(";\n");
        }
        Item::Struct(node) => {
            indent_line(out, indent);
            out.push_str("struct ");
            out.push_str(&node.name);
            out.push_str(" {\n");
            for field in &node.fields {
                indent_line(out, indent + 1);
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
            out.push_str("enum ");
            out.push_str(&node.name);
            out.push_str(" {\n");
            for variant in &node.variants {
                indent_line(out, indent + 1);
                out.push_str(&variant.name);
                out.push_str(",\n");
            }
            indent_line(out, indent);
            out.push_str("}\n");
        }
        Item::Fn(node) => write_fn(node, out, indent),
        Item::Doc(node) => write_doc(node, out, indent),
        Item::Comment(node) => write_comment(node, out, indent),
    }
}

fn write_fn(node: &ItemFn, out: &mut String, indent: usize) {
    indent_line(out, indent);
    out.push_str("fn ");
    out.push_str(&node.name);
    out.push('(');
    for (index, param) in node.params.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&param.name);
        out.push_str(": ");
        out.push_str(&format_type(&param.ty));
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

fn write_stmt_list(stmts: &[Stmt], out: &mut String, indent: usize) {
    for stmt in stmts {
        write_stmt(stmt, out, indent);
    }
}

fn write_stmt(stmt: &Stmt, out: &mut String, indent: usize) {
    match stmt {
        Stmt::Let(node) => {
            indent_line(out, indent);
            out.push_str("let ");
            out.push_str(&format_pattern(&node.pat));
            out.push_str(" = ");
            out.push_str(&format_expr(&node.value, indent));
            out.push_str(";\n");
        }
        Stmt::Expr(node) => {
            indent_line(out, indent);
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
    out.push_str("///");
    if !node.text.is_empty() {
        out.push(' ');
        out.push_str(&node.text);
    }
    out.push('\n');
}

fn write_comment(node: &CommentNode, out: &mut String, indent: usize) {
    indent_line(out, indent);
    out.push_str("//");
    if !node.text.is_empty() {
        out.push(' ');
        out.push_str(&node.text);
    }
    out.push('\n');
}

fn format_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Ident(node) => node.name.clone(),
        Pattern::Wild(_) => "_".to_owned(),
    }
}

fn format_type(ty: &Type) -> String {
    match ty {
        Type::Path(node) => format_path(&node.path),
    }
}

fn format_expr(expr: &Expr, indent: usize) -> String {
    match expr {
        Expr::Path(node) => format_path(&node.path),
        Expr::Lit(node) => match &node.value {
            Literal::Int(value) => value.to_string(),
            Literal::Str(value) => format!("\"{}\"", escape_string(value)),
        },
        Expr::Group(node) => format!("({})", format_expr(&node.expr, indent)),
        Expr::Binary(node) => format!(
            "{} {} {}",
            format_expr(&node.lhs, indent),
            match node.op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Lt => "<",
            },
            format_expr(&node.rhs, indent)
        ),
        Expr::Unary(node) => format!(
            "{}{}",
            match node.op {
                UnaryOp::Neg => "-",
            },
            format_expr(&node.expr, indent)
        ),
        Expr::Call(node) => {
            let mut out = format_expr(&node.callee, indent);
            out.push('(');
            for (index, arg) in node.args.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format_expr(arg, indent));
            }
            out.push(')');
            out
        }
        Expr::Match(node) => {
            let mut out = String::new();
            out.push_str("match ");
            out.push_str(&format_expr(&node.scrutinee, indent));
            out.push_str(" {\n");
            for arm in &node.arms {
                indent_line(&mut out, indent + 1);
                out.push_str(&format_pattern(&arm.pat));
                if let Some(guard) = &arm.guard {
                    out.push_str(" if ");
                    out.push_str(&format_expr(guard, indent + 1));
                }
                out.push_str(" => ");
                out.push_str(&format_expr(&arm.body, indent + 1));
                out.push_str(",\n");
            }
            indent_line(&mut out, indent);
            out.push('}');
            out
        }
        Expr::Block(Block { stmts, .. }) => {
            let mut out = String::new();
            out.push_str("{\n");
            write_stmt_list(stmts, &mut out, indent + 1);
            indent_line(&mut out, indent);
            out.push('}');
            out
        }
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

fn indent_line(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
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
