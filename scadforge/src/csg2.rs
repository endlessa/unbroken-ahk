//! From-scratch CSG boolean kernel for 2D polygon regions.
//!
//! The exact 2D analogue of `csg.rs`: BSP-tree merging (Thibault & Naylor
//! 1987) lifted from planes-splitting-polygons in 3D to lines-splitting-
//! segments in 2D. A region's boundary is a set of directed segments wound
//! so the filled area is always on the LEFT of each edge (`poly2::
//! oriented_contours` gives exactly this: CCW solids, CW holes); each
//! segment's outward normal therefore points to its right. One region's
//! segments are classified against the other's line-BSP (front/back/
//! collinear, splitting spanning segments), and union / difference /
//! intersection are assembled from the surviving pieces by the same clip
//! sequences the 3D kernel uses.
//!
//! Preview-grade, no external geometry library. It assumes reasonably
//! clean, closed input (our primitives and the reassembler produce it) and
//! uses a fixed epsilon for line classification; it is exact for the common
//! cases and robust to shared/collinear edges. The surviving segment soup
//! is stitched back into even-odd `Poly2` contours by endpoint chaining
//! with tolerance snapping.

use crate::poly2::{self, Poly2};
use std::collections::HashMap;

/// Line-classification tolerance. 2D coordinates live in the unit-to-
/// hundreds range, matching the 3D kernel's choice.
const EPS: f64 = 1e-7;

type V2 = [f64; 2];

