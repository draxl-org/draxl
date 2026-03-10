use crate::error::{parse_error, ParseError};
use crate::syntax::{PendingMeta, Token, TokenKind};
use draxl_ast::{
    BinaryOp, Block, CommentNode, DocNode, Expr, ExprBinary, ExprCall, ExprGroup, ExprLit,
    ExprMatch, ExprPath, ExprUnary, Field, File, Item, ItemEnum, ItemFn, ItemMod, ItemStruct,
    ItemUse, Literal, MatchArm, Meta, Param, PatIdent, PatWild, Path, Pattern, Span, Stmt,
    StmtExpr, StmtLet, Type, TypePath, UnaryOp, UseGlob, UseGroup, UseName, UsePathTree, UseTree,
    Variant,
};

pub(crate) struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    index: usize,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            source,
            tokens,
            index: 0,
        }
    }

    pub(crate) fn parse_file(&mut self) -> Result<File, ParseError> {
        let mut items = Vec::new();
        while !self.at_eof() {
            items.push(self.parse_item("file_items")?);
        }
        Ok(File { items })
    }

    pub(crate) fn parse_item_fragment(&mut self) -> Result<Item, ParseError> {
        let meta = self.parse_required_meta(None, "item fragment")?;
        let item = self.parse_item_after_meta(meta)?;
        self.expect_fragment_end("item fragment")?;
        Ok(item)
    }

    pub(crate) fn parse_field_fragment(&mut self) -> Result<Field, ParseError> {
        let mut meta = self.parse_required_meta(None, "field fragment")?;
        let (name, _) = self.expect_ident("expected field name")?;
        self.expect_simple(TokenKind::Colon, "expected `:` after field name")?;
        let ty = self.parse_type()?;
        let end = ty
            .meta()
            .span
            .map(|span| span.end)
            .unwrap_or(meta.span.unwrap().end);
        set_meta_end(&mut meta, end);
        self.expect_fragment_end("field fragment")?;
        Ok(Field { meta, name, ty })
    }

    pub(crate) fn parse_variant_fragment(&mut self) -> Result<Variant, ParseError> {
        let mut meta = self.parse_required_meta(None, "variant fragment")?;
        let (name, end) = self.expect_ident("expected variant name")?;
        set_meta_end(&mut meta, end);
        self.expect_fragment_end("variant fragment")?;
        Ok(Variant { meta, name })
    }

    pub(crate) fn parse_param_fragment(&mut self) -> Result<Param, ParseError> {
        let mut meta = self.parse_required_meta(None, "parameter fragment")?;
        let (name, _) = self.expect_ident("expected parameter name")?;
        self.expect_simple(TokenKind::Colon, "expected `:` after parameter name")?;
        let ty = self.parse_type()?;
        let end = ty
            .meta()
            .span
            .map(|span| span.end)
            .unwrap_or(meta.span.unwrap().end);
        set_meta_end(&mut meta, end);
        self.expect_fragment_end("parameter fragment")?;
        Ok(Param { meta, name, ty })
    }

    pub(crate) fn parse_stmt_fragment(&mut self) -> Result<Stmt, ParseError> {
        let meta = self.parse_required_meta(None, "statement fragment")?;
        let stmt = match self.current_kind() {
            TokenKind::DocComment(_) => Stmt::Doc(self.parse_doc_node(meta)?),
            TokenKind::LineComment(_) => Stmt::Comment(self.parse_comment_node(meta)?),
            TokenKind::Mod
            | TokenKind::Use
            | TokenKind::Struct
            | TokenKind::Enum
            | TokenKind::Fn => Stmt::Item(self.parse_item_after_meta(meta)?),
            TokenKind::Let => self.parse_let_stmt_after_meta(meta)?,
            _ => self.parse_expr_stmt_after_meta(meta)?,
        };
        self.expect_fragment_end("statement fragment")?;
        Ok(stmt)
    }

    pub(crate) fn parse_match_arm_fragment(&mut self) -> Result<MatchArm, ParseError> {
        let mut meta = self.parse_required_meta(None, "match arm fragment")?;
        let (pat, _) = self.parse_pattern()?;
        let guard = if self.check_simple(&TokenKind::If) {
            self.bump();
            Some(self.parse_expr()?.0)
        } else {
            None
        };
        self.expect_simple(TokenKind::FatArrow, "expected `=>` in match arm")?;
        let (body, body_end) = self.parse_expr()?;
        set_meta_end(&mut meta, body_end);
        self.expect_fragment_end("match arm fragment")?;
        Ok(MatchArm {
            meta,
            pat,
            guard,
            body,
        })
    }

    pub(crate) fn parse_expr_fragment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_expr()?.0;
        self.expect_fragment_end("expression fragment")?;
        Ok(expr)
    }

    pub(crate) fn parse_type_fragment(&mut self) -> Result<Type, ParseError> {
        let ty = self.parse_type()?;
        self.expect_fragment_end("type fragment")?;
        Ok(ty)
    }

    pub(crate) fn parse_pattern_fragment(&mut self) -> Result<Pattern, ParseError> {
        let pattern = self.parse_pattern()?.0;
        self.expect_fragment_end("pattern fragment")?;
        Ok(pattern)
    }

    pub(crate) fn parse_doc_fragment(&mut self) -> Result<DocNode, ParseError> {
        let meta = self.parse_required_meta(None, "doc fragment")?;
        let doc = self.parse_doc_node(meta)?;
        self.expect_fragment_end("doc fragment")?;
        Ok(doc)
    }

    pub(crate) fn parse_comment_fragment(&mut self) -> Result<CommentNode, ParseError> {
        let meta = self.parse_required_meta(None, "comment fragment")?;
        let comment = self.parse_comment_node(meta)?;
        self.expect_fragment_end("comment fragment")?;
        Ok(comment)
    }

    fn parse_item(&mut self, slot: &str) -> Result<Item, ParseError> {
        let meta = self.parse_required_meta(Some(slot), "item")?;
        self.parse_item_after_meta(meta)
    }

    fn parse_item_after_meta(&mut self, meta: Meta) -> Result<Item, ParseError> {
        match self.current_kind() {
            TokenKind::DocComment(_) => Ok(Item::Doc(self.parse_doc_node(meta)?)),
            TokenKind::LineComment(_) => Ok(Item::Comment(self.parse_comment_node(meta)?)),
            TokenKind::Mod => self.parse_mod_item(meta),
            TokenKind::Use => self.parse_use_item(meta),
            TokenKind::Struct => self.parse_struct_item(meta),
            TokenKind::Enum => self.parse_enum_item(meta),
            TokenKind::Fn => self.parse_fn_item(meta),
            _ => Err(self.error_current(
                "expected a doc comment, line comment, or supported item after metadata prefix",
            )),
        }
    }

    fn parse_mod_item(&mut self, mut meta: Meta) -> Result<Item, ParseError> {
        self.expect_simple(TokenKind::Mod, "expected `mod`")?;
        let (name, _) = self.expect_ident("expected module name")?;
        self.expect_simple(TokenKind::LBrace, "expected `{` after module name")?;
        let mut items = Vec::new();
        while !self.check_simple(&TokenKind::RBrace) {
            if self.at_eof() {
                return Err(self.error_current("expected `}` to close module"));
            }
            items.push(self.parse_item("items")?);
        }
        let end = self
            .expect_simple(TokenKind::RBrace, "expected `}` to close module")?
            .end;
        set_meta_end(&mut meta, end);
        Ok(Item::Mod(ItemMod { meta, name, items }))
    }

    fn parse_use_item(&mut self, mut meta: Meta) -> Result<Item, ParseError> {
        self.expect_simple(TokenKind::Use, "expected `use`")?;
        let (tree, end) = self.parse_use_tree()?;
        let semi = self.expect_simple(TokenKind::Semi, "expected `;` after use item")?;
        set_meta_end(&mut meta, semi.end);
        let _ = end;
        Ok(Item::Use(ItemUse { meta, tree }))
    }

    fn parse_struct_item(&mut self, mut meta: Meta) -> Result<Item, ParseError> {
        self.expect_simple(TokenKind::Struct, "expected `struct`")?;
        let (name, _) = self.expect_ident("expected struct name")?;
        self.expect_simple(TokenKind::LBrace, "expected `{` after struct name")?;
        let mut fields = Vec::new();
        while !self.check_simple(&TokenKind::RBrace) {
            fields.push(self.parse_field()?);
            if self.check_simple(&TokenKind::Comma) {
                self.bump();
            } else if !self.check_simple(&TokenKind::RBrace) {
                return Err(self.error_current("expected `,` or `}` after struct field"));
            }
        }
        let end = self
            .expect_simple(TokenKind::RBrace, "expected `}` to close struct")?
            .end;
        set_meta_end(&mut meta, end);
        Ok(Item::Struct(ItemStruct { meta, name, fields }))
    }

    fn parse_enum_item(&mut self, mut meta: Meta) -> Result<Item, ParseError> {
        self.expect_simple(TokenKind::Enum, "expected `enum`")?;
        let (name, _) = self.expect_ident("expected enum name")?;
        self.expect_simple(TokenKind::LBrace, "expected `{` after enum name")?;
        let mut variants = Vec::new();
        while !self.check_simple(&TokenKind::RBrace) {
            variants.push(self.parse_variant()?);
            if self.check_simple(&TokenKind::Comma) {
                self.bump();
            } else if !self.check_simple(&TokenKind::RBrace) {
                return Err(self.error_current("expected `,` or `}` after enum variant"));
            }
        }
        let end = self
            .expect_simple(TokenKind::RBrace, "expected `}` to close enum")?
            .end;
        set_meta_end(&mut meta, end);
        Ok(Item::Enum(ItemEnum {
            meta,
            name,
            variants,
        }))
    }

    fn parse_fn_item(&mut self, mut meta: Meta) -> Result<Item, ParseError> {
        self.expect_simple(TokenKind::Fn, "expected `fn`")?;
        let (name, _) = self.expect_ident("expected function name")?;
        self.expect_simple(TokenKind::LParen, "expected `(` after function name")?;
        let mut params = Vec::new();
        if !self.check_simple(&TokenKind::RParen) {
            loop {
                params.push(self.parse_param()?);
                if self.check_simple(&TokenKind::Comma) {
                    self.bump();
                    if self.check_simple(&TokenKind::RParen) {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        self.expect_simple(TokenKind::RParen, "expected `)` after parameter list")?;
        let ret_ty = if self.check_simple(&TokenKind::Arrow) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        let (body, end) = self.parse_block()?;
        set_meta_end(&mut meta, end);
        Ok(Item::Fn(ItemFn {
            meta,
            name,
            params,
            ret_ty,
            body,
        }))
    }

    fn parse_field(&mut self) -> Result<Field, ParseError> {
        let mut meta = self.parse_required_meta(Some("fields"), "field")?;
        let (name, _) = self.expect_ident("expected field name")?;
        self.expect_simple(TokenKind::Colon, "expected `:` after field name")?;
        let ty = self.parse_type()?;
        let end = ty
            .meta()
            .span
            .map(|span| span.end)
            .unwrap_or(meta.span.unwrap().end);
        set_meta_end(&mut meta, end);
        Ok(Field { meta, name, ty })
    }

    fn parse_variant(&mut self) -> Result<Variant, ParseError> {
        let mut meta = self.parse_required_meta(Some("variants"), "variant")?;
        let (name, end) = self.expect_ident("expected variant name")?;
        set_meta_end(&mut meta, end);
        Ok(Variant { meta, name })
    }

    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let mut meta = self.parse_required_meta(Some("params"), "parameter")?;
        let (name, _) = self.expect_ident("expected parameter name")?;
        self.expect_simple(TokenKind::Colon, "expected `:` after parameter name")?;
        let ty = self.parse_type()?;
        let end = ty
            .meta()
            .span
            .map(|span| span.end)
            .unwrap_or(meta.span.unwrap().end);
        set_meta_end(&mut meta, end);
        Ok(Param { meta, name, ty })
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let mut meta = self.parse_required_meta(None, "type")?;
        let (path, end) = self.parse_path()?;
        set_meta_end(&mut meta, end);
        Ok(Type::Path(TypePath { meta, path }))
    }

    fn parse_block(&mut self) -> Result<(Block, usize), ParseError> {
        let open = self.expect_simple(TokenKind::LBrace, "expected `{` to start block")?;
        let mut stmts = Vec::new();
        while !self.check_simple(&TokenKind::RBrace) {
            if self.at_eof() {
                return Err(self.error_at(open, "expected `}` to close block"));
            }
            stmts.push(self.parse_stmt()?);
        }
        let close = self.expect_simple(TokenKind::RBrace, "expected `}` to close block")?;
        Ok((Block { meta: None, stmts }, close.end))
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.current_kind() {
            TokenKind::DocComment(_) | TokenKind::LineComment(_) => {
                Err(self
                    .error_current("comments in Draxl source require a preceding metadata prefix"))
            }
            TokenKind::Mod
            | TokenKind::Use
            | TokenKind::Struct
            | TokenKind::Enum
            | TokenKind::Fn
            | TokenKind::Let
            | TokenKind::Match
            | TokenKind::Minus
            | TokenKind::LParen
            | TokenKind::Ident(_)
            | TokenKind::Int(_)
            | TokenKind::Str(_) => Err(self.error_current(
                "semantic block children require a leading metadata prefix such as `@s1[a]`",
            )),
            TokenKind::At => {
                let meta = self.parse_required_meta(Some("body"), "statement")?;
                match self.current_kind() {
                    TokenKind::DocComment(_) => Ok(Stmt::Doc(self.parse_doc_node(meta)?)),
                    TokenKind::LineComment(_) => Ok(Stmt::Comment(self.parse_comment_node(meta)?)),
                    TokenKind::Mod
                    | TokenKind::Use
                    | TokenKind::Struct
                    | TokenKind::Enum
                    | TokenKind::Fn => Ok(Stmt::Item(self.parse_item_after_meta(meta)?)),
                    TokenKind::Let => self.parse_let_stmt_after_meta(meta),
                    _ => self.parse_expr_stmt_after_meta(meta),
                }
            }
            _ => self.parse_expr_stmt_missing_meta(),
        }
    }

    fn parse_let_stmt_after_meta(&mut self, mut meta: Meta) -> Result<Stmt, ParseError> {
        self.expect_simple(TokenKind::Let, "expected `let`")?;
        let (pat, _) = self.parse_pattern()?;
        self.expect_simple(TokenKind::Eq, "expected `=` in let statement")?;
        let (value, value_end) = self.parse_expr()?;
        let semi = self.expect_simple(TokenKind::Semi, "expected `;` after let statement")?;
        let _ = value_end;
        set_meta_end(&mut meta, semi.end);
        Ok(Stmt::Let(StmtLet { meta, pat, value }))
    }

    fn parse_expr_stmt_after_meta(&mut self, mut meta: Meta) -> Result<Stmt, ParseError> {
        let (expr, _) = self.parse_expr()?;
        let has_semi = if self.check_simple(&TokenKind::Semi) {
            let semi = self.bump();
            set_meta_end(&mut meta, semi.span.end);
            true
        } else {
            if !self.check_simple(&TokenKind::RBrace) {
                return Err(self.error_current(
                    "expression statements require `;` unless they are the final block expression",
                ));
            }
            if let Some(expr_meta) = expr.meta().and_then(|expr_meta| expr_meta.span) {
                set_meta_end(&mut meta, expr_meta.end);
            }
            false
        };
        Ok(Stmt::Expr(StmtExpr {
            meta,
            expr,
            has_semi,
        }))
    }

    fn parse_expr_stmt_missing_meta(&mut self) -> Result<Stmt, ParseError> {
        Err(self.error_current(
            "expression statements require a leading metadata prefix such as `@s1[a]`",
        ))
    }

    fn parse_pattern(&mut self) -> Result<(Pattern, usize), ParseError> {
        let mut meta = self.parse_optional_meta(None, "pattern")?;
        match self.current_kind() {
            TokenKind::Ident(_) => {
                let (name, end) = self.expect_ident("expected identifier pattern")?;
                if let Some(meta) = &mut meta {
                    set_meta_end(meta, end);
                }
                Ok((Pattern::Ident(PatIdent { meta, name }), end))
            }
            TokenKind::Underscore => {
                let end = self
                    .expect_simple(TokenKind::Underscore, "expected `_` pattern")?
                    .end;
                if let Some(meta) = &mut meta {
                    set_meta_end(meta, end);
                }
                Ok((Pattern::Wild(PatWild { meta }), end))
            }
            _ => Err(self.error_current("expected identifier or `_` pattern")),
        }
    }

    fn parse_expr(&mut self) -> Result<(Expr, usize), ParseError> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<(Expr, usize), ParseError> {
        let leading_meta = self.parse_optional_meta(None, "expression")?;
        let (mut lhs, mut end) = self.parse_prefix_expr()?;

        loop {
            if self.check_simple(&TokenKind::LParen) {
                if 9 < min_bp {
                    break;
                }
                let (args, call_end) = self.parse_call_args()?;
                lhs = Expr::Call(ExprCall {
                    meta: None,
                    callee: Box::new(lhs),
                    args,
                });
                end = call_end;
                continue;
            }

            let (op, l_bp, r_bp) = match self.current_kind() {
                TokenKind::Plus => (BinaryOp::Add, 3, 4),
                TokenKind::Minus => (BinaryOp::Sub, 3, 4),
                TokenKind::Lt => (BinaryOp::Lt, 1, 2),
                _ => break,
            };

            if l_bp < min_bp {
                break;
            }

            self.bump();
            let (rhs, rhs_end) = self.parse_expr_bp(r_bp)?;
            lhs = Expr::Binary(ExprBinary {
                meta: None,
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            });
            end = rhs_end;
        }

        if let Some(mut meta) = leading_meta {
            set_meta_end(&mut meta, end);
            self.attach_meta(&mut lhs, meta)?;
        }

        Ok((lhs, end))
    }

    fn parse_prefix_expr(&mut self) -> Result<(Expr, usize), ParseError> {
        match self.current_kind() {
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::Minus => {
                self.bump();
                let (expr, end) = self.parse_expr_bp(7)?;
                Ok((
                    Expr::Unary(ExprUnary {
                        meta: None,
                        op: UnaryOp::Neg,
                        expr: Box::new(expr),
                    }),
                    end,
                ))
            }
            TokenKind::LBrace => {
                let (block, end) = self.parse_block()?;
                Ok((Expr::Block(block), end))
            }
            TokenKind::LParen => {
                self.bump();
                let (expr, _) = self.parse_expr()?;
                let end = self
                    .expect_simple(TokenKind::RParen, "expected `)` after grouped expression")?
                    .end;
                Ok((
                    Expr::Group(ExprGroup {
                        meta: None,
                        expr: Box::new(expr),
                    }),
                    end,
                ))
            }
            TokenKind::Ident(_) => {
                let (path, end) = self.parse_path()?;
                Ok((Expr::Path(ExprPath { meta: None, path }), end))
            }
            TokenKind::Int(_) => {
                let (value, end) = self.expect_int("expected integer literal")?;
                Ok((
                    Expr::Lit(ExprLit {
                        meta: None,
                        value: Literal::Int(value),
                    }),
                    end,
                ))
            }
            TokenKind::Str(_) => {
                let (value, end) = self.expect_string("expected string literal")?;
                Ok((
                    Expr::Lit(ExprLit {
                        meta: None,
                        value: Literal::Str(value),
                    }),
                    end,
                ))
            }
            _ => Err(self.error_current("expected expression in the supported Draxl subset")),
        }
    }

    fn parse_match_expr(&mut self) -> Result<(Expr, usize), ParseError> {
        self.expect_simple(TokenKind::Match, "expected `match`")?;
        let (scrutinee, _) = self.parse_expr()?;
        self.expect_simple(TokenKind::LBrace, "expected `{` after match scrutinee")?;
        let mut arms = Vec::new();
        while !self.check_simple(&TokenKind::RBrace) {
            arms.push(self.parse_match_arm()?);
            if self.check_simple(&TokenKind::Comma) {
                self.bump();
            } else if !self.check_simple(&TokenKind::RBrace) {
                return Err(self.error_current("expected `,` or `}` after match arm"));
            }
        }
        let end = self
            .expect_simple(TokenKind::RBrace, "expected `}` to close match expression")?
            .end;
        Ok((
            Expr::Match(ExprMatch {
                meta: None,
                scrutinee: Box::new(scrutinee),
                arms,
            }),
            end,
        ))
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, ParseError> {
        let mut meta = self.parse_required_meta(Some("arms"), "match arm")?;
        let (pat, _) = self.parse_pattern()?;
        let guard = if self.check_simple(&TokenKind::If) {
            self.bump();
            Some(self.parse_expr()?.0)
        } else {
            None
        };
        self.expect_simple(TokenKind::FatArrow, "expected `=>` in match arm")?;
        let (body, body_end) = self.parse_expr()?;
        set_meta_end(&mut meta, body_end);
        Ok(MatchArm {
            meta,
            pat,
            guard,
            body,
        })
    }

    fn parse_call_args(&mut self) -> Result<(Vec<Expr>, usize), ParseError> {
        self.expect_simple(TokenKind::LParen, "expected `(` for call arguments")?;
        let mut args = Vec::new();
        if self.check_simple(&TokenKind::RParen) {
            let end = self
                .expect_simple(TokenKind::RParen, "expected `)` after call arguments")?
                .end;
            return Ok((args, end));
        }

        loop {
            args.push(self.parse_expr()?.0);
            if self.check_simple(&TokenKind::Comma) {
                self.bump();
                if self.check_simple(&TokenKind::RParen) {
                    break;
                }
            } else {
                break;
            }
        }

        let end = self
            .expect_simple(TokenKind::RParen, "expected `)` after call arguments")?
            .end;
        Ok((args, end))
    }

    fn parse_doc_node(&mut self, mut meta: Meta) -> Result<DocNode, ParseError> {
        match self.bump().kind {
            TokenKind::DocComment(text) => {
                let end = self.previous().span.end;
                set_meta_end(&mut meta, end);
                Ok(DocNode { meta, text })
            }
            _ => Err(self.error_previous("expected doc comment after metadata prefix")),
        }
    }

    fn parse_comment_node(&mut self, mut meta: Meta) -> Result<CommentNode, ParseError> {
        match self.bump().kind {
            TokenKind::LineComment(text) => {
                let end = self.previous().span.end;
                set_meta_end(&mut meta, end);
                Ok(CommentNode { meta, text })
            }
            _ => Err(self.error_previous("expected line comment after metadata prefix")),
        }
    }

    fn parse_path(&mut self) -> Result<(Path, usize), ParseError> {
        let (first, mut end) = self.expect_ident("expected path segment")?;
        let mut segments = vec![first];
        while self.check_simple(&TokenKind::DoubleColon) {
            self.bump();
            let (segment, segment_end) = self.expect_ident("expected path segment after `::`")?;
            segments.push(segment);
            end = segment_end;
        }
        Ok((Path { segments }, end))
    }

    fn parse_use_tree(&mut self) -> Result<(UseTree, usize), ParseError> {
        match self.current_kind() {
            TokenKind::Ident(_) => {
                let (name, end) = self.expect_ident("expected `use` tree segment")?;
                if self.check_simple(&TokenKind::DoubleColon) {
                    self.bump();
                    let (tree, tree_end) = self.parse_use_tree()?;
                    Ok((
                        UseTree::Path(UsePathTree {
                            prefix: name,
                            tree: Box::new(tree),
                        }),
                        tree_end,
                    ))
                } else {
                    Ok((UseTree::Name(UseName { name }), end))
                }
            }
            TokenKind::LBrace => self.parse_use_group(),
            TokenKind::Star => {
                let end = self
                    .expect_simple(TokenKind::Star, "expected `*` in `use` tree")?
                    .end;
                Ok((UseTree::Glob(UseGlob), end))
            }
            _ => {
                Err(self.error_current("expected a `use` tree segment, `{...}` group, or `*` glob"))
            }
        }
    }

    fn parse_use_group(&mut self) -> Result<(UseTree, usize), ParseError> {
        self.expect_simple(TokenKind::LBrace, "expected `{` to start `use` group")?;
        let mut items = Vec::new();
        if !self.check_simple(&TokenKind::RBrace) {
            loop {
                let (tree, _) = self.parse_use_tree()?;
                items.push(tree);
                if self.check_simple(&TokenKind::Comma) {
                    self.bump();
                    if self.check_simple(&TokenKind::RBrace) {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        let end = self
            .expect_simple(TokenKind::RBrace, "expected `}` to close `use` group")?
            .end;
        Ok((UseTree::Group(UseGroup { items }), end))
    }

    fn parse_required_meta(
        &mut self,
        slot: Option<&str>,
        context: &str,
    ) -> Result<Meta, ParseError> {
        let pending = self.parse_pending_meta()?.ok_or_else(|| {
            self.error_current(&format!(
                "expected metadata prefix such as `@x1` before {} in Draxl source",
                context
            ))
        })?;
        self.finish_meta(pending, slot)
    }

    fn parse_optional_meta(
        &mut self,
        slot: Option<&str>,
        _context: &str,
    ) -> Result<Option<Meta>, ParseError> {
        let Some(pending) = self.parse_pending_meta()? else {
            return Ok(None);
        };
        Ok(Some(self.finish_meta(pending, slot)?))
    }

    fn parse_pending_meta(&mut self) -> Result<Option<PendingMeta>, ParseError> {
        if !self.check_simple(&TokenKind::At) {
            return Ok(None);
        }
        let start = self.bump().span.start;
        let (id_value, mut end) =
            self.expect_ident("expected metadata identifier after `@` in Draxl source")?;
        let mut rank = None;
        let mut anchor = None;

        if self.check_simple(&TokenKind::LBracket) {
            self.bump();
            let (rank_value, _) =
                self.expect_ident("expected rank identifier inside `[...]` metadata suffix")?;
            rank = Some(rank_value);
            end = self
                .expect_simple(
                    TokenKind::RBracket,
                    "expected `]` after metadata rank suffix",
                )?
                .end;
        }

        if self.check_simple(&TokenKind::Arrow) {
            self.bump();
            let (anchor_value, anchor_end) =
                self.expect_ident("expected anchor identifier after `->` in metadata prefix")?;
            anchor = Some(anchor_value);
            end = anchor_end;
        }

        Ok(Some(PendingMeta {
            id: Some(id_value),
            rank,
            anchor,
            span: Span { start, end },
        }))
    }

    fn finish_meta(&self, pending: PendingMeta, slot: Option<&str>) -> Result<Meta, ParseError> {
        let id = pending.id.ok_or_else(|| {
            self.error_at_span(
                pending.span,
                "Draxl metadata prefix requires an identifier after `@`",
            )
        })?;
        Ok(Meta {
            id,
            rank: pending.rank,
            anchor: pending.anchor,
            slot: slot.map(str::to_owned),
            span: Some(pending.span),
        })
    }

    fn attach_meta(&self, expr: &mut Expr, meta: Meta) -> Result<(), ParseError> {
        match expr {
            Expr::Path(node) => self.set_optional_meta(&mut node.meta, meta),
            Expr::Lit(node) => self.set_optional_meta(&mut node.meta, meta),
            Expr::Group(node) => self.set_optional_meta(&mut node.meta, meta),
            Expr::Binary(node) => self.set_optional_meta(&mut node.meta, meta),
            Expr::Unary(node) => self.set_optional_meta(&mut node.meta, meta),
            Expr::Call(node) => self.set_optional_meta(&mut node.meta, meta),
            Expr::Match(node) => self.set_optional_meta(&mut node.meta, meta),
            Expr::Block(node) => self.set_optional_meta(&mut node.meta, meta),
        }
    }

    fn set_optional_meta(&self, slot: &mut Option<Meta>, meta: Meta) -> Result<(), ParseError> {
        if slot.is_some() {
            let span = meta.span.unwrap_or(Span { start: 0, end: 0 });
            return Err(parse_error(
                self.source,
                span,
                "expression received duplicate Draxl metadata",
            ));
        }
        *slot = Some(meta);
        Ok(())
    }

    fn check_simple(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.current_kind()) == std::mem::discriminant(kind)
    }

    fn current_kind(&self) -> &TokenKind {
        &self.tokens[self.index].kind
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.index - 1]
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens[self.index].clone();
        self.index += 1;
        token
    }

    fn at_eof(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Eof)
    }

    fn expect_simple(&mut self, kind: TokenKind, message: &str) -> Result<Span, ParseError> {
        if self.check_simple(&kind) {
            Ok(self.bump().span)
        } else {
            Err(self.error_current(message))
        }
    }

    fn expect_ident(&mut self, message: &str) -> Result<(String, usize), ParseError> {
        match self.bump().kind {
            TokenKind::Ident(name) => {
                let end = self.previous().span.end;
                Ok((name, end))
            }
            _ => Err(self.error_previous(message)),
        }
    }

    fn expect_int(&mut self, message: &str) -> Result<(i64, usize), ParseError> {
        match self.bump().kind {
            TokenKind::Int(value) => {
                let end = self.previous().span.end;
                Ok((value, end))
            }
            _ => Err(self.error_previous(message)),
        }
    }

    fn expect_string(&mut self, message: &str) -> Result<(String, usize), ParseError> {
        match self.bump().kind {
            TokenKind::Str(value) => {
                let end = self.previous().span.end;
                Ok((value, end))
            }
            _ => Err(self.error_previous(message)),
        }
    }

    fn expect_fragment_end(&self, context: &str) -> Result<(), ParseError> {
        if self.at_eof() {
            Ok(())
        } else {
            Err(self.error_current(&format!("unexpected trailing tokens after {context}")))
        }
    }

    fn error_current(&self, message: &str) -> ParseError {
        self.error_at(self.current().span, message)
    }

    fn error_previous(&self, message: &str) -> ParseError {
        self.error_at(self.previous().span, message)
    }

    fn error_at_span(&self, span: Span, message: &str) -> ParseError {
        self.error_at(span, message)
    }

    fn error_at(&self, span: Span, message: &str) -> ParseError {
        parse_error(self.source, span, message)
    }
}

fn set_meta_end(meta: &mut Meta, end: usize) {
    if let Some(span) = &mut meta.span {
        span.end = end;
    }
}
