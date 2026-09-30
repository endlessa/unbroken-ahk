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
    /// The plane through three points, or `None` if they are collinear.
    ///
    /// The degeneracy test is RELATIVE, and that matters more than it looks.
    /// It used to be `dot(n, n) < EPS * EPS` on the UNNORMALIZED cross
    /// product — an absolute area floor of 5e-8 square units in a kernel with
    /// no intrinsic unit. Every triangle smaller than that was silently
    /// dropped before any boolean ran, so the same model built in millimetres
    /// and in metres gave different answers: a 0.1 mm cube intersected with
    /// itself came back EMPTY, a difference returned the minuend with the
    /// cutter ignored entirely, and a union returned one operand. It also
    /// fired at ordinary scale on dense input — 8 of 22,496 facets of
    /// `sphere(r=1, $fn=150)` were below the floor.
    ///
    /// `|u x v| / (|u| |v|)` is the sine of the angle between the edges, so
    /// this rejects SLIVERS (which genuinely have no reliable normal) and
    /// keeps small-but-well-shaped triangles at any scale.
    fn from_points(a: V3, b: V3, c: V3) -> Option<Plane> {
        let (u, v) = (sub(b, a), sub(c, a));
        let n = cross(u, v);
        let denom = (dot(u, u) * dot(v, v)).sqrt();
        if !(denom > 0.0) || dot(n, n).sqrt() / denom < SLIVER_SINE {
            return None; // collinear, or a sliver with no usable normal
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

/// Deepest the BSP will recurse. A balanced tree over n facets is far
/// shallower; this only bounds pathological input (see `build_at`).
const MAX_BSP_DEPTH: usize = 4096;

thread_local! {
    /// Set when a BSP build hit the no-progress guard or the depth cap. Those
    /// guards exist so pathological coordinates cannot abort the process, but
    /// the price is a truncated tree — and a truncated tree yields a mesh that
    /// is merely approximate, and at large magnitudes genuinely open. Silently
    /// handing back an open mesh is its own defect, so the evaluator reads
    /// this and says so.
    static DEGRADED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn mark_degraded() {
    DEGRADED.with(|d| d.set(true));
}

/// The 2D segment-BSP shares the flag: from the user's side it is one
/// "the kernel had to approximate" condition.
pub(crate) fn mark_degraded_2d() {
    mark_degraded();
}

/// Is this mesh convex — every vertex on or behind every face plane?
///
/// Used only to decide whether minkowski's approximation warning is
/// warranted. The test is O(V*F), so beyond a budget it answers "not provably
/// convex", which errs toward warning rather than toward silence.
pub fn is_convex(m: &Mesh) -> bool {
    let v = m.positions.len();
    let f = m.tris.len();
    if v == 0 || f == 0 || v.saturating_mul(f) > 5_000_000 {
        return false;
    }
    // Tolerance relative to the model's own extent, so this does not repeat
    // the absolute-threshold mistake `Plane::from_points` just shed.
    let mut extent: f64 = 0.0;
    for p in &m.positions {
        for c in p {
            extent = extent.max(c.abs());
        }
    }
    let tol = extent.max(1.0) * 1e-9;
    for t in &m.tris {
        let (a, b, c) = (
            m.positions[t[0] as usize],
            m.positions[t[1] as usize],
            m.positions[t[2] as usize],
        );
        let Some(plane) = Plane::from_points(a, b, c) else { continue };
        for p in &m.positions {
            if dot(plane.normal, *p) - plane.w > tol {
                return false;
            }
        }
    }
    true
}

/// Read and clear the degradation flag for this thread.
pub fn take_degraded() -> bool {
    DEGRADED.with(|d| d.replace(false))
}

/// A triangle is degenerate when the sine of the angle between two of its
/// edges falls below this — i.e. it is a sliver, at any scale. Compare the
/// absolute-area test this replaced, which made the kernel scale-dependent.
const SLIVER_SINE: f64 = 1e-12;

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
        // Only a build that starts a FRESH tree can degrade the result. The
        // merge stage of a boolean re-builds into a node that already has a
        // plane, and its tree is only ever read back through `all_polygons`,
        // which collects facets regardless of tree shape — so the guard
        // firing there changes nothing. Tracking indiscriminately made the
        // degradation warning appear on six of nine perfectly ordinary demo
        // scenes, which is worse than not warning at all.
        let track = self.plane.is_none();
        self.build_at(polygons, 0, track);
    }

    /// Build the tree, refusing to recurse forever.
    ///
    /// `split_polygon`'s epsilon is absolute, so at large coordinate
    /// magnitudes a facet can be classified as neither coplanar with the
    /// chosen plane nor cleanly on one side of it. The child then receives
    /// the SAME set its parent had and recurses until the stack dies:
    /// `difference() { cube(1e9, center=true); sphere(r=0.6e9, $fn=12); }`
    /// aborted the whole process with SIGABRT — and a model measured in
    /// microns reaches 1e9 without trying.
    ///
    /// The no-progress check is the precise guard (a partition that moved
    /// nothing never will); the depth cap is the backstop for any other route
    /// to the same place. Degrading a boolean beats killing the process.
    fn build_at(&mut self, polygons: Vec<Polygon>, depth: usize, track: bool) {
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
        let stuck = front.len() == polygons.len() || back.len() == polygons.len();
        if depth >= MAX_BSP_DEPTH || stuck {
            if track {
                mark_degraded();
            }
            self.polygons.extend(front);
            self.polygons.extend(back);
            return;
        }
        if !front.is_empty() {
            self.front
                .get_or_insert_with(|| Box::new(Node::new()))
                .build_at(front, depth + 1, track);
        }
        if !back.is_empty() {
            self.back
                .get_or_insert_with(|| Box::new(Node::new()))
                .build_at(back, depth + 1, track);
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

/// How far a merged volume may fall outside the bounds before the merge is
/// judged to have failed. A boolean re-tessellates everything it touches, so
/// the volume is arrived at by a different sum of different triangles and
/// will not match to the last bit; a tenth of a percent is far above that
/// error and far below any real loss, which runs to halves and tenths.
const UNION_SLACK: f64 = 1e-3;

/// One solid in a union that is still being built.
///
/// The pieces of a `Sum` share no volume with each other. That invariant is
/// the whole design, and it is established by proof for every piece that
/// joins the set, never assumed; it pays for itself twice over.
///
/// It makes the BOUNDS a merge is checked against the bounds of the two
/// solids being merged and nothing else. A union contains each operand, so
/// it cannot enclose less than the larger; it is covered by them together,
/// so it cannot enclose more than their sum. Both hold for any union, need
/// no oracle and cost one pass over the triangles each, and between them
/// they catch a boolean that has gone wrong in either direction -- a heart
/// of eleven shells lost a third of itself, 168,222 mm^3 down to 135,556,
/// when the aorta was merged in. What they must NOT be is bounds on the
/// whole accumulated subtree: a fraction of that is a tolerance unrelated
/// geometry can inflate without limit, which is how a 1.0 mm overlap between
/// two 40 mm boxes came to vanish in silence when a 300 mm cube 500 mm away
/// joined the same union -- and vanish for one ordering of the children and
/// not another, which the language reference forbids outright: "Order of
/// children never affects the result".
///
/// And it means geometry that never meets never goes near a BSP. That is not
/// a micro-optimisation: a DNA segment of 44 solids that never touch spends
/// 2.1 seconds being evaluated and, before this, 139 seconds in the
/// export-time union, which then discovered it had joined nothing; a
/// suspension bridge's 501 members came out of the BSP as 695,916 triangles
/// carrying 52,617 T-junctions, to say what concatenation says exactly.
struct Piece {
    mesh: Mesh,
    lo: [f64; 3],
    hi: [f64; 3],
    /// A vertex, for the containment test that rules out nesting.
    on: V3,
    /// Volume this piece is known to enclose, at least and at most. The two
    /// are equal for a solid, and apart only for a piece that is the
    /// concatenation of a refused pair, which encloses less than their sum.
    vlo: f64,
    vhi: f64,
    /// Do no two solids in here overlap?
    ///
    /// True for one solid and for any number of solids already proved
    /// pairwise disjoint; false only for the concatenation of a pair whose
    /// merge was refused. A ray cast counts crossings, so it answers "inside"
    /// only under exactly that condition: a point inside two STACKED solids
    /// is crossed an even number of times and reads as outside. A piece that
    /// is marked unclean declines to speak for its own interior.
    clean: bool,
}

impl Piece {
    /// One solid, whether it came in as an operand or out of a boolean.
    fn of(mesh: Mesh) -> Option<Piece> {
        let (lo, hi) = crate::geom::bounds(&mesh)?;
        let on = *mesh.positions.first()?;
        // An inside-out operand still encloses |v|; it is only the sign that
        // is wrong. Taking the magnitude for the ceiling keeps the upper
        // bound honest, while the floor stays at zero because a solid wound
        // the wrong way cannot be claimed to contain anything.
        let v = mesh.signed_volume();
        Some(Piece { mesh, lo, hi, on, vlo: v.max(0.0), vhi: v.abs(), clean: true })
    }

    /// Several solids already proved to share no volume, carried as one.
    ///
    /// This is what makes a hub cheap. A girder that 400 deck planks all rest
    /// on is 400 pairs in the touch graph and not one pair anywhere else, so
    /// merging the planks in one at a time re-tessellates the whole growing
    /// girder 400 times -- measured, and it is the shape of the quadratic:
    /// 32 triangles into 18,746, then into 24,377, then 30,140, then 37,069.
    /// The planks share no volume with each other, so their union IS their
    /// concatenation, and the BSP can take all 400 of them against the girder
    /// in a single boolean.
    fn group(mut parts: Vec<Piece>) -> Piece {
        let mut acc = parts.pop().expect("a group has at least one piece");
        for p in parts {
            for k in 0..3 {
                acc.lo[k] = acc.lo[k].min(p.lo[k]);
                acc.hi[k] = acc.hi[k].max(p.hi[k]);
            }
            append(&mut acc.mesh, &p.mesh);
            acc.vlo += p.vlo;
            acc.vhi += p.vhi;
            acc.clean &= p.clean;
        }
        acc
    }

    /// Two solids that would not merge, kept side by side.
    ///
    /// Concatenation is not a union -- the result self-intersects where the
    /// operands did -- but it is every triangle that went in, which beats a
    /// merge that dropped some of them, and the caller is told.
    fn glued(a: Piece, b: Piece) -> Piece {
        let mut lo = a.lo;
        let mut hi = a.hi;
        for k in 0..3 {
            lo[k] = lo[k].min(b.lo[k]);
            hi[k] = hi[k].max(b.hi[k]);
        }
        Piece {
            mesh: concat(&a.mesh, &b.mesh),
            lo,
            hi,
            on: a.on,
            vlo: a.vlo.max(b.vlo),
            vhi: a.vhi + b.vhi,
            clean: false,
        }
    }
}

/// Is `inner` inside `outer`, as boxes?
fn box_contains(outer: &Piece, inner: &Piece) -> bool {
    (0..3).all(|k| outer.lo[k] <= inner.lo[k] && inner.hi[k] <= outer.hi[k])
}

/// Do two pieces' boxes overlap at all?
///
/// Two solids whose boxes do not are already each other's union: nothing to
/// clip, nothing to split. Taking the BSP anyway is not just slow, it is
/// destructive -- every polygon that straddles any plane of the other's tree
/// gets cut, so a union of twenty disjoint spheres came back with seven times
/// the triangles it went in with. The test is exact, so this is a shortcut,
/// not an approximation; it is only far too weak on its own, which is what
/// `pieces_disjoint` is for.
fn boxes_meet(a: &Piece, b: &Piece) -> bool {
    (0..3).all(|k| a.lo[k] <= b.hi[k] && b.lo[k] <= a.hi[k])
}

/// Does this point lie inside the closed solid, by crossing parity?
///
/// None when the ray could not be trusted: it struck an edge, a vertex, or
/// the surface itself, where the count is one either way. Four directions are
/// tried before giving up, and giving up means the caller keeps its boolean.
fn point_inside(m: &Mesh, p: V3) -> Option<bool> {
    // Directions with no small integer relationship between components, so a
    // ray is unlikely to run along an axis-aligned face or an edge of a
    // regular sweep.
    const DIRS: [V3; 4] = [
        [0.577_35, 0.577_36, 0.577_37],
        [0.801_78, -0.267_26, 0.534_52],
        [-0.408_25, 0.816_50, 0.408_26],
        [0.267_25, 0.534_51, -0.801_79],
    ];
    for d in DIRS {
        if let Some(n) = crossings(m, p, d) {
            return Some(n % 2 == 1);
        }
    }
    None
}

fn crossings(m: &Mesh, o: V3, d: V3) -> Option<usize> {
    // Barycentric coordinates are dimensionless, so this tolerance needs no
    // scaling; the distance along the ray does, and is measured against the
    // triangle's own size.
    const BARY: f64 = 1e-9;
    let mut n = 0usize;
    for t in &m.tris {
        let a = m.positions[t[0] as usize];
        let e1 = sub(m.positions[t[1] as usize], a);
        let e2 = sub(m.positions[t[2] as usize], a);
        let pv = cross(d, e2);
        let det = dot(e1, pv);
        let scale = (dot(e1, e1) * dot(e2, e2)).sqrt();
        if scale <= 0.0 {
            continue; // a degenerate triangle bounds nothing
        }
        if det.abs() <= scale * BARY {
            // The ray lies in the triangle's plane. It contributes no
            // crossing, but if it is ALSO near the triangle, the parity is
            // not to be trusted.
            continue;
        }
        let inv = 1.0 / det;
        let tv = sub(o, a);
        let u = dot(tv, pv) * inv;
        let qv = cross(tv, e1);
        let v = dot(d, qv) * inv;
        let w = 1.0 - u - v;
        if u < -BARY || v < -BARY || w < -BARY {
            continue; // misses
        }
        if u < BARY || v < BARY || w < BARY {
            return None; // grazes an edge or a corner
        }
        let along = dot(e2, qv) * inv;
        if along.abs() <= scale.sqrt() * BARY {
            return None; // the point is ON the surface
        }
        if along > 0.0 {
            n += 1;
        }
    }
    Some(n)
}

/// Can these two be shown to share no volume, without running a boolean?
///
/// Answers in the SUFFICIENT direction only: "certainly not" or "cannot
/// say", and "cannot say" costs only the boolean that was going to run
/// anyway. Bounding boxes cannot do it -- a backbone helix and a box girder
/// both have boxes containing most of their model -- so the question goes to
/// the triangles.
///
/// Two closed solids share volume only if their surfaces cross, OR one is
/// wholly inside the other. Nesting is ruled out first and by boxes, since a
/// nested solid's box is inside its container's; where a box allows nesting
/// the point settles it. What remains is a surface-crossing test.
fn pieces_disjoint(a: &Piece, b: &Piece) -> bool {
    let nested = |outer: &Piece, inner: &Piece| {
        box_contains(outer, inner)
            && (!outer.clean || point_inside(&outer.mesh, inner.on) != Some(false))
    };
    if nested(a, b) || nested(b, a) {
        return false;
    }
    !surfaces_may_touch(&a.mesh, &b.mesh)
}

/// Might these two share volume? The question `absorb` actually asks.
fn may_share(a: &Piece, b: &Piece) -> bool {
    boxes_meet(a, b) && !pieces_disjoint(a, b)
}

/// Two triangles are separated when some axis separates their projections.
///
/// Triangles are convex, so the separating-axis theorem applies exactly, and
/// the axes that need testing are the two face normals and the nine cross
/// products of one edge with another. Finding one is a PROOF of separation;
/// finding none here is treated as "they may touch", which is the safe way
/// round -- a wrong "separated" leaves two overlapping solids concatenated
/// and self-intersecting, while a wrong "may touch" only costs the boolean
/// that would have run anyway.
fn tris_separated(p: &[V3; 3], q: &[V3; 3], eps: f64) -> bool {
    let span = |t: &[V3; 3], d: V3| {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for v in t {
            let x = dot(*v, d);
            lo = lo.min(x);
            hi = hi.max(x);
        }
        (lo, hi)
    };
    let pe = [sub(p[1], p[0]), sub(p[2], p[1]), sub(p[0], p[2])];
    let qe = [sub(q[1], q[0]), sub(q[2], q[1]), sub(q[0], q[2])];
    let mut axes: Vec<V3> = Vec::with_capacity(11);
    axes.push(cross(pe[0], pe[1]));
    axes.push(cross(qe[0], qe[1]));
    for u in &pe {
        for v in &qe {
            axes.push(cross(*u, *v));
        }
    }
    for d in axes {
        let len = dot(d, d).sqrt();
        // A near-zero axis carries no information: parallel edges, or a
        // degenerate triangle. Skipping it can only lose a proof, never
        // invent one.
        if len < 1e-300 {
            continue;
        }
        let (pl, ph) = span(p, d);
        let (ql, qh) = span(q, d);
        let gap = (ql - ph).max(pl - qh);
        if gap > eps * len {
            return true;
        }
    }
    false
}

/// Do any triangle of `a` and any triangle of `b` come close enough to touch?
///
/// Answers conservatively: false only when every pair was proved apart.
fn surfaces_may_touch(a: &Mesh, b: &Mesh) -> bool {
    use std::collections::HashMap;
    // The grid is built over the FIRST mesh and walked with the second, so
    // the smaller one goes first: a hanger against a girder should pay for
    // the hanger's triangles, not the girder's.
    let (a, b) = if a.tris.len() <= b.tris.len() { (a, b) } else { (b, a) };
    let (Some((alo, ahi)), Some((blo, bhi))) = (crate::geom::bounds(a), crate::geom::bounds(b))
    else {
        return true;
    };
    // Only the region the two boxes share can hold a meeting, so everything
    // outside it is skipped before any arithmetic.
    let mut lo = [0.0; 3];
    let mut hi = [0.0; 3];
    for k in 0..3 {
        lo[k] = alo[k].max(blo[k]);
        hi[k] = ahi[k].min(bhi[k]);
        if lo[k] > hi[k] {
            return false;
        }
    }
    // Tolerance is relative to the pair's own size, and small: solids that
    // touch EXACTLY must come out as "may touch", because concatenating two
    // solids that share a face leaves that face in the mesh twice. Let those
    // go to the boolean, which is where they are handled today.
    let extent = (0..3).fold(0.0f64, |m, k| m.max(ahi[k].max(bhi[k]) - alo[k].min(blo[k])));
    let eps = extent * 1e-9;

    let corners = |m: &Mesh, t: &[u32; 3]| {
        [m.positions[t[0] as usize], m.positions[t[1] as usize], m.positions[t[2] as usize]]
    };
    let tri_box = |v: &[V3; 3]| {
        let mut l = v[0];
        let mut h = v[0];
        for p in &v[1..] {
            for k in 0..3 {
                l[k] = l[k].min(p[k]);
                h[k] = h[k].max(p[k]);
            }
        }
        (l, h)
    };
    // Grid resolution from the triangle count, so cells hold a few triangles
    // each whatever the model's size.
    let n = ((a.tris.len().max(b.tris.len()) as f64).cbrt().ceil() as i64).clamp(4, 64);
    let step: Vec<f64> = (0..3).map(|k| ((hi[k] - lo[k]) / n as f64).max(1e-300)).collect();
    let cell = |v: f64, k: usize| (((v - lo[k]) / step[k]).floor() as i64).clamp(0, n - 1);

    // Insert a's triangles. A triangle spanning a large part of the grid is
    // cheap to insert once and expensive to insert everywhere, so the budget
    // gives up rather than thrashing; giving up means "may touch".
    let budget = 400 * (a.tris.len() + b.tris.len() + 16);
    let mut grid: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::new();
    let mut inserted = 0usize;
    for (i, t) in a.tris.iter().enumerate() {
        let v = corners(a, t);
        let (tl, th) = tri_box(&v);
        if (0..3).any(|k| th[k] < lo[k] - eps || tl[k] > hi[k] + eps) {
            continue; // outside the shared region entirely
        }
        let (c0, c1) = (
            [cell(tl[0], 0), cell(tl[1], 1), cell(tl[2], 2)],
            [cell(th[0], 0), cell(th[1], 1), cell(th[2], 2)],
        );
        for x in c0[0]..=c1[0] {
            for y in c0[1]..=c1[1] {
                for z in c0[2]..=c1[2] {
                    grid.entry((x, y, z)).or_default().push(i);
                    inserted += 1;
                    if inserted > budget {
                        return true;
                    }
                }
            }
        }
    }
    if grid.is_empty() {
        return false;
    }
    let mut tested = 0usize;
    let pair_budget = 4_000 * (a.tris.len() + b.tris.len() + 16);
    for t in &b.tris {
        let v = corners(b, t);
        let (tl, th) = tri_box(&v);
        if (0..3).any(|k| th[k] < lo[k] - eps || tl[k] > hi[k] + eps) {
            continue;
        }
        let (c0, c1) = (
            [cell(tl[0], 0), cell(tl[1], 1), cell(tl[2], 2)],
            [cell(th[0], 0), cell(th[1], 1), cell(th[2], 2)],
        );
        for x in c0[0]..=c1[0] {
            for y in c0[1]..=c1[1] {
                for z in c0[2]..=c1[2] {
                    let Some(list) = grid.get(&(x, y, z)) else { continue };
                    for &i in list {
                        let w = corners(a, &a.tris[i]);
                        tested += 1;
                        if tested > pair_budget {
                            return true;
                        }
                        if !tris_separated(&w, &v, eps) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// The most triangles ONE CONNECTED COMPONENT will be put through a BSP.
///
/// A BSP plane is infinite, so every polygon that straddles one is cut
/// whether or not the boolean touches it, and a merged mesh comes back ten
/// times the triangle count it went in with -- two spheres of 23,800
/// triangles came out at 220,531. Past some size that has to be refused,
/// because an export that never returns is worse than one a validator
/// complains about.
///
/// What it is measured AGAINST is the fix. It used to be the whole model's
/// triangle count, gated on whether any two bounding boxes overlapped --
/// and a box test on a model with one long part is always true, so a
/// suspension bridge of 501 solids that share no volume at all, and need no
/// boolean at all, was one triangle over the line from being refused
/// wholesale. Only a connected component of "these two might share volume"
/// ever reaches a BSP, so only a component is weighed, and a model of a
/// million disjoint triangles now merges without hesitating because there
/// is nothing to merge.
///
/// The refusal is still a refusal: the component is concatenated, which
/// self-intersects where its parts do, and the caller is told which and how
/// big. The old behaviour was worse than that -- it skipped SILENTLY past
/// the threshold in the sense that the clean-looking answer was the wrong
/// one: two overlapping spheres exported at 8,369 mm^3 against a true union
/// of 7,506, with holes 0 and two components, while the side of the
/// threshold that did the work reported 15,919 boundary edges.
pub const MAX_MERGE_TRIS: usize = 25_000;

thread_local! {
    /// Pairwise unions that had to fall back to concatenation, and unions
    /// that came out right only after swapping the operands. Read and
    /// cleared by `take_union_trouble`.
    static UNION_TROUBLE: std::cell::Cell<(usize, usize)> =
        const { std::cell::Cell::new((0, 0)) };
    /// The largest component the budget refused, if any.
    static UNION_BUDGET: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Triangles in the largest component the budget refused since the last
/// call, or 0 if none. Clears.
pub fn take_union_budget() -> usize {
    UNION_BUDGET.with(|c| c.replace(0))
}

/// (fell back to concatenation, rescued by swapping the operands) since the
/// last call, which this clears.
pub fn take_union_trouble() -> (usize, usize) {
    UNION_TROUBLE.with(|c| c.replace((0, 0)))
}

fn note_union(fallback: bool) {
    UNION_TROUBLE.with(|c| {
        let (f, s) = c.get();
        c.set(if fallback { (f + 1, s) } else { (f, s + 1) });
    });
}

/// Reduce a union to the fewest booleans that can express it.
///
/// Three observations, in the order they are used.
///
/// SOLIDS THAT SHARE NO VOLUME ARE ALREADY THEIR OWN UNION. Their
/// concatenation is exactly right, and putting them through a BSP is not just
/// slow but destructive -- every polygon straddling any plane of the other's
/// tree gets cut, so a union of twenty disjoint spheres came back with seven
/// times the triangles it went in with, and a suspension bridge's 501 members
/// came out as 695,916 triangles carrying 52,617 T-junctions to say what
/// concatenation says exactly. So the first thing built is the graph of which
/// pieces MIGHT share volume, and the answer comes from a separating axis
/// between their triangles, not from bounding boxes, which on a long part are
/// useless -- a girder's box holds every hanger, tower and anchorage in the
/// model.
///
/// WHAT DOES SHARE VOLUME IS A COMPONENT OF THAT GRAPH, AND COMPONENTS ARE
/// INDEPENDENT. Each is reduced on its own, and the result does not depend on
/// the order the children were written in -- which the reference requires
/// ("Order of children never affects the result") and the pairwise tree this
/// replaced could not manage, because it merged neighbours in written order.
///
/// WITHIN A COMPONENT, PIECES THAT DO NOT TOUCH EACH OTHER CAN STILL BE
/// CONCATENATED. A girder that four hundred deck planks rest on is four
/// hundred edges and no edge anywhere else; merging the planks in one at a
/// time re-tessellates the growing girder four hundred times. Colouring the
/// component so that no two adjacent pieces share a colour puts every plank
/// in one colour class and the girder in another, and the whole thing is ONE
/// boolean. Greedy colouring by descending degree needs at most one more
/// colour than the largest degree, and on a hub it needs two.
fn reduce(pieces: Vec<Piece>) -> Mesh {
    let n = pieces.len();
    // Which pairs might share volume? Boxes settle most of it outright; only
    // the pairs whose boxes meet pay for a separating axis.
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        for j in (i + 1)..n {
            if may_share(&pieces[i], &pieces[j]) {
                adj[i].push(j);
                adj[j].push(i);
            }
        }
    }

    let mut seen = vec![false; n];
    let mut pieces: Vec<Option<Piece>> = pieces.into_iter().map(Some).collect();
    let mut out = Mesh::empty();
    for root in 0..n {
        if seen[root] {
            continue;
        }
        // One component, by depth-first walk.
        let mut group = Vec::new();
        let mut stack = vec![root];
        seen[root] = true;
        while let Some(v) = stack.pop() {
            group.push(v);
            for &w in &adj[v] {
                if !seen[w] {
                    seen[w] = true;
                    stack.push(w);
                }
            }
        }
        let solid = if group.len() == 1 {
            pieces[group[0]].take().expect("each piece is taken once")
        } else {
            // A model that says its parts share no volume can be held to it,
            // and this is where the claim is actually decided. Naming the
            // members that met is worth far more than a count: on a
            // suspension bridge whose header claims 20 mm of air everywhere
            // two members meet, it is the difference between "the volume is
            // 0.08 m^3 under prediction" and "these two boxes overlap".
            if std::env::var_os("SCADFORGE_UNION_TRACE").is_some() {
                eprintln!("union component of {} solids:", group.len());
                for v in group.iter().copied() {
                    if let Some(q) = pieces[v].as_ref() {
                        eprintln!(
                            "   {} tris, box [{:.4} {:.4} {:.4}] .. [{:.4} {:.4} {:.4}], \
                             meets {}",
                            q.mesh.tris.len(),
                            q.lo[0], q.lo[1], q.lo[2],
                            q.hi[0], q.hi[1], q.hi[2],
                            adj[v].len()
                        );
                    }
                }
            }
            let sum: f64 = group.iter().filter_map(|&v| pieces[v].as_ref()).map(|q| q.vhi).sum();
            let made = merge_component(&group, &adj, &mut pieces);
            if std::env::var_os("SCADFORGE_UNION_TRACE").is_some() {
                eprintln!(
                    "   -> {:.6} where the parts sum to {:.6}; they share {:.6}",
                    made.vhi,
                    sum,
                    sum - made.vhi
                );
            }
            made
        };
        append(&mut out, &solid.mesh);
    }
    out
}

/// Colour a component so no two touching pieces share a colour, concatenate
/// each colour class, then merge the classes smallest first.
fn merge_component(
    group: &[usize],
    adj: &[Vec<usize>],
    pieces: &mut [Option<Piece>],
) -> Piece {
    // Descending degree: the classic greedy order, and the one that puts a
    // hub in a class of its own and everything hanging off it in one other.
    let mut order: Vec<usize> = group.to_vec();
    order.sort_by_key(|&v| std::cmp::Reverse(adj[v].len()));

    let mut colour: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut classes: Vec<Vec<usize>> = Vec::new();
    for v in order {
        let mut taken = vec![false; classes.len() + 1];
        for &w in &adj[v] {
            if let Some(&c) = colour.get(&w) {
                taken[c] = true;
            }
        }
        let c = taken.iter().position(|&t| !t).expect("one colour is always free");
        if c == classes.len() {
            classes.push(Vec::new());
        }
        classes[c].push(v);
        colour.insert(v, c);
    }

    let mut solids: Vec<Piece> = classes
        .into_iter()
        .map(|c| {
            Piece::group(c.into_iter().map(|v| pieces[v].take().expect("taken once")).collect())
        })
        .collect();
    // Smallest first, so the growing result meets the big operand last and
    // is re-tessellated by it once rather than at every step.
    solids.sort_by_key(|p| p.mesh.tris.len());
    let total: usize = solids.iter().map(|p| p.mesh.tris.len()).sum();
    if total > MAX_MERGE_TRIS {
        UNION_BUDGET.with(|c| c.set(c.get().max(total)));
        let mut it = solids.into_iter();
        let mut acc = it.next().expect("a component has at least one class");
        for p in it {
            acc = Piece::glued(acc, p);
        }
        return acc;
    }
    let mut it = solids.into_iter();
    let mut acc = it.next().expect("a component has at least one class");
    for p in it {
        acc = merge(acc, p);
    }
    // Only a component that actually went through a BSP can carry
    // T-junctions, so this is asked exactly where it can answer and never
    // of the concatenations, which are the bulk of a large model.
    acc.mesh = crate::geom::weld_tjunctions(&acc.mesh);
    acc
}

/// Union two solids that are known, or at least suspected, to meet.
fn merge(a: Piece, b: Piece) -> Piece {
    let (lo, hi) = (a.vlo.max(b.vlo), a.vhi + b.vhi);
    let holds = |m: &Mesh| {
        let v = m.signed_volume();
        v >= lo * (1.0 - UNION_SLACK) && v <= hi * (1.0 + UNION_SLACK)
    };

    let merged = boolean(&a.mesh, &b.mesh, Op::Union);
    let got_merged = merged.signed_volume();
    if holds(&merged) {
        if let Some(p) = Piece::of(merged) {
            return p;
        }
    }
    // A BSP is not symmetric in its operands: the first supplies the planes
    // the second is cut by, so `a u b` and `b u a` are two different
    // computations of the same answer, and one of them failing says nothing
    // about the other. Trying the swap costs a second boolean only on the
    // designs that needed it.
    let swapped = boolean(&b.mesh, &a.mesh, Op::Union);
    let got_swapped = swapped.signed_volume();
    if holds(&swapped) {
        if let Some(p) = Piece::of(swapped) {
            note_union(false);
            return p;
        }
    }
    // The warning can only say how many merges failed -- it has no name for
    // the operands and no units, since the reduction runs in the normalised
    // frame. This says which pair and by how much, which is what actually
    // locates the offending shell. Volumes are in the working frame, so read
    // them against each other, not as millimetres.
    if std::env::var_os("SCADFORGE_UNION_TRACE").is_some() {
        eprintln!(
            "union refused: a={} tris [{:.4}, {:.4}]  b={} tris [{:.4}, {:.4}]  \
             merged {:.4}, swapped {:.4}, needed [{:.4}, {:.4}]",
            a.mesh.tris.len(),
            a.vlo,
            a.vhi,
            b.mesh.tris.len(),
            b.vlo,
            b.vhi,
            got_merged,
            got_swapped,
            lo,
            hi
        );
        for (tag, p) in [("a", &a), ("b", &b)] {
            eprintln!(
                "   {tag} box [{:.3} {:.3} {:.3}] .. [{:.3} {:.3} {:.3}]",
                p.lo[0], p.lo[1], p.lo[2], p.hi[0], p.hi[1], p.hi[2]
            );
        }
    }
    note_union(true);
    Piece::glued(a, b)
}

/// The working frame a boolean is solved in.
///
/// Every plane carries `w = dot(normal, a)` and classification asks for
/// `dot(normal, p) - w`, so both terms grow with the coordinates while
/// their difference stays the small distance being tested. At coordinates
/// of 1e9 that difference has an ulp near the 1e-7 tolerance itself, and
/// past that the classification is noise: coincident faces stop reading as
/// coincident and a boolean of two ordinary cubes returns nonsense. At the
/// other end a shape of side 1e-7 is entirely inside the tolerance and the
/// result comes back EMPTY. Both are reachable from ordinary input — a
/// translate() far out, or a model authored in metres with millimetre
/// features.
///
/// Booleans commute with translation and uniform scaling, so the operands
/// are moved to the origin and scaled to unit extent, solved there, and put
/// back. The scale is a POWER OF TWO, so both scalings are exact in binary
/// floating point; the shift is applied only where the geometry is far
/// enough out that every coordinate shares an exponent with the centre,
/// which makes that subtraction exact too. Geometry already near unit scale
/// at the origin gets the identity and comes back bit-identical to before.
#[derive(Clone, Copy)]
struct Frame {
    c: V3,
    s: f64,
}

impl Frame {
    const ID: Frame = Frame { c: [0.0, 0.0, 0.0], s: 1.0 };

    fn of(meshes: &[Mesh]) -> Frame {
        // Two different questions, and they need two different answers.
        //
        // REACH — how far the whole scene sits from the origin — decides
        // whether to recentre, and the shift is exact only when every
        // coordinate shares an exponent with the centre, which is what the
        // "further out than a few of its own extents" test establishes.
        //
        // DETAIL — the SMALLEST extent any one operand has — decides the
        // scale, because that is the finest structure the tolerance must
        // still resolve. Scaling by the JOINT span instead was a bug of its
        // own making: a cutter 1e8 away stretched the span to 1e8, the unit
        // cube it was meant to miss was scaled to 1e-8, and it vanished
        // inside the plane tolerance. difference() { cube(1);
        // translate([1e8,0,0]) cube(1); } rendered NOTHING.
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        let mut detail = f64::INFINITY;
        for m in meshes {
            let mut mlo = [f64::INFINITY; 3];
            let mut mhi = [f64::NEG_INFINITY; 3];
            for p in &m.positions {
                if !p.iter().all(|v| v.is_finite()) {
                    continue;
                }
                for k in 0..3 {
                    mlo[k] = mlo[k].min(p[k]);
                    mhi[k] = mhi[k].max(p[k]);
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
            let mspan = (0..3).fold(0.0f64, |a, k| a.max(mhi[k] - mlo[k]));
            if mspan.is_finite() && mspan > 0.0 {
                detail = detail.min(mspan);
            }
        }
        let reach = (0..3).fold(0.0f64, |a, k| a.max(hi[k] - lo[k]));
        if !reach.is_finite() || reach <= 0.0 {
            return Frame::ID;
        }
        if !detail.is_finite() || detail <= 0.0 {
            detail = reach;
        }
        let e = detail.log2().round();
        let s = if e.is_finite() && e != 0.0 && e.abs() < 900.0 {
            2f64.powi(-(e as i32))
        } else {
            1.0
        };
        let mut c = [0.0; 3];
        for k in 0..3 {
            if lo[k].abs().max(hi[k].abs()) > reach * 4.0 {
                c[k] = (lo[k] + hi[k]) * 0.5;
            }
        }
        Frame { c, s }
    }

    fn is_identity(&self) -> bool {
        self.c == [0.0, 0.0, 0.0] && self.s == 1.0
    }

    fn fwd(&self, m: &Mesh) -> Mesh {
        let mut q = m.clone();
        for p in &mut q.positions {
            for k in 0..3 {
                p[k] = (p[k] - self.c[k]) * self.s;
            }
        }
        q
    }

    fn inv(&self, m: &Mesh) -> Mesh {
        let mut q = m.clone();
        for p in &mut q.positions {
            for k in 0..3 {
                p[k] = p[k] / self.s + self.c[k];
            }
        }
        q
    }
}

/// Solve `op` in the operands' own working frame.
fn framed<F: FnOnce(&[Mesh]) -> Mesh>(meshes: &[Mesh], op: F) -> Mesh {
    let f = Frame::of(meshes);
    if f.is_identity() {
        return op(meshes);
    }
    let scaled: Vec<Mesh> = meshes.iter().map(|m| f.fwd(m)).collect();
    f.inv(&op(&scaled))
}

/// n-ary union of a list of meshes (empty meshes are skipped).
pub fn union_all(meshes: &[Mesh]) -> Mesh {
    // One operand is its own union. Going through the working frame anyway
    // scales the coordinates out and back, and a scale that is not a power of
    // two is not an exact round trip -- a lone `cube(4)` came back with a
    // volume of 63.99999999999999. Nothing to solve, so nothing to move.
    let mut solid = meshes.iter().filter(|m| !m.positions.is_empty());
    let (Some(first), None) = (solid.next(), solid.next()) else {
        return framed(meshes, union_all_raw).welded();
    };
    first.welded()
}

fn union_all_raw(meshes: &[Mesh]) -> Mesh {
    reduce(
        meshes
            .iter()
            .filter(|m| !m.positions.is_empty())
            .filter_map(|m| Piece::of(m.clone()))
            .collect(),
    )
}

/// Concatenate two meshes, rebasing the second's indices.
fn concat(a: &Mesh, b: &Mesh) -> Mesh {
    let mut out = a.clone();
    append(&mut out, b);
    out
}

/// Append in place, rebasing the second's indices.
///
/// The same thing `concat` does without copying the left operand, which
/// matters wherever a concatenation is built up in a loop: `concat` there is
/// quadratic in the finished mesh, and the loops that do it are the ones
/// over a union's components and over a colour class, which is exactly where
/// the pieces are most numerous. A geodesic of 782 components was copying
/// about twelve million triangles to assemble thirty thousand.
fn append(out: &mut Mesh, b: &Mesh) {
    let base = out.positions.len() as u32;
    out.positions.extend_from_slice(&b.positions);
    out.tris.extend(b.tris.iter().map(|t| [t[0] + base, t[1] + base, t[2] + base]));
}

/// difference: the first mesh minus the union of the rest.
pub fn difference(first: &Mesh, rest: &[Mesh]) -> Mesh {
    if first.positions.is_empty() {
        return Mesh::empty(); // empty minuend → empty, always
    }
    let mut all = Vec::with_capacity(1 + rest.len());
    all.push(first.clone());
    all.extend_from_slice(rest);
    crate::geom::weld_tjunctions(&framed(&all, |m| difference_raw(&m[0], &m[1..])))
}

fn difference_raw(first: &Mesh, rest: &[Mesh]) -> Mesh {
    if first.positions.is_empty() {
        return Mesh::empty();
    }
    let cutters = union_all_raw(rest);
    if cutters.positions.is_empty() {
        return first.clone();
    }
    boolean(first, &cutters, Op::Difference)
}

/// n-ary intersection: the region common to every mesh. An empty operand
/// annihilates the result (A ∩ ∅ = ∅), so intersection is commutative.
pub fn intersection_all(meshes: &[Mesh]) -> Mesh {
    crate::geom::weld_tjunctions(&framed(meshes, intersection_all_raw))
}

fn intersection_all_raw(meshes: &[Mesh]) -> Mesh {
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
    /// A step's V1·V2 exceeded the cap: carries the offending point count
    /// and the last successfully-folded accumulator (so the caller can
    /// still show the completed prefix, not just the raw first operand).
    TooLarge { count: usize, partial: Mesh },
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
            return Minkowski::TooLarge { count: product, partial: acc };
        }
        let mut pts = Vec::with_capacity(product);
        for &pa in &acc.positions {
            for &pb in &m.positions {
                pts.push([pa[0] + pb[0], pa[1] + pb[1], pa[2] + pb[2]]);
            }
        }
        // Through hull(), not convex_hull() directly: hull() is wrapped in
        // the frame precisely because the incremental hull rejects a
        // candidate point with an ABSOLUTE distance test. Minkowski skipped
        // the wrapper and kept the scale dependence — a 1e-6 operand came
        // back 18% short, a 1e-7 one nearly 80% short.
        acc = hull(&[Mesh { positions: pts, tris: Vec::new() }]);
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
    // The incremental hull rejects a candidate point with an absolute
    // distance test, so a solid of side 1e-6 came back 9% short and one of
    // 1e-9 came back EMPTY. Solved in the same normalised frame as the
    // booleans.
    framed(meshes, hull_raw)
}

fn hull_raw(meshes: &[Mesh]) -> Mesh {
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

    // A BTreeSet, not a HashSet: the horizon is read back to build the cap
    // faces, and `HashSet` iteration order is seeded randomly per process —
    // reading it made hull() emit a DIFFERENT mesh on every run of the same
    // file. Any container whose order reaches the output has to be ordered.
    let mut vis_edges: std::collections::BTreeSet<(usize, usize)> =
        std::collections::BTreeSet::new();
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
        // Walk the faces in their own order (not the set's) so the cap
        // faces are appended deterministically; the set is only consulted
        // for the reverse-edge test and to keep each edge once.
        let mut horizon: Vec<(usize, usize)> = Vec::new();
        let mut on_horizon: std::collections::BTreeSet<(usize, usize)> =
            std::collections::BTreeSet::new();
        for (fi, f) in faces.iter().enumerate() {
            if !visible[fi] {
                continue;
            }
            for e in [(f.v[0], f.v[1]), (f.v[1], f.v[2]), (f.v[2], f.v[0])] {
                if vis_edges.contains(&(e.1, e.0)) {
                    continue; // interior to the visible cap, not its boundary
                }
                if on_horizon.insert(e) {
                    horizon.push(e);
                }
            }
        }
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

/// Does this directed-edge set form ONE simple loop — every vertex once
/// as a start and once as an end, AND all edges on a single cycle? Used
/// to reject a horizon corrupted by near-coplanar visibility flips before
/// it caps into a non-manifold shell. A degree check alone would also
/// accept a disjoint union of cycles, so this additionally walks the
/// successor map and requires the walk to cover every edge.
fn is_simple_loop(edges: &[(usize, usize)]) -> bool {
    if edges.len() < 3 {
        return false;
    }
    use std::collections::HashMap;
    let mut succ: HashMap<usize, usize> = HashMap::new();
    let mut inc: HashMap<usize, u32> = HashMap::new();
    for &(a, b) in edges {
        // A repeated start vertex means out-degree > 1 → not simple.
        if succ.insert(a, b).is_some() {
            return false;
        }
        *inc.entry(b).or_insert(0) += 1;
    }
    if inc.len() != edges.len() || inc.values().any(|&c| c != 1) {
        return false; // some vertex has in-degree != 1
    }
    // Walk from an arbitrary start; ONE cycle first returns to the start
    // after exactly edges.len() hops. Returning early means a shorter
    // sub-cycle exists (a disjoint union), so reject it.
    let start = edges[0].0;
    let mut cur = start;
    for step in 0..edges.len() {
        cur = match succ.get(&cur) {
            Some(&n) => n,
            None => return false,
        };
        if cur == start && step + 1 < edges.len() {
            return false;
        }
    }
    cur == start
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

    /// A closed 2-manifold has every undirected edge shared by exactly
    /// two triangles.
    fn is_closed_manifold(m: &Mesh) -> bool {
        use std::collections::HashMap;
        if m.tris.is_empty() {
            return false;
        }
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
        for t in &m.tris {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let key = if a < b { (a, b) } else { (b, a) };
                *edges.entry(key).or_insert(0) += 1;
            }
        }
        edges.values().all(|&c| c == 2)
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
    fn boolean_results_share_their_vertices() {
        // The BSP carries every polygon's corners independently, so a boolean
        // handed back a mesh in which NO two triangles shared a vertex: a
        // union of two DISJOINT cubes — nothing cut at all — came back with
        // 72 vertices for 24 triangles, every one of its 72 half-edges
        // unmatched. STL does not care, being a soup format, but OFF, AMF and
        // 3MF write explicit indices, so a `difference()` exported 3065 vertex
        // records for 982 distinct positions.
        //
        // Found by differential testing. The geometry was never wrong — 640k
        // ray-parity checks against an independent point-in-mesh oracle agree
        // everywhere, including on the hard case below — it was the topology
        // that was thrown away.
        let a = geom::cube([1.0, 1.0, 1.0], false);
        let mut b = geom::cube([1.0, 1.0, 1.0], false);
        for p in &mut b.positions {
            p[0] += 5.0;
        }
        // Disjoint: the answer IS the two inputs, so it must be exactly as
        // welded as they are — a closed 2-manifold.
        let u = union_all(&[a.clone(), b.clone()]);
        assert_eq!(u.positions.len(), 16, "disjoint union must not explode its vertices");
        assert_eq!(u.tris.len(), 24);
        assert!(is_closed_manifold(&u), "a union that cuts nothing must stay manifold");
        assert!((signed_volume(&u) - 2.0).abs() < 1e-9);

        // Where it really does cut, welding cannot make the result manifold —
        // the splitter leaves T-junctions, which is a separate matter — but no
        // position may appear twice.
        for m in [
            union_all(&[a.clone(), {
                let mut c = a.clone();
                for p in &mut c.positions {
                    p[0] += 0.5;
                }
                c
            }]),
            difference(&geom::cube([10.0, 10.0, 10.0], true), &[geom::sphere(6.0, 24)]),
            intersection_all(&[geom::cube([2.0, 2.0, 2.0], true), geom::sphere(1.3, 12)]),
        ] {
            let mut seen = std::collections::HashSet::new();
            for p in &m.positions {
                let k = [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()];
                assert!(seen.insert(k), "position {p:?} appears twice in a boolean result");
            }
            // ...and every triangle still has three distinct corners.
            for t in &m.tris {
                assert!(t[0] != t[1] && t[1] != t[2] && t[0] != t[2], "degenerate triangle {t:?}");
            }
            assert!(!m.tris.is_empty());
        }
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

    /// The bounds a union has to satisfy, and the report when it cannot.
    ///
    /// Both are checkable without an oracle: a union contains each of its
    /// operands, so it encloses at least the largest; and it is covered by
    /// them together, so it encloses at most their sum. Checking only the
    /// FINISHED union against the largest single operand is far too weak --
    /// a heart of eleven shells lost a third of itself when the aorta was
    /// merged in and still sat above its largest part -- so the bounds are
    /// carried down the reduction and each pairwise step is held to what the
    /// last one achieved.
    #[test]
    fn a_union_is_held_to_the_volume_bounds_it_must_satisfy() {
        let at = |x: f64, y: f64| {
            let mut c = geom::cube([1.0, 1.0, 1.0], false);
            for p in &mut c.positions {
                p[0] += x;
                p[1] += y;
            }
            c
        };
        // A staircase of eight unit cubes, each overlapping the last by half.
        // True volume: 1 + 7*0.5 = 4.5, and every intermediate step of the
        // reduction has to be inside its own bounds to get there.
        let stack: Vec<Mesh> = (0..8).map(|i| at(i as f64 * 0.5, 0.0)).collect();
        let _ = take_union_trouble();
        let u = union_all(&stack);
        assert!((signed_volume(&u) - 4.5).abs() < 1e-6, "vol {}", signed_volume(&u));
        assert_eq!(take_union_trouble(), (0, 0), "an ordinary union reports no trouble");

        // Disjoint operands never reach a boolean at all, and their volumes
        // simply add -- which is the one case where the upper bound is tight.
        let apart: Vec<Mesh> = (0..8).map(|i| at(i as f64 * 4.0, 0.0)).collect();
        let u = union_all(&apart);
        assert!((signed_volume(&u) - 8.0).abs() < 1e-9, "vol {}", signed_volume(&u));
        assert_eq!(take_union_trouble(), (0, 0));

        // The floor is the point: whatever else a union does, it may not come
        // back holding less than one of the things put into it.
        let mixed = vec![geom::cube([4.0, 4.0, 4.0], true), at(0.0, 0.0), at(0.25, 0.25)];
        let biggest = mixed.iter().map(|m| signed_volume(m)).fold(0.0, f64::max);
        let u = union_all(&mixed);
        assert!(signed_volume(&u) >= biggest * (1.0 - 1e-6), "{} < {biggest}", signed_volume(&u));
    }

    /// Proving two solids apart without running a boolean.
    ///
    /// The whole value is in the sufficient direction: "certainly not
    /// touching" must never be said of solids that do touch, because the
    /// answer is then a concatenation that self-intersects. The other
    /// direction costs only the boolean that would have run anyway.
    #[test]
    fn disjointness_is_proved_only_when_it_holds() {
        let at = |x: f64, y: f64, z: f64, s: f64| {
            let mut c = geom::cube([s, s, s], true);
            for p in &mut c.positions {
                p[0] += x;
                p[1] += y;
                p[2] += z;
            }
            c
        };
        let pair =
            |a: Mesh, b: Mesh| pieces_disjoint(&Piece::of(a).unwrap(), &Piece::of(b).unwrap());

        // Clear of each other, boxes not even overlapping.
        assert!(pair(at(0.0, 0.0, 0.0, 1.0), at(5.0, 0.0, 0.0, 1.0)));
        // Clear of each other, boxes overlapping: two rods crossing at right
        // angles with a gap between them. This is the case the whole thing
        // exists for, and bounding boxes cannot see it.
        let mut rod_x = geom::cube([8.0, 0.5, 0.5], true);
        let mut rod_y = geom::cube([0.5, 8.0, 0.5], true);
        for p in &mut rod_y.positions {
            p[2] += 1.0; // lift it clear
        }
        assert!(pair(rod_x.clone(), rod_y.clone()), "crossed rods with a gap are disjoint");
        // Lower it until they interpenetrate and the proof must fail.
        for p in &mut rod_y.positions {
            p[2] -= 0.8;
        }
        assert!(!pair(rod_x.clone(), rod_y.clone()), "interpenetrating rods are not disjoint");
        // Touching EXACTLY, face to face. Not disjoint: concatenating them
        // would leave the shared face in the mesh twice.
        for p in &mut rod_x.positions {
            p[2] += 0.0;
        }
        assert!(!pair(at(0.0, 0.0, 0.0, 1.0), at(1.0, 0.0, 0.0, 1.0)), "face contact is contact");
        // Overlapping by a sliver.
        assert!(!pair(at(0.0, 0.0, 0.0, 1.0), at(0.999, 0.0, 0.0, 1.0)));
        // NESTED, with no surfaces touching at all. The surface test alone
        // would call this disjoint; the containment ray is what catches it.
        assert!(!pair(at(0.0, 0.0, 0.0, 4.0), at(0.0, 0.0, 0.0, 1.0)), "a box inside a box");
        // And nested off-centre, so the inner one's box is nowhere near the
        // outer one's faces.
        assert!(!pair(at(0.0, 0.0, 0.0, 6.0), at(1.4, 1.4, 1.4, 1.0)));

        // The union itself must agree: crossed rods with a gap add up, and
        // the nested pair does not.
        let mut clear = rod_y.clone();
        for p in &mut clear.positions {
            p[2] += 3.0;
        }
        let u = union_all(&[rod_x.clone(), clear.clone()]);
        let want = signed_volume(&rod_x) + signed_volume(&clear);
        assert!((signed_volume(&u) - want).abs() < 1e-9, "{} vs {want}", signed_volume(&u));
        let u = union_all(&[at(0.0, 0.0, 0.0, 4.0), at(0.0, 0.0, 0.0, 1.0)]);
        assert!((signed_volume(&u) - 64.0).abs() < 1e-6, "nested union is the outer one: {}", signed_volume(&u));
    }

    /// A union must not lose an overlap for being small, or for what else is
    /// in the same union, or for meeting exactly at a face.
    ///
    /// All three came from measuring the merged volume against a tolerance
    /// and keeping the concatenation when it looked like the sum. It cannot
    /// work: two solids that genuinely meet can share arbitrarily little
    /// volume, and two that meet at a face share none at all, so no threshold
    /// separates them from solids that never met. The three exports that
    /// exposed it are reproduced here as they were.
    #[test]
    fn a_union_keeps_every_overlap_it_is_given() {
        let box40 = |dz: f64| {
            let mut m = geom::cube([40.0, 40.0, 40.0], false);
            for p in &mut m.positions {
                p[2] += dz;
            }
            m
        };
        let vol = |m: &Mesh| signed_volume(m);

        // Six microns of overlap on a 40 mm box: 40*40*0.006 = 9.6 mm^3 out
        // of 128,000, which is 7.5e-5 of the pair and vanished under a
        // threshold of 1e-4. The two boxes exported interpenetrating.
        let u = union_all(&[box40(0.0), box40(40.0 - 0.006)]);
        assert!(
            (vol(&u) - (128_000.0 - 9.6)).abs() < 1e-3,
            "a 9.6 mm^3 overlap is 9.6 mm^3: {}",
            vol(&u)
        );

        // A whole millimetre of overlap -- 1,600 mm^3 -- and a 300 mm cube
        // 500 mm away that shares nothing with either. The threshold was a
        // fraction of the running total, so the cube raised it past the
        // overlap and the overlap disappeared; and it disappeared for one
        // ordering of the children and not the other, which the reference
        // forbids: "Order of children never affects the result".
        let mut far = geom::cube([300.0, 300.0, 300.0], false);
        for p in &mut far.positions {
            p[0] += 500.0;
        }
        let want = 128_000.0 - 1_600.0 + 27_000_000.0;
        for order in [
            [box40(0.0), box40(39.0), far.clone()],
            [box40(0.0), far.clone(), box40(39.0)],
            [far.clone(), box40(39.0), box40(0.0)],
        ] {
            let got = vol(&union_all(&order));
            assert!((got - want).abs() < 1e-3, "{got} should be {want} in any order");
        }

        // Two cubes sharing exactly a face. Their union has precisely the
        // volume of their sum, so no measurement of volume can tell it from a
        // pair that never met -- and the concatenation that was kept carried
        // four triangles on the shared plane, an interior wall in a solid the
        // reference says is one solid ("interior face dissolves").
        let mut right = geom::cube([40.0, 40.0, 40.0], false);
        for p in &mut right.positions {
            p[0] += 40.0;
        }
        let u = union_all(&[geom::cube([40.0, 40.0, 40.0], false), right]);
        assert!((vol(&u) - 128_000.0).abs() < 1e-6, "vol {}", vol(&u));
        let wall = u
            .tris
            .iter()
            .filter(|t| {
                t.iter().all(|&i| (u.positions[i as usize][0] - 40.0).abs() < 1e-9)
            })
            .count();
        assert_eq!(wall, 0, "the shared face is interior and must not be in the mesh");
        assert_eq!(crate::geom::closedness_note(&u), None, "and the result is one closed solid");
    }

    /// Four hundred planks on one girder is four hundred edges and no edge
    /// anywhere else, so it is ONE boolean, not four hundred.
    ///
    /// Merging them in one at a time re-tessellates the growing girder every
    /// time; on a suspension bridge that was 32 triangles into 18,746, then
    /// 24,377, then 30,140, and the export did not finish in ten minutes. The
    /// planks share no volume with each other, so the colouring puts them all
    /// in one class and the BSP sees them once.
    #[test]
    fn a_hub_costs_one_boolean_not_one_per_spoke() {
        let mut parts = vec![geom::cube([100.0, 2.0, 2.0], false)];
        for i in 0..40 {
            let mut plank = geom::cube([1.0, 6.0, 1.0], false);
            for p in &mut plank.positions {
                p[0] += 2.0 * i as f64;
                p[1] -= 2.0;
                p[2] += 1.0; // sunk half way into the girder
            }
            parts.push(plank);
        }
        // The girder is 100 x 2 x 2 = 400. Each plank is 1 x 6 x 1 = 6 and
        // sits in the girder over 1 x 2 x 1 = 2, so the union is
        // 400 + 40 * (6 - 2) = 560.
        let u = union_all(&parts);
        let got = signed_volume(&u);
        assert!((got - 560.0).abs() < 1e-6, "{got} should be 560");
        // 492 triangles go in and 1,926 come out: the cut cost of ONE
        // boolean over the whole set. Merging the planks in one at a time
        // cuts the girder again at every step, which is what the suspension
        // bridge measured -- 32 triangles into 18,746, then 24,377, then
        // 30,140 -- and the export did not finish in ten minutes. The bound
        // is loose because the BSP's exact cut count is not the point.
        assert!(u.tris.len() < 4_000, "one boolean, not forty: {} triangles", u.tris.len());

        // And the planks on their own -- pairwise disjoint, nothing to
        // merge -- must not go near a BSP at all: their union is exactly
        // their concatenation, triangle for triangle.
        let planks = &parts[1..];
        let loose = union_all(planks);
        assert_eq!(loose.tris.len(), planks.iter().map(|m| m.tris.len()).sum::<usize>());
        assert_eq!(crate::geom::closedness_note(&loose), None, "and nothing was cut");
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
        let h = hull(&[cube.clone()]);
        assert!((signed_volume(&h).abs() - 8.0).abs() < 1e-6, "vol {}", signed_volume(&h));
        // Raw sign (not abs): the hull is OUTWARD-wound — an inverted
        // hull would pass every abs() check but be culled by the viewer.
        assert!(signed_volume(&h) > 0.0, "hull output must be outward-wound");
        assert!(is_closed_manifold(&h), "hull must be a closed 2-manifold");
        // Hull is convex: every original vertex lies on or behind every
        // output face plane (no point pokes outside).
        let (lo, hi) = bounds(&h);
        assert!((lo[0] + 1.0).abs() < 1e-9 && (hi[0] - 1.0).abs() < 1e-9);

        // Coplanar-base cone (a pyramid): the flat base is one large facet
        // reached over many coplanar rim points — a path the cube misses.
        let cone = geom::cylinder(4.0, 3.0, 0.0, false, 16);
        let h = hull(&[cone.clone()]);
        assert!(signed_volume(&h) > 0.0 && is_closed_manifold(&h));
        assert!((signed_volume(&h).abs() - signed_volume(&cone).abs()).abs() < 1e-6);

        // Duplicate/coincident vertices (two identical cubes) must not
        // corrupt the hull.
        let h = hull(&[cube.clone(), cube.clone()]);
        assert!((signed_volume(&h).abs() - 8.0).abs() < 1e-6 && is_closed_manifold(&h));

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
            Minkowski::TooLarge { .. } => panic!("cube⊕small-sphere must fit the cap"),
        };
        let (lo, hi) = bounds(&rounded);
        // Grown from [-5,5] toward [-7,7] on X (sphere radius 2).
        assert!(lo[0] < -6.5 && hi[0] > 6.5, "grew: {:?}..{:?}", lo, hi);
        assert!(signed_volume(&rounded).abs() > signed_volume(&cube).abs());
        // Raw sign (not abs): the result is OUTWARD-wound like every mesh.
        assert!(signed_volume(&rounded) > 0.0, "minkowski output must be outward");

        // Single operand is identity (passes through unchanged, NOT hulled).
        match minkowski(&[geom::cube([2.0, 2.0, 2.0], true)]) {
            Minkowski::Ok(m) => assert!((signed_volume(&m).abs() - 8.0).abs() < 1e-9),
            Minkowski::TooLarge { .. } => panic!(),
        }

        // Oversized product is reported (with the completed partial), not hung.
        let s = geom::sphere(5.0, 40);
        match minkowski(&[s.clone(), s]) {
            Minkowski::TooLarge { count, partial } => {
                assert!(count > MINKOWSKI_MAX_POINTS);
                assert!(!partial.positions.is_empty()); // the first operand survived
            }
            Minkowski::Ok(_) => panic!("two dense spheres must exceed the cap"),
        }
    }

    #[test]
    fn is_simple_loop_rejects_disjoint_cycles() {
        // One triangle loop — accepted.
        assert!(is_simple_loop(&[(0, 1), (1, 2), (2, 0)]));
        // Two disjoint triangles: every vertex degree 1 in/out, but NOT a
        // single cycle — must be rejected (the tightened guard).
        assert!(!is_simple_loop(&[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)]));
        // A vertex with out-degree 2 — rejected.
        assert!(!is_simple_loop(&[(0, 1), (0, 2), (1, 2)]));
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
        // Either it degenerated to empty (acceptable) or it is closed.
        assert!(h.tris.is_empty() || is_closed_manifold(&h), "non-manifold hull");
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

    #[test]
    fn booleans_are_scale_invariant() {
        // REGRESSION. `Plane::from_points` rejected a triangle by an ABSOLUTE
        // area floor (|n| < 1e-7 on the unnormalized cross product), so every
        // facet below ~5e-8 square units was silently dropped before any
        // boolean ran. The same model in millimetres and in metres gave
        // different answers: intersections annihilated, unions returned one
        // operand, differences returned the minuend with the cutter ignored.
        // It fired at ORDINARY scale too — 8 of 22,496 facets of
        // sphere(r=1,$fn=150) were below the floor.
        //
        // The property: a boolean's volume, divided by scale^3, is constant.
        let mut reference: Option<f64> = None;
        for k in [2i32, 0, -2, -4, -6] {
            let s = 10f64.powi(k);
            let a = geom::cube([s, s, s], true);
            let mut b = geom::cube([s, s, s], true);
            for p in b.positions.iter_mut() {
                p[0] += s * 0.5;
            }
            let out = intersection_all(&[a, b]);
            assert!(!out.tris.is_empty(), "scale 1e{k}: intersection came back EMPTY");
            let norm = signed_volume(&out).abs() / (s * s * s);
            match reference {
                None => reference = Some(norm),
                Some(r0) => assert!(
                    (norm - r0).abs() / r0 < 1e-9,
                    "scale 1e{k}: normalized volume {norm} != {r0}"
                ),
            }
        }
        // A sliver — zero area at ANY scale — is still rejected.
        assert!(Plane::from_points([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]).is_none());
        // ...while a small, well-shaped triangle is not.
        assert!(Plane::from_points([0.0, 0.0, 0.0], [1e-9, 0.0, 0.0], [0.0, 1e-9, 0.0]).is_some());
    }

    #[test]
    fn a_merge_does_not_report_degradation() {
        // The guards that stop a pathological BSP from aborting the process
        // truncate the tree, which can leave an open mesh — so the evaluator
        // warns. But the MERGE stage of every boolean rebuilds into a node
        // that already has a plane, and its tree is only read back through
        // all_polygons; the guard firing there changes nothing. Tracking both
        // put the warning on six of nine ordinary demo scenes.
        //
        // This test used to prove the flag CAN fire by running a boolean at a
        // magnitude of 1e11, where the plane tolerance stopped resolving
        // anything and every split came back stuck. Magnitude is no longer
        // pathological input — booleans are solved in a normalised frame now
        // — so that half is gone, and `booleans_hold_their_shape_at_any_
        // magnitude` asserts the opposite and stronger thing in its place.
        let _ = take_degraded();
        let out = difference(&geom::cube([10.0, 10.0, 10.0], true), &[geom::sphere(6.0, 12)]);
        assert!(!out.tris.is_empty());
        assert!(!take_degraded(), "an ordinary boolean reported degradation");
    }

    /// minkowski() built its point cloud and called convex_hull directly,
    /// skipping the frame that hull() is wrapped in — and so kept the exact
    /// scale dependence that wrapper exists to remove. A 1e-6 operand came
    /// back 18% short and a 1e-7 one nearly 80% short.
    #[test]
    fn minkowski_holds_its_shape_at_any_magnitude() {
        fn vol(m: &Mesh) -> f64 {
            if m.positions.is_empty() {
                return 0.0;
            }
            let o = m.positions[0];
            m.tris
                .iter()
                .map(|t| {
                    let r = |i: u32| sub(m.positions[i as usize], o);
                    dot(r(t[0]), cross(r(t[1]), r(t[2]))) / 6.0
                })
                .sum::<f64>()
                .abs()
        }
        let at = |s: f64| -> f64 {
            let c = geom::cube([10.0 * s, 10.0 * s, 10.0 * s], false);
            let b = geom::sphere(2.0 * s, 12);
            match minkowski(&[c, b]) {
                Minkowski::Ok(m) => vol(&m) / (s * s * s),
                Minkowski::TooLarge { .. } => panic!("hit the pair cap"),
            }
        };
        let base = at(1.0);
        assert!(base > 0.0, "reference minkowski volume {base}");
        for s in [1e-2, 1e-4, 1e-5, 1e-6, 1e-7, 1e3, 1e6] {
            let got = at(s);
            assert!(
                (got - base).abs() <= base * 1e-9,
                "minkowski at scale {s:e}: {got} vs {base}"
            );
        }
    }

    /// A boolean must give the same answer wherever the geometry sits and
    /// whatever it is measured in — a model authored in metres with
    /// millimetre features, or one a translate() has put a long way out,
    /// is not a harder problem, only a differently written one.
    ///
    /// It used to be a much harder problem. Every plane carries
    /// `w = dot(normal, a)` and classification asks for `dot(normal, p) - w`,
    /// so both terms grow with the coordinates while their difference stays
    /// the small distance being tested: at 1e9 that difference has an ulp
    /// near the 1e-7 tolerance itself and the classification becomes noise,
    /// while at a side of 1e-7 the whole solid is inside the tolerance and
    /// the result comes back EMPTY.
    ///
    /// The geometry is built ONCE and its vertices moved, so the primitives'
    /// own fragment counts cannot vary with scale and the boolean is the only
    /// thing under test.
    #[test]
    fn booleans_hold_their_shape_at_any_magnitude() {
        fn vol(m: &Mesh) -> f64 {
            if m.positions.is_empty() {
                return 0.0;
            }
            // Measured about a local origin: summing products of raw
            // coordinates is itself unstable far from it.
            let o = m.positions[0];
            m.tris
                .iter()
                .map(|t| {
                    let r = |i: u32| sub(m.positions[i as usize], o);
                    let (a, b, c) = (r(t[0]), r(t[1]), r(t[2]));
                    dot(a, cross(b, c)) / 6.0
                })
                .sum::<f64>()
                .abs()
        }
        let cube = geom::cube([1.0, 1.0, 1.0], true);
        let ball = geom::sphere(0.6, 12);
        let place = |m: &Mesh, s: f64, d: f64| {
            let mut q = m.clone();
            for p in &mut q.positions {
                for k in 0..3 {
                    p[k] = p[k] * s + d;
                }
            }
            q
        };
        let mut far = ball.clone();
        for p in &mut far.positions {
            p[0] += 2.0; // clear of the cube, so the hull spans both
        }
        let base = [
            vol(&union_all(&[cube.clone(), ball.clone()])),
            vol(&difference(&cube, &[ball.clone()])),
            vol(&intersection_all(&[cube.clone(), ball.clone()])),
            vol(&hull(&[cube.clone(), far.clone()])),
        ];
        assert!(base.iter().all(|v| *v > 0.0), "reference volumes {:?}", base);
        for (s, d) in [
            (1e-9, 0.0),
            (1e-6, 0.0),
            (1e-3, 0.0),
            (1e6, 0.0),
            (1e11, 0.0),
            (1.0, 1e6),
            (1.0, 1e9),
            (1.0, 1e12),
            (1e-6, 1e-3),
            (1e6, 1e11),
        ] {
            let (a, b) = (place(&cube, s, d), place(&ball, s, d));
            let h = place(&far, s, d);
            let got = [
                vol(&union_all(&[a.clone(), b.clone()])),
                vol(&difference(&a, &[b.clone()])),
                vol(&intersection_all(&[a.clone(), b.clone()])),
                vol(&hull(&[a.clone(), h])),
            ];
            // Moving a shape out to `d` quantises its vertices onto the grid
            // of representable numbers there, so the shape itself can only be
            // held to ulp(d)/s in relative terms. That is the input's limit,
            // not the kernel's; below it the boolean must be exact.
            let grain = (d.abs() / s) * f64::EPSILON * 16.0;
            for k in 0..4 {
                let scaled = got[k] / (s * s * s);
                assert!(
                    (scaled - base[k]).abs() <= base[k] * (1e-9 + grain),
                    "op {} at side {:e} offset {:e}: {} vs {}",
                    ["union", "difference", "intersection", "hull"][k],
                    s,
                    d,
                    scaled,
                    base[k]
                );
            }
        }
    }

    #[test]
    fn convexity_test_separates_the_minkowski_cases() {
        assert!(is_convex(&geom::cube([2.0, 3.0, 4.0], true)), "a cube is convex");
        assert!(is_convex(&geom::sphere(1.0, 12)), "a sphere is convex");
        assert!(is_convex(&geom::cylinder(3.0, 1.0, 0.0, true, 12)), "a cone is convex");
        // An L: two boxes unioned is not.
        let l = union_all(&[
            geom::cube([10.0, 3.0, 3.0], true),
            geom::cube([3.0, 10.0, 3.0], true),
        ]);
        assert!(!is_convex(&l), "an L-shape is not convex");
    }

    #[test]
    fn hull_emission_is_reproducible() {
        // REGRESSION. The horizon edges were read back out of a HashSet,
        // and Rust seeds HashSet's hasher randomly PER PROCESS — so the cap
        // faces were appended in a different order every run, and `hull()`
        // emitted a different mesh (different triangles AND different
        // vertex numbering) each time the same file was exported. Five runs
        // of one two-sphere hull gave five different STLs.
        //
        // The property that broke is reproducibility of the emitted
        // sequence, so that is what this pins: an exact snapshot. The
        // set-level test below does NOT catch it — it sorts the very
        // ordering the bug perturbs — which is why both exist.
        let pts: Vec<V3> = vec![
            [0.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [4.0, 3.0, 0.0],
            [0.0, 3.0, 0.0],
            [0.0, 0.0, 2.0],
            [4.0, 0.0, 2.0],
            [4.0, 3.0, 2.0],
            [0.0, 3.0, 2.0],
        ];
        let m = convex_hull(&pts);
        assert_eq!(
            m.tris,
            vec![
                [0, 1, 2],
                [0, 2, 3],
                [2, 4, 3],
                [1, 0, 5],
                [0, 3, 5],
                [3, 4, 5],
                [4, 1, 6],
                [1, 5, 6],
                [5, 4, 6],
                [4, 2, 7],
                [2, 1, 7],
                [1, 4, 7],
            ],
            "hull emission order drifted — check for an unordered container \
             whose iteration reaches the output"
        );
    }

    #[test]
    fn hull_output_does_not_depend_on_point_order() {
        // A strictly convex point set (no four points coplanar) has a
        // unique hull triangulation, so the emitted triangles must match as
        // a SET whatever order the points arrive in. Independent of the
        // reproducibility pin above: this one would still hold under a
        // deterministic-but-order-sensitive implementation.
        let pts: Vec<V3> = (0..60)
            .map(|i| {
                // A deterministic spiral on the unit sphere: irrational
                // pitch keeps any four points off a common plane.
                let k = i as f64;
                let z = 1.0 - 2.0 * (k + 0.5) / 60.0;
                let r = (1.0 - z * z).sqrt();
                let a = k * 2.399963229728653; // golden angle, radians
                [r * a.cos() * 10.0, r * a.sin() * 10.0, z * 10.0]
            })
            .collect();

        let canonical = |m: &Mesh| {
            let mut tris: Vec<[String; 3]> = m
                .tris
                .iter()
                .map(|t| {
                    // Key each triangle by its vertex COORDINATES, sorted,
                    // so index numbering and winding rotation don't matter.
                    let mut v: Vec<String> = t
                        .iter()
                        .map(|&i| {
                            let p = m.positions[i as usize];
                            format!("{:.9},{:.9},{:.9}", p[0], p[1], p[2])
                        })
                        .collect();
                    v.sort();
                    [v[0].clone(), v[1].clone(), v[2].clone()]
                })
                .collect();
            tris.sort();
            tris
        };

        let base = canonical(&convex_hull(&pts));
        assert!(base.len() > 50, "expected a real hull, got {} tris", base.len());

        let mut rotated = pts.clone();
        rotated.rotate_left(17);
        assert_eq!(base, canonical(&convex_hull(&rotated)), "rotating the input changed the hull");

        let mut reversed = pts.clone();
        reversed.reverse();
        assert_eq!(base, canonical(&convex_hull(&reversed)), "reversing the input changed the hull");
    }

    #[test]
    fn booleans_survive_large_coordinates() {
        // REGRESSION. `difference() { cube(1e9, center=true); sphere(0.6e9); }`
        // aborted the PROCESS with a stack overflow: split_polygon's epsilon
        // is absolute, so at that magnitude a facet is neither coplanar with
        // the chosen plane nor cleanly on one side, and build() handed the
        // same set to a child forever. A model measured in microns reaches
        // 1e9 without trying. Accuracy may degrade out here; aborting may not.
        for mag in [1.0e6f64, 1.0e9, 1.0e10, 1.0e13] {
            let mut cube = geom::cube([mag, mag, mag], true);
            let mut ball = geom::sphere(0.6 * mag, 12);
            // Nudge both off-origin too — the same defect showed up there.
            for p in cube.positions.iter_mut().chain(ball.positions.iter_mut()) {
                p[0] += mag * 0.5;
            }
            let out = difference(&cube, &[ball]);
            assert!(!out.tris.is_empty(), "magnitude {mag}: difference came back empty");
            assert!(
                out.positions.iter().all(|p| p.iter().all(|c| c.is_finite())),
                "magnitude {mag}: non-finite vertices"
            );
        }
    }

    // ---- Refuter #1 adversarial probes for convex_hull correctness ----

    /// Is `p` present in the hull's vertex set (within tolerance)?
    fn retained(m: &Mesh, p: V3) -> bool {
        m.positions.iter().any(|q| dist2(*q, p) < 1e-12)
    }

    /// Convex + closed-2-manifold checks on a hull output.
    fn assert_clean_hull(m: &Mesh, all_pts: &[V3]) {
        use std::collections::HashMap;
        // Closed: every undirected edge shared by exactly two triangles.
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
        for t in &m.tris {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let key = if a < b { (a, b) } else { (b, a) };
                *edges.entry(key).or_insert(0) += 1;
            }
        }
        assert!(edges.values().all(|&c| c == 2), "non-manifold hull");
        // Convex: no input point strictly in front of any output face plane.
        for f in &m.tris {
            let a = m.positions[f[0] as usize];
            let b = m.positions[f[1] as usize];
            let c = m.positions[f[2] as usize];
            let n = norm(cross(sub(b, a), sub(c, a)));
            let w = dot(n, a);
            for p in all_pts {
                assert!(dot(n, *p) - w < 1e-9, "point {:?} pokes out of a face", p);
            }
        }
    }

    #[test]
    fn hull_retains_coplanar_cap_corners() {
        // An octagonal PRISM: two big flat coplanar caps (z=0 and z=10) of
        // 8 corners each. Every one of the 16 corners is a genuine hull
        // vertex. This is the case a fixed-EPS visibility test is supposed
        // to be able to miss: a cap corner is coplanar (dist 0) with its
        // cap face and only pokes out past a cap edge, seen by a side face.
        let n = 8usize;
        let r = 5.0;
        let mut pts: Vec<V3> = Vec::new();
        for k in 0..n {
            let a = std::f64::consts::TAU * (k as f64) / (n as f64);
            pts.push([r * a.cos(), r * a.sin(), 0.0]);
            pts.push([r * a.cos(), r * a.sin(), 10.0]);
        }
        let m = hull(&[Mesh { positions: pts.clone(), tris: vec![] }]);
        for p in &pts {
            assert!(retained(&m, *p), "prism corner {:?} was dropped from the hull", p);
        }
        assert_clean_hull(&m, &pts);
    }

    #[test]
    fn hull_retains_square_bipyramid_equator() {
        // Square-equator bipyramid: 4 coplanar equator corners (z=0) plus a
        // top and bottom apex. All 6 are hull vertices; the equator corners
        // are each coplanar with an equatorial-ish facet chord.
        let pts: Vec<V3> = vec![
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [-2.0, 0.0, 0.0],
            [0.0, -2.0, 0.0],
            [0.0, 0.0, 3.0],
            [0.0, 0.0, -3.0],
        ];
        let m = hull(&[Mesh { positions: pts.clone(), tris: vec![] }]);
        for p in &pts {
            assert!(retained(&m, *p), "bipyramid vertex {:?} dropped", p);
        }
        assert_clean_hull(&m, &pts);
        // Octahedron volume = (1/3) * base-diagonal-area * total-height.
        // base is a square of diagonal 4 -> area 8; height 6 -> vol 16.
        assert!((signed_volume(&m).abs() - 16.0).abs() < 1e-6, "vol {}", signed_volume(&m));
    }

    // Deterministic LCG so the battery is reproducible without extern crates.
    fn lcg(state: &mut u64) -> f64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((*state >> 11) as f64) / ((1u64 << 53) as f64) // in [0,1)
    }

    #[test]
    fn refuter_adversarial_hull_battery() {
        // (1) Icosahedron: 12 vertices, ALL extreme, highly symmetric facets.
        let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
        let mut ico: Vec<V3> = Vec::new();
        for &s1 in &[-1.0, 1.0] {
            for &s2 in &[-1.0, 1.0] {
                ico.push([0.0, s1 * 1.0, s2 * phi]);
                ico.push([s1 * 1.0, s2 * phi, 0.0]);
                ico.push([s1 * phi, 0.0, s2 * 1.0]);
            }
        }
        let m = hull(&[Mesh { positions: ico.clone(), tris: vec![] }]);
        for p in &ico {
            assert!(retained(&m, *p), "icosahedron vertex {:?} dropped", p);
        }
        assert_clean_hull(&m, &ico);
        assert!(signed_volume(&m) > 0.0, "icosa vol {}", signed_volume(&m));

        // (2) Rotated cube (non-axis-aligned coplanar faces) with midpoints on
        // every face and edge -> lots of exactly-coplanar non-vertex points.
        let mut cube: Vec<V3> = Vec::new();
        for &x in &[-1.0, 0.0, 1.0] {
            for &y in &[-1.0, 0.0, 1.0] {
                for &z in &[-1.0, 0.0, 1.0] {
                    cube.push([x, y, z]);
                }
            }
        }
        // Rotate by an irrational-ish angle about all three axes.
        let (ca, sa) = (0.6, 0.8);
        for p in &mut cube {
            let [x, y, z] = *p;
            let x2 = ca * x - sa * y;
            let y2 = sa * x + ca * y;
            let y3 = ca * y2 - sa * z;
            let z3 = sa * y2 + ca * z;
            *p = [x2 * 100.0, y3 * 100.0, z3 * 100.0];
        }
        let m = hull(&[Mesh { positions: cube.clone(), tris: vec![] }]);
        assert_clean_hull(&m, &cube);
        assert!(signed_volume(&m) > 0.0, "rot-cube vol {}", signed_volume(&m));
        // The 8 rotated CORNERS (|x|=|y|=|z|=1 before rotation) must survive.
        for p in &cube {
            // corner iff pre-rotation it was a unit corner; detect by magnitude.
            if (dot(*p, *p).sqrt() - (3.0_f64.sqrt() * 100.0)).abs() < 1e-6 {
                assert!(retained(&m, *p), "rotated cube corner {:?} dropped", p);
            }
        }

        // (3) Random clouds at several scales and orderings, with an interior
        // point forced FIRST (pts[0] not extreme) and far outliers appended.
        let mut st: u64 = 0x1234_5678_9abc_def0;
        for &scale in &[0.5f64, 1.0, 100.0, 10000.0] {
            for _trial in 0..40 {
                let n = 30 + (lcg(&mut st) * 120.0) as usize;
                let mut pts: Vec<V3> = Vec::new();
                // interior-ish seed first
                pts.push([0.0, 0.0, 0.0]);
                for _ in 0..n {
                    pts.push([
                        (lcg(&mut st) - 0.5) * 2.0 * scale,
                        (lcg(&mut st) - 0.5) * 2.0 * scale,
                        (lcg(&mut st) - 0.5) * 2.0 * scale,
                    ]);
                }
                // a couple of clear far outliers -> guaranteed hull vertices
                let outliers = [
                    [scale * 3.0, 0.1, -0.2],
                    [-0.3, scale * 3.0, 0.4],
                    [0.2, -0.1, scale * 3.0],
                ];
                for o in outliers {
                    pts.push(o);
                }
                let m = hull(&[Mesh { positions: pts.clone(), tris: vec![] }]);
                if m.tris.is_empty() {
                    continue; // degenerate draw; skip
                }
                assert_clean_hull(&m, &pts);
                assert!(
                    signed_volume(&m) > 0.0,
                    "random cloud negative/zero vol {} scale {}",
                    signed_volume(&m),
                    scale
                );
                for o in outliers {
                    assert!(retained(&m, o), "far outlier {:?} dropped (scale {})", o, scale);
                }
            }
        }
    }

    #[test]
    fn refuter_dodeca_and_dense_sphere() {
        // Dodecahedron: 20 vertices, 12 PENTAGONAL faces (each 3 coplanar
        // tris). Every vertex is extreme; pentagon interiors stress coplanar
        // near-silhouette visibility.
        let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
        let ip = 1.0 / phi;
        let mut d: Vec<V3> = Vec::new();
        for &x in &[-1.0, 1.0] {
            for &y in &[-1.0, 1.0] {
                for &z in &[-1.0, 1.0] {
                    d.push([x, y, z]);
                }
            }
        }
        for &a in &[-ip, ip] {
            for &b in &[-phi, phi] {
                d.push([0.0, a, b]);
                d.push([a, b, 0.0]);
                d.push([b, 0.0, a]);
            }
        }
        let m = hull(&[Mesh { positions: d.clone(), tris: vec![] }]);
        for p in &d {
            assert!(retained(&m, *p), "dodecahedron vertex {:?} dropped", p);
        }
        assert_clean_hull(&m, &d);
        assert!(signed_volume(&m) > 0.0);

        // Dense sphere, points fed in a deliberately adversarial order (sorted
        // by angle so consecutive points are near-coplanar-adjacent), plus an
        // interior point first. Every sphere point is extreme -> all retained.
        let n = 1200usize;
        let ga = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
        let mut pts: Vec<V3> = vec![[0.0, 0.0, 0.0]];
        for i in 0..n {
            let z = 1.0 - 2.0 * (i as f64 + 0.5) / (n as f64);
            let r = (1.0 - z * z).max(0.0).sqrt();
            let t = ga * (i as f64);
            pts.push([5.0 * r * t.cos(), 5.0 * r * t.sin(), 5.0 * z]);
        }
        // sort the sphere points (excluding the interior seed) by z then angle
        let seed = pts[0];
        let mut sph = pts[1..].to_vec();
        sph.sort_by(|a, b| {
            a[2].total_cmp(&b[2]).then(a[1].atan2(a[0]).total_cmp(&b[1].atan2(b[0])))
        });
        let mut ordered = vec![seed];
        ordered.extend(sph.iter().cloned());
        let m = hull(&[Mesh { positions: ordered.clone(), tris: vec![] }]);
        let kept = sph.iter().filter(|p| retained(&m, **p)).count();
        assert_eq!(kept, n, "sorted-order sphere dropped {} pts", n - kept);
        assert_clean_hull(&m, &ordered);
        // Euler characteristic of a closed genus-0 triangulation: V - E + F = 2.
        use std::collections::HashSet;
        let mut es: HashSet<(u32, u32)> = HashSet::new();
        for t in &m.tris {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                es.insert(if a < b { (a, b) } else { (b, a) });
            }
        }
        let (v, e, f) = (m.positions.len() as i64, es.len() as i64, m.tris.len() as i64);
        assert_eq!(v - e + f, 2, "Euler char != 2 (V={} E={} F={})", v, e, f);
    }

    #[test]
    fn hull_retains_all_points_on_sphere() {
        // A Fibonacci-style point cloud entirely ON a sphere: EVERY point is
        // a hull vertex, so a correct hull must retain all of them.
        let n = 500usize;
        let ga = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
        let mut pts: Vec<V3> = Vec::new();
        for i in 0..n {
            let z = 1.0 - 2.0 * (i as f64 + 0.5) / (n as f64);
            let r = (1.0 - z * z).max(0.0).sqrt();
            let t = ga * (i as f64);
            pts.push([10.0 * r * t.cos(), 10.0 * r * t.sin(), 10.0 * z]);
        }
        let m = hull(&[Mesh { positions: pts.clone(), tris: vec![] }]);
        let kept = pts.iter().filter(|p| retained(&m, **p)).count();
        assert_eq!(kept, n, "only {}/{} sphere points retained", kept, n);
        assert_clean_hull(&m, &pts);
    }
}