fn sub(a: V2, b: V2) -> V2 {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: V2, b: V2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn lerp(a: V2, b: V2, t: f64) -> V2 {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}
fn dist2(a: V2, b: V2) -> f64 {
    let d = sub(a, b);
    dot(d, d)
}

/// An oriented line: the set { p : dot(normal, p) = w }. `front` (the
/// outside of the solid) is dot(normal, p) > w.
#[derive(Clone)]
struct Line {
    normal: V2,
    w: f64,
}

impl Line {
    /// The supporting line of a directed segment a→b, with the segment's
    /// OUTWARD normal (to its right, since the filled area is on its left):
    /// for direction d = b−a the right-perpendicular is [d.y, −d.x].
    fn from_seg(a: V2, b: V2) -> Option<Line> {
        let d = sub(b, a);
        let n = [d[1], -d[0]];
        let len = dot(n, n).sqrt();
        if !len.is_finite() || len < EPS {
            return None; // zero-length or non-finite (NaN/inf coord) segment
        }
        let normal = [n[0] / len, n[1] / len];
        Some(Line { w: dot(normal, a), normal })
    }

    fn flip(&mut self) {
        self.normal = [-self.normal[0], -self.normal[1]];
        self.w = -self.w;
    }
}

/// A directed boundary segment carrying its supporting line (filled area on
/// its left).
#[derive(Clone)]
struct Seg {
    a: V2,
    b: V2,
    line: Line,
}

impl Seg {
    fn new(a: V2, b: V2) -> Option<Seg> {
        Line::from_seg(a, b).map(|line| Seg { a, b, line })
    }

    fn flip(&mut self) {
        std::mem::swap(&mut self.a, &mut self.b);
        self.line.flip();
    }
}

const COPLANAR: u8 = 0;
const FRONT: u8 = 1;
const BACK: u8 = 2;
const SPANNING: u8 = 3;

fn classify_point(line: &Line, p: V2) -> u8 {
    let t = dot(line.normal, p) - line.w;
    if t < -EPS {
        BACK
    } else if t > EPS {
        FRONT
    } else {
        COPLANAR
    }
}

/// Split `seg` by `line`, routing the (possibly split) pieces into the four
/// buckets — collinear segments whose normal agrees / disagrees with the
/// line, and the strictly-front / strictly-back parts.
fn split_segment(
    line: &Line,
    seg: &Seg,
    coplanar_front: &mut Vec<Seg>,
    coplanar_back: &mut Vec<Seg>,
    front: &mut Vec<Seg>,
    back: &mut Vec<Seg>,
) {
    let ta = classify_point(line, seg.a);
    let tb = classify_point(line, seg.b);
    match ta | tb {
        COPLANAR => {
            if dot(line.normal, seg.line.normal) > 0.0 {
                coplanar_front.push(seg.clone());
            } else {
                coplanar_back.push(seg.clone());
            }
        }
        FRONT => front.push(seg.clone()),
        BACK => back.push(seg.clone()),
        _ => {
            // SPANNING: one endpoint front, one back — cut at the crossing.
            let denom = dot(line.normal, sub(seg.b, seg.a));
            let t = (line.w - dot(line.normal, seg.a)) / denom;
            let mid = lerp(seg.a, seg.b, t);
            let (fa, fb, ba, bb) = if ta == FRONT {
                (seg.a, mid, mid, seg.b)
            } else {
                (mid, seg.b, seg.a, mid)
            };
            if let Some(mut s) = Seg::new(fa, fb) {
                s.line = seg.line.clone(); // keep the parent's exact line
                front.push(s);
            }
            if let Some(mut s) = Seg::new(ba, bb) {
                s.line = seg.line.clone();
                back.push(s);
            }
        }
    }
}

fn classify(line: &Line, seg: &Seg) -> u8 {
    classify_point(line, seg.a) | classify_point(line, seg.b)
}

/// Pick a split line that balances the segment set and minimizes splits —
/// any line is correct, this only governs tree depth and cost (mirrors
/// `csg::choose_plane`).
fn choose_line(segs: &[Seg]) -> Line {
    const MAX_CANDIDATES: usize = 24;
    const SPLIT_WEIGHT: f64 = 8.0;
    let n = segs.len();
    let step = (n / MAX_CANDIDATES).max(1);
    let mut best = segs[0].line.clone();
    let mut best_score = f64::INFINITY;
    let mut i = 0;
    while i < n {
        let line = &segs[i].line;
        let (mut front, mut back, mut splits) = (0i64, 0i64, 0i64);
        for s in segs {
            match classify(line, s) {
                FRONT => front += 1,
                BACK => back += 1,
                SPANNING => splits += 1,
                _ => {}
            }
        }
        let score = splits as f64 * SPLIT_WEIGHT + (front - back).abs() as f64;
        if score < best_score {
            best_score = score;
            best = line.clone();
        }
        i += step;
    }
    best
}

/// A 2D BSP node: `line` splits the plane, `segs` are the collinear
/// boundary pieces stored here.
struct Node {
    line: Option<Line>,
    front: Option<Box<Node>>,
    back: Option<Box<Node>>,
    segs: Vec<Seg>,
}

impl Node {
    fn new() -> Node {
        Node { line: None, front: None, back: None, segs: Vec::new() }
    }

    fn from(segs: Vec<Seg>) -> Node {
        let mut node = Node::new();
        node.build(segs);
        node
    }

    fn build(&mut self, segs: Vec<Seg>) {
        if segs.is_empty() {
            return;
        }
        if self.line.is_none() {
            self.line = Some(choose_line(&segs));
        }
        let line = self.line.clone().unwrap();
        let mut coplanar_front = Vec::new();
        let mut coplanar_back = Vec::new();
        let mut front = Vec::new();
        let mut back = Vec::new();
        for s in &segs {
            split_segment(&line, s, &mut coplanar_front, &mut coplanar_back, &mut front, &mut back);
        }
        self.segs.extend(coplanar_front);
        self.segs.extend(coplanar_back);
        if !front.is_empty() {
            self.front.get_or_insert_with(|| Box::new(Node::new())).build(front);
        }
        if !back.is_empty() {
            self.back.get_or_insert_with(|| Box::new(Node::new())).build(back);
        }
    }

    fn invert(&mut self) {
        for s in &mut self.segs {
            s.flip();
        }
        if let Some(line) = &mut self.line {
            line.flip();
        }
        if let Some(f) = &mut self.front {
            f.invert();
        }
        if let Some(b) = &mut self.back {
            b.invert();
        }
        std::mem::swap(&mut self.front, &mut self.back);
    }

    /// Return the parts of `segs` that lie OUTSIDE this solid.
    fn clip_segments(&self, segs: Vec<Seg>) -> Vec<Seg> {
        let line = match &self.line {
            Some(l) => l.clone(),
            None => return segs,
        };
        let mut coplanar_front = Vec::new();
        let mut coplanar_back = Vec::new();
        let mut front = Vec::new();
        let mut back = Vec::new();
        for s in &segs {
            split_segment(&line, s, &mut coplanar_front, &mut coplanar_back, &mut front, &mut back);
        }
        // A coincident edge is kept or dropped with the half-plane it faces.
        front.extend(coplanar_front);
        back.extend(coplanar_back);
        let front = match &self.front {
            Some(f) => f.clip_segments(front),
            None => front,
        };
        let back = match &self.back {
            Some(b) => b.clip_segments(back),
            None => Vec::new(),
        };
        let mut out = front;
        out.extend(back);
        out
    }

    fn clip_to(&mut self, other: &Node) {
        self.segs = other.clip_segments(std::mem::take(&mut self.segs));
        if let Some(f) = &mut self.front {
            f.clip_to(other);
        }
        if let Some(b) = &mut self.back {
            b.clip_to(other);
        }
    }

    fn all_segments(&self, out: &mut Vec<Seg>) {
        out.extend(self.segs.iter().cloned());
        if let Some(f) = &self.front {
            f.all_segments(out);
        }
        if let Some(b) = &self.back {
            b.all_segments(out);
        }
    }
}

enum Op {
    Union,
    Difference,
    Intersection,
}

fn boolean(a: Vec<Seg>, b: Vec<Seg>, op: Op) -> Vec<Seg> {
    // Empty-operand identities, per op (mirrors csg::boolean). An empty BSP's
    // clip is a no-op (Node::clip_segments returns its input when line is
    // None), so without this guard clipping against an empty operand would
    // RESURRECT the other operand — e.g. an empty intermediate from a
    // disjoint pairwise-fold sub-result would make n-ary intersection2 return
    // a non-empty, order-dependent wrong answer (A∩B=∅ then (∅)∩C ⇒ C).
    if a.is_empty() || b.is_empty() {
        return match op {
            Op::Union => if a.is_empty() { b } else { a },
            Op::Difference => if a.is_empty() { Vec::new() } else { a },
            Op::Intersection => Vec::new(),
        };
    }
    let mut a = Node::from(a);
    let mut b = Node::from(b);
    match op {
        Op::Union => {
            a.clip_to(&b);
            b.clip_to(&a);
            b.invert();
            b.clip_to(&a);
            b.invert();
            let mut bp = Vec::new();
            b.all_segments(&mut bp);
            a.build(bp);
        }
        Op::Difference => {
            a.invert();
            a.clip_to(&b);
            b.clip_to(&a);
            b.invert();
            b.clip_to(&a);
            b.invert();
            let mut bp = Vec::new();
            b.all_segments(&mut bp);
            a.build(bp);
            a.invert();
        }
        Op::Intersection => {
            a.invert();
            b.clip_to(&a);
            b.invert();
            a.clip_to(&b);
            b.clip_to(&a);
            let mut bp = Vec::new();
            b.all_segments(&mut bp);
            a.build(bp);
            a.invert();
        }
    }
    let mut out = Vec::new();
    a.all_segments(&mut out);
    out
}

/// The oriented boundary segments of a region (filled on the left).
fn region_segments(poly: &Poly2) -> Vec<Seg> {
    let mut segs = Vec::new();
    for contour in poly2::oriented_contours(poly) {
        // Drop any contour carrying a non-finite coordinate (e.g. a user
        // 1/0 reaching a vertex) so a NaN/inf can't poison the BSP.
        if contour.iter().any(|p| !p[0].is_finite() || !p[1].is_finite()) {
            continue;
        }
        let n = contour.len();
        for i in 0..n {
            if let Some(s) = Seg::new(contour[i], contour[(i + 1) % n]) {
                segs.push(s);
            }
        }
    }
    segs
}

/// Snap/merge tolerance for stitching the boolean output back into
/// contours. Deliberately an order above the classification EPS (1e-7): the
/// same geometric crossing computed two ways (A's segment split by B's line
/// vs B's segment split by A's line) can disagree by ~1e-9·|coord|, which is
/// ~1e-7 at the hundreds-scale coordinates this kernel targets — so a
/// tolerance below that would fail to weld coincident endpoints and leave
/// loops open. The cost is that a genuine sub-1e-6 sliver is collapsed; that
/// is below the BSP's own EPS resolution anyway (exact arithmetic, not a
/// wider snap, would be the real fix — out of scope for a preview kernel).
const SNAP: f64 = 1e-6;

/// Stitch a directed-segment soup (filled on the left) back into even-odd
/// `Poly2` contours by endpoint chaining. Endpoints are merged within SNAP
/// so a segment and the piece it was split from meet at one vertex; at a
/// junction the sharpest right (most-clockwise) turn is taken, which keeps
/// the filled region on the left and traces non-crossing loops.
fn segments_to_poly(segs: &[Seg]) -> Poly2 {
    // 1. Canonical vertices (tolerance merge over a SNAP-sized grid).
    let mut verts: Vec<V2> = Vec::new();
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for s in segs {
        let u = canon(s.a, &mut verts, &mut grid);
        let v = canon(s.b, &mut verts, &mut grid);
        if u != v {
            edges.push((u, v));
        }
    }
    if edges.is_empty() {
        return Poly2::new(Vec::new());
    }
    // 2. Adjacency: start vertex → outgoing edge indices.
    let mut out: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, &(u, _)) in edges.iter().enumerate() {
        out.entry(u).or_default().push(i);
    }
    // 3. Chain edges into closed loops.
    let mut used = vec![false; edges.len()];
    let mut contours: Vec<Vec<V2>> = Vec::new();
    for start in 0..edges.len() {
        if used[start] {
            continue;
        }
        let start_v = edges[start].0;
        let mut cur = start;
        let mut contour: Vec<V2> = Vec::new();
        let mut steps = 0;
        let mut closed = false;
        loop {
            used[cur] = true;
            let (u, v) = edges[cur];
            contour.push(verts[u]);
            steps += 1;
            if v == start_v {
                closed = true; // returned to the loop's start vertex
                break;
            }
            match next_edge(v, u, &edges, &out, &used, &verts) {
                Some(nx) => cur = nx,
                None => break, // dead end: an open chain, discarded below
            }
            if steps > edges.len() + 4 {
                break; // safety against a pathological cycle
            }
        }
        // Only CLOSED loops are real contours. An open chain (dead end, or the
        // safety break) must be dropped, not pushed — pushing it would forge a
        // closing edge from its tail back to its head that exists in neither
        // operand.
        if closed && contour.len() >= 3 {
            contours.push(contour);
        }
    }
    Poly2::new(contours)
}

