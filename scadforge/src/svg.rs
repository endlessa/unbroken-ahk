//! SVG import (2D) — a from-scratch, zero-dependency reader for the element
//! subset OpenSCAD 2021.01 imports: `<path>`, `<rect>` (incl. rounded corners),
//! `<circle>`, `<ellipse>`, `<line>`, `<polyline>`, `<polygon>`, with `<g>`
//! transforms applied. Only FILL geometry is imported — strokes are not
//! expanded — and `<text>` is ignored (convert to paths first). Curves (path
//! béziers/arcs, circles/ellipses/rounded rects) are flattened with the
//! `$fn/$fa/$fs` in scope at the `import()` call.
//!
//! Coordinate convention (the exact inverse of `io::write_svg`, so a region
//! survives an export→import round-trip): user-space `(x, y)` maps to model
//! `(x·s, −y·s)` where `s` converts user units to millimetres — the SVG Y axis
//! points down, so it is flipped, and no viewBox translation is applied (the
//! content keeps its absolute placement, landing in y<0 for a top-left origin,
//! matching the reference's non-centered default). `px`/unitless user units
//! scale by `dpi` (25.4/dpi mm each); a physical width unit with a viewBox
//! scales user units by physical_mm / viewBox_width.

use crate::geom::fragments;
use crate::poly2::{Poly2, Vec2};

/// A 2×3 affine transform `[a, b, c, d, e, f]` in SVG order:
/// `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
type Xform = [f64; 6];

const IDENTITY: Xform = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn apply(t: &Xform, p: Vec2) -> Vec2 {
    [t[0] * p[0] + t[2] * p[1] + t[4], t[1] * p[0] + t[3] * p[1] + t[5]]
}

/// Compose two transforms: `mul(a, b)` applies `b` first, then `a`.
fn mul(a: &Xform, b: &Xform) -> Xform {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

/// Read an SVG document into one even-odd 2D region plus warnings. Malformed or
/// unsupported constructs are skipped (with a warning for `<text>`); a document
/// with no fillable geometry yields an empty region, never an error.
pub fn read_svg(text: &str, dpi: f64, fn_: f64, fa: f64, fs: f64) -> (Poly2, Vec<String>) {
    let mut warnings = Vec::new();
    let scale = viewport_scale(text, dpi);
    // Walk elements, maintaining a transform stack for nested <g>.
    let mut stack: Vec<Xform> = vec![IDENTITY];
    let mut contours: Vec<Vec<Vec2>> = Vec::new();
    let frag = |r: f64| fragments(r.abs().max(1e-6), fn_, fa, fs).max(3) as usize;

    for tok in ElementScanner::new(text) {
        let cur = *stack.last().unwrap();
        match tok {
            Element::GroupOpen(attrs) => {
                let t = attr(&attrs, "transform").map(|s| parse_transform(&s)).unwrap_or(IDENTITY);
                stack.push(mul(&cur, &t));
            }
            Element::GroupClose => {
                if stack.len() > 1 {
                    stack.pop();
                }
            }
            Element::Shape(name, attrs) => {
                // An element may carry its own transform on top of the stack.
                let local =
                    attr(&attrs, "transform").map(|s| mul(&cur, &parse_transform(&s))).unwrap_or(cur);
                if name == "text" {
                    warnings.push("SVG <text> is ignored on import (convert text to paths)".into());
                    continue;
                }
                for c in shape_contours(&name, &attrs, &frag) {
                    if c.len() >= 2 {
                        contours.push(c.into_iter().map(|p| apply(&local, p)).collect());
                    }
                }
            }
        }
    }

    // Global viewport transform: user units → mm, Y flipped.
    for c in &mut contours {
        for p in c.iter_mut() {
            *p = [p[0] * scale, -p[1] * scale];
        }
    }
    // Keep only real fillable contours: at least a triangle, and every point
    // finite (a `NaN`/`inf` coordinate from a malformed attribute is dropped).
    contours.retain(|c| c.len() >= 3 && c.iter().all(|p| p[0].is_finite() && p[1].is_finite()));
    (Poly2::new(contours), warnings)
}

/// The user-unit → millimetre scale from the root `<svg>` width/height/viewBox.
fn viewport_scale(text: &str, dpi: f64) -> f64 {
    let px_per_mm = dpi / 25.4;
    let svg_attrs = match find_open_tag(text, "svg") {
        Some(a) => a,
        None => return 1.0 / px_per_mm.max(1e-9), // default: user units are px
    };
    let viewbox = attr(&svg_attrs, "viewBox").and_then(|v| {
        let n: Vec<f64> = v.split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        if n.len() == 4 { Some(n[2]) } else { None }
    });
    let width = attr(&svg_attrs, "width").map(|w| parse_length(&w, px_per_mm));
    match (width, viewbox) {
        // Physical width + viewBox: each user unit spans physical_mm / vb_width.
        (Some((mm, _)), Some(vb)) if vb.abs() > 1e-9 => mm / vb,
        // Width only (no viewBox): user units are already whatever the width is.
        (Some((_, unit_mm)), None) => unit_mm,
        // viewBox only, or nothing: user units are px.
        _ => 1.0 / px_per_mm.max(1e-9),
    }
}

/// Parse an SVG length into `(total_mm, mm_per_user_unit)`. For `px`/unitless
/// the per-unit factor is `25.4/dpi`; physical units convert exactly.
fn parse_length(s: &str, px_per_mm: f64) -> (f64, f64) {
    let s = s.trim();
    let split = s.find(|c: char| c.is_alphabetic() || c == '%').unwrap_or(s.len());
    let value: f64 = s[..split].trim().parse().unwrap_or(0.0);
    let unit = s[split..].trim();
    let per_unit = match unit {
        "mm" => 1.0,
        "cm" => 10.0,
        "in" => 25.4,
        "pt" => 25.4 / 72.0,
        "pc" => 25.4 / 6.0,
        // "px" and unitless share the dpi conversion.
        _ => 1.0 / px_per_mm.max(1e-9),
    };
    (value * per_unit, per_unit)
}

// --- element scanning -------------------------------------------------------

enum Element {
    GroupOpen(String),  // raw attribute text of a <g ...>
    GroupClose,
    Shape(String, String), // (tag name, raw attribute text)
}

/// Yields group open/close and shape elements from an SVG document. This is a
/// permissive tag scanner (not a full XML parser): it skips comments, the XML
/// declaration, DOCTYPE, and CDATA, and treats `<g/>` self-closing as an open
/// with an immediate close.
struct ElementScanner<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> ElementScanner<'a> {
    fn new(text: &'a str) -> Self {
        ElementScanner { s: text.as_bytes(), i: 0 }
    }
}

