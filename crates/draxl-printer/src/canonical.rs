use draxl_ast::{
    Block, Expr, ExprBinary, ExprCall, ExprGroup, ExprMatch, ExprUnary, File, Item, ItemEnum,
    ItemFn, ItemMod, ItemStruct, ItemUse, MatchArm, Meta, Stmt, StmtExpr, StmtLet,
};

/// Returns a canonicalized clone of the file.
pub fn canonicalize_file(file: &File) -> File {
    File {
        items: canonicalize_items(&file.items, false),
    }
}

pub(crate) fn canonicalize_items(items: &[Item], ordered: bool) -> Vec<Item> {
    let mut trivia = Vec::new();
    let mut pending = Vec::new();
    let mut semantic = Vec::new();

    for item in items {
        let item = canonicalize_item(item);
        match item {
            Item::Doc(_) | Item::Comment(_) => {
                if item.meta().anchor.is_some() {
                    trivia.push((item.meta().anchor.clone(), item));
                } else {
                    pending.push(item);
                }
            }
            _ => {
                let target = item.meta().id.clone();
                for pending_item in pending.drain(..) {
                    trivia.push((Some(target.clone()), pending_item));
                }
                semantic.push(item);
            }
        }
    }

    for pending_item in pending {
        trivia.push((None, pending_item));
    }

    if ordered {
        semantic
            .sort_by(|left, right| meta_sort_key(left.meta()).cmp(&meta_sort_key(right.meta())));
    }

    attach_item_trivia(semantic, trivia)
}

fn canonicalize_item(item: &Item) -> Item {
    match item {
        Item::Mod(node) => Item::Mod(ItemMod {
            meta: node.meta.clone(),
            name: node.name.clone(),
            items: canonicalize_items(&node.items, true),
        }),
        Item::Use(node) => Item::Use(ItemUse {
            meta: node.meta.clone(),
            tree: node.tree.clone(),
        }),
        Item::Struct(node) => {
            let mut fields = node.fields.clone();
            fields
                .sort_by(|left, right| meta_sort_key(&left.meta).cmp(&meta_sort_key(&right.meta)));
            Item::Struct(ItemStruct {
                meta: node.meta.clone(),
                name: node.name.clone(),
                fields,
            })
        }
        Item::Enum(node) => {
            let mut variants = node.variants.clone();
            variants
                .sort_by(|left, right| meta_sort_key(&left.meta).cmp(&meta_sort_key(&right.meta)));
            Item::Enum(ItemEnum {
                meta: node.meta.clone(),
                name: node.name.clone(),
                variants,
            })
        }
        Item::Fn(node) => {
            let mut params = node.params.clone();
            params
                .sort_by(|left, right| meta_sort_key(&left.meta).cmp(&meta_sort_key(&right.meta)));
            Item::Fn(ItemFn {
                meta: node.meta.clone(),
                name: node.name.clone(),
                params,
                ret_ty: node.ret_ty.clone(),
                body: canonicalize_block(&node.body),
            })
        }
        Item::Doc(node) => Item::Doc(node.clone()),
        Item::Comment(node) => Item::Comment(node.clone()),
    }
}

pub(crate) fn canonicalize_block(block: &Block) -> Block {
    let mut trivia = Vec::new();
    let mut pending = Vec::new();
    let mut semantic = Vec::new();

    for stmt in &block.stmts {
        let stmt = canonicalize_stmt(stmt);
        match stmt {
            Stmt::Doc(_) | Stmt::Comment(_) => {
                if stmt
                    .meta()
                    .is_some_and(|meta| meta.anchor.as_ref().is_some())
                {
                    trivia.push((stmt.meta().and_then(|meta| meta.anchor.clone()), stmt));
                } else {
                    pending.push(stmt);
                }
            }
            _ => {
                let target = stmt
                    .meta()
                    .expect("semantic block children always carry metadata")
                    .id
                    .clone();
                for pending_stmt in pending.drain(..) {
                    trivia.push((Some(target.clone()), pending_stmt));
                }
                semantic.push(stmt);
            }
        }
    }

    for pending_stmt in pending {
        trivia.push((None, pending_stmt));
    }

    semantic.sort_by(|left, right| stmt_sort_key(left).cmp(&stmt_sort_key(right)));

    Block {
        meta: block.meta.clone(),
        stmts: attach_stmt_trivia(semantic, trivia),
    }
}