/// Look up or register a canonical vertex id for `p`, merging any existing
/// vertex within SNAP (checked across the 3×3 neighbourhood of grid cells).
fn canon(p: V2, verts: &mut Vec<V2>, grid: &mut HashMap<(i64, i64), Vec<usize>>) -> usize {
    let cell = ((p[0] / SNAP).round() as i64, (p[1] / SNAP).round() as i64);
    for dx in -1..=1 {
        for dy in -1..=1 {
            if let Some(list) = grid.get(&(cell.0 + dx, cell.1 + dy)) {
                for &vi in list {
                    if dist2(verts[vi], p) <= SNAP * SNAP {
                        return vi;
                    }
                }
            }
        }
    }
    let id = verts.len();
    verts.push(p);
    grid.entry(cell).or_default().push(id);
    id
}

/// Choose the next boundary edge leaving `v` (arrived from `prev`): the one
/// making the sharpest clockwise turn from the reversed incoming direction,
/// so the filled region stays on the left. Backtracking along the incoming
/// edge is taken only as a last resort.
fn next_edge(
    v: usize,
    prev: usize,
    edges: &[(usize, usize)],
    out: &HashMap<usize, Vec<usize>>,
    used: &[bool],
    verts: &[V2],
) -> Option<usize> {
    let cands = out.get(&v)?;
    let r = sub(verts[prev], verts[v]); // points back the way we came
    let mut best = None;
    let mut best_score = f64::INFINITY;
    for &e in cands {
        if used[e] {
            continue;
        }
        let d = sub(verts[edges[e].1], verts[v]);
        let mut a = cw_angle(r, d);
        if a < 1e-9 {
            a = std::f64::consts::TAU; // exact reverse: least preferred
        }
        if a < best_score {
            best_score = a;
            best = Some(e);
        }
    }
    best
}

