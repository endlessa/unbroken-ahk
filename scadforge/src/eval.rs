//! Evaluator: AST → colored meshes.
//!
//! The slice covers cube/sphere/cylinder, translate/rotate/scale, color,
//! union/group, assignments, `for` over ranges and vectors, and the
//! $fn/$fa/$fs resolution rules from the reference. Union is mesh
//! concatenation for preview purposes — real CSG booleans are the next
//! kernel phase, and difference()/intersection() say so loudly instead of
//! rendering something wrong.

use crate::ast::{Arg, BinOp, Expr, Stmt};
use crate::geom::{self, Mesh};
use crate::value::Value;
use std::collections::HashMap;

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
}

pub fn evaluate(program: &[Stmt]) -> EvalOutput {
    let mut out = EvalOutput::default();
    let mut env = Env::root();
    eval_stmts(program, &mut env, &mut out);
    out
}

#[derive(Clone)]
struct Env {
    vars: HashMap<String, Value>,
}

impl Env {
    fn root() -> Env {
        let mut vars = HashMap::new();
        // Reference defaults: $fn=0 (disabled), $fa=12°, $fs=2 units.
        vars.insert("$fn".into(), Value::Num(0.0));
        vars.insert("$fa".into(), Value::Num(12.0));
        vars.insert("$fs".into(), Value::Num(2.0));
        // PI is a builtin constant (a variable, not a function).
        vars.insert("PI".into(), Value::Num(std::f64::consts::PI));
        Env { vars }
    }
}

fn eval_stmts(stmts: &[Stmt], env: &mut Env, out: &mut EvalOutput) {
    for stmt in stmts {
        match stmt {
            Stmt::Assign { name, value } => {
                let v = eval_expr(value, env, out);
                env.vars.insert(name.clone(), v);
            }
            Stmt::For { var, iter, body } => {
                let iter_v = eval_expr(iter, env, out);
                for item in iterate(&iter_v, out) {
                    // Loop variables scope to the body: a child scope that
                    // inherits everything and leaks nothing.
                    let mut child = env.clone();
                    child.vars.insert(var.clone(), item);
                    eval_stmts(body, &mut child, out);
                }
            }
            Stmt::Call { .. } => {
                let shapes = eval_call(stmt, env, out);
                out.shapes.extend(shapes);
            }
        }
    }
}

