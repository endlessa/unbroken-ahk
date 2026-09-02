//! Recursive-descent parser for the SCAD-compatible subset.
//!
//! Grammar (slice):
//!   program  := stmt*
//!   stmt     := IDENT '=' expr ';'
//!             | 'for' '(' IDENT '=' expr ')' child
//!             | IDENT '(' args ')' child
//!   child    := ';' | '{' stmt* '}' | stmt          (bare single child)
//!   args     := (arg (',' arg)*)?
//!   arg      := IDENT '=' expr | expr
//!   expr     := term (('+'|'-') term)*
//!   term     := unary (('*'|'/'|'%') unary)*
//!   unary    := '-' unary | primary
//!   primary  := NUM | STR | 'true' | 'false' | 'undef' | IDENT
//!             | '(' expr ')' | '[' vector-or-range ']'

use crate::ast::{Arg, BinOp, Expr, Stmt};
use crate::lexer::{lex, Tok, Token};

pub fn parse(src: &str) -> Result<Vec<Stmt>, String> {
    let tokens = lex(src)?;
    let mut p = Parser { tokens, pos: 0 };
    let mut stmts = Vec::new();
    while !p.at_end() {
        stmts.push(p.stmt()?);
    }
    Ok(stmts)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos).map(|t| &t.tok)
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

    fn stmt(&mut self) -> Result<Stmt, String> {
        let name = match self.peek() {
            Some(Tok::Ident(s)) => s.clone(),
            _ => return Err(format!("expected a statement but found {}", self.here())),
        };
        self.pos += 1;

        if name == "for" {
            self.expect(&Tok::LParen, "after 'for'")?;
            let var = match self.bump() {
                Some(Tok::Ident(s)) => s,
                _ => return Err(format!("expected loop variable, found {}", self.here())),
            };
            self.expect(&Tok::Assign, "in for(...)")?;
            let iter = self.expr()?;
            self.expect(&Tok::RParen, "to close for(...)")?;
            let body = self.child()?;
            return Ok(Stmt::For { var, iter, body });
        }

        match self.peek() {
            Some(Tok::Assign) => {
                self.pos += 1;
                let value = self.expr()?;
                self.expect(&Tok::Semi, "after assignment")?;
                Ok(Stmt::Assign { name, value })
            }
            Some(Tok::LParen) => {
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

    /// A call's child position: ';' (leaf), a block, or one bare child.
    fn child(&mut self) -> Result<Vec<Stmt>, String> {
        match self.peek() {
            Some(Tok::Semi) => {
                self.pos += 1;
                Ok(Vec::new())
            }
            Some(Tok::LBrace) => {
                self.pos += 1;
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
            _ => Ok(vec![self.stmt()?]),
        }
    }

    fn args(&mut self) -> Result<Vec<Arg>, String> {
        let mut args = Vec::new();
        if self.peek() == Some(&Tok::RParen) {
            return Ok(args);
        }
        loop {
            // IDENT '=' expr is a named argument; anything else positional.
            let named = matches!(
                (self.peek(), self.tokens.get(self.pos + 1).map(|t| &t.tok)),
                (Some(Tok::Ident(_)), Some(Tok::Assign))
            );
            if named {
                let name = match self.bump() {
                    Some(Tok::Ident(s)) => s,
                    _ => unreachable!("guarded by the named check"),
                };
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

    fn expr(&mut self) -> Result<Expr, String> {
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
        if self.peek() == Some(&Tok::Minus) {
            self.pos += 1;
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.bump() {
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::Str(s)) => Ok(Expr::Str(s)),
            Some(Tok::Ident(s)) => Ok(match s.as_str() {
                "true" => Expr::Bool(true),
                "false" => Expr::Bool(false),
                "undef" => Expr::Undef,
                _ => Expr::Ident(s),
            }),
            Some(Tok::LParen) => {
                let e = self.expr()?;
                self.expect(&Tok::RParen, "to close '('")?;
                Ok(e)
            }
            Some(Tok::LBracket) => self.vector_or_range(),
            _ => {
                self.pos = self.pos.saturating_sub(1);
                Err(format!("expected an expression but found {}", self.here()))
            }
        }
    }

    /// After '[': either a range [a:b] / [a:s:b] or a vector [a, b, ...].
    fn vector_or_range(&mut self) -> Result<Expr, String> {
        if self.peek() == Some(&Tok::RBracket) {
            self.pos += 1;
            return Ok(Expr::Vector(Vec::new()));
        }
        let first = self.expr()?;
        if self.peek() == Some(&Tok::Colon) {
            self.pos += 1;
            let second = self.expr()?;
            let (step, end) = if self.peek() == Some(&Tok::Colon) {
                self.pos += 1;
                (Some(Box::new(second)), self.expr()?)
            } else {
                (None, second)
            };
            self.expect(&Tok::RBracket, "to close the range")?;
            return Ok(Expr::Range { start: Box::new(first), step, end: Box::new(end) });
        }
        let mut items = vec![first];
        while self.peek() == Some(&Tok::Comma) {
            self.pos += 1;
            items.push(self.expr()?);
        }
        self.expect(&Tok::RBracket, "to close the vector")?;
        Ok(Expr::Vector(items))
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
            Stmt::For { var, iter, body } => {
                assert_eq!(var, "i");
                assert!(matches!(iter, Expr::Range { step: Some(_), .. }));
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
    fn errors_name_the_offending_position() {
        let err = parse("cube(1) cube(2)").unwrap_err();
        assert!(err.contains("end of input"), "got: {}", err);
        let err = parse("translate() {").unwrap_err();
        assert!(err.contains("unterminated"), "got: {}", err);
    }
}
