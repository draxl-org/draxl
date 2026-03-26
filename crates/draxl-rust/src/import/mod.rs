//! Importing ordinary Rust source into the Draxl AST.

use draxl_ast as ast;
use std::collections::BTreeMap;
use std::error::Error as StdError;
use std::fmt;
use syn::spanned::Spanned;

/// Imports ordinary Rust source into the Draxl AST.
pub fn import_source(source: &str) -> Result<ast::File, ImportError> {
    let parsed = syn::parse_file(source).map_err(ImportError::RustParse)?;
    Importer::default().import_file(&parsed)
}

/// Import failure while converting ordinary Rust into Draxl.
#[derive(Debug)]
pub enum ImportError {
    /// The Rust source could not be parsed by the import frontend.
    RustParse(syn::Error),
    /// The Rust source used syntax outside the currently supported import subset.
    Unsupported(syn::Error),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RustParse(error) => write!(f, "Rust parse failed: {error}"),
            Self::Unsupported(error) => write!(f, "unsupported Rust syntax: {error}"),
        }
    }
}

impl StdError for ImportError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::RustParse(error) => Some(error),
            Self::Unsupported(error) => Some(error),
        }
    }
}

#[derive(Debug, Default)]
struct Importer {
    ids: IdAllocator,
}

#[derive(Debug, Default)]
struct IdAllocator {
    counts: BTreeMap<&'static str, usize>,
}

impl IdAllocator {
    fn next(&mut self, prefix: &'static str) -> String {
        let next = self.counts.entry(prefix).or_insert(0);
        *next += 1;
        format!("{prefix}{next:04}")
    }
}

#[derive(Debug)]
struct Placement {
    slot: Option<&'static str>,
    rank: Option<String>,
}

impl Placement {
    fn file_item() -> Self {
        Self {
            slot: Some("file_items"),
            rank: None,
        }
    }

    fn ranked(slot: &'static str, index: usize) -> Self {
        Self {
            slot: Some(slot),
            rank: Some(format!("r{:04}", index + 1)),
        }
    }
}