impl<'a> Iterator for ElementScanner<'a> {
    type Item = Element;
    fn next(&mut self) -> Option<Element> {
        let s = self.s;
        while self.i < s.len() {
            if s[self.i] != b'<' {
                self.i += 1;
                continue;
            }
            // Skip comments / declarations / CDATA.
            if s[self.i..].starts_with(b"<!--") {
                if let Some(end) = find(&s[self.i..], b"-->") {
                    self.i += end + 3;
                } else {
                    self.i = s.len();
                }
                continue;
            }
            if s[self.i..].starts_with(b"<?") || s[self.i..].starts_with(b"<!") {
                if let Some(end) = s[self.i..].iter().position(|&b| b == b'>') {
                    self.i += end + 1;
                } else {
                    self.i = s.len();
                }
                continue;
            }
            // A real element tag: read to the matching '>'.
            let close = match s[self.i..].iter().position(|&b| b == b'>') {
                Some(c) => self.i + c,
                None => {
                    self.i = s.len();
                    return None;
                }
            };
            let raw = &s[self.i + 1..close]; // between < and >
            self.i = close + 1;
            let raw_str = String::from_utf8_lossy(raw);
            let body = raw_str.trim();
            if let Some(rest) = body.strip_prefix('/') {
                // Closing tag </g> (case-insensitive, matching the open path).
                if rest.trim().eq_ignore_ascii_case("g") {
                    return Some(Element::GroupClose);
                }
                continue;
            }
            let self_closing = body.ends_with('/');
            let body = body.trim_end_matches('/').trim();
            // Split the tag name from its attributes.
            let (name, attrs) = match body.find(|c: char| c.is_whitespace()) {
                Some(p) => (&body[..p], body[p..].trim()),
                None => (body, ""),
            };
            let name = name.to_ascii_lowercase();
            match name.as_str() {
                "g" => {
                    // A self-closing <g .../> has no children, so it opens and
                    // immediately closes — emitting nothing keeps its transform
                    // from leaking onto following siblings (there is no matching
                    // </g> to pop it).
                    if self_closing {
                        continue;
                    }
                    return Some(Element::GroupOpen(attrs.to_string()));
                }
                "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon"
                | "text" => {
                    return Some(Element::Shape(name, attrs.to_string()));
                }
                _ => continue,
            }
        }
        None
    }
}

/// Find the attribute text of the first `<name ...>` open tag in the document.
fn find_open_tag(text: &str, name: &str) -> Option<String> {
    let needle = format!("<{}", name);
    let start = text.find(&needle)?;
    let after = &text[start + needle.len()..];
    // The char right after the name must be whitespace or '>' (avoid <svgfoo>).
    if !after.starts_with(|c: char| c.is_whitespace() || c == '>' || c == '/') {
        return None;
    }
    let close = after.find('>')?;
    Some(after[..close].trim_end_matches('/').trim().to_string())
}

