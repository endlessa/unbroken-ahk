//! Geometry kernel (slice): meshes, 4x4 affine transforms, and the three
//! 3D primitives, built to the converged language reference's semantics —
//! the exact $fn/$fa/$fs fragment formula, sphere rings at half-step polar
//! offsets (never a pole vertex), cylinder apex collapse at r=0.

pub type Vec3 = [f64; 3];
pub type Mat4 = [[f64; 4]; 4];

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub tris: Vec<[u32; 3]>,
}

impl Mesh {
    pub fn empty() -> Mesh {
        Mesh { positions: Vec::new(), tris: Vec::new() }
    }

    /// Merge vertices that sit at EXACTLY the same position, and drop any
    /// triangle left with a repeated corner.
    ///
    /// The BSP carries every polygon's corners independently, so a boolean
    /// hands back a mesh in which no two triangles share a vertex at all:
    /// `difference() { cube(10, center = true); sphere(6, $fn = 24); }`
    /// exported 3065 vertex records for 982 distinct positions. STL does not
    /// care — it is a soup format — but OFF, AMF and 3MF all write explicit
    /// indices, so the topology those formats exist to express was thrown
    /// away and the files came out three times larger than the geometry
    /// needs.
    ///
    /// Only bit-identical positions merge, so this cannot move a vertex or
    /// change what the mesh encloses; it is bookkeeping, not repair. The
    /// T-junctions the splitter leaves behind are a separate matter and
    /// survive this.
    pub fn welded(&self) -> Mesh {
        use std::collections::HashMap;
        let mut map: HashMap<[u64; 3], u32> = HashMap::new();
        let mut positions: Vec<Vec3> = Vec::with_capacity(self.positions.len());
        let mut at: Vec<u32> = Vec::with_capacity(self.positions.len());
        for p in &self.positions {
            // -0.0 and 0.0 are one position, as they are for the STL welder.
            let bits = |v: f64| if v == 0.0 { 0f64.to_bits() } else { v.to_bits() };
            let key = [bits(p[0]), bits(p[1]), bits(p[2])];
            let next = positions.len() as u32;
            let idx = *map.entry(key).or_insert(next);
            if idx == next {
                positions.push(*p);
            }
            at.push(idx);
        }
        let tris = self
            .tris
            .iter()
            .map(|t| [at[t[0] as usize], at[t[1] as usize], at[t[2] as usize]])
            .filter(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2])
            .collect();
        Mesh { positions, tris }
    }

    /// Drop triangles that no export format can represent as a surface.
    ///
    /// The 2D fill sweep resolves crossings numerically, so two scanline
    /// crossings can land an ULP apart and the trapezoid between them comes
    /// out a sliver: real at f64 (area 5e-17 on a 143-unit glyph), and
    /// exactly collinear once written. Across the example corpus that was
    /// 222 facets in 2.5 million, every one of them exported as
    /// `facet normal 0 0 0` -- a facet whose normal the STL format requires
    /// and which no reader can recover, because the three vertices listed
    /// beside it are collinear.
    ///
    /// The test is the writers' own resolution, not a magic number. The text
    /// formats print coordinates with `{:.6}`, so they land on a 1e-6 grid;
    /// binary STL stores f32, whose spacing near a coordinate is about
    /// `|x| * f32::EPSILON`. On a grid of spacing `res` the thinnest triangle
    /// that is still a triangle has `|cross| == res * res`, so anything below
    /// that is collinear as written however it is written. Dropping it
    /// removes no surface -- volume and area over the whole corpus are
    /// unchanged to twelve significant digits -- and it happens ONCE, at the
    /// export funnel, so every format agrees on the triangle list.
    pub fn without_unrepresentable(&self) -> Mesh {
        /// |cross| -- twice the area -- of a triangle whose vertices have
        /// been moved onto the grid a writer will put them on.
        fn cross_on_grid(pts: [Vec3; 3], snap: impl Fn(f64) -> f64) -> f64 {
            let g = |p: Vec3| [snap(p[0]), snap(p[1]), snap(p[2])];
            let (a, b, c) = (g(pts[0]), g(pts[1]), g(pts[2]));
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt()
        }
        let keep = |t: &[u32; 3]| {
            let pts = [
                self.positions[t[0] as usize],
                self.positions[t[1] as usize],
                self.positions[t[2] as usize],
            ];
            let mag = pts.iter().flatten().fold(0.0f64, |m, c| m.max(c.abs()));
            // The text writers print `{:.6}`, landing on a 1e-6 grid; binary
            // STL stores f32. The vertices are SNAPPED before the test, not
            // just compared against the grid: rounding moves each corner by
            // up to half a step, which is itself enough to flatten a triangle
            // that was thin but real beforehand.
            let text = cross_on_grid(pts, |x| (x * 1e6).round() / 1e6);
            let f32_res = (mag * f32::EPSILON as f64).max(f32::MIN_POSITIVE as f64);
            let binary = cross_on_grid(pts, |x| x as f32 as f64);
            text.is_finite()
                && binary.is_finite()
                && text >= 1e-12
                && binary >= f32_res * f32_res
        };
        Mesh {
            positions: self.positions.clone(),
            tris: self.tris.iter().copied().filter(|t| keep(t)).collect(),
        }
    }
}

