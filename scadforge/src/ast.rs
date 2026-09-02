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
    /// Unary + — accepted as a no-op on numbers per the reference.
    Pos(Box<Expr>),
    Not(Box<Expr>),
    /// cond ? then : else — both branches LAZY (only the taken one
    /// evaluates), the property recursive library code depends on.
    Ternary { cond: Box<Expr>, then: Box<Expr>, els: Box<Expr> },
    Index { base: Box<Expr>, index: Box<Expr> },
    Member { base: Box<Expr>, name: String },
    /// Function-call operator in expression position (builtins for now;
    /// functions and modules are separate namespaces).
    Call { name: String, args: Vec<Arg> },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
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