/// Read one attribute value from a raw attribute string (`key="value"` or
/// `key='value'`). Returns the unescaped-enough value (basic entities only).
/// Byte-boundary-safe: it searches on `str::find` (always a char boundary) and
/// never slices at an arbitrary index, so a multi-byte value elsewhere in the
/// string (e.g. `id="café"`) can't panic the parse. Shared with the 3MF reader.
pub(crate) fn attr(attrs: &str, key: &str) -> Option<String> {
    let mut from = 0;
    while let Some(rel) = attrs[from..].find(key) {
        let i = from + rel; // a char boundary (start of the match)
        from = i + key.len();
        // The key must be a whole token: preceded by start/space, followed by
        // optional space then '=' (so `strokewidth` doesn't match `stroke`).
        let before_ok = i == 0 || attrs[..i].chars().next_back().is_some_and(|c| c.is_whitespace());
        if !before_ok {
            continue;
        }
        let trimmed = attrs[from..].trim_start();
        if let Some(after_eq) = trimmed.strip_prefix('=') {
            let after_eq = after_eq.trim_start();
            let quote = after_eq.chars().next();
            if quote == Some('"') || quote == Some('\'') {
                let q = quote.unwrap();
                let body = &after_eq[1..];
                if let Some(end) = body.find(q) {
                    return Some(unescape(&body[..end]));
                }
            }
        }
    }
    None
}

fn unescape(s: &str) -> String {
    s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"")
}

fn attr_f(attrs: &str, key: &str, default: f64) -> f64 {
    attr(attrs, key).and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

// --- shapes -----------------------------------------------------------------

fn shape_contours(name: &str, attrs: &str, frag: &dyn Fn(f64) -> usize) -> Vec<Vec<Vec2>> {
    match name {
        "rect" => rect_contour(attrs, frag),
        "circle" => {
            let (cx, cy, r) = (attr_f(attrs, "cx", 0.0), attr_f(attrs, "cy", 0.0), attr_f(attrs, "r", 0.0));
            if !(r > 0.0) { vec![] } else { vec![ellipse_pts(cx, cy, r, r, frag(r))] }
        }
        "ellipse" => {
            let cx = attr_f(attrs, "cx", 0.0);
            let cy = attr_f(attrs, "cy", 0.0);
            let rx = attr_f(attrs, "rx", 0.0);
            let ry = attr_f(attrs, "ry", 0.0);
            if !(rx > 0.0 && ry > 0.0) { vec![] } else { vec![ellipse_pts(cx, cy, rx, ry, frag(rx.max(ry)))] }
        }
        "line" => {
            let p0 = [attr_f(attrs, "x1", 0.0), attr_f(attrs, "y1", 0.0)];
            let p1 = [attr_f(attrs, "x2", 0.0), attr_f(attrs, "y2", 0.0)];
            vec![vec![p0, p1]] // no fill area — filtered out later
        }
        "polyline" | "polygon" => vec![parse_points(&attr(attrs, "points").unwrap_or_default())],
        "path" => parse_path(&attr(attrs, "d").unwrap_or_default(), frag),
        _ => vec![],
    }
}

fn rect_contour(attrs: &str, frag: &dyn Fn(f64) -> usize) -> Vec<Vec<Vec2>> {
    let x = attr_f(attrs, "x", 0.0);
    let y = attr_f(attrs, "y", 0.0);
    let w = attr_f(attrs, "width", 0.0);
    let h = attr_f(attrs, "height", 0.0);
    if !(w > 0.0 && h > 0.0) {
        return vec![];
    }
    // Rounded corners: rx/ry (either implies the other), clamped to half-extent.
    let mut rx = attr(attrs, "rx").and_then(|v| v.trim().parse::<f64>().ok());
    let mut ry = attr(attrs, "ry").and_then(|v| v.trim().parse::<f64>().ok());
    if rx.is_none() {
        rx = ry;
    }
    if ry.is_none() {
        ry = rx;
    }
    let rx = rx.unwrap_or(0.0).min(w / 2.0).max(0.0);
    let ry = ry.unwrap_or(0.0).min(h / 2.0).max(0.0);
    if rx <= 0.0 || ry <= 0.0 {
        return vec![vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]]];
    }
    let n = frag(rx.max(ry)).max(2);
    let quarter = (n / 4).max(2);
    let mut pts = Vec::new();
    // Corner centers, walking clockwise from the top-left, each a 90° arc.
    let arcs = [
        ([x + rx, y + ry], std::f64::consts::PI, 1.5 * std::f64::consts::PI), // TL
        ([x + w - rx, y + ry], 1.5 * std::f64::consts::PI, std::f64::consts::TAU), // TR
        ([x + w - rx, y + h - ry], 0.0, 0.5 * std::f64::consts::PI), // BR
        ([x + rx, y + h - ry], 0.5 * std::f64::consts::PI, std::f64::consts::PI), // BL
    ];
    for (c, a0, a1) in arcs {
        for k in 0..=quarter {
            let a = a0 + (a1 - a0) * k as f64 / quarter as f64;
            pts.push([c[0] + rx * a.cos(), c[1] + ry * a.sin()]);
        }
    }
    vec![pts]
}

fn ellipse_pts(cx: f64, cy: f64, rx: f64, ry: f64, n: usize) -> Vec<Vec2> {
    (0..n)
        .map(|k| {
            let a = std::f64::consts::TAU * k as f64 / n as f64;
            [cx + rx * a.cos(), cy + ry * a.sin()]
        })
        .collect()
}