impl Importer {
    fn import_file(mut self, file: &syn::File) -> Result<ast::File, ImportError> {
        if file.shebang.is_some() {
            return Err(unsupported(
                file,
                "shebang lines are unsupported in Rust import",
            ));
        }
        self.check_attrs(&file.attrs, "file")?;
        let items = file
            .items
            .iter()
            .map(|item| self.import_item(item, Placement::file_item()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ast::File { items })
    }

    fn import_item(
        &mut self,
        item: &syn::Item,
        placement: Placement,
    ) -> Result<ast::Item, ImportError> {
        match item {
            syn::Item::Mod(node) => self.import_mod(node, placement),
            syn::Item::Use(node) => self.import_use(node, placement),
            syn::Item::Struct(node) => self.import_struct(node, placement),
            syn::Item::Enum(node) => self.import_enum(node, placement),
            syn::Item::Fn(node) => self.import_fn(node, placement),
            syn::Item::Macro(node) => Err(unsupported(node, "macro items are unsupported")),
            _ => Err(unsupported(
                item,
                "only `mod`, `use`, `struct`, `enum`, and `fn` items are supported",
            )),
        }
    }

    fn import_mod(
        &mut self,
        node: &syn::ItemMod,
        placement: Placement,
    ) -> Result<ast::Item, ImportError> {
        self.check_attrs(&node.attrs, "module")?;
        self.ensure_inherited_visibility(&node.vis, "module visibility modifiers are unsupported")?;
        if node.unsafety.is_some() {
            return Err(unsupported(node, "`unsafe mod` is unsupported"));
        }
        let Some((_, items)) = &node.content else {
            return Err(unsupported(
                node,
                "out-of-line `mod foo;` declarations are unsupported",
            ));
        };
        let items = items
            .iter()
            .enumerate()
            .map(|(index, item)| self.import_item(item, Placement::ranked("items", index)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ast::Item::Mod(ast::ItemMod {
            meta: self.meta("m", placement),
            name: node.ident.to_string(),
            items,
        }))
    }

    fn import_use(
        &mut self,
        node: &syn::ItemUse,
        placement: Placement,
    ) -> Result<ast::Item, ImportError> {
        self.check_attrs(&node.attrs, "use item")?;
        self.ensure_inherited_visibility(
            &node.vis,
            "use item visibility modifiers are unsupported",
        )?;
        if node.leading_colon.is_some() {
            return Err(unsupported(
                node,
                "leading `::` in `use` items is unsupported",
            ));
        }
        Ok(ast::Item::Use(ast::ItemUse {
            meta: self.meta("u", placement),
            tree: self.import_use_tree(&node.tree)?,
        }))
    }

    fn import_use_tree(&mut self, tree: &syn::UseTree) -> Result<ast::UseTree, ImportError> {
        match tree {
            syn::UseTree::Name(node) => Ok(ast::UseTree::Name(ast::UseName {
                name: node.ident.to_string(),
            })),
            syn::UseTree::Path(node) => Ok(ast::UseTree::Path(ast::UsePathTree {
                prefix: node.ident.to_string(),
                tree: Box::new(self.import_use_tree(&node.tree)?),
            })),
            syn::UseTree::Group(node) => Ok(ast::UseTree::Group(ast::UseGroup {
                items: node
                    .items
                    .iter()
                    .map(|item| self.import_use_tree(item))
                    .collect::<Result<Vec<_>, _>>()?,
            })),
            syn::UseTree::Glob(_) => Ok(ast::UseTree::Glob(ast::UseGlob)),
            syn::UseTree::Rename(node) => Err(unsupported(
                node,
                "`use path as name` renames are unsupported",
            )),
        }
    }

    fn import_struct(
        &mut self,
        node: &syn::ItemStruct,
        placement: Placement,
    ) -> Result<ast::Item, ImportError> {
        self.check_attrs(&node.attrs, "struct")?;
        self.ensure_inherited_visibility(&node.vis, "struct visibility modifiers are unsupported")?;
        self.ensure_no_generics(&node.generics, "struct generics are unsupported")?;
        let syn::Fields::Named(fields) = &node.fields else {
            return Err(unsupported(
                node,
                "only named-field structs are supported in Rust import",
            ));
        };
        let fields = fields
            .named
            .iter()
            .enumerate()
            .map(|(index, field)| self.import_field(field, index))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ast::Item::Struct(ast::ItemStruct {
            meta: self.meta("st", placement),
            name: node.ident.to_string(),
            fields,
        }))
    }

    fn import_field(
        &mut self,
        field: &syn::Field,
        index: usize,
    ) -> Result<ast::Field, ImportError> {
        self.check_attrs(&field.attrs, "struct field")?;
        self.ensure_inherited_visibility(&field.vis, "field visibility modifiers are unsupported")?;
        let Some(ident) = &field.ident else {
            return Err(unsupported(field, "only named struct fields are supported"));
        };
        Ok(ast::Field {
            meta: self.meta("fd", Placement::ranked("fields", index)),
            name: ident.to_string(),
            ty: self.import_type(&field.ty)?,
        })
    }

    fn import_enum(
        &mut self,
        node: &syn::ItemEnum,
        placement: Placement,
    ) -> Result<ast::Item, ImportError> {
        self.check_attrs(&node.attrs, "enum")?;
        self.ensure_inherited_visibility(&node.vis, "enum visibility modifiers are unsupported")?;
        self.ensure_no_generics(&node.generics, "enum generics are unsupported")?;
        let variants = node
            .variants
            .iter()
            .enumerate()
            .map(|(index, variant)| self.import_variant(variant, index))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ast::Item::Enum(ast::ItemEnum {
            meta: self.meta("en", placement),
            name: node.ident.to_string(),
            variants,
        }))
    }

    fn import_variant(
        &mut self,
        variant: &syn::Variant,
        index: usize,
    ) -> Result<ast::Variant, ImportError> {
        self.check_attrs(&variant.attrs, "enum variant")?;
        if !matches!(variant.fields, syn::Fields::Unit) {
            return Err(unsupported(
                variant,
                "only unit enum variants are supported in Rust import",
            ));
        }
        if variant.discriminant.is_some() {
            return Err(unsupported(
                variant,
                "enum variant discriminants are unsupported",
            ));
        }
        Ok(ast::Variant {
            meta: self.meta("v", Placement::ranked("variants", index)),
            name: variant.ident.to_string(),
        })
    }

