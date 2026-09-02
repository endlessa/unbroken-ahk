//! Evaluator: AST → colored meshes + diagnostics.
//!
//! Phase 2 core: hoisted two-phase scopes (definitions hoist; assignment
//! slots evaluate first-slot/last-write-wins; statements then run against
//! final values), three namespaces (modules / functions / variables),
//! lazy children with the caller-lexical / callee-dynamic split, a
//! dynamic `$`-environment, tail-call elimination for function recursion,
//! list comprehensions, first-class function values, and the shared
//! echo/str value formatter. Union is still mesh concatenation — real
//! CSG booleans are phase 3.

use crate::ast::{Arg, BinOp, Expr, Param, Stmt, VecItem};
use crate::geom::{self, Mesh};
use crate::value::{FuncVal, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A mesh plus the display color assigned by the NEAREST enclosing
/// color() node (None = uncolored; ancestors must not override).
#[derive(Debug, Clone)]
pub struct Shape {
    pub mesh: Mesh,
    pub color: Option<[f64; 4]>,
}

#[derive(Debug, Default)]
pub struct EvalOutput {
    pub shapes: Vec<Shape>,
    /// Non-fatal diagnostics (unknown modules, bad arguments): evaluation
    /// continues so one typo doesn't blank the whole preview.
    pub warnings: Vec<String>,
    /// ECHO: lines, in depth-first instantiation order.
    pub echoes: Vec<String>,
    /// A fatal diagnostic (assert failure, recursion limit): evaluation
    /// halted here; shapes/echoes hold everything produced before it.
    pub error: Option<String>,
}

/// Evaluation runs on a dedicated thread with a large stack so deep
/// non-tail recursion hits our own depth guard, not the platform stack.
pub fn evaluate(program: &[Stmt]) -> EvalOutput {
    std::thread::scope(|s| {
        let handle = std::thread::Builder::new()
            .stack_size(256 * 1024 * 1024)
            .spawn_scoped(s, || evaluate_inner(program))
            .expect("failed to spawn the evaluator thread");
        handle.join().unwrap_or_else(|_| EvalOutput {
            error: Some("ERROR: the evaluator crashed (please report this script)".into()),
            ..EvalOutput::default()
        })
    })
}

fn evaluate_inner(program: &[Stmt]) -> EvalOutput {
    let mut ctx = Ctx {
        out: EvalOutput::default(),
        dynv: DynScope::root(),
        mod_stack: Vec::new(),
        children: None,
        fn_depth: 0,
    };
    let root = Scope::root();
    let shapes = exec_scope(program, &root, &mut ctx);
    ctx.out.shapes.extend(shapes);
    ctx.out
}

/// Non-tail function recursion guard (tail calls are eliminated and do
/// not count).
const MAX_FN_DEPTH: usize = 10_000;
/// Module instantiation recursion guard.
const MAX_MODULE_DEPTH: usize = 2_000;
/// Backstop for runaway tail loops / C-style generators, far above the
/// reference's practical contracts.
const MAX_TAIL_ITERS: usize = 50_000_000;
const MAX_GENERATOR_ITERS: usize = 1_000_000;
const MAX_RANGE_ITEMS: usize = 1_000_000;

// ---------------------------------------------------------------------------
// Environments

struct FuncDef {
    params: Vec<Param>,
    body: Expr,
}

struct ModDef {
    params: Vec<Param>,
    body: Vec<Stmt>,
}

/// One lexical scope: variables fill in during slot evaluation; function
/// and module definitions are hoisted at scope creation.
pub struct Scope {
    vars: RefCell<HashMap<String, Value>>,
    funcs: HashMap<String, Rc<FuncDef>>,
    modules: HashMap<String, Rc<ModDef>>,
    parent: Option<Rc<Scope>>,
}

impl Scope {
    fn root() -> Rc<Scope> {
        let mut vars = HashMap::new();
        // PI is a builtin constant (a variable, not a function).
        vars.insert("PI".to_string(), Value::Num(std::f64::consts::PI));
        Rc::new(Scope {
            vars: RefCell::new(vars),
            funcs: HashMap::new(),
            modules: HashMap::new(),
            parent: None,
        })
    }

    fn child(parent: &Rc<Scope>) -> Rc<Scope> {
        Rc::new(Scope {
            vars: RefCell::new(HashMap::new()),
            funcs: HashMap::new(),
            modules: HashMap::new(),
            parent: Some(parent.clone()),
        })
    }

    fn lookup(self: &Rc<Scope>, name: &str) -> Option<Value> {
        let mut cur = Some(self.clone());
        while let Some(s) = cur {
            if let Some(v) = s.vars.borrow().get(name) {
                return Some(v.clone());
            }
            cur = s.parent.clone();
        }
        None
    }

    fn lookup_module(self: &Rc<Scope>, name: &str) -> Option<(Rc<ModDef>, Rc<Scope>)> {
        let mut cur = Some(self.clone());
        while let Some(s) = cur {
            if let Some(m) = s.modules.get(name) {
                return Some((m.clone(), s.clone()));
            }
            cur = s.parent.clone();
        }
        None
    }
}

/// The dynamic `$`-environment: a chain snapshotted along the
/// instantiation/call path, separate from lexical scoping.
struct DynScope {
    vars: RefCell<HashMap<String, Value>>,
    parent: Option<Rc<DynScope>>,
}

impl DynScope {
    fn root() -> Rc<DynScope> {
        let mut vars = HashMap::new();
        // Reference defaults: $fn=0 (disabled), $fa=12°, $fs=2 units.
        vars.insert("$fn".to_string(), Value::Num(0.0));
        vars.insert("$fa".to_string(), Value::Num(12.0));
        vars.insert("$fs".to_string(), Value::Num(2.0));
        vars.insert("$t".to_string(), Value::Num(0.0));
        vars.insert("$preview".to_string(), Value::Bool(true));
        vars.insert("$children".to_string(), Value::Num(0.0));
        vars.insert("$parent_modules".to_string(), Value::Num(0.0));
        Rc::new(DynScope { vars: RefCell::new(vars), parent: None })
    }

    fn layer(parent: &Rc<DynScope>) -> Rc<DynScope> {
        Rc::new(DynScope { vars: RefCell::new(HashMap::new()), parent: Some(parent.clone()) })
    }

    fn lookup(self: &Rc<DynScope>, name: &str) -> Option<Value> {
        let mut cur = Some(self.clone());
        while let Some(s) = cur {
            if let Some(v) = s.vars.borrow().get(name) {
                return Some(v.clone());
            }
            cur = s.parent.clone();
        }
        None
    }
}

/// The children a module call was given: instantiated lazily by each
/// children() call, in the CALLER's lexical scope under the CALLEE's
/// current `$`-environment.
struct ChildrenCtx {
    stmts: Rc<Vec<Stmt>>,
    lex: Rc<Scope>,
}

struct Ctx {
    out: EvalOutput,
    dynv: Rc<DynScope>,
    /// User-module instantiation stack, innermost last (parent_module).
    mod_stack: Vec<String>,
    children: Option<Rc<ChildrenCtx>>,
    fn_depth: usize,
}

impl Ctx {
    fn halted(&self) -> bool {
        self.out.error.is_some()
    }

    fn halt(&mut self, msg: String) {
        if self.out.error.is_none() {
            self.out.error = Some(msg);
        }
    }
}

// ---------------------------------------------------------------------------
// Scope execution: the two-phase rule

/// Count of addressable children in a call-site block: statements other
/// than assignments and definitions.
fn geometry_stmts(stmts: &[Stmt]) -> Vec<&Stmt> {
    stmts
        .iter()
        .filter(|s| {
            !matches!(s, Stmt::Assign { .. } | Stmt::ModuleDef { .. } | Stmt::FunctionDef { .. })
        })
        .collect()
}

/// Execute a statement list as one lexical scope: hoist definitions,
/// evaluate assignment slots (first-slot position, last-write-wins), then
/// run the remaining statements against the final values.
fn exec_scope(stmts: &[Stmt], parent: &Rc<Scope>, ctx: &mut Ctx) -> Vec<Shape> {
    let scope = build_scope(stmts, parent, ctx);
    let saved_dyn = ctx.dynv.clone();
    ctx.dynv = DynScope::layer(&saved_dyn);
    eval_slots(stmts, &scope, ctx);
    let mut shapes = Vec::new();
    for stmt in stmts {
        if ctx.halted() {
            break;
        }
        shapes.extend(exec_stmt(stmt, &scope, ctx));
    }
    ctx.dynv = saved_dyn;
    shapes
}

/// Hoist module/function definitions (duplicates warn, last wins) into a
/// fresh child scope.
fn build_scope(stmts: &[Stmt], parent: &Rc<Scope>, ctx: &mut Ctx) -> Rc<Scope> {
    let mut funcs: HashMap<String, Rc<FuncDef>> = HashMap::new();
    let mut modules: HashMap<String, Rc<ModDef>> = HashMap::new();
    for stmt in stmts {
        match stmt {
            Stmt::ModuleDef { name, params, body } => {
                if modules
                    .insert(
                        name.clone(),
                        Rc::new(ModDef { params: params.clone(), body: body.clone() }),
                    )
                    .is_some()
                {
                    ctx.out
                        .warnings
                        .push(format!("module {}() was redefined; the last definition wins", name));
                }
            }
            Stmt::FunctionDef { name, params, body } => {
                if funcs
                    .insert(
                        name.clone(),
                        Rc::new(FuncDef { params: params.clone(), body: body.clone() }),
                    )
                    .is_some()
                {
                    ctx.out
                        .warnings
                        .push(format!("function {}() was redefined; the last definition wins", name));
                }
            }
            _ => {}
        }
    }
    Rc::new(Scope {
        vars: RefCell::new(HashMap::new()),
        funcs,
        modules,
        parent: Some(parent.clone()),
    })
}

/// Assignment slots: the first textual assignment to a name owns the
/// slot; later assignments replace its expression (with a warning) rather
/// than adding a new binding. Slots evaluate in slot order before any
/// other statement runs.
fn eval_slots(stmts: &[Stmt], scope: &Rc<Scope>, ctx: &mut Ctx) {
    let mut order: Vec<&str> = Vec::new();
    let mut winning: HashMap<&str, &Expr> = HashMap::new();
    for stmt in stmts {
        if let Stmt::Assign { name, value } = stmt {
            if winning.insert(name.as_str(), value).is_some() {
                ctx.out.warnings.push(format!("variable '{}' was reassigned", name));
            } else {
                order.push(name.as_str());
            }
        }
    }
    for name in order {
        if ctx.halted() {
            return;
        }
        let v = eval_expr(winning[name], scope, ctx);
        set_var(scope, ctx, name, v);
    }
}

/// Bind a value in a scope; `$`-names also enter the current dynamic
/// layer so descendant calls see them.
fn set_var(scope: &Rc<Scope>, ctx: &mut Ctx, name: &str, v: Value) {
    if name.starts_with('$') {
        ctx.dynv.vars.borrow_mut().insert(name.to_string(), v.clone());
    }
    scope.vars.borrow_mut().insert(name.to_string(), v);
}

fn exec_stmt(stmt: &Stmt, scope: &Rc<Scope>, ctx: &mut Ctx) -> Vec<Shape> {
    match stmt {
        Stmt::Assign { .. } | Stmt::ModuleDef { .. } | Stmt::FunctionDef { .. } => Vec::new(),
        Stmt::Block(body) => exec_scope(body, scope, ctx),
        Stmt::If { cond, then, els } => {
            let c = eval_expr(cond, scope, ctx);
            if truthy(&c) {
                exec_scope(then, scope, ctx)
            } else {
                exec_scope(els, scope, ctx)
            }
        }
        Stmt::Let { bindings, body, deprecated_assign } => {
            if *deprecated_assign {
                ctx.out.warnings.push(
                    "DEPRECATED: Assign is deprecated. Use a regular assignment instead.".into(),
                );
            }
            let saved_dyn = ctx.dynv.clone();
            ctx.dynv = DynScope::layer(&saved_dyn);
            let bind_scope = sequential_bindings(bindings, scope, ctx);
            let shapes = exec_scope(body, &bind_scope, ctx);
            ctx.dynv = saved_dyn;
            shapes
        }
        Stmt::For { bindings, body } => {
            let iterables: Vec<(String, Vec<Value>)> = bindings
                .iter()
                .map(|(name, expr)| {
                    let v = eval_expr(expr, scope, ctx);
                    (name.clone(), iterate(&v, ctx))
                })
                .collect();
            let mut shapes = Vec::new();
            cross_product(&iterables, 0, &mut Vec::new(), &mut |vals, ctx| {
                let saved_dyn = ctx.dynv.clone();
                ctx.dynv = DynScope::layer(&saved_dyn);
                let iter_scope = Scope::child(scope);
                for (name, v) in vals {
                    set_var(&iter_scope, ctx, name, v.clone());
                }
                shapes.extend(exec_scope(body, &iter_scope, ctx));
                ctx.dynv = saved_dyn;
            }, ctx);
            shapes
        }
        Stmt::Call { name, args, children } => exec_call(name, args, children, scope, ctx),
    }
}

/// Sequential bindings for let/assign/comprehension-let: each binding
/// sees the earlier ones; no last-write-wins reordering.
fn sequential_bindings(
    bindings: &[(String, Expr)],
    parent: &Rc<Scope>,
    ctx: &mut Ctx,
) -> Rc<Scope> {
    let scope = Scope::child(parent);
    for (name, expr) in bindings {
        if ctx.halted() {
            break;
        }
        let v = eval_expr(expr, &scope, ctx);
        set_var(&scope, ctx, name, v);
    }
    scope
}

/// a-major cross product over the evaluated iterables of a multi-binding
/// for.
fn cross_product<'a>(
    iterables: &'a [(String, Vec<Value>)],
    idx: usize,
    acc: &mut Vec<(&'a str, Value)>,
    f: &mut dyn FnMut(&[(&'a str, Value)], &mut Ctx),
    ctx: &mut Ctx,
) {
    if ctx.halted() {
        return;
    }
    if idx == iterables.len() {
        f(acc, ctx);
        return;
    }
    let (name, items) = &iterables[idx];
    for item in items {
        acc.push((name.as_str(), item.clone()));
        cross_product(iterables, idx + 1, acc, f, ctx);
        acc.pop();
        if ctx.halted() {
            return;
        }
    }
}

/// What one loop iteration binds: vectors element-wise, ranges
/// arithmetically (with the legacy reversed-range swap), strings per
/// code point, and every scalar (numbers, booleans, undef, functions)
/// exactly once.
fn iterate(v: &Value, ctx: &mut Ctx) -> Vec<Value> {
    match v {
        Value::Vector(items) => items.clone(),
        Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
        Value::Range { start, step, end, implicit_step } => {
            let mut items = Vec::new();
            if *step == 0.0 || !step.is_finite() {
                ctx.out
                    .warnings
                    .push("for: range step must be a nonzero finite number".into());
                return items;
            }
            // Legacy two-part reversed range [10:1]: DEPRECATED, bounds
            // swap and iteration ascends. The explicit-step form does
            // NOT swap — [10:1:0] simply yields zero iterations.
            let (start, end) = if *implicit_step && start > end {
                ctx.out.warnings.push(
                    "DEPRECATED: using ranges of the form [begin:end] with begin \
                     greater than end; bounds swapped"
                        .into(),
                );
                (end, start)
            } else {
                (start, end)
            };
            let mut x = *start;
            // Inclusive of end, in either direction, with a sane cap.
            while (*step > 0.0 && x <= *end + 1e-12) || (*step < 0.0 && x >= *end - 1e-12) {
                items.push(Value::Num(x));
                if items.len() >= MAX_RANGE_ITEMS {
                    ctx.out.warnings.push(format!(
                        "for: range truncated at {} iterations",
                        MAX_RANGE_ITEMS
                    ));
                    break;
                }
                x += *step;
            }
            items
        }
        other => vec![other.clone()],
    }
}

// ---------------------------------------------------------------------------
// Module instantiation

/// One evaluated argument.
struct EvArg {
    name: Option<String>,
    value: Value,
}

fn eval_args(args: &[Arg], scope: &Rc<Scope>, ctx: &mut Ctx) -> Vec<EvArg> {
    args.iter()
        .map(|a| EvArg { name: a.name.clone(), value: eval_expr(&a.value, scope, ctx) })
        .collect()
}

fn exec_call(name: &str, args: &[Arg], children: &[Stmt], scope: &Rc<Scope>, ctx: &mut Ctx) -> Vec<Shape> {
    // Arguments evaluate eagerly, once, in the caller's scope — even for
    // arguments that end up discarded.
    let ev = eval_args(args, scope, ctx);
    if ctx.halted() {
        return Vec::new();
    }

    // $-named arguments bind dynamically for the callee's whole subtree,
    // on every kind of call, without declaration.
    let saved_dyn = ctx.dynv.clone();
    ctx.dynv = DynScope::layer(&saved_dyn);
    for a in &ev {
        if let Some(n) = &a.name {
            if n.starts_with('$') {
                ctx.dynv.vars.borrow_mut().insert(n.clone(), a.value.clone());
            }
        }
    }

    let shapes = if let Some((def, def_scope)) = scope.lookup_module(name) {
        call_user_module(name, &def, &def_scope, &ev, children, scope, ctx)
    } else {
        call_builtin_module(name, args, &ev, children, scope, ctx)
    };
    ctx.dynv = saved_dyn;
    shapes
}

fn call_user_module(
    name: &str,
    def: &Rc<ModDef>,
    def_scope: &Rc<Scope>,
    ev: &[EvArg],
    children: &[Stmt],
    caller: &Rc<Scope>,
    ctx: &mut Ctx,
) -> Vec<Shape> {
    if ctx.mod_stack.len() >= MAX_MODULE_DEPTH {
        ctx.halt(format!("ERROR: Recursion detected calling module '{}'", name));
        return Vec::new();
    }
    let param_scope = bind_params(name, &def.params, ev, def_scope, ctx);

    let saved_children = ctx.children.take();
    ctx.children = Some(Rc::new(ChildrenCtx {
        stmts: Rc::new(children.to_vec()),
        lex: caller.clone(),
    }));
    ctx.mod_stack.push(name.to_string());
    {
        let mut dv = ctx.dynv.vars.borrow_mut();
        dv.insert("$children".into(), Value::Num(geometry_stmts(children).len() as f64));
        dv.insert("$parent_modules".into(), Value::Num(ctx.mod_stack.len() as f64));
    }

    let shapes = exec_scope(&def.body, &param_scope, ctx);

    ctx.mod_stack.pop();
    ctx.children = saved_children;
    shapes
}

/// The shared argument matcher for user modules, named functions, and
/// function literals: positionals fill declaration order (skipping slots
/// already taken by name), unknown plain named arguments warn and are
/// discarded, `$`-named ones were already bound dynamically, defaults
/// evaluate in the callee's scope with earlier parameters visible, and
/// anything left is undef.
fn bind_params(
    callee: &str,
    params: &[Param],
    ev: &[EvArg],
    def_scope: &Rc<Scope>,
    ctx: &mut Ctx,
) -> Rc<Scope> {
    let scope = Scope::child(def_scope);
    let mut bound: HashMap<&str, Value> = HashMap::new();
    let mut too_many = false;
    let mut next_pos = 0usize;
    for a in ev {
        match &a.name {
            Some(n) if n.starts_with('$') => {} // already in the dynamic layer
            Some(n) => {
                if params.iter().any(|p| &p.name == n) {
                    if bound.insert(params.iter().find(|p| &p.name == n).unwrap().name.as_str(), a.value.clone()).is_some() {
                        ctx.out.warnings.push(format!(
                            "{}: parameter '{}' was bound more than once; the last binding wins",
                            callee, n
                        ));
                    }
                } else {
                    ctx.out.warnings.push(format!(
                        "{}: unknown parameter '{}' ignored",
                        callee, n
                    ));
                }
            }
            None => {
                // Fill the next slot not already taken by a named binding.
                while next_pos < params.len() && bound.contains_key(params[next_pos].name.as_str())
                {
                    next_pos += 1;
                }
                if next_pos < params.len() {
                    bound.insert(params[next_pos].name.as_str(), a.value.clone());
                    next_pos += 1;
                } else if !too_many {
                    too_many = true;
                    ctx.out
                        .warnings
                        .push(format!("{}: too many unnamed arguments", callee));
                }
            }
        }
    }
    for p in params {
        let v = match bound.remove(p.name.as_str()) {
            Some(v) => v,
            None => {
                // A declared $-formal inherits a dynamic value before
                // falling back to its default.
                let inherited = if p.name.starts_with('$') {
                    ctx.dynv.lookup(&p.name)
                } else {
                    None
                };
                match (inherited, &p.default) {
                    (Some(v), _) => v,
                    (None, Some(d)) => eval_expr(d, &scope, ctx),
                    (None, None) => Value::Undef,
                }
            }
        };
        set_var(&scope, ctx, &p.name, v);
    }
    scope
}

/// Instantiate selected children at a children()/child() call site:
/// lazily, afresh per call, in the caller's lexical scope under the
/// callee's current `$`-environment.
fn instantiate_children(selection: Option<&Value>, ctx: &mut Ctx) -> Vec<Shape> {
    let cctx = match ctx.children.clone() {
        Some(c) => c,
        None => {
            ctx.out
                .warnings
                .push("children() called outside a module body".into());
            return Vec::new();
        }
    };
    // The call-site block is a scope: hoist + slots run fresh on every
    // children() call, then the selected geometry statements instantiate.
    let scope = build_scope(&cctx.stmts, &cctx.lex, ctx);
    eval_slots(&cctx.stmts, &scope, ctx);
    let geo = geometry_stmts(&cctx.stmts);
    let indices: Vec<i64> = match selection {
        None => (0..geo.len() as i64).collect(),
        Some(Value::Num(n)) => vec![to_index(*n)],
        Some(Value::Vector(items)) => items
            .iter()
            .map(|v| v.as_num().map(to_index).unwrap_or(-1))
            .collect(),
        Some(r @ Value::Range { .. }) => {
            let items = iterate(r, ctx);
            items.iter().map(|v| v.as_num().map(to_index).unwrap_or(-1)).collect()
        }
        Some(other) => {
            ctx.out.warnings.push(format!(
                "children: index must be a number, vector, or range, got {}",
                other.type_name()
            ));
            return Vec::new();
        }
    };
    let mut shapes = Vec::new();
    for i in indices {
        if ctx.halted() {
            break;
        }
        if i < 0 || i as usize >= geo.len() {
            ctx.out.warnings.push(format!(
                "children index ({}) out of bounds ({} children)",
                i,
                geo.len()
            ));
            continue;
        }
        shapes.extend(exec_stmt(geo[i as usize], &scope, ctx));
    }
    shapes
}

fn to_index(n: f64) -> i64 {
    if n.is_finite() {
        n.trunc() as i64
    } else {
        -1
    }
}

fn call_builtin_module(
    name: &str,
    args: &[Arg],
    ev: &[EvArg],
    children: &[Stmt],
    scope: &Rc<Scope>,
    ctx: &mut Ctx,
) -> Vec<Shape> {
    let bound = bind_builtin_args(name, ev, ctx);

    match name {
        "cube" => {
            let size = match bound.get("size") {
                Some(Value::Num(s)) => [*s, *s, *s],
                Some(v @ Value::Vector(_)) => match v.as_vec3() {
                    Some(s) => s,
                    None => {
                        ctx.out
                            .warnings
                            .push("cube: size must be a number or [x, y, z]".into());
                        return Vec::new();
                    }
                },
                Some(Value::Undef) | None => [1.0, 1.0, 1.0],
                Some(other) => {
                    ctx.out.warnings.push(format!(
                        "cube: size must be a number or vector, got {}",
                        other.type_name()
                    ));
                    return Vec::new();
                }
            };
            let center = bound.get("center").and_then(Value::as_bool).unwrap_or(false);
            no_children(name, children, ctx);
            leaf(geom::cube(size, center))
        }
        "sphere" => {
            // d overrides r per the reference.
            let r = match (bound.get("d"), bound.get("r")) {
                (Some(Value::Num(d)), _) => d / 2.0,
                (_, Some(Value::Num(r))) => *r,
                _ => 1.0,
            };
            let n = resolve_fragments(r, ctx);
            no_children(name, children, ctx);
            leaf(geom::sphere(r, n))
        }
        "cylinder" => {
            let num = |key: &str| bound.get(key).and_then(Value::as_num);
            let r_both = num("d").map(|d| d / 2.0).or_else(|| num("r"));
            let r1 = num("d1").map(|d| d / 2.0).or_else(|| num("r1")).or(r_both).unwrap_or(1.0);
            let r2 = num("d2").map(|d| d / 2.0).or_else(|| num("r2")).or(r_both).unwrap_or(1.0);
            let h = num("h").unwrap_or(1.0);
            let center = bound.get("center").and_then(Value::as_bool).unwrap_or(false);
            let n = resolve_fragments(r1.max(r2), ctx);
            no_children(name, children, ctx);
            leaf(geom::cylinder(h, r1, r2, center, n))
        }
        "translate" => {
            let matrix = match bound.get("v").and_then(Value::as_vec3) {
                Some(v) => Some(geom::translation(v)),
                None => {
                    ctx.out
                        .warnings
                        .push("translate: v must be a vector like [x, y, z]".into());
                    None
                }
            };
            transform_children(children, scope, ctx, matrix)
        }
        "scale" => {
            let matrix = match bound.get("v") {
                Some(Value::Num(s)) => Some(geom::scaling([*s, *s, *s])),
                Some(v @ Value::Vector(_)) => v.as_vec3().map(geom::scaling),
                _ => {
                    ctx.out.warnings.push("scale: v must be a number or vector".into());
                    None
                }
            };
            transform_children(children, scope, ctx, matrix)
        }
        "rotate" => {
            let matrix = match (bound.get("a"), bound.get("v")) {
                (Some(Value::Num(deg)), Some(axis @ Value::Vector(_))) => match axis.as_vec3() {
                    Some(axis) => Some(geom::rotation_axis(*deg, axis)),
                    None => {
                        ctx.out.warnings.push("rotate: v must be a numeric vector".into());
                        None
                    }
                },
                (Some(Value::Num(deg)), _) => Some(geom::rotation_xyz([0.0, 0.0, *deg])),
                (Some(vec @ Value::Vector(_)), _) => match vec.as_vec3() {
                    Some(deg) => Some(geom::rotation_xyz(deg)),
                    None => {
                        ctx.out
                            .warnings
                            .push("rotate: a must be a scalar or [x, y, z] degrees".into());
                        None
                    }
                },
                _ => {
                    ctx.out.warnings.push("rotate: missing angle".into());
                    None
                }
            };
            transform_children(children, scope, ctx, matrix)
        }
        "color" => {
            let rgba = parse_color(bound.get("c"), bound.get("alpha"), ctx);
            let mut shapes = exec_scope(children, scope, ctx);
            if let Some(rgba) = rgba {
                for s in &mut shapes {
                    // NEAREST color wins: a color set deeper in the tree
                    // (already Some) is never overridden by an ancestor.
                    if s.color.is_none() {
                        s.color = Some(rgba);
                    }
                }
            }
            shapes
        }
        "union" | "group" => exec_scope(children, scope, ctx),
        "difference" | "intersection" | "hull" | "minkowski" | "intersection_for" => {
            ctx.out.warnings.push(format!(
                "{}() is not implemented in this slice yet (CSG booleans are \
                 the next kernel phase) — children are shown un-combined",
                name
            ));
            exec_scope(children, scope, ctx)
        }
        "children" => instantiate_children(bound.get("index"), ctx),
        "child" => {
            ctx.out.warnings.push(
                "DEPRECATED: child() will be removed in future releases. Use children() instead."
                    .into(),
            );
            let idx = bound.get("index").cloned().unwrap_or(Value::Num(0.0));
            instantiate_children(Some(&idx), ctx)
        }
        "echo" => {
            // Children are accepted and ignored; $-named args bind (they
            // are already in the dynamic layer) but do not print.
            let line = ev
                .iter()
                .filter(|a| a.name.as_deref().map_or(true, |n| !n.starts_with('$')))
                .map(|a| match &a.name {
                    Some(n) => format!("{} = {}", n, fmt_value(&a.value, true)),
                    None => fmt_value(&a.value, true),
                })
                .collect::<Vec<_>>()
                .join(", ");
            ctx.out.echoes.push(format!("ECHO: {}", line));
            Vec::new()
        }
        "assert" => {
            let (cond, msg) = assert_args(ev);
            if truthy(&cond) {
                exec_scope(children, scope, ctx)
            } else {
                let text = args
                    .first()
                    .filter(|a| a.name.is_none())
                    .map(|a| serialize_expr(&a.value));
                ctx.halt(assert_failure_message(text.as_deref(), msg.as_ref()));
                Vec::new()
            }
        }
        other => {
            // Per the reference, the statement — children included — is
            // skipped entirely.
            ctx.out.warnings.push(format!("unknown module '{}' ignored", other));
            Vec::new()
        }
    }
}

/// A leaf primitive given children warns and ignores them.
fn no_children(name: &str, children: &[Stmt], ctx: &mut Ctx) {
    if !geometry_stmts(children).is_empty() {
        ctx.out
            .warnings
            .push(format!("module {}() does not support child modules", name));
    }
}

fn assert_args(ev: &[EvArg]) -> (Value, Option<Value>) {
    let mut cond = Value::Undef;
    let mut msg = None;
    let mut pos = 0;
    for a in ev {
        match a.name.as_deref() {
            Some("condition") => cond = a.value.clone(),
            Some("message") => msg = Some(a.value.clone()),
            Some(_) => {}
            None => {
                match pos {
                    0 => cond = a.value.clone(),
                    1 => msg = Some(a.value.clone()),
                    _ => {}
                }
                pos += 1;
            }
        }
    }
    (cond, msg)
}

fn assert_failure_message(cond_text: Option<&str>, msg: Option<&Value>) -> String {
    let cond = cond_text.unwrap_or("false");
    match msg {
        Some(m) => format!("ERROR: Assertion '({})' failed: {}", cond, fmt_value(m, true)),
        None => format!("ERROR: Assertion '({})' failed", cond),
    }
}

fn leaf(mesh: Mesh) -> Vec<Shape> {
    if mesh.positions.is_empty() {
        Vec::new()
    } else {
        vec![Shape { mesh, color: None }]
    }
}

fn transform_children(
    children: &[Stmt],
    scope: &Rc<Scope>,
    ctx: &mut Ctx,
    matrix: Option<geom::Mat4>,
) -> Vec<Shape> {
    let mut shapes = exec_scope(children, scope, ctx);
    if let Some(m) = matrix {
        for s in &mut shapes {
            geom::apply(&m, &mut s.mesh);
        }
        shapes
    } else {
        // Bad transform arguments: drop the subtree rather than render it
        // untransformed in the wrong place. The warning already fired.
        Vec::new()
    }
}

/// Positional-parameter names per builtin module, per the reference
/// signatures.
fn positional_names(module: &str) -> &'static [&'static str] {
    match module {
        "cube" => &["size", "center"],
        "sphere" => &["r"],
        "cylinder" => &["h", "r1", "r2", "center"],
        "translate" | "scale" => &["v"],
        "rotate" => &["a", "v"],
        "color" => &["c", "alpha"],
        "children" | "child" => &["index"],
        _ => &[],
    }
}

fn bind_builtin_args(module: &str, ev: &[EvArg], ctx: &mut Ctx) -> HashMap<String, Value> {
    let names = positional_names(module);
    let mut bound = HashMap::new();
    let mut pos = 0;
    for a in ev {
        match &a.name {
            Some(n) if n.starts_with('$') => {} // dynamic, already bound
            Some(n) => {
                bound.insert(n.clone(), a.value.clone());
            }
            None => {
                match names.get(pos) {
                    Some(n) => {
                        bound.insert((*n).to_string(), a.value.clone());
                    }
                    None => {
                        if !matches!(module, "echo" | "assert") {
                            ctx.out.warnings.push(format!(
                                "{}: too many positional arguments (expected at most {})",
                                module,
                                names.len()
                            ));
                        }
                    }
                }
                pos += 1;
            }
        }
    }
    bound
}

/// $fn/$fa/$fs resolve through the dynamic environment (per-call $-args
/// were already layered on top by exec_call).
fn resolve_fragments(r: f64, ctx: &mut Ctx) -> u32 {
    let get = |key: &str, default: f64| {
        ctx.dynv.lookup(key).and_then(|v| v.as_num()).unwrap_or(default)
    };
    geom::fragments(r, get("$fn", 0.0), get("$fa", 12.0), get("$fs", 2.0))
}

fn parse_color(c: Option<&Value>, alpha: Option<&Value>, ctx: &mut Ctx) -> Option<[f64; 4]> {
    let mut rgba = match c {
        Some(Value::Vector(items)) if items.len() == 3 || items.len() == 4 => {
            let mut v = [0.0, 0.0, 0.0, 1.0];
            for (i, item) in items.iter().enumerate() {
                v[i] = item.as_num()?;
            }
            v
        }
        Some(Value::Str(name)) => named_color(name).or_else(|| hex_color(name)).or_else(|| {
            ctx.out.warnings.push(format!("color: unknown color '{}'", name));
            None
        })?,
        _ => {
            ctx.out
                .warnings
                .push("color: expected a name, \"#hex\", or [r, g, b(, a)]".into());
            return None;
        }
    };
    if let Some(Value::Num(a)) = alpha {
        // The alpha parameter overrides any alpha already in c.
        rgba[3] = *a;
    }
    Some(rgba)
}

fn named_color(name: &str) -> Option<[f64; 4]> {
    // A small starter set of the CSS keywords; the full ~147-name table
    // comes with the color milestone.
    let rgb: [f64; 3] = match name.to_ascii_lowercase().as_str() {
        "red" => [1.0, 0.0, 0.0],
        "green" => [0.0, 0.5, 0.0],
        "lime" => [0.0, 1.0, 0.0],
        "blue" => [0.0, 0.0, 1.0],
        "yellow" => [1.0, 1.0, 0.0],
        "orange" => [1.0, 0.647, 0.0],
        "purple" => [0.5, 0.0, 0.5],
        "cyan" | "aqua" => [0.0, 1.0, 1.0],
        "magenta" | "fuchsia" => [1.0, 0.0, 1.0],
        "white" => [1.0, 1.0, 1.0],
        "black" => [0.0, 0.0, 0.0],
        "gray" | "grey" => [0.5, 0.5, 0.5],
        "silver" => [0.753, 0.753, 0.753],
        "gold" => [1.0, 0.843, 0.0],
        "goldenrod" => [0.855, 0.647, 0.125],
        "steelblue" => [0.275, 0.51, 0.706],
        "tomato" => [1.0, 0.388, 0.278],
        _ => return None,
    };
    Some([rgb[0], rgb[1], rgb[2], 1.0])
}

fn hex_color(s: &str) -> Option<[f64; 4]> {
    let hex = s.strip_prefix('#')?;
    let nib = |c: u8| (c as char).to_digit(16).map(|d| d as f64);
    let bytes = hex.as_bytes();
    let (r, g, b, a) = match bytes.len() {
        3 | 4 => {
            let mut v = [0.0; 4];
            for (i, &c) in bytes.iter().enumerate() {
                let d = nib(c)?;
                v[i] = (d * 16.0 + d) / 255.0; // nibble n expands to nn
            }
            (v[0], v[1], v[2], if bytes.len() == 4 { v[3] } else { 1.0 })
        }
        6 | 8 => {
            let mut v = [0.0; 4];
            for i in 0..bytes.len() / 2 {
                v[i] = (nib(bytes[2 * i])? * 16.0 + nib(bytes[2 * i + 1])?) / 255.0;
            }
            (v[0], v[1], v[2], if bytes.len() == 8 { v[3] } else { 1.0 })
        }
        _ => return None,
    };
    Some([r, g, b, a])
}

// ---------------------------------------------------------------------------
// Expression evaluation

fn eval_expr(expr: &Expr, scope: &Rc<Scope>, ctx: &mut Ctx) -> Value {
    if ctx.halted() {
        return Value::Undef;
    }
    match expr {
        Expr::Num(n) => Value::Num(*n),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Str(s) => Value::Str(s.clone()),
        Expr::Undef => Value::Undef,
        Expr::Ident(name) => {
            if name.starts_with('$') {
                // Unset $-variables read undef silently (libraries probe
                // optional flags like $slop expecting no warning).
                return ctx.dynv.lookup(name).unwrap_or(Value::Undef);
            }
            match scope.lookup(name) {
                Some(v) => v,
                None => {
                    ctx.out
                        .warnings
                        .push(format!("unknown variable '{}' (undef)", name));
                    Value::Undef
                }
            }
        }
        Expr::Vector(items) => {
            let mut out = Vec::new();
            eval_vec_items(items, scope, ctx, &mut out);
            Value::Vector(out)
        }
        Expr::Range { start, step, end } => {
            let implicit_step = step.is_none();
            let s = eval_expr(start, scope, ctx).as_num();
            let st = match step {
                Some(e) => eval_expr(e, scope, ctx).as_num(),
                None => Some(1.0),
            };
            let e = eval_expr(end, scope, ctx).as_num();
            match (s, st, e) {
                (Some(s), Some(st), Some(e)) => {
                    Value::Range { start: s, step: st, end: e, implicit_step }
                }
                _ => {
                    ctx.out.warnings.push("range bounds must be numbers".into());
                    Value::Undef
                }
            }
        }
        Expr::Neg(inner) => {
            let v = eval_expr(inner, scope, ctx);
            negate(v, ctx)
        }
        Expr::Pos(inner) => match eval_expr(inner, scope, ctx) {
            Value::Num(n) => Value::Num(n),
            other => {
                ctx.out
                    .warnings
                    .push(format!("undefined operation (+ {})", other.type_name()));
                Value::Undef
            }
        },
        Expr::Not(inner) => {
            // Never undef: strictly boolean, even for undef/nan operands.
            let v = eval_expr(inner, scope, ctx);
            Value::Bool(!truthy(&v))
        }
        Expr::Ternary { cond, then, els } => {
            // Only the taken branch evaluates — the laziness recursive
            // library code depends on (undef and nan conds included:
            // undef is falsy, nan is TRUTHY).
            let c = eval_expr(cond, scope, ctx);
            if truthy(&c) {
                eval_expr(then, scope, ctx)
            } else {
                eval_expr(els, scope, ctx)
            }
        }
        Expr::Index { base, index } => {
            let b = eval_expr(base, scope, ctx);
            let i = eval_expr(index, scope, ctx);
            index_value(&b, &i)
        }
        Expr::Member { base, name } => {
            let b = eval_expr(base, scope, ctx);
            member_value(&b, name)
        }
        Expr::Let { bindings, body } => {
            let saved_dyn = ctx.dynv.clone();
            ctx.dynv = DynScope::layer(&saved_dyn);
            let bind_scope = sequential_bindings(bindings, scope, ctx);
            let v = eval_expr(body, &bind_scope, ctx);
            ctx.dynv = saved_dyn;
            v
        }
        Expr::EchoExpr { args, body } => {
            emit_echo(args, scope, ctx);
            eval_expr(body, scope, ctx)
        }
        Expr::AssertExpr { args, body } => {
            let ev = eval_args(args, scope, ctx);
            let (cond, msg) = assert_args(&ev);
            if !truthy(&cond) {
                let text = args
                    .first()
                    .filter(|a| a.name.is_none())
                    .map(|a| serialize_expr(&a.value));
                ctx.halt(assert_failure_message(text.as_deref(), msg.as_ref()));
                return Value::Undef;
            }
            eval_expr(body, scope, ctx)
        }
        Expr::FnLiteral { params, body } => Value::Function(Rc::new(FuncVal {
            params: params.clone(),
            body: (**body).clone(),
            env: scope.clone() as Rc<dyn std::any::Any>,
        })),
        Expr::Call { name, args } => {
            // is_undef(name) is the defined-guard idiom: it probes without
            // the unknown-variable warning.
            if name == "is_undef" && args.len() == 1 && args[0].name.is_none() {
                if let Expr::Ident(var) = &args[0].value {
                    let defined = if var.starts_with('$') {
                        ctx.dynv.lookup(var).is_some()
                    } else {
                        scope.lookup(var).is_some()
                    };
                    let is_undef = match (defined, if var.starts_with('$') { ctx.dynv.lookup(var) } else { scope.lookup(var) }) {
                        (true, Some(v)) => matches!(v, Value::Undef),
                        _ => true,
                    };
                    return Value::Bool(is_undef);
                }
            }
            match resolve_function(name, scope) {
                Some((params, body, env)) => {
                    let ev = eval_args(args, scope, ctx);
                    call_function(name, params, body, env, ev, ctx)
                }
                None => {
                    let ev = eval_args(args, scope, ctx);
                    call_builtin(name, &ev, scope, ctx)
                }
            }
        }
        Expr::CallValue { callee, args } => {
            let f = eval_expr(callee, scope, ctx);
            match f {
                Value::Function(fv) => {
                    let ev = eval_args(args, scope, ctx);
                    let env = fn_env(&fv);
                    call_function("<function>", fv.params.clone(), fv.body.clone(), env, ev, ctx)
                }
                other => {
                    ctx.out
                        .warnings
                        .push(format!("can't call a {} as a function", other.type_name()));
                    Value::Undef
                }
            }
        }
        Expr::Binary { op, lhs, rhs } => match op {
            // && and || short-circuit and return strict booleans —
            // never the operand value.
            BinOp::And => {
                let l = eval_expr(lhs, scope, ctx);
                if !truthy(&l) {
                    Value::Bool(false)
                } else {
                    let r = eval_expr(rhs, scope, ctx);
                    Value::Bool(truthy(&r))
                }
            }
            BinOp::Or => {
                let l = eval_expr(lhs, scope, ctx);
                if truthy(&l) {
                    Value::Bool(true)
                } else {
                    let r = eval_expr(rhs, scope, ctx);
                    Value::Bool(truthy(&r))
                }
            }
            _ => {
                let l = eval_expr(lhs, scope, ctx);
                let r = eval_expr(rhs, scope, ctx);
                binary_op(*op, l, r, ctx)
            }
        },
    }
}

fn emit_echo(args: &[Arg], scope: &Rc<Scope>, ctx: &mut Ctx) {
    let ev = eval_args(args, scope, ctx);
    let line = ev
        .iter()
        .filter(|a| a.name.as_deref().map_or(true, |n| !n.starts_with('$')))
        .map(|a| match &a.name {
            Some(n) => format!("{} = {}", n, fmt_value(&a.value, true)),
            None => fmt_value(&a.value, true),
        })
        .collect::<Vec<_>>()
        .join(", ");
    ctx.out.echoes.push(format!("ECHO: {}", line));
}

/// Recover the captured lexical scope from a function value.
fn fn_env(fv: &Rc<FuncVal>) -> Rc<Scope> {
    fv.env
        .clone()
        .downcast::<Scope>()
        .expect("function values always capture an evaluator scope")
}

/// Resolve an identifier callee in expression position: the function
/// namespace first (walking outward), then variables holding function
/// values; user definitions shadow builtins.
fn resolve_function(name: &str, scope: &Rc<Scope>) -> Option<(Vec<Param>, Expr, Rc<Scope>)> {
    let mut cur = Some(scope.clone());
    while let Some(s) = cur {
        if let Some(f) = s.funcs.get(name) {
            return Some((f.params.clone(), f.body.clone(), s.clone()));
        }
        if let Some(Value::Function(fv)) = s.vars.borrow().get(name) {
            return Some((fv.params.clone(), fv.body.clone(), fn_env(fv)));
        }
        cur = s.parent.clone();
    }
    None
}

/// What a tail-position evaluation produced: a final value, or a tail
/// call to run in the caller's reused frame.
enum Tail {
    Done(Value),
    Next { params: Vec<Param>, body: Expr, env: Rc<Scope>, args: Vec<EvArg> },
}

/// Call a user function (named or value) with tail-call elimination: the
/// taken ternary branch, let bodies, and echo/assert trailing expressions
/// are tail positions.
fn call_function(
    name: &str,
    mut params: Vec<Param>,
    mut body: Expr,
    mut env: Rc<Scope>,
    mut args: Vec<EvArg>,
    ctx: &mut Ctx,
) -> Value {
    ctx.fn_depth += 1;
    if ctx.fn_depth > MAX_FN_DEPTH {
        ctx.fn_depth -= 1;
        ctx.halt(format!("ERROR: Recursion detected calling function '{}'", name));
        return Value::Undef;
    }
    let entry_dyn = ctx.dynv.clone();
    let mut iters = 0usize;
    let result = loop {
        if ctx.halted() {
            break Value::Undef;
        }
        iters += 1;
        if iters > MAX_TAIL_ITERS {
            ctx.halt(format!(
                "ERROR: function '{}' exceeded {} tail-recursive iterations",
                name, MAX_TAIL_ITERS
            ));
            break Value::Undef;
        }
        // Fresh dynamic layer per iteration (so tail loops don't grow the
        // chain), carrying this call's $-args.
        ctx.dynv = DynScope::layer(&entry_dyn);
        for a in &args {
            if let Some(n) = &a.name {
                if n.starts_with('$') {
                    ctx.dynv.vars.borrow_mut().insert(n.clone(), a.value.clone());
                }
            }
        }
        let param_scope = bind_params(name, &params, &args, &env, ctx);
        match eval_tail(&body, &param_scope, ctx) {
            Tail::Done(v) => break v,
            Tail::Next { params: p, body: b, env: e, args: a } => {
                params = p;
                body = b;
                env = e;
                args = a;
            }
        }
    };
    ctx.dynv = entry_dyn;
    ctx.fn_depth -= 1;
    result
}

fn eval_tail(expr: &Expr, scope: &Rc<Scope>, ctx: &mut Ctx) -> Tail {
    if ctx.halted() {
        return Tail::Done(Value::Undef);
    }
    match expr {
        Expr::Ternary { cond, then, els } => {
            let c = eval_expr(cond, scope, ctx);
            if truthy(&c) {
                eval_tail(then, scope, ctx)
            } else {
                eval_tail(els, scope, ctx)
            }
        }
        Expr::Let { bindings, body } => {
            // The binding layer stays lexically captured by the tail
            // continuation; the dynamic layer belongs to this frame.
            let bind_scope = sequential_bindings(bindings, scope, ctx);
            eval_tail(body, &bind_scope, ctx)
        }
        Expr::EchoExpr { args, body } => {
            emit_echo(args, scope, ctx);
            eval_tail(body, scope, ctx)
        }
        Expr::AssertExpr { args, body } => {
            let ev = eval_args(args, scope, ctx);
            let (cond, msg) = assert_args(&ev);
            if !truthy(&cond) {
                let text = args
                    .first()
                    .filter(|a| a.name.is_none())
                    .map(|a| serialize_expr(&a.value));
                ctx.halt(assert_failure_message(text.as_deref(), msg.as_ref()));
                return Tail::Done(Value::Undef);
            }
            eval_tail(body, scope, ctx)
        }
        Expr::Call { name, args } => match resolve_function(name, scope) {
            Some((params, body, env)) => {
                let ev = eval_args(args, scope, ctx);
                Tail::Next { params, body, env, args: ev }
            }
            None => Tail::Done(eval_expr(expr, scope, ctx)),
        },
        Expr::CallValue { callee, args } => {
            let f = eval_expr(callee, scope, ctx);
            match f {
                Value::Function(fv) => {
                    let ev = eval_args(args, scope, ctx);
                    Tail::Next {
                        params: fv.params.clone(),
                        body: fv.body.clone(),
                        env: fn_env(&fv),
                        args: ev,
                    }
                }
                other => {
                    ctx.out
                        .warnings
                        .push(format!("can't call a {} as a function", other.type_name()));
                    Tail::Done(Value::Undef)
                }
            }
        }
        _ => Tail::Done(eval_expr(expr, scope, ctx)),
    }
}

// ---------------------------------------------------------------------------
// Vector literals and comprehensions

fn eval_vec_items(items: &[VecItem], scope: &Rc<Scope>, ctx: &mut Ctx, out: &mut Vec<Value>) {
    for item in items {
        if ctx.halted() {
            return;
        }
        match item {
            VecItem::One(e) => out.push(eval_expr(e, scope, ctx)),
            VecItem::Each(e) => {
                let v = eval_expr(e, scope, ctx);
                match v {
                    Value::Vector(inner) => out.extend(inner),
                    Value::Str(s) => {
                        out.extend(s.chars().map(|c| Value::Str(c.to_string())));
                    }
                    r @ Value::Range { .. } => out.extend(iterate(&r, ctx)),
                    other => out.push(other),
                }
            }
            VecItem::CFor { bindings, rest } => {
                let iterables: Vec<(String, Vec<Value>)> = bindings
                    .iter()
                    .map(|(name, expr)| {
                        let v = eval_expr(expr, scope, ctx);
                        (name.clone(), iterate(&v, ctx))
                    })
                    .collect();
                cross_product(&iterables, 0, &mut Vec::new(), &mut |vals, ctx| {
                    let saved_dyn = ctx.dynv.clone();
                    ctx.dynv = DynScope::layer(&saved_dyn);
                    let iter_scope = Scope::child(scope);
                    for (name, v) in vals {
                        set_var(&iter_scope, ctx, name, v.clone());
                    }
                    eval_vec_items(rest, &iter_scope, ctx, out);
                    ctx.dynv = saved_dyn;
                }, ctx);
            }
            VecItem::CForC { inits, cond, updates, rest } => {
                let mut cur = sequential_bindings(inits, scope, ctx);
                let mut iters = 0usize;
                loop {
                    if ctx.halted() {
                        return;
                    }
                    let c = eval_expr(cond, &cur, ctx);
                    if !truthy(&c) {
                        break;
                    }
                    eval_vec_items(rest, &cur, ctx, out);
                    // SIMULTANEOUS updates: all RHS evaluate against the
                    // current iteration before any rebinding, so
                    // a = b, b = a + b really is the Fibonacci step.
                    let new_vals: Vec<(String, Value)> = updates
                        .iter()
                        .map(|(name, expr)| (name.clone(), eval_expr(expr, &cur, ctx)))
                        .collect();
                    let next = Scope::child(scope);
                    // Carry over every init binding, then apply updates.
                    for (name, _) in inits {
                        if let Some(v) = cur.lookup(name) {
                            next.vars.borrow_mut().insert(name.clone(), v);
                        }
                    }
                    for (name, v) in new_vals {
                        set_var(&next, ctx, &name, v);
                    }
                    cur = next;
                    iters += 1;
                    if iters >= MAX_GENERATOR_ITERS {
                        ctx.out.warnings.push(format!(
                            "generator for: truncated at {} iterations",
                            MAX_GENERATOR_ITERS
                        ));
                        break;
                    }
                }
            }
            VecItem::CIf { cond, then, els } => {
                let c = eval_expr(cond, scope, ctx);
                if truthy(&c) {
                    eval_vec_items(then, scope, ctx, out);
                } else {
                    eval_vec_items(els, scope, ctx, out);
                }
            }
            VecItem::CLet { bindings, rest } => {
                let saved_dyn = ctx.dynv.clone();
                ctx.dynv = DynScope::layer(&saved_dyn);
                let bind_scope = sequential_bindings(bindings, scope, ctx);
                eval_vec_items(rest, &bind_scope, ctx, out);
                ctx.dynv = saved_dyn;
            }
        }
    }
}

// -- Operator semantics (per the language reference) ------------------------

/// Exactly five falsy values: false, 0/-0, "", [], undef. Everything else
/// is truthy — including nan (the test is a plain != 0), "false", [0],
/// ranges, and function values.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Num(n) => *n != 0.0, // nan != 0 is true → truthy
        Value::Str(s) => !s.is_empty(),
        Value::Vector(items) => !items.is_empty(),
        Value::Range { .. } => true,
        Value::Function(_) => true,
        Value::Undef => false,
    }
}

/// Deep structural equality, total over all value pairs. Numbers by IEEE
/// (nan never equal, even to itself), cross-type simply unequal,
/// undef == undef true, ranges by begin/step/end, functions by identity.
fn value_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Num(x), Value::Num(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Undef, Value::Undef) => true,
        (Value::Vector(x), Value::Vector(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| value_eq(a, b))
        }
        (
            Value::Range { start: s1, step: t1, end: e1, .. },
            Value::Range { start: s2, step: t2, end: e2, .. },
        ) => s1 == s2 && t1 == t2 && e1 == e2,
        (Value::Function(x), Value::Function(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

fn range_count(start: f64, step: f64, end: f64) -> f64 {
    if step > 0.0 && start <= end {
        ((end - start) / step).floor() + 1.0
    } else if step < 0.0 && start >= end {
        ((start - end) / -step).floor() + 1.0
    } else {
        0.0
    }
}

/// Three-way ordering for the 2021.01 generic relational operators.
/// Ok(Some(_)): ordered. Ok(None): same-type but IEEE-unordered (nan) —
/// every relational is false. Err(msg): undefined operation → undef.
fn value_cmp(a: &Value, b: &Value) -> Result<Option<std::cmp::Ordering>, String> {
    use std::cmp::Ordering;
    match (a, b) {
        (Value::Num(x), Value::Num(y)) => Ok(x.partial_cmp(y)),
        // Strings order by Unicode code point (UTF-8 byte order agrees).
        (Value::Str(x), Value::Str(y)) => Ok(Some(x.cmp(y))),
        (Value::Bool(x), Value::Bool(y)) => Ok(Some(x.cmp(y))), // false < true
        (Value::Vector(x), Value::Vector(y)) => {
            for (i, (ea, eb)) in x.iter().zip(y.iter()).enumerate() {
                match value_cmp(ea, eb) {
                    Ok(Some(Ordering::Equal)) => continue,
                    Ok(Some(order)) => return Ok(Some(order)),
                    // An incomparable element pair propagates undef.
                    Ok(None) => return Err(format!("in vector comparison at index {}", i)),
                    Err(e) => return Err(format!("{} (in vector comparison at index {})", e, i)),
                }
            }
            // A strict prefix sorts before the longer vector.
            Ok(Some(x.len().cmp(&y.len())))
        }
        (
            Value::Range { start: s1, step: t1, end: e1, .. },
            Value::Range { start: s2, step: t2, end: e2, .. },
        ) => {
            // Ranges order by begin, then step, then element count.
            let keys1 = [*s1, *t1, range_count(*s1, *t1, *e1)];
            let keys2 = [*s2, *t2, range_count(*s2, *t2, *e2)];
            for (k1, k2) in keys1.iter().zip(&keys2) {
                match k1.partial_cmp(k2) {
                    Some(Ordering::Equal) => continue,
                    Some(order) => return Ok(Some(order)),
                    None => return Ok(None),
                }
            }
            Ok(Some(Ordering::Equal))
        }
        _ => Err(format!("undefined operation ({} < {})", a.type_name(), b.type_name())),
    }
}

fn negate(v: Value, ctx: &mut Ctx) -> Value {
    match v {
        Value::Num(n) => Value::Num(-n),
        // Recursive so -matrix works.
        Value::Vector(items) => {
            Value::Vector(items.into_iter().map(|item| negate(item, ctx)).collect())
        }
        other => {
            ctx.out
                .warnings
                .push(format!("undefined operation (- {})", other.type_name()));
            Value::Undef
        }
    }
}

/// + and -: numbers, or equal-length vectors elementwise (recursive, so
/// matrix+matrix works). NO broadcasting: scalar+vector is undef.
fn add_sub(sub: bool, l: &Value, r: &Value, ctx: &mut Ctx) -> Value {
    match (l, r) {
        (Value::Num(a), Value::Num(b)) => Value::Num(if sub { a - b } else { a + b }),
        (Value::Vector(x), Value::Vector(y)) if x.len() == y.len() => Value::Vector(
            x.iter().zip(y).map(|(a, b)| add_sub(sub, a, b, ctx)).collect(),
        ),
        _ => {
            ctx.out.warnings.push(format!(
                "undefined operation ({} {} {})",
                l.type_name(),
                if sub { "-" } else { "+" },
                r.type_name()
            ));
            Value::Undef
        }
    }
}

fn numeric_vec(v: &[Value]) -> Option<Vec<f64>> {
    v.iter().map(Value::as_num).collect()
}

fn matrix_rows(v: &[Value]) -> Option<Vec<Vec<f64>>> {
    let rows: Option<Vec<Vec<f64>>> = v
        .iter()
        .map(|row| match row {
            Value::Vector(cells) => numeric_vec(cells),
            _ => None,
        })
        .collect();
    let rows = rows?;
    // Reject ragged matrices.
    match rows.first() {
        Some(first) if rows.iter().all(|r| r.len() == first.len()) && !first.is_empty() => {
            Some(rows)
        }
        _ => None,
    }
}

fn num_vector(v: Vec<f64>) -> Value {
    Value::Vector(v.into_iter().map(Value::Num).collect())
}

/// * dispatches by shape: num*num, num*vector (elementwise, recursive),
/// vector*vector = DOT PRODUCT, matrix*vector, vector*matrix,
/// matrix*matrix.
fn multiply(l: &Value, r: &Value, ctx: &mut Ctx) -> Value {
    fn scale(n: f64, v: &Value, ctx: &mut Ctx) -> Value {
        match v {
            Value::Num(m) => Value::Num(n * m),
            Value::Vector(items) => {
                Value::Vector(items.iter().map(|item| scale(n, item, ctx)).collect())
            }
            other => {
                ctx.out
                    .warnings
                    .push(format!("undefined operation (number * {})", other.type_name()));
                Value::Undef
            }
        }
    }
    match (l, r) {
        (Value::Num(a), Value::Num(b)) => Value::Num(a * b),
        (Value::Num(n), v @ Value::Vector(_)) | (v @ Value::Vector(_), Value::Num(n)) => {
            scale(*n, v, ctx)
        }
        (Value::Vector(x), Value::Vector(y)) => {
            match (numeric_vec(x), numeric_vec(y), matrix_rows(x), matrix_rows(y)) {
                // vector · vector — a scalar (the classic trap).
                (Some(a), Some(b), _, _) if a.len() == b.len() => {
                    Value::Num(a.iter().zip(&b).map(|(p, q)| p * q).sum())
                }
                // matrix * vector: rows dot v.
                (_, Some(v), Some(m), _) if m[0].len() == v.len() => num_vector(
                    m.iter().map(|row| row.iter().zip(&v).map(|(p, q)| p * q).sum()).collect(),
                ),
                // vector * matrix: v dot columns.
                (Some(v), _, _, Some(m)) if m.len() == v.len() => num_vector(
                    (0..m[0].len())
                        .map(|j| v.iter().zip(&m).map(|(p, row)| p * row[j]).sum())
                        .collect(),
                ),
                // matrix * matrix.
                (_, _, Some(a), Some(b)) if a[0].len() == b.len() => Value::Vector(
                    a.iter()
                        .map(|row| {
                            num_vector(
                                (0..b[0].len())
                                    .map(|j| {
                                        row.iter().zip(&b).map(|(p, brow)| p * brow[j]).sum()
                                    })
                                    .collect(),
                            )
                        })
                        .collect(),
                ),
                _ => {
                    ctx.out
                        .warnings
                        .push("undefined operation (vector * vector: shape mismatch)".into());
                    Value::Undef
                }
            }
        }
        _ => {
            ctx.out.warnings.push(format!(
                "undefined operation ({} * {})",
                l.type_name(),
                r.type_name()
            ));
            Value::Undef
        }
    }
}

/// /: num/num; vector/num and num/vector elementwise (recursive);
/// vector/vector is undef.
fn divide(l: &Value, r: &Value, ctx: &mut Ctx) -> Value {
    match (l, r) {
        (Value::Num(a), Value::Num(b)) => {
            if *b == 0.0 {
                ctx.out.warnings.push("division by zero".into());
            }
            Value::Num(a / b)
        }
        (Value::Vector(items), Value::Num(_)) => Value::Vector(
            items.iter().map(|item| divide(item, r, ctx)).collect(),
        ),
        (Value::Num(_), Value::Vector(items)) => Value::Vector(
            items.iter().map(|item| divide(l, item, ctx)).collect(),
        ),
        _ => {
            ctx.out.warnings.push(format!(
                "undefined operation ({} / {})",
                l.type_name(),
                r.type_name()
            ));
            Value::Undef
        }
    }
}

fn binary_op(op: BinOp, l: Value, r: Value, ctx: &mut Ctx) -> Value {
    match op {
        BinOp::Add => add_sub(false, &l, &r, ctx),
        BinOp::Sub => add_sub(true, &l, &r, ctx),
        BinOp::Mul => multiply(&l, &r, ctx),
        BinOp::Div => divide(&l, &r, ctx),
        BinOp::Mod => match (l.as_num(), r.as_num()) {
            (Some(a), Some(b)) => Value::Num(a % b), // C fmod semantics
            _ => {
                ctx.out.warnings.push(format!(
                    "undefined operation ({} % {})",
                    l.type_name(),
                    r.type_name()
                ));
                Value::Undef
            }
        },
        BinOp::Pow => match (l.as_num(), r.as_num()) {
            (Some(a), Some(b)) => Value::Num(a.powf(b)), // C pow: 0^0 == 1
            _ => {
                ctx.out.warnings.push(format!(
                    "undefined operation ({} ^ {})",
                    l.type_name(),
                    r.type_name()
                ));
                Value::Undef
            }
        },
        BinOp::Eq => Value::Bool(value_eq(&l, &r)),
        BinOp::Ne => Value::Bool(!value_eq(&l, &r)),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => match value_cmp(&l, &r) {
            Ok(Some(order)) => Value::Bool(match op {
                BinOp::Lt => order.is_lt(),
                BinOp::Le => order.is_le(),
                BinOp::Gt => order.is_gt(),
                _ => order.is_ge(),
            }),
            // nan: every relational is false, not undef.
            Ok(None) => Value::Bool(false),
            Err(msg) => {
                ctx.out.warnings.push(msg);
                Value::Undef
            }
        },
        BinOp::And | BinOp::Or => unreachable!("short-circuit ops handled in eval_expr"),
    }
}

/// v[i]: the index TRUNCATES toward zero (v[1.5] == v[1]); negative,
/// non-finite, out-of-bounds, or non-numeric indices yield undef,
/// silently. Strings index by code point; ranges expose [0]/[1]/[2] =
/// begin/step/end.
fn index_value(base: &Value, index: &Value) -> Value {
    let i = match index {
        Value::Num(n) if n.is_finite() && *n >= 0.0 => n.trunc() as usize,
        _ => return Value::Undef,
    };
    match base {
        Value::Vector(items) => items.get(i).cloned().unwrap_or(Value::Undef),
        Value::Str(s) => s
            .chars()
            .nth(i)
            .map(|c| Value::Str(c.to_string()))
            .unwrap_or(Value::Undef),
        Value::Range { start, step, end, .. } => match i {
            0 => Value::Num(*start),
            1 => Value::Num(*step),
            2 => Value::Num(*end),
            _ => Value::Undef,
        },
        _ => Value::Undef,
    }
}

/// .x/.y/.z on vectors, .begin/.step/.end on ranges; every other member
/// or receiver is undef, silently.
fn member_value(base: &Value, name: &str) -> Value {
    match (base, name) {
        (Value::Vector(items), "x") => items.first().cloned().unwrap_or(Value::Undef),
        (Value::Vector(items), "y") => items.get(1).cloned().unwrap_or(Value::Undef),
        (Value::Vector(items), "z") => items.get(2).cloned().unwrap_or(Value::Undef),
        (Value::Range { start, .. }, "begin") => Value::Num(*start),
        (Value::Range { step, .. }, "step") => Value::Num(*step),
        (Value::Range { end, .. }, "end") => Value::Num(*end),
        _ => Value::Undef,
    }
}

// ---------------------------------------------------------------------------
// The shared value formatter (str / echo / assert messages)

/// The reference's 6-significant-digit number formatter: integers without
/// a decimal point, fixed notation for 1e-5 <= |x| < 1e6, scientific with
/// an unpadded signed exponent outside that, "0" for negative zero.
pub fn fmt_num(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf".into() } else { "-inf".into() };
    }
    if x == 0.0 {
        return "0".into(); // UNIQUE_ZERO: -0 renders as 0
    }
    // Round to 6 significant digits first; the exponent decides notation.
    let sci = format!("{:.5e}", x);
    let (mantissa, exp) = sci.split_once('e').expect("{:.5e} always has an exponent");
    let exp: i32 = exp.parse().expect("exponent is an integer");
    if (-5..6).contains(&exp) {
        let prec = (5 - exp).max(0) as usize;
        let fixed = format!("{:.*}", prec, x);
        let trimmed = if fixed.contains('.') {
            fixed.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            fixed
        };
        trimmed
    } else {
        let m = if mantissa.contains('.') {
            mantissa.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            mantissa.to_string()
        };
        format!("{}e{}{}", m, if exp >= 0 { "+" } else { "-" }, exp.abs())
    }
}

/// Render a value the way echo does. `quote_strings` is false only for
/// str()'s TOP-LEVEL string arguments (the concatenation idiom); strings
/// nested in vectors always quote. Interior quotes are not re-escaped.
pub fn fmt_value(v: &Value, quote_strings: bool) -> String {
    match v {
        Value::Num(n) => fmt_num(*n),
        Value::Bool(b) => if *b { "true".into() } else { "false".into() },
        Value::Undef => "undef".into(),
        Value::Str(s) => {
            if quote_strings {
                format!("\"{}\"", s)
            } else {
                s.clone()
            }
        }
        Value::Vector(items) => {
            let inner: Vec<String> = items.iter().map(|i| fmt_value(i, true)).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Range { start, step, end, .. } => {
            format!("[{} : {} : {}]", fmt_num(*start), fmt_num(*step), fmt_num(*end))
        }
        Value::Function(fv) => {
            let params: Vec<String> = fv
                .params
                .iter()
                .map(|p| match &p.default {
                    Some(d) => format!("{} = {}", p.name, serialize_expr(d)),
                    None => p.name.clone(),
                })
                .collect();
            format!("function({}) {}", params.join(", "), serialize_expr(&fv.body))
        }
    }
}

/// Source-like rendering of an expression, used for assert's condition
/// text and function-value display.
pub fn serialize_expr(e: &Expr) -> String {
    match e {
        Expr::Num(n) => fmt_num(*n),
        Expr::Bool(b) => if *b { "true".into() } else { "false".into() },
        Expr::Str(s) => format!("\"{}\"", s),
        Expr::Undef => "undef".into(),
        Expr::Ident(s) => s.clone(),
        Expr::Vector(items) => {
            let inner: Vec<String> = items
                .iter()
                .map(|i| match i {
                    VecItem::One(e) => serialize_expr(e),
                    VecItem::Each(e) => format!("each {}", serialize_expr(e)),
                    _ => "...".into(),
                })
                .collect();
            format!("[{}]", inner.join(", "))
        }
        Expr::Range { start, step, end } => match step {
            Some(s) => format!(
                "[{} : {} : {}]",
                serialize_expr(start),
                serialize_expr(s),
                serialize_expr(end)
            ),
            None => format!("[{} : {}]", serialize_expr(start), serialize_expr(end)),
        },
        Expr::Binary { op, lhs, rhs } => {
            let op = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%",
                BinOp::Pow => "^",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::Eq => "==",
                BinOp::Ne => "!=",
                BinOp::And => "&&",
                BinOp::Or => "||",
            };
            format!("({} {} {})", serialize_expr(lhs), op, serialize_expr(rhs))
        }
        Expr::Neg(inner) => format!("-{}", serialize_expr(inner)),
        Expr::Pos(inner) => format!("+{}", serialize_expr(inner)),
        Expr::Not(inner) => format!("!{}", serialize_expr(inner)),
        Expr::Ternary { cond, then, els } => format!(
            "({} ? {} : {})",
            serialize_expr(cond),
            serialize_expr(then),
            serialize_expr(els)
        ),
        Expr::Index { base, index } => {
            format!("{}[{}]", serialize_expr(base), serialize_expr(index))
        }
        Expr::Member { base, name } => format!("{}.{}", serialize_expr(base), name),
        Expr::Call { name, args } => format!("{}({})", name, serialize_args(args)),
        Expr::CallValue { callee, args } => {
            format!("{}({})", serialize_expr(callee), serialize_args(args))
        }
        Expr::Let { bindings, body } => {
            format!("let ({}) {}", serialize_bindings(bindings), serialize_expr(body))
        }
        Expr::EchoExpr { args, body } => {
            format!("echo({}) {}", serialize_args(args), serialize_expr(body))
        }
        Expr::AssertExpr { args, body } => {
            format!("assert({}) {}", serialize_args(args), serialize_expr(body))
        }
        Expr::FnLiteral { params, body } => {
            let params: Vec<String> = params
                .iter()
                .map(|p| match &p.default {
                    Some(d) => format!("{} = {}", p.name, serialize_expr(d)),
                    None => p.name.clone(),
                })
                .collect();
            format!("function({}) {}", params.join(", "), serialize_expr(body))
        }
    }
}

fn serialize_args(args: &[Arg]) -> String {
    args.iter()
        .map(|a| match &a.name {
            Some(n) => format!("{} = {}", n, serialize_expr(&a.value)),
            None => serialize_expr(&a.value),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn serialize_bindings(b: &[(String, Expr)]) -> String {
    b.iter()
        .map(|(n, e)| format!("{} = {}", n, serialize_expr(e)))
        .collect::<Vec<_>>()
        .join(", ")
}

// -- Builtin functions -------------------------------------------------------

/// Trig is in DEGREES with exact values at every multiple of 30 and 45
/// (degree-exact trig, 2019.05+): the angle folds mod 360 first, so
/// sin(36000) == 0 exactly and sin(30) == 0.5 exactly.
fn exact_sin_deg(x: f64) -> Option<f64> {
    let mut r = x % 360.0;
    if r < 0.0 {
        r += 360.0;
    }
    let table: &[(f64, f64)] = &[
        (0.0, 0.0),
        (30.0, 0.5),
        (45.0, std::f64::consts::FRAC_1_SQRT_2),
        (60.0, 3.0_f64.sqrt() / 2.0),
        (90.0, 1.0),
        (120.0, 3.0_f64.sqrt() / 2.0),
        (135.0, std::f64::consts::FRAC_1_SQRT_2),
        (150.0, 0.5),
        (180.0, 0.0),
        (210.0, -0.5),
        (225.0, -std::f64::consts::FRAC_1_SQRT_2),
        (240.0, -(3.0_f64.sqrt()) / 2.0),
        (270.0, -1.0),
        (300.0, -(3.0_f64.sqrt()) / 2.0),
        (315.0, -std::f64::consts::FRAC_1_SQRT_2),
        (330.0, -0.5),
    ];
    table.iter().find(|(deg, _)| *deg == r).map(|(_, v)| *v)
}

/// Total-precision-loss guard from the reference: |angle| ≥ 2^52 * 360
/// returns NaN for sin/cos/tan.
const TRIG_MAX: f64 = 4_503_599_627_370_496.0 * 360.0; // 2^52 * 360

fn sin_deg(x: f64) -> f64 {
    if !x.is_finite() || x.abs() >= TRIG_MAX {
        return f64::NAN;
    }
    exact_sin_deg(x).unwrap_or_else(|| x.to_radians().sin())
}

fn cos_deg(x: f64) -> f64 {
    if !x.is_finite() || x.abs() >= TRIG_MAX {
        return f64::NAN;
    }
    exact_sin_deg(x + 90.0).unwrap_or_else(|| x.to_radians().cos())
}

/// A small deterministic PRNG for rands(): splitmix64 seeding a
/// xorshift64* stream. Not OpenSCAD's exact stream (that is
/// implementation-defined per platform anyway), but seeded calls are
/// reproducible within this implementation.
struct Prng(u64);

impl Prng {
    fn seeded(seed: f64) -> Prng {
        let mut z = seed.to_bits().wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Prng((z ^ (z >> 31)) | 1)
    }

    fn unseeded() -> Prng {
        use std::time::{SystemTime, UNIX_EPOCH};
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
            .unwrap_or(0x1234_5678);
        let n = COUNTER.fetch_add(0x9E37_79B9, std::sync::atomic::Ordering::Relaxed);
        Prng::seeded((nanos ^ n.rotate_left(17)) as f64)
    }

    fn next_unit(&mut self) -> f64 {
        // xorshift64*: good enough distribution for geometry jitter.
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let bits = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (bits >> 11) as f64 / (1u64 << 53) as f64 // uniform [0, 1)
    }
}

fn call_builtin(name: &str, ev: &[EvArg], scope: &Rc<Scope>, ctx: &mut Ctx) -> Value {
    let _ = scope;
    // Builtin functions bind positionally; $-named args were already
    // layered dynamically by callers that need them.
    let vals: Vec<Value> = ev
        .iter()
        .filter(|a| a.name.is_none())
        .map(|a| a.value.clone())
        .collect();
    let num = |i: usize| -> Option<f64> { vals.get(i).and_then(Value::as_num) };
    let one_num = |ctx: &mut Ctx, f: &dyn Fn(f64) -> f64| -> Value {
        match num(0) {
            Some(x) => Value::Num(f(x)),
            None => {
                ctx.out.warnings.push(format!("{}: expected a number", name));
                Value::Undef
            }
        }
    };

    match name {
        "abs" => one_num(ctx, &f64::abs),
        // Comparison-based sign: 0 for -0 and (per the VERIFY note) nan.
        "sign" => one_num(ctx, &|x| {
            if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                0.0
            }
        }),
        "sin" => one_num(ctx, &sin_deg),
        "cos" => one_num(ctx, &cos_deg),
        "tan" => one_num(ctx, &|x| {
            let (s, c) = (sin_deg(x), cos_deg(x));
            s / c
        }),
        "asin" => one_num(ctx, &|x| x.asin().to_degrees()),
        "acos" => one_num(ctx, &|x| x.acos().to_degrees()),
        "atan" => one_num(ctx, &|x| x.atan().to_degrees()),
        "atan2" => match (num(0), num(1)) {
            (Some(y), Some(x)) => Value::Num(y.atan2(x).to_degrees()),
            _ => {
                ctx.out.warnings.push("atan2: expected two numbers (y, x)".into());
                Value::Undef
            }
        },
        "floor" => one_num(ctx, &f64::floor),
        "ceil" => one_num(ctx, &f64::ceil),
        // Ties round away from zero: round(2.5)==3, round(-2.5)==-3.
        "round" => one_num(ctx, &f64::round),
        "ln" => one_num(ctx, &f64::ln),
        "log" => one_num(ctx, &f64::log10), // base 10 — ln is natural
        "exp" => one_num(ctx, &f64::exp),
        "sqrt" => one_num(ctx, &f64::sqrt),
        "pow" => match (num(0), num(1)) {
            (Some(b), Some(e)) => Value::Num(b.powf(e)), // identical to ^
            _ => {
                ctx.out.warnings.push("pow: expected two numbers (base, exponent)".into());
                Value::Undef
            }
        },
        "min" | "max" => min_max(name, &vals, ctx),
        "norm" => match vals.first() {
            Some(Value::Vector(items)) => match numeric_vec(items) {
                // Naive sum of squares (empty vector → 0).
                Some(nums) => Value::Num(nums.iter().map(|x| x * x).sum::<f64>().sqrt()),
                None => {
                    ctx.out.warnings.push("norm: vector elements must be numbers".into());
                    Value::Undef
                }
            },
            _ => {
                ctx.out.warnings.push("norm: expected a vector".into());
                Value::Undef
            }
        },
        "cross" => match (vals.first(), vals.get(1)) {
            (Some(Value::Vector(a)), Some(Value::Vector(b))) => {
                match (numeric_vec(a), numeric_vec(b)) {
                    (Some(a), Some(b)) if a.len() == 3 && b.len() == 3 => num_vector(vec![
                        a[1] * b[2] - a[2] * b[1],
                        a[2] * b[0] - a[0] * b[2],
                        a[0] * b[1] - a[1] * b[0],
                    ]),
                    // 2D form returns the scalar cross product.
                    (Some(a), Some(b)) if a.len() == 2 && b.len() == 2 => {
                        Value::Num(a[0] * b[1] - a[1] * b[0])
                    }
                    _ => {
                        ctx.out
                            .warnings
                            .push("cross: expected two numeric vectors of length 2 or 3".into());
                        Value::Undef
                    }
                }
            }
            _ => {
                ctx.out.warnings.push("cross: expected two vectors".into());
                Value::Undef
            }
        },
        // len: vectors by top-level count, strings by code POINTS;
        // everything else (ranges included) is undef.
        "len" => match vals.first() {
            Some(Value::Vector(items)) => Value::Num(items.len() as f64),
            Some(Value::Str(s)) => Value::Num(s.chars().count() as f64),
            _ => Value::Undef,
        },
        // concat: one-level append; non-vectors (strings and ranges
        // included) append as single values; zero arguments → [].
        "concat" => {
            let mut items = Vec::new();
            for v in &vals {
                match v {
                    Value::Vector(inner) => items.extend(inner.iter().cloned()),
                    other => items.push(other.clone()),
                }
            }
            Value::Vector(items)
        }
        "str" => {
            // Top-level strings are UNQUOTED (the concatenation idiom);
            // everything nested quotes like echo.
            Value::Str(vals.iter().map(|v| fmt_value(v, false)).collect())
        }
        "chr" => {
            let mut s = String::new();
            for v in &vals {
                chr_append(v, &mut s, ctx);
            }
            Value::Str(s)
        }
        "ord" => match vals.first() {
            Some(Value::Str(s)) => match s.chars().next() {
                Some(c) => Value::Num(c as u32 as f64),
                None => Value::Undef,
            },
            _ => Value::Undef,
        },
        "search" => search_builtin(&vals, ctx),
        "lookup" => lookup_builtin(&vals, ctx),
        "rands" => rands_builtin(&vals, ctx),
        "version" => Value::Vector(vec![Value::Num(2021.0), Value::Num(1.0)]),
        "version_num" => Value::Num(20210100.0),
        "parent_module" => {
            let n = match num(0) {
                Some(n) if n.is_finite() && n >= 0.0 => n.trunc() as usize,
                _ => {
                    ctx.out
                        .warnings
                        .push("parent_module: expected a non-negative number".into());
                    return Value::Undef;
                }
            };
            match ctx.mod_stack.len().checked_sub(n + 1).and_then(|i| ctx.mod_stack.get(i)) {
                Some(name) => Value::Str(name.clone()),
                None => {
                    ctx.out.warnings.push(format!(
                        "parent_module: index ({}) out of range ({} modules on the stack)",
                        n,
                        ctx.mod_stack.len()
                    ));
                    Value::Undef
                }
            }
        }
        "is_undef" => Value::Bool(matches!(vals.first(), Some(Value::Undef) | None)),
        // is_num carve-out: false for nan.
        "is_num" => Value::Bool(matches!(vals.first(), Some(Value::Num(n)) if !n.is_nan())),
        "is_bool" => Value::Bool(matches!(vals.first(), Some(Value::Bool(_)))),
        "is_string" => Value::Bool(matches!(vals.first(), Some(Value::Str(_)))),
        // Ranges are NOT lists.
        "is_list" => Value::Bool(matches!(vals.first(), Some(Value::Vector(_)))),
        "is_function" => Value::Bool(matches!(vals.first(), Some(Value::Function(_)))),
        // No object values exist in this implementation.
        "is_object" => Value::Bool(false),
        other => {
            ctx.out.warnings.push(format!("unknown function '{}'", other));
            Value::Undef
        }
    }
}

/// chr(): numbers become code points (truncated toward zero), vectors
/// contribute per element, ranges expand; invalid code points contribute
/// nothing.
fn chr_append(v: &Value, s: &mut String, ctx: &mut Ctx) {
    match v {
        Value::Num(n) => {
            if n.is_finite() && *n >= 1.0 {
                if let Some(c) = char::from_u32(n.trunc() as i64 as u32) {
                    s.push(c);
                }
            }
        }
        Value::Vector(items) => {
            for item in items {
                // Nested vectors contribute nothing (only numbers do).
                if matches!(item, Value::Num(_)) {
                    chr_append(item, s, ctx);
                }
            }
        }
        r @ Value::Range { .. } => {
            for item in iterate(r, ctx) {
                chr_append(&item, s, ctx);
            }
        }
        _ => {}
    }
}

/// search(): per-term scans with the mode-dependent (and deliberately
/// irregular) result shapes from the reference.
fn search_builtin(vals: &[Value], ctx: &mut Ctx) -> Value {
    let match_value = match vals.first() {
        Some(v) => v,
        None => {
            ctx.out.warnings.push("search: expected a search term and a target".into());
            return Value::Undef;
        }
    };
    let target = match vals.get(1) {
        Some(v) => v,
        None => {
            ctx.out.warnings.push("search: expected a target as the second argument".into());
            return Value::Undef;
        }
    };
    let per_match = vals.get(2).and_then(Value::as_num).unwrap_or(1.0);
    let per_match = if per_match.is_finite() && per_match >= 0.0 {
        per_match.trunc() as usize
    } else {
        0
    };
    let index_col = vals.get(3).and_then(Value::as_num).unwrap_or(0.0);
    let index_col = if index_col.is_finite() && index_col >= 0.0 {
        index_col.trunc() as usize
    } else {
        0
    };

    // A string match_value explodes into per-character terms.
    let terms: Vec<Value> = match match_value {
        Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
        Value::Vector(items) => items.clone(),
        other => vec![other.clone()],
    };

    // The target's comparable elements: string characters, or vector
    // elements (rows compare by their index_col column).
    let elements: Vec<Value> = match target {
        Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
        Value::Vector(items) => items
            .iter()
            .map(|item| match item {
                // Table rows compare by their index_col column.
                Value::Vector(row) => row.get(index_col).cloned().unwrap_or(Value::Undef),
                other => other.clone(),
            })
            .collect(),
        _ => {
            ctx.out.warnings.push("search: target must be a string or vector".into());
            return Value::Undef;
        }
    };

    let mut flat = Vec::new();
    let mut nested = Vec::new();
    for term in &terms {
        let matches: Vec<Value> = elements
            .iter()
            .enumerate()
            .filter(|(_, e)| value_eq(term, e))
            .map(|(i, _)| Value::Num(i as f64))
            .collect();
        if per_match == 1 {
            // Default mode: an unmatched term is skipped ENTIRELY (the
            // flat result can be shorter than the term list).
            match matches.first() {
                Some(first) => flat.push(first.clone()),
                None => ctx
                    .out
                    .warnings
                    .push(format!("search term not found: {}", fmt_value(term, true))),
            }
        } else {
            let take = if per_match == 0 { matches.len() } else { per_match };
            nested.push(Value::Vector(matches.into_iter().take(take).collect()));
        }
    }
    if per_match == 1 {
        Value::Vector(flat)
    } else {
        Value::Vector(nested)
    }
}

/// lookup(): exact hit, linear interpolation between bracketing keys, and
/// clamping at both ends.
fn lookup_builtin(vals: &[Value], ctx: &mut Ctx) -> Value {
    let key = match vals.first().and_then(Value::as_num) {
        Some(k) => k,
        None => {
            ctx.out.warnings.push("lookup: key must be a number".into());
            return Value::Undef;
        }
    };
    let table = match vals.get(1) {
        Some(Value::Vector(rows)) => rows,
        _ => {
            ctx.out.warnings.push("lookup: table must be a vector of [key, value] pairs".into());
            return Value::Undef;
        }
    };
    let mut pairs: Vec<(f64, f64)> = Vec::new();
    for row in table {
        match row {
            Value::Vector(kv) if kv.len() >= 2 => {
                match (kv[0].as_num(), kv[1].as_num()) {
                    (Some(k), Some(v)) => pairs.push((k, v)),
                    _ => ctx
                        .out
                        .warnings
                        .push("lookup: table rows must hold numeric [key, value]".into()),
                }
            }
            _ => ctx.out.warnings.push("lookup: table rows must be [key, value] pairs".into()),
        }
    }
    if pairs.is_empty() {
        ctx.out.warnings.push("lookup: empty table".into());
        return Value::Undef;
    }
    // Exact hit wins bit-exactly, before any interpolation arithmetic.
    if let Some((_, v)) = pairs.iter().find(|(k, _)| *k == key) {
        return Value::Num(*v);
    }
    let (lo, lo_v) = pairs
        .iter()
        .filter(|(k, _)| *k < key)
        .cloned()
        .fold(None::<(f64, f64)>, |best, cur| match best {
            Some(b) if b.0 >= cur.0 => Some(b),
            _ => Some(cur),
        })
        .map_or((f64::NEG_INFINITY, f64::NAN), |p| p);
    let (hi, hi_v) = pairs
        .iter()
        .filter(|(k, _)| *k > key)
        .cloned()
        .fold(None::<(f64, f64)>, |best, cur| match best {
            Some(b) if b.0 <= cur.0 => Some(b),
            _ => Some(cur),
        })
        .map_or((f64::INFINITY, f64::NAN), |p| p);
    if lo.is_infinite() {
        // Below the smallest key: clamp to the first value.
        return Value::Num(hi_v);
    }
    if hi.is_infinite() {
        return Value::Num(lo_v);
    }
    Value::Num(lo_v + (hi_v - lo_v) * (key - lo) / (hi - lo))
}

/// rands(min, max, count, seed): always a vector; seeded calls are
/// reproducible within this implementation.
fn rands_builtin(vals: &[Value], ctx: &mut Ctx) -> Value {
    let (min_v, max_v, count) = match (
        vals.first().and_then(Value::as_num),
        vals.get(1).and_then(Value::as_num),
        vals.get(2).and_then(Value::as_num),
    ) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => {
            ctx.out
                .warnings
                .push("rands: expected min_value, max_value, value_count".into());
            return Value::Undef;
        }
    };
    if !count.is_finite() || count < 0.0 {
        ctx.out
            .warnings
            .push("rands: cannot create a negative number of random values".into());
        return Value::Vector(Vec::new());
    }
    let n = count.trunc() as usize;
    let (lo, hi) = if min_v <= max_v { (min_v, max_v) } else { (max_v, min_v) };
    let mut rng = match vals.get(3).and_then(Value::as_num) {
        Some(seed) => Prng::seeded(seed),
        None => Prng::unseeded(),
    };
    Value::Vector(
        (0..n)
            .map(|_| Value::Num(lo + rng.next_unit() * (hi - lo)))
            .collect(),
    )
}

/// min/max: with 2+ arguments the extreme of the arguments; with exactly
/// one VECTOR argument the extreme element. Ordering is the generic
/// relational ordering (strings work). Empty vector or a single
/// non-vector argument is undef with a warning.
fn min_max(name: &str, vals: &[Value], ctx: &mut Ctx) -> Value {
    let items: Vec<Value> = match vals {
        [Value::Vector(items)] => items.clone(),
        _ if vals.len() >= 2 => vals.to_vec(),
        _ => {
            ctx.out
                .warnings
                .push(format!("{}: expected a vector or at least two arguments", name));
            return Value::Undef;
        }
    };
    if items.is_empty() {
        ctx.out.warnings.push(format!("{}: empty vector", name));
        return Value::Undef;
    }
    let mut best = items[0].clone();
    for item in &items[1..] {
        match value_cmp(item, &best) {
            Ok(Some(order)) => {
                let take = if name == "min" { order.is_lt() } else { order.is_gt() };
                if take {
                    best = item.clone();
                }
            }
            Ok(None) => {} // nan never wins a comparison
            Err(msg) => {
                ctx.out.warnings.push(format!("{}: {}", name, msg));
                return Value::Undef;
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn run(src: &str) -> EvalOutput {
        evaluate(&parse(src).unwrap())
    }

    /// Evaluate one expression against a fresh root scope; returns
    /// (value, warnings).
    fn ev(expr_src: &str) -> (Value, Vec<String>) {
        let prog = parse(&format!("x = {};", expr_src)).unwrap();
        let value = match &prog[0] {
            Stmt::Assign { value, .. } => value.clone(),
            other => panic!("expected assign, got {:?}", other),
        };
        let mut ctx = Ctx {
            out: EvalOutput::default(),
            dynv: DynScope::root(),
            mod_stack: Vec::new(),
            children: None,
            fn_depth: 0,
        };
        let root = Scope::root();
        let v = eval_expr(&value, &root, &mut ctx);
        (v, ctx.out.warnings)
    }

    fn n(expr_src: &str) -> f64 {
        match ev(expr_src) {
            (Value::Num(x), w) => {
                assert!(w.is_empty(), "{}: unexpected warnings {:?}", expr_src, w);
                x
            }
            (other, _) => panic!("{}: expected a number, got {:?}", expr_src, other),
        }
    }

    fn b(expr_src: &str) -> bool {
        match ev(expr_src) {
            (Value::Bool(x), _) => x,
            (other, _) => panic!("{}: expected a bool, got {:?}", expr_src, other),
        }
    }

    fn s(expr_src: &str) -> String {
        match ev(expr_src) {
            (Value::Str(x), _) => x,
            (other, _) => panic!("{}: expected a string, got {:?}", expr_src, other),
        }
    }

    // -- ported phase-1 pins ------------------------------------------------

    #[test]
    fn cube_translate_and_variables() {
        let out = run("s = 2; translate([10, 0, 0]) cube(s, center = true);");
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
        assert_eq!(out.shapes.len(), 1);
        let xs: Vec<f64> = out.shapes[0].mesh.positions.iter().map(|p| p[0]).collect();
        assert!(xs.iter().all(|&x| (x - 9.0).abs() < 1e-9 || (x - 11.0).abs() < 1e-9));
    }

    #[test]
    fn for_over_range_expands_inclusively() {
        let out = run("for (i = [0 : 2 : 6]) translate([i, 0, 0]) cube(1);");
        assert_eq!(out.shapes.len(), 4); // 0, 2, 4, 6 — end inclusive
        let out = run("for (i = [3, 5]) cube(i);");
        assert_eq!(out.shapes.len(), 2);
    }

    #[test]
    fn fn_special_variable_scopes_and_call_override() {
        // Env-level $fn.
        let out = run("$fn = 6; sphere(10);");
        assert_eq!(out.shapes[0].mesh.positions.len(), 6 * 3); // 6 meridians, 3 rings
        // Per-call named $fn wins over the scoped variable.
        let out = run("$fn = 6; sphere(10, $fn = 8);");
        assert_eq!(out.shapes[0].mesh.positions.len(), 8 * 4);
        // Default path: sphere(10) → 30 fragments, 15 rings.
        let out = run("sphere(10);");
        assert_eq!(out.shapes[0].mesh.positions.len(), 30 * 15);
    }

    #[test]
    fn nearest_color_wins() {
        let out = run("color(\"red\") color([0, 0, 1]) cube(1);");
        assert_eq!(out.shapes[0].color, Some([0.0, 0.0, 1.0, 1.0]));
        // Hex form with alpha override.
        let out = run("color(\"#00ff00\", alpha = 0.5) cube(1);");
        let c = out.shapes[0].color.unwrap();
        assert!((c[1] - 1.0).abs() < 1e-9 && (c[3] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn diameter_overrides_radius() {
        let out = run("sphere(r = 3, d = 4);");
        // d wins: radius 2 → all vertices at length 2.
        let p = out.shapes[0].mesh.positions[0];
        let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        assert!((len - 2.0).abs() < 1e-9);
        let out = run("cylinder(h = 4, d1 = 6, d2 = 0);");
        assert!(out.shapes[0].mesh.positions.iter().any(|p| *p == [0.0, 0.0, 4.0]));
    }

    #[test]
    fn unknown_module_warns_and_continues() {
        let out = run("frobnicate(1); cube(1);");
        assert_eq!(out.shapes.len(), 1);
        assert!(out.warnings.iter().any(|w| w.contains("frobnicate")));
        let out = run("difference() { cube(2); sphere(1); }");
        assert_eq!(out.shapes.len(), 2);
        assert!(out.warnings.iter().any(|w| w.contains("not implemented")));
        // An unknown module's children are skipped, not evaluated.
        let out = run("mystery() { cube(1); }");
        assert!(out.shapes.is_empty());
    }

    #[test]
    fn rotate_forms() {
        // Scalar = about Z: +X corner moves to +Y.
        let out = run("rotate(90) translate([5, 0, 0]) cube(1);");
        assert!(out.shapes[0].mesh.positions.iter().all(|p| p[1] >= 4.999));
        // Angle-axis about X sends +Y to +Z.
        let out = run("rotate(90, [1, 0, 0]) translate([0, 5, 0]) cube(1);");
        assert!(out.shapes[0].mesh.positions.iter().all(|p| p[2] >= 4.999));
    }

    #[test]
    fn bad_transform_args_drop_the_subtree_not_misplace_it() {
        let out = run("translate(5) cube(1);");
        assert!(out.shapes.is_empty());
        assert!(out.warnings.iter().any(|w| w.contains("translate")));
    }

    #[test]
    fn precedence_corners_match_the_reference() {
        assert_eq!(n("-2 ^ 2"), -4.0);
        assert_eq!(n("2 ^ -3"), 0.125);
        assert_eq!(n("2 ^ 3 ^ 2"), 512.0); // 2^(3^2)
        assert_eq!(n("0 ^ 0"), 1.0); // C pow convention
        assert!(!b("1 == 1 == 2"));
        assert!(b("true || false && false"));
        assert_eq!(n("-[5, 6][0]"), -5.0);
        assert_eq!(n("false ? 1 : true ? 2 : 3"), 2.0);
    }

    #[test]
    fn trig_is_degree_based_and_exact_at_special_angles() {
        assert_eq!(n("sin(30)"), 0.5); // EXACT, not 0.49999...
        assert_eq!(n("sin(180)"), 0.0); // not 1.2e-16
        assert_eq!(n("sin(36000)"), 0.0); // folded before evaluation
        assert_eq!(n("cos(90)"), 0.0);
        assert_eq!(n("sin(45)"), std::f64::consts::FRAC_1_SQRT_2);
        assert_eq!(n("tan(45)"), 1.0);
        assert_eq!(n("sin(210)"), -0.5);
        assert_eq!(n("atan2(1, 1)"), 45.0);
        assert_eq!(n("atan2(1, -1)"), 135.0);
        assert_eq!(n("asin(1)"), 90.0);
        assert!(n("sin(1e300)").is_nan()); // total-precision-loss guard
    }

    #[test]
    fn truthiness_has_exactly_five_falsy_values_and_nan_is_truthy() {
        for falsy in ["false", "0", "\"\"", "[]", "undef"] {
            assert!(b(&format!("!{}", falsy)), "{} must be falsy", falsy);
        }
        for truthy_v in ["\"false\"", "[0]", "[[]]", "-1", "[1:2]"] {
            assert!(!b(&format!("!{}", truthy_v)), "{} must be truthy", truthy_v);
        }
        let (v, w) = ev("(0/0) ? 1 : 2");
        assert_eq!(v, Value::Num(1.0));
        assert!(w.iter().all(|m| m.contains("division")), "{:?}", w);
        assert!(!b("!(0/0)"));
        // Function values are truthy.
        assert!(!b("!(function (x) x)"));
    }

    #[test]
    fn logic_short_circuits_and_returns_strict_booleans() {
        let (v, w) = ev("false && unknown_var");
        assert_eq!(v, Value::Bool(false));
        assert!(w.is_empty(), "short-circuit must not evaluate rhs: {:?}", w);
        let (v, w) = ev("true || unknown_var");
        assert_eq!(v, Value::Bool(true));
        assert!(w.is_empty());
        let (v, _) = ev("5 && 7");
        assert_eq!(v, Value::Bool(true));
        let (v, w) = ev("true ? 42 : unknown_var");
        assert_eq!(v, Value::Num(42.0));
        assert!(w.is_empty());
    }

    #[test]
    fn equality_is_total_deep_and_uncoerced() {
        assert!(b("undef == undef"));
        assert!(b("[1, [2, \"x\"]] == [1, [2, \"x\"]]"));
        assert!(!b("[1, 2] == [1, 2, 3]"));
        assert!(!b("1 == \"1\"")); // zero coercion
        assert!(!b("true == 1"));
        assert!(b("0 == -0"));
        assert!(!b("(0/0) == (0/0)"));
        assert!(b("(0/0) != (0/0)"));
        assert!(b("[0:5] == [0:1:5]"));
        assert!(!b("[0:5] == [0,1,2,3,4,5]")); // range != list
    }

    #[test]
    fn relational_ordering_is_generic_over_same_type_values() {
        assert!(b("\"Z\" < \"a\"")); // code points, no locale
        assert!(b("\"ab\" < \"abc\""));
        assert!(b("false < true"));
        assert!(b("[1, 2] < [1, 3]")); // lexicographic
        assert!(b("[1] < [1, 0]")); // strict prefix sorts first
        let (v, w) = ev("(0/0) < 1");
        assert_eq!(v, Value::Bool(false));
        assert!(w.iter().all(|m| m.contains("division")), "{:?}", w);
        let (v, w) = ev("1 < \"a\"");
        assert_eq!(v, Value::Undef);
        assert!(w.iter().any(|m| m.contains("undefined operation")), "{:?}", w);
    }

    #[test]
    fn indexing_truncates_and_never_wraps() {
        assert_eq!(n("[10, 20, 30][1.5]"), 20.0); // truncation toward zero
        let (v, _) = ev("[10, 20][-1]"); // no negative indexing
        assert_eq!(v, Value::Undef);
        let (v, _) = ev("[10, 20][5]");
        assert_eq!(v, Value::Undef);
        let (v, _) = ev("\"h\u{e4}x\"[1]");
        assert_eq!(v, Value::Str("\u{e4}".to_string()));
        assert_eq!(n("[2:3:11][1]"), 3.0);
        assert_eq!(n("[0:5][1]"), 1.0); // implicit step is 1
        assert_eq!(n("[[1, 2], [3, 4]][1][0]"), 3.0);
    }

    #[test]
    fn member_access_on_vectors_and_ranges() {
        assert_eq!(n("[7, 8, 9].y"), 8.0);
        assert_eq!(n("[[1, 2, 3], [4, 5, 6]][1].z"), 6.0);
        let (v, w) = ev("[1].y"); // short vector: silent undef
        assert_eq!(v, Value::Undef);
        assert!(w.is_empty());
        assert_eq!(n("[2:3:11].begin"), 2.0);
        assert_eq!(n("[2:3:11].step"), 3.0);
        assert_eq!(n("[2:3:11].end"), 11.0);
        let (v, w) = ev("\"abc\".x"); // non-vector receiver: silent undef
        assert_eq!(v, Value::Undef);
        assert!(w.is_empty());
    }

    #[test]
    fn arithmetic_shape_dispatch_matches_the_reference() {
        assert_eq!(ev("[1, 2] + [3, 4]").0, ev("[4, 6]").0);
        let (v, w) = ev("[1, 2] + 1");
        assert_eq!(v, Value::Undef);
        assert!(w.iter().any(|m| m.contains("undefined operation")));
        assert_eq!(ev("2 * [3, 4]").0, ev("[6, 8]").0);
        assert_eq!(n("[1, 2] * [3, 4]"), 11.0); // dot product, not elementwise
        assert_eq!(ev("[[1, 0], [0, 2]] * [3, 4]").0, ev("[3, 8]").0);
        assert_eq!(ev("[[1, 2]] * [[3], [4]]").0, ev("[[11]]").0);
        assert_eq!(ev("[10, 4] / 2").0, ev("[5, 2]").0);
        assert_eq!(ev("10 / [2, 5]").0, ev("[5, 2]").0);
        assert_eq!(ev("[1] / [1]").0, Value::Undef);
        assert_eq!(ev("-[[1, -2], [3, 4]]").0, ev("[[-1, 2], [-3, -4]]").0);
    }

    #[test]
    fn builtin_functions_follow_reference_semantics() {
        assert_eq!(n("min(5, 3, 8)"), 3.0);
        assert_eq!(n("max([4, 2, 9])"), 9.0);
        let (v, w) = ev("min([])");
        assert_eq!(v, Value::Undef);
        assert!(w.iter().any(|m| m.contains("empty")));
        assert_eq!(ev("max(\"a\", \"b\")").0, Value::Str("b".into())); // generic ordering
        assert_eq!(n("norm([3, 4])"), 5.0);
        assert_eq!(n("norm([])"), 0.0);
        assert_eq!(ev("cross([1, 0, 0], [0, 1, 0])").0, ev("[0, 0, 1]").0);
        assert_eq!(n("cross([1, 2], [3, 4])"), -2.0); // 2D scalar form
        assert_eq!(n("len(\"h\u{e4}x\")"), 3.0); // code points, not bytes
        assert_eq!(ev("len([0:10])").0, Value::Undef); // ranges are not lists
        assert_eq!(ev("concat([1], [2, 3], 4)").0, ev("[1, 2, 3, 4]").0);
        assert_eq!(ev("concat([[1]], [2])").0, ev("[[1], 2]").0);
        assert_eq!(ev("concat(\"ab\", \"cd\")").0, ev("[\"ab\", \"cd\"]").0);
        assert_eq!(ev("concat()").0, Value::Vector(vec![]));
        assert_eq!(n("round(2.5)"), 3.0);
        assert_eq!(n("round(-2.5)"), -3.0);
        assert_eq!(n("sign(-0)"), 0.0);
        assert_eq!(n("ln(exp(1))"), 1.0);
        assert_eq!(n("log(1000)"), 3.0); // base 10
        assert_eq!(n("pow(0, 0)"), 1.0);
        assert!((n("PI") - std::f64::consts::PI).abs() < 1e-15);
        assert!(!b("is_num(0/0)"));
        assert!(b("is_num(5)"));
        assert!(!b("is_list([0:10])"));
        assert!(b("is_list([])"));
        assert!(b("is_undef(undef)"));
        let (v, w) = ev("frob(1)");
        assert_eq!(v, Value::Undef);
        assert!(w.iter().any(|m| m.contains("unknown function 'frob'")));
    }

    #[test]
    fn reversed_ranges_follow_the_legacy_rules() {
        let out = run("for (i = [4:1]) cube(i);");
        assert_eq!(out.shapes.len(), 4);
        assert!(out.warnings.iter().any(|w| w.contains("DEPRECATED")));
        let out = run("for (i = [10:1:0]) cube(i);");
        assert!(out.shapes.is_empty());
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
        let out = run("for (i = [10:-2:0]) cube(1);");
        assert_eq!(out.shapes.len(), 6);
    }

    #[test]
    fn expressions_drive_geometry() {
        let out = run(
            "for (i = [0:3]) translate([i * 10, 0, 0]) \
             cylinder(h = 2 + 8 * sin(i * 30), r = i < 2 ? 1 : 3, $fn = 12);",
        );
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
        assert_eq!(out.shapes.len(), 4);
        // i=1: h = 2 + 8*sin(30) = 6 exactly (degree-exact trig).
        let zs: Vec<f64> = out.shapes[1].mesh.positions.iter().map(|p| p[2]).collect();
        assert!(zs.iter().cloned().fold(f64::MIN, f64::max) == 6.0);
    }

    // -- phase 2: scoping ---------------------------------------------------

    #[test]
    fn scoping_follows_the_two_phase_slot_rule() {
        // Last write wins at the FIRST slot position: b sees the final a.
        let out = run("a = 1; b = a; a = 2; echo(b);");
        assert_eq!(out.echoes, vec!["ECHO: 2"]);
        assert!(out.warnings.iter().any(|w| w.contains("reassigned")));
        // Statements observe final values even between the assignments.
        let out = run("a = 1; echo(a); a = 5;");
        assert_eq!(out.echoes, vec!["ECHO: 5"]);
        // Forward references between assignments do NOT work.
        let out = run("b = a; a = 2; echo(b);");
        assert_eq!(out.echoes, vec!["ECHO: undef"]);
        assert!(out.warnings.iter().any(|w| w.contains("unknown variable 'a'")));
        // Child scopes shadow; outer bindings never mutate.
        let out = run("x = 5; if (true) { echo(x); x = 6; } echo(x);");
        assert_eq!(out.echoes, vec!["ECHO: 6", "ECHO: 5"]);
        // Child-scope self-reference reads the OUTER value.
        let out = run("x = 5; if (true) { x = x + 1; echo(x); }");
        assert_eq!(out.echoes, vec!["ECHO: 6"]);
    }

    #[test]
    fn three_namespaces_coexist() {
        let out = run("r = 5; function r(x) = x * 2; module r() { cube(r); } echo(r, r(3)); r();");
        assert_eq!(out.echoes, vec!["ECHO: 5, 6"]);
        assert_eq!(out.shapes.len(), 1);
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    }

    #[test]
    fn if_else_only_instantiates_the_taken_branch() {
        let out = run("if (false) cube(1); else sphere(1, $fn = 6);");
        assert_eq!(out.shapes.len(), 1);
        assert!(out.shapes[0].mesh.positions.len() < 24); // the sphere, not the cube
        // The untaken branch produces nothing at all — not even echoes.
        let out = run("if (true) echo(\"yes\"); else echo(\"no\");");
        assert_eq!(out.echoes, vec!["ECHO: \"yes\""]);
        // nan conditions take the THEN branch (truthy).
        let out = run("if (0/0) cube(1);");
        assert_eq!(out.shapes.len(), 1);
        // else-if chains are plain nesting.
        let out = run("x = 2; if (x == 1) echo(1); else if (x == 2) echo(2); else echo(3);");
        assert_eq!(out.echoes, vec!["ECHO: 2"]);
    }

    #[test]
    fn let_and_assign_statements_bind_sequentially() {
        let out = run("let (a = 1, b = a + 1) cube([b, 1, 1]);");
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
        let xs: Vec<f64> = out.shapes[0].mesh.positions.iter().map(|p| p[0]).collect();
        assert!(xs.iter().cloned().fold(f64::MIN, f64::max) == 2.0);
        // let(a=a) reads the OUTER a.
        let out = run("a = 1; let (a = a + 1) echo(a); echo(a);");
        assert_eq!(out.echoes, vec!["ECHO: 2", "ECHO: 1"]);
        // assign() still works, with the deprecation diagnostic.
        let out = run("assign (a = 2) cube(a);");
        assert_eq!(out.shapes.len(), 1);
        assert!(out.warnings.iter().any(|w| w.contains("DEPRECATED") && w.contains("Assign")));
    }

    // -- phase 2: modules, functions, children ------------------------------

    #[test]
    fn modules_hoist_bind_and_default() {
        // Callable before the textual definition.
        let out = run("m(3); module m(a, b = a * 2) cube([a, b, 1]);");
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
        let ys: Vec<f64> = out.shapes[0].mesh.positions.iter().map(|p| p[1]).collect();
        assert!(ys.iter().cloned().fold(f64::MIN, f64::max) == 6.0); // b defaulted to a*2
        // Named arguments in any order; unknown named ones warn + drop.
        let out = run("module m(a, b) cube([a, b, 1]); m(b = 2, a = 1, bogus = 9);");
        assert_eq!(out.shapes.len(), 1);
        assert!(out.warnings.iter().any(|w| w.contains("bogus")));
        // Missing parameter without a default is undef (cube survives via
        // its own default size handling for undef).
        let out = run("module m(a) echo(a); m();");
        assert_eq!(out.echoes, vec!["ECHO: undef"]);
        // User definitions shadow builtins.
        let out = run("module cube(size) sphere(1, $fn = 6); cube(5);");
        assert_eq!(out.shapes[0].mesh.positions.len(), 6 * 3);
    }

    #[test]
    fn children_are_lazy_reinstantiated_and_indexed() {
        // children() stamps all children; repeated calls re-stamp.
        let out = run("module twice() { children(); children(); } twice() cube(1);");
        assert_eq!(out.shapes.len(), 2);
        // Index selection, and $children count.
        let out = run(
            "module pick() { children(1); echo($children); } \
             pick() { cube(1); sphere(1, $fn = 6); }",
        );
        assert_eq!(out.shapes.len(), 1);
        assert_eq!(out.shapes[0].mesh.positions.len(), 6 * 3); // the sphere
        assert_eq!(out.echoes, vec!["ECHO: 2"]);
        // Out-of-bounds warns and contributes nothing.
        let out = run("module p() children(5); p() cube(1);");
        assert!(out.shapes.is_empty());
        assert!(out.warnings.iter().any(|w| w.contains("out of bounds")));
        // Children re-evaluate afresh per stamping: echo fires twice.
        let out = run("module twice() { children(); children(); } twice() echo(\"hi\");");
        assert_eq!(out.echoes.len(), 2);
        // Legacy child() still works with a DEPRECATED diagnostic.
        let out = run("module one() child(0); one() cube(1);");
        assert_eq!(out.shapes.len(), 1);
        assert!(out.warnings.iter().any(|w| w.contains("child()")));
    }

    #[test]
    fn children_see_caller_lexicals_and_callee_dollars() {
        // Caller's lexical scope for plain names...
        let out = run("x = 1; module m() { x = 99; children(); } m() echo(x);");
        assert_eq!(out.echoes, vec!["ECHO: 1"]);
        // ...but the callee's dynamic $-environment.
        let out = run("module m() { $q = 5; children(); } m() echo($q);");
        assert_eq!(out.echoes, vec!["ECHO: 5"]);
        // The classic $fn propagation: the module re-tessellates its
        // children.
        let out = run("module fine() { $fn = 8; children(); } fine() sphere(10);");
        assert_eq!(out.shapes[0].mesh.positions.len(), 8 * 4);
    }

    #[test]
    fn functions_recurse_with_tail_call_elimination() {
        // 200k tail-recursive frames must complete (TCE contract).
        let out = run(
            "function count(nn, acc = 0) = nn <= 0 ? acc : count(nn - 1, acc + 1); \
             echo(count(200000));",
        );
        assert!(out.error.is_none(), "{:?}", out.error);
        assert_eq!(out.echoes, vec!["ECHO: 200000"]);
        // Tail position extends through let/echo/assert wrappers.
        let out = run(
            "function f(nn) = nn <= 0 ? \"done\" : let (m = nn - 1) f(m); \
             echo(f(150000));",
        );
        assert!(out.error.is_none(), "{:?}", out.error);
        assert_eq!(out.echoes, vec!["ECHO: \"done\""]);
        // Non-tail recursion hits the depth guard with a named error.
        let out = run("function f(nn) = nn <= 0 ? 0 : 1 + f(nn - 1); x = f(20000);");
        assert!(out.error.as_deref().unwrap_or("").contains("Recursion detected calling function 'f'"));
        // Module recursion is bounded too.
        let out = run("module r() r(); r();");
        assert!(out.error.as_deref().unwrap_or("").contains("Recursion detected calling module 'r'"));
    }

    #[test]
    fn function_literals_are_first_class() {
        // Storage, calling, currying, vector dispatch.
        let out = run(
            "adder = function (a) function (b) a + b; \
             fs = [function (x) x, function (x) -x]; \
             echo(adder(2)(3), fs[1](3));",
        );
        assert_eq!(out.echoes, vec!["ECHO: 5, -3"]);
        // Self-recursion through the storing variable.
        let out = run("fac = function (nn) nn <= 0 ? 1 : nn * fac(nn - 1); echo(fac(5));");
        assert_eq!(out.echoes, vec!["ECHO: 120"]);
        // is_function distinguishes values from named definitions.
        assert!(b("is_function(function (x) x)"));
        assert!(!b("is_function(5)"));
        // $-names resolve dynamically at CALL time, not capture time.
        let out = run("f = function () $q; module m() { $q = 5; echo(f()); } m();");
        assert_eq!(out.echoes, vec!["ECHO: 5"]);
        // Calling a non-function value warns and yields undef.
        let (v, w) = ev("[1, 2][0](9)");
        assert_eq!(v, Value::Undef);
        assert!(w.iter().any(|m| m.contains("can't call")));
    }

    // -- phase 2: comprehensions --------------------------------------------

    #[test]
    fn comprehensions_cover_every_clause() {
        assert_eq!(ev("[for (i = [0:4]) if (i % 2 == 0) i * i]").0, ev("[0, 4, 16]").0);
        // Cross product is a-major.
        assert_eq!(ev("[for (a = [1, 2], b = [10, 20]) a + b]").0, ev("[11, 21, 12, 22]").0);
        // C-style generator with SIMULTANEOUS updates: Fibonacci.
        assert_eq!(
            ev("[for (a = 0, b = 1; a < 10; a = b, b = a + b) a]").0,
            ev("[0, 1, 1, 2, 3, 5, 8]").0
        );
        // each splices one level; ranges materialize; strings split.
        assert_eq!(
            ev("[each [1, 2], each [3:5], each \"ab\", each 7]").0,
            ev("[1, 2, 3, 4, 5, \"a\", \"b\", 7]").0
        );
        assert_eq!(ev("[each [[1], [2]]]").0, ev("[[1], [2]]").0);
        // Mixing plain literals with clauses (2019.05).
        assert_eq!(ev("[0, for (i = [1:2]) i, 9]").0, ev("[0, 1, 2, 9]").0);
        // A lone if; let before the element; if/else arms.
        assert_eq!(ev("[if (false) 1]").0, Value::Vector(vec![]));
        assert_eq!(ev("[let (a = 1) a]").0, ev("[1]").0);
        assert_eq!(
            ev("[for (i = [0:3]) if (i % 2 == 0) i else -i]").0,
            ev("[0, -1, 2, -3]").0
        );
        // Scalars iterate once; strings iterate per code point.
        assert_eq!(ev("[for (i = 5) i]").0, ev("[5]").0);
        assert_eq!(ev("[for (c = \"abc\") c]").0, ev("[\"a\", \"b\", \"c\"]").0);
        let (v, _) = ev("[for (i = undef) i]");
        assert_eq!(v, Value::Vector(vec![Value::Undef]));
    }

    #[test]
    fn for_statement_iterates_strings_scalars_and_cross_products() {
        let out = run("for (c = \"ab\") cube(1);");
        assert_eq!(out.shapes.len(), 2);
        let out = run("for (i = 5) cube(i);");
        assert_eq!(out.shapes.len(), 1);
        let out = run("for (a = [0, 1], b = [0, 1, 2]) cube(a + b + 1);");
        assert_eq!(out.shapes.len(), 6); // cross product
    }

    // -- phase 2: echo / assert / str ---------------------------------------

    #[test]
    fn echo_renders_like_the_reference() {
        let out = run("echo(\"hi\", 42, [1, \"a\"], true, undef, [0:10]);");
        assert_eq!(
            out.echoes,
            vec!["ECHO: \"hi\", 42, [1, \"a\"], true, undef, [0 : 1 : 10]"]
        );
        // Named args print name = value; $-named ones bind, not print.
        let out = run("echo(b = 2, $fn = 5);");
        assert_eq!(out.echoes, vec!["ECHO: b = 2"]);
        // Expression form prints per evaluation and yields the body.
        let out = run("function t(x) = echo(\"trace\") x * 2; echo(t(1), t(2));");
        assert_eq!(out.echoes, vec!["ECHO: \"trace\"", "ECHO: \"trace\"", "ECHO: 2, 4"]);
    }

    #[test]
    fn assert_halts_evaluation_with_the_condition_text() {
        let out = run("cube(1); assert(1 + 1 == 3, \"math broke\"); cube(1);");
        assert_eq!(out.shapes.len(), 1); // nothing after the failure
        let err = out.error.unwrap();
        assert!(err.contains("Assertion") && err.contains("failed"), "{}", err);
        assert!(err.contains("((1 + 1) == 3)"), "{}", err);
        assert!(err.contains("\"math broke\""), "{}", err);
        // Success is silent; children instantiate.
        let out = run("assert(true) cube(1);");
        assert!(out.error.is_none());
        assert_eq!(out.shapes.len(), 1);
        // Truthiness traps: [] fails, "false" passes, nan passes.
        assert!(run("assert([]);").error.is_some());
        assert!(run("assert(\"false\");").error.is_none());
        assert!(run("assert(0/0);").error.is_none());
        // Expression form gates function bodies.
        let out = run("function f(x) = assert(x > 0, \"need positive\") x; y = f(-1);");
        assert!(out.error.as_deref().unwrap_or("").contains("need positive"));
    }

    #[test]
    fn str_uses_the_shared_six_digit_formatter() {
        assert_eq!(s("str()"), "");
        assert_eq!(s("str(1/3)"), "0.333333");
        assert_eq!(s("str(PI)"), "3.14159");
        assert_eq!(s("str(10.0)"), "10"); // integral: no decimal point
        assert_eq!(s("str(1000000)"), "1e+6"); // scientific from 1e6 up
        assert_eq!(s("str(1234567)"), "1.23457e+6"); // rounded, famously
        assert_eq!(s("str(0.0001)"), "0.0001");
        assert_eq!(s("str(0.00001)"), "0.00001"); // fixed persists to 1e-5
        assert_eq!(ev("str(1.1e-6)").0, Value::Str("1.1e-6".into()));
        assert_eq!(ev("str(1e100)").0, Value::Str("1e+100".into()));
        assert_eq!(s("str(-0)"), "0"); // UNIQUE_ZERO
        assert_eq!(ev("str(1/0)").0, Value::Str("inf".into()));
        assert_eq!(ev("str(0/0)").0, Value::Str("nan".into()));
        // Top-level strings unquoted (concatenation); nested ones quoted.
        assert_eq!(s("str(\"a\", \"b\")"), "ab");
        assert_eq!(s("str([\"a\"])"), "[\"a\"]");
        assert_eq!(s("str(true, undef, [])"), "trueundef[]");
        assert_eq!(s("str([0:10])"), "[0 : 1 : 10]");
    }

    #[test]
    fn chr_and_ord_speak_unicode() {
        assert_eq!(s("chr(65)"), "A");
        assert_eq!(s("chr(65, 97)"), "Aa");
        assert_eq!(s("chr([104, 105])"), "hi");
        assert_eq!(s("chr([65:70])"), "ABCDEF");
        assert_eq!(s("chr(0)"), ""); // invalid code points contribute nothing
        assert_eq!(s("chr(-5)"), "");
        assert_eq!(s("chr(65.7)"), "A"); // truncation toward zero
        assert_eq!(s("chr()"), "");
        assert_eq!(n("len(chr(128169))"), 1.0); // astral plane: ONE char
        assert_eq!(n("ord(\"a\")"), 97.0);
        assert_eq!(n("ord(\"abc\")"), 97.0); // first code point only
        assert_eq!(ev("ord(\"\")").0, Value::Undef);
        assert_eq!(ev("ord(5)").0, Value::Undef);
        assert_eq!(n("ord(chr(128522))"), 128522.0); // roundtrip law
    }

    #[test]
    fn search_has_the_reference_result_shapes() {
        assert_eq!(ev("search(\"a\", \"abcdabcd\")").0, ev("[0]").0);
        assert_eq!(ev("search(\"ab\", \"abcdabcd\")").0, ev("[0, 1]").0);
        assert_eq!(ev("search(\"ba\", \"abcdabcd\")").0, ev("[1, 0]").0); // term order
        assert_eq!(ev("search(\"aa\", \"aa\")").0, ev("[0, 0]").0);
        assert_eq!(ev("search(\"a\", \"abcdabcd\", 0)").0, ev("[[0, 4]]").0);
        // Unmatched terms are SKIPPED from the flat default-mode output.
        let (v, w) = ev("search(\"e\", \"abcdabcd\")");
        assert_eq!(v, Value::Vector(vec![]));
        assert!(w.iter().any(|m| m.contains("not found")));
        // Modes 0/N keep an [] slot instead.
        assert_eq!(ev("search(\"abe\", \"abcd\", 0)").0, ev("[[0], [1], []]").0);
        // Whole-element search wraps the term.
        assert_eq!(ev("search([\"b\"], [\"a\", \"b\", \"c\"])").0, ev("[1]").0);
        // Table search compares the index column.
        assert_eq!(
            ev("search(\"b\", [[\"a\", 1], [\"b\", 2], [\"b\", 3]], 0)").0,
            ev("[[1, 2]]").0
        );
        assert_eq!(ev("search(2, [[\"a\", 1], [\"b\", 2]], 0, 1)").0, ev("[[1]]").0);
        // N>1 caps the per-term match count.
        assert_eq!(ev("search(\"a\", \"aaaa\", 3)").0, ev("[[0, 1, 2]]").0);
    }

    #[test]
    fn lookup_interpolates_and_clamps() {
        assert_eq!(n("lookup(0.5, [[0, 0], [0.5, 1], [1, 0]])"), 1.0); // exact
        assert_eq!(n("lookup(0.25, [[0, 0], [0.5, 1], [1, 0]])"), 0.5); // interp
        assert_eq!(n("lookup(-99, [[0, 7], [1, 9]])"), 7.0); // clamp low
        assert_eq!(n("lookup(99, [[0, 7], [1, 9]])"), 9.0); // clamp high
        assert_eq!(n("lookup(5, [[2, 3]])"), 3.0); // single pair
        let (v, w) = ev("lookup(1, [])");
        assert_eq!(v, Value::Undef);
        assert!(w.iter().any(|m| m.contains("empty")));
    }

    #[test]
    fn rands_version_and_parent_module() {
        // Seeded calls are reproducible; count and bounds behave.
        let out = run("a = rands(0, 1, 3, 42); c = rands(0, 1, 3, 42); echo(a == c, len(a));");
        assert_eq!(out.echoes, vec!["ECHO: true, 3"]);
        assert_eq!(ev("rands(0, 1, 0)").0, Value::Vector(vec![]));
        assert_eq!(ev("rands(7, 7, 2)").0, ev("[7, 7]").0);
        assert!(b("version() == [2021, 1]"));
        assert_eq!(n("version_num()"), 20210100.0);
        // parent_module walks the dynamic instantiation stack.
        let out = run(
            "module inner() echo(parent_module(0), parent_module(1), $parent_modules); \
             module outer() inner(); \
             outer();",
        );
        assert_eq!(out.echoes, vec!["ECHO: \"inner\", \"outer\", 2"]);
    }

    #[test]
    fn dollar_variables_scope_dynamically() {
        // Through function calls.
        let out = run("function probe() = $flag; module m() { $flag = 7; echo(probe()); } m();");
        assert_eq!(out.echoes, vec!["ECHO: 7"]);
        // let($fn) affects tessellation for its dynamic extent.
        let out = run("let ($fn = 6) sphere(10);");
        assert_eq!(out.shapes[0].mesh.positions.len(), 6 * 3);
        // Unset $-variables read undef silently.
        let (v, w) = ev("$never_set");
        assert_eq!(v, Value::Undef);
        assert!(w.is_empty(), "{:?}", w);
        // Unknown PLAIN named args warn; $-named ones never do.
        let out = run("module m(a) cube(a); m(1, $anything = 3);");
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    }
}