/// Clockwise angle in [0, τ) to rotate `from` onto `to`.
fn cw_angle(from: V2, to: V2) -> f64 {
    let mut d = from[1].atan2(from[0]) - to[1].atan2(to[0]);
    while d < 0.0 {
        d += std::f64::consts::TAU;
    }
    d
}

// -- Public API -------------------------------------------------------------

/// n-ary union of 2D regions (the boundary of the combined filled area).
/// A single region passes through unchanged; empty regions are skipped.
pub fn union2(regions: &[Poly2]) -> Poly2 {
    let mut items: Vec<Vec<Seg>> =
        regions.iter().filter(|r| !r.is_empty()).map(region_segments).collect();
    if items.is_empty() {
        return Poly2::new(Vec::new());
    }
    if items.len() == 1 {
        // Identity: don't round-trip a clean single region through the BSP.
        return regions.iter().find(|r| !r.is_empty()).unwrap().clone();
    }
    // Balanced pairwise fold (see csg::reduce_pairwise).
    while items.len() > 1 {
        let mut next = Vec::with_capacity(items.len().div_ceil(2));
        let mut i = 0;
        while i + 1 < items.len() {
            next.push(boolean(items[i].clone(), items[i + 1].clone(), Op::Union));
            i += 2;
        }
        if i < items.len() {
            next.push(items[i].clone());
        }
        items = next;
    }
    segments_to_poly(&items.into_iter().next().unwrap())
}

