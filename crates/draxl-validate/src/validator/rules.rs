use super::Validator;
use draxl_ast::{Block, Expr, File, Item, MatchArm, Meta, Pattern, Stmt, Type};
use std::collections::{BTreeMap, BTreeSet};

impl Validator {
    pub(crate) fn validate_file(&mut self, file: &File) {
        self.validate_item_container(&file.items, "file_items", false);
        for item in &file.items {
            self.validate_item(item);
        }
    }

    fn validate_item(&mut self, item: &Item) {
        match item {
            Item::Mod(node) => {
                self.check_anchor_usage(&node.meta);
                self.validate_item_container(&node.items, "items", true);
                for child in &node.items {
                    self.validate_item(child);
                }
            }
            Item::Use(node) => {
                self.check_anchor_usage(&node.meta);
            }
            Item::Struct(node) => {
                self.check_anchor_usage(&node.meta);
                self.check_ranked_meta(
                    node.fields
                        .iter()
                        .map(|field| (&field.meta, "struct field")),
                    "fields",
                );
                for field in &node.fields {
                    self.validate_type(&field.ty);
                }
            }
            Item::Enum(node) => {
                self.check_anchor_usage(&node.meta);
                self.check_ranked_meta(
                    node.variants
                        .iter()
                        .map(|variant| (&variant.meta, "enum variant")),
                    "variants",
                );
            }
            Item::Fn(node) => {
                self.check_anchor_usage(&node.meta);
                self.check_ranked_meta(
                    node.params
                        .iter()
                        .map(|param| (&param.meta, "function parameter")),
                    "params",
                );
                for param in &node.params {
                    self.validate_type(&param.ty);
                }
                if let Some(ret_ty) = &node.ret_ty {
                    self.validate_type(ret_ty);
                }
                self.validate_block(&node.body);
            }
            Item::Doc(_) | Item::Comment(_) => {}
        }
    }

    fn validate_block(&mut self, block: &Block) {
        if let Some(meta) = &block.meta {
            self.check_anchor_usage(meta);
        }
        self.validate_stmt_container(&block.stmts, "body");
        for stmt in &block.stmts {
            self.validate_stmt(stmt);
        }
    }

