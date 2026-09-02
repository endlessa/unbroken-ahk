//! Recursive-descent parser for the SCAD-compatible subset.
//!
//! Grammar (phase 2 slice):
//!   program  := stmt*
//!   stmt     := IDENT '=' expr ';'
//!             | 'module' IDENT '(' params ')' child
//!             | 'function' IDENT '(' params ')' '=' expr ';'
//!             | 'for' '(' bindings ')' child
//!             | 'if' '(' expr ')' child ['else' child]
//!             | ('let'|'assign') '(' bindings ')' child
//!             | '{' stmt* '}'
//!             | IDENT '(' args ')' child
//!   child    := ';' | '{' stmt* '}' | stmt          (bare single child)
//!   expr     := ternary, with let/echo/assert expression forms and
//!               'function' '(' params ')' expr literals at primary level
//!   vector   := '[' vec-item (',' vec-item)* ']' where a vec-item may be
//!               a comprehension clause chain (for / C-style for / each /
//!               if-else / let)

use crate::ast::{Arg, BinOp, Expr, Param, Stmt, VecItem};
use crate::lexer::{lex, Tok, Token};

/// The 2021.01 reserved set: not usable as identifiers in ANY namespace.
/// echo and assert joined it with their 2019.05 expression forms; assign
/// deliberately did NOT (assign = 1; stays legal).
const KEYWORDS: &[&str] = &[
    "module", "function", "if", "else", "for", "let", "each", "echo", "assert",
    "include", "use", "true", "false", "undef",
];

/// Parse on a dedicated large-stack thread so deeply nested input reaches
/// the depth guard (a graceful Err) instead of overflowing the caller's
/// stack — the HTTP handler runs on a small default-stack thread.
pub fn parse(src: &str) -> Result<Vec<Stmt>, String> {
    let src = src.to_string();
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(256 * 1024 * 1024)
            .spawn_scoped(s, move || parse_inner(&src))
            .expect("failed to spawn the parser thread")
            .join()
            .unwrap_or_else(|_| Err("parser crashed on pathological input".into()))
    })
}

fn parse_inner(src: &str) -> Result<Vec<Stmt>, String> {
    let tokens = lex(src)?;
    let mut p = Parser { tokens, pos: 0, depth: 0 };
    let mut stmts = Vec::new();
    while !p.at_end() {
        stmts.push(p.stmt()?);
    }
    Ok(stmts)
}

/// Graceful bound on nesting depth; far above any real script, well below
/// what the large parser stack can hold.
const MAX_PARSE_DEPTH: usize = 2000;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
}

