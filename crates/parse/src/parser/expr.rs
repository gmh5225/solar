use crate::{PResult, Parser};
use sulk_ast::{ast::*, token::*};
use sulk_interface::kw;

impl<'a> Parser<'a> {
    /// Parses an expression.
    pub fn parse_expr(&mut self) -> PResult<'a, Box<Expr>> {
        let expr = self.parse_binary_expr(4)?;
        if self.eat(&TokenKind::Question) {
            let then = self.parse_expr()?;
            self.expect(&TokenKind::Colon)?;
            let else_ = self.parse_expr()?;
            let span = expr.span.to(self.prev_token.span);
            Ok(Box::new(Expr { span, kind: ExprKind::Ternary(expr, then, else_) }))
        } else {
            let kind = if let Some(binop_eq) = self.token.as_binop_eq() {
                Some(binop_eq)
            } else if self.token.kind == TokenKind::Eq {
                None
            } else {
                return Ok(expr);
            };
            self.bump(); // binop token
            let rhs = self.parse_expr()?;
            let span = expr.span.to(self.prev_token.span);
            Ok(Box::new(Expr { span, kind: ExprKind::Assign(expr, kind, rhs) }))
        }
    }

    /// Parses a binary expression.
    fn parse_binary_expr(&mut self, min_precedence: usize) -> PResult<'a, Box<Expr>> {
        let mut expr = self.parse_unary_expr()?;
        let mut precedence = token_precedence(&self.token);
        while precedence >= min_precedence {
            while token_precedence(&self.token) == precedence {
                // Parse a**b**c as a**(b**c)
                let next_precedence = if self.token.kind == TokenKind::BinOp(BinOpToken::Star) {
                    precedence + 1
                } else {
                    precedence
                };

                let token = self.token.clone();
                self.bump(); // binop token

                let rhs = self.parse_binary_expr(next_precedence)?;

                let span = expr.span.to(self.prev_token.span);

                let kind = if let Some(binop) = token.as_binop() {
                    ExprKind::Binary(expr, binop, rhs)
                } else if let Some(binop_eq) = token.as_binop_eq() {
                    ExprKind::Assign(expr, Some(binop_eq), rhs)
                } else if token.kind == TokenKind::Eq {
                    ExprKind::Assign(expr, None, rhs)
                } else {
                    let msg = format!("unkown binop token: {token:?}");
                    self.dcx().bug(msg).span(span).emit();
                };
                expr = Box::new(Expr { span, kind });
            }
            precedence -= 1;
        }
        Ok(expr)
    }

    /// Parses a unary expression.
    fn parse_unary_expr(&mut self) -> PResult<'a, Box<Expr>> {
        if self.eat(&TokenKind::BinOp(BinOpToken::Plus)) {
            self.dcx().err("unary plus is not supported").emit();
        }

        let lo = self.token.span;
        if self.eat_keyword(kw::Delete) {
            self.parse_unary_expr().map(|expr| {
                let span = lo.to(self.prev_token.span);
                Box::new(Expr { span, kind: ExprKind::Delete(expr) })
            })
        } else if let Some(unop) = self.token.as_unop(false) {
            self.bump(); // unop
            self.parse_unary_expr().map(|expr| {
                let span = lo.to(self.prev_token.span);
                Box::new(Expr { span, kind: ExprKind::Unary(unop, expr) })
            })
        } else {
            self.parse_lhs_expr().map(|expr| {
                if let Some(unop) = self.token.as_unop(true) {
                    self.bump(); // unop
                    let span = lo.to(self.prev_token.span);
                    Box::new(Expr { span, kind: ExprKind::Unary(unop, expr) })
                } else {
                    expr
                }
            })
        }
    }

    /// Parses a primary left-hand-side expression.
    fn parse_lhs_expr(&mut self) -> PResult<'a, Box<Expr>> {
        let lo = self.token.span;
        let mut expr = if self.eat_keyword(kw::New) {
            self.parse_type().map(|ty| {
                let span = lo.to(self.prev_token.span);
                Box::new(Expr { span, kind: ExprKind::New(ty) })
            })
        } else if self.eat_keyword(kw::Payable) {
            Ok(Box::new(Expr { span: lo, kind: ExprKind::Payable }))
        } else {
            self.parse_primary_expr()
        }?;
        loop {
            let kind = if self.eat(&TokenKind::Dot) {
                // expr.member
                let member = self.parse_ident_any()?;
                ExprKind::Member(expr, member)
            } else if self.check(&TokenKind::OpenDelim(Delimiter::Parenthesis)) {
                // expr(args)
                let args = self.parse_call_args()?;
                ExprKind::Call(expr, args)
            } else if self.eat(&TokenKind::OpenDelim(Delimiter::Bracket)) {
                // expr[], expr[start?], expr[start?:end?]
                let kind = if self.check(&TokenKind::CloseDelim(Delimiter::Bracket)) {
                    let start =
                        if self.check(&TokenKind::Colon) { None } else { Some(self.parse_expr()?) };
                    if self.eat_noexpect(&TokenKind::Colon) {
                        // expr[start?:end?]
                        let end = if self.check(&TokenKind::CloseDelim(Delimiter::Bracket)) {
                            None
                        } else {
                            Some(self.parse_expr()?)
                        };
                        IndexKind::Range(start, end)
                    } else {
                        // expr[start?]
                        IndexKind::Index(start)
                    }
                } else {
                    // expr[]
                    IndexKind::Index(None)
                };
                self.expect(&TokenKind::CloseDelim(Delimiter::Bracket))?;
                ExprKind::Index(expr, kind)
            } else if self.check(&TokenKind::OpenDelim(Delimiter::Brace)) {
                // This may be `try` statement block.
                if !self.look_ahead(1).is_ident() || self.look_ahead(2).kind != TokenKind::Colon {
                    break;
                }

                // expr{args}
                let args = self.parse_named_args()?;
                ExprKind::CallOptions(expr, args)
            } else {
                break;
            };
            let span = lo.to(self.prev_token.span);
            expr = Box::new(Expr { span, kind });
        }
        Ok(expr)
    }

    /// Parses a primary expression.
    fn parse_primary_expr(&mut self) -> PResult<'a, Box<Expr>> {
        let lo = self.token.span;
        let kind = if self.token.is_lit() || self.token.is_bool_lit() {
            let (lit, sub) = self.parse_lit_with_subdenomination()?;
            ExprKind::Lit(lit, sub)
        } else if self.eat_keyword(kw::Type) {
            self.expect(&TokenKind::OpenDelim(Delimiter::Parenthesis))?;
            let ty = self.parse_type()?;
            self.expect(&TokenKind::CloseDelim(Delimiter::Parenthesis))?;
            ExprKind::TypeCall(ty)
        } else if self.check_elementary_type() {
            let ty = self.parse_type()?;
            ExprKind::Type(ty)
        } else if self.check_ident() {
            let ident = self.parse_ident()?;
            ExprKind::Ident(ident)
        } else if self.check(&TokenKind::OpenDelim(Delimiter::Parenthesis))
            || self.check(&TokenKind::OpenDelim(Delimiter::Bracket))
        {
            // Array or tuple expression.
            let TokenKind::OpenDelim(close_delim) = self.token.kind else { unreachable!() };
            self.bump(); // open delim
            let is_array = close_delim == Delimiter::Bracket;
            let list = self.parse_seq_optional_items(close_delim, |this| this.parse_expr())?;
            if is_array {
                if !list.iter().all(Option::is_some) {
                    let msg = "array expression components cannot be empty";
                    let span = lo.to(self.prev_token.span);
                    return Err(self.dcx().err(msg).span(span));
                }
                // SAFETY: All elements are checked to be `Some` above.
                ExprKind::Array(unsafe { vec_option_box_unwrap_unchecked(list) })
            } else {
                ExprKind::Tuple(list)
            }
        } else {
            return self.unexpected();
        };
        let span = lo.to(self.prev_token.span);
        Ok(Box::new(Expr { span, kind }))
    }

    /// Parses a list of function call arguments.
    pub(super) fn parse_call_args(&mut self) -> PResult<'a, CallArgs> {
        if self.look_ahead(1).kind == TokenKind::OpenDelim(Delimiter::Brace) {
            self.expect(&TokenKind::OpenDelim(Delimiter::Parenthesis))?;
            let args = self.parse_named_args().map(CallArgs::Named)?;
            self.expect(&TokenKind::CloseDelim(Delimiter::Parenthesis))?;
            Ok(args)
        } else {
            self.parse_unnamed_args().map(CallArgs::Unnamed)
        }
    }

    /// Parses a list of named arguments: `{a: b, c: d, ...}`
    fn parse_named_args(&mut self) -> PResult<'a, NamedArgList> {
        self.parse_delim_comma_seq(Delimiter::Brace, Self::parse_named_arg).map(|(x, _)| x)
    }

    /// Parses a single named argument: `a: b`.
    fn parse_named_arg(&mut self) -> PResult<'a, NamedArg> {
        let name = self.parse_ident()?;
        self.expect(&TokenKind::Colon)?;
        let value = self.parse_expr()?;
        Ok(NamedArg { name, value })
    }

    /// Parses a list of expressions: `(a, b, c, ...)`.
    fn parse_unnamed_args(&mut self) -> PResult<'a, Vec<Box<Expr>>> {
        self.parse_paren_comma_seq(Self::parse_expr).map(|(x, _)| x)
    }
}

