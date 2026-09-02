//! AST for the SCAD-compatible subset.

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Ident(String),
    Vector(Vec<Expr>),
    /// [start : end] or [start : step : end] — step is None for the
    /// two-part form (meaning 1).
    Range { start: Box<Expr>, step: Option<Box<Expr>>, end: Box<Expr> },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Neg(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    /// None for positional arguments.
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// name = expr;
    Assign { name: String, value: Expr },
    /// module_name(args) <children>  — children is empty for the `;` form,
    /// one statement for the bare-child form, many for a `{ ... }` block.
    Call { name: String, args: Vec<Arg>, children: Vec<Stmt> },
    /// for (var = iterable) <child-or-block>
    For { var: String, iter: Expr, body: Vec<Stmt> },
}
