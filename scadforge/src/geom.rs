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
}

/// GRID_FINE from the reference: radii below 2^-20 always get 3 fragments.
const GRID_FINE: f64 = 1.0 / 1_048_576.0;

/// The single exact tessellation formula from the reference:
/// fragments(r, $fn, $fa, $fs) =
///   (r < 2^-20) ? 3
///   : ($fn > 0) ? max(int($fn), 3)
///   : ceil(max(min(360/$fa, 2*PI*r/$fs), 5))
/// $fa/$fs below 0.01 clamp to 0.01 (the reference's WARNING behavior;
/// the slice clamps silently).
pub fn fragments(r: f64, fn_: f64, fa: f64, fs: f64) -> u32 {
    if r < GRID_FINE {
        return 3;
    }
    if fn_ > 0.0 {
        return (fn_ as i64).max(3) as u32;
    }
    let fa = fa.max(0.01);
    let fs = fs.max(0.01);
    let by_angle = 360.0 / fa;
    let by_arc = 2.0 * std::f64::consts::PI * r / fs;
    by_angle.min(by_arc).max(5.0).ceil() as u32
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
    let (s, c) = deg.to_radians().sin_cos();
    let mut m = identity();
    m[1][1] = c;
    m[1][2] = -s;
    m[2][1] = s;
    m[2][2] = c;
    m
}

fn rot_y(deg: f64) -> Mat4 {
    let (s, c) = deg.to_radians().sin_cos();
    let mut m = identity();
    m[0][0] = c;
    m[0][2] = s;
    m[2][0] = -s;
    m[2][2] = c;
    m
}

fn rot_z(deg: f64) -> Mat4 {
    let (s, c) = deg.to_radians().sin_cos();
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
pub fn rotation_axis(deg: f64, v: Vec3) -> Mat4 {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len == 0.0 {
        return rot_z(deg);
    }
    let (x, y, z) = (v[0] / len, v[1] / len, v[2] / len);
    let (s, c) = deg.to_radians().sin_cos();
    let t = 1.0 - c;
    [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y, 0.0],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x, 0.0],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
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
    if size.iter().any(|&s| !(s > 0.0)) {
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
    if !(r > 0.0) {
        return Mesh::empty();
    }
    let n = n.max(3) as usize;
    let rings = (n as f64 / 2.0).ceil() as usize;
    let mut positions = Vec::with_capacity(rings * n);
    for i in 0..rings {
        let phi = std::f64::consts::PI * (i as f64 + 0.5) / rings as f64;
        let (rz, z) = (r * phi.sin(), r * phi.cos());
        for j in 0..n {
            let theta = 2.0 * std::f64::consts::PI * j as f64 / n as f64;
            positions.push([rz * theta.cos(), rz * theta.sin(), z]);
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
    if !(h > 0.0) || r1 < 0.0 || r2 < 0.0 || (r1 == 0.0 && r2 == 0.0) {
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
            for j in 0..n {
                let theta = 2.0 * std::f64::consts::PI * j as f64 / n as f64;
                positions.push([r * theta.cos(), r * theta.sin(), z]);
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