    fn validate_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(node) => {
                self.check_anchor_usage(&node.meta);
                self.validate_pattern(&node.pat);
                self.validate_expr(&node.value);
            }
            Stmt::Expr(node) => {
                self.check_anchor_usage(&node.meta);
                if node.meta.rank.is_none() {
                    self.push(format!(
                        "expression statement `{}` in slot `body` is missing `rank`",
                        node.meta.id
                    ));
                }
                self.validate_expr(&node.expr);
            }
            Stmt::Item(item) => {
                if item.meta().rank.is_none() {
                    self.push(format!(
                        "nested item `{}` in slot `body` is missing `rank`",
                        item.meta().id
                    ));
                }
                self.validate_item(item);
            }
            Stmt::Doc(_) | Stmt::Comment(_) => {}
        }
    }

    fn validate_expr(&mut self, expr: &Expr) {
        if let Some(meta) = expr.meta() {
            self.check_anchor_usage(meta);
        }
        match expr {
            Expr::Path(_) | Expr::Lit(_) => {}
            Expr::Group(node) => self.validate_expr(&node.expr),
            Expr::Binary(node) => {
                self.validate_expr(&node.lhs);
                self.validate_expr(&node.rhs);
            }
            Expr::Unary(node) => self.validate_expr(&node.expr),
            Expr::Call(node) => {
                self.validate_expr(&node.callee);
                for arg in &node.args {
                    self.validate_expr(arg);
                }
            }
            Expr::Match(node) => {
                self.validate_expr(&node.scrutinee);
                self.check_ranked_meta(
                    node.arms.iter().map(|arm| (&arm.meta, "match arm")),
                    "arms",
                );
                for arm in &node.arms {
                    self.validate_match_arm(arm);
                }
            }
            Expr::Block(block) => self.validate_block(block),
        }
    }

    fn validate_match_arm(&mut self, arm: &MatchArm) {
        self.check_anchor_usage(&arm.meta);
        self.validate_pattern(&arm.pat);
        if let Some(guard) = &arm.guard {
            self.validate_expr(guard);
        }
        self.validate_expr(&arm.body);
    }

    fn validate_pattern(&mut self, pattern: &Pattern) {
        if let Some(meta) = pattern.meta() {
            self.check_anchor_usage(meta);
        }
    }

    fn validate_type(&mut self, ty: &Type) {
        self.check_anchor_usage(ty.meta());
    }

    fn validate_item_container(&mut self, items: &[Item], slot: &str, ordered: bool) {
        let semantic = items
            .iter()
            .filter(|item| !matches!(item, Item::Doc(_) | Item::Comment(_)))
            .collect::<Vec<_>>();
        if ordered {
            self.check_ranked_meta(
                semantic.iter().map(|item| (item.meta(), "module item")),
                slot,
            );
        }
        let local_targets = semantic
            .iter()
            .map(|item| item.meta().id.as_str())
            .collect::<BTreeSet<_>>();
        let mut pending = Vec::new();
        for item in items {
            match item {
                Item::Doc(node) => {
                    self.validate_trivia_attachment(
                        &node.meta,
                        "doc comment",
                        slot,
                        &local_targets,
                        &mut pending,
                    );
                }
                Item::Comment(node) => {
                    self.validate_trivia_attachment(
                        &node.meta,
                        "line comment",
                        slot,
                        &local_targets,
                        &mut pending,
                    );
                }
                _ => pending.clear(),
            }
        }
        self.finish_pending_trivia(slot, pending);
    }

    fn validate_stmt_container(&mut self, stmts: &[Stmt], slot: &str) {
        let semantic = stmts
            .iter()
            .filter(|stmt| !matches!(stmt, Stmt::Doc(_) | Stmt::Comment(_)))
            .collect::<Vec<_>>();
        let mut ranks = BTreeMap::new();
        for stmt in &semantic {
            let meta = stmt
                .meta()
                .expect("semantic block children always carry metadata");
            let label = match stmt {
                Stmt::Let(_) => "let statement",
                Stmt::Expr(_) => "expression statement",
                Stmt::Item(_) => "nested item",
                Stmt::Doc(_) | Stmt::Comment(_) => "statement",
            };
            if meta.rank.is_none() {
                self.push(format!(
                    "{} `{}` in slot `{}` is missing `rank`",
                    label, meta.id, slot
                ));
            } else if let Some(rank) = &meta.rank {
                if rank.is_empty() {
                    self.push(format!(
                        "{} `{}` in slot `{}` uses an empty `rank`",
                        label, meta.id, slot
                    ));
                }
                if let Some(previous) = ranks.insert(rank.clone(), meta.id.clone()) {
                    self.push(format!(
                        "slot `{}` uses duplicate rank `{}` on `{}` and `{}`",
                        slot, rank, previous, meta.id
                    ));
                }
            }
        }
        let local_targets = semantic
            .iter()
            .map(|stmt| {
                stmt.meta()
                    .expect("semantic block children always carry metadata")
                    .id
                    .as_str()
            })
            .collect::<BTreeSet<_>>();
        let mut pending = Vec::new();
        for stmt in stmts {
            match stmt {
                Stmt::Doc(node) => {
                    self.validate_trivia_attachment(
                        &node.meta,
                        "doc comment",
                        slot,
                        &local_targets,
                        &mut pending,
                    );
                }
                Stmt::Comment(node) => {
                    self.validate_trivia_attachment(
                        &node.meta,
                        "line comment",
                        slot,
                        &local_targets,
                        &mut pending,
                    );
                }
                _ => pending.clear(),
            }
        }
        self.finish_pending_trivia(slot, pending);
    }

    fn check_ranked_meta<'a>(
        &mut self,
        metas: impl Iterator<Item = (&'a Meta, &'static str)>,
        slot: &str,
    ) {
        let mut seen_ranks = BTreeMap::new();
        for (meta, label) in metas {
            self.check_anchor_usage(meta);
            let Some(rank) = &meta.rank else {
                self.push(format!(
                    "{} `{}` in slot `{}` is missing `rank`",
                    label, meta.id, slot
                ));
                continue;
            };
            if rank.is_empty() {
                self.push(format!(
                    "{} `{}` in slot `{}` uses an empty `rank`",
                    label, meta.id, slot
                ));
            }
            if let Some(previous) = seen_ranks.insert(rank.clone(), meta.id.clone()) {
                self.push(format!(
                    "slot `{}` uses duplicate rank `{}` on `{}` and `{}`",
                    slot, rank, previous, meta.id
                ));
            }
        }
    }

    fn validate_trivia_attachment(
        &mut self,
        meta: &Meta,
        label: &'static str,
        slot: &str,
        local_targets: &BTreeSet<&str>,
        pending: &mut Vec<(String, &'static str)>,
    ) {
        self.check_anchor_usage(meta);
        if let Some(anchor) = &meta.anchor {
            if !local_targets.contains(anchor.as_str()) {
                self.push(format!(
                    "{} `{}` in slot `{}` must anchor a sibling semantic node, found `{}`",
                    label, meta.id, slot, anchor
                ));
            }
        } else {
            pending.push((meta.id.clone(), label));
        }
    }

    fn finish_pending_trivia(&mut self, slot: &str, pending: Vec<(String, &'static str)>) {
        for (id, label) in pending {
            self.push(format!(
                "{} `{}` in slot `{}` is detached and needs a following sibling or `->anchor`",
                label, id, slot
            ));
        }
    }

    fn check_anchor_usage(&mut self, meta: &Meta) {
        if let Some(anchor) = &meta.anchor {
            if !self.seen_ids.contains_key(anchor) {
                self.push(format!(
                    "anchor `{}` referenced by `{}` does not match any node id in the file",
                    anchor, meta.id
                ));
            }
        }
    }
}