/// The most fragments any primitive will tessellate a full circle into.
///
/// `sphere` is QUADRATIC in this (rings x n), so the bound is what keeps a
/// large $fn from turning into an uncatchable allocation abort — and an abort
/// takes the .echo diagnostic stream down with it. 1024 is the same ceiling
/// the 2D offset kernel already uses, is visually indistinguishable from
/// smooth at any sane viewing size, and still yields a 1M-triangle sphere.
/// `eval::resolve_fragments` warns when it bites, so the clamp is never
/// silent.
pub const MAX_FRAGMENTS: u32 = 1024;

/// GRID_FINE from the reference: radii below 2^-20 always get 3 fragments.
const GRID_FINE: f64 = 1.0 / 1_048_576.0;

/// The single exact tessellation formula from the reference:
/// fragments(r, $fn, $fa, $fs) =
///   (r < 2^-20 or non-finite r/$fn) ? 3
///   : ($fn > 0) ? max(int($fn), 3)
///   : ceil(max(min(360/$fa, 2*PI*r/$fs), 5))
///
/// The non-finite test comes BEFORE `$fn > 0`, per the reference: "NaN or
/// +/-inf $fn instead short-circuits to exactly 3 fragments (it shares the
/// tiny-radius branch, tested BEFORE $fn > 0)". Order matters enormously
/// here — infinity passes `> 0`, and `f64::INFINITY as i64` SATURATES to
/// i64::MAX, which `as u32` then truncates to 4_294_967_295. The caller
/// duly asked for a 4.29-billion-point circle and the allocator aborted the
/// process. `$fn = 360/steps` with `steps == 0` reaches infinity without
/// trying, and an abort is not catchable: no .echo file was written either,
/// destroying the one channel that would have explained it.
///
/// $fa/$fs below 0.01 clamp to 0.01. The clamp WARNING the reference
/// describes is raised by the caller (`eval::resolve_fragments`), which has
/// the diagnostic stream; this function stays pure and clamps defensively.
pub fn fragments(r: f64, fn_: f64, fa: f64, fs: f64) -> u32 {
    if !r.is_finite() || r < GRID_FINE || !fn_.is_finite() {
        return 3;
    }
    if fn_ > 0.0 {
        // Saturating on both sides: the cast above is exactly where the
        // 68 GB allocation came from. u32::MAX was still far too generous —
        // `sphere` allocates ~n^2/2 vertices, so a merely LARGE finite $fn
        // (40000, a plausible "max quality" value or a trailing-zero typo)
        // asked for 19 GB and aborted the process just as fatally. An
        // allocation failure is a Rust abort, not a catchable panic, so the
        // .echo file explaining it never got written either.
        return (fn_ as i64).clamp(3, MAX_FRAGMENTS as i64) as u32;
    }
    let fa = fa.max(0.01);
    let fs = fs.max(0.01);
    let by_angle = 360.0 / fa;
    let by_arc = 2.0 * std::f64::consts::PI * r / fs;
    let n = by_angle.min(by_arc).max(5.0).ceil();
    if !n.is_finite() { 3 } else { n.clamp(3.0, MAX_FRAGMENTS as f64) as u32 }
}

