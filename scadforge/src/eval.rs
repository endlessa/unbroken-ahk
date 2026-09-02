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
        Value::Range { start, step, end } => {
            let mut items = Vec::new();
            if *step == 0.0 || !step.is_finite() {
                out.warnings.push("for: range step must be a nonzero finite number".into());
                return items;
            }
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
            let s = eval_expr(start, env, out).as_num();
            let st = match step {
                Some(e) => eval_expr(e, env, out).as_num(),
                None => Some(1.0),
            };
            let e = eval_expr(end, env, out).as_num();
            match (s, st, e) {
                (Some(s), Some(st), Some(e)) => Value::Range { start: s, step: st, end: e },
                _ => {
                    out.warnings.push("range bounds must be numbers".into());
                    Value::Undef
                }
            }
        }
        Expr::Neg(inner) => match eval_expr(inner, env, out) {
            Value::Num(n) => Value::Num(-n),
            Value::Vector(items) => Value::Vector(
                items
                    .into_iter()
                    .map(|v| match v {
                        Value::Num(n) => Value::Num(-n),
                        other => other,
                    })
                    .collect(),
            ),
            other => {
                out.warnings.push(format!("cannot negate a {}", other.type_name()));
                Value::Undef
            }
        },
        Expr::Binary { op, lhs, rhs } => {
            let l = eval_expr(lhs, env, out);
            let r = eval_expr(rhs, env, out);
            match (l.as_num(), r.as_num()) {
                (Some(a), Some(b)) => Value::Num(match op {
                    BinOp::Add => a + b,
                    BinOp::Sub => a - b,
                    BinOp::Mul => a * b,
                    BinOp::Div => a / b,
                    BinOp::Mod => a % b,
                }),
                _ => {
                    out.warnings.push(format!(
                        "arithmetic needs numbers, got {} and {}",
                        l.type_name(),
                        r.type_name()
                    ));
                    Value::Undef
                }
            }
        }
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
}