fn canonicalize_stmt(stmt: &Stmt) -> Stmt {
    match stmt {
        Stmt::Let(node) => Stmt::Let(StmtLet {
            meta: node.meta.clone(),
            pat: node.pat.clone(),
            value: canonicalize_expr(&node.value),
        }),
        Stmt::Expr(node) => Stmt::Expr(StmtExpr {
            meta: node.meta.clone(),
            expr: canonicalize_expr(&node.expr),
            has_semi: node.has_semi,
        }),
        Stmt::Item(item) => Stmt::Item(canonicalize_item(item)),
        Stmt::Doc(node) => Stmt::Doc(node.clone()),
        Stmt::Comment(node) => Stmt::Comment(node.clone()),
    }
}

fn canonicalize_expr(expr: &Expr) -> Expr {
    match expr {
        Expr::Path(node) => Expr::Path(node.clone()),
        Expr::Lit(node) => Expr::Lit(node.clone()),
        Expr::Group(node) => Expr::Group(ExprGroup {
            meta: node.meta.clone(),
            expr: Box::new(canonicalize_expr(&node.expr)),
        }),
        Expr::Binary(node) => Expr::Binary(ExprBinary {
            meta: node.meta.clone(),
            lhs: Box::new(canonicalize_expr(&node.lhs)),
            op: node.op,
            rhs: Box::new(canonicalize_expr(&node.rhs)),
        }),
        Expr::Unary(node) => Expr::Unary(ExprUnary {
            meta: node.meta.clone(),
            op: node.op,
            expr: Box::new(canonicalize_expr(&node.expr)),
        }),
        Expr::Call(node) => Expr::Call(ExprCall {
            meta: node.meta.clone(),
            callee: Box::new(canonicalize_expr(&node.callee)),
            args: node.args.iter().map(canonicalize_expr).collect(),
        }),
        Expr::Match(node) => {
            let mut arms = node.arms.clone();
            arms.sort_by(|left, right| meta_sort_key(&left.meta).cmp(&meta_sort_key(&right.meta)));
            let arms = arms
                .into_iter()
                .map(|arm| MatchArm {
                    meta: arm.meta,
                    pat: arm.pat,
                    guard: arm.guard.map(|expr| canonicalize_expr(&expr)),
                    body: canonicalize_expr(&arm.body),
                })
                .collect();
            Expr::Match(ExprMatch {
                meta: node.meta.clone(),
                scrutinee: Box::new(canonicalize_expr(&node.scrutinee)),
                arms,
            })
        }
        Expr::Block(node) => Expr::Block(canonicalize_block(node)),
    }
}

fn attach_item_trivia(
    mut semantic: Vec<Item>,
    mut trivia: Vec<(Option<String>, Item)>,
) -> Vec<Item> {
    let mut out = Vec::new();
    for item in semantic.drain(..) {
        let target = item.meta().id.clone();
        let mut index = 0;
        while index < trivia.len() {
            if trivia[index].0.as_deref() == Some(target.as_str()) {
                out.push(trivia.remove(index).1);
            } else {
                index += 1;
            }
        }
        out.push(item);
    }
    out.extend(trivia.into_iter().map(|(_, item)| item));
    out
}

fn attach_stmt_trivia(
    mut semantic: Vec<Stmt>,
    mut trivia: Vec<(Option<String>, Stmt)>,
) -> Vec<Stmt> {
    let mut out = Vec::new();
    for stmt in semantic.drain(..) {
        let target = stmt
            .meta()
            .expect("semantic block children always carry metadata")
            .id
            .clone();
        let mut index = 0;
        while index < trivia.len() {
            if trivia[index].0.as_deref() == Some(target.as_str()) {
                out.push(trivia.remove(index).1);
            } else {
                index += 1;
            }
        }
        out.push(stmt);
    }
    out.extend(trivia.into_iter().map(|(_, stmt)| stmt));
    out
}

pub(crate) fn meta_sort_key(meta: &Meta) -> (String, String) {
    (
        meta.rank.clone().unwrap_or_else(|| "~".to_owned()),
        meta.id.clone(),
    )
}

pub(crate) fn stmt_sort_key(stmt: &Stmt) -> (String, String) {
    let meta = stmt
        .meta()
        .expect("semantic block children always carry metadata");
    meta_sort_key(meta)
}