/// difference: the first region minus the union of the rest.
pub fn difference2(first: &Poly2, rest: &[Poly2]) -> Poly2 {
    if first.is_empty() {
        return Poly2::new(Vec::new());
    }
    let cutters = union2(rest);
    if cutters.is_empty() {
        return first.clone();
    }
    segments_to_poly(&boolean(region_segments(first), region_segments(&cutters), Op::Difference))
}

/// n-ary intersection: the region common to every operand. An empty
/// operand annihilates the result (A ∩ ∅ = ∅).
pub fn intersection2(regions: &[Poly2]) -> Poly2 {
    if regions.is_empty() || regions.iter().any(|r| r.is_empty()) {
        return Poly2::new(Vec::new());
    }
    let mut items: Vec<Vec<Seg>> = regions.iter().map(region_segments).collect();
    while items.len() > 1 {
        let mut next = Vec::with_capacity(items.len().div_ceil(2));
        let mut i = 0;
        while i + 1 < items.len() {
            next.push(boolean(items[i].clone(), items[i + 1].clone(), Op::Intersection));
            i += 2;
        }
        if i < items.len() {
            next.push(items[i].clone());
        }
        items = next;
    }
    segments_to_poly(&items.into_iter().next().unwrap())
}

// -- 2D convex hull (hull()) ------------------------------------------------

/// The 2D convex hull of every vertex of every region (Andrew's monotone
/// chain, CCW). Fewer than three non-collinear points → empty.
pub fn hull2(regions: &[Poly2]) -> Poly2 {
    let mut pts: Vec<V2> = Vec::new();
    for r in regions {
        for c in &r.contours {
            pts.extend(c.iter().filter(|p| p.iter().all(|x| x.is_finite())));
        }
    }
    let hull = convex_hull2(&pts);
    if hull.len() < 3 {
        Poly2::new(Vec::new())
    } else {
        Poly2::new(vec![hull])
    }
}

fn convex_hull2(input: &[V2]) -> Vec<V2> {
    let mut pts = input.to_vec();
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    pts.dedup();
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }
    let cross = |o: V2, a: V2, b: V2| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut hull: Vec<V2> = Vec::with_capacity(2 * n);
    // Lower hull.
    for &p in &pts {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(p);
    }
    // Upper hull.
    let lower = hull.len() + 1;
    for &p in pts.iter().rev().skip(1) {
        while hull.len() >= lower && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(p);
    }
    hull.pop(); // last point == first
    hull
}

// -- 2D Minkowski sum (minkowski()) -----------------------------------------

/// The largest pairwise vertex-sum count a single 2D Minkowski step will
/// hull before skipping (mirrors the 3D cap; 2D is cheaper).
pub const MINKOWSKI2_MAX_POINTS: usize = 60_000;

/// Outcome of a 2D Minkowski attempt, so the caller can warn precisely.
pub enum Minkowski2 {
    Ok(Poly2),
    TooLarge { count: usize, partial: Poly2 },
}