    fn import_fn(
        &mut self,
        node: &syn::ItemFn,
        placement: Placement,
    ) -> Result<ast::Item, ImportError> {
        self.check_attrs(&node.attrs, "function")?;
        self.ensure_inherited_visibility(
            &node.vis,
            "function visibility modifiers are unsupported",
        )?;
        self.ensure_supported_signature(&node.sig)?;
        let params = node
            .sig
            .inputs
            .iter()
            .enumerate()
            .map(|(index, arg)| self.import_param(arg, index))
            .collect::<Result<Vec<_>, _>>()?;
        let ret_ty = match &node.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => Some(self.import_type(ty)?),
        };
        Ok(ast::Item::Fn(ast::ItemFn {
            meta: self.meta("f", placement),
            name: node.sig.ident.to_string(),
            params,
            ret_ty,
            body: self.import_block(&node.block)?,
        }))
    }

    fn import_param(&mut self, arg: &syn::FnArg, index: usize) -> Result<ast::Param, ImportError> {
        let syn::FnArg::Typed(node) = arg else {
            return Err(unsupported(
                arg,
                "method receivers are unsupported; only free functions are supported",
            ));
        };
        self.check_attrs(&node.attrs, "function parameter")?;
        let name = self.import_param_name(&node.pat)?;
        Ok(ast::Param {
            meta: self.meta("p", Placement::ranked("params", index)),
            name,
            ty: self.import_type(&node.ty)?,
        })
    }

    fn import_param_name(&mut self, pat: &syn::Pat) -> Result<String, ImportError> {
        match pat {
            syn::Pat::Ident(node) => {
                self.check_attrs(&node.attrs, "function parameter pattern")?;
                if node.by_ref.is_some() || node.mutability.is_some() || node.subpat.is_some() {
                    return Err(unsupported(
                        node,
                        "parameter patterns must be plain identifiers",
                    ));
                }
                Ok(node.ident.to_string())
            }
            _ => Err(unsupported(
                pat,
                "parameter patterns must be plain identifiers",
            )),
        }
    }

    fn import_block(&mut self, block: &syn::Block) -> Result<ast::Block, ImportError> {
        let stmts = block
            .stmts
            .iter()
            .enumerate()
            .map(|(index, stmt)| self.import_stmt(stmt, index))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ast::Block { meta: None, stmts })
    }

    fn import_stmt(&mut self, stmt: &syn::Stmt, index: usize) -> Result<ast::Stmt, ImportError> {
        match stmt {
            syn::Stmt::Local(node) => {
                self.check_attrs(&node.attrs, "let statement")?;
                let Some(init) = &node.init else {
                    return Err(unsupported(
                        node,
                        "`let` statements without initializers are unsupported",
                    ));
                };
                if init.diverge.is_some() {
                    return Err(unsupported(node, "`let ... else` is unsupported"));
                }
                Ok(ast::Stmt::Let(ast::StmtLet {
                    meta: self.meta("s", Placement::ranked("body", index)),
                    pat: self.import_pattern(&node.pat)?,
                    value: self.import_expr(&init.expr)?,
                }))
            }
            syn::Stmt::Item(item) => Ok(ast::Stmt::Item(
                self.import_item(item, Placement::ranked("body", index))?,
            )),
            syn::Stmt::Expr(expr, semi) => Ok(ast::Stmt::Expr(ast::StmtExpr {
                meta: self.meta("s", Placement::ranked("body", index)),
                expr: self.import_expr(expr)?,
                has_semi: semi.is_some(),
            })),
            syn::Stmt::Macro(node) => Err(unsupported(node, "statement macros are unsupported")),
        }
    }

    fn import_pattern(&mut self, pat: &syn::Pat) -> Result<ast::Pattern, ImportError> {
        match pat {
            syn::Pat::Ident(node) => {
                self.check_attrs(&node.attrs, "pattern")?;
                if node.by_ref.is_some() || node.mutability.is_some() || node.subpat.is_some() {
                    return Err(unsupported(
                        node,
                        "only plain identifier and wildcard patterns are supported",
                    ));
                }
                Ok(ast::Pattern::Ident(ast::PatIdent {
                    meta: Some(self.detached_meta("pt")),
                    name: node.ident.to_string(),
                }))
            }
            syn::Pat::Wild(node) => {
                self.check_attrs(&node.attrs, "pattern")?;
                Ok(ast::Pattern::Wild(ast::PatWild {
                    meta: Some(self.detached_meta("pt")),
                }))
            }
            _ => Err(unsupported(
                pat,
                "only plain identifier and wildcard patterns are supported",
            )),
        }
    }

    fn import_type(&mut self, ty: &syn::Type) -> Result<ast::Type, ImportError> {
        match ty {
            syn::Type::Path(node) => Ok(ast::Type::Path(ast::TypePath {
                meta: self.detached_meta("t"),
                path: self.import_path(&node.path, node.qself.is_some(), "type paths")?,
            })),
            _ => Err(unsupported(ty, "only path types are supported")),
        }
    }

    fn import_expr(&mut self, expr: &syn::Expr) -> Result<ast::Expr, ImportError> {
        self.import_expr_with_root_meta(expr, true)
    }

    fn import_expr_with_root_meta(
        &mut self,
        expr: &syn::Expr,
        allow_root_meta: bool,
    ) -> Result<ast::Expr, ImportError> {
        match expr {
            syn::Expr::Path(node) => {
                self.check_attrs(&node.attrs, "expression")?;
                Ok(ast::Expr::Path(ast::ExprPath {
                    meta: allow_root_meta.then(|| self.detached_meta("e")),
                    path: self.import_path(&node.path, node.qself.is_some(), "expression paths")?,
                }))
            }
            syn::Expr::Lit(node) => {
                self.check_attrs(&node.attrs, "expression")?;
                Ok(ast::Expr::Lit(ast::ExprLit {
                    meta: allow_root_meta.then(|| self.detached_meta("l")),
                    value: self.import_lit(&node.lit, node)?,
                }))
            }
            syn::Expr::Paren(node) => {
                self.check_attrs(&node.attrs, "expression")?;
                Ok(ast::Expr::Group(ast::ExprGroup {
                    meta: allow_root_meta.then(|| self.detached_meta("e")),
                    expr: Box::new(self.import_expr_with_root_meta(&node.expr, true)?),
                }))
            }
            syn::Expr::Group(node) => {
                self.check_attrs(&node.attrs, "expression")?;
                Ok(ast::Expr::Group(ast::ExprGroup {
                    meta: allow_root_meta.then(|| self.detached_meta("e")),
                    expr: Box::new(self.import_expr_with_root_meta(&node.expr, true)?),
                }))
            }
            syn::Expr::Binary(node) => {
                self.check_attrs(&node.attrs, "expression")?;
                Ok(ast::Expr::Binary(ast::ExprBinary {
                    meta: allow_root_meta.then(|| self.detached_meta("e")),
                    lhs: Box::new(self.import_expr_with_root_meta(&node.left, false)?),
                    op: self.import_bin_op(&node.op)?,
                    rhs: Box::new(self.import_expr_with_root_meta(&node.right, true)?),
                }))
            }
            syn::Expr::Unary(node) => {
                self.check_attrs(&node.attrs, "expression")?;
                let syn::UnOp::Neg(_) = node.op else {
                    return Err(unsupported(
                        node,
                        "only unary minus is supported in expressions",
                    ));
                };
                Ok(ast::Expr::Unary(ast::ExprUnary {
                    meta: allow_root_meta.then(|| self.detached_meta("e")),
                    op: ast::UnaryOp::Neg,
                    expr: Box::new(self.import_expr_with_root_meta(&node.expr, true)?),
                }))
            }
            syn::Expr::Call(node) => {
                self.check_attrs(&node.attrs, "expression")?;
                Ok(ast::Expr::Call(ast::ExprCall {
                    meta: allow_root_meta.then(|| self.detached_meta("e")),
                    callee: Box::new(self.import_expr_with_root_meta(&node.func, false)?),
                    args: node
                        .args
                        .iter()
                        .map(|arg| self.import_expr_with_root_meta(arg, true))
                        .collect::<Result<Vec<_>, _>>()?,
                }))
            }
            syn::Expr::Match(node) => {
                self.check_attrs(&node.attrs, "expression")?;
                Ok(ast::Expr::Match(ast::ExprMatch {
                    meta: allow_root_meta.then(|| self.detached_meta("e")),
                    scrutinee: Box::new(self.import_expr_with_root_meta(&node.expr, true)?),
                    arms: node
                        .arms
                        .iter()
                        .enumerate()
                        .map(|(index, arm)| self.import_arm(arm, index))
                        .collect::<Result<Vec<_>, _>>()?,
                }))
            }
            syn::Expr::Block(node) => {
                self.check_attrs(&node.attrs, "expression")?;
                if node.label.is_some() {
                    return Err(unsupported(
                        node,
                        "labeled block expressions are unsupported",
                    ));
                }
                let mut block = self.import_block(&node.block)?;
                block.meta = allow_root_meta.then(|| self.detached_meta("e"));
                Ok(ast::Expr::Block(block))
            }
            _ => Err(unsupported(
                expr,
                "expression is outside the currently supported Rust import subset",
            )),
        }
    }

    fn import_lit(
        &mut self,
        lit: &syn::Lit,
        span_node: &impl Spanned,
    ) -> Result<ast::Literal, ImportError> {
        match lit {
            syn::Lit::Int(node) => {
                if !node.suffix().is_empty() {
                    return Err(unsupported(
                        span_node,
                        "integer literal suffixes are unsupported",
                    ));
                }
                node.base10_parse::<i64>()
                    .map(ast::Literal::Int)
                    .map_err(|_| {
                        unsupported(
                            span_node,
                            "only decimal integer literals that fit in `i64` are supported",
                        )
                    })
            }
            syn::Lit::Str(node) => Ok(ast::Literal::Str(node.value())),
            _ => Err(unsupported(
                span_node,
                "only integer and string literals are supported",
            )),
        }
    }

    fn import_bin_op(&mut self, op: &syn::BinOp) -> Result<ast::BinaryOp, ImportError> {
        match op {
            syn::BinOp::Add(_) => Ok(ast::BinaryOp::Add),
            syn::BinOp::Sub(_) => Ok(ast::BinaryOp::Sub),
            syn::BinOp::Lt(_) => Ok(ast::BinaryOp::Lt),
            _ => Err(unsupported(
                op,
                "only `+`, `-`, and `<` binary operators are supported",
            )),
        }
    }

    fn import_arm(&mut self, arm: &syn::Arm, index: usize) -> Result<ast::MatchArm, ImportError> {
        self.check_attrs(&arm.attrs, "match arm")?;
        Ok(ast::MatchArm {
            meta: self.meta("a", Placement::ranked("arms", index)),
            pat: self.import_pattern(&arm.pat)?,
            guard: arm
                .guard
                .as_ref()
                .map(|(_, expr)| self.import_expr(expr))
                .transpose()?,
            body: self.import_expr(&arm.body)?,
        })
    }

    fn import_path(
        &mut self,
        path: &syn::Path,
        has_qself: bool,
        context: &str,
    ) -> Result<ast::Path, ImportError> {
        if has_qself {
            return Err(unsupported(
                path,
                format!("qualified self in {context} is unsupported"),
            ));
        }
        if path.leading_colon.is_some() {
            return Err(unsupported(
                path,
                format!("leading `::` in {context} is unsupported"),
            ));
        }
        let mut segments = Vec::with_capacity(path.segments.len());
        for segment in &path.segments {
            if !matches!(segment.arguments, syn::PathArguments::None) {
                return Err(unsupported(
                    segment,
                    format!("generic arguments in {context} are unsupported"),
                ));
            }
            segments.push(segment.ident.to_string());
        }
        Ok(ast::Path { segments })
    }

    fn ensure_supported_signature(&self, sig: &syn::Signature) -> Result<(), ImportError> {
        if sig.constness.is_some() {
            return Err(unsupported(sig, "`const fn` is unsupported"));
        }
        if sig.asyncness.is_some() {
            return Err(unsupported(sig, "`async fn` is unsupported"));
        }
        if sig.unsafety.is_some() {
            return Err(unsupported(sig, "`unsafe fn` is unsupported"));
        }
        if sig.abi.is_some() {
            return Err(unsupported(sig, "extern function ABIs are unsupported"));
        }
        self.ensure_no_generics(&sig.generics, "function generics are unsupported")?;
        if sig.variadic.is_some() {
            return Err(unsupported(sig, "variadic functions are unsupported"));
        }
        Ok(())
    }

    fn ensure_no_generics(
        &self,
        generics: &syn::Generics,
        message: &str,
    ) -> Result<(), ImportError> {
        if !generics.params.is_empty() || generics.where_clause.is_some() {
            return Err(unsupported(generics, message));
        }
        Ok(())
    }

    fn ensure_inherited_visibility(
        &self,
        vis: &syn::Visibility,
        message: &str,
    ) -> Result<(), ImportError> {
        if !matches!(vis, syn::Visibility::Inherited) {
            return Err(unsupported(vis, message));
        }
        Ok(())
    }

    fn check_attrs(&self, attrs: &[syn::Attribute], context: &str) -> Result<(), ImportError> {
        for attr in attrs {
            if attr.path().is_ident("doc") {
                continue;
            }
            return Err(unsupported(
                attr,
                format!("unsupported attribute on {context}; only doc comments are ignored"),
            ));
        }
        Ok(())
    }

    fn meta(&mut self, prefix: &'static str, placement: Placement) -> ast::Meta {
        ast::Meta {
            id: self.ids.next(prefix),
            rank: placement.rank,
            anchor: None,
            slot: placement.slot.map(str::to_owned),
            span: None,
        }
    }

    fn detached_meta(&mut self, prefix: &'static str) -> ast::Meta {
        ast::Meta {
            id: self.ids.next(prefix),
            rank: None,
            anchor: None,
            slot: None,
            span: None,
        }
    }
}

