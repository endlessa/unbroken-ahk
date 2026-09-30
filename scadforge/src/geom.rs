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
    /// The test is the writers' own resolution, not a magic number, and it is
    /// the RESOLUTION OF THE FORMAT BEING WRITTEN. The text formats print
    /// coordinates with `{:.6}`, so they land on a 1e-6 grid; binary STL
    /// stores f32, whose spacing near a coordinate is about `|x| *
    /// f32::EPSILON` -- so its step is per-axis, read off each axis's own
    /// coordinates. On a grid of spacing `res` the thinnest triangle that is
    /// still a triangle has `|cross| == res * res`, so anything below that is
    /// collinear as written.
    ///
    /// Applying f32's answer to all of them was tidy and wrong. f32's step at
    /// a coordinate of 1e6 is 0.119, so a millimetre-sized face a kilometre
    /// from the origin has all three corners land on one f32 point and cannot
    /// be written to a binary STL at all -- but OFF, AMF, 3MF and ASCII STL
    /// print it exactly, and were losing it for a limitation none of them
    /// has. Dropping what the chosen format cannot carry removes no surface
    /// -- volume and area over the whole corpus are unchanged to twelve
    /// significant digits -- and it happens ONCE per export, on the merged
    /// mesh, so a format never disagrees with itself.
    /// The volume the mesh encloses, SIGNED by its orientation: positive
    /// when the faces are wound so the solid is on the inside of them,
    /// negative when the mesh is inside-out.
    ///
    /// The divergence theorem over a closed surface, which costs one pass
    /// and needs no adjacency, so it is cheap enough to ask at an export.
    /// It answers two questions that are otherwise expensive: whether a
    /// mesh is inside-out, and whether a boolean lost something.
    pub fn signed_volume(&self) -> f64 {
        let mut v = 0.0;
        for t in &self.tris {
            let a = self.positions[t[0] as usize];
            let b = self.positions[t[1] as usize];
            let c = self.positions[t[2] as usize];
            v += (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0;
        }
        v
    }

    /// Undirected edges used an odd number of times IN THE FILE the chosen
    /// format will write — coordinates moved onto that format's grid, points
    /// that land together treated as one, and every triangle kept.
    ///
    /// Asking this of the mesh as HELD gives a different answer, and not
    /// always in the direction you would guess. The rounding that collapses
    /// a triangle also brings its neighbours' corners together, and can
    /// close the gap the collapse opened: a cycloidal gear drops 48
    /// triangles and the f64 mesh left behind has 62 boundary edges, while
    /// the binary STL it writes is closed.
    ///
    /// Degenerate triangles are COUNTED, not dropped, because the file keeps
    /// them: a facet whose two corners coincide contributes a self-loop
    /// edge, and any reader tallying edges sees it.
    pub fn open_edges_as_written(&self, grid: Grid) -> usize {
        use std::collections::HashMap;
        let snap = |v: f64| match grid {
            Grid::Text => (v * 1e6).round() / 1e6,
            Grid::Binary => v as f32 as f64,
        };
        let key = |p: &Vec3| {
            let b = |v: f64| {
                let v = snap(v);
                if v == 0.0 { 0f64.to_bits() } else { v.to_bits() }
            };
            [b(p[0]), b(p[1]), b(p[2])]
        };
        let mut at: HashMap<[u64; 3], u32> = HashMap::new();
        let ids: Vec<u32> = self
            .positions
            .iter()
            .map(|p| {
                let n = at.len() as u32;
                *at.entry(key(p)).or_insert(n)
            })
            .collect();
        let mut use_count: HashMap<(u32, u32), usize> = HashMap::new();
        for t in &self.tris {
            for k in 0..3 {
                let (a, b) = (ids[t[k] as usize], ids[t[(k + 1) % 3] as usize]);
                *use_count.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        use_count.values().filter(|c| *c % 2 == 1).count()
    }

    pub fn without_unrepresentable(&self, grid: Grid) -> Mesh {
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
            // The text writers print `{:.6}`, landing on a 1e-6 grid; binary
            // STL stores f32. The vertices are SNAPPED before the test, not
            // just compared against the grid: rounding moves each corner by
            // up to half a step, which is itself enough to flatten a triangle
            // that was thin but real beforehand.
            let text = cross_on_grid(pts, |x| (x * 1e6).round() / 1e6);
            let binary = cross_on_grid(pts, |x| x as f32 as f64);
            // f32's grid is not uniform the way the text one is -- its step
            // is proportional to the magnitude it is near -- so the step has
            // to be read off EACH AXIS separately. It used to come from the
            // single largest coordinate of the triangle, over all three axes
            // at once, which judged a triangle by an axis it is not thin
            // along: a 0.1 x 0.1 end cap lifted to z = 1e6 was measured
            // against z's step of 0.119 and deleted, though its own corners
            // sit on x and y steps of 1.2e-8 and it is written out exactly by
            // every format. A 0.1 x 0.1 x 100 post 1e6 from the origin came
            // back as an open tube with both caps missing and a third of its
            // volume gone, and a sphere far enough out vanished entirely --
            // reported, untruthfully, as "Current top level object is empty".
            //
            // A cross component pairs two axes, so it lands on a grid of
            // their two steps multiplied; the thinnest triangle that is still
            // a triangle is the smallest of those three products. Comparing
            // against that keeps the guard doing its real job -- dropping a
            // triangle whose corners come out exactly collinear once written,
            // which no reader can assign a normal to -- and makes it
            // independent of where the model sits, which is what it always
            // claimed to be.
            let step = |axis: usize| {
                let m = pts.iter().fold(0.0f64, |m, p| m.max(p[axis].abs()));
                (m * f32::EPSILON as f64).max(f32::MIN_POSITIVE as f64)
            };
            let (sx, sy, sz) = (step(0), step(1), step(2));
            let floor = (sx * sy).min(sy * sz).min(sz * sx);
            // And the writers' own condition, which is about the triangle as
            // the evaluator built it rather than as any grid renders it:
            // `triangle_normal` gives up below |cross| = 1e-12 and writes
            // `0 0 0`. The old magnitude-scaled floor happened to catch those
            // too, so making that floor honest meant stating this separately.
            let raw = cross_on_grid(pts, |x| x);
            let representable =
                raw.is_finite() && text.is_finite() && raw >= 1e-12 && text >= 1e-12;
            match grid {
                Grid::Text => representable,
                // The same 1e-12 applies to the f32 form, because that is
                // the rule the READER applies to it: `triangle_normal` gives
                // up below it and `read_stl` drops such a triangle on the way
                // back in. Writing one the reader would refuse is a
                // disagreement between two halves of this crate, whether or
                // not any file has hit it yet.
                Grid::Binary => {
                    representable && binary.is_finite() && binary >= 1e-12 && binary >= floor
                }
            }
        };
        Mesh {
            positions: self.positions.clone(),
            tris: self.tris.iter().copied().filter(|t| keep(t)).collect(),
        }
    }
}

/// The precision an export format writes coordinates at.
///
/// The only thing that distinguishes one format from another as far as
/// geometry goes, and the only reason a triangle may have to be dropped.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grid {
    /// `{:.6}` decimal: OFF, AMF, 3MF and ASCII STL.
    Text,
    /// f32: binary STL.
    Binary,
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

/// Triangulate one face of a polyhedron.
///
/// The reference says ">3-vertex faces are fan/ear triangulated internally"
/// and marks the exact choice as one to pin against an oracle -- which this
/// project may not read. So the choice is made on the merits, and a blind
/// fan from vertex 0 loses on two faces that turn up constantly.
///
/// A CONCAVE face fanned from vertex 0 puts triangles outside itself. An
/// L-shaped face is the smallest example: three of its five fan triangles
/// cover ground the face does not.
///
/// A face with a COLLINEAR VERTEX AT THE FAN APEX produces a zero-area
/// triangle, and that one is worse, because the mesh looks fine until
/// export. Take a quad A,B,C,D with A, B, C in a line: the fan is (A,B,C),
/// which has no area, plus (A,C,D). The degenerate one is dropped at the
/// export funnel -- no format can write a facet whose three corners are
/// collinear, and this kernel's own STL reader would refuse it -- and with
/// it go edges A-B and B-C, which the neighbouring faces still use. Two
/// boundary edges in a mesh that was closed when it was built. Ear clipping
/// triangulates the same quad as (D,A,B) + (D,B,C), which keeps B on the
/// boundary and has no degenerate triangle to lose.
///
/// The ear test is the standard one, in the face's own plane by Newell's
/// normal, which is defined for a non-planar face too: an ear is a convex
/// corner no other vertex of the face lies inside. Ears with area are taken
/// first, so a degenerate ear is cut only when the face leaves no choice.
fn face_tris(points: &[Vec3], face: &[usize]) -> Vec<[usize; 3]> {
    face_tris_in(points, face, None)
}

/// As `face_tris`, but with the face's plane supplied.
///
/// Newell's normal is the right answer when nothing better is known, and
/// the wrong one when something is: a triangle that has had points inserted
/// along its edges lies in ITS OWN plane, and for a sliver that plane is
/// known far more accurately than the area-weighted normal of the polygon,
/// whose terms nearly cancel. Letting it be re-derived flipped the
/// projection on two faces of a character model and wound their pieces
/// backwards.
fn face_tris_in(points: &[Vec3], face: &[usize], plane: Option<Vec3>) -> Vec<[usize; 3]> {
    let fan = || (1..face.len() - 1).map(|k| [face[0], face[k], face[k + 1]]).collect::<Vec<_>>();
    if face.len() == 3 {
        return vec![[face[0], face[1], face[2]]];
    }
    // Newell: the area-weighted normal, which is the face's own plane even
    // when the face does not have one, and which no single corner's cross
    // product can be trusted to give (any corner may be collinear).
    let mut n = plane.unwrap_or([0.0; 3]);
    if plane.is_none() {
        for i in 0..face.len() {
            let a = points[face[i]];
            let b = points[face[(i + 1) % face.len()]];
            n[0] += (a[1] - b[1]) * (a[2] + b[2]);
            n[1] += (a[2] - b[2]) * (a[0] + b[0]);
            n[2] += (a[0] - b[0]) * (a[1] + b[1]);
        }
    }
    let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if !(nl > 0.0) {
        return fan(); // the whole face is degenerate; a fan is as good as anything
    }
    let n = [n[0] / nl, n[1] / nl, n[2] / nl];
    // An in-plane basis, taken from whichever axis is least aligned with the
    // normal so the cross product is well conditioned.
    let k = (0..3).fold(0, |m, i| if n[i].abs() < n[m].abs() { i } else { m });
    let mut ax = [0.0; 3];
    ax[k] = 1.0;
    let u = {
        let c = [
            n[1] * ax[2] - n[2] * ax[1],
            n[2] * ax[0] - n[0] * ax[2],
            n[0] * ax[1] - n[1] * ax[0],
        ];
        let l = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
        [c[0] / l, c[1] / l, c[2] / l]
    };
    let v = [
        n[1] * u[2] - n[2] * u[1],
        n[2] * u[0] - n[0] * u[2],
        n[0] * u[1] - n[1] * u[0],
    ];
    // u x v = n, so the loop comes out counter-clockwise in (u, v).
    let p2: Vec<[f64; 2]> = face
        .iter()
        .map(|&i| {
            let p = points[i];
            [
                p[0] * u[0] + p[1] * u[1] + p[2] * u[2],
                p[0] * v[0] + p[1] * v[1] + p[2] * v[2],
            ]
        })
        .collect();
    let cross = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    // A tolerance on the face's own scale, so "has area" means the same
    // thing on a 1 mm face and a 1 km one.
    let span = p2.iter().fold(0.0f64, |m, q| {
        m.max((q[0] - p2[0][0]).abs().max((q[1] - p2[0][1]).abs()))
    });
    let eps = (span * span * 1e-12).max(f64::MIN_POSITIVE);

    let mut loopv: Vec<usize> = (0..face.len()).collect();
    let mut out = Vec::with_capacity(face.len() - 2);
    let mut guard = 0usize;
    while loopv.len() > 3 {
        guard += 1;
        if guard > face.len() * face.len() + 8 {
            // Self-intersecting or otherwise unclippable: no triangulation
            // is right, so fall back rather than spin.
            return fan();
        }
        let m = loopv.len();
        // How many corners of a loop are flat -- the count a clip is chosen
        // to drive down, because a flat corner is a triangle that will be
        // dropped by the writer and take two of the mesh's edges with it.
        let flats = |l: &[usize]| {
            let n = l.len();
            (0..n)
                .filter(|&j| {
                    cross(p2[l[(j + n - 1) % n]], p2[l[j]], p2[l[(j + 1) % n]]).abs() <= eps
                })
                .count()
        };
        // (flats left behind, -area) — smaller is better, so an ear that
        // leaves no flat corner beats a bigger one that does.
        let mut best: Option<((usize, f64), usize)> = None;
        for j in 0..m {
            let (a, b, c) = (loopv[(j + m - 1) % m], loopv[j], loopv[(j + 1) % m]);
            let area = cross(p2[a], p2[b], p2[c]);
            if area < 0.0 {
                continue; // a reflex corner is never an ear
            }
            // No other remaining vertex may lie inside the candidate ear.
            let clear = loopv.iter().all(|&x| {
                x == a
                    || x == b
                    || x == c
                    || !(cross(p2[a], p2[b], p2[x]) > eps
                        && cross(p2[b], p2[c], p2[x]) > eps
                        && cross(p2[c], p2[a], p2[x]) > eps)
            });
            if !clear {
                continue;
            }
            // A clip that removes a flat corner emits the degenerate
            // triangle itself; one that leaves a flat corner behind only
            // postpones it, and the last three vertices are emitted with no
            // choice at all. Looking one step ahead is enough to tell the
            // two apart, and faces are small.
            let mut rest = loopv.clone();
            rest.remove(j);
            let cost = (
                if area <= eps { usize::MAX } else { flats(&rest) },
                -area,
            );
            if best.as_ref().is_none_or(|&(w, _)| cost < w) {
                best = Some((cost, j));
            }
        }
        // If nothing qualified, the face is not simple and the fan is the
        // honest fallback.
        let Some((_, j)) = best else { return fan() };
        let (a, b, c) = (loopv[(j + m - 1) % m], loopv[j], loopv[(j + 1) % m]);
        out.push([face[a], face[b], face[c]]);
        loopv.remove(j);
    }
    out.push([face[loopv[0]], face[loopv[1]], face[loopv[2]]]);
    out
}

/// polyhedron(points, faces): build the mesh exactly as given — no vertex
/// merging or validation beyond index checks. Faces with >3 vertices are ear
/// triangulated by `face_tris`. The reference winds faces CW-from-outside;
/// our meshes are CCW-from-outside, so each triangle is reversed. Returns
/// the mesh plus any per-face warnings.
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
        for t in face_tris(points, face) {
            // Reversed → CCW outward.
            tris.push([t[0] as u32, t[2] as u32, t[1] as u32]);
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
        let mesh = Mesh { positions: kept, tris };
        warnings.extend(closedness_note(&mesh));
        return (mesh, warnings);
    }
    let mesh = Mesh { positions: points.to_vec(), tris };
    warnings.extend(closedness_note(&mesh));
    (mesh, warnings)
}

