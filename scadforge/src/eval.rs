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

use crate::ast::{Arg, BinOp, Expr, Modifier, Param, Stmt, VecItem};
use crate::csg;
use crate::csg2;
use crate::geom::{self, Mesh};
use crate::io;
use crate::poly2::{self, Poly2};
use crate::value::{FuncVal, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A renderable mesh plus the display color from the NEAREST enclosing
/// color() node (None = uncolored; ancestors must not override).
///
/// `outline` is Some(..) exactly for 2D shapes: it holds the boundary
/// contours at z=0 (the `mesh` is then their triangulated flat fill). 3D
/// shapes leave it None. Extrusion, offset and projection read/produce
/// it; mixing a 2D shape into a 3D boolean is an error.
///
/// The three display flags carry modifier-character state up the tree:
/// `rooted` (`!` — this shape is inside a root-marked subtree; if any exists
/// the design shows only rooted shapes), `highlight` (`#` — draw tinted, but
/// still part of the CSG result), and `background` (`%` — draw as a ghost and
/// EXCLUDE from every boolean and export).
#[derive(Debug, Clone)]
pub struct Shape {
    pub mesh: Mesh,
    pub color: Option<[f64; 4]>,
    pub outline: Option<Poly2>,
    pub rooted: bool,
    pub highlight: bool,
    pub background: bool,
}

impl Shape {
    /// A 2D shape: triangulate the region to a flat z=0 fill mesh and keep
    /// the contours for downstream 2D operations.
    fn flat(poly: Poly2) -> Option<Shape> {
        let (verts, tris) = poly2::triangulate(&poly);
        if tris.is_empty() {
            return None;
        }
        let positions = verts.iter().map(|v| [v[0], v[1], 0.0]).collect();
        Some(Shape {
            mesh: Mesh { positions, tris },
            color: None,
            outline: Some(poly),
            rooted: false,
            highlight: false,
            background: false,
        })
    }
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
    /// The evaluated instantiation tree, recorded only when the run asked
    /// for it (`evaluate_recording`). `None` on a normal render.
    pub csg: Option<crate::csgfmt::CsgNode>,
}

/// Evaluation runs on a dedicated thread with a large stack so deep
/// non-tail recursion hits our own depth guard, not the platform stack.
/// Resolve include/use directives (paths relative to `base_dir`) and then
/// evaluate. Resolution warnings (missing include/use files) are prepended to
/// the diagnostic stream; a parse error in the resolved source is fatal.
pub fn evaluate_source(source: &str, base_dir: &std::path::Path) -> EvalOutput {
    evaluate_source_for(source, base_dir, false)
}

/// `evaluate_source`, optionally recording the instantiation tree for the
/// `.csg` export (`out.csg`).
pub fn evaluate_source_for(
    source: &str,
    base_dir: &std::path::Path,
    record_csg: bool,
) -> EvalOutput {
    let resolved = crate::preproc::resolve(source, base_dir);
    if let Some(err) = resolved.error {
        return EvalOutput { error: Some(err), warnings: resolved.warnings, ..Default::default() };
    }
    let mut out = evaluate_maybe_recording(&resolved.program, record_csg);
    if !resolved.warnings.is_empty() {
        let mut w = resolved.warnings;
        w.append(&mut out.warnings);
        out.warnings = w;
    }
    out
}

pub fn evaluate(program: &[Stmt]) -> EvalOutput {
    evaluate_maybe_recording(program, false)
}

/// Evaluate while recording the instantiation tree for `.csg` export. The
/// geometry kernel still runs (so one code path serves both modes and the
/// tree can never disagree with the render); only the tree is extra.
pub fn evaluate_recording(program: &[Stmt]) -> EvalOutput {
    evaluate_maybe_recording(program, true)
}

fn evaluate_maybe_recording(program: &[Stmt], record: bool) -> EvalOutput {
    std::thread::scope(|s| {
        let handle = std::thread::Builder::new()
            .stack_size(256 * 1024 * 1024)
            .spawn_scoped(s, || evaluate_inner(program, record))
            .expect("failed to spawn the evaluator thread");
        handle.join().unwrap_or_else(|_| EvalOutput {
            error: Some("ERROR: the evaluator crashed (please report this script)".into()),
            ..EvalOutput::default()
        })
    })
}

fn evaluate_inner(program: &[Stmt], record: bool) -> EvalOutput {
    let mut ctx = Ctx {
        out: EvalOutput::default(),
        dynv: DynScope::root(),
        mod_stack: Vec::new(),
        children: None,
        fn_depth: 0,
        cycle_scopes: Vec::new(),
        csg: record.then(|| vec![CsgFrame { head: Some("group()".into()), nodes: Vec::new() }]),
    };
    let root = Scope::root();
    let mut shapes = exec_scope(program, &root, &mut ctx);
    // Root modifier (`!`): if any shape is root-marked, the design shows ONLY
    // root-marked shapes — everything else is pruned (their side effects have
    // already run, and ancestor transforms are already baked into these
    // meshes). Multiple/independent `!` subtrees all survive (union of roots).
    if shapes.iter().any(|s| s.rooted) {
        shapes.retain(|s| s.rooted);
    }
    ctx.out.shapes.extend(shapes);
    // The root frame was never closed (nothing to close it into); take it
    // directly as the tree's `group()` root.
    if let Some(st) = &mut ctx.csg {
        let root_frame = st.remove(0);
        ctx.out.csg = Some(crate::csgfmt::CsgNode {
            head: root_frame.head.unwrap_or_else(|| "group()".into()),
            children: root_frame.nodes,
        });
    }
    // Function values capture their defining scope; storing one in a
    // scope on that capture chain forms an Rc cycle. Clearing the
    // variables of every scope that received a function-containing value
    // breaks all such cycles so the render's memory actually frees.
    for s in ctx.cycle_scopes.drain(..) {
        s.vars.borrow_mut().clear();
    }
    // Stamp the diagnostic class onto each message. Warnings collected through
    // the run carry only their text; the reference surface prefixes every
    // line with its class (WARNING/DEPRECATED/ERROR). DEPRECATED/ERROR lines
    // already carry their prefix, so only unclassified lines become WARNINGs.
    // (File/line suffixes are not emitted — the evaluator does not yet track
    // source positions; a documented partial.)
    for w in &mut ctx.out.warnings {
        if !(w.starts_with("WARNING:")
            || w.starts_with("DEPRECATED:")
            || w.starts_with("ERROR:"))
        {
            *w = format!("WARNING: {}", w);
        }
    }
    ctx.out
}

/// Would storing `v` in `scope` close an Rc cycle? Only if some function
/// inside `v` captured an environment from which `scope` is reachable
/// (Scope → vars → Function → env → … → Scope). A function passed as an
/// argument, whose closure was defined elsewhere, forms no cycle — so we
/// don't retain those scopes (the fold-with-callback O(n) leak).
fn stores_cycle(scope: &Rc<Scope>, v: &Value) -> bool {
    match v {
        Value::Function(fv) => {
            let mut cur = Some(fn_env(fv));
            while let Some(s) = cur {
                if Rc::ptr_eq(&s, scope) {
                    return true;
                }
                cur = s.parent.clone();
            }
            false
        }
        Value::Vector(items) => items.iter().any(|i| stores_cycle(scope, i)),
        _ => false,
    }
}

/// Non-tail function recursion guard (tail calls are eliminated and do
/// not count).
const MAX_FN_DEPTH: usize = 10_000;
/// Module instantiation recursion guard.
const MAX_MODULE_DEPTH: usize = 2_000;
/// Backstop for runaway tail loops / C-style generators, far above the
/// reference's practical contracts.
const MAX_TAIL_ITERS: usize = 50_000_000;
const MAX_GENERATOR_ITERS: usize = 10_000_000;
/// Above the reference's pinned [0:1e6] = 1_000_001-iteration workload,
/// but still a bounded backstop for a public render endpoint.
const MAX_RANGE_ITEMS: usize = 4_000_000;

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
/// current `$`-environment. `outer` is the children context that was in
/// force AT the call site, restored while the block runs — so a
/// children() statement INSIDE the block forwards the caller's own
/// children (the wrapper idiom) instead of re-instantiating itself
/// forever.
struct ChildrenCtx {
    stmts: Rc<Vec<Stmt>>,
    lex: Rc<Scope>,
    outer: Option<Rc<ChildrenCtx>>,
}

/// One open level of the `.csg` recording. `head` is filled in by whoever
/// knows the canonical spelling — usually after argument binding, which
/// happens well inside the call — so it starts empty and is set later. A
/// frame closed with no head splices its children into the parent instead
/// of wrapping them (that is how `children()`, `echo(...) child;`, and
/// unknown modules disappear from the tree without losing their subtree).
struct CsgFrame {
    head: Option<String>,
    nodes: Vec<crate::csgfmt::CsgNode>,
}

struct Ctx {
    out: EvalOutput,
    dynv: Rc<DynScope>,
    /// User-module instantiation stack, innermost last (parent_module).
    mod_stack: Vec<String>,
    children: Option<Rc<ChildrenCtx>>,
    fn_depth: usize,
    /// Scopes holding function-containing values — potential Rc cycles,
    /// broken at the end of evaluation.
    cycle_scopes: Vec<Rc<Scope>>,
    /// `.csg` recording, innermost frame last. `None` during a normal
    /// render: the hooks below all short-circuit, so recording costs
    /// nothing when it is off.
    csg: Option<Vec<CsgFrame>>,
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

    // -- .csg recording -----------------------------------------------
    //
    // Every statement that can produce geometry records EXACTLY ONE node,
    // so the child-grouping structure of the source survives into the
    // export: `difference() { for (…) cube(); sphere(); }` must not become
    // a difference with four operands.

    fn csg_on(&self) -> bool {
        self.csg.is_some()
    }

    /// Open a frame. Pair with exactly one `csg_close`.
    fn csg_open(&mut self) {
        if let Some(st) = &mut self.csg {
            st.push(CsgFrame { head: None, nodes: Vec::new() });
        }
    }

    /// Name the frame currently open. Called once the canonical head is
    /// known; leaving it unset makes the frame splice on close.
    fn csg_head(&mut self, head: String) {
        if let Some(st) = &mut self.csg {
            if let Some(f) = st.last_mut() {
                f.head = Some(head);
            }
        }
    }

    /// Close the open frame: wrap its nodes under its head and append to
    /// the parent. A frame that never got a head becomes a plain `group()`.
    ///
    /// It must WRAP, never splice. The renderer counts one operand per child
    /// statement (`eval_children_grouped` pushes one `Vec<Shape>` each), so
    /// `children()` forwarding two shapes into an `intersection()` is ONE
    /// operand, not two. Splicing them made the export re-import as a
    /// three-operand intersection and silently changed the result. The same
    /// rule keeps a statement that draws nothing — `echo`, an unknown module,
    /// a transform whose subtree was dropped — recording the EMPTY operand
    /// the renderer counted.
    fn csg_close(&mut self) {
        let Some(st) = &mut self.csg else { return };
        let Some(frame) = st.pop() else { return };
        let Some(parent) = st.last_mut() else { return };
        parent.nodes.push(crate::csgfmt::CsgNode {
            head: frame.head.unwrap_or_else(|| "group()".into()),
            children: frame.nodes,
        });
    }

    /// Forget everything recorded in the open frame. Used when a builtin
    /// drops its subtree (a bad transform argument), so the export does not
    /// hoist children the render never drew.
    fn csg_clear(&mut self) {
        if let Some(st) = &mut self.csg {
            if let Some(f) = st.last_mut() {
                f.nodes.clear();
            }
        }
    }

    /// Prefix the most recently recorded node with a modifier character.
    /// `#`, `%` and `!` are all valid OpenSCAD statement prefixes, so the
    /// export stays re-importable and re-imports to the same meaning.
    fn csg_modify(&mut self, ch: char) {
        if let Some(st) = &mut self.csg {
            if let Some(node) = st.last_mut().and_then(|f| f.nodes.last_mut()) {
                node.head.insert(0, ch);
            }
        }
    }

    /// The `$fn`/`$fa`/`$fs` triple in effect right now.
    fn csg_frags(&self) -> crate::csgfmt::Frags {
        let get = |key: &str, default: f64| {
            self.dynv.lookup(key).and_then(|v| v.as_num()).unwrap_or(default)
        };
        crate::csgfmt::Frags {
            fn_: get("$fn", 0.0),
            fa: get("$fa", 12.0),
            fs: get("$fs", 2.0),
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
    if stores_cycle(scope, &v) {
        // Dedup consecutive registrations so a loop rebinding the same
        // scope doesn't grow the list per iteration.
        if ctx.cycle_scopes.last().map_or(true, |s| !Rc::ptr_eq(s, scope)) {
            ctx.cycle_scopes.push(scope.clone());
        }
    }
    scope.vars.borrow_mut().insert(name.to_string(), v);
}

/// Run `body` with one `.csg` frame open under `head`, so everything it
/// instantiates lands inside a single node. A no-op wrapper when recording
/// is off.
fn csg_grouped<T>(ctx: &mut Ctx, head: &str, body: impl FnOnce(&mut Ctx) -> T) -> T {
    ctx.csg_open();
    ctx.csg_head(head.to_string());
    let out = body(ctx);
    ctx.csg_close();
    out
}

fn exec_stmt(stmt: &Stmt, scope: &Rc<Scope>, ctx: &mut Ctx) -> Vec<Shape> {
    match stmt {
        Stmt::Assign { .. } | Stmt::ModuleDef { .. } | Stmt::FunctionDef { .. } => Vec::new(),
        Stmt::Modified { modifier, stmt } => {
            // `*` disables: the subtree is not instantiated at all, so its
            // echo/assert side effects never fire (an early-out, not a
            // post-hoc filter). It still OCCUPIES its operand slot though —
            // the renderer pushes an empty group for the statement — so the
            // export records that empty group rather than nothing.
            if *modifier == Modifier::Disable {
                if ctx.csg_on() {
                    csg_grouped(ctx, "group()", |_| ());
                }
                return Vec::new();
            }
            // The others fully instantiate the subtree (side effects run),
            // then tag the produced shapes so the flag propagates up the tree.
            let mut shapes = exec_stmt(stmt, scope, ctx);
            // The inner statement recorded exactly one node; mark it. `#`,
            // `%` and `!` are valid statement prefixes, so the export stays
            // re-importable and re-imports to the same meaning.
            match modifier {
                Modifier::Root => ctx.csg_modify('!'),
                Modifier::Highlight => ctx.csg_modify('#'),
                Modifier::Background => ctx.csg_modify('%'),
                Modifier::Disable => {}
            }
            match modifier {
                Modifier::Root => {
                    for s in &mut shapes {
                        s.rooted = true;
                    }
                    shapes
                }
                Modifier::Background => {
                    for s in &mut shapes {
                        s.background = true;
                    }
                    shapes
                }
                Modifier::Highlight => {
                    // `#` is a geometric NO-OP: the real geometry stays in the
                    // CSG result unchanged, PLUS a display-only pink ghost
                    // overlay is added — so a #-marked cutter both cuts AND
                    // shows where it cut (the debug idiom). The ghost carries
                    // `background` so it bypasses every boolean.
                    let mut out = Vec::with_capacity(shapes.len() * 2);
                    for s in shapes {
                        let mut ghost = s.clone();
                        ghost.highlight = true;
                        ghost.background = true;
                        out.push(s);
                        out.push(ghost);
                    }
                    out
                }
                Modifier::Disable => unreachable!(),
            }
        }
        // Blocks, ifs, lets and loops each record ONE `group()` node, even
        // when they produce zero or many shapes: a surrounding
        // `difference()` counts them as one operand, and the export has to
        // agree or the re-import subtracts different things.
        Stmt::Block(body) => csg_grouped(ctx, "group()", |ctx| exec_scope(body, scope, ctx)),
        Stmt::If { cond, then, els } => {
            let c = eval_expr(cond, scope, ctx);
            csg_grouped(ctx, "group()", |ctx| {
                if truthy(&c) {
                    exec_scope(then, scope, ctx)
                } else {
                    exec_scope(els, scope, ctx)
                }
            })
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
            let shapes = csg_grouped(ctx, "group()", |ctx| exec_scope(body, &bind_scope, ctx));
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
            // One group for the whole loop, iterations flattened inside it.
            ctx.csg_open();
            ctx.csg_head("group()".into());
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
            ctx.csg_close();
            shapes
        }
        Stmt::IntersectionFor { bindings, body } => {
            // Same header as `for`, but each iteration's geometry is one
            // operand and the operands are intersected (not unioned).
            let iterables: Vec<(String, Vec<Value>)> = bindings
                .iter()
                .map(|(name, expr)| {
                    let v = eval_expr(expr, scope, ctx);
                    (name.clone(), iterate(&v, ctx))
                })
                .collect();
            let mut operands: Vec<Mesh> = Vec::new();
            let mut color = None;
            // `intersection_for` records as a plain `intersection()` over
            // one `group()` per iteration — the resolved form of the loop.
            ctx.csg_open();
            ctx.csg_head("intersection()".into());
            cross_product(&iterables, 0, &mut Vec::new(), &mut |vals, ctx| {
                let saved_dyn = ctx.dynv.clone();
                ctx.dynv = DynScope::layer(&saved_dyn);
                let iter_scope = Scope::child(scope);
                for (name, v) in vals {
                    set_var(&iter_scope, ctx, name, v.clone());
                }
                let shapes = csg_grouped(ctx, "group()", |ctx| exec_scope(body, &iter_scope, ctx));
                if color.is_none() {
                    color = shapes.iter().find_map(|s| s.color);
                }
                operands.push(combine_group(&shapes).0);
                ctx.dynv = saved_dyn;
            }, ctx);
            ctx.csg_close();
            if operands.is_empty() {
                Vec::new()
            } else {
                leaf_colored(csg::intersection_all(&operands), color)
            }
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
                (*end, *start)
            } else {
                (*start, *end)
            };
            // ONE canonical count/iteration routine, shared with the
            // relational comparison via range_count — no ad-hoc epsilon,
            // so [0:0.1:0.3] yields exactly 3 elements, not 4.
            let count = range_count(start, *step, end);
            if !count.is_finite() || count <= 0.0 {
                return items;
            }
            if count > MAX_RANGE_ITEMS as f64 {
                ctx.out.warnings.push(format!(
                    "for: range of {} elements truncated at {}",
                    fmt_num(count),
                    MAX_RANGE_ITEMS
                ));
            }
            let n = (count as usize).min(MAX_RANGE_ITEMS);
            for k in 0..n {
                items.push(Value::Num(start + k as f64 * step));
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

    // One frame per instantiation. Whoever knows the canonical head fills
    // it in; a frame left unnamed (echo, assert, children(), an unknown
    // module) splices its subtree into the parent instead of wrapping it.
    ctx.csg_open();
    let shapes = if let Some((def, def_scope)) = scope.lookup_module(name) {
        call_user_module(name, &def, &def_scope, &ev, children, scope, ctx)
    } else {
        call_builtin_module(name, args, &ev, children, scope, ctx)
    };
    ctx.csg_close();
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
    // A user module has no representation in the evaluated tree: its body
    // is inlined under a plain group().
    ctx.csg_head("group()".into());

    let saved_children = ctx.children.take();
    ctx.children = Some(Rc::new(ChildrenCtx {
        stmts: Rc::new(children.to_vec()),
        lex: caller.clone(),
        outer: saved_children.clone(),
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
    // The block runs in the CALLER's context: restore the children that
    // were in force at the call site so a nested children() forwards
    // them (and a top-level block gets the outside-module warning)
    // instead of recursing into this very block.
    let saved = ctx.children.take();
    ctx.children = cctx.outer.clone();
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
    ctx.children = saved;
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
    // The canonical head for everything whose spelling follows from its
    // bound arguments alone. Transforms, `color` and `resize` set theirs
    // below instead, from the values they actually computed; the modules
    // that record nothing (echo/assert/children/unknown) return None here
    // and leave the frame unnamed.
    if ctx.csg_on() {
        let frags = ctx.csg_frags();
        if let Some(head) = crate::csgfmt::builtin_head(name, &bound, frags) {
            ctx.csg_head(head);
        }
    }

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
        "mirror" => {
            let matrix = match bound.get("v").and_then(Value::as_vec3) {
                Some([0.0, 0.0, 0.0]) => {
                    ctx.out.warnings.push("mirror: v must not be the zero vector".into());
                    Some(geom::identity()) // pass children through
                }
                Some(v) => Some(geom::mirror(v)),
                None => {
                    ctx.out.warnings.push("mirror: v must be a vector like [x, y, z]".into());
                    None
                }
            };
            transform_children(children, scope, ctx, matrix)
        }
        "multmatrix" => {
            let matrix = match bound.get("m") {
                Some(Value::Vector(rows)) => match matrix_rows_f64(rows) {
                    Some(rows) => {
                        let m = geom::matrix_from_rows(&rows);
                        // Non-finite matrices drop the subtree (reference).
                        if m.iter().flatten().all(|v| v.is_finite()) {
                            Some(m)
                        } else {
                            ctx.out.warnings.push(
                                "multmatrix: matrix contains NaN/Infinity — subtree removed"
                                    .into(),
                            );
                            None
                        }
                    }
                    None => {
                        ctx.out
                            .warnings
                            .push("multmatrix: m must be a list of numeric rows".into());
                        None
                    }
                },
                _ => {
                    ctx.out.warnings.push("multmatrix: m must be a 4x4 (or 3x4) matrix".into());
                    None
                }
            };
            transform_children(children, scope, ctx, matrix)
        }
        "resize" => {
            if ctx.csg_on() {
                let ns = bound.get("newsize").and_then(Value::as_vec3).unwrap_or([0.0; 3]);
                ctx.csg_head(crate::csgfmt::resize_head(ns, resize_auto_flags(bound.get("auto"))));
            }
            let mut shapes = exec_scope(children, scope, ctx);
            let newsize = bound.get("newsize").and_then(Value::as_vec3);
            match newsize {
                Some(newsize) => {
                    if let Some(m) = resize_matrix(&shapes, newsize, bound.get("auto")) {
                        for s in &mut shapes {
                            geom::apply(&m, &mut s.mesh);
                        }
                    }
                    shapes
                }
                None => {
                    ctx.out
                        .warnings
                        .push("resize: newsize must be a vector like [x, y, z]".into());
                    shapes
                }
            }
        }
        "polyhedron" => {
            let points = bound.get("points").and_then(vec3_list);
            let faces = bound.get("faces").and_then(index_lists);
            match (points, faces) {
                (Some(points), Some(faces)) => {
                    let (mesh, warnings) = geom::polyhedron(&points, &faces);
                    ctx.out.warnings.extend(warnings);
                    no_children(name, children, ctx);
                    leaf(mesh)
                }
                _ => {
                    ctx.out
                        .warnings
                        .push("polyhedron: expected points=[[x,y,z],...] and faces=[[i,...],...]".into());
                    Vec::new()
                }
            }
        }
        "square" => {
            let size = match bound.get("size") {
                Some(Value::Num(s)) => [*s, *s],
                Some(Value::Vector(items)) if items.len() == 2 => {
                    match (items[0].as_num(), items[1].as_num()) {
                        (Some(x), Some(y)) => [x, y],
                        _ => {
                            ctx.out.warnings.push("square: size must be numeric".into());
                            return Vec::new();
                        }
                    }
                }
                Some(Value::Undef) | None => [1.0, 1.0],
                _ => {
                    ctx.out.warnings.push("square: size must be a number or [x, y]".into());
                    return Vec::new();
                }
            };
            let center = bound.get("center").and_then(Value::as_bool).unwrap_or(false);
            no_children(name, children, ctx);
            Shape::flat(poly2::square(size, center)).into_iter().collect()
        }
        "circle" => {
            let r = match (bound.get("d"), bound.get("r")) {
                (Some(Value::Num(d)), _) => d / 2.0,
                (_, Some(Value::Num(r))) => *r,
                _ => 1.0,
            };
            let n = resolve_fragments(r, ctx);
            no_children(name, children, ctx);
            Shape::flat(poly2::circle(r, n)).into_iter().collect()
        }
        "polygon" => {
            let points = bound.get("points").and_then(vec2_list);
            let paths = match bound.get("paths") {
                Some(Value::Undef) | None => None,
                Some(v) => match index_lists(v) {
                    Some(p) => Some(p),
                    None => {
                        ctx.out.warnings.push("polygon: paths must be a list of index lists".into());
                        return Vec::new();
                    }
                },
            };
            match points {
                Some(points) => {
                    let (poly, warnings) = poly2::polygon(&points, paths.as_deref());
                    ctx.out.warnings.extend(warnings);
                    no_children(name, children, ctx);
                    Shape::flat(poly).into_iter().collect()
                }
                None => {
                    ctx.out.warnings.push("polygon: points must be a list of [x, y]".into());
                    Vec::new()
                }
            }
        }
        "linear_extrude" => {
            let poly = collect_2d(children, scope, ctx);
            // Default height is 100; non-finite/non-numeric keeps the default.
            let height = bound
                .get("height")
                .and_then(Value::as_num)
                .filter(|h| h.is_finite())
                .unwrap_or(100.0);
            if height <= 0.0 || poly.is_empty() {
                return Vec::new(); // height ≤ 0 clamps to empty, silently
            }
            let center = bound.get("center").and_then(Value::as_bool).unwrap_or(false);
            let twist = bound.get("twist").and_then(Value::as_num).unwrap_or(0.0);
            let scale = match bound.get("scale") {
                Some(Value::Num(s)) => [*s, *s],
                Some(v @ Value::Vector(_)) => {
                    v.as_vec3().map(|a| [a[0], a[1]]).unwrap_or([1.0, 1.0])
                }
                _ => [1.0, 1.0],
            };
            let slices = match bound.get("slices").and_then(Value::as_num) {
                Some(s) if s >= 1.0 => s.trunc() as usize,
                _ if twist != 0.0 => {
                    let fa = ctx.dynv.lookup("$fa").and_then(|v| v.as_num()).unwrap_or(12.0);
                    (twist.abs() / fa.max(1.0)).ceil().max(1.0) as usize
                }
                _ => 1,
            };
            // Record the slice count the sweep ACTUALLY used. With `twist`
            // set and `slices` omitted it comes from $fa, and printing the
            // literal argument (or 1) exported a flat prism instead of the
            // twisted sweep.
            if ctx.csg_on() {
                let frags = ctx.csg_frags();
                ctx.csg_head(crate::csgfmt::linear_extrude_head(
                    height, center,
                    bound.get("convexity").and_then(Value::as_num).unwrap_or(1.0),
                    twist, slices as f64, scale, frags,
                ));
            }
            let (positions, tris) =
                poly2::extrude_linear(&poly, height, center, twist, slices, scale);
            leaf(Mesh { positions, tris })
        }
        "rotate_extrude" => {
            let poly = collect_2d(children, scope, ctx);
            if poly.is_empty() {
                return Vec::new();
            }
            let angle = bound.get("angle").and_then(Value::as_num).unwrap_or(360.0);
            if angle == 0.0 {
                return Vec::new();
            }
            // Fragments from the largest profile radius, scaled by the sweep.
            let max_r = poly
                .contours
                .iter()
                .flatten()
                .map(|p| p[0].abs())
                .fold(0.0, f64::max);
            let base = resolve_fragments(max_r, ctx) as f64;
            let frags = ((base * angle.abs() / 360.0).trunc() as usize).max(1);
            match poly2::extrude_rotate(&poly, angle, frags) {
                Ok((positions, tris)) => leaf(Mesh { positions, tris }),
                Err(e) => {
                    ctx.out.warnings.push(format!("ERROR: {}", e));
                    Vec::new()
                }
            }
        }
        "offset" => {
            // The union of all 2D children is offset once (collect_2d unions;
            // 3D children warn and are skipped).
            let poly = collect_2d(children, scope, ctx);
            if poly.is_empty() {
                return Vec::new();
            }
            let nverts: usize = poly.contours.iter().map(|c| c.len()).sum();
            if nverts > csg2::OFFSET_MAX_VERTS {
                ctx.out.warnings.push(format!(
                    "offset(): {} outline vertices exceed the preview cap ({}); reduce $fn \
                     on the children",
                    nverts,
                    csg2::OFFSET_MAX_VERTS
                ));
                return Vec::new();
            }
            let r = bound.get("r").and_then(Value::as_num).filter(|v| v.is_finite());
            let delta = bound.get("delta").and_then(Value::as_num).filter(|v| v.is_finite());
            let chamfer = bound.get("chamfer").and_then(Value::as_bool).unwrap_or(false);
            // r wins over delta; with neither, the reference default is a
            // 1-unit round-join offset (offset() ≡ offset(r = 1)).
            let (dist, join) = match (r, delta) {
                (Some(r), _) => (r, csg2::Join::Round),
                (None, Some(d)) => (d, if chamfer { csg2::Join::Chamfer } else { csg2::Join::Miter }),
                (None, None) => (1.0, csg2::Join::Round),
            };
            let frags_full = resolve_fragments(dist.abs(), ctx);
            let result = csg2::offset2(&poly, dist, join, frags_full);
            // A negative offset can annihilate the region — empty, no warning.
            Shape::flat(result).into_iter().collect()
        }
        "import" | "import_stl" | "import_off" | "import_dxf" => {
            if name != "import" {
                ctx.out.warnings.push(format!(
                    "DEPRECATED: The {}() module will be removed in future releases. \
                     Use import() instead.",
                    name
                ));
            }
            no_children(name, children, ctx);
            let path = match bound.get("file").or_else(|| bound.get("filename")) {
                Some(Value::Str(s)) => s.clone(),
                _ => {
                    ctx.out.warnings.push("import(): expected a file name string".into());
                    return Vec::new();
                }
            };
            // dpi (SVG only) scales px/unitless user units; default 96.
            let dpi = bound.get("dpi").and_then(Value::as_num).unwrap_or(96.0);
            if ctx.csg_on() {
                let frags = ctx.csg_frags();
                ctx.csg_head(crate::csgfmt::import_head(
                    &path,
                    bound.get("layer"),
                    bound.get("convexity").and_then(Value::as_num).unwrap_or(1.0),
                    dpi,
                    frags,
                ));
            }
            import_file(&path, dpi, ctx)
        }
        "projection" => {
            let groups = eval_children_grouped(children, scope, ctx);
            if groups.is_empty() {
                return Vec::new();
            }
            let cut = bound.get("cut").and_then(Value::as_bool).unwrap_or(false);
            // 2021.01 warns on and skips 2D children (the reverse of the 2D
            // arms' 3D-child skip); only 3D geometry projects.
            let mut combined = Mesh::empty();
            let mut saw_2d = false;
            for s in groups.iter().flatten() {
                if s.outline.is_some() {
                    saw_2d = true;
                } else if !s.mesh.positions.is_empty() {
                    // Concatenate facets (an exact 3D union is unnecessary — the
                    // silhouette re-unions them and the cut collects all
                    // crossings); implicit-union-before-projection is preserved
                    // for the common single/nested-solid cases.
                    let base = combined.positions.len() as u32;
                    combined.positions.extend_from_slice(&s.mesh.positions);
                    for t in &s.mesh.tris {
                        combined.tris.push([t[0] + base, t[1] + base, t[2] + base]);
                    }
                }
            }
            if saw_2d {
                ctx.out
                    .warnings
                    .push("Ignoring 2D child object for 3D operation".into());
            }
            if combined.tris.is_empty() {
                return Vec::new();
            }
            // Both modes fold per-facet geometry through the 2D kernel, so both
            // are capped (cut is cheaper, but a huge straddling mesh still
            // stitches an unbounded number of segments).
            if combined.tris.len() > csg2::PROJECT_MAX_TRIS {
                ctx.out.warnings.push(format!(
                    "projection(): {} facets exceed the preview cap ({}); reduce $fn on the \
                     children",
                    combined.tris.len(),
                    csg2::PROJECT_MAX_TRIS
                ));
                return Vec::new();
            }
            Shape::flat(csg2::project(&combined, cut)).into_iter().collect()
        }
        "color" => {
            let rgba = parse_color(bound.get("c"), bound.get("alpha"), ctx);
            if ctx.csg_on() {
                ctx.csg_head(crate::csgfmt::color_head(rgba));
            }
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
        // union/group stay preview concatenation: the reference's F5
        // preview does not compute the exact union either, and it is
        // visually identical for non-overlapping or opaque solids.
        "union" | "group" => exec_scope(children, scope, ctx),
        "render" => {
            // Identity that FORCES exact evaluation: children are implicitly
            // unioned and baked into one opaque leaf. We are always exact, so
            // this just computes the real union (unlike union/group's preview
            // concatenation). `!`/`%`/`#`-ghost children pass through.
            let shapes = exec_scope(children, scope, ctx);
            let (combinable, pass): (Vec<Shape>, Vec<Shape>) =
                shapes.into_iter().partition(|s| !s.rooted && !s.background);
            let any_2d_child = combinable.iter().any(|s| s.outline.is_some());
            let all_2d_child = combinable
                .iter()
                .all(|s| s.outline.is_some() || s.mesh.positions.is_empty());
            let color = combinable.iter().find_map(|s| s.color);
            let mut out = if combinable.is_empty() {
                Vec::new()
            } else if any_2d_child && all_2d_child {
                let regions: Vec<Poly2> =
                    combinable.iter().filter_map(|s| s.outline.clone()).collect();
                let mut sh =
                    Shape::flat(csg2::union2(&regions)).into_iter().collect::<Vec<_>>();
                for s in &mut sh {
                    s.color = color;
                }
                sh
            } else {
                let meshes: Vec<Mesh> = combinable.into_iter().map(|s| s.mesh).collect();
                leaf_colored(csg::union_all(&meshes), color)
            };
            out.extend(pass);
            out
        }
        "surface" => {
            no_children(name, children, ctx);
            let path = match bound.get("file").or_else(|| bound.get("filename")) {
                Some(Value::Str(s)) => s.clone(),
                _ => {
                    ctx.out.warnings.push("WARNING: surface(): expected a file name string".into());
                    return Vec::new();
                }
            };
            let text = match sandboxed_path(&path).and_then(|p| std::fs::read_to_string(p).ok()) {
                Some(t) => t,
                None => {
                    ctx.out.warnings.push(format!("WARNING: Can't open import file '{}'.", path));
                    return Vec::new();
                }
            };
            let center = bound.get("center").and_then(Value::as_bool).unwrap_or(false);
            let invert = bound.get("invert").and_then(Value::as_bool).unwrap_or(false);
            if ctx.csg_on() {
                ctx.csg_head(crate::csgfmt::surface_head(
                    &path, center, invert,
                    bound.get("convexity").and_then(Value::as_num).unwrap_or(1.0),
                ));
            }
            let grid = io::parse_surface_text(&text);
            leaf(io::heightmap_solid(&grid, center, invert))
        }
        "text" => {
            no_children(name, children, ctx);
            // A non-string text argument yields empty geometry (the reference
            // does NOT auto-str() it); an empty string is silently empty.
            let s = match bound.get("text") {
                Some(Value::Str(s)) => s.clone(),
                Some(Value::Undef) | None => return Vec::new(),
                Some(_) => {
                    ctx.out
                        .warnings
                        .push("WARNING: text(): text argument must be a string".into());
                    return Vec::new();
                }
            };
            let size = bound.get("size").and_then(Value::as_num).filter(|v| *v > 0.0).unwrap_or(10.0);
            let spacing = bound.get("spacing").and_then(Value::as_num).unwrap_or(1.0);
            let as_string = |v: Option<&Value>| match v {
                Some(Value::Str(s)) => Some(s.clone()),
                _ => None,
            };
            let halign = as_string(bound.get("halign"));
            let valign = as_string(bound.get("valign"));
            if let Some(h) = &halign {
                if !matches!(h.as_str(), "left" | "center" | "right") {
                    ctx.out.warnings.push(format!(
                        "WARNING: text(): unknown halign '{}'; using \"left\".",
                        h
                    ));
                }
            }
            if let Some(v) = &valign {
                if !matches!(v.as_str(), "baseline" | "top" | "center" | "bottom") {
                    ctx.out.warnings.push(format!(
                        "WARNING: text(): unknown valign '{}'; using \"baseline\".",
                        v
                    ));
                }
            }
            // Curve flattening follows $fn/$fa/$fs (on the size as the radius).
            let steps = (resolve_fragments(size, ctx) as usize / 8).clamp(2, 24);
            let region = text_region(&s, size, spacing, halign.as_deref(), valign.as_deref(), steps);
            Shape::flat(region).into_iter().collect()
        }
        "dxf_linear_extrude" | "dxf_rotate_extrude" => {
            // Deprecated aliases: extrude a DXF file loaded via file=/layer=.
            let modern = if name == "dxf_linear_extrude" { "linear_extrude" } else { "rotate_extrude" };
            ctx.out.warnings.push(format!(
                "DEPRECATED: The {}() module will be removed in future releases. Use {}() instead.",
                name, modern
            ));
            no_children(name, children, ctx);
            let path = match bound.get("file").or_else(|| bound.get("filename")) {
                Some(Value::Str(s)) => s.clone(),
                _ => return Vec::new(),
            };
            let poly = import_file(&path, 96.0, ctx).into_iter().find_map(|s| s.outline);
            let poly = match poly {
                Some(p) if !p.is_empty() => p,
                _ => return Vec::new(),
            };
            if name == "dxf_linear_extrude" {
                let height = bound
                    .get("height")
                    .and_then(Value::as_num)
                    .filter(|h| h.is_finite())
                    .unwrap_or(100.0);
                if height <= 0.0 {
                    return Vec::new();
                }
                let center = bound.get("center").and_then(Value::as_bool).unwrap_or(false);
                let twist = bound.get("twist").and_then(Value::as_num).unwrap_or(0.0);
                let (positions, tris) =
                    poly2::extrude_linear(&poly, height, center, twist, 1, [1.0, 1.0]);
                leaf(Mesh { positions, tris })
            } else {
                let max_r = poly.contours.iter().flatten().map(|p| p[0].abs()).fold(0.0, f64::max);
                let frags = resolve_fragments(max_r, ctx).max(1) as usize;
                match poly2::extrude_rotate(&poly, 360.0, frags) {
                    Ok((positions, tris)) => leaf(Mesh { positions, tris }),
                    Err(e) => {
                        ctx.out.warnings.push(format!("ERROR: {}", e));
                        Vec::new()
                    }
                }
            }
        }
        "difference" => {
            let mut groups = eval_children_grouped(children, scope, ctx);
            if groups.is_empty() {
                return Vec::new();
            }
            // `!`/`%` children bypass the boolean (and can promote the minuend).
            let pass = extract_passthrough(&mut groups);
            let mut out = if groups.is_empty() {
                Vec::new() // every child was passthrough
            } else if all_2d(&groups) {
                let color = groups[0].iter().find_map(|s| s.color);
                let first = group_region(&groups[0]);
                let rest: Vec<Poly2> = groups[1..].iter().map(|g| group_region(g)).collect();
                let mut shapes =
                    Shape::flat(csg2::difference2(&first, &rest)).into_iter().collect::<Vec<_>>();
                for s in &mut shapes {
                    s.color = color;
                }
                shapes
            } else if any_2d(&groups) {
                ctx.out.warnings.push(
                    "difference(): mixing 2D and 3D children is unsupported — shown un-combined"
                        .into(),
                );
                groups.into_iter().flatten().collect()
            } else {
                let (minuend, color) = combine_group(&groups[0]);
                // Empty minuend → empty result, regardless of later children.
                if minuend.positions.is_empty() {
                    Vec::new()
                } else {
                    let cutters: Vec<Mesh> =
                        groups[1..].iter().map(|g| combine_group(g).0).collect();
                    leaf_colored(csg::difference(&minuend, &cutters), color)
                }
            };
            out.extend(pass);
            out
        }
        "intersection" => {
            let mut groups = eval_children_grouped(children, scope, ctx);
            if groups.is_empty() {
                return Vec::new();
            }
            // `!`/`%` children bypass the intersection (a `%` child can't
            // annihilate it); they pass through for display.
            let pass = extract_passthrough(&mut groups);
            let mut out = if groups.is_empty() {
                Vec::new()
            } else if all_2d(&groups) {
                let color = groups[0].iter().find_map(|s| s.color);
                let regions: Vec<Poly2> = groups.iter().map(|g| group_region(g)).collect();
                let mut shapes =
                    Shape::flat(csg2::intersection2(&regions)).into_iter().collect::<Vec<_>>();
                for s in &mut shapes {
                    s.color = color;
                }
                shapes
            } else if any_2d(&groups) {
                ctx.out.warnings.push(
                    "intersection(): mixing 2D and 3D children is unsupported — shown un-combined"
                        .into(),
                );
                groups.into_iter().flatten().collect()
            } else {
                let color = groups[0].iter().find_map(|s| s.color);
                let meshes: Vec<Mesh> = groups.iter().map(|g| combine_group(g).0).collect();
                leaf_colored(csg::intersection_all(&meshes), color)
            };
            out.extend(pass);
            out
        }
        "hull" => {
            let groups = eval_children_grouped(children, scope, ctx);
            if groups.is_empty() {
                return Vec::new();
            }
            if all_2d(&groups) {
                // 2D hull of every child's outline points (colors dropped,
                // per hull's reference semantics).
                let regions: Vec<Poly2> =
                    groups.iter().flatten().filter_map(|s| s.outline.clone()).collect();
                return Shape::flat(csg2::hull2(&regions)).into_iter().collect();
            }
            if any_2d(&groups) {
                ctx.out.warnings.push(
                    "hull(): mixing 2D and 3D children is unsupported — shown un-combined".into(),
                );
                return groups.into_iter().flatten().collect();
            }
            let meshes: Vec<Mesh> = groups.into_iter().flatten().map(|s| s.mesh).collect();
            let n: usize = meshes.iter().map(|m| m.positions.len()).sum();
            if n > csg::HULL_MAX_POINTS {
                ctx.out.warnings.push(format!(
                    "hull(): {} vertices exceed the preview cap ({}); reduce $fn on the \
                     children",
                    n,
                    csg::HULL_MAX_POINTS
                ));
                return Vec::new();
            }
            // Children colors are DROPPED (reference hull EDGE[6]); leaving
            // the result uncolored lets an enclosing color() apply.
            leaf(csg::hull(&meshes))
        }
        "minkowski" => {
            let groups = eval_children_grouped(children, scope, ctx);
            if groups.is_empty() {
                return Vec::new();
            }
            if all_2d(&groups) {
                let regions: Vec<Poly2> =
                    groups.iter().flatten().filter_map(|s| s.outline.clone()).collect();
                let nonempty = regions.iter().filter(|r| !r.is_empty()).count();
                if nonempty >= 2 {
                    ctx.out.warnings.push(
                        "minkowski(): result is exact for convex operands; concave operands \
                         are approximated by their convex sum"
                            .into(),
                    );
                }
                return match csg2::minkowski2(&regions) {
                    csg2::Minkowski2::Ok(poly) => Shape::flat(poly).into_iter().collect(),
                    csg2::Minkowski2::TooLarge { count, partial } => {
                        ctx.out.warnings.push(format!(
                            "minkowski(): {} pairwise points exceed the preview cap ({}); \
                             reduce $fn on the operands — showing the partial fold",
                            count,
                            csg2::MINKOWSKI2_MAX_POINTS
                        ));
                        Shape::flat(partial).into_iter().collect()
                    }
                };
            }
            if any_2d(&groups) {
                ctx.out.warnings.push(
                    "minkowski(): mixing 2D and 3D children is unsupported — shown un-combined"
                        .into(),
                );
                return groups.into_iter().flatten().collect();
            }
            let meshes: Vec<Mesh> = groups.iter().map(|g| combine_group(g).0).collect();
            let nonempty = meshes.iter().filter(|m| !m.positions.is_empty()).count();
            if nonempty >= 2 {
                ctx.out.warnings.push(
                    "minkowski(): result is exact for convex operands; concave operands \
                     are approximated by their convex sum"
                        .into(),
                );
            }
            // Children colors are DROPPED (reference minkowski EDGE[9]).
            match csg::minkowski(&meshes) {
                csg::Minkowski::Ok(mesh) => leaf(mesh),
                csg::Minkowski::TooLarge { count, partial } => {
                    ctx.out.warnings.push(format!(
                        "minkowski(): {} pairwise points exceed the preview cap ({}); \
                         reduce $fn on the operands — showing the partial fold",
                        count,
                        csg::MINKOWSKI_MAX_POINTS
                    ));
                    leaf(partial)
                }
            }
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
        vec![Shape {
            mesh,
            color: None,
            outline: None,
            rooted: false,
            highlight: false,
            background: false,
        }]
    }
}

fn leaf_colored(mesh: Mesh, color: Option<[f64; 4]>) -> Vec<Shape> {
    if mesh.positions.is_empty() {
        Vec::new()
    } else {
        vec![Shape {
            mesh,
            color,
            outline: None,
            rooted: false,
            highlight: false,
            background: false,
        }]
    }
}

/// Reduce one CSG child (a group of shapes) to a single operand mesh plus
/// the color that should represent it: the nearest color set within the
/// child. A child that is itself several shapes is unioned so the operand
/// is one clean solid.
fn combine_group(shapes: &[Shape]) -> (Mesh, Option<[f64; 4]>) {
    let color = shapes.iter().find_map(|s| s.color);
    let meshes: Vec<Mesh> = shapes.iter().map(|s| s.mesh.clone()).collect();
    let mesh = match meshes.len() {
        0 => Mesh::empty(),
        1 => meshes.into_iter().next().unwrap(),
        _ => csg::union_all(&meshes),
    };
    (mesh, color)
}

/// Evaluate a call's children as one lexical scope, but keep each
/// geometry statement's shapes in its OWN group — CSG operators need to
/// tell the first child (the minuend) from the rest.
fn eval_children_grouped(children: &[Stmt], parent: &Rc<Scope>, ctx: &mut Ctx) -> Vec<Vec<Shape>> {
    let scope = build_scope(children, parent, ctx);
    let saved_dyn = ctx.dynv.clone();
    ctx.dynv = DynScope::layer(&saved_dyn);
    eval_slots(children, &scope, ctx);
    let mut groups = Vec::new();
    for stmt in children {
        if ctx.halted() {
            break;
        }
        match stmt {
            Stmt::Assign { .. } | Stmt::ModuleDef { .. } | Stmt::FunctionDef { .. } => {}
            _ => groups.push(exec_stmt(stmt, &scope, ctx)),
        }
    }
    ctx.dynv = saved_dyn;
    groups
}

/// A matrix's rows as f64 (each row a Value::Vector of numbers).
fn matrix_rows_f64(rows: &[Value]) -> Option<Vec<Vec<f64>>> {
    rows.iter()
        .map(|r| match r {
            Value::Vector(cells) => cells.iter().map(Value::as_num).collect::<Option<Vec<f64>>>(),
            _ => None,
        })
        .collect()
}

/// A list of 3-vectors (polyhedron points); shorter vectors zero-fill.
fn vec3_list(v: &Value) -> Option<Vec<geom::Vec3>> {
    match v {
        Value::Vector(items) => items.iter().map(|p| p.as_vec3()).collect(),
        _ => None,
    }
}

/// A list of 2-vectors (polygon points).
fn vec2_list(v: &Value) -> Option<Vec<poly2::Vec2>> {
    match v {
        Value::Vector(items) => items
            .iter()
            .map(|p| match p {
                Value::Vector(c) if c.len() == 2 => match (c[0].as_num(), c[1].as_num()) {
                    (Some(x), Some(y)) => Some([x, y]),
                    _ => None,
                },
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// A list of index lists (polyhedron faces). Invalid indices (negative,
/// non-finite, non-numeric) become usize::MAX so polyhedron's bounds
/// check drops just that face; a non-vector face becomes empty (dropped).
fn index_lists(v: &Value) -> Option<Vec<Vec<usize>>> {
    match v {
        Value::Vector(faces) => Some(
            faces
                .iter()
                .map(|f| match f {
                    Value::Vector(idxs) => idxs
                        .iter()
                        .map(|i| match i.as_num() {
                            Some(n) if n.is_finite() && n >= 0.0 => n.trunc() as usize,
                            _ => usize::MAX,
                        })
                        .collect(),
                    _ => Vec::new(),
                })
                .collect(),
        ),
        _ => None,
    }
}

/// resize's per-axis `auto` flags: a bool applies to all three axes, a
/// vector gives them per-axis.
fn resize_auto_flags(auto: Option<&Value>) -> [bool; 3] {
    match auto {
        Some(Value::Bool(b)) => [*b; 3],
        Some(Value::Vector(items)) => {
            let mut f = [false; 3];
            for (k, it) in items.iter().take(3).enumerate() {
                f[k] = matches!(it, Value::Bool(true));
            }
            f
        }
        _ => [false; 3],
    }
}

/// resize(newsize, auto): measure the evaluated child bounding box, derive
/// per-axis factors (newsize/bbox where newsize != 0 and the box is
/// non-degenerate), fill 1 elsewhere, and share the first specified factor
/// into auto axes. The result is a plain scale about the origin.
fn resize_matrix(shapes: &[Shape], newsize: [f64; 3], auto: Option<&Value>) -> Option<geom::Mat4> {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for s in shapes {
        if let Some((l, h)) = geom::bounds(&s.mesh) {
            for k in 0..3 {
                lo[k] = lo[k].min(l[k]);
                hi[k] = hi[k].max(h[k]);
            }
        }
    }
    if !lo[0].is_finite() {
        return None; // no geometry to measure
    }
    let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    let mut factor = [1.0f64; 3];
    for k in 0..3 {
        // A zero-size axis is left unchanged (guarded division, no inf).
        if newsize[k] != 0.0 && size[k] > 0.0 {
            factor[k] = newsize[k] / size[k];
        }
    }
    let auto3 = resize_auto_flags(auto);
    let specified: Vec<usize> = (0..3).filter(|&k| newsize[k] != 0.0 && size[k] > 0.0).collect();
    // Auto axes share the first explicitly-specified factor (a documented
    // choice for the reference's unpinned multi-axis auto rule).
    if let Some(&first) = specified.first() {
        for k in 0..3 {
            if auto3[k] && !specified.contains(&k) {
                factor[k] = factor[first];
            }
        }
    }
    Some(geom::scaling(factor))
}

/// True if any child group holds a 2D shape — flags a 2D/3D mix in the CSG
/// arms so it can warn rather than combine incompatible geometry.
fn any_2d(groups: &[Vec<Shape>]) -> bool {
    groups.iter().flatten().any(|s| s.outline.is_some())
}

/// True if EVERY non-empty child shape is 2D — the CSG arms then run the 2D
/// kernel instead of the 3D mesh kernel. An all-empty tree counts as 2D
/// (nothing to combine either way).
fn all_2d(groups: &[Vec<Shape>]) -> bool {
    groups
        .iter()
        .flatten()
        .all(|s| s.outline.is_some() || s.mesh.positions.is_empty())
}

/// The unioned 2D region of one child group (each shape contributes its
/// outline; overlaps merge). A group with no 2D content yields an empty
/// region.
fn group_region(shapes: &[Shape]) -> Poly2 {
    let regions: Vec<Poly2> = shapes.iter().filter_map(|s| s.outline.clone()).collect();
    csg2::union2(&regions)
}

/// Pull the shapes that must bypass a boolean out of its child groups:
/// rooted (`!`) and background (`%`) shapes never participate in the CSG —
/// they pass through for display only (a `%` cutter ghosts without cutting;
/// a `!` child shows raw, the boolean discarded). A group emptied *by this
/// removal* is dropped so the next sibling promotes (the `%`/`!`-first-child
/// promotion), while a group that was already empty (an `if(false)` branch)
/// is kept — an empty minuend still yields an empty difference.
fn extract_passthrough(groups: &mut Vec<Vec<Shape>>) -> Vec<Shape> {
    let mut pass = Vec::new();
    let mut i = 0;
    while i < groups.len() {
        let before = groups[i].len();
        let mut keep = Vec::with_capacity(before);
        for s in groups[i].drain(..) {
            if s.rooted || s.background {
                pass.push(s);
            } else {
                keep.push(s);
            }
        }
        let removed = before - keep.len();
        groups[i] = keep;
        if groups[i].is_empty() && removed > 0 {
            groups.remove(i);
        } else {
            i += 1;
        }
    }
    pass
}

/// Collect the child 2D geometry for an extrusion into one region: the true
/// 2D UNION of every 2D child (overlapping children merge, so an extrusion
/// of two crossing squares is a plus, not a plus with a hole in the
/// overlap). 3D children are skipped with a warning.
fn collect_2d(children: &[Stmt], scope: &Rc<Scope>, ctx: &mut Ctx) -> Poly2 {
    let shapes = exec_scope(children, scope, ctx);
    let mut regions = Vec::new();
    let mut saw_3d = false;
    for s in shapes {
        match s.outline {
            Some(poly) => regions.push(poly),
            None => {
                if !s.mesh.positions.is_empty() {
                    saw_3d = true;
                }
            }
        }
    }
    if saw_3d {
        ctx.out
            .warnings
            .push("Ignoring 3D child object for 2D operation".into());
    }
    csg2::union2(&regions)
}

/// Read an external mesh file as a 3D primitive. Format is chosen by
/// extension (case-insensitive). A missing/unreadable file warns and yields
/// empty geometry (evaluation continues, per the reference); an unsupported
/// extension is an error. The path is resolved relative to the working
/// directory and may not escape it (no absolute paths, no `..`).
/// Lay a string out as a 2D region: each glyph's flattened outline is scaled
/// by size/units-per-em, advanced by the (spacing-scaled) glyph advance, and
/// collected into one even-odd region (so counters like 'O'/'e' are holes and
/// glyphs sit side by side). halign/valign anchor the run using the font's
/// advance width and ascent/descent metrics. Uses the bundled default face;
/// font selection and shaping (kerning/ligatures/bidi) are not yet applied.
fn text_region(
    s: &str,
    size: f64,
    spacing: f64,
    halign: Option<&str>,
    valign: Option<&str>,
    steps: usize,
) -> Poly2 {
    let font = match crate::font::default_font() {
        Some(f) => f,
        None => return Poly2::new(Vec::new()),
    };
    let scale = size / font.units_per_em;
    // Every glyph's contours go into ONE even-odd region: a glyph's counters
    // (O/e/a) are holes, and side-by-side glyphs at normal spacing don't
    // overlap, so even-odd fills each correctly. Preview-grade limitation: the
    // reference UNIONS the glyphs, so heavily OVERLAPPING glyphs (spacing < 1,
    // negative spacing, or self-overlapping faces) can show an even-odd hole
    // in the overlap; a robust union of complex holed glyph outlines is beyond
    // this kernel's triangulator (it would lose area), so it is left as-is.
    let mut contours: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut pen = 0.0f64; // in font units
    for ch in s.chars() {
        let gid = font.glyph_id(ch);
        for c in font.glyph_contours(gid, steps) {
            contours.push(c.iter().map(|p| [(pen + p[0]) * scale, p[1] * scale]).collect());
        }
        pen += font.advance(gid) * spacing;
    }
    let mut region = Poly2::new(contours);
    let width = pen * scale;
    // Horizontal anchor from the total advance width.
    let dx = match halign {
        Some("center") => -width / 2.0,
        Some("right") => -width,
        _ => 0.0, // "left" / default / unknown
    };
    // Vertical anchor from font metrics (baseline default; "bottom" floats
    // above y=0 by the descent margin, matching the reference).
    let dy = match valign {
        Some("top") => -font.ascent * scale,
        Some("center") => -(font.ascent + font.descent) / 2.0 * scale,
        Some("bottom") => -font.descent * scale,
        _ => 0.0, // "baseline" / default / unknown
    };
    if dx != 0.0 || dy != 0.0 {
        for c in &mut region.contours {
            for p in c {
                p[0] += dx;
                p[1] += dy;
            }
        }
    }
    region
}

/// Resolve a data-file path (import/surface) relative to the working directory
/// and require it to stay under it — refusing absolute paths, `..` traversal,
/// and symlink escapes (canonicalized). None => refused. A missing file
/// returns its lexical path so the caller's read fails and warns.
fn sandboxed_path(path: &str) -> Option<std::path::PathBuf> {
    let p = std::path::Path::new(path);
    if p.is_absolute() || p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return None;
    }
    let cwd = std::env::current_dir().ok()?;
    let joined = cwd.join(p);
    match joined.canonicalize() {
        Ok(canon) => {
            let root = cwd.canonicalize().unwrap_or(cwd);
            if canon.starts_with(&root) {
                Some(canon)
            } else {
                None // symlink escaping the working directory
            }
        }
        Err(_) => Some(joined), // missing file: let the read fail and warn
    }
}

fn import_file(path: &str, dpi: f64, ctx: &mut Ctx) -> Vec<Shape> {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    // DXF and SVG are 2D vector formats — they import as a 2D shape, not a mesh.
    // Their curve tessellation follows the $fn/$fa/$fs in scope at the call.
    if ext == "dxf" || ext == "svg" {
        let resolved = match sandboxed_path(path) {
            Some(p) => p,
            None => {
                ctx.out.warnings.push(format!("WARNING: Can't open import file '{}'.", path));
                return Vec::new();
            }
        };
        let text = match std::fs::read_to_string(&resolved) {
            Ok(t) => t,
            Err(_) => {
                ctx.out.warnings.push(format!("WARNING: Can't open import file '{}'.", path));
                return Vec::new();
            }
        };
        let get = |k: &str, d: f64| ctx.dynv.lookup(k).and_then(|v| v.as_num()).unwrap_or(d);
        let (fn_, fa, fs) = (get("$fn", 0.0), get("$fa", 12.0), get("$fs", 2.0));
        let (poly, warns) = if ext == "svg" {
            crate::svg::read_svg(&text, dpi, fn_, fa, fs)
        } else {
            io::read_dxf(&text, fn_, fa, fs)
        };
        ctx.out.warnings.extend(warns);
        return Shape::flat(poly).into_iter().collect();
    }
    // 3MF is a ZIP+XML mesh container (binary), read from bytes like STL.
    if ext == "3mf" {
        let resolved = match sandboxed_path(path) {
            Some(p) => p,
            None => {
                ctx.out.warnings.push(format!("WARNING: Can't open import file '{}'.", path));
                return Vec::new();
            }
        };
        return match std::fs::read(&resolved).ok().map(|b| io::read_3mf(&b)) {
            Some(Ok(m)) if !m.tris.is_empty() => leaf(m),
            Some(Ok(_)) => Vec::new(),
            _ => {
                ctx.out.warnings.push(format!("WARNING: Can't open import file '{}'.", path));
                Vec::new()
            }
        };
    }
    let format = match io::format_from_ext(path) {
        Some(f) => f,
        None => {
            ctx.halt(format!(
                "ERROR: Unsupported file format while trying to import file '{}'",
                path
            ));
            return Vec::new();
        }
    };
    let resolved = match sandboxed_path(path) {
        Some(p) => p,
        None => {
            ctx.out
                .warnings
                .push(format!("WARNING: Can't open import file '{}'.", path));
            return Vec::new();
        }
    };
    let mesh = match format {
        io::MeshFormat::Stl => match std::fs::read(&resolved) {
            Ok(bytes) => io::read_stl(&bytes),
            Err(_) => Err(String::new()),
        },
        io::MeshFormat::Off => match std::fs::read_to_string(&resolved) {
            Ok(text) => io::read_off(&text),
            Err(_) => Err(String::new()),
        },
        io::MeshFormat::Amf => match std::fs::read_to_string(&resolved) {
            Ok(text) => Ok(io::read_amf(&text)),
            Err(_) => Err(String::new()),
        },
    };
    match mesh {
        Ok(m) if !m.tris.is_empty() => leaf(m),
        Ok(_) => Vec::new(), // parsed but empty
        Err(_) => {
            ctx.out
                .warnings
                .push(format!("WARNING: Can't open import file '{}'.", path));
            Vec::new()
        }
    }
}

/// Collect the design's SOLID (non-`%`-background) 2D outlines for export to a
/// 2D vector format. A top level that is 3D (and has no 2D geometry) is an
/// error, as is empty geometry — matching the reference's 2D-export messages.
pub fn export_2d(out: &EvalOutput) -> Result<Vec<Poly2>, String> {
    let mut regions = Vec::new();
    let mut saw_3d = false;
    for s in &out.shapes {
        if s.background {
            continue;
        }
        match &s.outline {
            Some(poly) => regions.push(poly.clone()),
            None => {
                if !s.mesh.positions.is_empty() {
                    saw_3d = true;
                }
            }
        }
    }
    if regions.is_empty() {
        return Err(if saw_3d {
            "Current top level object is not a 2D object".into()
        } else {
            "Current top level object is empty".into()
        });
    }
    Ok(regions)
}

/// Merge the design's SOLID (non-`%`-background) 3D geometry into one mesh
/// for export. A purely-2D top level is an error, as is empty geometry (both
/// matching the reference's export messages). Preview-grade: top-level bodies
/// are concatenated, not boolean-unioned (a valid multi-body mesh).
pub fn export_mesh(out: &EvalOutput) -> Result<Mesh, String> {
    let mut combined = Mesh::empty();
    let mut saw_2d = false;
    for s in &out.shapes {
        if s.background {
            continue; // % is excluded from all exports
        }
        if s.outline.is_some() {
            saw_2d = true;
            continue;
        }
        let base = combined.positions.len() as u32;
        combined.positions.extend_from_slice(&s.mesh.positions);
        for t in &s.mesh.tris {
            combined.tris.push([t[0] + base, t[1] + base, t[2] + base]);
        }
    }
    if combined.tris.is_empty() {
        return Err(if saw_2d {
            "Current top level object is not a 3D object".into()
        } else {
            "Current top level object is empty".into()
        });
    }
    Ok(combined)
}

/// Headless render-and-export: apply Customizer overrides (`-D name=value`),
/// resolve include/use, evaluate, and serialize to `format`. This is the CLI's
/// core (`scadforge -o out.stl -D w=40 in.scad`) and the exact
/// override-then-export path the web `/export` route runs, so the command line
/// and the app agree. A fatal evaluation error is returned as `Err`.
pub fn render_export(
    source: &str,
    base_dir: &std::path::Path,
    overrides: &[(String, String)],
    format: &str,
) -> Result<String, String> {
    let effective = crate::customizer::apply_overrides(source, overrides);
    let out = evaluate_source_for(&effective, base_dir, format == "csg");
    // `.echo` captures the console stream regardless of a fatal error.
    if format == "echo" {
        return Ok(echo_stream(&out));
    }
    if let Some(e) = out.error {
        return Err(e);
    }
    export_string(&out, format)
}

/// Export the design to a format's serialized bytes. Binary formats (`3mf`)
/// are produced directly; every text format delegates to `export_string`. This
/// is the CLI's export core, so it covers the binary formats the String-based
/// HTTP path can't.
pub fn export_bytes(out: &EvalOutput, format: &str) -> Result<Vec<u8>, String> {
    match format {
        "3mf" => Ok(crate::io::write_3mf(&export_mesh(out)?)),
        other => export_string(out, other).map(String::into_bytes),
    }
}

/// The `.echo` export: the console message stream — every `ECHO:` line, then
/// the class-prefixed diagnostics (WARNING/DEPRECATED), then a fatal ERROR if
/// one halted the run. Unlike the geometry exports it is produced even when
/// evaluation errored, so a `.echo` of an asserting script still captures the
/// assert message (matching the reference's console-stream semantics).
pub fn echo_stream(out: &EvalOutput) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.extend(out.echoes.iter().cloned());
    lines.extend(out.warnings.iter().cloned());
    if let Some(e) = &out.error {
        lines.push(e.clone());
    }
    let mut s = lines.join("\n");
    if !s.is_empty() {
        s.push('\n');
    }
    s
}

/// Headless render to bytes: apply Customizer overrides, evaluate, and
/// serialize to `format` (binary formats included). The CLI's core.
pub fn render_export_bytes(
    source: &str,
    base_dir: &std::path::Path,
    overrides: &[(String, String)],
    format: &str,
) -> Result<Vec<u8>, String> {
    let effective = crate::customizer::apply_overrides(source, overrides);
    let out = evaluate_source_for(&effective, base_dir, format == "csg");
    // `.echo` captures the console stream regardless of a fatal error.
    if format == "echo" {
        return Ok(echo_stream(&out).into_bytes());
    }
    if let Some(e) = out.error {
        return Err(e);
    }
    export_bytes(&out, format)
}

/// Export the design to a text format's serialized string, dispatching by a
/// lowercase format tag (`stl`|`off`|`amf` for the solid mesh, `svg`|`dxf`|`pdf`
/// for the 2D outlines). The shared core behind both the HTTP `/export` route
/// and the CLI headless render, so the two never drift. `stl` is ASCII (binary STL
/// is available programmatically via `io::write_stl_binary`). An unknown tag,
/// or geometry of the wrong dimensionality, is an `Err` with the reference's
/// message.
pub fn export_string(out: &EvalOutput, format: &str) -> Result<String, String> {
    match format {
        "echo" => Ok(echo_stream(out)),
        "csg" => match &out.csg {
            Some(tree) => Ok(crate::csgfmt::render(tree)),
            // The tree is only recorded when the run asked for it; a caller
            // that evaluated without recording gets a clear message rather
            // than a silently empty file.
            None => Err("the .csg export needs a recording evaluation \
                         (eval::evaluate_source_for(.., true))"
                .into()),
        },
        "svg" => Ok(crate::io::write_svg(&export_2d(out)?)),
        "dxf" => Ok(crate::io::write_dxf_2d(&export_2d(out)?)),
        "pdf" => Ok(crate::io::write_pdf(&export_2d(out)?)),
        "off" => Ok(crate::io::write_off(&export_mesh(out)?)),
        "amf" => Ok(crate::io::write_amf(&export_mesh(out)?)),
        "stl" => Ok(crate::io::write_stl_ascii(&export_mesh(out)?)),
        other => Err(format!("unsupported export format '{}'", other)),
    }
}

fn transform_children(
    children: &[Stmt],
    scope: &Rc<Scope>,
    ctx: &mut Ctx,
    matrix: Option<geom::Mat4>,
) -> Vec<Shape> {
    // A non-finite matrix drops the subtree, for EVERY affine transform.
    // `multmatrix` already did this (the reference requires it), but
    // `translate([0, 0, 0/0])` built a NaN matrix and pushed NaN vertices all
    // the way into the STL — `facet normal NaN NaN NaN`, which no slicer will
    // read. One rule for all of them, applied where they share a path.
    let matrix = match matrix {
        Some(m) if !m.iter().flatten().all(|v| v.is_finite()) => {
            ctx.out.warnings.push(
                "transform: matrix contains NaN/Infinity — subtree removed".into(),
            );
            None
        }
        other => other,
    };
    // translate/rotate/scale/mirror/multmatrix all record as the one
    // matrix they resolved to — that collapse is what "every variable
    // resolved" means for transforms.
    if let (true, Some(m)) = (ctx.csg_on(), &matrix) {
        ctx.csg_head(crate::csgfmt::multmatrix_head(m));
    }
    let mut shapes = exec_scope(children, scope, ctx);
    // Bad arguments drop the subtree from the render; drop it from the
    // export too, or the unnamed frame would splice the children upward
    // and the re-import would draw what the render did not.
    if ctx.csg_on() && matrix.is_none() {
        ctx.csg_clear();
    }
    if let Some(m) = matrix {
        for s in &mut shapes {
            match &mut s.outline {
                // 2D shapes use the matrix's 2D reduction (z row/column
                // dropped), keeping outline and flat fill at z=0.
                Some(poly) => apply_2d(&m, poly, &mut s.mesh),
                None => geom::apply(&m, &mut s.mesh),
            }
        }
        shapes
    } else {
        // Bad transform arguments: drop the subtree rather than render it
        // untransformed in the wrong place. The warning already fired.
        Vec::new()
    }
}

/// Apply a transform's 2D reduction to a 2D shape: the upper-left 2×2 plus
/// x/y translation act on the contours and the flat fill (which stays at
/// z=0). A negative 2D determinant (reflection) rewinds the fill.
fn apply_2d(m: &geom::Mat4, poly: &mut Poly2, mesh: &mut Mesh) {
    let tx = |x: f64, y: f64| {
        (m[0][0] * x + m[0][1] * y + m[0][3], m[1][0] * x + m[1][1] * y + m[1][3])
    };
    for c in &mut poly.contours {
        for v in c {
            let (x, y) = tx(v[0], v[1]);
            *v = [x, y];
        }
    }
    for p in &mut mesh.positions {
        let (x, y) = tx(p[0], p[1]);
        *p = [x, y, 0.0];
    }
    let det2 = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    if det2 < 0.0 {
        for t in &mut mesh.tris {
            t.swap(1, 2);
        }
    }
}

/// Positional-parameter names per builtin module, per the reference
/// signatures.
fn positional_names(module: &str) -> &'static [&'static str] {
    match module {
        "cube" => &["size", "center"],
        "sphere" => &["r"],
        "cylinder" => &["h", "r1", "r2", "center"],
        "translate" | "scale" | "mirror" => &["v"],
        "rotate" => &["a", "v"],
        "multmatrix" => &["m"],
        "resize" => &["newsize", "auto"],
        "polyhedron" => &["points", "faces", "convexity"],
        "square" => &["size", "center"],
        "circle" => &["r"],
        "polygon" => &["points", "paths", "convexity"],
        "linear_extrude" => &["height"],
        "rotate_extrude" => &["angle"],
        "offset" => &["r"],
        "projection" => &["cut"],
        "import" | "import_stl" | "import_off" | "import_dxf" => {
            &["file", "convexity", "layer", "dpi"]
        }
        "surface" => &["file", "center", "convexity"],
        "render" => &["convexity"],
        "text" => &["text", "size", "font", "halign", "valign", "spacing", "direction"],
        "dxf_linear_extrude" => &["file", "layer", "height", "origin", "scale"],
        "dxf_rotate_extrude" => &["file", "layer", "origin"],
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
        // A parenthesized identifier is transparent as a value; it only
        // steered the callee resolution in postfix.
        Expr::Paren(inner) => eval_expr(inner, scope, ctx),
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
            match resolve_function(name, scope) {
                Some((params, body, env)) => {
                    let ev = eval_args(args, scope, ctx);
                    call_function(name, params, body, env, ev, ctx)
                }
                None => {
                    // is_undef(<ident>) is the defined-guard idiom: probe
                    // without the unknown-variable warning. Only when no
                    // user function shadows is_undef (checked above).
                    if name == "is_undef" && args.len() == 1 && args[0].name.is_none() {
                        if let Expr::Ident(var) = &args[0].value {
                            let looked = if var.starts_with('$') {
                                ctx.dynv.lookup(var)
                            } else {
                                scope.lookup(var)
                            };
                            let is_undef = match looked {
                                Some(v) => matches!(v, Value::Undef),
                                None => true,
                            };
                            return Value::Bool(is_undef);
                        }
                    }
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
    Next {
        params: Vec<Param>,
        body: Expr,
        env: Rc<Scope>,
        args: Vec<EvArg>,
        /// The dynamic `$`-environment in force at the tail-call site.
        /// The next frame runs within it (TCE must be transparent to
        /// dynamic scoping), not reset to the original entry env.
        dynv: Rc<DynScope>,
    },
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
    // The base env each frame layers over: the entry env first, then the
    // dynamic env captured at each tail call so eliminated frames' $-args
    // and let($q) bindings stay visible. Empty frame layers collapse away
    // (base_for_next) so plain tail loops keep O(1) dynamic-chain depth.
    let mut base_dyn = entry_dyn.clone();
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
        ctx.dynv = DynScope::layer(&base_dyn);
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
            Tail::Next { params: p, body: b, env: e, args: a, dynv } => {
                params = p;
                body = b;
                env = e;
                args = a;
                base_dyn = base_for_next(&dynv);
            }
        }
    };
    ctx.dynv = entry_dyn;
    ctx.fn_depth -= 1;
    result
}

/// The base a tail call's next frame should layer over: the captured env,
/// but with transparent (empty) layers skipped so a plain tail loop does
/// not build one dynamic-chain link per iteration.
fn base_for_next(dyn_at_call: &Rc<DynScope>) -> Rc<DynScope> {
    let mut cur = dyn_at_call.clone();
    loop {
        let empty = cur.vars.borrow().is_empty();
        match (empty, cur.parent.clone()) {
            (true, Some(parent)) => cur = parent,
            _ => return cur,
        }
    }
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
                Tail::Next { params, body, env, args: ev, dynv: ctx.dynv.clone() }
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
                        dynv: ctx.dynv.clone(),
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
                // The clause's bindings ($-names included) scope to the
                // clause only: layer the dynamic env so they never leak
                // into the enclosing scope's later statements.
                let outer_dyn = ctx.dynv.clone();
                ctx.dynv = DynScope::layer(&outer_dyn);
                let mut cur = sequential_bindings(inits, scope, ctx);
                let mut iters = 0usize;
                loop {
                    if ctx.halted() {
                        break;
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
                            set_var(&next, ctx, name, v);
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
                ctx.dynv = outer_dyn;
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
            format!("function({}) {}", params.join(", "), serialize_body(&fv.body))
        }
    }
}

/// Like serialize_expr but without the outermost parentheses — the
/// source-like form for a function value's body (echo/str display),
/// where `function(x) x + 1` reads better than `function(x) (x + 1)`.
/// Sub-expressions keep their disambiguating parens.
fn serialize_body(e: &Expr) -> String {
    match e {
        Expr::Binary { op, lhs, rhs } => {
            format!("{} {} {}", serialize_expr(lhs), binop_str(*op), serialize_expr(rhs))
        }
        Expr::Ternary { cond, then, els } => format!(
            "{} ? {} : {}",
            serialize_expr(cond),
            serialize_expr(then),
            serialize_expr(els)
        ),
        _ => serialize_expr(e),
    }
}

fn binop_str(op: BinOp) -> &'static str {
    match op {
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
            format!("({} {} {})", serialize_expr(lhs), binop_str(*op), serialize_expr(rhs))
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
        Expr::Paren(inner) => format!("({})", serialize_expr(inner)),
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

/// Documented positional-parameter names for the builtin FUNCTIONS, so
/// named arguments (rands(..., seed_value=7), search(..., index_col_num=1))
/// resolve to the right slot instead of being dropped.
fn builtin_param_names(name: &str) -> &'static [&'static str] {
    match name {
        "atan2" => &["y", "x"],
        "pow" => &["base", "exponent"],
        "cross" => &["a", "b"],
        "search" => &["match_value", "string_or_vector", "num_returns_per_match", "index_col_num"],
        "lookup" => &["key", "table"],
        "rands" => &["min_value", "max_value", "value_count", "seed_value"],
        "parent_module" => &["index"],
        _ => &[],
    }
}

fn call_builtin(name: &str, ev: &[EvArg], scope: &Rc<Scope>, ctx: &mut Ctx) -> Value {
    let _ = scope;
    // Resolve arguments to positions using the shared rule: positionals
    // fill declaration order, named ones land in their slot, unknown
    // plain names warn and are dropped ($-names were layered dynamically
    // by callers already). Builtins without a name table (the pure
    // one/two-number math fns) just take positionals in order.
    let names = builtin_param_names(name);
    let vals: Vec<Value> = if names.is_empty() {
        ev.iter().filter(|a| a.name.is_none()).map(|a| a.value.clone()).collect()
    } else {
        let mut slots: Vec<Option<Value>> = vec![None; names.len()];
        let mut extra: Vec<Value> = Vec::new();
        let mut pos = 0usize;
        for a in ev {
            match &a.name {
                Some(n) if n.starts_with('$') => {}
                Some(n) => match names.iter().position(|p| p == n) {
                    Some(i) => slots[i] = Some(a.value.clone()),
                    None => ctx
                        .out
                        .warnings
                        .push(format!("{}: unknown parameter '{}' ignored", name, n)),
                },
                None => {
                    while pos < names.len() && slots[pos].is_some() {
                        pos += 1;
                    }
                    if pos < names.len() {
                        slots[pos] = Some(a.value.clone());
                        pos += 1;
                    } else {
                        extra.push(a.value.clone());
                    }
                }
            }
        }
        // Trailing unset slots stay absent; a hole before a set slot
        // becomes undef so later positionals keep their index.
        let last = slots.iter().rposition(Option::is_some);
        let mut vals: Vec<Value> = match last {
            Some(i) => slots[..=i]
                .iter()
                .map(|s| s.clone().unwrap_or(Value::Undef))
                .collect(),
            None => Vec::new(),
        };
        vals.extend(extra);
        vals
    };
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
            // Bounds-check BEFORE the cast: n as u32 wraps modulo 2^32,
            // which would resurrect valid characters from huge inputs
            // (2^32 + 65 must contribute nothing, not "A").
            let t = n.trunc();
            if n.is_finite() && (1.0..=0x10FFFF as f64).contains(&t) {
                if let Some(c) = char::from_u32(t as u32) {
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

    // ---- .csg export: the evaluated instantiation tree ----------------

    fn csg_of(src: &str) -> String {
        render_export(src, std::path::Path::new("."), &[], "csg").unwrap()
    }

    /// The oracle in one line: does re-running the export reproduce the
    /// design? Compares the exported STL, so it covers geometry, not just
    /// tree shape.
    fn assert_csg_round_trips(src: &str) {
        let base = std::path::Path::new(".");
        let tree = csg_of(src);
        let before = render_export(src, base, &[], "stl")
            .unwrap_or_else(|e| panic!("source did not export: {e}\n{src}"));
        let after = render_export(&tree, base, &[], "stl")
            .unwrap_or_else(|e| panic!("exported .csg did not re-import: {e}\n{tree}"));
        assert_eq!(before, after, "re-importing the .csg changed the geometry\n{tree}");
    }

    /// Like `assert_csg_round_trips`, but for scripts whose render is empty:
    /// the export must reproduce THAT too, rather than conjuring geometry.
    fn assert_csg_round_trips_or_both_empty(src: &str) {
        let base = std::path::Path::new(".");
        let tree = csg_of(src);
        let before = render_export(src, base, &[], "stl");
        let after = render_export(&tree, base, &[], "stl");
        match (before, after) {
            (Ok(a), Ok(b)) => assert_eq!(a, b, "geometry changed on re-import\n{tree}"),
            (Err(_), Err(_)) => {}
            (Ok(_), Err(e)) => panic!("re-import lost the geometry: {e}\n{tree}"),
            (Err(_), Ok(_)) => panic!("re-import INVENTED geometry the render refused\n{tree}"),
        }
    }

    #[test]
    fn csg_resolves_variables_loops_and_user_modules() {
        // Everything here has to disappear: the variable, the loop, the
        // user module, and the arithmetic in the arguments.
        let tree = csg_of(
            "n = 2; module post() { cylinder(h = 4, r = 1, $fn = 5); }\n\
             for (i = [0 : n]) translate([i * 10, 0, 0]) post();",
        );
        assert_eq!(
            tree,
            "group() {\n\
             \tgroup() {\n\
             \t\tmultmatrix([[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]) {\n\
             \t\t\tgroup() {\n\
             \t\t\t\tcylinder($fn = 5, $fa = 12, $fs = 2, h = 4, r1 = 1, r2 = 1, center = false);\n\
             \t\t\t}\n\
             \t\t}\n\
             \t\tmultmatrix([[1, 0, 0, 10], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]) {\n\
             \t\t\tgroup() {\n\
             \t\t\t\tcylinder($fn = 5, $fa = 12, $fs = 2, h = 4, r1 = 1, r2 = 1, center = false);\n\
             \t\t\t}\n\
             \t\t}\n\
             \t\tmultmatrix([[1, 0, 0, 20], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]) {\n\
             \t\t\tgroup() {\n\
             \t\t\t\tcylinder($fn = 5, $fa = 12, $fs = 2, h = 4, r1 = 1, r2 = 1, center = false);\n\
             \t\t\t}\n\
             \t\t}\n\
             \t}\n\
             }\n",
            "actual:\n{}",
            tree
        );
    }

    #[test]
    fn csg_keeps_one_node_per_child_statement() {
        // THE structural invariant. A `for` that emits three cubes is ONE
        // operand of the difference, not three: flattening it would make
        // the re-import subtract two of the cubes from the first.
        let src = "difference() { for (i = [0:2]) translate([i, 0, 0]) cube(1); sphere(0.4); }";
        let tree = csg_of(src);
        let top: Vec<&str> = tree
            .lines()
            .filter(|l| l.starts_with("\t\t") && !l.starts_with("\t\t\t"))
            .collect();
        assert_eq!(
            top,
            vec!["\t\tgroup() {", "\t\t}", "\t\tsphere($fn = 0, $fa = 12, $fs = 2, r = 0.4);"],
            "difference must see exactly two operands:\n{}",
            tree
        );
        assert_csg_round_trips(src);
    }

    #[test]
    fn csg_records_modifiers_as_statement_prefixes() {
        // `*` never instantiates, so it leaves no trace; `#`, `%` and `!`
        // are recorded as the prefixes they are — which keeps the export
        // valid OpenSCAD AND keeps its meaning on re-import.
        let tree = csg_of("difference() { cube(4, center = true); #sphere(1); %sphere(2); *sphere(3); }");
        assert!(tree.contains("#sphere("), "highlight kept:\n{}", tree);
        assert!(tree.contains("%sphere("), "background kept:\n{}", tree);
        assert!(!tree.contains("r = 3"), "disabled subtree must not appear:\n{}", tree);
        assert!(csg_of("!cube(2); sphere(1);").contains("!cube("), "root kept");
    }

    #[test]
    fn csg_drops_a_subtree_the_render_dropped() {
        // A bad transform argument drops its children from the render. The
        // export must drop them too — an unnamed frame that spliced its
        // children upward would draw a cube the preview never showed.
        let tree = csg_of("translate(\"nope\") cube(3);");
        assert!(!tree.contains("cube"), "dropped subtree leaked into the export:\n{}", tree);
    }

    #[test]
    fn csg_round_trips_the_language() {
        // One scene per construct that has its own recording path.
        for src in [
            "cube([1, 2, 3], center = true);",
            "sphere(d = 4, $fn = 12);",
            "cylinder(h = 3, r1 = 2, r2 = 0, $fn = 9);",
            "polyhedron(points = [[0,0,0],[2,0,0],[0,2,0],[0,0,2]], \
                         faces = [[0,2,1],[0,1,3],[1,2,3],[2,0,3]]);",
            "rotate([20, 30, 40]) scale([1, 2, 0.5]) mirror([1, 0, 0]) cube(2);",
            "multmatrix([[1,0,0,1],[0,1,0,2],[0,0,1,3],[0,0,0,1]]) cube(2);",
            "difference() { cube(4, center = true); sphere(2.4, $fn = 16); }",
            "intersection() { cube(4, center = true); sphere(2.6, $fn = 16); }",
            "union() { cube(2); translate([1,1,1]) sphere(1, $fn = 10); }",
            "hull() { cube(1); translate([4, 0, 0]) sphere(1, $fn = 10); }",
            "minkowski() { cube([4, 4, 1]); cylinder(r = 1, h = 1, $fn = 8); }",
            "render() cube(2);",
            "color(\"tomato\") cube(2);",
            "color([0.2, 0.4, 0.6, 0.5]) cube(2);",
            "resize([10, 0, 0], auto = true) cube(2);",
            "linear_extrude(height = 3, twist = 45, slices = 6, scale = 0.5) square(4, center = true);",
            "rotate_extrude(angle = 270, $fn = 24) translate([4, 0]) square([1, 2]);",
            "linear_extrude(2) offset(r = 1, $fn = 16) square([6, 3], center = true);",
            "linear_extrude(2) offset(delta = -0.5) square([6, 3], center = true);",
            "linear_extrude(1) polygon(points = [[0,0],[5,0],[5,4],[2,6],[0,4]]);",
            "linear_extrude(1) text(\"Ab\", size = 8, halign = \"center\");",
            "linear_extrude(1) projection(cut = true) translate([0,0,-1]) sphere(3, $fn = 14);",
            "intersection_for (a = [0 : 60 : 179]) rotate([0, 0, a]) cube([8, 2, 2], center = true);",
            "module wrap() { children(); } wrap() cube(2);",
            "module twice() { children(0); translate([3,0,0]) children(0); } twice() sphere(1, $fn = 8);",
            "let (h = 4) cube([1, 1, h]);",
            "if (true) cube(2); else sphere(1);",
            "for (i = [0:2]) { translate([i*3, 0, 0]) cube(1); }",
            "assert(1 < 2) cube(2);",
            "echo(\"noise\"); cube(2);",
            "$fn = 7; sphere(2);",
        ] {
            assert_csg_round_trips(src);
        }
    }

    #[test]
    fn csg_records_one_node_per_statement_even_when_it_draws_nothing() {
        // REGRESSION. The recorder used to SPLICE an unnamed frame's nodes
        // into its parent, so a statement that instantiated two shapes
        // recorded two nodes — and a surrounding boolean gained an operand.
        // The renderer counts one operand per child statement, so the export
        // has to as well, whether the statement drew two shapes, one, or none.
        for src in [
            // children() forwards two shapes: ONE operand of the intersection.
            "module m() { intersection() { cube(20, center=true); children(); } }\n\
             m() { sphere(12); translate([16,0,0]) sphere(12); }",
            "intersection() { cube(20, center=true); \
             assert(true) { sphere(12); translate([16,0,0]) sphere(12); } }",
            // Statements that draw NOTHING still hold their operand slot.
            "intersection() { cube(20, center=true); *sphere(12); }",
            "intersection() { cube(20, center=true); nosuchmodule(); }",
            "difference() { translate(\"oops\") cube(20, center=true); sphere(11); }",
            "difference() { cube(10); echo(\"noise\"); }",
        ] {
            assert_csg_round_trips_or_both_empty(src);
        }
    }

    #[test]
    fn csg_modifier_lands_on_its_own_statement() {
        // REGRESSION. csg_modify prefixes the last node in the frame. When
        // the modified statement recorded nothing, that was the PRECEDING
        // SIBLING — so `cube(10); %echo("hi");` marked the CUBE background,
        // and the re-import rendered nothing at all.
        let tree = csg_of("cube(10);\n%echo(\"hi\");");
        assert!(!tree.contains("%cube"), "the % landed on the cube:\n{}", tree);
        assert_csg_round_trips("cube(10);\n%echo(\"hi\");");
        assert_csg_round_trips("cube(10);\n#nosuchmodule();");
    }

    #[test]
    fn csg_records_resolved_slices_invert_and_dpi() {
        // REGRESSION. These three values are computed by the renderer, not
        // readable off the bound arguments: `slices` comes from $fa when
        // twist is set and slices omitted, and invert/dpi were simply absent
        // from their heads. Each one silently changed the re-imported model.
        let tree = csg_of("linear_extrude(height = 5, twist = 90) square([4,6], center=true);");
        assert!(!tree.contains("slices = 1,"), "twist flattened to one slice:\n{}", tree);
        assert_csg_round_trips("linear_extrude(height = 5, twist = 90) square([4,6], center=true);");
        assert_csg_round_trips("linear_extrude(height = 4, twist = 180, $fa = 3) square(3);");
        // invert/dpi need real files, so just pin that the heads carry them.
        let f = crate::csgfmt::Frags { fn_: 0.0, fa: 12.0, fs: 2.0 };
        assert!(crate::csgfmt::surface_head("h.dat", true, true, 1.0).contains("invert = true"));
        assert!(crate::csgfmt::import_head("a.svg", None, 1.0, 25.4, f).contains("dpi = 25.4"));
    }

    #[test]
    fn csg_does_not_export_a_shape_the_renderer_rejected() {
        // REGRESSION. builtin_head coerced sizes the renderer refuses, so a
        // statement that drew nothing exported a unit cube.
        for src in ["cube(size = \"big\");", "square([4,5,6]);", "cube(size = [1,2]);"] {
            assert_csg_round_trips_or_both_empty(src);
        }
    }

    #[test]
    fn non_finite_transforms_drop_their_subtree() {
        // REGRESSION. `multmatrix` dropped a non-finite matrix (the reference
        // requires it) but `translate` did not, so translate([0,0,0/0]) wrote
        // an STL full of `facet normal NaN NaN NaN` that no slicer can read.
        // Every affine transform shares one rule now.
        for src in [
            "translate([0, 0, 0/0]) cube(3);",
            "scale([1, 1/0, 1]) cube(3);",
            "rotate([0/0, 0, 0]) cube(3);",
        ] {
            let out = run(src);
            assert!(
                out.shapes.iter().all(|s| s
                    .mesh
                    .positions
                    .iter()
                    .all(|p| p.iter().all(|c| c.is_finite()))),
                "non-finite vertices reached the mesh for: {src}"
            );
            assert!(
                out.warnings.iter().any(|w| w.contains("NaN/Infinity")),
                "the dropped subtree was not reported for: {src}"
            );
        }
    }

    #[test]
    fn csg_needs_a_recording_evaluation() {
        // Asking a non-recording EvalOutput for its tree is an error with a
        // pointer to the fix, not a silently empty file.
        let out = run("cube(1);");
        let err = export_string(&out, "csg").unwrap_err();
        assert!(err.contains("recording"), "message: {err}");
    }

    #[test]
    fn echo_export_captures_the_console_stream() {
        let base = std::path::Path::new(".");
        // A normal run: ECHO lines and a warning, no geometry needed.
        let src = "echo(\"hi\", 1 + 2); frobnicate();";
        let s = render_export(src, base, &[], "echo").unwrap();
        assert!(s.contains("ECHO: \"hi\", 3"), "echo stream: {s}");
        assert!(s.contains("WARNING") && s.contains("frobnicate"), "warnings included: {s}");

        // A run that fatally errors still yields the stream (ECHO before the
        // assert, plus the error line) rather than failing the export.
        let bad = "echo(\"before\"); assert(false, \"boom\"); cube(1);";
        let out = render_export(bad, base, &[], "echo").unwrap();
        assert!(out.contains("ECHO: \"before\""), "pre-assert echo kept: {out}");
        assert!(out.contains("boom") || out.contains("assert"), "error captured: {out}");
    }

    #[test]
    fn render_export_applies_defines_and_serializes() {
        // A parametric cube whose edge is a Customizer parameter.
        let src = "e = 2; // [1:10]\ncube(e);";
        let base = std::path::Path::new(".");
        // No overrides → edge-2 cube; ASCII STL mentions a 2.0 vertex.
        let stl = render_export(src, base, &[], "stl").unwrap();
        assert!(stl.contains("facet"), "STL body: {}", &stl[..stl.len().min(80)]);
        // -D e=5 grows the cube; the STL must now carry a 5 coordinate and no 2.
        let stl5 = render_export(src, base, &[("e".into(), "5".into())], "stl").unwrap();
        assert!(stl5.contains("5.0") || stl5.contains(" 5 "), "override took effect");
        // A rejected override (wrong kind) leaves the default in force.
        let stl_bad = render_export(src, base, &[("e".into(), "\"x\"".into())], "stl").unwrap();
        assert_eq!(stl, stl_bad);
        // An unknown format is an error, not a panic.
        assert!(render_export(src, base, &[], "obj").is_err());
        // A 2D design exports SVG; asking for STL is the reference's error.
        let sq = "s = 4; // [1:9]\nsquare(s);";
        assert!(render_export(sq, base, &[], "svg").unwrap().contains("<svg"));
        assert!(render_export(sq, base, &[], "stl").unwrap_err().contains("not a 3D"));
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
            cycle_scopes: Vec::new(),
            csg: None,
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
        // An unknown module's children are skipped, not evaluated.
        let out = run("mystery() { cube(1); }");
        assert!(out.shapes.is_empty());
    }

    fn total_volume(out: &EvalOutput) -> f64 {
        // Winding-aware signed volume summed over every shape.
        let mut v = 0.0;
        for s in &out.shapes {
            for t in &s.mesh.tris {
                let a = s.mesh.positions[t[0] as usize];
                let b = s.mesh.positions[t[1] as usize];
                let c = s.mesh.positions[t[2] as usize];
                v += (a[0] * (b[1] * c[2] - b[2] * c[1])
                    + a[1] * (b[2] * c[0] - b[0] * c[2])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0;
            }
        }
        v.abs()
    }

    #[test]
    fn csg_operators_combine_real_geometry() {
        // difference carves a hole: 2^3 cube minus a 1^3 cube = 7.
        let out = run("difference() { cube(2, center = true); cube(1, center = true); }");
        assert_eq!(out.shapes.len(), 1);
        assert!((total_volume(&out) - 7.0).abs() < 1e-5, "vol {}", total_volume(&out));
        // intersection of two offset unit cubes = a 0.5-thick slab.
        let out = run("intersection() { cube(1); translate([0.5,0,0]) cube(1); }");
        assert!((total_volume(&out) - 0.5).abs() < 1e-5, "vol {}", total_volume(&out));
        // difference keeps the minuend's (first child's) color.
        let out = run("difference() { color(\"red\") cube(2, center=true); cube(1, center=true); }");
        assert_eq!(out.shapes[0].color, Some([1.0, 0.0, 0.0, 1.0]));
        // The subtrahends are implicitly unioned before subtracting.
        let out = run(
            "difference() { cube(3, center=true); \
             translate([-1,0,0]) cube(1, center=true); \
             translate([1,0,0]) cube(1, center=true); }",
        );
        assert!((total_volume(&out) - (27.0 - 2.0)).abs() < 1e-5, "vol {}", total_volume(&out));
        // Empty minuend → empty result.
        let out = run("difference() { if (false) cube(1); cube(5); }");
        assert!(out.shapes.is_empty());
        // intersection with an empty child annihilates, in any order
        // (commutative), matching strict set semantics.
        assert!(run("intersection() { cube(5); if (false) sphere(1); }").shapes.is_empty());
        assert!(run("intersection() { if (false) sphere(1); cube(5); }").shapes.is_empty());
        // hull() convexifies the combined children into one solid.
        let out = run("hull() { translate([-8,0,0]) sphere(2, $fn=12); translate([8,0,0]) sphere(2, $fn=12); }");
        assert_eq!(out.shapes.len(), 1);
        assert!(total_volume(&out) > 0.0);
        // minkowski() rounds a cube with a sphere → one larger solid.
        let plain = run("cube(10, center=true);");
        let out = run("minkowski() { cube(10, center=true); sphere(2, $fn=12); }");
        assert_eq!(out.shapes.len(), 1);
        assert!(total_volume(&out) > total_volume(&plain));
        // hull()/minkowski() DROP children colors; an enclosing color()
        // then applies (reference hull EDGE[6] / minkowski EDGE[9]).
        let out = run("color(\"red\") hull() { color(\"blue\") cube(1); translate([3,0,0]) sphere(1,$fn=8); }");
        assert_eq!(out.shapes[0].color, Some([1.0, 0.0, 0.0, 1.0]));
        // A bare hull() with a colored child stays uncolored (color dropped).
        let out = run("hull() { color(\"blue\") cube(1); translate([3,0,0]) cube(1); }");
        assert_eq!(out.shapes[0].color, None);
    }

    /// Flat-area (z=0) sum over a 2D shape's fill mesh.
    fn flat_area(out: &EvalOutput) -> f64 {
        let mut a = 0.0;
        for s in &out.shapes {
            for t in &s.mesh.tris {
                let p = &s.mesh.positions;
                let (u, v, w) = (p[t[0] as usize], p[t[1] as usize], p[t[2] as usize]);
                a += ((v[0] - u[0]) * (w[1] - u[1]) - (v[1] - u[1]) * (w[0] - u[0])).abs() / 2.0;
            }
        }
        a
    }

    #[test]
    fn two_d_primitives_and_transforms() {
        // square lives at z=0, area x*y, and is tagged 2D (outline present).
        let out = run("square([4, 3]);");
        assert_eq!(out.shapes.len(), 1);
        assert!(out.shapes[0].outline.is_some(), "square must be 2D");
        assert!(out.shapes[0].mesh.positions.iter().all(|p| p[2] == 0.0));
        assert!((flat_area(&out) - 12.0).abs() < 1e-9);
        // circle($fn) is a regular n-gon; $fn=6 → hexagon area = 1.5·√3·r².
        let out = run("circle(2, $fn=6);");
        assert!((flat_area(&out) - 1.5 * 3f64.sqrt() * 4.0).abs() < 1e-9);
        // polygon with an explicit hole path (even-odd) → outer minus hole.
        let out = run(
            "polygon(points=[[0,0],[10,0],[10,10],[0,10],[3,3],[7,3],[7,7],[3,7]], \
             paths=[[0,1,2,3],[4,5,6,7]]);",
        );
        assert!((flat_area(&out) - 84.0).abs() < 1e-6, "area {}", flat_area(&out));
        // A 2D translate keeps it flat and shifts it; a 3D rotate reduces
        // to its 2D part (stays at z=0).
        let out = run("translate([5, 0, 0]) square([2, 2]);");
        assert!(out.shapes[0].mesh.positions.iter().all(|p| p[2] == 0.0 && p[0] >= 4.999));
        let out = run("rotate([90, 0, 0]) square([2, 2]);");
        assert!(out.shapes[0].mesh.positions.iter().all(|p| p[2] == 0.0), "2D reduction keeps z=0");
        // Feeding a 2D shape to a 3D boolean warns rather than misbehaving.
        let out = run("difference() { cube(4, center=true); square(2); }");
        assert!(out.warnings.iter().any(|w| w.contains("2D")));
    }

    #[test]
    fn extrusions_bridge_2d_to_3d() {
        // linear_extrude a square → a box (now a real 3D shape, no outline).
        let out = run("linear_extrude(height=5) square([3, 2]);");
        assert_eq!(out.shapes.len(), 1);
        assert!(out.shapes[0].outline.is_none(), "extrusion output is 3D");
        assert!((total_volume(&out) - 30.0).abs() < 1e-9);
        // Default height is 100 (the famous trap).
        let out = run("linear_extrude() square(1);");
        assert!((total_volume(&out) - 100.0).abs() < 1e-9);
        // A twisted extrusion still closes (positive volume).
        let out = run("linear_extrude(height=10, twist=90, $fn=16) square([4,1], center=true);");
        assert!(total_volume(&out) > 0.0 && out.shapes[0].outline.is_none());
        // rotate_extrude a rectangle offset on X → a washer, volume 12π.
        let out = run("rotate_extrude($fn=128) translate([2,0]) square([2,1]);");
        assert!((total_volume(&out) - std::f64::consts::PI * 12.0).abs() < 0.3);
        // A profile crossing X=0 errors (empty + warning), doesn't crash.
        let out = run("rotate_extrude() translate([-1,0]) square([2,1]);");
        assert!(out.shapes.is_empty());
        assert!(out.warnings.iter().any(|w| w.contains("same X")));
    }

    #[test]
    fn two_d_booleans() {
        // 2D difference: a 10×10 square minus a centered 4×4 → area 84, and
        // the result is still 2D (carries an outline) so it can extrude.
        let out = run(
            "difference() { square(10); translate([3,3]) square(4); }",
        );
        assert_eq!(out.shapes.len(), 1);
        assert!(out.shapes[0].outline.is_some(), "2D difference stays 2D");
        assert!((flat_area(&out) - 84.0).abs() < 1e-5, "area {}", flat_area(&out));
        assert!(out.warnings.is_empty(), "clean 2D difference must not warn");

        // 2D intersection of two overlapping squares → their overlap.
        let out = run("intersection() { square(10); translate([6,6]) square(10); }");
        assert!((flat_area(&out) - 16.0).abs() < 1e-5, "overlap area {}", flat_area(&out));

        // 2D union (via linear_extrude collecting crossing squares): a plus
        // sign, NOT a plus with a hole — area = two 2×10 bars minus the 2×2
        // overlap = 36, and it extrudes to a volume of 36.
        let out = run(
            "linear_extrude(height=1) { translate([-5,-1]) square([10,2]); \
             translate([-1,-5]) square([2,10]); }",
        );
        assert!((total_volume(&out) - 36.0).abs() < 1e-5, "plus volume {}", total_volume(&out));

        // 2D difference feeds an extrusion: the ring above, 2 tall → 168.
        let out = run(
            "linear_extrude(height=2) difference() { square(10); translate([3,3]) square(4); }",
        );
        assert!((total_volume(&out) - 168.0).abs() < 1e-4, "ring prism vol {}", total_volume(&out));

        // 2D hull of two separated squares fills the gap (area exceeds the 2
        // squares' 2·1 and it stays 2D).
        let out = run("hull() { square(1); translate([4,0]) square(1); }");
        assert!(out.shapes[0].outline.is_some(), "2D hull stays 2D");
        assert!(flat_area(&out) > 2.0, "hull area {}", flat_area(&out));

        // 2D minkowski grows a square by a circle (rounded square), strictly
        // larger than the 4-unit square alone.
        let plain = run("square([2,2]);");
        let out = run("minkowski() { square([2,2]); circle(1, $fn=32); }");
        assert!(flat_area(&out) > flat_area(&plain), "minkowski must grow the square");

        // Color carries from the first child of a 2D difference.
        let out = run("difference() { color(\"red\") square(4); square(2); }");
        assert_eq!(out.shapes[0].color, Some([1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn offset_and_projection() {
        // offset(r) rounds+grows a square: 10×10 + four 10×2 strips + π·2².
        let out = run("offset(r = 2) square(10);");
        assert_eq!(out.shapes.len(), 1);
        assert!(out.shapes[0].outline.is_some(), "offset output is 2D");
        let want = 100.0 + 80.0 + std::f64::consts::PI * 4.0;
        assert!((flat_area(&out) - want).abs() < 1.5, "offset(r) area {}", flat_area(&out));
        // offset(delta) miters the corners → 196; chamfer cuts them → 188.
        assert!((flat_area(&run("offset(delta = 2) square(10);")) - 196.0).abs() < 0.5);
        assert!((flat_area(&run("offset(delta = 2, chamfer = true) square(10);")) - 188.0).abs() < 0.5);
        // Bare offset() defaults to a 1-unit round offset (offset(r=1)).
        let bare = flat_area(&run("offset() square(10);"));
        let r1 = flat_area(&run("offset(r = 1) square(10);"));
        assert!((bare - r1).abs() < 1e-6, "offset() must equal offset(r=1)");
        // A negative offset can annihilate the shape silently (no warning).
        let out = run("offset(delta = -2) square(3);");
        assert!(out.shapes.is_empty());
        assert!(out.warnings.is_empty(), "silent annihilation");
        // offset feeds an extrusion (2D→2D→3D): rounded square, 1 tall.
        let out = run("linear_extrude(height = 1) offset(r = 2) square(10);");
        assert!((total_volume(&out) - want).abs() < 1.5, "extruded offset vol {}", total_volume(&out));

        // projection(cut=false): the shadow of a centered cube → 10×10 = 100.
        let out = run("projection() cube(10, center = true);");
        assert_eq!(out.shapes.len(), 1);
        assert!(out.shapes[0].outline.is_some(), "projection output is 2D");
        assert!((flat_area(&out) - 100.0).abs() < 1e-4, "silhouette area {}", flat_area(&out));
        // cut=false ignores Z: a cube lifted to z∈[45,55] casts the same shadow.
        let out = run("projection() translate([0,0,50]) cube(10, center = true);");
        assert!((flat_area(&out) - 100.0).abs() < 1e-4, "Z-ignored area {}", flat_area(&out));
        // cut=true slices at z=0: a straddling cube → the 100-area section.
        let out = run("projection(cut = true) cube(10, center = true);");
        assert!((flat_area(&out) - 100.0).abs() < 1e-4, "cut area {}", flat_area(&out));
        // cut=true of a solid entirely above z=0 → empty, silently.
        let out = run("projection(cut = true) translate([0,0,10]) cube(10, center = true);");
        assert!(out.shapes.is_empty());
        // The projected silhouette re-extrudes (the flatten→re-extrude idiom).
        let out = run("linear_extrude(height = 3) projection() cube(10, center = true);");
        assert!((total_volume(&out) - 300.0).abs() < 1e-3, "re-extrude vol {}", total_volume(&out));
    }

    #[test]
    fn number_formatting_matches_2021_01() {
        // The double-conversion 6-significant-digit rules: fixed for
        // 1e-5 ≤ |x| < 1e6, scientific (lowercase e, signed, no zero-pad)
        // otherwise; trailing zeros trimmed; -0 → 0.
        let cases = [
            (1.0 / 3.0, "0.333333"),
            (std::f64::consts::PI, "3.14159"),
            (10.0, "10"),
            (100000.0, "100000"),
            (1_000_000.0, "1e+6"),
            (1_234_567.0, "1.23457e+6"),
            (0.0001, "0.0001"),
            (0.00001, "0.00001"),
            (1.1e-6, "1.1e-6"),
            (1e100, "1e+100"),
            (-0.0, "0"),
            (0.1 + 0.2, "0.3"),
        ];
        for (x, want) in cases {
            assert_eq!(fmt_num(x), want, "fmt_num({})", x);
        }
        assert_eq!(fmt_num(f64::INFINITY), "inf");
        assert_eq!(fmt_num(f64::NEG_INFINITY), "-inf");
        assert_eq!(fmt_num(f64::NAN), "nan");
        // Value display forms: quoted strings inside vectors, spaced-colon
        // ranges, nested vectors.
        assert_eq!(fmt_value(&Value::Str("a".into()), true), "\"a\"");
        assert_eq!(fmt_value(&Value::Str("a".into()), false), "a"); // str() top-level bare
    }

    #[test]
    fn diagnostic_surface_prefixes_each_class() {
        // Plain diagnostics become WARNING:; DEPRECATED keeps its own class
        // (never double-prefixed).
        let out = run("frob(1); cube(1);");
        assert!(!out.warnings.is_empty());
        assert!(
            out.warnings.iter().all(|w| w.starts_with("WARNING:") || w.starts_with("DEPRECATED:")),
            "every diagnostic carries a class prefix: {:?}",
            out.warnings
        );
        assert!(out.warnings.iter().any(|w| w.starts_with("WARNING: unknown module")));
        let out = run("assign(x = 1) cube(x);");
        assert!(out.warnings.iter().any(|w| w.starts_with("DEPRECATED:")));
        assert!(!out.warnings.iter().any(|w| w.starts_with("WARNING: DEPRECATED")));
    }

    #[test]
    fn import_and_export() {
        // Round-trip: write an ASCII STL cube to a CWD-relative file, import
        // it back, and check the volume survives.
        let path = "scadforge_test_import_cube.stl";
        let cube = geom::cube([2.0, 3.0, 4.0], false); // volume 24
        std::fs::write(path, io::write_stl_ascii(&cube)).unwrap();
        let out = run(&format!("import(\"{}\");", path));
        std::fs::remove_file(path).ok();
        assert_eq!(out.shapes.len(), 1, "imported mesh is one 3D shape");
        assert!(out.shapes[0].outline.is_none(), "imported STL is 3D");
        assert!((total_volume(&out) - 24.0).abs() < 1e-3, "imported vol {}", total_volume(&out));

        // 3MF round-trip through the CLI's export core: render a cube to .3mf
        // bytes (a real ZIP), write them, import back, and check the volume.
        let p3 = "scadforge_test_import.3mf";
        let bytes = render_export_bytes("cube([2,3,4]);", std::path::Path::new("."), &[], "3mf").unwrap();
        std::fs::write(p3, &bytes).unwrap();
        let out = run(&format!("import(\"{}\");", p3));
        std::fs::remove_file(p3).ok();
        assert!(out.shapes.len() == 1 && out.shapes[0].outline.is_none(), "imported 3MF is one 3D mesh");
        assert!((total_volume(&out) - 24.0).abs() < 1e-3, "3MF round-trip vol {}", total_volume(&out));

        // Error paths (no file needed).
        assert!(
            run("import(\"nope.stl\");").warnings.iter().any(|w| w.contains("Can't open")),
            "missing file warns, not fatal"
        );
        assert!(run("import(\"model.xyz\");").error.is_some(), "unsupported format is an error");
        assert!(
            run("import(\"/etc/hosts.stl\");").warnings.iter().any(|w| w.contains("Can't open")),
            "an absolute path is refused"
        );
        assert!(
            run("import(\"../secret.stl\");").warnings.iter().any(|w| w.contains("Can't open")),
            "a .. traversal is refused"
        );

        // export_mesh: 3D exports; 2D-only and empty are the reference errors;
        // `%` background is excluded from the export.
        assert!(export_mesh(&run("cube(2, center=true);")).is_ok());
        assert!(export_mesh(&run("square(5);")).unwrap_err().contains("not a 3D object"));
        assert!(export_mesh(&run("if (false) cube(1);")).unwrap_err().contains("empty"));
        assert!(
            export_mesh(&run("%cube(2, center=true);")).unwrap_err().contains("empty"),
            "an only-% scene exports empty"
        );

        // The HTTP export route serializes ASCII STL / OFF and reports errors.
        let stl = crate::http::handle("POST", "/export?format=stl", "cube(2, center=true);");
        assert_eq!(stl.status, "200 OK");
        assert!(stl.body.starts_with("solid scadforge_model"));
        let off = crate::http::handle("POST", "/export?format=off", "cube(2, center=true);");
        assert!(off.body.starts_with("OFF\n"));
        let bad = crate::http::handle("POST", "/export?format=stl", "square(5);");
        assert_eq!(bad.status, "422 Unprocessable Entity");
        assert!(bad.body.contains("not a 3D object"));
    }

    #[test]
    fn include_and_use() {
        let base = std::env::current_dir().unwrap();

        // include: geometry runs, definitions register, and a later same-name
        // assignment in the main file wins (override-after-include).
        std::fs::write(
            "scadforge_test_inc.scad",
            "w = 3;\nmodule widget() { cube([2, 2, 2]); }\ntranslate([0, 20, 0]) sphere(3);\n",
        )
        .unwrap();
        let out = evaluate_source(
            "include <scadforge_test_inc.scad>\nw = 7;\nwidget();\ncube([w, 1, 1]);",
            &base,
        );
        std::fs::remove_file("scadforge_test_inc.scad").ok();
        assert!(out.error.is_none(), "include error: {:?}", out.error);
        // widget() cube + the included sphere + the main w-sized cube.
        assert!(out.shapes.len() >= 3, "include runs geometry AND exposes defs");
        let maxx = out
            .shapes
            .iter()
            .flat_map(|s| s.mesh.positions.iter())
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((maxx - 7.0).abs() < 1e-6, "main override w=7 wins, maxx {}", maxx);

        // use: only definitions are exposed; the used file's geometry is NOT
        // instantiated.
        std::fs::write(
            "scadforge_test_use.scad",
            "module gadget() { cube([5, 5, 5]); }\nsphere(100);\n",
        )
        .unwrap();
        let out = evaluate_source("use <scadforge_test_use.scad>\ngadget();", &base);
        std::fs::remove_file("scadforge_test_use.scad").ok();
        assert_eq!(out.shapes.len(), 1, "use exposes defs only, not geometry");
        assert!((total_volume(&out) - 125.0).abs() < 1e-6, "gadget vol {}", total_volume(&out));

        // Missing files warn (distinct messages) and are non-fatal.
        let out = evaluate_source("include <nope.scad>\ncube(1);", &base);
        assert!(out.warnings.iter().any(|w| w.contains("Can't open include file")));
        assert_eq!(out.shapes.len(), 1, "evaluation continues past a missing include");
        let out = evaluate_source("use <nope.scad>\ncube(1);", &base);
        assert!(out.warnings.iter().any(|w| w.contains("Can't open library")));
    }

    #[test]
    fn text_renders_glyphs_as_2d_geometry() {
        // A letter becomes a 2D shape with real filled area.
        let out = run("text(\"A\", size = 20);");
        assert_eq!(out.shapes.len(), 1);
        assert!(out.shapes[0].outline.is_some(), "text output is 2D");
        assert!(flat_area(&out) > 0.0, "'A' has ink");
        // A wider string covers more area than one glyph.
        let one = flat_area(&run("text(\"I\", size = 20);"));
        let many = flat_area(&run("text(\"IIII\", size = 20);"));
        assert!(many > one * 2.0, "four glyphs cover more than one: {} vs {}", many, one);
        // Empty string → empty, no warning.
        let out = run("text(\"\");");
        assert!(out.shapes.is_empty() && out.warnings.is_empty());
        // A non-string argument warns and yields nothing (no auto-str()).
        let out = run("text(42);");
        assert!(out.shapes.is_empty() && out.warnings.iter().any(|w| w.contains("must be a string")));
        // text() extrudes to 3D (the standard label idiom).
        let out = run("linear_extrude(height = 3) text(\"Hi\", size = 15);");
        assert!(out.shapes[0].outline.is_none() && total_volume(&out) > 0.0, "extruded label");
        // halign=center shifts the run left of the origin (its bbox straddles x=0).
        let c = run("text(\"WWWW\", size = 20, halign = \"center\");");
        let xs: Vec<f64> = c.shapes[0].mesh.positions.iter().map(|p| p[0]).collect();
        let (lo, hi) = (xs.iter().cloned().fold(f64::INFINITY, f64::min),
                        xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
        assert!(lo < 0.0 && hi > 0.0, "centered text straddles x=0: {}..{}", lo, hi);
        // An unknown alignment warns and falls back.
        assert!(run("text(\"A\", halign = \"middle\");").warnings.iter().any(|w| w.contains("halign")));
    }

    #[test]
    fn render_and_surface() {
        // render() bakes its children into ONE exact-union leaf (unlike
        // union/group's preview concatenation).
        let out = run("render() { cube(2, center=true); translate([1,0,0]) cube(2, center=true); }");
        assert_eq!(out.shapes.len(), 1, "render bakes one leaf");
        // Two 2³ cubes overlapping half in X: 8 + 8 − 4 = 12.
        assert!((total_volume(&out) - 12.0).abs() < 1e-4, "render union vol {}", total_volume(&out));
        // The same children under group() stay two separate shapes.
        assert_eq!(run("group() { cube(2); translate([1,0,0]) cube(2); }").shapes.len(), 2);
        // render() accepts convexity silently.
        assert!(run("render(convexity = 4) cube(1);").warnings.is_empty());

        // surface() builds a 3D heightmap solid from a text grid file.
        let path = "scadforge_test_surface.dat";
        std::fs::write(path, "# hill\n0 0 0\n0 5 0\n0 0 0\n").unwrap();
        let out = run(&format!("surface(\"{}\");", path));
        std::fs::remove_file(path).ok();
        assert_eq!(out.shapes.len(), 1);
        assert!(out.shapes[0].outline.is_none(), "surface output is 3D");
        assert!(total_volume(&out) > 0.0, "the hill has volume");
        // A missing surface file warns and is non-fatal.
        assert!(
            run("surface(\"nope.dat\");").warnings.iter().any(|w| w.contains("Can't open")),
            "missing surface file warns"
        );
    }

    #[test]
    fn two_d_vector_io() {
        // SVG export of a 2D design.
        let svg = crate::http::handle("POST", "/export?format=svg", "square([10, 6]);");
        assert_eq!(svg.status, "200 OK");
        assert!(svg.body.contains("<svg") && svg.body.contains("fill-rule"));
        // DXF export of a 2D design.
        let dxf = crate::http::handle("POST", "/export?format=dxf", "circle(5, $fn = 32);");
        assert!(dxf.body.contains("LWPOLYLINE"));
        // PDF export of a 2D design: a well-formed one-page vector document.
        let pdf = crate::http::handle("POST", "/export?format=pdf", "square([10, 6]);");
        assert_eq!(pdf.status, "200 OK");
        assert_eq!(pdf.content_type, "application/pdf");
        assert!(pdf.body.starts_with("%PDF-1.4") && pdf.body.trim_end().ends_with("%%EOF"));
        // Exporting a 3D result to a 2D format is the reference error.
        let bad = crate::http::handle("POST", "/export?format=svg", "cube(3);");
        assert_eq!(bad.status, "422 Unprocessable Entity");
        assert!(bad.body.contains("not a 2D object"));
        // PDF of a 3D result is likewise the reference error.
        let bad_pdf = crate::http::handle("POST", "/export?format=pdf", "cube(3);");
        assert_eq!(bad_pdf.status, "422 Unprocessable Entity");
        // DXF round-trip: export a square to DXF, import it back, extrude.
        let path = "scadforge_test_rt.dxf";
        std::fs::write(path, crate::http::handle("POST", "/export?format=dxf", "square(10);").body)
            .unwrap();
        let out = run(&format!("linear_extrude(height = 2) import(\"{}\");", path));
        std::fs::remove_file(path).ok();
        assert!((total_volume(&out) - 200.0).abs() < 1e-3, "DXF round-trip vol {}", total_volume(&out));

        // Deprecated dxf_linear_extrude: extrude a DXF file directly, warning.
        let p2 = "scadforge_test_dxfle.dxf";
        std::fs::write(p2, crate::http::handle("POST", "/export?format=dxf", "square(8);").body)
            .unwrap();
        let out = run(&format!("dxf_linear_extrude(file = \"{}\", height = 5);", p2));
        std::fs::remove_file(p2).ok();
        assert!(out.warnings.iter().any(|w| w.contains("DEPRECATED") && w.contains("dxf_linear_extrude")));
        assert!((total_volume(&out) - 8.0 * 8.0 * 5.0).abs() < 1e-2, "dxf_linear_extrude vol {}", total_volume(&out));

        // SVG round-trip: export a square to SVG, import it back, extrude. The
        // reader is the exact inverse of the writer, so the volume is preserved.
        let ps = "scadforge_test_rt.svg";
        std::fs::write(ps, crate::http::handle("POST", "/export?format=svg", "square(10);").body)
            .unwrap();
        let out = run(&format!("linear_extrude(height = 2) import(\"{}\");", ps));
        std::fs::remove_file(ps).ok();
        assert!((total_volume(&out) - 200.0).abs() < 1e-3, "SVG round-trip vol {}", total_volume(&out));
    }

    #[test]
    fn modifier_characters() {
        // Signed volume of the SOLID (non-ghost) shapes only.
        fn solid_vol(out: &EvalOutput) -> f64 {
            let mut v = 0.0;
            for s in out.shapes.iter().filter(|s| !s.background) {
                for t in &s.mesh.tris {
                    let a = s.mesh.positions[t[0] as usize];
                    let b = s.mesh.positions[t[1] as usize];
                    let c = s.mesh.positions[t[2] as usize];
                    v += (a[0] * (b[1] * c[2] - b[2] * c[1]) + a[1] * (b[2] * c[0] - b[0] * c[2])
                        + a[2] * (b[0] * c[1] - b[1] * c[0]))
                        / 6.0;
                }
            }
            v.abs()
        }

        // `*` disable: the subtree is DROPPED and NOT instantiated — its
        // echo/assert side effects never fire (short-circuit, not filter).
        assert!((solid_vol(&run("*cube(10); cube(2);")) - 8.0).abs() < 1e-9);
        assert!(run("*echo(\"x\"); cube(1);").echoes.is_empty(), "* short-circuits echo");
        let out = run("*assert(false); cube(1);");
        assert!(out.error.is_none(), "* short-circuits a failing assert");

        // `!` root: siblings are pruned; ancestor transforms still apply.
        let out = run("!cube(2); cube(10);");
        assert!((solid_vol(&out) - 8.0).abs() < 1e-9, "only the rooted cube shows");
        assert!(out.shapes.iter().all(|s| s.rooted));
        let out = run("translate([100, 0, 0]) !cube(2); cube(10);");
        assert!(
            !out.shapes.is_empty() && out.shapes.iter().all(|s| s.mesh.positions.iter().all(|p| p[0] >= 99.999)),
            "ancestor translate still applies to the rooted subtree"
        );
        // `!` on a difference child discards the boolean, shows the raw child.
        let out = run("difference() { cube(10, center=true); !cube(2, center=true); }");
        assert!((solid_vol(&out) - 8.0).abs() < 1e-6, "boolean discarded, raw child: {}", solid_vol(&out));

        // `#` highlight: geometric NO-OP plus a pink ghost overlay.
        let out = run("#cube(2);");
        assert!((solid_vol(&out) - 8.0).abs() < 1e-9, "# keeps the solid geometry");
        assert!(out.shapes.iter().any(|s| s.highlight), "# adds a highlight overlay");
        // A #-marked cutter STILL cuts (4³ − 2³ = 56) and leaves a ghost.
        let out = run("difference() { cube(4, center=true); #cube(2, center=true); }");
        assert!((solid_vol(&out) - 56.0).abs() < 1e-4, "# cutter still cuts: {}", solid_vol(&out));
        assert!(out.shapes.iter().any(|s| s.highlight), "# cutter leaves a ghost");

        // `%` background: excluded from the CSG result, still shown as a ghost.
        let out = run("difference() { cube(4, center=true); %cube(2, center=true); }");
        assert!((solid_vol(&out) - 64.0).abs() < 1e-6, "% cutter does NOT cut: {}", solid_vol(&out));
        assert!(out.shapes.iter().any(|s| s.background), "% shows a ghost");
        // %-first-child promotes the next child to minuend (can't empty it).
        let out = run("difference() { %cube(2, center=true); cube(4, center=true); }");
        assert!((solid_vol(&out) - 64.0).abs() < 1e-6, "%-first promotes minuend: {}", solid_vol(&out));
        // %-child of intersection can't annihilate it.
        let out = run("intersection() { cube(4, center=true); %cube(20, center=true); }");
        assert!((solid_vol(&out) - 64.0).abs() < 1e-6, "% can't annihilate intersection: {}", solid_vol(&out));

        // Modifiers stack; `*` dominates (the subtree is simply gone).
        assert!((solid_vol(&run("*#cube(5); cube(1);")) - 1.0).abs() < 1e-9, "* over # → gone");
        assert!((solid_vol(&run("*!cube(9); cube(2);")) - 8.0).abs() < 1e-9, "* over ! → gone");

        // A modifier must prefix an instantiation — not a def/assign/naked block.
        assert!(parse("*x = 5;").is_err(), "modifier before assignment is a parse error");
        assert!(parse("#{ cube(1); }").is_err(), "modifier before a naked block is a parse error");
        assert!(parse("%module m() { cube(1); }").is_err(), "modifier before a def is a parse error");
        // But a modifier before for/if is fine.
        assert!(parse("*for (i = [0:3]) cube(1);").is_ok());
        assert!(parse("!if (true) cube(1);").is_ok());
    }

    #[test]
    fn phase4_3d_transforms_and_polyhedron() {
        // mirror across X: a box at +X lands at -X (winding stays outward,
        // so signed volume stays positive).
        let out = run("mirror([1,0,0]) translate([5,0,0]) cube(2);");
        assert_eq!(out.shapes.len(), 1);
        assert!(out.shapes[0].mesh.positions.iter().all(|p| p[0] <= 0.001));
        assert!(total_volume(&out) > 0.0, "mirror must keep outward winding");
        // Two nested mirrors with the same normal are identity.
        let a = run("mirror([1,0,0]) mirror([1,0,0]) translate([5,0,0]) cube(1);");
        assert!(a.shapes[0].mesh.positions.iter().all(|p| p[0] >= 4.999));
        // zero normal → warn + identity (children pass through).
        let z = run("mirror([0,0,0]) cube(1);");
        assert_eq!(z.shapes.len(), 1);
        assert!(z.warnings.iter().any(|w| w.contains("mirror")));

        // multmatrix as a plain translate (translation in last column).
        let out = run("multmatrix([[1,0,0,10],[0,1,0,0],[0,0,1,0],[0,0,0,1]]) cube(2);");
        assert!(out.shapes[0].mesh.positions.iter().all(|p| p[0] >= 9.999));
        // 3-row form is accepted (bottom row implied); a bogus 4th "perspective"
        // row must have no effect.
        let three = run("multmatrix([[2,0,0,0],[0,1,0,0],[0,0,1,0]]) cube(1);");
        assert!(three.shapes[0].mesh.positions.iter().any(|p| (p[0] - 2.0).abs() < 1e-9));

        // resize to a target bounding box.
        let out = run("resize([10,0,0]) cube(2);"); // x:2→10, y/z unchanged
        let (lo, hi) = geom::bounds(&out.shapes[0].mesh).unwrap();
        assert!((hi[0] - lo[0] - 10.0).abs() < 1e-9);
        assert!((hi[1] - lo[1] - 2.0).abs() < 1e-9);
        // auto shares the first specified factor into zero axes.
        let out = run("resize([10,0,0], auto=true) cube(2);");
        let (lo, hi) = geom::bounds(&out.shapes[0].mesh).unwrap();
        assert!((hi[1] - lo[1] - 10.0).abs() < 1e-6, "auto should scale y by 5x too");

        // polyhedron: a unit tetrahedron built by hand.
        let tet = run(
            "polyhedron(points=[[0,0,0],[1,0,0],[0,1,0],[0,0,1]], \
             faces=[[0,1,2],[0,3,1],[0,2,3],[1,3,2]]);",
        );
        assert_eq!(tet.shapes.len(), 1);
        assert!((total_volume(&tet) - 1.0 / 6.0).abs() < 1e-9, "vol {}", total_volume(&tet));
        // Out-of-bounds face index drops that face with a warning.
        let bad = run("polyhedron(points=[[0,0,0],[1,0,0],[0,1,0]], faces=[[0,1,2],[0,1,9]]);");
        assert!(bad.warnings.iter().any(|w| w.contains("out of bounds")));
    }

    #[test]
    fn intersection_for_folds_with_intersection() {
        // Two rotated long bars: intersection_for keeps only their common
        // core, where a plain for() would union them.
        let common = run(
            "intersection_for (a = [0, 90]) rotate([0,0,a]) cube([10,2,2], center=true);",
        );
        let unioned = run("for (a = [0, 90]) rotate([0,0,a]) cube([10,2,2], center=true);");
        assert_eq!(common.shapes.len(), 1);
        assert!(total_volume(&common) < total_volume(&unioned));
        // The common core is the 2x2x2 overlap at the center.
        assert!((total_volume(&common) - 8.0).abs() < 1e-4, "vol {}", total_volume(&common));
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
    fn function_value_scope_cycles_are_broken_after_evaluation() {
        // f's closure captures the scope that stores f — an Rc cycle
        // unless evaluation breaks it on the way out.
        let prog = parse(
            "f = function (x) x; g = [function () 1, 2]; y = f(2) + g[1];",
        )
        .unwrap();
        let mut ctx = Ctx {
            out: EvalOutput::default(),
            dynv: DynScope::root(),
            mod_stack: Vec::new(),
            children: None,
            fn_depth: 0,
            cycle_scopes: Vec::new(),
            csg: None,
        };
        let root = Scope::root();
        let weak = Rc::downgrade(&root);
        let _ = exec_scope(&prog, &root, &mut ctx);
        assert!(!ctx.cycle_scopes.is_empty(), "function stores must register");
        for s in ctx.cycle_scopes.drain(..) {
            s.vars.borrow_mut().clear();
        }
        drop(root);
        drop(ctx);
        assert!(weak.upgrade().is_none(), "the scope graph leaked");
    }

    #[test]
    fn tco_is_transparent_to_dynamic_scoping() {
        // Tail-call elimination must not drop the $-bindings of the
        // frames it collapses.
        // (1) plain forwarding: a $-arg to the wrapper reaches the leaf,
        // even though each body is a bare tail call.
        let out = run(
            "function leaf() = $x; function mid() = leaf(); function f() = mid(); echo(f($x = 9));",
        );
        assert_eq!(out.echoes, vec!["ECHO: 9"]);
        // (2) let($q) established in tail position is visible in the call
        // it wraps.
        let out = run("function f(n) = n <= 0 ? $q : let($q = n) f(n - 1); echo(f(1));");
        assert_eq!(out.echoes, vec!["ECHO: 1"]);
        // (3) a $-arg on the first hop survives every later tail hop.
        let out = run("function g(n) = n <= 0 ? $x : g(n - 1); echo(g(3, $x = 9));");
        assert_eq!(out.echoes, vec!["ECHO: 9"]);
        // ...and a plain deep tail loop still runs (O(1) dynamic chain).
        let out = run(
            "function c(n, a = 0) = n <= 0 ? a : c(n - 1, a + 1); echo(c(200000));",
        );
        assert!(out.error.is_none(), "{:?}", out.error);
        assert_eq!(out.echoes, vec!["ECHO: 200000"]);
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

    // -- panel regressions --------------------------------------------------

    #[test]
    fn children_forwarding_does_not_infinitely_recurse() {
        // A children() inside a stamped block refers to the CALLER's
        // children (the wrapper idiom), not its own block — must not
        // stack-overflow the process.
        let out = run(
            "module inner() { children(); } \
             module outer() { inner() children(); } \
             outer() cube(1);",
        );
        assert!(out.error.is_none(), "{:?}", out.error);
        assert_eq!(out.shapes.len(), 1);
        // A children() reaching past the top-level block warns, not crashes.
        let out = run("module m() { children(); } m() children();");
        assert!(out.error.is_none(), "{:?}", out.error);
        assert!(out.warnings.iter().any(|w| w.contains("outside a module")));
    }

    #[test]
    fn parser_guards_deep_statement_nesting() {
        let src = "if (true) ".repeat(100_000) + "cube(1);";
        let err = crate::parser::parse(&src).unwrap_err();
        assert!(err.contains("nesting"), "got: {}", err);
        let src = "translate([0,0,0]) ".repeat(100_000) + "cube(1);";
        assert!(crate::parser::parse(&src).is_err());
    }

    #[test]
    fn reserved_words_are_not_identifiers() {
        for bad in [
            "echo = 1;",
            "assert = 1;",
            "module echo() cube(1);",
            "function assert(x) = x;",
            "let (each = 1) cube(1);",
            "x = if(true);",
            "y = each;",
        ] {
            assert!(crate::parser::parse(bad).is_err(), "must reject: {}", bad);
        }
        // assign is NOT reserved.
        assert!(crate::parser::parse("assign = 1;").is_ok());
        // Statement echo/assert calls still work.
        let out = run("echo(\"hi\"); assert(true) cube(1);");
        assert_eq!(out.echoes, vec!["ECHO: \"hi\""]);
        assert_eq!(out.shapes.len(), 1);
    }

    #[test]
    fn parenthesized_callee_takes_the_value_path() {
        // (f) forces variable-namespace resolution: the function VALUE
        // wins over the same-named function definition.
        let out = run("function f(x) = 1; f = function (x) 2; y = (f)(0); echo(y);");
        assert_eq!(out.echoes, vec!["ECHO: 2"]);
        // A bare f(0) still hits the function namespace.
        let out = run("function f(x) = 1; f = function (x) 2; echo(f(0));");
        assert_eq!(out.echoes, vec!["ECHO: 1"]);
    }

    #[test]
    fn user_functions_shadow_is_undef_consistently() {
        // Same result in tail and non-tail position.
        let out = run(
            "function is_undef(x) = 99; a = undef; \
             function g(z) = is_undef(z); \
             echo(is_undef(a), g(a));",
        );
        assert_eq!(out.echoes, vec!["ECHO: 99, 99"]);
        // Without a shadow, the defined-guard still suppresses the warning.
        let (v, w) = ev("is_undef(nothing_here)");
        assert_eq!(v, Value::Bool(true));
        assert!(w.is_empty(), "{:?}", w);
    }

    #[test]
    fn cstyle_generator_dollar_bindings_do_not_leak() {
        let out = run(
            "x = [for ($i = 0; $i < 3; $i = $i + 1) $i]; echo(x, is_undef($i));",
        );
        assert_eq!(out.echoes, vec!["ECHO: [0, 1, 2], true"]);
    }

    #[test]
    fn range_iteration_matches_the_count_formula() {
        // No stray epsilon element: [0:0.1:0.3] is 3 elements, agreeing
        // with range_count (used by comparison).
        assert_eq!(n("len([for (i = [0:0.1:0.3]) i])"), 3.0);
        assert_eq!(n("len([each [1:0.5:3]])"), 5.0);
        assert_eq!(n("len([each [0:1000000]])"), 1000001.0); // contract bar
    }

    #[test]
    fn chr_rejects_out_of_range_code_points() {
        assert_eq!(s("chr(4294967361)"), ""); // 2^32 + 65 must NOT be "A"
        assert_eq!(s("chr(1114112)"), ""); // just past U+10FFFF
        assert_eq!(s("chr(1114111)"), "\u{10FFFF}");
    }

    #[test]
    fn builtin_functions_accept_named_arguments() {
        // search mode by name.
        assert_eq!(
            ev("search(\"a\", \"aaaa\", num_returns_per_match = 0)").0,
            ev("[[0, 1, 2, 3]]").0
        );
        // rands seed by name is reproducible and matches positional.
        let out = run(
            "a = rands(0, 1, 3, seed_value = 7); b = rands(0, 1, 3, 7); echo(a == b);",
        );
        assert_eq!(out.echoes, vec!["ECHO: true"]);
        // atan2 by name.
        assert_eq!(n("atan2(y = 1, x = 1)"), 45.0);
        // lookup by name.
        assert_eq!(n("lookup(key = 0.25, table = [[0, 0], [0.5, 1]])"), 0.5);
        // Unknown named arg warns and is dropped, the call proceeds.
        let (v, w) = ev("lookup(0.5, [[0, 0], [1, 1]], bogus = 9)");
        assert_eq!(v, Value::Num(0.5));
        assert!(w.iter().any(|m| m.contains("bogus")));
    }

    #[test]
    fn function_value_renders_without_outer_parens() {
        assert_eq!(s("str(function (x, y = 1) x + y)"), "function(x, y = 1) x + y");
        // assert condition text keeps its disambiguating parens.
        let out = run("assert(1 + 1 == 3);");
        assert!(out.error.unwrap().contains("((1 + 1) == 3)"));
    }

    #[test]
    fn higher_order_tail_fold_stays_bounded() {
        // The callback is passed as a param but its closure is defined at
        // top level, so it closes no cycle with the per-iteration param
        // scope — those scopes must not be retained.
        let out = run(
            "id = function (x) x; \
             function fold(f, n, acc) = n <= 0 ? acc : fold(f, n - 1, f(acc) + 1); \
             echo(fold(id, 300000, 0));",
        );
        assert!(out.error.is_none(), "{:?}", out.error);
        assert_eq!(out.echoes, vec!["ECHO: 300000"]);
    }
}
