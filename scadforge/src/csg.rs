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
            self.plane = Some(polygons[0].plane.clone());
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
    if pa.is_empty() {
        // Union/difference of empty-with-B: identity rules handled by the
        // caller for difference; here union → b, intersection → empty.
        return match op {
            Op::Union => b.clone(),
            _ => Mesh::empty(),
        };
    }
    if pb.is_empty() {
        return a.clone();
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

/// n-ary union of a list of meshes (empty meshes are skipped).
pub fn union_all(meshes: &[Mesh]) -> Mesh {
    let mut iter = meshes.iter().filter(|m| !m.positions.is_empty());
    let mut acc = match iter.next() {
        Some(m) => m.clone(),
        None => return Mesh::empty(),
    };
    for m in iter {
        acc = boolean(&acc, m, Op::Union);
    }
    acc
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

/// n-ary intersection: the region common to every mesh.
pub fn intersection_all(meshes: &[Mesh]) -> Mesh {
    let mut iter = meshes.iter();
    let mut acc = match iter.next() {
        Some(m) => m.clone(),
        None => return Mesh::empty(),
    };
    for m in iter {
        if acc.positions.is_empty() {
            return Mesh::empty();
        }
        acc = boolean(&acc, m, Op::Intersection);
    }
    acc
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
}