/// What the $fa/$fs branch WOULD produce before the cap, so the evaluator can
/// tell whether the cap bit and say so. `MAX_FRAGMENTS`' own doc comment
/// promises "the clamp is never silent", but the warning only ever tested the
/// $fn branch: with $fn = 0 and small $fa/$fs, a `cylinder(h=1, r=100)` asked
/// for 3600 fragments, silently got 1024, and nothing in the console said the
/// model was rendered coarser than the script asked for.
pub fn uncapped_fragments(r: f64, fn_: f64, fa: f64, fs: f64) -> f64 {
    if fn_ > 0.0 || !r.is_finite() || r <= 0.0 {
        return 0.0; // the $fn branch, or a degenerate radius: not this path
    }
    let by_angle = 360.0 / fa.max(0.01);
    let by_arc = 2.0 * std::f64::consts::PI * r / fs.max(0.01);
    let n = by_angle.min(by_arc).max(5.0).ceil();
    if n.is_finite() { n } else { 0.0 }
}

// -- Matrices ---------------------------------------------------------------

pub fn identity() -> Mat4 {
    let mut m = [[0.0; 4]; 4];
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    m
}

pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut m = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            m[i][j] = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    m
}

pub fn translation(v: Vec3) -> Mat4 {
    let mut m = identity();
    m[0][3] = v[0];
    m[1][3] = v[1];
    m[2][3] = v[2];
    m
}

pub fn scaling(v: Vec3) -> Mat4 {
    let mut m = identity();
    m[0][0] = v[0];
    m[1][1] = v[1];
    m[2][2] = v[2];
    m
}

fn rot_x(deg: f64) -> Mat4 {
    let (s, c) = crate::trig::sin_cos_deg(deg);
    let mut m = identity();
    m[1][1] = c;
    m[1][2] = -s;
    m[2][1] = s;
    m[2][2] = c;
    m
}

fn rot_y(deg: f64) -> Mat4 {
    let (s, c) = crate::trig::sin_cos_deg(deg);
    let mut m = identity();
    m[0][0] = c;
    m[0][2] = s;
    m[2][0] = -s;
    m[2][2] = c;
    m
}

fn rot_z(deg: f64) -> Mat4 {
    let (s, c) = crate::trig::sin_cos_deg(deg);
    let mut m = identity();
    m[0][0] = c;
    m[0][1] = -s;
    m[1][0] = s;
    m[1][1] = c;
    m
}

/// rotate([ax, ay, az]) — the reference's fixed-axis order: about world X
/// first, THEN Y, THEN Z, i.e. the composite matrix Rz * Ry * Rx.
pub fn rotation_xyz(deg: Vec3) -> Mat4 {
    mul(&rot_z(deg[2]), &mul(&rot_y(deg[1]), &rot_x(deg[0])))
}

/// rotate(a, v) — angle-axis about the (normalized) vector v through the
/// origin, right-hand rule. A zero-length v falls back to plain Z rotation
/// per the reference's believed behavior.
/// Scale a direction vector so its largest component is 1, leaving the
/// direction exactly as it was.
///
/// `|v|` and `|v|^2` overflow and underflow long before `v` itself does:
/// `[1e-200, 0, 0]` squares to 0 and `[1e200, 0, 0]` squares to inf, so a
/// perfectly ordinary axis was read as "no axis". The reference is explicit
/// that "magnitude of v is irrelevant", and dividing by the largest
/// component first makes that true across the whole exponent range -- it is
/// an exact operation when the divisor is a power of two and at worst one
/// rounding otherwise, and every component stays in [-1, 1].
fn unit_scaled(v: Vec3) -> Option<Vec3> {
    let m = v.iter().fold(0.0f64, |a, c| a.max(c.abs()));
    if !(m > 0.0) || !m.is_finite() {
        return None;
    }
    Some([v[0] / m, v[1] / m, v[2] / m])
}

pub fn rotation_axis(deg: f64, v: Vec3) -> Mat4 {
    // Bring the vector into range BEFORE squaring it (see `unit_scaled`):
    // `rotate(90, [1e-200, 0, 0])` used to rotate about Z, and
    // `rotate(90, [1e200, 0, 0])` collapsed every vertex onto the origin.
    let Some(v) = unit_scaled(v) else { return rot_z(deg) };
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len == 0.0 {
        return rot_z(deg);
    }
    let (x, y, z) = (v[0] / len, v[1] / len, v[2] / len);
    let (s, c) = crate::trig::sin_cos_deg(deg);
    let t = 1.0 - c;
    [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y, 0.0],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x, 0.0],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// mirror(v): Householder reflection M = I - 2·n·nᵀ/|n|² across the plane
