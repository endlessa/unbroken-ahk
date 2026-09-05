//! The `.csg` export: the fully evaluated instantiation tree.
//!
//! This is the reference's "single most useful differential-testing oracle
//! format" — every variable, loop, function call, and user module resolved
//! away, leaving a canonical nesting of built-in primitives:
//!
//! ```text
//! group() {
//! 	group() {
//! 		multmatrix([[1, 0, 0, 10], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]) {
//! 			cube(size = [5, 5, 5], center = false);
//! 		}
//! 	}
//! }
//! ```
//!
//! Two invariants make it useful, and both are pinned by tests:
//!
//! 1. **The output is itself valid OpenSCAD.** Renaming `out.csg` to
//!    `out.scad` and re-running it reproduces the same geometry. Our tests
//!    exploit this directly: evaluate a script, export `.csg`, re-evaluate
//!    the export, and compare meshes.
//! 2. **The child-grouping structure survives.** `difference()` cares which
//!    operand is first, and a `for` loop that emits three cubes is ONE
//!    operand, not three. So every statement that can produce several
//!    shapes records exactly one node — a `group()` wrapper when it has no
//!    more specific head. Flattening a `for` into bare siblings would
//!    silently change what a surrounding `difference()` subtracts.
//!
//! This module owns the node model, the number/value spellings, and the
//! per-module head formatting. The recording itself lives in `eval.rs`,
//! which is the only place that knows when a module is instantiated.
//!
//! Reference entry: "export: debug/text formats (CSG, AST, TERM, ECHO,
//! NEF3, NEFDBG)".

use crate::geom::Mat4;
use crate::value::Value;
use std::collections::HashMap;

/// One instantiation in the evaluated tree. `head` is the full call text
/// (`cube(size = [1, 1, 1], center = false)`), possibly carrying a leading
/// modifier character.
#[derive(Debug, Clone, PartialEq)]
pub struct CsgNode {
    pub head: String,
    pub children: Vec<CsgNode>,
}

impl CsgNode {
    pub fn leaf(head: impl Into<String>) -> CsgNode {
        CsgNode { head: head.into(), children: Vec::new() }
    }

    pub fn group(children: Vec<CsgNode>) -> CsgNode {
        CsgNode { head: "group()".into(), children }
    }
}

/// Serialize a tree to `.csg` text: tab indentation, `;` after a childless
/// node, `{ … }` around children. A trailing newline closes the file.
pub fn render(root: &CsgNode) -> String {
    let mut out = String::new();
    write_node(root, 0, &mut out);
    out
}

fn write_node(node: &CsgNode, depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push('\t');
    }
    out.push_str(&node.head);
    if node.children.is_empty() {
        // A childless operator still prints an empty block, so that
        // `difference() { }` does not re-parse as a call statement with a
        // different arity. Primitives print `;`.
        if takes_children(&node.head) {
            out.push_str(" {\n");
            for _ in 0..depth {
                out.push('\t');
            }
            out.push_str("}\n");
        } else {
            out.push_str(";\n");
        }
        return;
    }
    out.push_str(" {\n");
    for c in &node.children {
        write_node(c, depth + 1, out);
    }
    for _ in 0..depth {
        out.push('\t');
    }
    out.push_str("}\n");
}

/// Whether a head names an operator that always takes a block. Read off the
/// head text (modifier prefixes and arguments stripped) rather than tracked
/// separately, so a node stays a plain (head, children) pair.
fn takes_children(head: &str) -> bool {
    let name = head
        .trim_start_matches(['%', '#', '!', '*'])
        .split('(')
        .next()
        .unwrap_or("");
    matches!(
        name,
        "group"
            | "union"
            | "difference"
            | "intersection"
            | "hull"
            | "minkowski"
            | "multmatrix"
            | "color"
            | "linear_extrude"
            | "rotate_extrude"
            | "offset"
            | "projection"
            | "render"
            | "resize"
    )
}

// ---------------------------------------------------------------------------
// Number and value spellings