fn iterate(v: &Value, out: &mut EvalOutput) -> Vec<Value> {
    match v {
        Value::Vector(items) => items.clone(),
        Value::Range { start, step, end, implicit_step } => {
            let mut items = Vec::new();
            if *step == 0.0 || !step.is_finite() {
                out.warnings.push("for: range step must be a nonzero finite number".into());
                return items;
            }
            // Legacy two-part reversed range [10:1]: DEPRECATED, bounds
            // swap and iteration ascends. The explicit-step form does
            // NOT swap — [10:1:0] simply yields zero iterations.
            let (start, end) = if *implicit_step && start > end {
                out.warnings.push(
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
                if items.len() > 100_000 {
                    out.warnings.push("for: range truncated at 100000 iterations".into());
                    break;
                }
                x += *step;
            }
            items
        }
        other => {
            out.warnings.push(format!("for: cannot iterate over a {}", other.type_name()));
            Vec::new()
        }
    }
}

/// Evaluate one Call statement into shapes (children already applied).
fn eval_call(stmt: &Stmt, env: &Env, out: &mut EvalOutput) -> Vec<Shape> {
    let (name, args, children) = match stmt {
        Stmt::Call { name, args, children } => (name.as_str(), args, children),
        Stmt::For { var, iter, body } => {
            // A `for` in child position contributes its iterations' shapes.
            let mut shapes = Vec::new();
            let mut scratch = EvalOutput::default();
            let iter_v = eval_expr(iter, &mut env.clone(), &mut scratch);
            for item in iterate(&iter_v, &mut scratch) {
                let mut child_env = env.clone();
                child_env.vars.insert(var.clone(), item);
                shapes.extend(eval_children(body, &child_env, &mut scratch));
            }
            out.warnings.extend(scratch.warnings);
            return shapes;
        }
        Stmt::Assign { .. } => return Vec::new(),
    };

    let bound = match bind_args(name, args, env, out) {
        Some(b) => b,
        None => return Vec::new(),
    };

    match name {
        "cube" => {
            let size = match bound.get("size") {
                Some(Value::Num(s)) => [*s, *s, *s],
                Some(v @ Value::Vector(_)) => match v.as_vec3() {
                    Some(s) => s,
                    None => {
                        out.warnings.push("cube: size must be a number or [x, y, z]".into());
                        return Vec::new();
                    }
                },
                Some(Value::Undef) | None => [1.0, 1.0, 1.0],
                Some(other) => {
                    out.warnings
                        .push(format!("cube: size must be a number or vector, got {}", other.type_name()));
                    return Vec::new();
                }
            };
            let center = bound.get("center").and_then(Value::as_bool).unwrap_or(false);
            leaf(geom::cube(size, center))
        }
        "sphere" => {
            // d overrides r per the reference.
            let r = match (bound.get("d"), bound.get("r")) {
                (Some(Value::Num(d)), _) => d / 2.0,
                (_, Some(Value::Num(r))) => *r,
                _ => 1.0,
            };
            let n = resolve_fragments(r, &bound, env);
            leaf(geom::sphere(r, n))
        }
        "cylinder" => {
            let num = |key: &str| bound.get(key).and_then(Value::as_num);
            let r_both = num("d").map(|d| d / 2.0).or_else(|| num("r"));
            let r1 = num("d1").map(|d| d / 2.0).or_else(|| num("r1")).or(r_both).unwrap_or(1.0);
            let r2 = num("d2").map(|d| d / 2.0).or_else(|| num("r2")).or(r_both).unwrap_or(1.0);
            let h = num("h").unwrap_or(1.0);
            let center = bound.get("center").and_then(Value::as_bool).unwrap_or(false);
            let n = resolve_fragments(r1.max(r2), &bound, env);
            leaf(geom::cylinder(h, r1, r2, center, n))
        }
        "translate" => {
            let matrix = match bound.get("v").and_then(Value::as_vec3) {
                Some(v) => Some(geom::translation(v)),
                None => {
                    out.warnings.push("translate: v must be a vector like [x, y, z]".into());
                    None
                }
            };
            transform_children(children, env, out, matrix)
        }
        "scale" => {
            let matrix = match bound.get("v") {
                Some(Value::Num(s)) => Some(geom::scaling([*s, *s, *s])),
                Some(v @ Value::Vector(_)) => v.as_vec3().map(geom::scaling),
                _ => {
                    out.warnings.push("scale: v must be a number or vector".into());
                    None
                }
            };
            transform_children(children, env, out, matrix)
        }
        "rotate" => {
            let matrix = match (bound.get("a"), bound.get("v")) {
                (Some(Value::Num(deg)), Some(axis @ Value::Vector(_))) => match axis.as_vec3() {
                    Some(axis) => Some(geom::rotation_axis(*deg, axis)),
                    None => {
                        out.warnings.push("rotate: v must be a numeric vector".into());
                        None
                    }
                },
                (Some(Value::Num(deg)), _) => Some(geom::rotation_xyz([0.0, 0.0, *deg])),
                (Some(vec @ Value::Vector(_)), _) => match vec.as_vec3() {
                    Some(deg) => Some(geom::rotation_xyz(deg)),
                    None => {
                        out.warnings.push("rotate: a must be a scalar or [x, y, z] degrees".into());
                        None
                    }
                },
                _ => {
                    out.warnings.push("rotate: missing angle".into());
                    None
                }
            };
            transform_children(children, env, out, matrix)
        }
        "color" => {
            let rgba = parse_color(bound.get("c"), bound.get("alpha"), out);
            let mut shapes = eval_children(children, env, out);
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
        "union" | "group" => eval_children(children, env, out),
        "difference" | "intersection" | "hull" | "minkowski" => {
            out.warnings.push(format!(
                "{}() is not implemented in this slice yet (CSG booleans are \
                 the next kernel phase) — children are shown un-combined",
                name
            ));
            eval_children(children, env, out)
        }
        other => {
            out.warnings.push(format!(
                "unknown module '{}' — this slice supports cube, sphere, cylinder, \
                 translate, rotate, scale, color, union, for",
                other
            ));
            Vec::new()
        }
    }
}

fn leaf(mesh: Mesh) -> Vec<Shape> {
    if mesh.positions.is_empty() {
        Vec::new()
    } else {
        vec![Shape { mesh, color: None }]
    }
}

fn eval_children(children: &[Stmt], env: &Env, out: &mut EvalOutput) -> Vec<Shape> {
    let mut env = env.clone();
    let mut shapes = Vec::new();
    for child in children {
        match child {
            Stmt::Assign { name, value } => {
                let v = eval_expr(value, &mut env, out);
                env.vars.insert(name.clone(), v);
            }
            _ => shapes.extend(eval_call(child, &env, out)),
        }
    }
    shapes
}

fn transform_children(
    children: &[Stmt],
    env: &Env,
    out: &mut EvalOutput,
    matrix: Option<geom::Mat4>,
) -> Vec<Shape> {
    let mut shapes = eval_children(children, env, out);
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

/// Positional-parameter names per module, per the reference signatures.
fn positional_names(module: &str) -> &'static [&'static str] {
    match module {
        "cube" => &["size", "center"],
        "sphere" => &["r"],
        "cylinder" => &["h", "r1", "r2", "center"],
        "translate" | "scale" => &["v"],
        "rotate" => &["a", "v"],
        "color" => &["c", "alpha"],
        _ => &[],
    }
}

fn bind_args(
    module: &str,
    args: &[Arg],
    env: &Env,
    out: &mut EvalOutput,
) -> Option<HashMap<String, Value>> {
    let names = positional_names(module);
    let mut bound = HashMap::new();
    let mut pos = 0;
    let mut scratch_env = env.clone();
    for arg in args {
        let value = eval_expr(&arg.value, &mut scratch_env, out);
        match &arg.name {
            Some(name) => {
                bound.insert(name.clone(), value);
            }
            None => {
                match names.get(pos) {
                    Some(name) => {
                        bound.insert((*name).to_string(), value);
                    }
                    None => out.warnings.push(format!(
                        "{}: too many positional arguments (expected at most {})",
                        module,
                        names.len()
                    )),
                }
                pos += 1;
            }
        }
    }
    Some(bound)
}

/// $fn/$fa/$fs: a per-call named argument wins over the scoped variable,
/// which was seeded with the reference defaults.
fn resolve_fragments(r: f64, bound: &HashMap<String, Value>, env: &Env) -> u32 {
    let get = |key: &str, default: f64| {
        bound
            .get(key)
            .and_then(Value::as_num)
            .or_else(|| env.vars.get(key).and_then(Value::as_num))
            .unwrap_or(default)
    };
    geom::fragments(r, get("$fn", 0.0), get("$fa", 12.0), get("$fs", 2.0))
}

fn parse_color(
    c: Option<&Value>,
    alpha: Option<&Value>,
    out: &mut EvalOutput,
) -> Option<[f64; 4]> {
    let mut rgba = match c {
        Some(Value::Vector(items)) if items.len() == 3 || items.len() == 4 => {
            let mut v = [0.0, 0.0, 0.0, 1.0];
            for (i, item) in items.iter().enumerate() {
                v[i] = item.as_num()?;
            }
            v
        }
        Some(Value::Str(name)) => named_color(name).or_else(|| hex_color(name)).or_else(|| {
            out.warnings.push(format!("color: unknown color '{}'", name));
            None
        })?,
        _ => {
            out.warnings.push("color: expected a name, \"#hex\", or [r, g, b(, a)]".into());
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

fn eval_expr(expr: &Expr, env: &mut Env, out: &mut EvalOutput) -> Value {
    match expr {
        Expr::Num(n) => Value::Num(*n),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Str(s) => Value::Str(s.clone()),
        Expr::Undef => Value::Undef,
        Expr::Ident(name) => match env.vars.get(name) {
            Some(v) => v.clone(),
            None => {
                out.warnings.push(format!("unknown variable '{}' (undef)", name));
                Value::Undef
            }
        },
        Expr::Vector(items) => {
            Value::Vector(items.iter().map(|e| eval_expr(e, env, out)).collect())
        }
        Expr::Range { start, step, end } => {
            let implicit_step = step.is_none();
            let s = eval_expr(start, env, out).as_num();
            let st = match step {
                Some(e) => eval_expr(e, env, out).as_num(),
                None => Some(1.0),
            };
            let e = eval_expr(end, env, out).as_num();
            match (s, st, e) {
                (Some(s), Some(st), Some(e)) => {
                    Value::Range { start: s, step: st, end: e, implicit_step }
                }
                _ => {
                    out.warnings.push("range bounds must be numbers".into());
                    Value::Undef
                }
            }
        }
        Expr::Neg(inner) => {
            let v = eval_expr(inner, env, out);
            negate(v, out)
        }
        Expr::Pos(inner) => match eval_expr(inner, env, out) {
            Value::Num(n) => Value::Num(n),
            other => {
                out.warnings.push(format!("undefined operation (+ {})", other.type_name()));
                Value::Undef
            }
        },
        Expr::Not(inner) => {
            // Never undef: strictly boolean, even for undef/nan operands.
            let v = eval_expr(inner, env, out);
            Value::Bool(!truthy(&v))
        }
        Expr::Ternary { cond, then, els } => {
            // Only the taken branch evaluates — the laziness recursive
            // library code depends on (undef and nan conds included:
            // undef is falsy, nan is TRUTHY).
            let c = eval_expr(cond, env, out);
            if truthy(&c) {
                eval_expr(then, env, out)
            } else {
                eval_expr(els, env, out)
            }
        }
        Expr::Index { base, index } => {
            let b = eval_expr(base, env, out);
            let i = eval_expr(index, env, out);
            index_value(&b, &i)
        }
        Expr::Member { base, name } => {
            let b = eval_expr(base, env, out);
            member_value(&b, name)
        }
        Expr::Call { name, args } => call_builtin(name, args, env, out),
        Expr::Binary { op, lhs, rhs } => match op {
            // && and || short-circuit and return strict booleans —
            // never the operand value.
            BinOp::And => {
                let l = eval_expr(lhs, env, out);
                if !truthy(&l) {
                    Value::Bool(false)
                } else {
                    let r = eval_expr(rhs, env, out);
                    Value::Bool(truthy(&r))
                }
            }
            BinOp::Or => {
                let l = eval_expr(lhs, env, out);
                if truthy(&l) {
                    Value::Bool(true)
                } else {
                    let r = eval_expr(rhs, env, out);
                    Value::Bool(truthy(&r))
                }
            }
            _ => {
                let l = eval_expr(lhs, env, out);
                let r = eval_expr(rhs, env, out);
                binary_op(*op, l, r, out)
            }
        },
    }
}

// -- Operator semantics (per the language reference) ------------------------

/// Exactly five falsy values: false, 0/-0, "", [], undef. Everything else
/// is truthy — including nan (the test is a plain != 0), "false", [0],
/// and ranges.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Num(n) => *n != 0.0, // nan != 0 is true → truthy
        Value::Str(s) => !s.is_empty(),
        Value::Vector(items) => !items.is_empty(),
        Value::Range { .. } => true,
        Value::Undef => false,
    }
}

/// Deep structural equality, total over all value pairs. Numbers by IEEE
/// (nan never equal, even to itself), cross-type simply unequal,
/// undef == undef true, ranges by begin/step/end.
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

fn negate(v: Value, out: &mut EvalOutput) -> Value {
    match v {
        Value::Num(n) => Value::Num(-n),
        // Recursive so -matrix works.
        Value::Vector(items) => {
            Value::Vector(items.into_iter().map(|item| negate(item, out)).collect())
        }
        other => {
            out.warnings.push(format!("undefined operation (- {})", other.type_name()));
            Value::Undef
        }
    }
}

/// + and -: numbers, or equal-length vectors elementwise (recursive, so
/// matrix+matrix works). NO broadcasting: scalar+vector is undef.
fn add_sub(sub: bool, l: &Value, r: &Value, out: &mut EvalOutput) -> Value {
    match (l, r) {
        (Value::Num(a), Value::Num(b)) => Value::Num(if sub { a - b } else { a + b }),
        (Value::Vector(x), Value::Vector(y)) if x.len() == y.len() => Value::Vector(
            x.iter().zip(y).map(|(a, b)| add_sub(sub, a, b, out)).collect(),
        ),
        _ => {
            out.warnings.push(format!(
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
fn multiply(l: &Value, r: &Value, out: &mut EvalOutput) -> Value {
    fn scale(n: f64, v: &Value, out: &mut EvalOutput) -> Value {
        match v {
            Value::Num(m) => Value::Num(n * m),
            Value::Vector(items) => {
                Value::Vector(items.iter().map(|item| scale(n, item, out)).collect())
            }
            other => {
                out.warnings
                    .push(format!("undefined operation (number * {})", other.type_name()));
                Value::Undef
            }
        }
    }
    match (l, r) {
        (Value::Num(a), Value::Num(b)) => Value::Num(a * b),
        (Value::Num(n), v @ Value::Vector(_)) | (v @ Value::Vector(_), Value::Num(n)) => {
            scale(*n, v, out)
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
                    out.warnings
                        .push("undefined operation (vector * vector: shape mismatch)".into());
                    Value::Undef
                }
            }
        }
        _ => {
            out.warnings.push(format!(
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
fn divide(l: &Value, r: &Value, out: &mut EvalOutput) -> Value {
    match (l, r) {
        (Value::Num(a), Value::Num(b)) => {
            if *b == 0.0 {
                out.warnings.push("division by zero".into());
            }
            Value::Num(a / b)
        }
        (Value::Vector(items), Value::Num(_)) => Value::Vector(
            items.iter().map(|item| divide(item, r, out)).collect(),
        ),
        (Value::Num(_), Value::Vector(items)) => Value::Vector(
            items.iter().map(|item| divide(l, item, out)).collect(),
        ),
        _ => {
            out.warnings.push(format!(
                "undefined operation ({} / {})",
                l.type_name(),
                r.type_name()
            ));
            Value::Undef
        }
    }
}

fn binary_op(op: BinOp, l: Value, r: Value, out: &mut EvalOutput) -> Value {
    match op {
        BinOp::Add => add_sub(false, &l, &r, out),
        BinOp::Sub => add_sub(true, &l, &r, out),
        BinOp::Mul => multiply(&l, &r, out),
        BinOp::Div => divide(&l, &r, out),
        BinOp::Mod => match (l.as_num(), r.as_num()) {
            (Some(a), Some(b)) => Value::Num(a % b), // C fmod semantics
            _ => {
                out.warnings.push(format!(
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
                out.warnings.push(format!(
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
                out.warnings.push(msg);
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

fn call_builtin(name: &str, args: &[Arg], env: &Env, out: &mut EvalOutput) -> Value {
    // Builtins bind positionally; named arguments other than $-vars are
    // not meaningful to them in this slice.
    let mut scratch = env.clone();
    let vals: Vec<Value> = args
        .iter()
        .filter(|a| a.name.is_none())
        .map(|a| eval_expr(&a.value, &mut scratch, out))
        .collect();
    let num = |i: usize| -> Option<f64> { vals.get(i).and_then(Value::as_num) };
    let one_num = |out: &mut EvalOutput, f: &dyn Fn(f64) -> f64| -> Value {
        match num(0) {
            Some(x) => Value::Num(f(x)),
            None => {
                out.warnings.push(format!("{}: expected a number", name));
                Value::Undef
            }
        }
    };

    match name {
        "abs" => one_num(out, &f64::abs),
        // Comparison-based sign: 0 for -0 and (per the VERIFY note) nan.
        "sign" => one_num(out, &|x| {
            if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                0.0
            }
        }),
        "sin" => one_num(out, &sin_deg),
        "cos" => one_num(out, &cos_deg),
        "tan" => one_num(out, &|x| {
            let (s, c) = (sin_deg(x), cos_deg(x));
            s / c
        }),
        "asin" => one_num(out, &|x| x.asin().to_degrees()),
        "acos" => one_num(out, &|x| x.acos().to_degrees()),
        "atan" => one_num(out, &|x| x.atan().to_degrees()),
        "atan2" => match (num(0), num(1)) {
            (Some(y), Some(x)) => Value::Num(y.atan2(x).to_degrees()),
            _ => {
                out.warnings.push("atan2: expected two numbers (y, x)".into());
                Value::Undef
            }
        },
        "floor" => one_num(out, &f64::floor),
        "ceil" => one_num(out, &f64::ceil),
        // Ties round away from zero: round(2.5)==3, round(-2.5)==-3.
        "round" => one_num(out, &f64::round),
        "ln" => one_num(out, &f64::ln),
        "log" => one_num(out, &f64::log10), // base 10 — ln is natural
        "exp" => one_num(out, &f64::exp),
        "sqrt" => one_num(out, &f64::sqrt),
        "pow" => match (num(0), num(1)) {
            (Some(b), Some(e)) => Value::Num(b.powf(e)), // identical to ^
            _ => {
                out.warnings.push("pow: expected two numbers (base, exponent)".into());
                Value::Undef
            }
        },
        "min" | "max" => min_max(name, &vals, out),
        "norm" => match vals.first() {
            Some(Value::Vector(items)) => match numeric_vec(items) {
                // Naive sum of squares (empty vector → 0).
                Some(nums) => Value::Num(nums.iter().map(|x| x * x).sum::<f64>().sqrt()),
                None => {
                    out.warnings.push("norm: vector elements must be numbers".into());
                    Value::Undef
                }
            },
            _ => {
                out.warnings.push("norm: expected a vector".into());
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
                        out.warnings.push("cross: expected two numeric vectors of length 2 or 3".into());
                        Value::Undef
                    }
                }
            }
            _ => {
                out.warnings.push("cross: expected two vectors".into());
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
        "is_undef" => Value::Bool(matches!(vals.first(), Some(Value::Undef) | None)),
        // is_num carve-out: false for nan.
        "is_num" => Value::Bool(matches!(vals.first(), Some(Value::Num(n)) if !n.is_nan())),
        "is_bool" => Value::Bool(matches!(vals.first(), Some(Value::Bool(_)))),
        "is_string" => Value::Bool(matches!(vals.first(), Some(Value::Str(_)))),
        // Ranges are NOT lists.
        "is_list" => Value::Bool(matches!(vals.first(), Some(Value::Vector(_)))),
        // No function values or object values exist in this slice.
        "is_function" | "is_object" => Value::Bool(false),
        other => {
            out.warnings.push(format!("unknown function '{}'", other));
            Value::Undef
        }
    }
}

/// min/max: with 2+ arguments the extreme of the arguments; with exactly
/// one VECTOR argument the extreme element. Ordering is the generic
/// relational ordering (strings work). Empty vector or a single
/// non-vector argument is undef with a warning.
fn min_max(name: &str, vals: &[Value], out: &mut EvalOutput) -> Value {
    let items: Vec<Value> = match vals {
        [Value::Vector(items)] => items.clone(),
        _ if vals.len() >= 2 => vals.to_vec(),
        _ => {
            out.warnings.push(format!("{}: expected a vector or at least two arguments", name));
            return Value::Undef;
        }
    };
    if items.is_empty() {
        out.warnings.push(format!("{}: empty vector", name));
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
                out.warnings.push(format!("{}: {}", name, msg));
                return Value::Undef;
            }
        }
    }
    best
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn run(src: &str) -> EvalOutput {
        evaluate(&parse(src).unwrap())
    }

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

    /// Evaluate one expression with a fresh root env; returns (value,
    /// warnings).
    fn ev(expr_src: &str) -> (Value, Vec<String>) {
        let prog = parse(&format!("x = {};", expr_src)).unwrap();
        let value = match &prog[0] {
            Stmt::Assign { value, .. } => value.clone(),
            other => panic!("expected assign, got {:?}", other),
        };
        let mut env = Env::root();
        let mut out = EvalOutput::default();
        let v = eval_expr(&value, &mut env, &mut out);
        (v, out.warnings)
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

    #[test]
    fn precedence_corners_match_the_reference() {
        // ^ binds tighter than unary minus; exponent operand re-enters
        // unary; right-associative.
        assert_eq!(n("-2 ^ 2"), -4.0);
        assert_eq!(n("2 ^ -3"), 0.125);
        assert_eq!(n("2 ^ 3 ^ 2"), 512.0); // 2^(3^2)
        assert_eq!(n("0 ^ 0"), 1.0); // C pow convention
        // No chained comparisons: (1==1)==2 compares a bool to 2.
        assert!(!b("1 == 1 == 2"));
        // && binds tighter than ||.
        assert!(b("true || false && false"));
        // Postfix beats unary: -v[0].
        assert_eq!(n("-[5, 6][0]"), -5.0);
        // Ternary chains right.
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
        // nan is TRUTHY (the test is a plain != 0), so the ternary takes
        // the THEN branch and !(0/0) is false.
        let (v, w) = ev("(0/0) ? 1 : 2");
        assert_eq!(v, Value::Num(1.0));
        assert!(w.iter().all(|m| m.contains("division")), "{:?}", w);
        assert!(!b("!(0/0)"));
    }

    #[test]
    fn logic_short_circuits_and_returns_strict_booleans() {
        // The skipped operand is never evaluated: an unknown variable
        // there must produce no warning.
        let (v, w) = ev("false && unknown_var");
        assert_eq!(v, Value::Bool(false));
        assert!(w.is_empty(), "short-circuit must not evaluate rhs: {:?}", w);
        let (v, w) = ev("true || unknown_var");
        assert_eq!(v, Value::Bool(true));
        assert!(w.is_empty());
        // Result is boolean, never the operand value.
        let (v, _) = ev("5 && 7");
        assert_eq!(v, Value::Bool(true));
        // Ternary laziness: the untaken branch never evaluates.
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
        // nan is unequal to everything, including itself.
        assert!(!b("(0/0) == (0/0)"));
        assert!(b("(0/0) != (0/0)"));
        // Ranges compare by components; the implicit step is still 1.
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
        // nan relationals are FALSE (IEEE), not undef — no warning.
        let (v, w) = ev("(0/0) < 1");
        assert_eq!(v, Value::Bool(false));
        assert!(w.iter().all(|m| m.contains("division")), "{:?}", w);
        // Mixed types are an undefined operation: undef WITH a warning.
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
        // Strings index by code POINT, not byte.
        let (v, _) = ev("\"h\u{e4}x\"[1]");
        assert_eq!(v, Value::Str("\u{e4}".to_string()));
        // Ranges expose [0]/[1]/[2] = begin/step/end.
        assert_eq!(n("[2:3:11][1]"), 3.0);
        assert_eq!(n("[0:5][1]"), 1.0); // implicit step is 1
        // Chained indexing for matrices.
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
        // Vector + vector is elementwise; NO broadcasting for +/-.
        assert_eq!(ev("[1, 2] + [3, 4]").0, ev("[4, 6]").0);
        let (v, w) = ev("[1, 2] + 1");
        assert_eq!(v, Value::Undef);
        assert!(w.iter().any(|m| m.contains("undefined operation")));
        // * dispatches: scale, DOT product, matrix forms.
        assert_eq!(ev("2 * [3, 4]").0, ev("[6, 8]").0);
        assert_eq!(n("[1, 2] * [3, 4]"), 11.0); // dot product, not elementwise
        assert_eq!(ev("[[1, 0], [0, 2]] * [3, 4]").0, ev("[3, 8]").0);
        assert_eq!(ev("[[1, 2]] * [[3], [4]]").0, ev("[[11]]").0);
        // Division: both scalar directions elementwise, vector/vector undef.
        assert_eq!(ev("[10, 4] / 2").0, ev("[5, 2]").0);
        assert_eq!(ev("10 / [2, 5]").0, ev("[5, 2]").0);
        assert_eq!(ev("[1] / [1]").0, Value::Undef);
        // Unary minus recurses into matrices.
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
        // concat: one level, strings are NOT lists, non-vectors whole.
        assert_eq!(ev("concat([1], [2, 3], 4)").0, ev("[1, 2, 3, 4]").0);
        assert_eq!(ev("concat([[1]], [2])").0, ev("[[1], 2]").0);
        assert_eq!(ev("concat(\"ab\", \"cd\")").0, ev("[\"ab\", \"cd\"]").0);
        assert_eq!(ev("concat()").0, Value::Vector(vec![]));
        // round ties away from zero; sign of -0 is 0.
        assert_eq!(n("round(2.5)"), 3.0);
        assert_eq!(n("round(-2.5)"), -3.0);
        assert_eq!(n("sign(-0)"), 0.0);
        assert_eq!(n("ln(exp(1))"), 1.0);
        assert_eq!(n("log(1000)"), 3.0); // base 10
        assert_eq!(n("pow(0, 0)"), 1.0);
        assert!((n("PI") - std::f64::consts::PI).abs() < 1e-15);
        // Type tests, with the is_num nan carve-out.
        assert!(!b("is_num(0/0)"));
        assert!(b("is_num(5)"));
        assert!(!b("is_list([0:10])"));
        assert!(b("is_list([])"));
        assert!(b("is_undef(undef)"));
        // Unknown function: warning + undef, evaluation continues.
        let (v, w) = ev("frob(1)");
        assert_eq!(v, Value::Undef);
        assert!(w.iter().any(|m| m.contains("unknown function 'frob'")));
    }

    #[test]
    fn reversed_ranges_follow_the_legacy_rules() {
        // Two-part [4:1]: DEPRECATED, bounds swap, ascending iteration.
        let out = run("for (i = [4:1]) cube(i);");
        assert_eq!(out.shapes.len(), 4);
        assert!(out.warnings.iter().any(|w| w.contains("DEPRECATED")));
        // Explicit-step [10:1:0]: NO swap — zero iterations, no warning.
        let out = run("for (i = [10:1:0]) cube(i);");
        assert!(out.shapes.is_empty());
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
        // Negative step descends inclusively.
        let out = run("for (i = [10:-2:0]) cube(1);");
        assert_eq!(out.shapes.len(), 6);
    }

    #[test]
    fn expressions_drive_geometry() {
        // The point of the milestone: computed arguments end to end.
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
}