/// through the origin with normal v. A zero/non-finite normal cannot be
/// normalized and yields identity (the caller warns), so children pass
/// through unreflected. Determinant is -1, so apply() rewinds faces.
pub fn mirror(v: Vec3) -> Mat4 {
    // Same reason as `rotation_axis`: `mirror([1e-200, 0, 0])` squared to
    // zero and passed its children through unreflected, in silence.
    let Some(v) = unit_scaled(v) else { return identity() };
    let n2 = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if !(n2 > 0.0) || !n2.is_finite() {
        return identity();
    }
    let mut m = identity();
    for i in 0..3 {
        for j in 0..3 {
            let kron = if i == j { 1.0 } else { 0.0 };
            m[i][j] = kron - 2.0 * v[i] * v[j] / n2;
        }
    }
    m
}

/// multmatrix(m): build the affine matrix from row-major nested lists.
/// Only the top three rows matter (column-vector convention, translation
/// in the last column); a supplied 4th row is ignored (no projective
/// divide), and missing/short rows are filled from the identity.
pub fn matrix_from_rows(rows: &[Vec<f64>]) -> Mat4 {
    let mut m = identity();
    for (i, row) in rows.iter().enumerate().take(3) {
        for (j, &val) in row.iter().enumerate().take(4) {
            m[i][j] = val;
        }
    }
    m
}

/// polyhedron(points, faces): build the mesh exactly as given — no vertex
/// merging or validation beyond index checks. Faces with >3 vertices are
/// fan-triangulated. The reference winds faces CW-from-outside; our meshes
/// are CCW-from-outside, so each fan triangle is reversed. Returns the
/// mesh plus any per-face warnings.
pub fn polyhedron(points: &[Vec3], faces: &[Vec<usize>]) -> (Mesh, Vec<String>) {
    let mut warnings = Vec::new();
    let mut tris = Vec::new();
    for face in faces {
        if face.len() < 3 {
            warnings.push("polyhedron: face with fewer than 3 vertices ignored".into());
            continue;
        }
        if let Some(&bad) = face.iter().find(|&&i| i >= points.len()) {
            warnings.push(format!("polyhedron: point index {} out of bounds; face dropped", bad));
            continue;
        }
        for k in 1..face.len() - 1 {
            // Reversed fan (face[0], face[k+1], face[k]) → CCW outward.
            tris.push([face[0] as u32, face[k + 1] as u32, face[k] as u32]);
        }
    }
    // Points no face refers to are legal and "silently ignored" per the
    // reference — but kept in `positions` they are not ignored at all:
    // positions is what bounds(), hull() and the boolean kernels' framing
    // all measure. One stray point made resize([10,10,10]) scale a unit
    // tetrahedron by 0.1 instead of 10.
    let used: Vec<bool> = {
        let mut u = vec![false; points.len()];
        for t in &tris {
            for &i in t {
                u[i as usize] = true;
            }
        }
        u
    };
    if used.iter().any(|u| !u) {
        let mut remap = vec![u32::MAX; points.len()];
        let mut kept: Vec<Vec3> = Vec::with_capacity(points.len());
        for (i, p) in points.iter().enumerate() {
            if used[i] {
                remap[i] = kept.len() as u32;
                kept.push(*p);
            }
        }
        for t in &mut tris {
            for i in t.iter_mut() {
                *i = remap[*i as usize];
            }
        }
        return (Mesh { positions: kept, tris }, warnings);
    }
    (Mesh { positions: points.to_vec(), tris }, warnings)
}