/// Split every triangle edge that another vertex lands in the middle of.
///
/// A BSP cuts one side's polygons against the other side's planes, so a
/// vertex of one surface routinely comes to rest partway along an edge of
/// the other. The surface stays continuous and the volume stays right; the
/// TOPOLOGY does not. That edge is used once by the triangle that owns it
/// and twice by the pair on the other side, so the undirected edge is used
/// an odd number of times and reads as a hole. Two cubes overlapping by a
/// quarter union to exactly 5,000 mm^3 and twelve boundary edges; two
/// spheres to the right volume and 15,919 of them.
///
/// It is a real defect and not a cosmetic one. Every downstream test of a
/// mesh -- is it closed, is it manifold, can it be printed -- fails on a
/// T-junction, and so does the next boolean, because a surface it cannot
/// walk is a surface it cannot classify. It also meant no union of solids
/// that actually MEET could ever come out clean, which is a large part of
/// what the modeller is for.
///
/// The repair adds no geometry: a vertex already on an edge is inserted
/// into the triangle that owns that edge, and the triangle is re-cut around
/// it. Nothing moves, so volume and area are unchanged.
pub fn weld_tjunctions(mesh: &Mesh) -> Mesh {
    use std::collections::HashMap;
    // Coincident points must share an index first, or a "T-junction" that is
    // really two copies of the same point is never seen.
    let mesh = mesh.welded();
    let Some((lo, hi)) = bounds(&mesh) else { return mesh };
    let extent = (0..3).fold(0.0f64, |m, k| m.max(hi[k] - lo[k]));
    if !(extent > 0.0) {
        return mesh;
    }
    // A point this far off an edge is on it as far as any writer is
    // concerned, and inserting it moves the surface by no more than that.
    // A point this far off an edge is on it as far as any writer is
    // concerned, and inserting it moves the surface by no more than that.
    //
    // Tight on purpose, and swept to find out: at 1e-7 a character model
    // went from 1,459 boundary edges to 1,500 and grew its first
    // inconsistently wound edge; at 1e-4 it went to 6,565 and 75. Loosening
    // does not catch more T-junctions, it MANUFACTURES them -- a vertex
    // inserted into an edge it is not really on moves that edge, and the
    // triangle on the other side, which did not get the same insertion, no
    // longer matches. The residue at 1e-9 is not a T-junction problem at
    // all: those vertices sit 1e-5 to 1e-4 of the model off the edge, which
    // is a crack in the BSP's arithmetic and wants fixing there.
    let eps = extent * 1e-9;

    // Vertices on a grid, so an edge asks only its own neighbourhood.
    let n = ((mesh.positions.len() as f64).cbrt().ceil() as i64).clamp(4, 128);
    let step: Vec<f64> = (0..3).map(|k| ((hi[k] - lo[k]) / n as f64).max(1e-300)).collect();
    let cell = |v: f64, k: usize| (((v - lo[k]) / step[k]).floor() as i64).clamp(0, n - 1);
    let mut grid: HashMap<(i64, i64, i64), Vec<u32>> = HashMap::new();
    for (i, p) in mesh.positions.iter().enumerate() {
        grid.entry((cell(p[0], 0), cell(p[1], 1), cell(p[2], 2))).or_default().push(i as u32);
    }

    let sub = |a: Vec3, b: Vec3| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: Vec3, b: Vec3| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

    let mut positions = mesh.positions.clone();
    let mut out: Vec<[u32; 3]> = Vec::with_capacity(mesh.tris.len());
    let mut splits: Vec<Vec<(f64, u32)>> = vec![Vec::new(); 3];
    for t in &mesh.tris {
        for e in 0..3 {
            splits[e].clear();
            let (ia, ib) = (t[e], t[(e + 1) % 3]);
            let (a, b) = (mesh.positions[ia as usize], mesh.positions[ib as usize]);
            let ab = sub(b, a);
            let len2 = dot(ab, ab);
            if len2 <= 0.0 {
                continue;
            }
            // Endpoints are excluded by the same tolerance, measured along
            // the edge, so a point at a corner is never "in the middle".
            let m = eps / len2.sqrt();
            let (mut c0, mut c1) = ([0i64; 3], [0i64; 3]);
            for k in 0..3 {
                let (u, v) = (a[k].min(b[k]) - eps, a[k].max(b[k]) + eps);
                c0[k] = cell(u, k);
                c1[k] = cell(v, k);
            }
            // A long edge crossing the whole model would ask every cell;
            // giving up on it leaves that one edge unwelded, which is the
            // safe direction.
            let span = (c1[0] - c0[0] + 1) * (c1[1] - c0[1] + 1) * (c1[2] - c0[2] + 1);
            if span > 8192 {
                continue;
            }
            for x in c0[0]..=c1[0] {
                for y in c0[1]..=c1[1] {
                    for z in c0[2]..=c1[2] {
                        let Some(list) = grid.get(&(x, y, z)) else { continue };
                        for &k in list {
                            if k == ia || k == ib || k == t[(e + 2) % 3] {
                                continue;
                            }
                            let ak = sub(mesh.positions[k as usize], a);
                            let u = dot(ak, ab) / len2;
                            if u <= m || u >= 1.0 - m {
                                continue;
                            }
                            let perp =
                                [ak[0] - ab[0] * u, ak[1] - ab[1] * u, ak[2] - ab[2] * u];
                            if dot(perp, perp) > eps * eps {
                                continue;
                            }
                            splits[e].push((u, k));
                        }
                    }
                }
            }
            splits[e].sort_by(|p, q| p.0.total_cmp(&q.0));
            splits[e].dedup_by_key(|p| p.1);
        }
        if splits.iter().all(|s| s.is_empty()) {
            out.push(*t);
            continue;
        }
        // The triangle becomes a polygon: each corner, then whatever landed
        // on the edge leaving it, in order along that edge.
        let mut face: Vec<usize> = Vec::with_capacity(3 + splits.iter().map(Vec::len).sum::<usize>());
        for e in 0..3 {
            face.push(t[e] as usize);
            face.extend(splits[e].iter().map(|&(_, k)| k as usize));
        }
        // Ear clipping in the face's own plane, which is what handles the
        // collinear corners the inserted points create. The loop order is
        // the triangle's own, so the pieces keep its winding.
        // The triangle's OWN normal, not one re-derived from the polygon:
        // the inserted points are on its edges, so the plane is unchanged,
        // and for a sliver the polygon's Newell sum is mostly cancellation.
        let (p0, p1, p2) = (
            mesh.positions[t[0] as usize],
            mesh.positions[t[1] as usize],
            mesh.positions[t[2] as usize],
        );
        let (u, v) = (sub(p1, p0), sub(p2, p0));
        let plane = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        // A triangle with no area has no orientation, so there is nothing to
        // check a re-cut against: `dot(piece, plane) < 0` is false whatever
        // the piece does, the guard below is vacuous, and whatever the ear
        // clip decided stands. That is the gap the third tier could not
        // close on its own -- a lattice hull kept two inconsistently wound
        // edges through all three tiers.
        //
        // Such a triangle contributes no surface and will be dropped by the
        // writer anyway, so splitting it can only invent an orientation
        // nobody asked for. It goes through unchanged.
        if dot(plane, plane) <= 0.0 {
            out.push(*t);
            continue;
        }
        // Fan the polygon from the triangle's own centroid.
        //
        // Ear clipping was tried here first, in two planes, with a check
        // that re-cut again if any piece disagreed with the triangle's
        // normal. It was delicate and it was not sound: on a sliver of area
        // 1e-09 in a 160-unit model, the check is below its own noise floor
        // -- the piece's normal is 1e-09 too, so `dot(piece, plane) > 0` can
        // come out true for a piece that is geometrically reversed. Three
        // successive repairs aimed at that check and none of them removed
        // the last two inconsistently wound edges of a lattice hull.
        //
        // The fan needs no check at all. The polygon here is always a
        // TRIANGLE WITH EXTRA POINTS ON ITS EDGES, so it is convex, its
        // centroid is strictly inside it, and a fan from an interior point
        // of a convex polygon covers it exactly with every piece taking the
        // polygon's own orientation -- by construction, not by luck. It
        // creates no T-junction, because the boundary is untouched.
        //
        // It also turns out to leave FEWER holes, which was not the reason
        // for it and is the better argument for it. An ear clip can cut a
        // sliver that the export funnel then drops, reopening the hole the
        // weld had just closed; a fan from the centroid cannot, because its
        // pieces all reach the middle. Measured against the ear clip:
        // elliptical 89 boundary edges to 36, a sounding hull 357 to 237, a
        // lattice hull 9,499 to 7,143, a character 888 to 880. It costs one
        // interior vertex and n pieces where the ear clip gave n - 2 --
        // between 5 and 11 per cent more triangles, and only on the faces
        // that are split at all.
        let c = [
            (p0[0] + p1[0] + p2[0]) / 3.0,
            (p0[1] + p1[1] + p2[1]) / 3.0,
            (p0[2] + p1[2] + p2[2]) / 3.0,
        ];
        let ci = positions.len() as u32;
        positions.push(c);
        for k in 0..face.len() {
            out.push([ci, face[k] as u32, face[(k + 1) % face.len()] as u32]);
        }
    }
    Mesh { positions, tris: out }
}