/// Parse a `points="x,y x,y ..."` list (commas or whitespace separated).
fn parse_points(s: &str) -> Vec<Vec2> {
    let nums: Vec<f64> = s
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse().ok())
        .collect();
    nums.chunks_exact(2).map(|p| [p[0], p[1]]).collect()
}

// --- path data --------------------------------------------------------------

/// Parse a path `d` string into one contour per subpath, flattening curves.
fn parse_path(d: &str, frag: &dyn Fn(f64) -> usize) -> Vec<Vec<Vec2>> {
    let mut r = PathReader::new(d);
    let mut subpaths: Vec<Vec<Vec2>> = Vec::new();
    let mut cur = [0.0, 0.0];
    let mut start = [0.0, 0.0];
    let mut path: Vec<Vec2> = Vec::new();
    let mut last_cmd = ' ';
    let mut last_ctrl: Option<Vec2> = None; // reflected control for S/T

    let flush = |subpaths: &mut Vec<Vec<Vec2>>, path: &mut Vec<Vec2>| {
        if path.len() >= 2 {
            subpaths.push(std::mem::take(path));
        } else {
            path.clear();
        }
    };

    while let Some(cmd) = r.command() {
        let rel = cmd.is_ascii_lowercase();
        match cmd.to_ascii_uppercase() {
            'M' => {
                let (mut x, mut y) = (r.num(), r.num());
                if rel {
                    x += cur[0];
                    y += cur[1];
                }
                flush(&mut subpaths, &mut path);
                cur = [x, y];
                start = cur;
                path.push(cur);
                // Subsequent coordinate pairs are implicit lineto.
                while r.has_num() {
                    let (mut lx, mut ly) = (r.num(), r.num());
                    if rel {
                        lx += cur[0];
                        ly += cur[1];
                    }
                    cur = [lx, ly];
                    path.push(cur);
                }
                last_ctrl = None;
            }
            'L' => {
                while r.has_num() {
                    let (mut x, mut y) = (r.num(), r.num());
                    if rel {
                        x += cur[0];
                        y += cur[1];
                    }
                    cur = [x, y];
                    path.push(cur);
                }
                last_ctrl = None;
            }
            'H' => {
                while r.has_num() {
                    let mut x = r.num();
                    if rel {
                        x += cur[0];
                    }
                    cur = [x, cur[1]];
                    path.push(cur);
                }
                last_ctrl = None;
            }
            'V' => {
                while r.has_num() {
                    let mut y = r.num();
                    if rel {
                        y += cur[1];
                    }
                    cur = [cur[0], y];
                    path.push(cur);
                }
                last_ctrl = None;
            }
            'C' => {
                while r.has_num() {
                    let c1 = r.pt(rel, cur);
                    let c2 = r.pt(rel, cur);
                    let end = r.pt(rel, cur);
                    flatten_cubic(&mut path, cur, c1, c2, end, frag);
                    cur = end;
                    last_ctrl = Some(c2);
                }
            }
            'S' => {
                while r.has_num() {
                    let c1 = reflect(last_ctrl, cur, last_cmd, &['C', 'S']);
                    let c2 = r.pt(rel, cur);
                    let end = r.pt(rel, cur);
                    flatten_cubic(&mut path, cur, c1, c2, end, frag);
                    cur = end;
                    last_ctrl = Some(c2);
                    last_cmd = 'S'; // later packed sets reflect off this S
                }
            }
            'Q' => {
                while r.has_num() {
                    let c1 = r.pt(rel, cur);
                    let end = r.pt(rel, cur);
                    flatten_quad(&mut path, cur, c1, end, frag);
                    cur = end;
                    last_ctrl = Some(c1);
                }
            }
            'T' => {
                while r.has_num() {
                    let c1 = reflect(last_ctrl, cur, last_cmd, &['Q', 'T']);
                    let end = r.pt(rel, cur);
                    flatten_quad(&mut path, cur, c1, end, frag);
                    cur = end;
                    last_ctrl = Some(c1);
                    last_cmd = 'T'; // later packed sets reflect off this T
                }
            }
            'A' => {
                while r.has_num() {
                    let rx = r.num();
                    let ry = r.num();
                    let rot = r.num();
                    let large = r.flag();
                    let sweep = r.flag();
                    let end = r.pt(rel, cur);
                    flatten_arc(&mut path, cur, rx, ry, rot, large, sweep, end, frag);
                    cur = end;
                }
                last_ctrl = None;
            }
            'Z' => {
                // Close the subpath. Per SVG, if a drawing command (not M)
                // follows, the next subpath restarts at the SAME initial point,
                // so re-seed the buffer with it (a lone Z, or an M next, just
                // discards this single-point buffer via flush / the M flush).
                if path.len() >= 2 {
                    subpaths.push(std::mem::take(&mut path));
                }
                path.clear();
                cur = start;
                path.push(start);
                last_ctrl = None;
            }
            _ => break, // unknown command: stop (permissive)
        }
        last_cmd = cmd.to_ascii_uppercase();
    }
    flush(&mut subpaths, &mut path);
    subpaths
}

