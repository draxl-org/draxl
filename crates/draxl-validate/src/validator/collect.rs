use super::Validator;
use draxl_ast::{Block, Expr, File, Item, MatchArm, Meta, Pattern, Stmt, Type};

impl Validator {
    pub(crate) fn collect_file_ids(&mut self, file: &File) {
        for item in &file.items {
            self.collect_item_ids(item);
        }
    }

    fn collect_item_ids(&mut self, item: &Item) {
        self.record_meta(item.meta(), "item");
        match item {
            Item::Mod(node) => {
                for child in &node.items {
                    self.collect_item_ids(child);
                }
            }
            Item::Use(_) => {}
            Item::Struct(node) => {
                for field in &node.fields {
                    self.record_meta(&field.meta, "field");
                    self.collect_type_ids(&field.ty);
                }
            }
            Item::Enum(node) => {
                for variant in &node.variants {
                    self.record_meta(&variant.meta, "variant");
                }
            }
            Item::Fn(node) => {
                for param in &node.params {
                    self.record_meta(&param.meta, "parameter");
                    self.collect_type_ids(&param.ty);
                }
                if let Some(ret_ty) = &node.ret_ty {
                    self.collect_type_ids(ret_ty);
                }
                self.collect_block_ids(&node.body);
            }
            Item::Doc(_) | Item::Comment(_) => {}
        }
    }

    fn collect_block_ids(&mut self, block: &Block) {
        if let Some(meta) = &block.meta {
            self.record_meta(meta, "block");
        }
        for stmt in &block.stmts {
            self.collect_stmt_ids(stmt);
        }
    }

    fn collect_stmt_ids(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(node) => {
                self.record_meta(&node.meta, "let statement");
                self.collect_pattern_ids(&node.pat);
                self.collect_expr_ids(&node.value);
            }
            Stmt::Expr(node) => {
                self.record_meta(&node.meta, "expression statement");
                self.collect_expr_ids(&node.expr);
            }
            Stmt::Item(item) => self.collect_item_ids(item),
            Stmt::Doc(node) => self.record_meta(&node.meta, "doc comment"),
            Stmt::Comment(node) => self.record_meta(&node.meta, "line comment"),
        }
    }

    fn collect_expr_ids(&mut self, expr: &Expr) {
        if let Some(meta) = expr.meta() {
            self.record_meta(meta, "expression");
        }
        match expr {
            Expr::Path(_) | Expr::Lit(_) => {}
            Expr::Group(node) => self.collect_expr_ids(&node.expr),
            Expr::Binary(node) => {
                self.collect_expr_ids(&node.lhs);
                self.collect_expr_ids(&node.rhs);
            }
            Expr::Unary(node) => self.collect_expr_ids(&node.expr),
            Expr::Call(node) => {
                self.collect_expr_ids(&node.callee);
                for arg in &node.args {
                    self.collect_expr_ids(arg);
                }
            }
            Expr::Match(node) => {
                self.collect_expr_ids(&node.scrutinee);
                for arm in &node.arms {
                    self.collect_match_arm_ids(arm);
                }
            }
            Expr::Block(block) => self.collect_block_ids(block),
        }
    }

    fn collect_match_arm_ids(&mut self, arm: &MatchArm) {
        self.record_meta(&arm.meta, "match arm");
        self.collect_pattern_ids(&arm.pat);
        if let Some(guard) = &arm.guard {
            self.collect_expr_ids(guard);
        }
        self.collect_expr_ids(&arm.body);
    }

    fn collect_pattern_ids(&mut self, pattern: &Pattern) {
        if let Some(meta) = pattern.meta() {
            self.record_meta(meta, "pattern");
        }
    }

    fn collect_type_ids(&mut self, ty: &Type) {
        self.record_meta(ty.meta(), "type");
    }

    fn record_meta(&mut self, meta: &Meta, kind: &'static str) {
        if let Some(previous) = self.seen_ids.insert(meta.id.clone(), kind) {
            self.push(format!(
                "duplicate id `{}` appears on both a {} and a {}",
                meta.id, previous, kind
            ));
        }
    }
}