fn unsupported<T: Spanned>(node: &T, message: impl Into<String>) -> ImportError {
    ImportError::Unsupported(syn::Error::new(node.span(), message.into()))
}

#[cfg(test)]
mod tests {
    use super::import_source;
    use crate::lower_file;
    use draxl_ast as ast;

    fn lower_imported(imported: &ast::File) -> String {
        lower_file(imported)
    }

    #[test]
    fn imports_a_simple_function_body() {
        let source = r#"
mod demo {
    fn add_one(x: i64) -> i64 {
        let y = (x + 1);
        y
    }
}
"#;

        let imported = import_source(source).expect("simple function should import");

        let [ast::Item::Mod(module)] = imported.items.as_slice() else {
            panic!("expected one imported module, found {:?}", imported.items);
        };
        assert_eq!(module.meta.id, "m0001");
        let [ast::Item::Fn(function)] = module.items.as_slice() else {
            panic!("expected one imported function, found {:?}", module.items);
        };
        assert_eq!(function.meta.id, "f0001");
        assert_eq!(function.params[0].meta.id, "p0001");
        assert_eq!(function.body.stmts.len(), 2);
        assert_eq!(
            lower_imported(&imported),
            "mod demo {\n  fn add_one(x: i64) -> i64 {\n    let y = (x + 1);\n    y\n  }\n}\n\n"
        );
    }

