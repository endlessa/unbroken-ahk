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

use crate::geom::Mesh;
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

/// Deepest the segment-BSP will recurse. A healthy tree over n segments is
/// far shallower; this only bounds pathological input (see `build_at`).
const MAX_BSP_DEPTH: usize = 4096;

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
        self.build_at(segs, 0);
    }

    /// Build the tree, refusing to recurse forever.
    ///
    /// `split_segment`'s epsilon is absolute, so at large coordinate
    /// magnitudes a segment can be classified as neither coplanar with the
    /// chosen line nor cleanly on one side of it. The child then receives the
    /// SAME set its parent had, picks the same line, and recurses until the
    /// stack dies — `offset(delta = -4e9) circle(r = 1e10, $fn = 6)` aborted
    /// the whole process with SIGABRT, which a user cannot even catch.
    ///
    /// Two guards. The no-progress check is the precise one: a partition that
    /// moved nothing will never move anything, so the segments stay here as
    /// coplanar rather than recursing. The depth cap is the blunt backstop for
    /// any other path to the same place — degrading a boolean's accuracy is
    /// always better than killing the process.
    fn build_at(&mut self, segs: Vec<Seg>, depth: usize) {
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
        let stuck = front.len() == segs.len() || back.len() == segs.len();
        if depth >= MAX_BSP_DEPTH || stuck {
            self.segs.extend(front);
            self.segs.extend(back);
            return;
        }
        if !front.is_empty() {
            self.front
                .get_or_insert_with(|| Box::new(Node::new()))
                .build_at(front, depth + 1);
        }
        if !back.is_empty() {
            self.back
                .get_or_insert_with(|| Box::new(Node::new()))
                .build_at(back, depth + 1);
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
    // 3. Prune edges that cannot lie on ANY cycle. The BSP fold is not exact:
    // the same crossing computed from either operand can disagree slightly, and
    // the SNAP merge above can collapse a hair-thin segment to zero length. Both
    // leave dangling ends, so the soup is a set of cycles PLUS a few open
    // chains. Every edge of a real loop has an incoming edge at its source and
    // an outgoing edge at its target, so repeatedly dropping edges that fail
    // that test removes the open chains and leaves the cycles intact.
    let mut alive = vec![true; edges.len()];
    loop {
        let mut indeg = vec![0usize; verts.len()];
        let mut outdeg = vec![0usize; verts.len()];
        for (i, &(u, v)) in edges.iter().enumerate() {
            if alive[i] {
                outdeg[u] += 1;
                indeg[v] += 1;
            }
        }
        let mut changed = false;
        for (i, &(u, v)) in edges.iter().enumerate() {
            if alive[i] && (indeg[u] == 0 || outdeg[v] == 0) {
                alive[i] = false;
                changed = true;
            }
        }
        if !changed {
            break; // fixpoint: what remains is cycle-only
        }
    }

    // 4. Chain edges into closed loops. A traversal that dead-ends must NOT
    // consume the edges it walked: those edges usually belong to a genuine loop
    // that a different start would trace, and permanently marking them used
    // lets one bad start annihilate many good contours (the whole region could
    // come back empty). So mark speculatively and roll back on failure.
    let mut used = vec![false; edges.len()];
    for (i, &a) in alive.iter().enumerate() {
        if !a {
            used[i] = true; // pruned: never traced, never re-tried
        }
    }
    let mut contours: Vec<Vec<V2>> = Vec::new();
    let mut path: Vec<usize> = Vec::new();
    for start in 0..edges.len() {
        if used[start] {
            continue;
        }
        let start_v = edges[start].0;
        let mut cur = start;
        let mut contour: Vec<V2> = Vec::new();
        path.clear();
        let mut closed = false;
        loop {
            used[cur] = true;
            path.push(cur);
            let (u, v) = edges[cur];
            contour.push(verts[u]);
            if v == start_v {
                closed = true; // returned to the loop's start vertex
                break;
            }
            match next_edge(v, u, &edges, &out, &used, &verts) {
                Some(nx) => cur = nx,
                None => break, // dead end: an open chain, rolled back below
            }
            if path.len() > edges.len() + 4 {
                break; // safety against a pathological cycle
            }
        }
        // Only CLOSED loops are real contours. An open chain (dead end, or the
        // safety break) must be dropped, not pushed — pushing it would forge a
        // closing edge from its tail back to its head that exists in neither
        // operand — and its edges are released for another start to use.
        if closed && contour.len() >= 3 {
            contours.push(contour);
        } else {
            for &e in &path {
                used[e] = false;
            }
            used[start] = true; // don't retry this exact dead-end start
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

// -- 2D offset (offset()) ---------------------------------------------------

/// Above this input-vertex count the caller skips `offset2`: the dilation
/// unions O(V) slab+cap primitives through the BSP kernel, so a very
/// high-$fn 2D child could otherwise grind the public preview for seconds.
pub const OFFSET_MAX_VERTS: usize = 4_000;

/// Convex-corner treatment for `offset2`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Join {
    /// Round (arc) corners — the `r` mode; arcs tessellated by `frags_full`.
    Round,
    /// Straight miter corners — the `delta` mode; corners extend to their
    /// sharp intersection (capped by a generous miter limit).
    Miter,
    /// Flat-cut corners — `delta` with `chamfer = true`.
    Chamfer,
}

/// Offset (grow / shrink) a 2D region by `dist`. Positive grows the outer
/// boundary outward and shrinks holes; negative does the reverse and can
/// annihilate the region or split it into islands (both silently, per the
/// reference). `join` selects the convex-corner treatment; `frags_full` is
/// the full-circle fragment count for round arcs (from $fn/$fa/$fs on
/// |dist|).
///
/// Clean-room: positive offset is the Minkowski dilation of the filled
/// region — assembled as the union of the region, its outward edge slabs,
/// and a convex-corner cap per vertex — so concavity, holes and splits fall
/// out of the union2 kernel. Negative offset is the erosion via the
/// complement identity  erode(P) = B − dilate(B − P)  over a padded box B.
pub fn offset2(region: &Poly2, dist: f64, join: Join, frags_full: u32) -> Poly2 {
    if region.is_empty() {
        return Poly2::new(Vec::new());
    }
    if dist == 0.0 || !dist.is_finite() {
        return region.clone(); // identity (a zero/invalid offset is a no-op)
    }
    if dist > 0.0 {
        dilate(region, dist, join, frags_full)
    } else {
        // Erosion: complement P inside a box padded well beyond reach, dilate
        // the complement inward by |dist|, then subtract it back from the box.
        let (lo, hi) = region_bbox(region);
        let grow = -dist;
        let pad = grow * 2.0 + 1.0;
        let bcont = box_contour([lo[0] - pad, lo[1] - pad], [hi[0] + pad, hi[1] + pad]);
        let mut comp_contours = vec![bcont.clone()];
        comp_contours.extend(region.contours.iter().cloned());
        let comp = Poly2::new(comp_contours); // even-odd: box minus P
        let grown = dilate(&comp, grow, join, frags_full);
        difference2(&Poly2::new(vec![bcont]), &[grown])
    }
}

fn region_bbox(poly: &Poly2) -> (V2, V2) {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for c in &poly.contours {
        for p in c {
            lo[0] = lo[0].min(p[0]);
            lo[1] = lo[1].min(p[1]);
            hi[0] = hi[0].max(p[0]);
            hi[1] = hi[1].max(p[1]);
        }
    }
    (lo, hi)
}

fn box_contour(lo: V2, hi: V2) -> Vec<V2> {
    vec![[lo[0], lo[1]], [hi[0], lo[1]], [hi[0], hi[1]], [lo[0], hi[1]]]
}

/// Dilate (grow) a region by `dist > 0` with the given corner join, as the
/// union of the region, its outward edge slabs, and a cap at each convex
/// (gap-opening) vertex.
fn dilate(region: &Poly2, dist: f64, join: Join, frags_full: u32) -> Poly2 {
    // All three joins go through the direct raw-offset-curve algorithm, which
    // computes the dilation with NO boolean at all: it emits the (self-
    // intersecting) offset curve, splits it at every crossing, and keeps the
    // pieces that lie on the region's boundary.
    //
    // The union-of-slabs-and-caps construction this replaced was riddled with
    // tangencies — every piece's boundary sits at exactly `dist`, touching its
    // neighbours and never crossing them — and the segment-BSP shredded the
    // resulting cycles, returning an EMPTY region on many concave profiles.
    // It failed the standard corner-rounding idiom on 15 of 24 star cases and
    // on every gear profile at every radius. See `crate::offset`.
    let steps = frags_full.clamp(8, 1024) as usize;
    let (contours, _stats) = crate::offset::offset_polygon(&region.contours, dist, steps, join);
    Poly2::new(contours)
}

// -- Projection (projection()) ----------------------------------------------

/// Above this projected-triangle count the silhouette union is skipped (the
/// per-facet 2D union is the slow path — a public preview must not hang).
pub const PROJECT_MAX_TRIS: usize = 4_000;

/// Project a 3D mesh to 2D at z=0. `cut = false`: the full silhouette
/// (shadow) — the union of every non-vertical facet's XY projection,
/// ignoring Z entirely. `cut = true`: the planar cross-section where the
/// solid crosses z=0 — the section segments of every straddling triangle
/// plus the outline of every in-plane facet, stitched into contours (empty
/// if the solid does not reach z=0). Preview-grade limitation: for cut mode
/// the caller concatenates multiple children's facets, so two solids that
/// OVERLAP exactly at z=0 have their section loops even-odd-combined rather
/// than 3D-unioned first — correct for a single or nested solid (the common
/// case), approximate for a partial overlap.
pub fn project(mesh: &Mesh, cut: bool) -> Poly2 {
    if cut {
        project_cut(mesh)
    } else {
        project_silhouette(mesh)
    }
}

fn project_silhouette(mesh: &Mesh) -> Poly2 {
    let mut tris: Vec<Poly2> = Vec::new();
    for t in &mesh.tris {
        let a = mesh.positions[t[0] as usize];
        let b = mesh.positions[t[1] as usize];
        let c = mesh.positions[t[2] as usize];
        let (a2, b2, c2) = ([a[0], a[1]], [b[0], b[1]], [c[0], c[1]]);
        let area2 = (b2[0] - a2[0]) * (c2[1] - a2[1]) - (b2[1] - a2[1]) * (c2[0] - a2[0]);
        if !area2.is_finite() || area2.abs() < 1e-9 {
            continue; // a vertical (edge-on) or degenerate facet casts no shadow
        }
        // Store each projected facet CCW; even-odd is irrelevant since union2
        // merges them, but a consistent winding keeps the operands clean.
        let contour = if area2 > 0.0 { vec![a2, b2, c2] } else { vec![a2, c2, b2] };
        tris.push(Poly2::new(vec![contour]));
    }
    union2(&tris)
}

fn project_cut(mesh: &Mesh) -> Poly2 {
    // Tolerance for "a vertex lies on the z=0 plane". Three-way classification
    // (above / below / on) is symmetric under a Z flip — the earlier strict
    // `>0` test made a solid's BOTTOM face on the plane yield the section but
    // its TOP face on the plane yield empty (identical geometry, opposite
    // answer). A triangle that STRADDLES (verts strictly on both sides) emits
    // a crossing segment; a triangle lying IN the plane contributes its
    // projected outline directly, so a face-on-plane cuts the same region from
    // either side.
    const ONZ: f64 = 1e-9;
    let mut segs: Vec<Seg> = Vec::new();
    let mut in_plane: Vec<Poly2> = Vec::new();
    for t in &mesh.tris {
        let v = [
            mesh.positions[t[0] as usize],
            mesh.positions[t[1] as usize],
            mesh.positions[t[2] as usize],
        ];
        let above = v.iter().filter(|p| p[2] > ONZ).count();
        let below = v.iter().filter(|p| p[2] < -ONZ).count();
        if above == 0 && below == 0 {
            // Facet lies in the plane: its projection is part of the section.
            let (a2, b2, c2) = ([v[0][0], v[0][1]], [v[1][0], v[1][1]], [v[2][0], v[2][1]]);
            let area2 = (b2[0] - a2[0]) * (c2[1] - a2[1]) - (b2[1] - a2[1]) * (c2[0] - a2[0]);
            if area2.abs() > ONZ {
                let contour = if area2 > 0.0 { vec![a2, b2, c2] } else { vec![a2, c2, b2] };
                in_plane.push(Poly2::new(vec![contour]));
            }
            continue;
        }
        if above == 0 || below == 0 {
            continue; // wholly on one side (a mere tangent touch is not a cut)
        }
        // Straddling: the two points where the plane crosses the triangle's
        // boundary (a vertex exactly on the plane is one of them).
        let mut pts: Vec<V2> = Vec::new();
        for e in 0..3 {
            let p0 = v[e];
            let p1 = v[(e + 1) % 3];
            let (z0, z1) = (p0[2], p1[2]);
            if (z0 > ONZ && z1 < -ONZ) || (z0 < -ONZ && z1 > ONZ) {
                let tt = z0 / (z0 - z1);
                pts.push([p0[0] + (p1[0] - p0[0]) * tt, p0[1] + (p1[1] - p0[1]) * tt]);
            } else if z0.abs() <= ONZ && z1.abs() > ONZ {
                pts.push([p0[0], p0[1]]); // edge starts on the plane
            }
        }
        // Dedup to two distinct endpoints (an on-plane vertex is shared).
        pts.dedup_by(|a, b| dist2(*a, *b) <= ONZ * ONZ);
        if pts.len() > 1 && dist2(pts[0], pts[pts.len() - 1]) <= ONZ * ONZ {
            pts.pop();
        }
        if pts.len() != 2 {
            continue;
        }
        // Orient the section segment by the triangle's normal so the stitched
        // loops wind consistently: dir = N × ẑ = (Ny, −Nx).
        let e1 = [v[1][0] - v[0][0], v[1][1] - v[0][1], v[1][2] - v[0][2]];
        let e2 = [v[2][0] - v[0][0], v[2][1] - v[0][1], v[2][2] - v[0][2]];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let dir = [n[1], -n[0]];
        let (pa, pb) = (pts[0], pts[1]);
        let along = (pb[0] - pa[0]) * dir[0] + (pb[1] - pa[1]) * dir[1];
        let (a, b) = if along >= 0.0 { (pa, pb) } else { (pb, pa) };
        if let Some(s) = Seg::new(a, b) {
            segs.push(s);
        }
    }
    let straddle = segments_to_poly(&segs);
    // A face-in-plane section and any straddle section unite into one region.
    if in_plane.is_empty() {
        return straddle;
    }
    let mut parts = in_plane;
    if !straddle.is_empty() {
        parts.push(straddle);
    }
    union2(&parts)
}

#[cfg(test)]
mod tests {

    /// End-to-end through the public offset2(): the corner-rounding idiom
    /// `offset(r=k) offset(delta=-k)` on a star used to return an EMPTY region
    /// for most k (15 of 24 star/radius combinations failed, and every gear
    /// profile failed at every radius). Round joins now go through the direct
    /// raw-offset-curve algorithm in `crate::offset`.
    #[test]
    fn rounding_idiom_survives_on_star_and_gear_profiles() {
        let ring = |n: usize, r_out: f64, r_in: f64| -> Poly2 {
            let pts: Vec<V2> = (0..2 * n)
                .map(|i| {
                    let a = (i as f64) * 180.0 / (n as f64);
                    let r = if i % 2 == 0 { r_out } else { r_in };
                    [r * a.to_radians().cos(), r * a.to_radians().sin()]
                })
                .collect();
            Poly2::new(vec![pts])
        };
        // Every join for the shrink AND every join for the re-grow: the
        // erosion half runs the SAME dilation on the complement, so a join
        // that is broken outward is broken inward too — which is exactly how
        // miter kept failing here after round was fixed.
        let joins = [Join::Round, Join::Miter, Join::Chamfer];
        for &(n, ro, ri) in &[(4usize, 17.0, 8.5), (6, 17.0, 8.5), (12, 18.0, 13.0)] {
            let src = ring(n, ro, ri);
            for &k in &[1.0f64, 2.0, 3.0] {
                for &shrink in &joins {
                    for &grow in &joins {
                        let eroded = offset2(&src, -k, shrink, 48);
                        let rounded = offset2(&eroded, k, grow, 48);
                        assert!(
                            !rounded.is_empty(),
                            "{}-point profile, k={}, {:?}->{:?}: idiom returned EMPTY",
                            n,
                            k,
                            shrink,
                            grow
                        );
                        // Rounding must not blow the shape up or collapse it.
                        let a_src = poly2::signed_area2(&src.contours[0]).abs() / 2.0;
                        let a_out: f64 = rounded
                            .contours
                            .iter()
                            .map(|c| poly2::signed_area2(c).abs() / 2.0)
                            .sum();
                        assert!(
                            a_out > 0.2 * a_src && a_out < 1.8 * a_src,
                            "{}-point k={} {:?}->{:?}: area {} vs source {}",
                            n,
                            k,
                            shrink,
                            grow,
                            a_out,
                            a_src
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn erosion_survives_large_coordinates() {
        // REGRESSION. `offset(delta = -4e9) circle(r = 1e10, $fn = 6)` aborted
        // the process: the erosion's padded-box complement drove the
        // segment-BSP into unbounded recursion at that coordinate magnitude.
        for mag in [1.0e6f64, 1.0e9, 1.0e10, 1.0e12] {
            let hex: Vec<V2> = (0..6)
                .map(|i| {
                    let a = std::f64::consts::PI * (i as f64) / 3.0;
                    [mag * a.cos(), mag * a.sin()]
                })
                .collect();
            let region = Poly2::new(vec![hex]);
            let out = offset2(&region, -0.4 * mag, Join::Round, 24);
            assert!(
                out.contours.iter().flatten().all(|p| p.iter().all(|c| c.is_finite())),
                "magnitude {mag}: non-finite vertices in the eroded region"
            );
        }
    }

    /// A dangling in-edge feeding a genuine loop must not destroy the loop.
    /// The BSP fold is not exact, so its segment soup is occasionally a set of
    /// cycles PLUS a few open chains. Tracing from an edge on such a chain
    /// walks into the loop and dead-ends; if that traversal kept the edges it
    /// consumed, the real contour would vanish and the whole region could come
    /// back EMPTY. Pruning non-cycle edges (and rolling back failed walks)
    /// keeps the loop.
    #[test]
    fn stitcher_keeps_loops_reachable_from_a_dangling_chain() {
        let sq = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let mut segs = Vec::new();
        // A dangling edge that feeds INTO the square's first vertex. Listed
        // first so the tracer starts on it and walks into the loop.
        segs.push(Seg::new([-7.0, -7.0], sq[0]).unwrap());
        for i in 0..4 {
            segs.push(Seg::new(sq[i], sq[(i + 1) % 4]).unwrap());
        }
        let out = segments_to_poly(&segs);
        assert_eq!(out.contours.len(), 1, "the square survives the dangling chain");
        assert!(
            (poly2::signed_area2(&out.contours[0]).abs() / 2.0 - 100.0).abs() < 1e-9,
            "area {}",
            poly2::signed_area2(&out.contours[0]).abs() / 2.0
        );
        // The dangling vertex must not appear in the traced contour.
        assert!(
            !out.contours[0].iter().any(|p| p[0] < -1.0),
            "open chain excluded: {:?}",
            out.contours[0]
        );
    }

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
    fn offset_grows_shrinks_and_joins_a_square() {
        let s = sq(0.0, 0.0, 10.0);
        // Round r=2: 100 (square) + 80 (four 10×2 edge strips) + π·2² (four
        // quarter-circle corners).
        let r = offset2(&s, 2.0, Join::Round, 64);
        let want = 100.0 + 80.0 + std::f64::consts::PI * 4.0;
        assert!((area(&r) - want).abs() < 1.0, "round area {} want {}", area(&r), want);
        // Miter delta=2: square corners → full 2×2 corner squares → 196.
        let m = offset2(&s, 2.0, Join::Miter, 0);
        assert!((area(&m) - 196.0).abs() < 0.5, "miter area {}", area(&m));
        // Chamfer delta=2: corners cut flat → half of each 2×2 → 188.
        let c = offset2(&s, 2.0, Join::Chamfer, 0);
        assert!((area(&c) - 188.0).abs() < 0.5, "chamfer area {}", area(&c));
        // Negative delta=-2: erode to a 6×6 square → 36.
        let e = offset2(&s, -2.0, Join::Miter, 0);
        assert!((area(&e) - 36.0).abs() < 0.5, "erode area {}", area(&e));
        assert_eq!(e.contours.len(), 1, "eroded square is one loop");
        // Over-erosion annihilates silently: a 3×3 eroded by 2 vanishes.
        let gone = offset2(&sq(0.0, 0.0, 3.0), -2.0, Join::Miter, 0);
        assert!(gone.is_empty() || area(&gone) < 1e-6, "should annihilate, area {}", area(&gone));
    }

    #[test]
    fn offset_grows_outer_and_shrinks_holes() {
        // A 10×10 ring with a 4×4 hole (area 84). A positive offset grows the
        // outer boundary and shrinks the hole, so the filled area rises and
        // the hole survives (outer + hole = two contours).
        let ring = difference2(&sq(0.0, 0.0, 10.0), &[sq(3.0, 3.0, 4.0)]);
        assert!((area(&ring) - 84.0).abs() < 1e-6);
        let grown = offset2(&ring, 1.0, Join::Round, 48);
        assert!(area(&grown) > 84.0, "offset ring area {} should exceed 84", area(&grown));
        assert_eq!(grown.contours.len(), 2, "outer + shrunken hole");
    }

    #[test]
    fn projection_silhouette_and_cut() {
        let cube = crate::geom::cube([10.0, 10.0, 10.0], true);
        // Silhouette (shadow) of a centered cube → a 10×10 square.
        let sil = project(&cube, false);
        assert!((area(&sil) - 100.0).abs() < 1e-6, "silhouette area {}", area(&sil));
        // cut=true at z=0 (the cube straddles the plane) → the 10×10 section.
        let cut = project(&cube, true);
        assert!((area(&cut) - 100.0).abs() < 1e-6, "cut area {}", area(&cut));
        // cut=false ignores Z entirely: lift the cube to z∈[45,55], same shadow.
        let mut high = cube.clone();
        for p in &mut high.positions {
            p[2] += 50.0;
        }
        assert!((area(&project(&high, false)) - 100.0).abs() < 1e-6, "Z must be ignored");
        // cut=true of the lifted cube (entirely above z=0) → empty, silently.
        assert!(project(&high, true).is_empty(), "cut above the plane is empty");

        // Tangency symmetry (review finding): a cube resting its BOTTOM face on
        // z=0 and the same cube resting its TOP face on z=0 must both cut the
        // full 100-area face — the earlier strict `>0` test gave 100 vs 0.
        let bottom = crate::geom::cube([10.0, 10.0, 10.0], false); // z ∈ [0,10]
        let mut top = bottom.clone();
        for p in &mut top.positions {
            p[2] -= 10.0; // z ∈ [-10,0]
        }
        let (ab, at) = (area(&project(&bottom, true)), area(&project(&top, true)));
        assert!(
            (ab - 100.0).abs() < 1e-6 && (at - 100.0).abs() < 1e-6,
            "face-on-plane cut must be Z-flip symmetric: bottom {} vs top {}",
            ab,
            at
        );
    }

    #[test]
    fn offset_zero_and_empty_are_identity() {
        let s = sq(0.0, 0.0, 5.0);
        assert!((area(&offset2(&s, 0.0, Join::Round, 16)) - 25.0).abs() < 1e-9);
        assert!(offset2(&Poly2::new(Vec::new()), 3.0, Join::Round, 16).is_empty());
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
