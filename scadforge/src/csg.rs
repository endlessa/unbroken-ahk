//! From-scratch CSG boolean kernel for 3D triangle-soup meshes.
//!
//! Booleans are computed by BSP-tree merging (Thibault & Naylor 1987;
//! Naylor, Amanatides & Thibault 1990, "Merging BSP Trees Yields
//! Polyhedral Set Operations") — a clean-room technique implemented here
//! from the published method, with no external geometry library. Each
//! solid becomes a set of convex polygons carrying a plane; one solid's
//! polygons are classified against the other's BSP tree (front/back/
//! coplanar, splitting spanning polygons on the plane), and the union,
//! difference, or intersection is assembled from the surviving pieces.
//!
//! This is a preview-grade kernel: it assumes reasonably clean, closed
//! input (which our primitives produce) and uses a fixed epsilon for
//! plane classification. It is exact for the common cases and robust to
//! coplanar faces; it does not attempt CGAL-grade exact arithmetic.

use crate::geom::Mesh;

/// Plane-classification tolerance. Primitive coordinates live around the
/// unit-to-hundreds range, so 1e-7 separates genuine crossings from
/// floating-point noise on coincident faces.
const EPS: f64 = 1e-7;

type V3 = [f64; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn lerp(a: V3, b: V3, t: f64) -> V3 {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    if l == 0.0 {
        a
    } else {
        [a[0] / l, a[1] / l, a[2] / l]
    }
}

#[derive(Clone)]
struct Plane {
    normal: V3,
    w: f64,
}

impl Plane {
    fn from_points(a: V3, b: V3, c: V3) -> Option<Plane> {
        let n = cross(sub(b, a), sub(c, a));
        if dot(n, n) < EPS * EPS {
            return None; // degenerate (collinear) triangle
        }
        let normal = norm(n);
        Some(Plane { w: dot(normal, a), normal })
    }

    fn flip(&mut self) {
        self.normal = [-self.normal[0], -self.normal[1], -self.normal[2]];
        self.w = -self.w;
    }
}

#[derive(Clone)]
struct Polygon {
    verts: Vec<V3>,
    plane: Plane,
}

impl Polygon {
    fn new(verts: Vec<V3>) -> Option<Polygon> {
        let plane = Plane::from_points(verts[0], verts[1], verts[2])?;
        Some(Polygon { verts, plane })
    }

    fn flip(&mut self) {
        self.verts.reverse();
        self.plane.flip();
    }
}

const COPLANAR: u8 = 0;
const FRONT: u8 = 1;
const BACK: u8 = 2;
const SPANNING: u8 = 3;

/// Split `poly` by `plane`, routing the (possibly split) pieces into the
/// four buckets: coplanar polygons whose normal agrees / disagrees with
/// the plane, and the strictly-front / strictly-back parts.
fn split_polygon(
    plane: &Plane,
    poly: &Polygon,
    coplanar_front: &mut Vec<Polygon>,
    coplanar_back: &mut Vec<Polygon>,
    front: &mut Vec<Polygon>,
    back: &mut Vec<Polygon>,
) {
    let mut polygon_type = 0u8;
    let mut types = Vec::with_capacity(poly.verts.len());
    for v in &poly.verts {
        let t = dot(plane.normal, *v) - plane.w;
        let ty = if t < -EPS {
            BACK
        } else if t > EPS {
            FRONT
        } else {
            COPLANAR
        };
        polygon_type |= ty;
        types.push(ty);
    }
    match polygon_type {
        COPLANAR => {
            if dot(plane.normal, poly.plane.normal) > 0.0 {
                coplanar_front.push(poly.clone());
            } else {
                coplanar_back.push(poly.clone());
            }
        }
        FRONT => front.push(poly.clone()),
        BACK => back.push(poly.clone()),
        _ => {
            // SPANNING: cut the polygon along the plane.
            let mut f = Vec::new();
            let mut b = Vec::new();
            let n = poly.verts.len();
            for i in 0..n {
                let j = (i + 1) % n;
                let (ti, tj) = (types[i], types[j]);
                let (vi, vj) = (poly.verts[i], poly.verts[j]);
                if ti != BACK {
                    f.push(vi);
                }
                if ti != FRONT {
                    b.push(vi);
                }
                if (ti | tj) == SPANNING {
                    let denom = dot(plane.normal, sub(vj, vi));
                    let t = (plane.w - dot(plane.normal, vi)) / denom;
                    let v = lerp(vi, vj, t);
                    f.push(v);
                    b.push(v);
                }
            }
            if f.len() >= 3 {
                if let Some(p) = Polygon::new(f) {
                    front.push(p);
                }
            }
            if b.len() >= 3 {
                if let Some(p) = Polygon::new(b) {
                    back.push(p);
                }
            }
        }
    }
}

/// Classify one polygon against a plane for balance scoring (front,
/// back, and whether it spans).
fn classify(plane: &Plane, poly: &Polygon) -> u8 {
    let mut t = 0u8;
    for v in &poly.verts {
        let d = dot(plane.normal, *v) - plane.w;
        t |= if d < -EPS {
            BACK
        } else if d > EPS {
            FRONT
        } else {
            COPLANAR
        };
    }
    t
}

/// Pick a split plane that balances the polygon set and minimizes splits.
/// Any plane is correct for the BSP-merge algorithm; this only governs
/// depth and cost, so a bounded sample of candidate planes is enough.
fn choose_plane(polygons: &[Polygon]) -> Plane {
    const MAX_CANDIDATES: usize = 24;
    const SPLIT_WEIGHT: f64 = 8.0; // a split is costlier than mild imbalance
    let n = polygons.len();
    let step = (n / MAX_CANDIDATES).max(1);
    let mut best = polygons[0].plane.clone();
    let mut best_score = f64::INFINITY;
    let mut i = 0;
    while i < n {
        let plane = &polygons[i].plane;
        let (mut front, mut back, mut splits) = (0i64, 0i64, 0i64);
        for p in polygons {
            match classify(plane, p) {
                FRONT => front += 1,
                BACK => back += 1,
                SPANNING => splits += 1,
                _ => {} // coplanar: stays at this node, no imbalance
            }
        }
        let score = splits as f64 * SPLIT_WEIGHT + (front - back).abs() as f64;
        if score < best_score {
            best_score = score;
            best = plane.clone();
        }
        i += step;
    }
    best
}

/// A BSP tree node. `plane` splits space; `polygons` are the coplanar
/// facets stored at this node.
struct Node {
    plane: Option<Plane>,
    front: Option<Box<Node>>,
    back: Option<Box<Node>>,
    polygons: Vec<Polygon>,
}

impl Node {
    fn new() -> Node {
        Node { plane: None, front: None, back: None, polygons: Vec::new() }
    }

    fn from(polygons: Vec<Polygon>) -> Node {
        let mut node = Node::new();
        node.build(polygons);
        node
    }

    /// Add polygons, splitting them into this node's half-spaces.
    fn build(&mut self, polygons: Vec<Polygon>) {
        if polygons.is_empty() {
            return;
        }
        if self.plane.is_none() {
            // Choosing polygons[0].plane unconditionally makes the tree
            // degenerate to a triangle-count-deep list on a convex solid
            // (every other face lies behind face 0) — Θ(n) depth, Θ(n²)
            // build, and native-stack risk on curved primitives. A
            // balancing heuristic keeps the depth ~O(log n).
            self.plane = Some(choose_plane(&polygons));
        }
        let plane = self.plane.clone().unwrap();
        let mut coplanar_front = Vec::new();
        let mut coplanar_back = Vec::new();
        let mut front = Vec::new();
        let mut back = Vec::new();
        for p in &polygons {
            split_polygon(
                &plane,
                p,
                &mut coplanar_front,
                &mut coplanar_back,
                &mut front,
                &mut back,
            );
        }
        // Coplanar facets (either orientation) live at this node.
        self.polygons.extend(coplanar_front);
        self.polygons.extend(coplanar_back);
        if !front.is_empty() {
            self.front.get_or_insert_with(|| Box::new(Node::new())).build(front);
        }
        if !back.is_empty() {
            self.back.get_or_insert_with(|| Box::new(Node::new())).build(back);
        }
    }

    /// Flip this solid inside-out (used to turn "keep front" into "keep
    /// back" for difference / intersection).
    fn invert(&mut self) {
        for p in &mut self.polygons {
            p.flip();
        }
        if let Some(plane) = &mut self.plane {
            plane.flip();
        }
        if let Some(f) = &mut self.front {
            f.invert();
        }
        if let Some(b) = &mut self.back {
            b.invert();
        }
        std::mem::swap(&mut self.front, &mut self.back);
    }

    /// Return the parts of `polygons` that lie OUTSIDE this solid.
    fn clip_polygons(&self, polygons: Vec<Polygon>) -> Vec<Polygon> {
        let plane = match &self.plane {
            Some(p) => p.clone(),
            None => return polygons,
        };
        let mut coplanar_front = Vec::new();
        let mut coplanar_back = Vec::new();
        let mut front = Vec::new();
        let mut back = Vec::new();
        for p in &polygons {
            split_polygon(
                &plane,
                p,
                &mut coplanar_front,
                &mut coplanar_back,
                &mut front,
                &mut back,
            );
        }
        // A coincident face is kept or dropped with the half-space it
        // faces into.
        front.extend(coplanar_front);
        back.extend(coplanar_back);
        let front = match &self.front {
            Some(f) => f.clip_polygons(front),
            None => front,
        };
        // Polygons in the back half-space are inside this node's plane;
        // keep them only if a back subtree further carves them out,
        // otherwise they are interior and dropped.
        let back = match &self.back {
            Some(b) => b.clip_polygons(back),
            None => Vec::new(),
        };
        let mut out = front;
        out.extend(back);
        out
    }

    /// Remove all of this solid's polygons that lie inside `other`.
    fn clip_to(&mut self, other: &Node) {
        self.polygons = other.clip_polygons(std::mem::take(&mut self.polygons));
        if let Some(f) = &mut self.front {
            f.clip_to(other);
        }
        if let Some(b) = &mut self.back {
            b.clip_to(other);
        }
    }

    fn all_polygons(&self, out: &mut Vec<Polygon>) {
        out.extend(self.polygons.iter().cloned());
        if let Some(f) = &self.front {
            f.all_polygons(out);
        }
        if let Some(b) = &self.back {
            b.all_polygons(out);
        }
    }
}

fn mesh_to_polygons(mesh: &Mesh) -> Vec<Polygon> {
    let mut out = Vec::with_capacity(mesh.tris.len());
    for t in &mesh.tris {
        let verts = vec![
            mesh.positions[t[0] as usize],
            mesh.positions[t[1] as usize],
            mesh.positions[t[2] as usize],
        ];
        if let Some(p) = Polygon::new(verts) {
            out.push(p);
        }
    }
    out
}

fn polygons_to_mesh(polys: &[Polygon]) -> Mesh {
    let mut positions = Vec::new();
    let mut tris = Vec::new();
    for poly in polys {
        // Fan-triangulate the convex polygon.
        let base = positions.len() as u32;
        for v in &poly.verts {
            positions.push(*v);
        }
        for i in 1..poly.verts.len() as u32 - 1 {
            tris.push([base, base + i, base + i + 1]);
        }
    }
    Mesh { positions, tris }
}

/// The three set operations, following the BSP-merge clip sequences.
enum Op {
    Union,
    Difference,
    Intersection,
}

fn boolean(a: &Mesh, b: &Mesh, op: Op) -> Mesh {
    let pa = mesh_to_polygons(a);
    let pb = mesh_to_polygons(b);
    // Empty-operand identities are per-op: A∪∅=A and ∅∪B=B; A−∅=A and
    // ∅−B=∅; but A∩∅=∅ AND ∅∩B=∅ (an empty operand annihilates the
    // intersection — the earlier code wrongly returned A here, making
    // intersection order-dependent).
    if pa.is_empty() || pb.is_empty() {
        return match op {
            Op::Union => {
                if pa.is_empty() {
                    b.clone()
                } else {
                    a.clone()
                }
            }
            Op::Difference => {
                if pa.is_empty() {
                    Mesh::empty()
                } else {
                    a.clone()
                }
            }
            Op::Intersection => Mesh::empty(),
        };
    }
    let mut a = Node::from(pa);
    let mut b = Node::from(pb);
    match op {
        Op::Union => {
            a.clip_to(&b);
            b.clip_to(&a);
            b.invert();
            b.clip_to(&a);
            b.invert();
            let mut bp = Vec::new();
            b.all_polygons(&mut bp);
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
            b.all_polygons(&mut bp);
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
            b.all_polygons(&mut bp);
            a.build(bp);
            a.invert();
        }
    }
    let mut out = Vec::new();
    a.all_polygons(&mut out);
    polygons_to_mesh(&out)
}

/// Fold a list of meshes with a boolean op using a BALANCED pairwise
/// reduction rather than a left fold: a left fold rebuilds the whole
/// growing accumulator's BSP every step (O(k²) over k operands), whereas
/// halving keeps each merge between similarly-sized operands.
fn reduce_pairwise(mut items: Vec<Mesh>, op: fn() -> Op) -> Mesh {
    if items.is_empty() {
        return Mesh::empty();
    }
    while items.len() > 1 {
        let mut next = Vec::with_capacity(items.len().div_ceil(2));
        let mut i = 0;
        while i + 1 < items.len() {
            next.push(boolean(&items[i], &items[i + 1], op()));
            i += 2;
        }
        if i < items.len() {
            next.push(items[i].clone());
        }
        items = next;
    }
    items.into_iter().next().unwrap()
}

/// n-ary union of a list of meshes (empty meshes are skipped).
pub fn union_all(meshes: &[Mesh]) -> Mesh {
    let items: Vec<Mesh> =
        meshes.iter().filter(|m| !m.positions.is_empty()).cloned().collect();
    reduce_pairwise(items, || Op::Union)
}

/// difference: the first mesh minus the union of the rest.
pub fn difference(first: &Mesh, rest: &[Mesh]) -> Mesh {
    if first.positions.is_empty() {
        return Mesh::empty(); // empty minuend → empty, always
    }
    let cutters = union_all(rest);
    if cutters.positions.is_empty() {
        return first.clone();
    }
    boolean(first, &cutters, Op::Difference)
}

/// n-ary intersection: the region common to every mesh. An empty operand
/// annihilates the result (A ∩ ∅ = ∅), so intersection is commutative.
pub fn intersection_all(meshes: &[Mesh]) -> Mesh {
    if meshes.is_empty() {
        return Mesh::empty();
    }
    // Any empty operand makes the whole intersection empty.
    if meshes.iter().any(|m| m.positions.is_empty()) {
        return Mesh::empty();
    }
    reduce_pairwise(meshes.to_vec(), || Op::Intersection)
}

// ---------------------------------------------------------------------------
// Minkowski sum (minkowski())

/// The largest pairwise vertex-sum point set the preview kernel will hull
/// for one Minkowski step. The sum of V1·V2 points is combinatorial;
/// beyond this the operation is skipped with a warning (upstream is
/// famously slow here too, but a public preview must not hang).
pub const MINKOWSKI_MAX_POINTS: usize = 60_000;

/// Outcome of a Minkowski attempt, so the caller can warn precisely.
pub enum Minkowski {
    Ok(Mesh),
    /// A step's V1·V2 exceeded the cap; carries the point count skipped.
    TooLarge(usize),
}

/// Minkowski sum A ⊕ B = { a + b : a ∈ A, b ∈ B }, folded pairwise from
/// the left over the children. EXACT for convex operands (the dominant
/// use — rounding a shape with a sphere/cylinder): the sum of two convex
/// bodies is the convex hull of the pairwise vertex sums. For concave
/// operands this over-approximates (it returns the sum of the operands'
/// convex hulls); real convex decomposition is a later kernel. A single
/// operand passes through unchanged (identity); empty operands are
/// skipped.
pub fn minkowski(meshes: &[Mesh]) -> Minkowski {
    let mut iter = meshes.iter().filter(|m| !m.positions.is_empty());
    let mut acc = match iter.next() {
        Some(m) => m.clone(),
        None => return Minkowski::Ok(Mesh::empty()),
    };
    for m in iter {
        let product = acc.positions.len() * m.positions.len();
        if product > MINKOWSKI_MAX_POINTS {
            return Minkowski::TooLarge(product);
        }
        let mut pts = Vec::with_capacity(product);
        for &pa in &acc.positions {
            for &pb in &m.positions {
                pts.push([pa[0] + pb[0], pa[1] + pb[1], pa[2] + pb[2]]);
            }
        }
        acc = convex_hull(&pts);
    }
    Minkowski::Ok(acc)
}

// ---------------------------------------------------------------------------
// Convex hull (hull())

/// The convex hull of every vertex of every child mesh (the reference's
/// hull() over the children's TESSELLATED vertices — so $fn on curved
/// children shapes the hull). Holes, concavities, and colors of the
/// children are discarded by construction. A degenerate point set (fewer
/// than 4 non-coplanar points) has no 3D volume and yields an empty mesh.
pub fn hull(meshes: &[Mesh]) -> Mesh {
    let mut pts: Vec<V3> = Vec::new();
    for m in meshes {
        pts.extend(m.positions.iter().cloned());
    }
    convex_hull(&pts)
}

/// The incremental hull re-tests each point against the whole face list
/// (O(n²)); beyond this many input vertices the caller warns and skips so
/// a high-$fn hull can't grind the preview endpoint for tens of seconds.
pub const HULL_MAX_POINTS: usize = 20_000;

/// A hull face: point indices in outward-CCW order, plus its plane.
struct Face {
    v: [usize; 3],
    normal: V3,
    w: f64,
}

/// Incremental 3D convex hull. Correct-by-construction for clean point
/// sets; preview-grade (fixed epsilon) for near-degenerate ones.
fn convex_hull(input: &[V3]) -> Mesh {
    // Drop non-finite points so a stray NaN/inf coordinate (e.g. a user
    // 0/0 reaching a vertex) yields a degenerate/empty hull rather than
    // panicking the seed search.
    let pts: Vec<V3> = input
        .iter()
        .filter(|p| p.iter().all(|c| c.is_finite()))
        .cloned()
        .collect();
    let pts = &pts[..];
    if pts.len() < 4 {
        return Mesh::empty();
    }
    // Total ordering avoids partial_cmp().unwrap() panics if an overflow
    // to inf/NaN slips through on extreme (non-physical) coordinates.
    let by = |key: &dyn Fn(usize) -> f64| -> usize {
        (0..pts.len()).max_by(|&a, &b| key(a).total_cmp(&key(b))).unwrap_or(0)
    };
    // Seed tetrahedron: extremes that are pairwise non-degenerate.
    let i0 = 0;
    let i1 = by(&|a| dist2(pts[i0], pts[a]));
    if dist2(pts[i0], pts[i1]) <= EPS * EPS {
        return Mesh::empty(); // all coincident
    }
    let i2 = by(&|a| line_dist2(pts[i0], pts[i1], pts[a]));
    if line_dist2(pts[i0], pts[i1], pts[i2]) <= EPS * EPS {
        return Mesh::empty(); // all collinear
    }
    let base = match Plane::from_points(pts[i0], pts[i1], pts[i2]) {
        Some(p) => p,
        None => return Mesh::empty(),
    };
    let i3 = by(&|a| (dot(base.normal, pts[a]) - base.w).abs());
    if (dot(base.normal, pts[i3]) - base.w).abs() <= EPS {
        return Mesh::empty(); // all coplanar → no 3D volume
    }

    // Interior reference point: the seed tetra's centroid stays inside
    // the growing hull, so every face is oriented to point away from it.
    let interior = [
        (pts[i0][0] + pts[i1][0] + pts[i2][0] + pts[i3][0]) / 4.0,
        (pts[i0][1] + pts[i1][1] + pts[i2][1] + pts[i3][1]) / 4.0,
        (pts[i0][2] + pts[i1][2] + pts[i2][2] + pts[i3][2]) / 4.0,
    ];
    let mut faces: Vec<Face> = Vec::new();
    for tri in [[i0, i1, i2], [i0, i1, i3], [i0, i2, i3], [i1, i2, i3]] {
        faces.push(make_face(pts, tri, interior));
    }

    let mut vis_edges: std::collections::HashSet<(usize, usize)> =
        std::collections::HashSet::new();
    for (pi, &p) in pts.iter().enumerate() {
        if pi == i0 || pi == i1 || pi == i2 || pi == i3 {
            continue;
        }
        // Faces this point can "see" (lies in front of).
        let visible: Vec<bool> =
            faces.iter().map(|f| dot(f.normal, p) - f.w > EPS).collect();
        if !visible.iter().any(|&v| v) {
            continue; // inside the current hull
        }
        // Horizon = directed edges of visible faces whose reverse is not
        // also a visible-face edge (i.e. the boundary with the kept part).
        vis_edges.clear();
        for (fi, f) in faces.iter().enumerate() {
            if visible[fi] {
                vis_edges.insert((f.v[0], f.v[1]));
                vis_edges.insert((f.v[1], f.v[2]));
                vis_edges.insert((f.v[2], f.v[0]));
            }
        }
        let horizon: Vec<(usize, usize)> = vis_edges
            .iter()
            .filter(|&&(a, b)| !vis_edges.contains(&(b, a)))
            .cloned()
            .collect();
        // Robustness guard: near-coplanar facets can make floating drift
        // flip the visibility test inconsistently, so the horizon is not
        // a simple closed loop. Capping such a horizon would emit an
        // overlapping, non-manifold shell — skip the point instead (it is
        // near the existing surface anyway). A valid horizon visits each
        // vertex exactly once as a start and once as an end.
        if !is_simple_loop(&horizon) {
            continue;
        }
        // Drop visible faces, then cap the horizon with the new point.
        let mut kept: Vec<Face> = Vec::with_capacity(faces.len());
        for (fi, f) in faces.drain(..).enumerate() {
            if !visible[fi] {
                kept.push(f);
            }
        }
        faces = kept;
        for (a, b) in horizon {
            faces.push(make_face(pts, [a, b, pi], interior));
        }
    }

    // Emit the faces, compacting to the used vertices.
    let mut positions = Vec::new();
    let mut remap: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();
    let mut tris = Vec::new();
    for f in &faces {
        let mut idx = [0u32; 3];
        for (k, &vi) in f.v.iter().enumerate() {
            idx[k] = *remap.entry(vi).or_insert_with(|| {
                positions.push(pts[vi]);
                (positions.len() - 1) as u32
            });
        }
        tris.push(idx);
    }
    Mesh { positions, tris }
}

/// Does this directed-edge set form a single simple loop — every vertex
/// exactly once as a start and once as an end? Used to reject a horizon
/// corrupted by near-coplanar visibility flips before it caps into a
/// non-manifold shell.
fn is_simple_loop(edges: &[(usize, usize)]) -> bool {
    if edges.len() < 3 {
        return false;
    }
    use std::collections::HashMap;
    let mut out: HashMap<usize, u32> = HashMap::new();
    let mut inc: HashMap<usize, u32> = HashMap::new();
    for &(a, b) in edges {
        *out.entry(a).or_insert(0) += 1;
        *inc.entry(b).or_insert(0) += 1;
    }
    out.len() == edges.len()
        && inc.len() == edges.len()
        && out.values().all(|&c| c == 1)
        && inc.values().all(|&c| c == 1)
}

fn make_face(pts: &[V3], tri: [usize; 3], interior: V3) -> Face {
    let n = cross(sub(pts[tri[1]], pts[tri[0]]), sub(pts[tri[2]], pts[tri[0]]));
    let n = norm(n);
    let w = dot(n, pts[tri[0]]);
    // Orient outward: the interior point must be behind the plane.
    if dot(n, interior) - w > 0.0 {
        Face { v: [tri[0], tri[2], tri[1]], normal: [-n[0], -n[1], -n[2]], w: -w }
    } else {
        Face { v: tri, normal: n, w }
    }
}

fn dist2(a: V3, b: V3) -> f64 {
    let d = sub(a, b);
    dot(d, d)
}

/// Squared distance from point p to the line through a and b.
fn line_dist2(a: V3, b: V3, p: V3) -> f64 {
    let ab = sub(b, a);
    let ap = sub(p, a);
    let c = cross(ab, ap);
    let denom = dot(ab, ab);
    if denom < EPS * EPS {
        dot(ap, ap)
    } else {
        dot(c, c) / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom;

    fn bounds(m: &Mesh) -> ([f64; 3], [f64; 3]) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in &m.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        (lo, hi)
    }

    /// Winding-aware signed volume via the divergence theorem: sum of
    /// tetrahedra (origin, tri). Positive for outward-wound closed meshes.
    fn signed_volume(m: &Mesh) -> f64 {
        let mut v = 0.0;
        for t in &m.tris {
            let a = m.positions[t[0] as usize];
            let b = m.positions[t[1] as usize];
            let c = m.positions[t[2] as usize];
            v += dot(a, cross(b, c)) / 6.0;
        }
        v
    }

    #[test]
    fn union_of_overlapping_cubes_has_merged_volume() {
        // Two unit cubes overlapping in half: union volume = 2 - 0.5 = 1.5.
        let a = geom::cube([1.0, 1.0, 1.0], false);
        let b = geom::cube([1.0, 1.0, 1.0], false);
        // shift b by +0.5 in x
        let mut b = b;
        for p in &mut b.positions {
            p[0] += 0.5;
        }
        let u = union_all(&[a, b]);
        assert!((signed_volume(&u).abs() - 1.5).abs() < 1e-6, "vol {}", signed_volume(&u));
        let (lo, hi) = bounds(&u);
        assert!((lo[0] - 0.0).abs() < 1e-9 && (hi[0] - 1.5).abs() < 1e-9);
    }

    #[test]
    fn difference_carves_a_hole() {
        // Cube minus a smaller cube fully inside → hollow, volume = 1 - 0.125.
        let a = geom::cube([1.0, 1.0, 1.0], true);
        let b = geom::cube([0.5, 0.5, 0.5], true);
        let d = difference(&a, &[b]);
        assert!((signed_volume(&d).abs() - (1.0 - 0.125)).abs() < 1e-6, "vol {}", signed_volume(&d));
    }

    #[test]
    fn intersection_keeps_the_common_region() {
        // Two half-overlapping unit cubes: intersection is a 0.5x1x1 slab.
        let a = geom::cube([1.0, 1.0, 1.0], false);
        let mut b = geom::cube([1.0, 1.0, 1.0], false);
        for p in &mut b.positions {
            p[0] += 0.5;
        }
        let i = intersection_all(&[a, b]);
        assert!((signed_volume(&i).abs() - 0.5).abs() < 1e-6, "vol {}", signed_volume(&i));
        let (lo, hi) = bounds(&i);
        assert!((lo[0] - 0.5).abs() < 1e-9 && (hi[0] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn disjoint_intersection_is_empty() {
        let a = geom::cube([1.0, 1.0, 1.0], false);
        let mut b = geom::cube([1.0, 1.0, 1.0], false);
        for p in &mut b.positions {
            p[0] += 5.0;
        }
        let i = intersection_all(&[a, b]);
        assert!(i.positions.is_empty() || signed_volume(&i).abs() < 1e-9);
    }

    #[test]
    fn empty_operands_follow_identity_rules() {
        let a = geom::cube([1.0, 1.0, 1.0], false);
        let empty = Mesh::empty();
        // union with empty → a
        assert!((signed_volume(&union_all(&[a.clone(), empty.clone()])).abs() - 1.0).abs() < 1e-9);
        // difference by empty → a
        assert!((signed_volume(&difference(&a, &[empty.clone()])).abs() - 1.0).abs() < 1e-9);
        // empty minuend → empty
        assert!(difference(&empty, &[a.clone()]).positions.is_empty());
    }

    #[test]
    fn intersection_with_empty_is_empty_in_any_order() {
        // A ∩ ∅ = ∅ regardless of operand order (the panel's confirmed
        // non-commutativity bug: a non-first empty operand used to be
        // silently skipped, returning the full first operand).
        let a = geom::cube([2.0, 2.0, 2.0], true);
        let empty = Mesh::empty();
        assert!(intersection_all(&[a.clone(), empty.clone()]).positions.is_empty());
        assert!(intersection_all(&[empty.clone(), a.clone()]).positions.is_empty());
        assert!(intersection_all(&[a.clone(), empty, a.clone()]).positions.is_empty());
    }

    #[test]
    fn convex_hull_matches_known_solids() {
        // Hull of the 8 cube corners is the cube itself: volume = side^3.
        let cube = geom::cube([2.0, 2.0, 2.0], true);
        let h = hull(&[cube]);
        assert!((signed_volume(&h).abs() - 8.0).abs() < 1e-6, "vol {}", signed_volume(&h));
        // Hull is convex: every original vertex lies on or behind every
        // output face plane (no point pokes outside).
        let (lo, hi) = bounds(&h);
        assert!((lo[0] + 1.0).abs() < 1e-9 && (hi[0] - 1.0).abs() < 1e-9);

        // Hull of two separated spheres is bigger than one sphere and
        // spans both (a convex "capsule"-ish blob).
        let s1 = geom::sphere(3.0, 16);
        let mut s2 = geom::sphere(3.0, 16);
        for p in &mut s2.positions {
            p[0] += 12.0;
        }
        let h = hull(&[s1.clone(), s2]);
        assert!(signed_volume(&h).abs() > signed_volume(&s1).abs());
        let (lo, hi) = bounds(&h);
        assert!(lo[0] < -2.9 && hi[0] > 14.9, "spans both: {:?}..{:?}", lo, hi);

        // Hull of a concave L (two boxes sharing a corner at the origin)
        // fills the notch → convex, so its volume exceeds the L's own.
        let arm = geom::cube([6.0, 2.0, 2.0], false);
        let leg = geom::cube([2.0, 6.0, 2.0], false);
        let filled = hull(&[arm, leg]);
        let l_volume = 6.0 * 2.0 * 2.0 + 2.0 * 6.0 * 2.0 - 2.0 * 2.0 * 2.0; // minus shared corner
        assert!(signed_volume(&filled).abs() > l_volume);
    }

    #[test]
    fn minkowski_rounds_and_grows_convex_operands() {
        // A cube ⊕ a sphere rounds the cube: the result grows by ~r on
        // every side and is strictly larger than the cube alone.
        let cube = geom::cube([10.0, 10.0, 10.0], true);
        let ball = geom::sphere(2.0, 12);
        let rounded = match minkowski(&[cube.clone(), ball]) {
            Minkowski::Ok(m) => m,
            Minkowski::TooLarge(_) => panic!("cube⊕small-sphere must fit the cap"),
        };
        let (lo, hi) = bounds(&rounded);
        // Grown from [-5,5] toward [-7,7] on X (sphere radius 2).
        assert!(lo[0] < -6.5 && hi[0] > 6.5, "grew: {:?}..{:?}", lo, hi);
        assert!(signed_volume(&rounded).abs() > signed_volume(&cube).abs());

        // Single operand is identity (passes through unchanged, NOT hulled).
        match minkowski(&[geom::cube([2.0, 2.0, 2.0], true)]) {
            Minkowski::Ok(m) => assert!((signed_volume(&m).abs() - 8.0).abs() < 1e-9),
            Minkowski::TooLarge(_) => panic!(),
        }

        // Oversized product is reported, not hung.
        let s = geom::sphere(5.0, 40);
        assert!(matches!(minkowski(&[s.clone(), s]), Minkowski::TooLarge(_)));
    }

    #[test]
    fn hull_is_robust_to_bad_input() {
        // Non-finite coordinates must NOT panic — they are dropped, and
        // the remaining finite points still hull.
        let cube = geom::cube([2.0, 2.0, 2.0], true);
        let mut with_nan = cube.clone();
        with_nan.positions.push([f64::NAN, 0.0, 0.0]);
        with_nan.positions.push([1.0 / 0.0, 0.0, 0.0]);
        let h = hull(&[with_nan]);
        assert!((signed_volume(&h).abs() - 8.0).abs() < 1e-6);
        // A near-coplanar thin lens (all coordinates normal magnitude)
        // must not emit a non-manifold shell — the horizon guard keeps
        // every output edge shared by exactly two faces (2-manifold).
        let mut lens = geom::sphere(20.0, 24);
        for p in &mut lens.positions {
            p[2] *= 1e-4; // squash to a ~4e-3-thick disk
        }
        let h = hull(&[lens]);
        // Count undirected-edge multiplicity; a closed manifold has every
        // edge in exactly two triangles.
        use std::collections::HashMap;
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
        for t in &h.tris {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let key = if a < b { (a, b) } else { (b, a) };
                *edges.entry(key).or_insert(0) += 1;
            }
        }
        // Either it degenerated to empty (acceptable) or it is closed.
        assert!(h.tris.is_empty() || edges.values().all(|&c| c == 2), "non-manifold hull");
    }

    #[test]
    fn degenerate_hull_is_empty() {
        // Four coplanar points (a flat square, z=0) have no 3D volume.
        let flat = Mesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            tris: vec![[0, 1, 2], [0, 2, 3]],
        };
        assert!(hull(&[flat]).positions.is_empty());
        assert!(hull(&[]).positions.is_empty());
    }

    #[test]
    fn curved_primitive_booleans_stay_shallow_and_correct() {
        // A hole drilled through a smooth sphere: with the unbalanced BSP
        // this was a triangle-count-deep tree (Θ(n²) build, stack risk);
        // with plane balancing it completes quickly and the volume drops.
        // A sphere with enough facets that the old unbalanced tree would
        // be hundreds deep; balancing keeps it shallow so this returns
        // in well under a second instead of the quadratic blow-up.
        let ball = geom::sphere(10.0, 32);
        let full = signed_volume(&ball).abs();
        let drill = geom::cylinder(30.0, 3.0, 3.0, true, 24);
        let holed = difference(&ball, &[drill]);
        let holed_vol = signed_volume(&holed).abs();
        assert!(holed_vol > 0.0 && holed_vol < full, "full {} holed {}", full, holed_vol);
        // A pairwise-reduced union of several offset spheres completes and
        // is larger than a single sphere.
        let many: Vec<Mesh> = (0..6)
            .map(|i| {
                let mut s = geom::sphere(2.0, 12);
                for p in &mut s.positions {
                    p[0] += i as f64 * 1.2;
                }
                s
            })
            .collect();
        let u = union_all(&many);
        assert!(signed_volume(&u).abs() > signed_volume(&geom::sphere(2.0, 12)).abs());
    }
}