/// The reflected control point for a smooth curve: reflect the previous control
/// about the current point if the previous command was a matching curve; else
/// the current point itself.
fn reflect(last_ctrl: Option<Vec2>, cur: Vec2, last_cmd: char, kinds: &[char]) -> Vec2 {
    match last_ctrl {
        Some(c) if kinds.contains(&last_cmd) => [2.0 * cur[0] - c[0], 2.0 * cur[1] - c[1]],
        _ => cur,
    }
}

fn flatten_cubic(out: &mut Vec<Vec2>, p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, frag: &dyn Fn(f64) -> usize) {
    let span = dist(p0, p1) + dist(p1, p2) + dist(p2, p3);
    let n = frag(span / 4.0).max(2);
    for k in 1..=n {
        let t = k as f64 / n as f64;
        let u = 1.0 - t;
        let x = u * u * u * p0[0] + 3.0 * u * u * t * p1[0] + 3.0 * u * t * t * p2[0] + t * t * t * p3[0];
        let y = u * u * u * p0[1] + 3.0 * u * u * t * p1[1] + 3.0 * u * t * t * p2[1] + t * t * t * p3[1];
        out.push([x, y]);
    }
}

fn flatten_quad(out: &mut Vec<Vec2>, p0: Vec2, p1: Vec2, p2: Vec2, frag: &dyn Fn(f64) -> usize) {
    let span = dist(p0, p1) + dist(p1, p2);
    let n = frag(span / 3.0).max(2);
    for k in 1..=n {
        let t = k as f64 / n as f64;
        let u = 1.0 - t;
        let x = u * u * p0[0] + 2.0 * u * t * p1[0] + t * t * p2[0];
        let y = u * u * p0[1] + 2.0 * u * t * p1[1] + t * t * p2[1];
        out.push([x, y]);
    }
}

