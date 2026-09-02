//! AST for the SCAD-compatible subset.

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Ident(String),
    /// Vector literal. Items may be plain expressions or comprehension
    /// clauses (for/each/if/let), which contribute zero or more elements
    /// in place.
    Vector(Vec<VecItem>),
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
    /// name(args): resolved through the function namespace (user
    /// definitions shadow builtins), then variables holding function
    /// values, then builtins.
    Call { name: String, args: Vec<Arg> },
    /// Non-identifier callee: (expr)(x), v[i](x), f(x)(y).
    CallValue { callee: Box<Expr>, args: Vec<Arg> },
    /// A parenthesized bare identifier. Only the callee decision cares:
    /// (f)(x) resolves through the VALUE path (variables), never the
    /// function namespace, per the calling-conventions contract.
    Paren(Box<Expr>),
    /// let (a = 1, b = a + 1) body — sequential bindings, lexical scope.
    Let { bindings: Vec<(String, Expr)>, body: Box<Expr> },
    /// echo(args) body — prints on every evaluation, yields body.
    EchoExpr { args: Vec<Arg>, body: Box<Expr> },
    /// assert(cond, msg) body — halts evaluation on falsy cond.
    AssertExpr { args: Vec<Arg>, body: Box<Expr> },
    /// function (params) body — evaluates to a first-class function value
    /// capturing the lexical scope.
    FnLiteral { params: Vec<Param>, body: Box<Expr> },
}

/// One element position inside a vector literal: a plain expression or a
/// comprehension clause. Clause "rest" chains hold exactly one item; if
/// with no else holds an empty `els`.
#[derive(Debug, Clone, PartialEq)]
pub enum VecItem {
    One(Expr),
    /// each expr — splices a vector/range/string one level.
    Each(Expr),
    /// for (a = v, b = w) rest — cross product, a-major.
    CFor { bindings: Vec<(String, Expr)>, rest: Vec<VecItem> },
    /// for (inits; cond; updates) rest — C-style generator with
    /// SIMULTANEOUS updates.
    CForC {
        inits: Vec<(String, Expr)>,
        cond: Expr,
        updates: Vec<(String, Expr)>,
        rest: Vec<VecItem>,
    },
    CIf { cond: Expr, then: Vec<VecItem>, els: Vec<VecItem> },
    CLet { bindings: Vec<(String, Expr)>, rest: Vec<VecItem> },
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

/// A declared parameter of a module, named function, or function literal.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    /// Default expression, evaluated at call time in the callee's scope
    /// with earlier parameters visible.
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// name = expr;
    Assign { name: String, value: Expr },
    /// module_name(args) <children>  — children is empty for the `;` form,
    /// one statement for the bare-child form, many for a `{ ... }` block.
    Call { name: String, args: Vec<Arg>, children: Vec<Stmt> },
    /// for (a = v, b = w) body — multiple bindings are a cross product,
    /// a-major.
    For { bindings: Vec<(String, Expr)>, body: Vec<Stmt> },
    /// intersection_for (a = v, ...) body — same header as `for`, but the
    /// per-iteration results are folded with INTERSECTION instead of the
    /// implicit union.
    IntersectionFor { bindings: Vec<(String, Expr)>, body: Vec<Stmt> },
    /// if (cond) then else els — only the taken branch instantiates.
    If { cond: Expr, then: Vec<Stmt>, els: Vec<Stmt> },
    /// let (bindings) body — sequential bindings scoping the subtree.
    /// `deprecated_assign` marks the legacy assign() spelling, which
    /// behaves identically plus a DEPRECATED diagnostic.
    Let { bindings: Vec<(String, Expr)>, body: Vec<Stmt>, deprecated_assign: bool },
    /// module name(params) body
    ModuleDef { name: String, params: Vec<Param>, body: Vec<Stmt> },
    /// function name(params) = expr;
    FunctionDef { name: String, params: Vec<Param>, body: Expr },
    /// Bare { ... } grouping block — a scope.
    Block(Vec<Stmt>),
}