    #[test]
    fn imports_match_use_and_shapes() {
        let source = r#"
mod shapes {
    use std::cmp::{self, *};

    struct Point {
        x: i64,
        y: i64,
    }

    enum Color {
        Red,
        Green,
    }

    fn abs(x: i64) -> i64 {
        match x {
            n if (n < 0) => (-n),
            _ => x,
        }
    }
}
"#;

        let imported = import_source(source).expect("supported Rust subset should import");

        let [ast::Item::Mod(module)] = imported.items.as_slice() else {
            panic!("expected one imported module, found {:?}", imported.items);
        };
        assert_eq!(module.items.len(), 4);
        assert!(matches!(
            &module.items[0],
            ast::Item::Use(node) if node.meta.id == "u0001"
        ));
        assert!(matches!(
            &module.items[1],
            ast::Item::Struct(node) if node.meta.id == "st0001" && node.fields.len() == 2
        ));
        assert!(matches!(
            &module.items[2],
            ast::Item::Enum(node) if node.meta.id == "en0001" && node.variants.len() == 2
        ));
        assert!(matches!(
            &module.items[3],
            ast::Item::Fn(node) if node.name == "abs" && node.body.stmts.len() == 1
        ));
        assert_eq!(
            lower_imported(&imported),
            "mod shapes {\n  use std::cmp::{self, *};\n\n  struct Point {\n    x: i64,\n    y: i64,\n  }\n\n  enum Color {\n    Red,\n    Green,\n  }\n\n  fn abs(x: i64) -> i64 {\n    match x {\n      n if (n < 0) => (-n),\n      _ => x,\n    }\n  }\n}\n\n"
        );
    }

    #[test]
    fn rejects_generics_and_visibility() {
        let error =
            import_source("pub fn run<T>(x: T) {}\n").expect_err("unsupported syntax should fail");
        let message = error.to_string();
        assert!(
            message.contains("visibility modifiers") || message.contains("generics"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn rejects_macros_and_tuple_structs() {
        let macro_error = import_source("fn run() { println!(\"hi\"); }\n")
            .expect_err("statement macro should fail");
        assert!(
            macro_error
                .to_string()
                .contains("statement macros are unsupported")
                || macro_error.to_string().contains("expression is outside"),
            "unexpected error: {macro_error}"
        );

        let struct_error =
            import_source("struct Pair(i64, i64);\n").expect_err("tuple struct should fail");
        assert!(
            struct_error
                .to_string()
                .contains("only named-field structs are supported"),
            "unexpected error: {struct_error}"
        );
    }
}