/// Report an IMPORTED mesh that is inside out.
///
/// Imports get this one test and not the other two, and the asymmetry is
/// deliberate. A file written by another tool may legitimately carry
/// T-junctions -- this kernel's own boolean output does, which is recorded in
/// the roadmap -- and the boundary-edge test cannot tell those from a hole,
/// so running it on every import would cry wolf on the kernel's own exports.
/// Enclosing a negative volume admits no such reading: the file is turned
/// through, and every boolean it takes part in will quietly lose geometry.
pub fn import_note(mesh: &Mesh, what: &str) -> Option<String> {
    if mesh.tris.is_empty() || mesh.signed_volume() >= 0.0 {
        return None;
    }
    Some(format!(
        "{what} encloses a negative volume, so it is inside out; booleans on it will \
         silently lose geometry (mirror it, or reverse the winding in whatever wrote it)"
    ))
}

/// Report a polyhedron that is not a closed surface.
///
/// The reference: "A polyhedron that is non-manifold only fails when it
/// participates in CSG or F6", where "the classic render error is
/// approximately 'ERROR: The given mesh is not closed! Unable to convert to
/// CGAL_Nef_Polyhedron.'" Nothing said anything: a shell with a face missing
/// went through a `difference()` and came back as arbitrary triangle soup,
/// silently. This says it where the mesh is BUILT, which is earlier than the
/// reference does and is the only place that can tell an author-supplied
/// shell from a boolean result (whose T-junctions are a separate matter and
/// would make the same test fire constantly).
///
/// Edges are matched by POSITION, not index: the reference accepts duplicate
/// points silently, and two vertices at the same place close a seam just as
/// well as one does.
pub(crate) fn closedness_note(mesh: &Mesh) -> Option<String> {
    use std::collections::HashMap;
    // Weld by position, but KEEP the degenerate triangles. `Mesh::welded`
    // drops them, and dropping one orphans its edges: a quad that closes on
    // itself at a pole, `[a, b, b, c]`, fans into `[a, b, b]` and `[a, b, c]`,
    // and discarding the first leaves `a-b` used once. Every capped sweep in
    // the example corpus tripped that. Kept, the flap seals its own edge --
    // which is the right topological reading, since it has no area to leak
    // through.
    let mut map: HashMap<[u64; 3], u32> = HashMap::new();
    let mut at: Vec<u32> = Vec::with_capacity(mesh.positions.len());
    let mut next = 0u32;
    for p in &mesh.positions {
        let bits = |v: f64| if v == 0.0 { 0f64.to_bits() } else { v.to_bits() };
        let key = [bits(p[0]), bits(p[1]), bits(p[2])];
        let idx = *map.entry(key).or_insert_with(|| {
            let i = next;
            next += 1;
            i
        });
        at.push(idx);
    }
    let welded = Mesh {
        positions: Vec::new(),
        tris: mesh
            .tris
            .iter()
            .map(|t| [at[t[0] as usize], at[t[1] as usize], at[t[2] as usize]])
            .collect(),
    };
    if welded.tris.is_empty() {
        return None;
    }
    // Every directed edge of a closed, consistently wound surface is paired
    // with exactly one opposite. Count both directions per undirected edge.
    let mut edges: HashMap<(u32, u32), (i32, i32)> = HashMap::new();
    for t in &welded.tris {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            if a == b {
                continue; // a zero-length edge bounds nothing
            }
            let key = if a < b { (a, b) } else { (b, a) };
            let slot = edges.entry(key).or_insert((0, 0));
            if a < b {
                slot.0 += 1;
            } else {
                slot.1 += 1;
            }
        }
    }
    // An edge used ONCE is a boundary: the surface has a hole there, and that
    // is exactly what "not closed" means.
    //
    // An edge used more than twice is NOT reported. Quads are fanned into
    // triangles here, and a quad that collapses at a pole leaves a zero-area
    // flap whose edges legitimately double up.
    let holes = edges.values().filter(|(f, r)| f + r == 1).count();
    // Closed is not the same as consistently wound, and the difference is
    // invisible until a boolean runs.
    //
    // A surface with a hole leaks. A surface whose two faces meet along an
    // edge and traverse it the SAME way does not leak -- every edge is used
    // twice, the parity test passes, the volume comes out near enough right
    // and it renders correctly, because shading uses |n|. What it has is no
    // consistent inside: the BSP asks "which side of this face is solid?"
    // and gets opposite answers from neighbours. An aorta built this way,
    // 576 triangles with 36 such edges and no holes at all, deleted a fifth
    // of a 101,301-triangle heart when it was merged in, in either operand
    // order.
    //
    // The test is made on the INDEX graph, before the weld, and that is the
    // whole reason it can be made at all. Tried on the welded graph it fired
    // on eight of the twenty-four example models, every one of them a sweep
    // whose separately-placed vertices happen to coincide -- a hull closing
    // to a point at the bow -- where welding brings two faces together that
    // were never adjacent and they can agree by accident. Two faces sharing
    // an INDEX edge were built adjacent on purpose, so their disagreement is
    // the author's, not the weld's. Edges whose partner is only found after
    // welding are simply not tested, which is conservative: this misses some
    // real inversions rather than inventing any.
    let mut by_index: HashMap<(u32, u32), (i32, i32)> = HashMap::new();
    for t in &mesh.tris {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            if a == b {
                continue;
            }
            let slot = by_index.entry(if a < b { (a, b) } else { (b, a) }).or_insert((0, 0));
            if a < b {
                slot.0 += 1;
            } else {
                slot.1 += 1;
            }
        }
    }
    let flipped =
        by_index.values().filter(|(f, r)| (*f == 2 && *r == 0) || (*f == 0 && *r == 2)).count();
    // Closed, consistently wound, and still inside out.
    //
    // This is the last member of the family and the one that hides longest.
    // Every edge is used twice and every pair of faces disagrees about the
    // edge between them in the right way, so both tests above pass; the
    // whole surface is simply turned through. It renders correctly, because
    // shading uses the absolute value of the normal, and the triangle count
    // and silhouette are right. What it encloses is a negative volume, which
    // is not a quantity a solid can have.
    //
    // It cost this project three models before it showed, and the thing that
    // finally showed it was a boolean losing geometry three steps later. A
    // solid's own volume is one pass over its triangles and says it outright.
    let inverted = holes == 0 && mesh.signed_volume() < 0.0;
    let plural = |n: usize| if n == 1 { "" } else { "s" };
    if inverted && flipped == 0 {
        // Say WHICH one. A polyhedron has no name, and a model that builds
        // three hundred of them in a loop needs more than the fact that one
        // of them is wrong; the box and the size locate it at a glance.
        let (blo, bhi) = bounds(mesh).unwrap_or(([0.0; 3], [0.0; 3]));
        return Some(format!(
            "polyhedron: the given mesh is closed and consistently wound, but inside out \
             (it encloses {:.6} where a solid must enclose a positive volume); every face \
             is listed the other way round from the way this kernel reads them, so \
             booleans on it will silently lose geometry. {} triangles, spanning \
             [{:.3} {:.3} {:.3}] to [{:.3} {:.3} {:.3}]",
            mesh.signed_volume(),
            mesh.tris.len(),
            blo[0],
            blo[1],
            blo[2],
            bhi[0],
            bhi[1],
            bhi[2]
        ));
    }
    match (holes, flipped) {
        (0, 0) => None,
        (0, n) => Some(format!(
            "polyhedron: the given mesh is closed but not consistently wound ({} edge{} \
             where the two faces run the same way round rather than opposite ways); \
             booleans on it are undefined and will silently lose geometry",
            n,
            plural(n)
        )),
        (h, 0) => Some(format!(
            "polyhedron: the given mesh is not closed ({} boundary edge{}); \
             booleans and exports on it are undefined",
            h,
            plural(h)
        )),
        (h, n) => Some(format!(
            "polyhedron: the given mesh is not closed ({} boundary edge{}) and is not \
             consistently wound ({} edge{} whose two faces run the same way round); \
             booleans and exports on it are undefined",
            h,
            plural(h),
            n,
            plural(n)
        )),
    }
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

    /// "A polyhedron that is non-manifold only fails when it participates in
    /// CSG or F6" — and nothing said anything, so a shell with a face missing
    /// went through a difference() and came back as arbitrary triangle soup.
    /// A face is ear triangulated, because a fan from vertex 0 gets two
    /// ordinary faces wrong.
    ///
    /// A collinear vertex at the apex makes a zero-area triangle, which the
    /// export funnel then drops -- no format can write a facet whose corners
    /// are collinear -- taking two edges of a closed mesh with it. And a
    /// concave face fanned from a reflex corner covers ground outside
    /// itself, so the solid comes out the wrong shape.
    #[test]
    fn a_polyhedron_face_is_ear_triangulated() {
        // A unit cube whose front face is given as five points, the fifth a
        // midpoint on its bottom edge. The bottom face uses the same point,
        // so the two agree edge for edge and the mesh is closed as written.
        // Fanned from vertex 0 the front face gave (0, 4, 1) -- three points
        // in a line -- and after the export funnel the cube had two boundary
        // edges.
        let pts: Vec<Vec3> = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.5, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.5, 1.0, 0.0],
        ];
        // Wound as the reference winds them, clockwise seen from outside.
        let faces: Vec<Vec<usize>> = vec![
            vec![3, 2, 1, 4, 0],    // front, with 4 collinear between 0 and 1
            vec![9, 6, 7, 8, 5],    // back, likewise
            vec![5, 8, 3, 0],       // left
            vec![2, 7, 6, 1],       // right
            vec![8, 7, 2, 3],       // top
            vec![4, 9, 5, 0],       // bottom, split at 4/9 to match the front
            vec![1, 6, 9, 4],
        ];
        let (mesh, w) = polyhedron(&pts, &faces);
        assert!(w.is_empty(), "closed as written: {w:?}");
        assert!((mesh.signed_volume() - 1.0).abs() < 1e-12, "vol {}", mesh.signed_volume());
        // No triangle has zero area, so the export funnel drops nothing and
        // the cube is still closed once written.
        let written = mesh.without_unrepresentable(Grid::Binary);
        assert_eq!(written.tris.len(), mesh.tris.len(), "nothing to drop");
        assert_eq!(closedness_note(&written), None, "and it survives the writer closed");

        // A CONCAVE face: an L in the z = 0 plane, as one six-sided face.
        // Fanned from vertex 0 (the reflex corner at [1,1]) it covers ground
        // the L does not; ear clipped it covers exactly the L, area 3.
        let l: Vec<Vec3> = vec![
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 2.0, 0.0],
            [1.0, 2.0, 0.0],
        ];
        let tris = face_tris(&l, &[0, 1, 2, 3, 4, 5]);
        assert_eq!(tris.len(), 4);
        let area: f64 = tris
            .iter()
            .map(|t| {
                let (a, b, c) = (l[t[0]], l[t[1]], l[t[2]]);
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) / 2.0
            })
            .sum();
        assert!((area - 3.0).abs() < 1e-12, "the L has area 3, not {area}");
    }

    #[test]
    fn an_open_polyhedron_is_reported() {
        let tet = |faces: &[&[usize]]| {
            let pts = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
            let f: Vec<Vec<usize>> = faces.iter().map(|x| x.to_vec()).collect();
            polyhedron(&pts, &f).1
        };
        // Wound the way this kernel reads a solid: every face's right-hand
        // normal points INTO it. Check it against the arithmetic rather than
        // against intuition -- the fixture that stood here before was the
        // other way round, and since the test only asked about closure,
        // nothing caught it for the life of the file.
        let closed: &[&[usize]] = &[&[0, 1, 2], &[0, 3, 1], &[1, 3, 2], &[0, 2, 3]];
        {
            let f: Vec<Vec<usize>> = closed.iter().map(|x| x.to_vec()).collect();
            let pts = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
            let v = polyhedron(&pts, &f).0.signed_volume();
            assert!((v - 1.0 / 6.0).abs() < 1e-12, "the fixture encloses {v}, not +1/6");
        }
        assert!(tet(closed).is_empty(), "a closed tetrahedron says nothing");

        // A face missing leaves three edges with nothing on the other side.
        let w = tet(&[&[0, 1, 2], &[0, 3, 1], &[1, 3, 2]]);
        assert!(w.iter().any(|m| m.contains("not closed") && m.contains("3 boundary")), "{w:?}");

        // ONE face wound the wrong way leaves no boundary at all -- the
        // parity test passes, the mesh renders correctly, and every boolean
        // on it is undefined. Reported separately, and by name.
        let w = tet(&[&[0, 1, 2], &[0, 3, 1], &[1, 3, 2], &[0, 3, 2]]);
        assert!(
            w.iter().any(|m| m.contains("not consistently wound") && m.contains("3 edges")),
            "{w:?}"
        );

        // EVERY face wound the other way is the case that hides longest:
        // closed, consistently wound, renders correctly, and encloses a
        // negative volume. Three models shipped like this before the kernel
        // could say so.
        let w = tet(&[&[0, 2, 1], &[0, 1, 3], &[1, 2, 3], &[0, 3, 2]]);
        assert!(w.iter().any(|m| m.contains("inside out")), "{w:?}");

        // Duplicate points close a seam as well as one point does: the
        // reference accepts them silently, so edges match by POSITION.
        let pts = [
            [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 0.0],
        ];
        let faces = vec![vec![0, 1, 2], vec![0, 3, 1], vec![1, 3, 2], vec![4, 2, 3]];
        assert!(polyhedron(&pts, &faces).1.is_empty(), "a duplicated corner still closes");

        // A quad that closes on itself at a pole fans into a degenerate
        // triangle plus a real one. Dropping the degenerate orphans its
        // edges, which flagged every capped sweep in the example corpus;
        // kept, the flap seals its own edge.
        let with_pole = vec![vec![0, 1, 2], vec![0, 3, 1], vec![1, 3, 2], vec![0, 2, 3, 3]];
        assert!(
            polyhedron(&pts[..4], &with_pole).1.is_empty(),
            "a quad that collapses at a pole fans into a zero-area flap whose edges \
             double up legitimately"
        );

        // The winding test is made on the INDEX graph, which is what keeps it
        // usable. Two faces that share no index but whose vertices coincide
        // -- a sweep closing to a point, and the reason the welded form of
        // this test fired on eight example models -- are not compared at all.
        let bow = [
            [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0], [0.0, 1.0, 0.0],
        ];
        let split_seam = vec![vec![0, 2, 1], vec![4, 5, 3], vec![1, 2, 3], vec![0, 3, 2]];
        let w = polyhedron(&bow, &split_seam).1;
        assert!(
            !w.iter().any(|m| m.contains("not consistently wound")),
            "faces brought together only by the weld are not judged: {w:?}"
        );
    }

    /// An imported mesh gets one test, and it is the one no other reading
    /// can explain away.
    #[test]
    fn an_imported_mesh_is_judged_only_on_its_volume() {
        let pts = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let right: Vec<Vec<usize>> = vec![vec![0, 1, 2], vec![0, 3, 1], vec![1, 3, 2], vec![0, 2, 3]];
        let wrong: Vec<Vec<usize>> = vec![vec![0, 2, 1], vec![0, 1, 3], vec![1, 2, 3], vec![0, 3, 2]];
        assert!(import_note(&polyhedron(&pts, &right).0, "import('x.stl')").is_none());
        let w = import_note(&polyhedron(&pts, &wrong).0, "import('x.stl')").unwrap();
        assert!(w.contains("inside out") && w.contains("x.stl"), "{w}");
        // An open mesh is NOT reported here: a file from another tool may
        // carry T-junctions, this kernel's own boolean output does, and the
        // boundary test cannot tell those from a hole.
        let open: Vec<Vec<usize>> = vec![vec![0, 1, 2], vec![0, 3, 1], vec![1, 3, 2]];
        assert!(import_note(&polyhedron(&pts, &open).0, "import('x.stl')").is_none());
        assert!(import_note(&Mesh::empty(), "import('x.stl')").is_none());
    }

    /// A closed, consistently wound cube says nothing -- the winding test must
    /// not fire on the most ordinary mesh there is.
    #[test]
    fn a_well_wound_box_is_silent() {
        let pts = [
            [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0],
        ];
        let faces: Vec<Vec<usize>> = vec![
            vec![0, 1, 2, 3],
            vec![4, 7, 6, 5],
            vec![0, 4, 5, 1],
            vec![1, 5, 6, 2],
            vec![2, 6, 7, 3],
            vec![3, 7, 4, 0],
        ];
        let (mesh, warnings) = polyhedron(&pts, &faces);
        assert!(warnings.is_empty(), "{warnings:?}");
        // ...and it is wound the way this kernel reads, so it encloses +1.
        assert!((mesh.signed_volume() - 1.0).abs() < 1e-12, "{}", mesh.signed_volume());
    }

    /// The 2D fill sweep resolves crossings numerically, so two scanline
    /// crossings can land an ULP apart and the trapezoid between them is a
    /// sliver: real at f64, exactly collinear once written to six
    /// significant digits. Every one of those exported as
    /// `facet normal 0 0 0` -- a facet whose normal the STL format requires
    /// and which no reader can recover from three collinear vertices.
    #[test]
    fn a_face_far_from_the_origin_is_judged_by_its_own_axes() {
        // The f32 floor used to come from the single largest coordinate of
        // the triangle, over all three axes at once. So a face that is thin
        // along x and y was measured against z's step: a 0.1 x 0.1 x 100 post
        // lifted to z = 1e6 lost BOTH end caps and exported as an open tube
        // with a third of its volume missing, and a sphere far enough out
        // vanished altogether -- reported as "Current top level object is
        // empty", which it was not.
        let at = |dz: f64| {
            let mut m = cube([0.1, 0.1, 100.0], false);
            for p in m.positions.iter_mut() {
                p[2] += dz;
            }
            m
        };
        for dz in [0.0, 1e6, 1e7] {
            let m = at(dz);
            let clean = m.without_unrepresentable(Grid::Binary);
            assert_eq!(clean.tris.len(), 12, "every face survives at z + {}", dz);
            assert_eq!(
                closedness_note(&clean),
                None,
                "and the solid is still closed at z + {}",
                dz
            );
        }

        // The guard still does its own job there: a triangle whose corners
        // come out collinear is dropped wherever it sits, because no reader
        // can assign it a normal.
        let mut flat = cube([0.1, 0.1, 100.0], false);
        for p in flat.positions.iter_mut() {
            p[2] += 1e6;
        }
        let n = flat.positions.len() as u32;
        flat.positions.push([0.0, 0.0, 1e6]);
        flat.positions.push([0.05, 0.0, 1e6]);
        flat.positions.push([0.1, 0.0, 1e6]);
        flat.tris.push([n, n + 1, n + 2]);
        assert_eq!(
            flat.without_unrepresentable(Grid::Binary).tris.len(),
            12,
            "the collinear triangle goes, the twelve real ones stay"
        );
    }

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

        let clean = m.without_unrepresentable(Grid::Binary);
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
        let flattened = thin.without_unrepresentable(Grid::Binary);
        assert_eq!(flattened.tris.len(), 12, "flattened by rounding");

        // A triangle that is small but REPRESENTABLE survives: the writers
        // resolve 1e-6, and 1e-3 is a thousand times that.
        let mut ok = cube([1.0, 1.0, 1.0], false);
        let n = ok.positions.len() as u32;
        ok.positions.push([0.25, 0.25, 0.0]);
        ok.positions.push([0.25 + 1e-3, 0.25, 0.0]);
        ok.positions.push([0.25, 0.25 + 1e-3, 0.0]);
        ok.tris.push([n, n + 1, n + 2]);
        let kept = ok.without_unrepresentable(Grid::Binary);
        assert_eq!(kept.tris.len(), 13, "a real small face is kept");

        // A triangle with a repeated vertex has no area at all.
        let mut dup = cube([1.0, 1.0, 1.0], false);
        dup.tris.push([0, 1, 1]);
        assert_eq!(dup.without_unrepresentable(Grid::Binary).tris.len(), 12);

        // An empty mesh has no bounds; it must come back empty, not panic.
        assert!(Mesh::empty().without_unrepresentable(Grid::Binary).tris.is_empty());
        assert!(Mesh::empty().without_unrepresentable(Grid::Text).tris.is_empty());
    }

    /// Dropping what a format cannot write can OPEN a mesh that was closed.
    ///
    /// This is the whole reason the export says so out loud. A prism whose
    /// cross-section has three consecutive corners closer together than one
    /// f32 step -- 3.8e-06 at a coordinate of 62 -- has a cap triangle whose
    /// three corners write to exactly one f32 line. No binary STL can carry
    /// it, dropping it takes three edges of each cap with it, and the file
    /// is open. It is not a modelling error and it is not a bug in the
    /// triangulator: it is the format's resolution, and the same design in
    /// OFF, AMF or 3MF is closed.
    #[test]
    fn dropping_an_unwritable_face_can_open_a_closed_mesh() {
        // 61.9999985 is 1.5e-06 below 62, which is inside the f32 half-step
        // of 1.9e-06, so it writes as 62.0 exactly.
        let x = 61.999_998_5_f64;
        assert_eq!(x as f32, 62.0f32, "the near corner writes as 62 in f32");
        let ring = [[62.0, 0.0], [x, -1.5], [60.0, -1.5], [60.0, 1.5], [x, 1.5]];
        let mut pts: Vec<Vec3> = Vec::new();
        for z in [0.0, 1.0] {
            for p in ring {
                pts.push([p[0], p[1], z]);
            }
        }
        // Caps hand-triangulated so the thin corner triangle is one of them,
        // which is what any triangulation of this outline has to produce.
        let mut faces: Vec<Vec<usize>> = vec![
            vec![4, 1, 0],
            vec![4, 2, 1],
            vec![4, 3, 2],
            vec![5, 6, 9],
            vec![6, 7, 9],
            vec![7, 8, 9],
        ];
        for i in 0..5 {
            let j = (i + 1) % 5;
            faces.push(vec![j, j + 5, i + 5, i]);
        }
        let (mesh, w) = polyhedron(&pts, &faces);
        assert!(w.is_empty(), "the prism is closed as written: {w:?}");

        let text = mesh.without_unrepresentable(Grid::Text);
        assert_eq!(text.tris.len(), mesh.tris.len(), "1e-06 resolves 1.5e-06");
        assert_eq!(closedness_note(&text), None, "so the text formats stay closed");

        let binary = mesh.without_unrepresentable(Grid::Binary);
        assert_eq!(binary.tris.len(), mesh.tris.len() - 2, "one cap triangle at each end");
        let note = closedness_note(&binary).expect("and the binary file is open");
        assert!(note.contains("6 boundary edges"), "{note}");
    }

    /// A boolean's T-junctions are closed without moving any surface.
    ///
    /// The BSP cuts one side against the other's planes, so a vertex of one
    /// surface lands partway along an edge of the other: the volume is
    /// right, the surface is continuous, and the mesh is open. Two cubes
    /// overlapping by a quarter come out at exactly 5,000 with twelve
    /// boundary edges. Inserting the stray vertex into the triangle that
    /// owns the edge closes it and adds no geometry.
    #[test]
    fn a_t_junction_closes_without_moving_anything() {
        // A unit square split into two triangles, and beside it a square
        // split into four along a middle vertex that lands on the shared
        // edge. As written the shared edge is used once on the left and
        // twice on the right.
        let pts: Vec<Vec3> = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.5, 0.0],  // the T: on the edge 1-2, and a corner on the right
            [2.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
        ];
        let mesh = Mesh {
            positions: pts,
            tris: vec![
                [0, 1, 2],
                [0, 2, 3],
                [1, 5, 4],
                [5, 6, 4],
                [4, 6, 2],
            ],
        };
        let before = area(&mesh);
        let open = boundary_edges(&mesh);
        assert!(open.contains(&(1, 2)), "the long edge is unmatched: {open:?}");

        let fixed = weld_tjunctions(&mesh);
        // The long triangle becomes a quad -- its own three corners plus the
        // point on its edge -- and is fanned from its centroid, so it goes
        // to four pieces rather than the two an ear clip would cut. Five
        // triangles in, one replaced by four, is eight. The extra vertex is
        // interior, which is why the rim below is unchanged.
        assert_eq!(fixed.tris.len(), 8, "the long triangle is fanned, not clipped");
        assert!((area(&fixed) - before).abs() < 1e-12, "and nothing moved");
        // Every interior edge is now shared; only the outer rim is boundary.
        let rim = boundary_edges(&fixed);
        assert_eq!(rim.len(), 6, "the outline of a 2x1 rectangle, split at the T: {rim:?}");
    }

    /// Total area, for checking that a repair moved no surface.
    fn area(m: &Mesh) -> f64 {
        m.tris
            .iter()
            .map(|t| {
                let (a, b, c) = (m.positions[t[0] as usize], m.positions[t[1] as usize], m.positions[t[2] as usize]);
                let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
                (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() / 2.0
            })
            .sum()
    }

    /// Undirected edges used an odd number of times, as (lo, hi) pairs.
    fn boundary_edges(m: &Mesh) -> Vec<(u32, u32)> {
        use std::collections::HashMap;
        let mut count: HashMap<(u32, u32), usize> = HashMap::new();
        for t in &m.tris {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                *count.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        let mut out: Vec<_> = count.into_iter().filter(|(_, c)| c % 2 == 1).map(|(e, _)| e).collect();
        out.sort();
        out
    }

    /// A face binary STL cannot hold is not a face the text formats may drop.
    ///
    /// f32's step at a coordinate of 1e6 is 1e6 * f32::EPSILON = 0.119, so a
    /// one-millimetre face out there has all three corners on ONE f32 point
    /// and there is no writing it to a binary STL. OFF, AMF, 3MF and ASCII
    /// STL print `{:.6}`, which resolves it five orders of magnitude over, so
    /// for them it is an ordinary face. One guard for both lost it from every
    /// format, for a limitation only one of them has.
    #[test]
    fn each_format_drops_only_what_it_cannot_write() {
        let far = 1.0e6;
        let mut m = cube([1.0, 1.0, 1.0], false);
        for p in m.positions.iter_mut() {
            for k in 0..3 {
                p[k] += far;
            }
        }
        let n = m.positions.len() as u32;
        m.positions.push([far, far, far]);
        m.positions.push([far + 1e-3, far, far]);
        m.positions.push([far, far + 1e-3, far]);
        m.tris.push([n, n + 1, n + 2]);

        // The three corners really are one point in f32, so the format that
        // stores f32 cannot carry the face...
        let snap = |p: Vec3| [p[0] as f32, p[1] as f32, p[2] as f32];
        assert_eq!(snap(m.positions[n as usize]), snap(m.positions[n as usize + 1]));
        assert_eq!(m.without_unrepresentable(Grid::Binary).tris.len(), 12);

        // ...and the formats that print decimals carry it exactly. A
        // millimetre is a thousand steps of their 1e-6 grid.
        let text = m.without_unrepresentable(Grid::Text);
        assert_eq!(text.tris.len(), 13, "OFF, AMF, 3MF and ASCII STL keep it");
        assert!(
            crate::io::write_off(&text).contains("1000000.001"),
            "and write the corner out in full"
        );
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