/// Axis-aligned bounding box of a mesh's vertices (None if empty).
pub fn bounds(mesh: &Mesh) -> Option<([f64; 3], [f64; 3])> {
    if mesh.positions.is_empty() {
        return None;
    }
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in &mesh.positions {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    Some((lo, hi))
}

pub fn apply(m: &Mat4, mesh: &mut Mesh) {
    for p in &mut mesh.positions {
        let (x, y, z) = (p[0], p[1], p[2]);
        *p = [
            m[0][0] * x + m[0][1] * y + m[0][2] * z + m[0][3],
            m[1][0] * x + m[1][1] * y + m[1][2] * z + m[1][3],
            m[2][0] * x + m[2][1] * y + m[2][2] * z + m[2][3],
        ];
    }
    // A negative-determinant transform (mirror, negative scale) flips
    // face orientation — rewind so outward stays outward.
    if det3(m) < 0.0 {
        for t in &mut mesh.tris {
            t.swap(1, 2);
        }
    }
}

fn det3(m: &Mat4) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

// -- Primitives -------------------------------------------------------------

/// cube(size, center): corner at the origin extending +X/+Y/+Z, or centered
/// on the origin in all three axes. Zero/negative components yield empty
/// geometry per the reference.
pub fn cube(size: Vec3, center: bool) -> Mesh {
    // `!(s > 0.0)` rejects 0, negative and NaN but ACCEPTS +inf, so
    // `cube(1/0)` built a mesh whose vertices were infinite and wrote them
    // into the STL with no diagnostic. The reference is explicit: "A size
    // component that is 0, negative, NaN, or inf yields empty geometry."
    if size.iter().any(|&s| !(s > 0.0) || !s.is_finite()) {
        return Mesh::empty();
    }
    let (o, e) = if center {
        ([-size[0] / 2.0, -size[1] / 2.0, -size[2] / 2.0], [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0])
    } else {
        ([0.0, 0.0, 0.0], size)
    };
    let p = |x: bool, y: bool, z: bool| -> Vec3 {
        [
            if x { e[0] } else { o[0] },
            if y { e[1] } else { o[1] },
            if z { e[2] } else { o[2] },
        ]
    };
    let positions = vec![
        p(false, false, false), // 0
        p(true, false, false),  // 1
        p(true, true, false),   // 2
        p(false, true, false),  // 3
        p(false, false, true),  // 4
        p(true, false, true),   // 5
        p(true, true, true),    // 6
        p(false, true, true),   // 7
    ];
    // Outward-wound quads, split into triangles.
    let quads: [[u32; 4]; 6] = [
        [0, 3, 2, 1], // bottom (z-)
        [4, 5, 6, 7], // top (z+)
        [0, 1, 5, 4], // front (y-)
        [2, 3, 7, 6], // back (y+)
        [1, 2, 6, 5], // right (x+)
        [3, 0, 4, 7], // left (x-)
    ];
    let mut tris = Vec::with_capacity(12);
    for q in quads {
        tris.push([q[0], q[1], q[2]]);
        tris.push([q[0], q[2], q[3]]);
    }
    Mesh { positions, tris }
}

/// sphere(r): rings at half-step polar offsets — ring i (0-based from +Z)
/// at phi = 180*(i+0.5)/R degrees with R = ceil(N/2) rings of N vertices;
/// N-gon caps close the poles. No vertex ever sits at (0,0,±r).
pub fn sphere(r: f64, n: u32) -> Mesh {
    if !(r > 0.0) || !r.is_finite() {
        return Mesh::empty();
    }
    let n = n.max(3) as usize;
    let rings = (n as f64 / 2.0).ceil() as usize;
    let mut positions = Vec::with_capacity(rings * n);
    // DEGREES, through the shared exact trig, exactly as the reference states
    // the layout: "Ring i lies at polar angle phi = 180*(i+0.5)/R degrees" and
    // "Each ring has N vertices at azimuth 360*j/N degrees". Radian trig put
    // 6.1e-17 dust on the axis vertices and, worse, ASYMMETRIC dust — the +Y
    // and -Y vertices of a 4-sided ring differed in magnitude, so the mesh was
    // not symmetric about a plane the shape obviously is.
    for i in 0..rings {
        let (sp, cp) = crate::trig::sin_cos_deg(180.0 * (i as f64 + 0.5) / rings as f64);
        let (rz, z) = (r * sp, r * cp);
        for j in 0..n {
            let (st, ct) = crate::trig::sin_cos_deg(360.0 * j as f64 / n as f64);
            positions.push([rz * ct, rz * st, z]);
        }
    }
    let mut tris = Vec::new();
    let idx = |ring: usize, j: usize| (ring * n + j % n) as u32;
    // Top cap fan over ring 0 (viewed from +Z, ring order is CCW).
    for j in 1..(n - 1) {
        tris.push([idx(0, 0), idx(0, j), idx(0, j + 1)]);
    }
    // Quads between adjacent rings.
    for i in 0..(rings - 1) {
        for j in 0..n {
            let (a, b) = (idx(i, j), idx(i, j + 1));
            let (c, d) = (idx(i + 1, j), idx(i + 1, j + 1));
            tris.push([a, c, d]);
            tris.push([a, d, b]);
        }
    }
    // Bottom cap fan over the last ring, wound the other way.
    let last = rings - 1;
    for j in 1..(n - 1) {
        tris.push([idx(last, 0), idx(last, j + 1), idx(last, j)]);
    }
    Mesh { positions, tris }
}

/// cylinder(h, r1, r2, center): along +Z ([0,h], or [-h/2,h/2] centered).
/// A radius of exactly 0 collapses that end to a single apex vertex.
pub fn cylinder(h: f64, r1: f64, r2: f64, center: bool, n: u32) -> Mesh {
    // The radius tests let NaN through (every comparison with NaN is false)
    // and both radius and height accepted +inf.
    if !(h > 0.0)
        || ![h, r1, r2].iter().all(|v| v.is_finite())
        || !(r1 >= 0.0)
        || !(r2 >= 0.0)
        || (r1 == 0.0 && r2 == 0.0)
    {
        return Mesh::empty();
    }
    let n = n.max(3) as usize;
    let (z0, z1) = if center { (-h / 2.0, h / 2.0) } else { (0.0, h) };
    let mut positions = Vec::new();
    let ring = |positions: &mut Vec<Vec3>, r: f64, z: f64| -> Vec<u32> {
        if r == 0.0 {
            positions.push([0.0, 0.0, z]);
            vec![(positions.len() - 1) as u32]
        } else {
            let base = positions.len() as u32;
            // "Both rings have N vertices at azimuth 360*i/N" — degrees.
            for j in 0..n {
                let (st, ct) = crate::trig::sin_cos_deg(360.0 * j as f64 / n as f64);
                positions.push([r * ct, r * st, z]);
            }
            (base..base + n as u32).collect()
        }
    };
    let bottom = ring(&mut positions, r1, z0);
    let top = ring(&mut positions, r2, z1);
    let mut tris = Vec::new();
    match (bottom.len(), top.len()) {
        (1, _) => {
            // Bottom apex: side fan, then the top cap.
            let apex = bottom[0];
            for j in 0..n {
                tris.push([apex, top[(j + 1) % n] as u32, top[j] as u32]);
            }
            for j in 1..(n - 1) {
                tris.push([top[0], top[j], top[j + 1]]);
            }
        }
        (_, 1) => {
            let apex = top[0];
            for j in 0..n {
                tris.push([apex, bottom[j], bottom[(j + 1) % n]]);
            }
            for j in 1..(n - 1) {
                tris.push([bottom[0], bottom[j + 1], bottom[j]]);
            }
        }
        _ => {
            for j in 0..n {
                let (a, b) = (bottom[j], bottom[(j + 1) % n]);
                let (c, d) = (top[j], top[(j + 1) % n]);
                tris.push([a, b, d]);
                tris.push([a, d, c]);
            }
            for j in 1..(n - 1) {
                tris.push([bottom[0], bottom[j + 1], bottom[j]]); // bottom cap (z-)
                tris.push([top[0], top[j], top[j + 1]]); // top cap (z+)
            }
        }
    }
    Mesh { positions, tris }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 2D fill sweep resolves crossings numerically, so two scanline
    /// crossings can land an ULP apart and the trapezoid between them is a
    /// sliver: real at f64, exactly collinear once written to six
    /// significant digits. Every one of those exported as
    /// `facet normal 0 0 0` -- a facet whose normal the STL format requires
    /// and which no reader can recover from three collinear vertices.
    #[test]
    fn unrepresentable_slivers_leave_the_export() {
        // A unit cube plus one sliver whose two far corners are an ULP apart
        // -- the shape the 2D sweep actually produces.
        let mut m = cube([1.0, 1.0, 1.0], false);
        let n = m.positions.len() as u32;
        let x = 0.5_f64;
        m.positions.push([0.25, 0.25, 0.0]);
        m.positions.push([x, 0.25, 0.0]);
        m.positions.push([f64::from_bits(x.to_bits() + 1), 0.25, 0.0]);
        m.tris.push([n, n + 1, n + 2]);
        assert_eq!(m.tris.len(), 13);

        let clean = m.without_unrepresentable();
        assert_eq!(clean.tris.len(), 12, "the sliver is gone and the cube is not");
        // Positions are untouched -- this drops faces, it does not move or
        // renumber anything.
        assert_eq!(clean.positions, m.positions);

        // Thin but REAL at f64, and still flattened by the writers' rounding:
        // a 1e-9 rise over a 0.5 run has |cross| = 5e-10, comfortably above
        // any bound on the unrounded numbers, yet both corners print the same
        // y at `{:.6}`. Testing the unrounded triangle kept these, and they
        // reappeared as collinear facets in the file.
        let mut thin = cube([1.0, 1.0, 1.0], false);
        let n = thin.positions.len() as u32;
        thin.positions.push([0.25, 0.25, 0.0]);
        thin.positions.push([0.75, 0.25, 0.0]);
        thin.positions.push([0.5, 0.25 + 1e-9, 0.0]);
        thin.tris.push([n, n + 1, n + 2]);
        assert_eq!(thin.without_unrepresentable().tris.len(), 12, "flattened by rounding");

        // A triangle that is small but REPRESENTABLE survives: the writers
        // resolve 1e-6, and 1e-3 is a thousand times that.
        let mut ok = cube([1.0, 1.0, 1.0], false);
        let n = ok.positions.len() as u32;
        ok.positions.push([0.25, 0.25, 0.0]);
        ok.positions.push([0.25 + 1e-3, 0.25, 0.0]);
        ok.positions.push([0.25, 0.25 + 1e-3, 0.0]);
        ok.tris.push([n, n + 1, n + 2]);
        assert_eq!(ok.without_unrepresentable().tris.len(), 13, "a real small face is kept");

        // A triangle with a repeated vertex has no area at all.
        let mut dup = cube([1.0, 1.0, 1.0], false);
        dup.tris.push([0, 1, 1]);
        assert_eq!(dup.without_unrepresentable().tris.len(), 12);

        // An empty mesh has no bounds; it must come back empty, not panic.
        assert!(Mesh::empty().without_unrepresentable().tris.is_empty());
    }

    #[test]
    fn fragment_formula_matches_the_reference_cases() {
        // sphere() defaults: r=1, $fa=12, $fs=2 → ceil(max(min(30, π), 5)) = 5.
        assert_eq!(fragments(1.0, 0.0, 12.0, 2.0), 5);
        // sphere(10): min(30, 31.4) = 30.
        assert_eq!(fragments(10.0, 0.0, 12.0, 2.0), 30);
        // $fn wins outright, floored at 3.
        assert_eq!(fragments(10.0, 6.0, 12.0, 2.0), 6);
        assert_eq!(fragments(10.0, 1.0, 12.0, 2.0), 3);
        // Sub-GRID_FINE radius short-circuits to 3 regardless of $fn.
        assert_eq!(fragments(1e-7, 64.0, 12.0, 2.0), 3);
    }

    #[test]
    fn cube_extents_follow_center() {
        let m = cube([2.0, 4.0, 6.0], false);
        assert_eq!(m.positions.len(), 8);
        assert_eq!(m.tris.len(), 12);
        assert!(m.positions.iter().all(|p| p[0] >= 0.0 && p[1] >= 0.0 && p[2] >= 0.0));
        let c = cube([2.0, 4.0, 6.0], true);
        assert!(c.positions.iter().any(|p| p[0] == -1.0));
        assert!(c.positions.iter().any(|p| p[2] == 3.0));
        // Zero/negative sizes yield empty geometry, not a degenerate mesh.
        assert_eq!(cube([0.0, 1.0, 1.0], false), Mesh::empty());
        assert_eq!(cube([-1.0, 1.0, 1.0], false), Mesh::empty());
    }

    #[test]
    fn sphere_has_no_pole_vertices_and_half_step_rings() {
        // $fn=6 → 6 meridians, ceil(6/2)=3 rings — the reference's
        // hexagonal 'gem', not a bipyramid.
        let m = sphere(1.0, 6);
        assert_eq!(m.positions.len(), 18);
        for p in &m.positions {
            assert!(
                (p[0].abs() > 1e-12) || (p[1].abs() > 1e-12),
                "pole vertex found at {:?}",
                p
            );
        }
        // All vertices on the sphere surface.
        for p in &m.positions {
            let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn cylinder_apex_collapse_and_centering() {
        // r2=0: a true cone with a single apex vertex, not a zero ring.
        let cone = cylinder(2.0, 3.0, 0.0, false, 8);
        assert_eq!(cone.positions.len(), 9);
        assert!(cone.positions.iter().any(|p| *p == [0.0, 0.0, 2.0]));
        let cyl = cylinder(2.0, 1.0, 1.0, true, 8);
        assert!(cyl.positions.iter().all(|p| p[2] == -1.0 || p[2] == 1.0));
        assert_eq!(cylinder(1.0, 0.0, 0.0, false, 8), Mesh::empty());
    }

    #[test]
    fn rotation_order_is_z_after_y_after_x() {
        // rotate([90, 0, 90]) applied to +X: Rx does nothing to +X,
        // then... order is Rz*Ry*Rx so +X → Rz(90) → +Y.
        let m = rotation_xyz([90.0, 0.0, 90.0]);
        let mut mesh = Mesh { positions: vec![[1.0, 0.0, 0.0]], tris: vec![] };
        apply(&m, &mut mesh);
        let p = mesh.positions[0];
        assert!((p[0]).abs() < 1e-9 && (p[1] - 1.0).abs() < 1e-9 && p[2].abs() < 1e-9);
        // And +Y → Rx(90) → +Z, unaffected by Rz. Composite must differ
        // from the X-last order.
        let mut mesh = Mesh { positions: vec![[0.0, 1.0, 0.0]], tris: vec![] };
        apply(&m, &mut mesh);
        let p = mesh.positions[0];
        assert!(p[0].abs() < 1e-9 && p[1].abs() < 1e-9 && (p[2] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn mirror_transforms_rewind_triangles() {
        let mut mesh = cube([1.0, 1.0, 1.0], true);
        let before = mesh.tris.clone();
        apply(&scaling([-1.0, 1.0, 1.0]), &mut mesh);
        assert_ne!(mesh.tris, before, "negative determinant must flip winding");
    }
}

#[cfg(test)]
mod exactness_tests {
    use super::*;

    #[test]
    fn primitive_rings_land_on_exact_degrees() {
        // The reference gives every ring in DEGREES — circle "Vertex i at
        // (r*cos(360*i/N), r*sin(360*i/N)) ... vertex 0 is exactly (r, 0)",
        // sphere "polar angle 180*(i+0.5)/R degrees" and "azimuth 360*j/N
        // degrees", cylinder "azimuth 360*i/N" — and the whole point of
        // trig.rs is that those are exact. Radian trig put 6.1e-17 dust on
        // the axis vertices, and ASYMMETRIC dust at that: the +Y and -Y
        // vertices of a 4-sided ring differed in magnitude, so the mesh was
        // not symmetric about a plane the shape plainly is.
        let c = cylinder(1.0, 1.0, 1.0, false, 4);
        assert_eq!(&c.positions[0..4], &[
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
        ]);
        // Three rings at 30/90/150 degrees: sin(30) is exactly 0.5.
        let s = sphere(1.0, 6);
        assert_eq!(s.positions[0][0], 0.5);
        assert_eq!(s.positions[0][1], 0.0);
        // A hexagon's second vertex is (cos 60, sin 60) = (0.5, sqrt(3)/2).
        let h = crate::poly2::circle(1.0, 6);
        assert_eq!(h.contours[0][0], [1.0, 0.0]);
        assert_eq!(h.contours[0][1][0], 0.5);
        // ...and a square's vertices are exactly on the axes.
        assert_eq!(crate::poly2::circle(2.0, 4).contours[0], vec![
            [2.0, 0.0],
            [0.0, 2.0],
            [-2.0, 0.0],
            [0.0, -2.0],
        ]);
    }

    #[test]
    fn the_fa_fs_branch_reports_what_it_wanted() {
        // MAX_FRAGMENTS' doc comment promises "the clamp is never silent",
        // but the warning only ever tested the $fn branch.
        assert_eq!(fragments(100.0, 0.0, 0.1, 0.1), MAX_FRAGMENTS);
        assert_eq!(uncapped_fragments(100.0, 0.0, 0.1, 0.1), 3600.0);
        // The $fn branch and a degenerate radius are not this path.
        assert_eq!(uncapped_fragments(100.0, 64.0, 0.1, 0.1), 0.0);
        assert_eq!(uncapped_fragments(0.0, 0.0, 0.1, 0.1), 0.0);
        // An ordinary radius does not trip the cap.
        assert!(uncapped_fragments(10.0, 0.0, 12.0, 2.0) <= MAX_FRAGMENTS as f64);
    }
}