/// Flatten an SVG endpoint-parameterized elliptical arc, per the spec's
/// endpoint→center conversion (F.6.5), then sample it.
#[allow(clippy::too_many_arguments)]
fn flatten_arc(
    out: &mut Vec<Vec2>,
    p0: Vec2,
    mut rx: f64,
    mut ry: f64,
    x_rot_deg: f64,
    large: bool,
    sweep: bool,
    p1: Vec2,
    frag: &dyn Fn(f64) -> usize,
) {
    if rx == 0.0 || ry == 0.0 || (p0[0] == p1[0] && p0[1] == p1[1]) {
        out.push(p1); // degenerate → straight line
        return;
    }
    rx = rx.abs();
    ry = ry.abs();
    let phi = x_rot_deg.to_radians();
    let (cp, sp) = (phi.cos(), phi.sin());
    // Step 1: transform to the ellipse's coordinate frame.
    let dx = (p0[0] - p1[0]) / 2.0;
    let dy = (p0[1] - p1[1]) / 2.0;
    let x1 = cp * dx + sp * dy;
    let y1 = -sp * dx + cp * dy;
    // Step 2: correct out-of-range radii.
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    // Step 3: compute the center in the transformed frame.
    let num = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut coef = if den > 0.0 { (num / den).sqrt() } else { 0.0 };
    if large == sweep {
        coef = -coef;
    }
    let cx1 = coef * rx * y1 / ry;
    let cy1 = -coef * ry * x1 / rx;
    // Step 4: back to user space center.
    let cx = cp * cx1 - sp * cy1 + (p0[0] + p1[0]) / 2.0;
    let cy = sp * cx1 + cp * cy1 + (p0[1] + p1[1]) / 2.0;
    // Step 5: start angle and sweep.
    let ang = |ux: f64, uy: f64, vx: f64, vy: f64| -> f64 {
        let dot = ux * vx + uy * vy;
        let len = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
        let mut a = (dot / len).clamp(-1.0, 1.0).acos();
        if ux * vy - uy * vx < 0.0 {
            a = -a;
        }
        a
    };
    let theta0 = ang(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut dtheta = ang((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
    if !sweep && dtheta > 0.0 {
        dtheta -= std::f64::consts::TAU;
    } else if sweep && dtheta < 0.0 {
        dtheta += std::f64::consts::TAU;
    }
    let n = (frag(rx.max(ry)) as f64 * (dtheta.abs() / std::f64::consts::TAU)).ceil().max(1.0) as usize;
    for k in 1..=n {
        let t = theta0 + dtheta * k as f64 / n as f64;
        let (ex, ey) = (rx * t.cos(), ry * t.sin());
        out.push([cp * ex - sp * ey + cx, sp * ex + cp * ey + cy]);
    }
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// A cursor over path `d` data that yields commands, numbers, and arc flags.
struct PathReader<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> PathReader<'a> {
    fn new(d: &'a str) -> Self {
        PathReader { s: d.as_bytes(), i: 0 }
    }
    fn skip_sep(&mut self) {
        while self.i < self.s.len() && (self.s[self.i].is_ascii_whitespace() || self.s[self.i] == b',') {
            self.i += 1;
        }
    }
    /// The next command letter, or None at end.
    fn command(&mut self) -> Option<char> {
        self.skip_sep();
        while self.i < self.s.len() {
            let c = self.s[self.i] as char;
            if c.is_ascii_alphabetic() {
                self.i += 1;
                return Some(c);
            }
            // Stray token before a command → skip it.
            self.i += 1;
            self.skip_sep();
        }
        None
    }
    /// Is another number available (before the next command letter)?
    fn has_num(&mut self) -> bool {
        self.skip_sep();
        if self.i >= self.s.len() {
            return false;
        }
        let c = self.s[self.i];
        c == b'-' || c == b'+' || c == b'.' || c.is_ascii_digit()
    }
    /// Read one number (float, with optional sign/fraction/exponent).
    fn num(&mut self) -> f64 {
        self.skip_sep();
        let start = self.i;
        let s = self.s;
        if self.i < s.len() && (s[self.i] == b'-' || s[self.i] == b'+') {
            self.i += 1;
        }
        while self.i < s.len() && s[self.i].is_ascii_digit() {
            self.i += 1;
        }
        if self.i < s.len() && s[self.i] == b'.' {
            self.i += 1;
            while self.i < s.len() && s[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        if self.i < s.len() && (s[self.i] == b'e' || s[self.i] == b'E') {
            self.i += 1;
            if self.i < s.len() && (s[self.i] == b'-' || s[self.i] == b'+') {
                self.i += 1;
            }
            while self.i < s.len() && s[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        std::str::from_utf8(&s[start..self.i]).ok().and_then(|t| t.parse().ok()).unwrap_or(0.0)
    }
    /// Read a point (two numbers), relative to `cur` when `rel`.
    fn pt(&mut self, rel: bool, cur: Vec2) -> Vec2 {
        let x = self.num();
        let y = self.num();
        if rel {
            [x + cur[0], y + cur[1]]
        } else {
            [x, y]
        }
    }
    /// Read an arc flag: exactly one `0`/`1` digit (they may be packed without
    /// separators, so a general number read would over-consume).
    fn flag(&mut self) -> bool {
        self.skip_sep();
        if self.i < self.s.len() {
            let c = self.s[self.i];
            if c == b'0' || c == b'1' {
                self.i += 1;
                return c == b'1';
            }
        }
        // Malformed: fall back to a full number read.
        self.num() != 0.0
    }
}

// --- transforms -------------------------------------------------------------

/// Parse an SVG `transform` attribute (a list of functions applied left→right).
fn parse_transform(s: &str) -> Xform {
    let mut t = IDENTITY;
    let mut rest = s.trim();
    while let Some(open) = rest.find('(') {
        let name = rest[..open].trim().rsplit(|c: char| c.is_whitespace() || c == ')').next().unwrap_or("").trim();
        let close = match rest[open..].find(')') {
            Some(c) => open + c,
            None => break,
        };
        let args: Vec<f64> = rest[open + 1..close]
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|a| !a.is_empty())
            .filter_map(|a| a.parse().ok())
            .collect();
        let m = transform_fn(name, &args);
        t = mul(&t, &m);
        rest = &rest[close + 1..];
    }
    t
}

fn transform_fn(name: &str, a: &[f64]) -> Xform {
    let g = |i: usize| a.get(i).copied();
    match name {
        "translate" => [1.0, 0.0, 0.0, 1.0, g(0).unwrap_or(0.0), g(1).unwrap_or(0.0)],
        "scale" => {
            let sx = g(0).unwrap_or(1.0);
            let sy = g(1).unwrap_or(sx);
            [sx, 0.0, 0.0, sy, 0.0, 0.0]
        }
        "rotate" => {
            let r = g(0).unwrap_or(0.0).to_radians();
            let (c, s) = (r.cos(), r.sin());
            let rot = [c, s, -s, c, 0.0, 0.0];
            match (g(1), g(2)) {
                (Some(cx), Some(cy)) => {
                    let to = [1.0, 0.0, 0.0, 1.0, cx, cy];
                    let from = [1.0, 0.0, 0.0, 1.0, -cx, -cy];
                    mul(&mul(&to, &rot), &from)
                }
                _ => rot,
            }
        }
        "matrix" if a.len() >= 6 => [a[0], a[1], a[2], a[3], a[4], a[5]],
        "skewX" => {
            let t = g(0).unwrap_or(0.0).to_radians().tan();
            [1.0, 0.0, t, 1.0, 0.0, 0.0]
        }
        "skewY" => {
            let t = g(0).unwrap_or(0.0).to_radians().tan();
            [1.0, t, 0.0, 1.0, 0.0, 0.0]
        }
        _ => IDENTITY,
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly2::signed_area2;

    // Total unsigned area of a region's contours (even-odd: holes subtract).
    fn area(p: &Poly2) -> f64 {
        // Order-independent for our simple fixtures: sum outer, subtract holes
        // by even-odd depth is complex — for single-contour tests use |A|/2, and
        // for a rect-with-hole compute outer minus inner explicitly in the test.
        p.contours.iter().map(|c| signed_area2(c).abs() / 2.0).sum()
    }

    fn read(svg: &str) -> Poly2 {
        read_svg(svg, 96.0, 64.0, 12.0, 2.0).0
    }

    #[test]
    fn rect_circle_and_polygon_have_right_area_and_yflip() {
        // A 10×10 rect at the origin, mm units, viewBox 1:1 → scale 1.
        let svg = "<svg width=\"10mm\" height=\"10mm\" viewBox=\"0 0 10 10\">\
                   <rect x=\"0\" y=\"0\" width=\"10\" height=\"10\"/></svg>";
        let p = read(svg);
        assert_eq!(p.contours.len(), 1);
        assert!((area(&p) - 100.0).abs() < 1e-9, "rect area {}", area(&p));
        // Y is flipped: every point lands in y<=0 (top-left origin convention).
        assert!(p.contours[0].iter().all(|q| q[1] <= 1e-9), "rect Y-flipped: {:?}", p.contours[0]);

        // circle r=5 → area ≈ 25π (tessellated).
        let c = read("<svg viewBox=\"0 0 20 20\" width=\"20mm\" height=\"20mm\">\
                      <circle cx=\"10\" cy=\"10\" r=\"5\"/></svg>");
        assert!((area(&c) - std::f64::consts::PI * 25.0).abs() < 2.0, "circle area {}", area(&c));

        // polygon triangle, area 50.
        let t = read("<svg width=\"10mm\" height=\"10mm\" viewBox=\"0 0 10 10\">\
                      <polygon points=\"0,0 10,0 0,10\"/></svg>");
        assert!((area(&t) - 50.0).abs() < 1e-9, "triangle area {}", area(&t));
    }

    #[test]
    fn path_lines_and_curves_close_a_region() {
        // A square via path linetos.
        let sq = read("<svg width=\"4mm\" height=\"4mm\" viewBox=\"0 0 4 4\">\
                       <path d=\"M0 0 L4 0 L4 4 L0 4 Z\"/></svg>");
        assert!((area(&sq) - 16.0).abs() < 1e-9, "path square {}", area(&sq));

        // A closed cubic + arc figure should produce a nonempty closed contour.
        let curved = read("<svg width=\"20mm\" height=\"20mm\" viewBox=\"0 0 20 20\">\
                           <path d=\"M2 10 C2 2 18 2 18 10 A8 8 0 0 1 2 10 Z\"/></svg>");
        assert!(!curved.is_empty());
        assert!(area(&curved) > 40.0, "curved area {}", area(&curved));
    }

    #[test]
    fn group_transform_is_applied() {
        // translate(10,0) shifts a unit rect; area is unchanged, position isn't.
        let g = read("<svg width=\"20mm\" height=\"20mm\" viewBox=\"0 0 20 20\">\
                      <g transform=\"translate(10,0)\">\
                      <rect x=\"0\" y=\"0\" width=\"5\" height=\"5\"/></g></svg>");
        assert!((area(&g) - 25.0).abs() < 1e-9);
        // After translate(10,0) and Y-flip, x spans [10,15].
        let minx = g.contours[0].iter().map(|q| q[0]).fold(f64::INFINITY, f64::min);
        assert!((minx - 10.0).abs() < 1e-9, "translated minx {}", minx);

        // scale(2) doubles each axis → 4× area.
        let s = read("<svg width=\"20mm\" height=\"20mm\" viewBox=\"0 0 20 20\">\
                      <g transform=\"scale(2)\"><rect x=\"0\" y=\"0\" width=\"3\" height=\"3\"/></g></svg>");
        assert!((area(&s) - 36.0).abs() < 1e-9, "scaled area {}", area(&s));
    }

    #[test]
    fn round_trips_through_our_svg_exporter() {
        // Export a rectangular ring (outer + hole) and read it straight back:
        // the reader is the exact inverse of the writer, so both contours and
        // the net area survive.
        let outer = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]];
        let hole = vec![[5.0, 3.0], [15.0, 3.0], [15.0, 7.0], [5.0, 7.0]];
        let region = Poly2::new(vec![outer, hole]);
        let svg = crate::io::write_svg(std::slice::from_ref(&region));
        let back = read(&svg);
        assert_eq!(back.contours.len(), 2, "outer + hole preserved");
        // Net even-odd area = 200 (outer) − 40 (hole) = 160.
        let net = area(&back) - 2.0 * 40.0; // area() sums both; subtract hole twice
        assert!((net - 160.0).abs() < 1e-6, "round-trip net area {}", net);
    }

    #[test]
    fn text_is_ignored_with_a_warning_and_junk_is_safe() {
        let (p, warns) = read_svg(
            "<svg viewBox=\"0 0 10 10\" width=\"10mm\" height=\"10mm\">\
             <text x=\"0\" y=\"0\">hi</text><rect width=\"10\" height=\"10\"/></svg>",
            96.0, 32.0, 12.0, 2.0,
        );
        assert!(warns.iter().any(|w| w.contains("<text>")));
        assert!((area(&p) - 100.0).abs() < 1e-9); // the rect still imports
        // Not-svg input yields an empty region, not a panic.
        assert!(read_svg("garbage", 96.0, 8.0, 12.0, 2.0).0.is_empty());
    }

    #[test]
    fn smooth_curve_reflects_on_implicit_repeated_sets() {
        // Two cubic segments packed under one S. The 2nd set must reflect off
        // the first S's control point → (6,-2), not degrade to the endpoint.
        // viewBox width == mm width → scale 1, so model y = -(svg y).
        let p = read("<svg width=\"12mm\" height=\"12mm\" viewBox=\"-4 -4 12 12\">\
                      <path d=\"M0 0 S2 2 4 0 6 -2 8 0 Z\"/></svg>");
        // The reflected 2nd control (6,-2) → after Y-flip (2,-2)... assert the
        // contour dips to a clearly-negative-in-model y near the 2nd segment
        // (t=0.5 y = -1.5 model, i.e. +1.5 after flip). The buggy version's
        // t=0.5 was -0.75 → +0.75. Check the extreme flipped-y magnitude.
        let max_y = p.contours[0].iter().map(|q| q[1]).fold(f64::NEG_INFINITY, f64::max);
        assert!(max_y > 1.2, "reflected control shapes the 2nd segment: max_y={}", max_y);
    }

    #[test]
    fn self_closing_group_does_not_leak_transform() {
        // An empty <g transform=.../> must NOT shift the following sibling.
        let p = read("<svg viewBox=\"0 0 20 20\" width=\"20mm\" height=\"20mm\">\
                      <g transform=\"translate(10,0)\"/>\
                      <rect x=\"0\" y=\"0\" width=\"5\" height=\"5\"/></svg>");
        let minx = p.contours[0].iter().map(|q| q[0]).fold(f64::INFINITY, f64::min);
        assert!((minx - 0.0).abs() < 1e-9, "sibling not translated: minx={}", minx);
    }

    #[test]
    fn closepath_then_continue_keeps_initial_point() {
        // Subpath 2 (after Z, no M) restarts at the initial point (0,0), so it
        // is the triangle (0,0)-(10,10)-(0,10), area 50, plus subpath 1's line.
        let p = read("<svg width=\"10mm\" height=\"10mm\" viewBox=\"0 0 10 10\">\
                      <path d=\"M0 0 L10 0 Z L10 10 L0 10 Z\"/></svg>");
        // subpath1 (0,0)-(10,0) is a 2-pt line (dropped); subpath2 is the tri.
        assert_eq!(p.contours.len(), 1, "only the triangle survives");
        assert!((area(&p) - 50.0).abs() < 1e-9, "restarted-subpath area {}", area(&p));
    }

    #[test]
    fn non_finite_dimensions_are_dropped() {
        // NaN/inf sizes and coordinates must not produce garbage geometry.
        assert!(read("<svg viewBox=\"0 0 9 9\" width=\"9mm\" height=\"9mm\">\
                      <rect width=\"NaN\" height=\"9\"/></svg>").is_empty());
        assert!(read("<svg viewBox=\"0 0 9 9\" width=\"9mm\" height=\"9mm\">\
                      <circle cx=\"0\" cy=\"0\" r=\"inf\"/></svg>").is_empty());
        // A NaN coordinate on an otherwise-valid rect drops that contour.
        let p = read("<svg viewBox=\"0 0 9 9\" width=\"9mm\" height=\"9mm\">\
                      <rect x=\"NaN\" y=\"0\" width=\"5\" height=\"5\"/></svg>");
        assert!(p.is_empty());
    }

    #[test]
    fn multibyte_attribute_values_do_not_panic() {
        // A multi-byte id/label before the geometry attributes must not panic
        // the boundary-sensitive attribute scan.
        let p = read("<svg width=\"10mm\" height=\"10mm\" viewBox=\"0 0 10 10\">\
                      <rect id=\"café—naïve\" x=\"0\" y=\"0\" width=\"10\" height=\"10\"/></svg>");
        assert!((area(&p) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn px_units_scale_by_dpi() {
        // No physical width and a px-based viewBox: 96 user units at 96 dpi = 1 in
        // = 25.4 mm, so a full-viewBox 96×96 rect is 25.4 mm on a side.
        let p = read_svg(
            "<svg viewBox=\"0 0 96 96\"><rect x=\"0\" y=\"0\" width=\"96\" height=\"96\"/></svg>",
            96.0, 16.0, 12.0, 2.0,
        ).0;
        let a = area(&p);
        assert!((a - 25.4 * 25.4).abs() < 1e-6, "px→mm area {}", a);
    }
}