/// Minkowski sum of 2D regions, folded left. EXACT for convex operands
/// (hull of pairwise vertex sums — the dominant rounding use); concave
/// operands are over-approximated by the sum of their convex hulls (the
/// caller warns). A single operand is identity; empty operands are skipped.
pub fn minkowski2(regions: &[Poly2]) -> Minkowski2 {
    let mut iter = regions.iter().filter(|r| !r.is_empty());
    let mut acc: Vec<V2> = match iter.next() {
        Some(r) => r.contours.iter().flatten().cloned().collect(),
        None => return Minkowski2::Ok(Poly2::new(Vec::new())),
    };
    // A lone operand passes through unchanged (not re-hulled).
    let mut folded = false;
    for r in iter {
        let bpts: Vec<V2> = r.contours.iter().flatten().cloned().collect();
        let product = acc.len() * bpts.len();
        if product > MINKOWSKI2_MAX_POINTS {
            let partial = if folded {
                let h = convex_hull2(&acc);
                if h.len() >= 3 { Poly2::new(vec![h]) } else { Poly2::new(Vec::new()) }
            } else {
                regions.iter().find(|r| !r.is_empty()).unwrap().clone()
            };
            return Minkowski2::TooLarge { count: product, partial };
        }
        let mut sums = Vec::with_capacity(product);
        for &pa in &acc {
            for &pb in &bpts {
                sums.push([pa[0] + pb[0], pa[1] + pb[1]]);
            }
        }
        acc = convex_hull2(&sums);
        folded = true;
    }
    if !folded {
        // Single operand: return it unchanged.
        return Minkowski2::Ok(regions.iter().find(|r| !r.is_empty()).unwrap().clone());
    }
    if acc.len() < 3 {
        Minkowski2::Ok(Poly2::new(Vec::new()))
    } else {
        Minkowski2::Ok(Poly2::new(vec![acc]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Total filled area of an even-odd region (via its triangulation).
    fn area(poly: &Poly2) -> f64 {
        let (v, t) = poly2::triangulate(poly);
        t.iter()
            .map(|tri| {
                let (a, b, c) = (v[tri[0] as usize], v[tri[1] as usize], v[tri[2] as usize]);
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.0
            })
            .sum()
    }

    fn sq(x0: f64, y0: f64, s: f64) -> Poly2 {
        Poly2::new(vec![vec![[x0, y0], [x0 + s, y0], [x0 + s, y0 + s], [x0, y0 + s]]])
    }

    #[test]
    fn union_of_overlapping_squares_merges_area() {
        // Two unit squares overlapping in a 0.5×1 strip: area 2 − 0.5 = 1.5.
        let u = union2(&[sq(0.0, 0.0, 1.0), sq(0.5, 0.0, 1.0)]);
        assert!((area(&u) - 1.5).abs() < 1e-6, "union area {}", area(&u));
    }

    #[test]
    fn union_of_disjoint_squares_keeps_both() {
        let u = union2(&[sq(0.0, 0.0, 1.0), sq(5.0, 0.0, 1.0)]);
        assert!((area(&u) - 2.0).abs() < 1e-6, "area {}", area(&u));
        assert_eq!(u.contours.len(), 2, "two disjoint loops");
    }

    #[test]
    fn difference_carves_a_notch_and_a_hole() {
        // Unit square minus a half-overlapping square: an L of area 0.5.
        let d = difference2(&sq(0.0, 0.0, 1.0), &[sq(0.5, 0.0, 1.0)]);
        assert!((area(&d) - 0.5).abs() < 1e-6, "notch area {}", area(&d));
        // A 10×10 square minus a 4×4 square fully inside → a ring, area 84,
        // and the result carries a hole contour.
        let ring = difference2(&sq(0.0, 0.0, 10.0), &[sq(3.0, 3.0, 4.0)]);
        assert!((area(&ring) - 84.0).abs() < 1e-6, "ring area {}", area(&ring));
        assert_eq!(ring.contours.len(), 2, "outer + hole");
    }

    #[test]
    fn intersection_keeps_the_overlap() {
        // Two half-overlapping unit squares → a 0.5×1 strip, area 0.5.
        let i = intersection2(&[sq(0.0, 0.0, 1.0), sq(0.5, 0.0, 1.0)]);
        assert!((area(&i) - 0.5).abs() < 1e-6, "overlap area {}", area(&i));
        // Disjoint → empty.
        let e = intersection2(&[sq(0.0, 0.0, 1.0), sq(5.0, 0.0, 1.0)]);
        assert!(e.is_empty(), "disjoint intersection must be empty");
    }

    #[test]
    fn empty_operands_follow_identity_rules() {
        let a = sq(0.0, 0.0, 2.0);
        let empty = Poly2::new(Vec::new());
        assert!((area(&union2(&[a.clone(), empty.clone()])) - 4.0).abs() < 1e-9);
        assert!((area(&difference2(&a, &[empty.clone()])) - 4.0).abs() < 1e-9);
        assert!(difference2(&empty, &[a.clone()]).is_empty());
        assert!(intersection2(&[a.clone(), empty.clone()]).is_empty());
        assert!(intersection2(&[empty, a]).is_empty());
    }

    #[test]
    fn nary_intersection_with_a_disjoint_pair_is_empty_in_any_order() {
        // Review Finding 1: a disjoint sub-pair produces an empty INTERMEDIATE
        // in the pairwise fold; without the empty-operand guard in boolean()
        // that empty resurrected the other operand, making the result
        // non-empty AND order-dependent.
        let a = sq(0.0, 0.0, 1.0);
        let b = sq(2.0, 0.0, 1.0); // disjoint from a
        let c = sq(0.0, 0.0, 1.0); // == a
        // A∩B already empty ⇒ A∩B∩C = ∅, whatever the operand order.
        assert!(intersection2(&[a.clone(), b.clone(), c.clone()]).is_empty());
        assert!(intersection2(&[a.clone(), c.clone(), b.clone()]).is_empty());
        assert!(intersection2(&[b, a, c]).is_empty());
    }

    #[test]
    fn non_finite_coordinates_do_not_panic() {
        // A contour with a NaN/inf vertex is dropped rather than poisoning the
        // BSP or panicking (robustness lens; this is a public /render path).
        let bad = Poly2::new(vec![vec![[0.0, 0.0], [f64::NAN, 1.0], [1.0, 1.0]]]);
        let good = sq(0.0, 0.0, 4.0);
        // union with a garbage region falls back to the clean one.
        let u = union2(&[bad.clone(), good.clone()]);
        assert!((area(&u) - 16.0).abs() < 1e-6);
        // difference by garbage leaves the minuend intact.
        let d = difference2(&good, &[bad]);
        assert!((area(&d) - 16.0).abs() < 1e-6);
    }

    #[test]
    fn hull_of_two_squares_fills_the_gap() {
        // Convex hull of two unit squares 4 apart on x: an 8-wide hexagon-ish
        // shell strictly larger than the 2 units of square area.
        let h = hull2(&[sq(0.0, 0.0, 1.0), sq(4.0, 0.0, 1.0)]);
        assert!(area(&h) > 2.0, "hull area {}", area(&h));
        // Hull spans x∈[0,5].
        let xs: Vec<f64> = h.contours[0].iter().map(|p| p[0]).collect();
        let (lo, hi) = (xs.iter().cloned().fold(f64::INFINITY, f64::min),
                        xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
        assert!((lo - 0.0).abs() < 1e-9 && (hi - 5.0).abs() < 1e-9);
    }

    #[test]
    fn minkowski_grows_a_square_by_a_square() {
        // A 2×2 square ⊕ a 2×2 square (both centered) = a 4×4 square: area 16.
        let a = sq(-1.0, -1.0, 2.0);
        let b = sq(-1.0, -1.0, 2.0);
        match minkowski2(&[a, b]) {
            Minkowski2::Ok(m) => assert!((area(&m) - 16.0).abs() < 1e-6, "mink area {}", area(&m)),
            Minkowski2::TooLarge { .. } => panic!("small squares must fit the cap"),
        }
        // Single operand is identity.
        match minkowski2(&[sq(0.0, 0.0, 3.0)]) {
            Minkowski2::Ok(m) => assert!((area(&m) - 9.0).abs() < 1e-9),
            Minkowski2::TooLarge { .. } => panic!(),
        }
    }

    #[test]
    fn union_result_extrudes_cleanly() {
        // A boolean result must round-trip through the extruder: union of two
        // overlapping squares extruded 1 tall has volume == area (1.5).
        let u = union2(&[sq(0.0, 0.0, 1.0), sq(0.5, 0.0, 1.0)]);
        let (p, t) = poly2::extrude_linear(&u, 1.0, false, 0.0, 1, [1.0, 1.0]);
        let mut vol = 0.0;
        for tri in &t {
            let a = p[tri[0] as usize];
            let b = p[tri[1] as usize];
            let c = p[tri[2] as usize];
            vol += (a[0] * (b[1] * c[2] - b[2] * c[1]) + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0;
        }
        assert!((vol.abs() - 1.5).abs() < 1e-6, "extruded union vol {}", vol);
    }
}