/// `.csg` numbers. The reference notes this is NOT echo's 6-digit `%g` —
/// ".csg historically prints more digits" — and that the format must be
/// pinned exactly because .csg diffing is the oracle channel. We print the
/// shortest representation that round-trips to the same f64, which is both
/// "more digits" (1/3 prints all 16) and the only choice that makes
/// invariant 1 exact: re-parsing the export cannot drift the geometry.
pub fn num(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf".into() } else { "inf".into() };
    }
    // Rust's Display for f64 is the shortest round-tripping form and
    // already prints integral values without a decimal point (5, not 5.0).
    // Normalize -0 to 0 so a mirrored transform does not produce a diff
    // against the same matrix built additively.
    if x == 0.0 {
        return "0".into();
    }
    format!("{}", x)
}

/// A value in `.csg` argument position. Ranges and functions cannot appear
/// in a resolved tree's arguments, but they are spelled rather than
/// panicked on so a malformed script still exports something readable.
pub fn value(v: &Value) -> String {
    match v {
        Value::Num(n) => num(*n),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Str(s) => string(s),
        Value::Vector(items) => {
            let parts: Vec<String> = items.iter().map(value).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Range { start, step, end, .. } => {
            format!("[{} : {} : {}]", num(*start), num(*step), num(*end))
        }
        Value::Function(_) => "function".into(),
        Value::Undef => "undef".into(),
    }
}

/// A double-quoted string with the escapes the lexer understands, so the
/// export re-parses to the identical string.
pub fn string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A 4×4 transform as a `multmatrix` head. Every affine transform
/// (translate/rotate/scale/mirror/multmatrix) collapses to this one
/// canonical spelling — that collapse is what "every variable resolved"
/// means for transforms, and it is why the reference lists `multmatrix`
/// among the canonical primitives.
pub fn multmatrix_head(m: &Mat4) -> String {
    let rows: Vec<String> = m
        .iter()
        .map(|r| {
            let cells: Vec<String> = r.iter().map(|c| num(*c)).collect();
            format!("[{}]", cells.join(", "))
        })
        .collect();
    format!("multmatrix([{}])", rows.join(", "))
}

/// The `$fn`/`$fa`/`$fs` triple in effect at an instantiation. Circular
/// primitives and the extrusions print it; box-like ones do not (they are
/// unaffected by it, per the reference).
#[derive(Debug, Clone, Copy)]
pub struct Frags {
    pub fn_: f64,
    pub fa: f64,
    pub fs: f64,
}

impl Frags {
    fn spell(&self) -> String {
        format!("$fn = {}, $fa = {}, $fs = {}", num(self.fn_), num(self.fa), num(self.fs))
    }
}

// ---------------------------------------------------------------------------
// Per-module heads

/// Format the canonical head for a built-in module from its already-bound
/// arguments. Returns `None` for modules that record no node of their own
/// (`echo`, `assert`, `children`, and anything unknown): their children,
/// if any, splice into the enclosing frame.
///
/// Transforms are absent here — they are recorded from `eval.rs` with the
/// matrix they actually built, via `multmatrix_head`.
pub fn builtin_head(module: &str, b: &HashMap<String, Value>, f: Frags) -> Option<String> {
    let num_of = |k: &str| b.get(k).and_then(Value::as_num);
    let bool_of = |k: &str, d: bool| b.get(k).and_then(Value::as_bool).unwrap_or(d);
    let str_of = |k: &str| match b.get(k) {
        Some(Value::Str(s)) => Some(s.clone()),
        _ => None,
    };
    // `d` overrides `r` per the reference; the tree records the resolved
    // radius, never the diameter the user happened to type.
    let radius = |r: &str, d: &str, default: f64| {
        num_of(d).map(|v| v / 2.0).or_else(|| num_of(r)).unwrap_or(default)
    };

    let head = match module {
        "cube" => {
            let size = match b.get("size") {
                Some(Value::Num(s)) => [*s, *s, *s],
                Some(v) => v.as_vec3().unwrap_or([1.0, 1.0, 1.0]),
                None => [1.0, 1.0, 1.0],
            };
            format!(
                "cube(size = [{}, {}, {}], center = {})",
                num(size[0]),
                num(size[1]),
                num(size[2]),
                bool_of("center", false)
            )
        }
        "sphere" => format!("sphere({}, r = {})", f.spell(), num(radius("r", "d", 1.0))),
        "cylinder" => {
            let both = num_of("d").map(|d| d / 2.0).or_else(|| num_of("r"));
            let r1 = num_of("d1").map(|d| d / 2.0).or_else(|| num_of("r1")).or(both).unwrap_or(1.0);
            let r2 = num_of("d2").map(|d| d / 2.0).or_else(|| num_of("r2")).or(both).unwrap_or(1.0);
            format!(
                "cylinder({}, h = {}, r1 = {}, r2 = {}, center = {})",
                f.spell(),
                num(num_of("h").unwrap_or(1.0)),
                num(r1),
                num(r2),
                bool_of("center", false)
            )
        }
        "polyhedron" => format!(
            "polyhedron(points = {}, faces = {}, convexity = {})",
            b.get("points").map(value).unwrap_or_else(|| "undef".into()),
            b.get("faces").or_else(|| b.get("triangles")).map(value).unwrap_or_else(|| "undef".into()),
            num(num_of("convexity").unwrap_or(1.0))
        ),
        "square" => {
            let size = match b.get("size") {
                Some(Value::Num(s)) => [*s, *s],
                Some(Value::Vector(items)) if items.len() >= 2 => {
                    [items[0].as_num().unwrap_or(1.0), items[1].as_num().unwrap_or(1.0)]
                }
                _ => [1.0, 1.0],
            };
            format!(
                "square(size = [{}, {}], center = {})",
                num(size[0]),
                num(size[1]),
                bool_of("center", false)
            )
        }
        "circle" => format!("circle({}, r = {})", f.spell(), num(radius("r", "d", 1.0))),
        "polygon" => format!(
            "polygon(points = {}, paths = {}, convexity = {})",
            b.get("points").map(value).unwrap_or_else(|| "undef".into()),
            b.get("paths").map(value).unwrap_or_else(|| "undef".into()),
            num(num_of("convexity").unwrap_or(1.0))
        ),
        "text" => format!(
            "text(text = {}, size = {}, spacing = {}, font = {}, direction = {}, \
             language = {}, script = {}, halign = {}, valign = {}, {})",
            string(&str_of("text").unwrap_or_default()),
            num(num_of("size").unwrap_or(10.0)),
            num(num_of("spacing").unwrap_or(1.0)),
            string(&str_of("font").unwrap_or_default()),
            string(&str_of("direction").unwrap_or_default()),
            string(&str_of("language").unwrap_or_else(|| "en".into())),
            string(&str_of("script").unwrap_or_default()),
            string(&str_of("halign").unwrap_or_else(|| "left".into())),
            string(&str_of("valign").unwrap_or_else(|| "baseline".into())),
            f.spell()
        ),
        "surface" => format!(
            "surface(file = {}, center = {}, convexity = {})",
            string(&str_of("file").unwrap_or_default()),
            bool_of("center", false),
            num(num_of("convexity").unwrap_or(1.0))
        ),
        "import" | "import_stl" | "import_off" | "import_dxf" => format!(
            "import(file = {}, layer = {}, convexity = {}, $fn = {}, $fa = {}, $fs = {})",
            string(&str_of("file").unwrap_or_default()),
            b.get("layer").map(value).unwrap_or_else(|| "undef".into()),
            num(num_of("convexity").unwrap_or(1.0)),
            num(f.fn_),
            num(f.fa),
            num(f.fs)
        ),
        "union" | "difference" | "intersection" | "hull" | "group" => format!("{}()", module),
        "minkowski" => {
            format!("minkowski(convexity = {})", num(num_of("convexity").unwrap_or(1.0)))
        }
        "render" => format!("render(convexity = {})", num(num_of("convexity").unwrap_or(1.0))),
        "linear_extrude" => format!(
            "linear_extrude(height = {}, center = {}, convexity = {}, twist = {}, \
             slices = {}, scale = {}, {})",
            num(num_of("height").unwrap_or(100.0)),
            bool_of("center", false),
            num(num_of("convexity").unwrap_or(1.0)),
            num(num_of("twist").unwrap_or(0.0)),
            num(num_of("slices").unwrap_or(1.0)),
            match b.get("scale") {
                Some(Value::Num(s)) => format!("[{}, {}]", num(*s), num(*s)),
                Some(v @ Value::Vector(_)) => value(v),
                _ => "[1, 1]".into(),
            },
            f.spell()
        ),
        "rotate_extrude" => format!(
            "rotate_extrude(angle = {}, convexity = {}, {})",
            num(num_of("angle").unwrap_or(360.0)),
            num(num_of("convexity").unwrap_or(2.0)),
            f.spell()
        ),
        "offset" => format!(
            "offset(r = {}, delta = {}, chamfer = {}, {})",
            b.get("r").map(value).unwrap_or_else(|| "undef".into()),
            b.get("delta").map(value).unwrap_or_else(|| "undef".into()),
            bool_of("chamfer", false),
            f.spell()
        ),
        "projection" => format!("projection(cut = {})", bool_of("cut", false)),
        _ => return None,
    };
    Some(head)
}

/// The `color()` head, built from the already-resolved RGBA. `color()` with
/// unparseable arguments records the identity colour rather than dropping
/// the subtree, so the geometry still round-trips.
pub fn color_head(rgba: Option<[f64; 4]>) -> String {
    let c = rgba.unwrap_or([-1.0, -1.0, -1.0, 1.0]);
    format!("color([{}, {}, {}, {}])", num(c[0]), num(c[1]), num(c[2]), num(c[3]))
}

/// The `resize()` head. `resize` is kept as itself rather than resolved to
/// a matrix: its factors depend on the child bounding box, which a reader
/// of the tree can recompute but which is not part of the instantiation.
pub fn resize_head(newsize: [f64; 3], auto: [bool; 3]) -> String {
    format!(
        "resize(newsize = [{}, {}, {}], auto = [{}, {}, {}])",
        num(newsize[0]),
        num(newsize[1]),
        num(newsize[2]),
        auto[0],
        auto[1],
        auto[2]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_round_trip_and_stay_readable() {
        assert_eq!(num(5.0), "5");
        assert_eq!(num(-0.0), "0");
        assert_eq!(num(0.5), "0.5");
        // "more digits than echo's %g": echo would print 0.333333.
        assert_eq!(num(1.0 / 3.0), "0.3333333333333333");
        assert_eq!(num(1.0 / 3.0).parse::<f64>().unwrap(), 1.0 / 3.0);
        assert_eq!(num(f64::INFINITY), "inf");
        assert_eq!(num(f64::NEG_INFINITY), "-inf");
        assert_eq!(num(f64::NAN), "nan");
    }

    #[test]
    fn strings_escape_so_the_export_reparses() {
        assert_eq!(string("hi"), "\"hi\"");
        assert_eq!(string("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
    }

    #[test]
    fn childless_operators_keep_their_block() {
        // `difference();` would parse as a call with no children and mean
        // something different from `difference() { }` on re-import.
        let t = CsgNode::leaf("difference()");
        assert_eq!(render(&t), "difference() {\n}\n");
        // A primitive is a statement, not a block.
        assert_eq!(render(&CsgNode::leaf("cube(size = [1, 1, 1], center = false)")),
                   "cube(size = [1, 1, 1], center = false);\n");
    }

    #[test]
    fn modifier_prefixes_do_not_hide_the_operator() {
        let t = CsgNode::leaf("%difference()");
        assert_eq!(render(&t), "%difference() {\n}\n");
    }

    #[test]
    fn nesting_indents_with_tabs() {
        let tree = CsgNode::group(vec![CsgNode {
            head: "multmatrix([[1, 0, 0, 2], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]])".into(),
            children: vec![CsgNode::leaf("cube(size = [1, 1, 1], center = false)")],
        }]);
        assert_eq!(
            render(&tree),
            "group() {\n\
             \tmultmatrix([[1, 0, 0, 2], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]) {\n\
             \t\tcube(size = [1, 1, 1], center = false);\n\
             \t}\n\
             }\n"
        );
    }

    #[test]
    fn diameter_resolves_to_radius() {
        let f = Frags { fn_: 0.0, fa: 12.0, fs: 2.0 };
        let mut b = HashMap::new();
        b.insert("d".to_string(), Value::Num(8.0));
        let head = builtin_head("sphere", &b, f).unwrap();
        assert!(head.ends_with("r = 4)"), "{}", head);
    }

    #[test]
    fn unknown_modules_record_nothing() {
        let f = Frags { fn_: 0.0, fa: 12.0, fs: 2.0 };
        assert!(builtin_head("echo", &HashMap::new(), f).is_none());
        assert!(builtin_head("nonesuch", &HashMap::new(), f).is_none());
    }
}