fn token_precedence(t: &Token) -> usize {
    use BinOpToken::*;
    use TokenKind::*;
    match t.kind {
        Question => 3,
        BinOpEq(_) => 2,
        Comma => 1,
        OrOr => 4,
        AndAnd => 5,
        BinOp(Or) => 8,
        BinOp(Caret) => 9,
        BinOp(Shl) => 11,
        BinOp(Sar) => 11,
        BinOp(Shr) => 11,
        BinOp(Plus) => 12,
        BinOp(Minus) => 12,
        BinOp(Star) => 13,
        BinOp(Slash) => 13,
        BinOp(Percent) => 13,
        StarStar => 4,
        Eq => 6,
        Ne => 6,
        Lt => 7,
        Gt => 7,
        Le => 7,
        Ge => 7,
        Walrus => 2,
        _ => 0,
    }
}

/// Converts a vector of `Option<Box<T>>` into a vector of `Box<T>`.
///
/// This only works because `Option<Box<T>>` is guaranteed to be a valid `Box<T>` when `Some` when
/// `T: Sized`.
///
/// # Safety
///
/// All elements of the vector must be `Some`.
#[inline]
unsafe fn vec_option_box_unwrap_unchecked<T>(vec: Vec<Option<Box<T>>>) -> Vec<Box<T>> {
    debug_assert!(vec.iter().all(Option::is_some));
    // SAFETY: Caller must ensure that all elements are `Some`.
    unsafe { std::mem::transmute(vec) }
}