impl Parser {
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos).map(|t| &t.tok)
    }

    fn peek2(&self) -> Option<&Tok> {
        self.tokens.get(self.pos + 1).map(|t| &t.tok)
    }

    fn peek_ident(&self) -> Option<&str> {
        match self.peek() {
            Some(Tok::Ident(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    fn here(&self) -> String {
        match self.tokens.get(self.pos) {
            Some(t) => format!("{:?} at byte {}", t.tok, t.pos),
            None => "end of input".into(),
        }
    }

    fn bump(&mut self) -> Option<Tok> {
        let t = self.tokens.get(self.pos).map(|t| t.tok.clone());
        self.pos += 1;
        t
    }

    fn expect(&mut self, want: &Tok, ctx: &str) -> Result<(), String> {
        if self.peek() == Some(want) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("expected {:?} {} but found {}", want, ctx, self.here()))
        }
    }

    fn ident(&mut self, ctx: &str) -> Result<String, String> {
        match self.bump() {
            Some(Tok::Ident(s)) => Ok(s),
            _ => {
                self.pos = self.pos.saturating_sub(1);
                Err(format!("expected an identifier {} but found {}", ctx, self.here()))
            }
        }
    }

    fn enter(&mut self) -> Result<(), String> {
        self.depth += 1;
        if self.depth > MAX_PARSE_DEPTH {
            return Err(format!("expression nesting exceeds {} levels", MAX_PARSE_DEPTH));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    /// Statement nesting shares the depth guard with expressions —
    /// chained bare children (translate() translate() ... cube();) and
    /// nested blocks must error gracefully, not overflow the stack.
    fn stmt(&mut self) -> Result<Stmt, String> {
        self.enter()?;
        let r = self.stmt_inner();
        self.leave();
        r
    }

    fn stmt_inner(&mut self) -> Result<Stmt, String> {
        if self.peek() == Some(&Tok::LBrace) {
            self.pos += 1;
            return Ok(Stmt::Block(self.block_body()?));
        }
        let name = match self.peek() {
            Some(Tok::Ident(s)) => s.clone(),
            _ => return Err(format!("expected a statement but found {}", self.here())),
        };
        self.pos += 1;

        match name.as_str() {
            "module" => {
                let name = self.ident("after 'module'")?;
                self.no_keyword(&name)?;
                self.expect(&Tok::LParen, "after the module name")?;
                let params = self.params()?;
                let body = self.child()?;
                return Ok(Stmt::ModuleDef { name, params, body });
            }
            "function" => {
                let name = self.ident("after 'function'")?;
                self.no_keyword(&name)?;
                self.expect(&Tok::LParen, "after the function name")?;
                let params = self.params()?;
                self.expect(&Tok::Assign, "after the function parameter list")?;
                let body = self.expr()?;
                self.expect(&Tok::Semi, "after the function body")?;
                return Ok(Stmt::FunctionDef { name, params, body });
            }
            "for" => {
                self.expect(&Tok::LParen, "after 'for'")?;
                let bindings = self.bindings("in for(...)")?;
                self.expect(&Tok::RParen, "to close for(...)")?;
                let body = self.child()?;
                return Ok(Stmt::For { bindings, body });
            }
            // intersection_for is a builtin module, not a reserved word,
            // so only its call form is special; intersection_for = 1; is
            // still a legal assignment.
            "intersection_for" if self.peek() == Some(&Tok::LParen) => {
                self.pos += 1;
                let bindings = self.bindings("in intersection_for(...)")?;
                self.expect(&Tok::RParen, "to close intersection_for(...)")?;
                let body = self.child()?;
                return Ok(Stmt::IntersectionFor { bindings, body });
            }
            "if" => {
                self.expect(&Tok::LParen, "after 'if'")?;
                let cond = self.expr()?;
                self.expect(&Tok::RParen, "to close if(...)")?;
                let then = self.child()?;
                let els = if self.peek_ident() == Some("else") {
                    self.pos += 1;
                    self.child()?
                } else {
                    Vec::new()
                };
                return Ok(Stmt::If { cond, then, els });
            }
            // echo/assert are reserved words, but their statement CALL
            // forms are of course legal.
            "echo" | "assert" if self.peek() == Some(&Tok::LParen) => {
                self.pos += 1;
                let args = self.args()?;
                self.expect(&Tok::RParen, "to close the argument list")?;
                let children = self.child()?;
                return Ok(Stmt::Call { name, args, children });
            }
            "let" | "assign" if self.peek() == Some(&Tok::LParen) => {
                // assign(...) with a child is the deprecated statement;
                // 'assign' is not reserved, so 'assign = 1;' stays legal
                // (handled by the guard on LParen above).
                self.pos += 1;
                let bindings = self.bindings("in the binding list")?;
                self.expect(&Tok::RParen, "to close the binding list")?;
                let body = self.child()?;
                return Ok(Stmt::Let { bindings, body, deprecated_assign: name == "assign" });
            }
            _ => {}
        }

        match self.peek() {
            Some(Tok::Assign) => {
                self.no_keyword(&name)?;
                self.pos += 1;
                let value = self.expr()?;
                self.expect(&Tok::Semi, "after assignment")?;
                Ok(Stmt::Assign { name, value })
            }
            Some(Tok::LParen) => {
                self.no_keyword(&name)?;
                self.pos += 1;
                let args = self.args()?;
                self.expect(&Tok::RParen, "to close the argument list")?;
                let children = self.child()?;
                Ok(Stmt::Call { name, args, children })
            }
            _ => Err(format!(
                "expected '=' or '(' after '{}' but found {}",
                name,
                self.here()
            )),
        }
    }

    fn no_keyword(&self, name: &str) -> Result<(), String> {
        if KEYWORDS.contains(&name) {
            Err(format!("'{}' is a reserved word", name))
        } else {
            Ok(())
        }
    }

    /// name = expr [, name = expr ...] — for `for`, `let`, `assign`.
    fn bindings(&mut self, ctx: &str) -> Result<Vec<(String, Expr)>, String> {
        let mut out = Vec::new();
        if self.peek() == Some(&Tok::RParen) {
            return Ok(out); // let() with an empty binding list is legal
        }
        loop {
            let name = self.ident(ctx)?;
            self.no_keyword(&name)?;
            self.expect(&Tok::Assign, ctx)?;
            let value = self.expr()?;
            out.push((name, value));
            match self.peek() {
                Some(Tok::Comma) => self.pos += 1,
                _ => break,
            }
        }
        Ok(out)
    }

    /// A call's child position: ';' (leaf), a block, or one bare child.
    fn child(&mut self) -> Result<Vec<Stmt>, String> {
        match self.peek() {
            Some(Tok::Semi) => {
                self.pos += 1;
                Ok(Vec::new())
            }
            Some(Tok::LBrace) => {
                self.pos += 1;
                self.block_body()
            }
            _ => Ok(vec![self.stmt()?]),
        }
    }

    fn block_body(&mut self) -> Result<Vec<Stmt>, String> {
        let mut body = Vec::new();
        while self.peek() != Some(&Tok::RBrace) {
            if self.at_end() {
                return Err("unterminated '{' block".into());
            }
            body.push(self.stmt()?);
        }
        self.pos += 1;
        Ok(body)
    }

    /// Declared parameters: IDENT [= default] list, closing ')'.
    fn params(&mut self) -> Result<Vec<Param>, String> {
        let mut params = Vec::new();
        if self.peek() == Some(&Tok::RParen) {
            self.pos += 1;
            return Ok(params);
        }
        loop {
            let name = self.ident("in the parameter list")?;
            self.no_keyword(&name)?;
            let default = if self.peek() == Some(&Tok::Assign) {
                self.pos += 1;
                Some(self.expr()?)
            } else {
                None
            };
            params.push(Param { name, default });
            match self.peek() {
                Some(Tok::Comma) => self.pos += 1,
                _ => break,
            }
        }
        self.expect(&Tok::RParen, "to close the parameter list")?;
        Ok(params)
    }

    fn args(&mut self) -> Result<Vec<Arg>, String> {
        let mut args = Vec::new();
        if self.peek() == Some(&Tok::RParen) {
            return Ok(args);
        }
        loop {
            // IDENT '=' expr is a named argument; anything else positional.
            let named = matches!(
                (self.peek(), self.peek2()),
                (Some(Tok::Ident(_)), Some(Tok::Assign))
            );
            if named {
                let name = match self.bump() {
                    Some(Tok::Ident(s)) => s,
                    _ => unreachable!("guarded by the named check"),
                };
                self.no_keyword(&name)?;
                self.pos += 1; // '='
                let value = self.expr()?;
                args.push(Arg { name: Some(name), value });
            } else {
                let value = self.expr()?;
                args.push(Arg { name: None, value });
            }
            match self.peek() {
                Some(Tok::Comma) => self.pos += 1,
                _ => break,
            }
        }
        Ok(args)
    }

    /// Full precedence chain, per the reference (high to low):
    /// postfix call/index/member > ^ (right-assoc) > unary ! - + >
    /// * / % > + - > < <= >= > > == != > && > || > ?: (right-assoc).
    /// let/echo/assert expression forms and function literals extend
    /// maximally to the right, like the ternary's arms.
    fn expr(&mut self) -> Result<Expr, String> {
        self.enter()?;
        let r = self.expr_inner();
        self.leave();
        r
    }

    fn expr_inner(&mut self) -> Result<Expr, String> {
        match self.peek_ident() {
            Some("let") if self.peek2() == Some(&Tok::LParen) => {
                self.pos += 2;
                let bindings = self.bindings("in let(...)")?;
                self.expect(&Tok::RParen, "to close let(...)")?;
                let body = self.expr()?;
                return Ok(Expr::Let { bindings, body: Box::new(body) });
            }
            Some("echo") if self.peek2() == Some(&Tok::LParen) => {
                self.pos += 2;
                let args = self.args()?;
                self.expect(&Tok::RParen, "to close echo(...)")?;
                let body = self.expr()?;
                return Ok(Expr::EchoExpr { args, body: Box::new(body) });
            }
            Some("assert") if self.peek2() == Some(&Tok::LParen) => {
                self.pos += 2;
                let args = self.args()?;
                self.expect(&Tok::RParen, "to close assert(...)")?;
                let body = self.expr()?;
                return Ok(Expr::AssertExpr { args, body: Box::new(body) });
            }
            Some("function") if self.peek2() == Some(&Tok::LParen) => {
                self.pos += 2;
                let params = self.params()?;
                let body = self.expr()?;
                return Ok(Expr::FnLiteral { params, body: Box::new(body) });
            }
            _ => {}
        }
        self.ternary()
    }

    fn ternary(&mut self) -> Result<Expr, String> {
        let cond = self.or_expr()?;
        if self.peek() == Some(&Tok::Question) {
            self.pos += 1;
            let then = self.expr()?;
            self.expect(&Tok::Colon, "in ternary '?:'")?;
            // Right-associative: the else branch swallows further ?:.
            let els = self.expr()?;
            return Ok(Expr::Ternary {
                cond: Box::new(cond),
                then: Box::new(then),
                els: Box::new(els),
            });
        }
        Ok(cond)
    }

    fn or_expr(&mut self) -> Result<Expr, String> {
        let mut lhs = self.and_expr()?;
        while self.peek() == Some(&Tok::OrOr) {
            self.pos += 1;
            let rhs = self.and_expr()?;
            lhs = Expr::Binary { op: BinOp::Or, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn and_expr(&mut self) -> Result<Expr, String> {
        let mut lhs = self.equality()?;
        while self.peek() == Some(&Tok::AndAnd) {
            self.pos += 1;
            let rhs = self.equality()?;
            lhs = Expr::Binary { op: BinOp::And, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn equality(&mut self) -> Result<Expr, String> {
        let mut lhs = self.relational()?;
        loop {
            let op = match self.peek() {
                Some(Tok::EqEq) => BinOp::Eq,
                Some(Tok::NotEq) => BinOp::Ne,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.relational()?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn relational(&mut self) -> Result<Expr, String> {
        let mut lhs = self.additive()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Lt) => BinOp::Lt,
                Some(Tok::Le) => BinOp::Le,
                Some(Tok::Gt) => BinOp::Gt,
                Some(Tok::Ge) => BinOp::Ge,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.additive()?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn additive(&mut self) -> Result<Expr, String> {
        let mut lhs = self.term()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Plus) => BinOp::Add,
                Some(Tok::Minus) => BinOp::Sub,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.term()?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn term(&mut self) -> Result<Expr, String> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Star) => BinOp::Mul,
                Some(Tok::Slash) => BinOp::Div,
                Some(Tok::Percent) => BinOp::Mod,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.unary()?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        self.enter()?;
        let r = match self.peek() {
            Some(Tok::Minus) => {
                self.pos += 1;
                self.unary().map(|e| Expr::Neg(Box::new(e)))
            }
            Some(Tok::Plus) => {
                self.pos += 1;
                self.unary().map(|e| Expr::Pos(Box::new(e)))
            }
            Some(Tok::Bang) => {
                self.pos += 1;
                self.unary().map(|e| Expr::Not(Box::new(e)))
            }
            _ => self.power(),
        };
        self.leave();
        r
    }

    /// ^ binds TIGHTER than unary minus (-2^2 == -4) but its exponent
    /// operand re-enters unary so 2^-3 parses; right-associative.
    fn power(&mut self) -> Result<Expr, String> {
        let base = self.postfix()?;
        if self.peek() == Some(&Tok::Caret) {
            self.pos += 1;
            let exp = self.unary()?;
            return Ok(Expr::Binary {
                op: BinOp::Pow,
                lhs: Box::new(base),
                rhs: Box::new(exp),
            });
        }
        Ok(base)
    }

    /// Postfix operators bind tightest: f(x), v[i], v.x — chainable
    /// (m[1].z, f(x)[0], fs[i](x), adder(2)(3)).
    fn postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.primary()?;
        loop {
            match self.peek() {
                Some(Tok::LBracket) => {
                    self.pos += 1;
                    let index = self.expr()?;
                    self.expect(&Tok::RBracket, "to close '[' index")?;
                    e = Expr::Index { base: Box::new(e), index: Box::new(index) };
                }
                Some(Tok::Dot) => {
                    self.pos += 1;
                    let name = match self.bump() {
                        Some(Tok::Ident(s)) => s,
                        _ => return Err(format!("expected member name after '.', found {}", self.here())),
                    };
                    e = Expr::Member { base: Box::new(e), name };
                }
                Some(Tok::LParen) => {
                    self.pos += 1;
                    let args = self.args()?;
                    self.expect(&Tok::RParen, "to close the call")?;
                    e = match e {
                        // An identifier callee resolves through the
                        // function namespace; anything else is a
                        // function-value call.
                        Expr::Ident(name) => Expr::Call { name, args },
                        other => Expr::CallValue { callee: Box::new(other), args },
                    };
                }
                _ => break,
            }
        }
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.bump() {
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::Str(s)) => Ok(Expr::Str(s)),
            Some(Tok::Ident(s)) => match s.as_str() {
                "true" => Ok(Expr::Bool(true)),
                "false" => Ok(Expr::Bool(false)),
                "undef" => Ok(Expr::Undef),
                // Reserved words are not expression identifiers ('x = each;'
                // and 'x = if(c);' are syntax errors, not undef).
                kw if KEYWORDS.contains(&kw) => {
                    Err(format!("'{}' is a reserved word", kw))
                }
                _ => Ok(Expr::Ident(s)),
            },
            Some(Tok::LParen) => {
                let e = self.expr()?;
                self.expect(&Tok::RParen, "to close '('")?;
                // A parenthesized bare identifier forces the VALUE path
                // when called: (f)(x) never hits the function namespace.
                Ok(match e {
                    id @ Expr::Ident(_) => Expr::Paren(Box::new(id)),
                    other => other,
                })
            }
            Some(Tok::LBracket) => self.vector_or_range(),
            _ => {
                self.pos = self.pos.saturating_sub(1);
                Err(format!("expected an expression but found {}", self.here()))
            }
        }
    }

    /// After '[': a range [a:b] / [a:s:b], or a vector whose items may be
    /// comprehension clauses.
    fn vector_or_range(&mut self) -> Result<Expr, String> {
        if self.peek() == Some(&Tok::RBracket) {
            self.pos += 1;
            return Ok(Expr::Vector(Vec::new()));
        }
        let first = self.vec_item()?;
        // Only a plain leading expression can begin a range.
        if let VecItem::One(first_expr) = &first {
            if self.peek() == Some(&Tok::Colon) {
                let first_expr = first_expr.clone();
                self.pos += 1;
                let second = self.expr()?;
                let (step, end) = if self.peek() == Some(&Tok::Colon) {
                    self.pos += 1;
                    (Some(Box::new(second)), self.expr()?)
                } else {
                    (None, second)
                };
                self.expect(&Tok::RBracket, "to close the range")?;
                return Ok(Expr::Range {
                    start: Box::new(first_expr),
                    step,
                    end: Box::new(end),
                });
            }
        }
        let mut items = vec![first];
        while self.peek() == Some(&Tok::Comma) {
            self.pos += 1;
            if self.peek() == Some(&Tok::RBracket) {
                break; // tolerate a trailing comma
            }
            items.push(self.vec_item()?);
        }
        self.expect(&Tok::RBracket, "to close the vector")?;
        Ok(Expr::Vector(items))
    }

    /// One vector element: a comprehension clause chain or a plain
    /// expression.
    fn vec_item(&mut self) -> Result<VecItem, String> {
        self.enter()?;
        let r = self.vec_item_inner();
        self.leave();
        r
    }

    fn vec_item_inner(&mut self) -> Result<VecItem, String> {
        match self.peek_ident() {
            Some("each") => {
                self.pos += 1;
                return Ok(VecItem::Each(self.expr()?));
            }
            Some("for") if self.peek2() == Some(&Tok::LParen) => {
                self.pos += 2;
                // C-style form is detected by the ';' after the inits.
                let inits = self.bindings("in for(...)")?;
                if self.peek() == Some(&Tok::Semi) {
                    self.pos += 1;
                    let cond = self.expr()?;
                    self.expect(&Tok::Semi, "after the generator condition")?;
                    let updates = self.bindings("in the generator update list")?;
                    self.expect(&Tok::RParen, "to close for(...)")?;
                    let rest = vec![self.vec_item()?];
                    return Ok(VecItem::CForC { inits, cond, updates, rest });
                }
                self.expect(&Tok::RParen, "to close for(...)")?;
                let rest = vec![self.vec_item()?];
                return Ok(VecItem::CFor { bindings: inits, rest });
            }
            Some("if") if self.peek2() == Some(&Tok::LParen) => {
                self.pos += 2;
                let cond = self.expr()?;
                self.expect(&Tok::RParen, "to close if(...)")?;
                let then = vec![self.vec_item()?];
                let els = if self.peek_ident() == Some("else") {
                    self.pos += 1;
                    vec![self.vec_item()?]
                } else {
                    Vec::new()
                };
                return Ok(VecItem::CIf { cond, then, els });
            }
            Some("let") if self.peek2() == Some(&Tok::LParen) => {
                self.pos += 2;
                let bindings = self.bindings("in let(...)")?;
                self.expect(&Tok::RParen, "to close let(...)")?;
                let rest = vec![self.vec_item()?];
                return Ok(VecItem::CLet { bindings, rest });
            }
            _ => {}
        }
        Ok(VecItem::One(self.expr()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_calls_with_named_args() {
        let ast = parse("translate(v = [1, 2, 3]) cube(2, center = true);").unwrap();
        assert_eq!(ast.len(), 1);
        match &ast[0] {
            Stmt::Call { name, args, children } => {
                assert_eq!(name, "translate");
                assert_eq!(args[0].name.as_deref(), Some("v"));
                assert_eq!(children.len(), 1);
                match &children[0] {
                    Stmt::Call { name, args, children } => {
                        assert_eq!(name, "cube");
                        assert_eq!(args[0].name, None);
                        assert_eq!(args[1].name.as_deref(), Some("center"));
                        assert_eq!(args[1].value, Expr::Bool(true));
                        assert!(children.is_empty());
                    }
                    other => panic!("expected cube call, got {:?}", other),
                }
            }
            other => panic!("expected call, got {:?}", other),
        }
    }

    #[test]
    fn ranges_and_vectors_disambiguate() {
        let ast = parse("for (i = [0 : 2 : 10]) cube(i);").unwrap();
        match &ast[0] {
            Stmt::For { bindings, body } => {
                assert_eq!(bindings[0].0, "i");
                assert!(matches!(bindings[0].1, Expr::Range { step: Some(_), .. }));
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected for, got {:?}", other),
        }
        let ast = parse("x = [1, 2 + 3];").unwrap();
        match &ast[0] {
            Stmt::Assign { value: Expr::Vector(items), .. } => assert_eq!(items.len(), 2),
            other => panic!("expected vector assign, got {:?}", other),
        }
    }

    #[test]
    fn precedence_and_unary_minus() {
        let ast = parse("x = 1 + 2 * -3;").unwrap();
        match &ast[0] {
            Stmt::Assign { value, .. } => match value {
                Expr::Binary { op: BinOp::Add, rhs, .. } => {
                    assert!(matches!(**rhs, Expr::Binary { op: BinOp::Mul, .. }));
                }
                other => panic!("expected top-level add, got {:?}", other),
            },
            other => panic!("expected assign, got {:?}", other),
        }
    }

    #[test]
    fn definitions_control_flow_and_literals_parse() {
        let ast = parse(
            "module m(a, b = 2) { cube(a); } \
             function f(x) = x * 2; \
             if (true) cube(1); else sphere(1); \
             let (a = 1) cube(a); \
             assign (a = 1) cube(a); \
             g = function (x) x + 1; \
             { cube(1); }",
        )
        .unwrap();
        assert!(matches!(&ast[0], Stmt::ModuleDef { name, params, .. }
            if name == "m" && params.len() == 2 && params[1].default.is_some()));
        assert!(matches!(&ast[1], Stmt::FunctionDef { name, .. } if name == "f"));
        assert!(matches!(&ast[2], Stmt::If { els, .. } if els.len() == 1));
        assert!(matches!(&ast[3], Stmt::Let { deprecated_assign: false, .. }));
        assert!(matches!(&ast[4], Stmt::Let { deprecated_assign: true, .. }));
        assert!(matches!(&ast[5], Stmt::Assign { value: Expr::FnLiteral { .. }, .. }));
        assert!(matches!(&ast[6], Stmt::Block(_)));
    }

    #[test]
    fn comprehensions_parse() {
        let ast = parse("x = [for (i = [0:4]) if (i % 2 == 0) i * i, each [7, 8], 9];").unwrap();
        match &ast[0] {
            Stmt::Assign { value: Expr::Vector(items), .. } => {
                assert!(matches!(&items[0], VecItem::CFor { .. }));
                assert!(matches!(&items[1], VecItem::Each(_)));
                assert!(matches!(&items[2], VecItem::One(_)));
            }
            other => panic!("expected vector assign, got {:?}", other),
        }
        let ast = parse("y = [for (a = 0, b = 1; a < 10; a = b, b = a + b) a];").unwrap();
        match &ast[0] {
            Stmt::Assign { value: Expr::Vector(items), .. } => {
                assert!(matches!(&items[0], VecItem::CForC { inits, updates, .. }
                    if inits.len() == 2 && updates.len() == 2));
            }
            other => panic!("expected generator assign, got {:?}", other),
        }
    }

    #[test]
    fn expression_forms_and_value_callees_parse() {
        let ast = parse("x = let (a = 1) echo(a) assert(a > 0) a + 1;").unwrap();
        match &ast[0] {
            Stmt::Assign { value: Expr::Let { body, .. }, .. } => {
                assert!(matches!(**body, Expr::EchoExpr { .. }));
            }
            other => panic!("expected let expr, got {:?}", other),
        }
        let ast = parse("y = fs[1](3) + (function (x) x)(2) + adder(2)(3);").unwrap();
        assert!(matches!(&ast[0], Stmt::Assign { .. }));
    }

    #[test]
    fn errors_name_the_offending_position() {
        let err = parse("cube(1) cube(2)").unwrap_err();
        assert!(err.contains("end of input"), "got: {}", err);
        let err = parse("translate() {").unwrap_err();
        assert!(err.contains("unterminated"), "got: {}", err);
        assert!(parse("module for() {}").is_err());
    }
}
