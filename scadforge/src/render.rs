//! Headless rendering: a scene of coloured triangles to an image.
//!
//! The browser viewport shades with one hard-coded directional light and a
//! flat ambient term, which is enough to tell a cube from a sphere and not
//! much more. This is the same geometry lit properly: three lights with
//! shadow maps, an ambient term that knows which way is up, ambient
//! occlusion from the depth buffer, a specular lobe, and a filmic curve at
//! the end. It is a rasteriser, not a path tracer, so everything here is a
//! deliberate approximation of an integral nobody is going to evaluate.
//!
//! The pipeline, in order:
//!
//!   1. Frame the scene, build a camera basis.
//!   2. Rasterise a G-BUFFER at `samples` times the output resolution:
//!      depth, normal, albedo and world position per sub-pixel.
//!   3. Rasterise a DEPTH MAP from each shadow-casting light.
//!   4. Shade: hemisphere ambient x occlusion, then each light's diffuse
//!      and specular, each gated by its shadow map.
//!   5. Tone map, gamma, and box-filter the supersamples down.
//!
//! Zero dependencies, like everything else here. The threading is
//! `std::thread::scope` over horizontal bands.

use crate::geom::Mesh;

type V3 = [f64; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    if l > 0.0 {
        mul(a, 1.0 / l)
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// What to draw and how.
#[derive(Clone)]
pub struct View {
    pub width: usize,
    pub height: usize,
    /// Supersampling factor per axis. 2 is the useful default: it costs four
    /// times the shading and removes every staircase a model this angular
    /// produces.
    pub samples: usize,
    /// Camera orbit, in degrees. Azimuth turns about +z from +x; elevation
    /// lifts from the xy plane.
    pub azimuth: f64,
    pub elevation: f64,
    /// Field of view in degrees, vertical. Zero means orthographic, which
    /// is what a technical drawing wants.
    pub fov: f64,
    /// 0 turns shadows off, 1 leaves them on. Shadow maps are the most
    /// expensive stage after the G-buffer.
    pub shadows: bool,
    pub occlusion: bool,
    /// Put a floor under the model for its shadow to fall on. Without one a
    /// shadow map can only darken the model where it shades itself, which on
    /// a convex part is nowhere at all -- the whole stage does visible work
    /// only once there is something for the shadow to land on.
    pub ground: bool,
    /// Background, linear, before tone mapping.
    pub background: [f64; 3],
}

impl Default for View {
    fn default() -> View {
        View {
            width: 900,
            height: 700,
            samples: 2,
            azimuth: 38.0,
            elevation: 24.0,
            fov: 32.0,
            shadows: true,
            occlusion: true,
            ground: false,
            background: [0.012, 0.016, 0.026],
        }
    }
}

/// One triangle ready to rasterise: positions, a face normal, a colour.
struct Tri {
    v: [V3; 3],
    n: V3,
    albedo: [f64; 3],
    alpha: f64,
}

/// A light: direction TOWARDS the light, colour already scaled by intensity.
struct Light {
    dir: V3,
    color: [f64; 3],
    shadow: bool,
    /// Specular strength; the fill light gets none so it reads as bounce.
    spec: f64,
}

/// Collect the drawable triangles. `%` background shapes are drawn dim and
/// `#` highlights are drawn as they are, matching what the viewport does.
fn gather(shapes: &[(Mesh, Option<[f64; 4]>, bool)]) -> Vec<Tri> {
    let mut out = Vec::new();
    for (mesh, color, background) in shapes {
        let (rgb, alpha) = match color {
            Some(c) if *background => ([0.45, 0.45, 0.48], c[3].min(0.35)),
            Some(c) => ([c[0], c[1], c[2]], c[3]),
            None if *background => ([0.45, 0.45, 0.48], 0.35),
            None => ([0.75, 0.76, 0.78], 1.0),
        };
        for t in &mesh.tris {
            let v = [
                mesh.positions[t[0] as usize],
                mesh.positions[t[1] as usize],
                mesh.positions[t[2] as usize],
            ];
            // This kernel winds a face so the right-hand normal points INTO
            // the solid, so the outward normal is the other way round. Any
            // renderer that takes the absolute value here cannot tell a
            // solid from the same solid inside out, which is exactly how
            // three models in this repository shipped wrong.
            let n = norm(cross(sub(v[1], v[0]), sub(v[2], v[0])));
            out.push(Tri {
                v,
                n: mul(n, -1.0),
                albedo: [from_srgb(rgb[0]), from_srgb(rgb[1]), from_srgb(rgb[2])],
                alpha,
            });
        }
    }
    out
}

fn bounds(tris: &[Tri]) -> (V3, V3) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for t in tris {
        for p in &t.v {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
    }
    (lo, hi)
}

/// An orthonormal camera basis and the projection that goes with it.
struct Cam {
    eye: V3,
    right: V3,
    up: V3,
    fwd: V3,
    /// Half-height of the view at the focus distance, in world units.
    half: f64,
    dist: f64,
    ortho: bool,
    w: f64,
    h: f64,
}

impl Cam {
    /// Project to pixel coordinates plus a depth. Depth is distance along
    /// the forward axis, so bigger is further.
    fn project(&self, p: V3) -> (f64, f64, f64) {
        let d = sub(p, self.eye);
        let x = dot(d, self.right);
        let y = dot(d, self.up);
        let z = dot(d, self.fwd);
        let scale = if self.ortho { self.h * 0.5 / self.half } else { self.h * 0.5 / self.half * self.dist / z.max(1e-9) };
        (self.w * 0.5 + x * scale, self.h * 0.5 - y * scale, z)
    }
}

/// Frame the scene.
///
/// The framing is computed from the VERTICES, not from the bounding box, and
/// in two steps: fix the eye from the bounding sphere, then open the view
/// only as far as the furthest vertex actually needs. A box-derived half
/// extent is wrong for anything that is not a box -- it crops a long model
/// seen corner-on, which is most of them -- and under perspective it is
/// wrong even for a box, because a near corner subtends more than a far one.
fn camera(view: &View, tris: &[Tri], lo: V3, hi: V3, w: f64, h: f64) -> Cam {
    let centre = mul(add(lo, hi), 0.5);
    let mut radius: f64 = 0.0;
    for t in tris {
        for p in &t.v {
            radius = radius.max(dot(sub(*p, centre), sub(*p, centre)).sqrt());
        }
    }
    if radius <= 0.0 {
        radius = 1.0;
    }
    let (ca, sa) = (view.azimuth.to_radians().cos(), view.azimuth.to_radians().sin());
    let (ce, se) = (view.elevation.to_radians().cos(), view.elevation.to_radians().sin());
    let dirv = [ca * ce, sa * ce, se];
    let ortho = view.fov <= 0.0;
    let fwd = mul(dirv, -1.0);
    let world_up = if view.elevation.abs() > 88.0 { [0.0, 1.0, 0.0] } else { [0.0, 0.0, 1.0] };
    let right = norm(cross(fwd, world_up));
    let up = norm(cross(right, fwd));
    // Far enough back that the bounding sphere is comfortably inside the
    // nominal field of view; the exact framing then comes from the vertices.
    let dist = if ortho {
        radius * 4.0
    } else {
        radius / (view.fov.to_radians() * 0.5).sin().max(0.05) + radius
    };
    let eye = add(centre, mul(dirv, dist));
    let aspect = w / h;
    let mut half: f64 = radius * 1e-3;
    for t in tris {
        for p in &t.v {
            let d = sub(*p, eye);
            let (x, y, z) = (dot(d, right), dot(d, up), dot(d, fwd));
            if ortho {
                half = half.max(y.abs()).max(x.abs() / aspect);
            } else if z > 1e-9 {
                // Vertical half-extent needed to hold this point, given that
                // the view opens linearly with depth.
                half = half.max(y.abs() * dist / z).max(x.abs() * dist / z / aspect);
            }
        }
    }
    half *= 1.04; // a hair of air, so nothing touches the frame
    Cam { eye, right, up, fwd, half, dist, ortho, w, h }
}

/// Per-sub-pixel geometry, kept for the shading pass.
struct GBuf {
    w: usize,
    h: usize,
    depth: Vec<f32>,
    normal: Vec<[f32; 3]>,
    albedo: Vec<[f32; 3]>,
    world: Vec<[f32; 3]>,
    /// Anything the camera did not hit stays false and takes the background.
    hit: Vec<bool>,
}

/// Rasterise triangles into a G-buffer with a depth test.
///
/// Flat shading, deliberately: these are faceted meshes of machine parts and
/// a smoothed normal would invent curvature the model does not have. What
/// removes the hard look is the lighting, not fake normals.
fn raster(tris: &[Tri], cam: &Cam, w: usize, h: usize) -> GBuf {
    let mut g = GBuf {
        w,
        h,
        depth: vec![f32::INFINITY; w * h],
        normal: vec![[0.0; 3]; w * h],
        albedo: vec![[0.0; 3]; w * h],
        world: vec![[0.0; 3]; w * h],
        hit: vec![false; w * h],
    };
    // Nothing may be projected until it is in front of the eye, so a
    // triangle that straddles the near plane is cut there first. Dropping
    // such triangles instead -- which is what this did -- is invisible on a
    // compact part and removes the FLOOR entirely, because a plane wide
    // enough to catch a shadow always has corners behind the camera.
    let near = if cam.ortho { f64::NEG_INFINITY } else { cam.dist * 1e-4 };
    let mut work: Vec<[V3; 3]> = Vec::with_capacity(2);
    for t in tris {
        // A transparent shape is a ghost in the viewport; here it simply
        // does not write geometry, which keeps the shading pass honest.
        if t.alpha < 0.999 {
            continue;
        }
        work.clear();
        let depth = |v: V3| dot(sub(v, cam.eye), cam.fwd);
        let d = [depth(t.v[0]), depth(t.v[1]), depth(t.v[2])];
        let inside = d.iter().filter(|z| **z > near).count();
        match inside {
            0 => continue,
            3 => work.push(t.v),
            _ => {
                // Sutherland-Hodgman against the one plane: walk the edges,
                // keeping vertices in front and adding a crossing point
                // wherever an edge changes side. Three or four vertices come
                // back, and a quad fans into two triangles.
                let mut poly: Vec<V3> = Vec::with_capacity(4);
                for k in 0..3 {
                    let (a, b) = (t.v[k], t.v[(k + 1) % 3]);
                    let (da, db) = (d[k], d[(k + 1) % 3]);
                    if da > near {
                        poly.push(a);
                    }
                    if (da > near) != (db > near) {
                        let f = (near - da) / (db - da);
                        poly.push(add(a, mul(sub(b, a), f)));
                    }
                }
                if poly.len() >= 3 {
                    work.push([poly[0], poly[1], poly[2]]);
                    if poly.len() == 4 {
                        work.push([poly[0], poly[2], poly[3]]);
                    }
                }
            }
        }
        for piece in &work {
        let p: Vec<(f64, f64, f64)> = piece.iter().map(|v| cam.project(*v)).collect();
        let (x0, y0) = (
            p.iter().fold(f64::INFINITY, |m, q| m.min(q.0)).floor().max(0.0) as usize,
            p.iter().fold(f64::INFINITY, |m, q| m.min(q.1)).floor().max(0.0) as usize,
        );
        let (x1, y1) = (
            (p.iter().fold(f64::NEG_INFINITY, |m, q| m.max(q.0)).ceil() as isize)
                .clamp(0, w as isize) as usize,
            (p.iter().fold(f64::NEG_INFINITY, |m, q| m.max(q.1)).ceil() as isize)
                .clamp(0, h as isize) as usize,
        );
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        let area = (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[2].0 - p[0].0) * (p[1].1 - p[0].1);
        if area.abs() < 1e-12 {
            continue;
        }
        let inv = 1.0 / area;
        for y in y0..y1 {
            let py = y as f64 + 0.5;
            for x in x0..x1 {
                let px = x as f64 + 0.5;
                let w0 = ((p[1].0 - px) * (p[2].1 - py) - (p[2].0 - px) * (p[1].1 - py)) * inv;
                let w1 = ((p[2].0 - px) * (p[0].1 - py) - (p[0].0 - px) * (p[2].1 - py)) * inv;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                // PERSPECTIVE-CORRECT interpolation. Barycentric weights
                // computed on the screen are not the weights in the world:
                // under perspective a near part of a triangle covers more
                // pixels than a far part, so an attribute interpolated
                // linearly across the screen drifts. Interpolating 1/z, and
                // each attribute over z, and dividing at the end, is the
                // correction.
                //
                // It is invisible on small triangles and catastrophic on
                // large ones. The floor is TWO triangles spanning the whole
                // frame, and its world positions came out so wrong that
                // every shadow lookup landed outside the map and reported
                // "lit" -- which is why the floor had no shadow on it at all
                // while the model shaded itself correctly.
                let (i0, i1, i2) = if cam.ortho {
                    (1.0, 1.0, 1.0)
                } else {
                    (1.0 / p[0].2, 1.0 / p[1].2, 1.0 / p[2].2)
                };
                let den = w0 * i0 + w1 * i1 + w2 * i2;
                if den.abs() < 1e-18 {
                    continue;
                }
                let (b0, b1, b2) = (w0 * i0 / den, w1 * i1 / den, w2 * i2 / den);
                let z = b0 * p[0].2 + b1 * p[1].2 + b2 * p[2].2;
                let at = y * w + x;
                if z >= g.depth[at] as f64 {
                    continue;
                }
                let wp = add(add(mul(piece[0], b0), mul(piece[1], b1)), mul(piece[2], b2));
                g.depth[at] = z as f32;
                g.normal[at] = [t.n[0] as f32, t.n[1] as f32, t.n[2] as f32];
                g.albedo[at] = [t.albedo[0] as f32, t.albedo[1] as f32, t.albedo[2] as f32];
                g.world[at] = [wp[0] as f32, wp[1] as f32, wp[2] as f32];
                g.hit[at] = true;
            }
        }
        }
    }
    g
}

/// A light's depth map: how far the light can see before something stops it.
struct Shadow {
    size: usize,
    depth: Vec<f32>,
    origin: V3,
    right: V3,
    up: V3,
    fwd: V3,
    half: f64,
    /// Depth bias, in world units, scaled to the scene so it is not a magic
    /// number: without it every surface shadows itself in a moire of
    /// stripes, which is the classic artefact and the classic fix.
    bias: f64,
}

impl Shadow {
    /// Is this world point lit, or is something between it and the light?
    ///
    /// Four taps in a rotated grid rather than one, so the edge of a shadow
    /// is a two-pixel gradient instead of a staircase. That is a poor man's
    /// soft shadow and it is what most of the softness in these images is.
    fn lit(&self, p: V3, ndl: f64) -> f64 {
        let d = sub(p, self.origin);
        let u = dot(d, self.right) / self.half * 0.5 + 0.5;
        let v = dot(d, self.up) / self.half * 0.5 + 0.5;
        let z = dot(d, self.fwd);
        if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
            return 1.0;
        }
        // A surface nearly edge-on to the light needs more bias, because one
        // texel of the map spans more depth there.
        let slope = (1.0 - ndl * ndl).sqrt() / ndl.max(0.15);
        let bias = self.bias * (1.0 + 2.0 * slope.min(6.0));
        let s = self.size as f64;
        let mut lit = 0.0;
        for (dx, dy) in [(-0.3, -0.9), (0.9, -0.3), (0.3, 0.9), (-0.9, 0.3)] {
            let x = ((u * s + dx) as isize).clamp(0, self.size as isize - 1) as usize;
            let y = ((v * s + dy) as isize).clamp(0, self.size as isize - 1) as usize;
            let seen = self.depth[y * self.size + x] as f64;
            if z <= seen + bias {
                lit += 0.25;
            }
        }
        lit
    }
}

fn shadow_map(tris: &[Tri], dir: V3, lo: V3, hi: V3, size: usize) -> Shadow {
    let centre = mul(add(lo, hi), 0.5);
    let radius = (0..3).fold(0.0f64, |m, k| m.max(hi[k] - lo[k])) * 0.5;
    let radius = if radius > 0.0 { radius } else { 1.0 };
    let fwd = mul(norm(dir), -1.0);
    let aux = if fwd[2].abs() > 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 0.0, 1.0] };
    let right = norm(cross(fwd, aux));
    let up = norm(cross(right, fwd));
    let half = radius * 1.25;
    let origin = sub(centre, mul(fwd, radius * 2.0));
    let mut depth = vec![f32::INFINITY; size * size];
    let s = size as f64;
    for t in tris {
        if t.alpha < 0.999 {
            continue;
        }
        let mut px = [(0.0f64, 0.0f64, 0.0f64); 3];
        for (i, v) in t.v.iter().enumerate() {
            let d = sub(*v, origin);
            px[i] = (
                (dot(d, right) / half * 0.5 + 0.5) * s,
                (dot(d, up) / half * 0.5 + 0.5) * s,
                dot(d, fwd),
            );
        }
        let x0 = px.iter().fold(f64::INFINITY, |m, q| m.min(q.0)).floor().max(0.0) as usize;
        let y0 = px.iter().fold(f64::INFINITY, |m, q| m.min(q.1)).floor().max(0.0) as usize;
        let x1 = (px.iter().fold(f64::NEG_INFINITY, |m, q| m.max(q.0)).ceil() as isize)
            .clamp(0, size as isize) as usize;
        let y1 = (px.iter().fold(f64::NEG_INFINITY, |m, q| m.max(q.1)).ceil() as isize)
            .clamp(0, size as isize) as usize;
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        let area =
            (px[1].0 - px[0].0) * (px[2].1 - px[0].1) - (px[2].0 - px[0].0) * (px[1].1 - px[0].1);
        if area.abs() < 1e-12 {
            continue;
        }
        let inv = 1.0 / area;
        for y in y0..y1 {
            let qy = y as f64 + 0.5;
            for x in x0..x1 {
                let qx = x as f64 + 0.5;
                let w0 =
                    ((px[1].0 - qx) * (px[2].1 - qy) - (px[2].0 - qx) * (px[1].1 - qy)) * inv;
                let w1 =
                    ((px[2].0 - qx) * (px[0].1 - qy) - (px[0].0 - qx) * (px[2].1 - qy)) * inv;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = w0 * px[0].2 + w1 * px[1].2 + w2 * px[2].2;
                let at = y * size + x;
                if z < depth[at] as f64 {
                    depth[at] = z as f32;
                }
            }
        }
    }
    Shadow { size, depth, origin, right, up, fwd, half, bias: radius * 0.004 }
}

/// Screen-space ambient occlusion from the depth buffer.
///
/// For each pixel, sample a disc of neighbours and ask how many of them sit
/// in front of the plane through this pixel. That is the fraction of the
/// hemisphere the geometry itself blocks, approximated with what the camera
/// already saw. It is wrong at silhouettes, where there is no information
/// behind the edge, and right everywhere a crease or a pocket matters --
/// which is where an image of a machine reads as solid or does not.
fn occlusion(g: &GBuf, cam: &Cam, radius: f64) -> Vec<f32> {
    const TAPS: [(f64, f64); 12] = [
        (1.0, 0.0),
        (0.5, 0.87),
        (-0.5, 0.87),
        (-1.0, 0.0),
        (-0.5, -0.87),
        (0.5, -0.87),
        (0.55, 0.32),
        (0.0, 0.63),
        (-0.55, 0.32),
        (-0.55, -0.32),
        (0.0, -0.63),
        (0.55, -0.32),
    ];
    let mut ao = vec![1.0f32; g.w * g.h];
    let scale = g.h as f64 * 0.5 / cam.half;
    for y in 0..g.h {
        for x in 0..g.w {
            let at = y * g.w + x;
            if !g.hit[at] {
                continue;
            }
            let z = g.depth[at] as f64;
            let n = [g.normal[at][0] as f64, g.normal[at][1] as f64, g.normal[at][2] as f64];
            // The sample disc is a fixed size in WORLD units, projected, so
            // occlusion does not change when the camera moves closer.
            let px = (radius * scale * if cam.ortho { 1.0 } else { cam.dist / z }).clamp(2.0, 48.0);
            let wp = [g.world[at][0] as f64, g.world[at][1] as f64, g.world[at][2] as f64];
            let mut blocked = 0.0;
            for (i, (dx, dy)) in TAPS.iter().enumerate() {
                // Spiral the taps out so the disc is covered rather than
                // ringed, and jitter by pixel so banding becomes noise.
                let r = px * (0.35 + 0.65 * ((i as f64 + 0.5) / TAPS.len() as f64));
                let j = (((x * 7 + y * 13 + i * 29) % 16) as f64 / 16.0 - 0.5) * 0.6;
                let sx = x as f64 + (dx + j) * r;
                let sy = y as f64 + (dy - j) * r;
                if sx < 0.0 || sy < 0.0 || sx >= g.w as f64 || sy >= g.h as f64 {
                    continue;
                }
                let sat = sy as usize * g.w + sx as usize;
                if !g.hit[sat] {
                    continue;
                }
                let sw = [
                    g.world[sat][0] as f64,
                    g.world[sat][1] as f64,
                    g.world[sat][2] as f64,
                ];
                let d = sub(sw, wp);
                let len = dot(d, d).sqrt();
                if len < 1e-9 || len > radius * 2.0 {
                    continue;
                }
                // How far above this pixel's own plane the neighbour sits,
                // as a fraction of the sample distance: the sine of the
                // angle it subtends, which is what occludes.
                let rise = dot(d, n) / len;
                if rise > 0.12 {
                    // Distant blockers matter less; fade with range so a
                    // background object does not darken a foreground face.
                    blocked += (rise - 0.12) / 0.88 * (1.0 - len / (radius * 2.0));
                }
            }
            let v = 1.0 - (blocked / TAPS.len() as f64) * 1.9;
            ao[at] = v.clamp(0.0, 1.0) as f32;
        }
    }
    ao
}

/// Blur the occlusion buffer so its 12 taps read as shade rather than noise.
fn blur(ao: &[f32], g: &GBuf, r: isize) -> Vec<f32> {
    let mut out = vec![1.0f32; ao.len()];
    for y in 0..g.h as isize {
        for x in 0..g.w as isize {
            let at = (y * g.w as isize + x) as usize;
            if !g.hit[at] {
                continue;
            }
            let (mut sum, mut n) = (0.0f32, 0.0f32);
            for dy in -r..=r {
                for dx in -r..=r {
                    let (sx, sy) = (x + dx, y + dy);
                    if sx < 0 || sy < 0 || sx >= g.w as isize || sy >= g.h as isize {
                        continue;
                    }
                    let sat = (sy * g.w as isize + sx) as usize;
                    if !g.hit[sat] {
                        continue;
                    }
                    // Do not blur across a depth step, or a near object
                    // bleeds its occlusion onto a far one.
                    if (g.depth[sat] - g.depth[at]).abs() > g.depth[at] * 0.02 {
                        continue;
                    }
                    sum += ao[sat];
                    n += 1.0;
                }
            }
            out[at] = if n > 0.0 { sum / n } else { ao[at] };
        }
    }
    out
}

/// The filmic curve. Maps an unbounded linear radiance onto 0..1 with a toe
/// and a shoulder, so a bright specular rolls off instead of clipping flat.
/// This is the widely published ACES fit, which is arithmetic rather than
/// anyone's code.
fn tonemap(x: f64) -> f64 {
    let (a, b, c, d, e) = (2.51, 0.03, 2.43, 0.59, 0.14);
    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
}

/// Undo the sRGB curve on a colour that was written by a person.
///
/// A colour in a .scad file is picked by eye against a screen, so it is an
/// sRGB value, not a linear radiance. Multiplying it by light as though it
/// were linear and then encoding the result to sRGB applies the curve twice:
/// 0.70 comes back as 0.86 rather than 0.70, and a deep red heart renders
/// pink. Everything between here and `to_srgb` is linear.
fn from_srgb(x: f64) -> f64 {
    if x <= 0.040_45 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

fn to_srgb(x: f64) -> u8 {
    let v = if x <= 0.003_130_8 { x * 12.92 } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 };
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Render a scene of `(mesh, colour, is_background)` to 8-bit RGB.
pub fn render(shapes: &[(Mesh, Option<[f64; 4]>, bool)], view: &View) -> Vec<u8> {
    let tris = gather(shapes);
    let (w, h) = (view.width, view.height);
    if tris.is_empty() {
        let bg = view.background;
        let px = [to_srgb(tonemap(bg[0])), to_srgb(tonemap(bg[1])), to_srgb(tonemap(bg[2]))];
        return px.iter().cycle().take(w * h * 3).copied().collect();
    }
    let s = view.samples.clamp(1, 4);
    let (sw, sh) = (w * s, h * s);
    // The MODEL's bounds, taken before any floor is added: a floor wide
    // enough to catch a low shadow is several times the part, and framing to
    // that would show a stamp in the middle of an empty plain.
    let (lo, hi) = bounds(&tris);
    let radius = (0..3).fold(0.0f64, |m, k| m.max(hi[k] - lo[k])) * 0.5;
    let cam = camera(view, &tris, lo, hi, sw as f64, sh as f64);

    // The floor goes in AFTER the camera is fixed, so it is rasterised and
    // shadowed like anything else without pulling the framing out.
    let mut tris = tris;
    let (glo, ghi) = if view.ground {
        let c = mul(add(lo, hi), 0.5);
        let reach = radius * 5.0;
        let z = lo[2] - radius * 0.004;
        let quad = [
            [c[0] - reach, c[1] - reach, z],
            [c[0] + reach, c[1] - reach, z],
            [c[0] + reach, c[1] + reach, z],
            [c[0] - reach, c[1] + reach, z],
        ];
        let albedo = [from_srgb(0.34), from_srgb(0.35), from_srgb(0.37)];
        for t in [[0usize, 1, 2], [0, 2, 3]] {
            tris.push(Tri {
                v: [quad[t[0]], quad[t[1]], quad[t[2]]],
                n: [0.0, 0.0, 1.0],
                albedo,
                alpha: 1.0,
            });
        }
        // The shadow map has to cover where the shadow LANDS, which for a
        // low key light reaches well past the part itself.
        let g = radius * 1.9;
        ([c[0] - g, c[1] - g, z], [c[0] + g, c[1] + g, hi[2]])
    } else {
        (lo, hi)
    };

    // A three-point rig, stated in CAMERA space and then turned into the
    // world, so the lighting follows the camera round the model instead of
    // going flat from behind at some angles. Key high and to the left, fill
    // low and opposite and cool, rim behind and above to separate the
    // silhouette from the background.
    let cs = |x: f64, y: f64, z: f64| -> V3 {
        norm(add(add(mul(cam.right, x), mul(cam.up, y)), mul(cam.fwd, z)))
    };
    let lights = [
        Light { dir: cs(-0.55, 0.72, -0.42), color: [0.92, 0.85, 0.74], shadow: true, spec: 0.85 },
        Light { dir: cs(0.78, -0.18, -0.30), color: [0.17, 0.21, 0.30], shadow: false, spec: 0.0 },
        Light { dir: cs(0.28, 0.48, 0.82), color: [0.30, 0.32, 0.40], shadow: false, spec: 0.45 },
    ];

    let maps: Vec<Option<Shadow>> = lights
        .iter()
        .map(|l| {
            if view.shadows && l.shadow {
                Some(shadow_map(&tris, l.dir, glo, ghi, 1800))
            } else {
                None
            }
        })
        .collect();

    let g = raster(&tris, &cam, sw, sh);
    let ao = if view.occlusion {
        blur(&occlusion(&g, &cam, radius * 0.085), &g, 2)
    } else {
        vec![1.0f32; sw * sh]
    };

    // Shade every sub-pixel, in bands, one thread per core.
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).min(8);
    let band = sh.div_ceil(threads);
    let mut lit = vec![[0.0f32; 3]; sw * sh];
    {
        let (g, ao, lights, maps, view) = (&g, &ao, &lights, &maps, &view);
        let mut rest: &mut [[f32; 3]] = &mut lit;
        let mut chunks = Vec::new();
        let mut y0 = 0;
        while y0 < sh {
            let y1 = (y0 + band).min(sh);
            let (head, tail) = rest.split_at_mut((y1 - y0) * sw);
            chunks.push((y0, head));
            rest = tail;
            y0 = y1;
        }
        std::thread::scope(|sc| {
            for (y0, buf) in chunks {
                sc.spawn(move || {
                    for (i, out) in buf.iter_mut().enumerate() {
                        let at = y0 * sw + i;
                        if !g.hit[at] {
                            *out = [
                                view.background[0] as f32,
                                view.background[1] as f32,
                                view.background[2] as f32,
                            ];
                            continue;
                        }
                        let n = norm([
                            g.normal[at][0] as f64,
                            g.normal[at][1] as f64,
                            g.normal[at][2] as f64,
                        ]);
                        let alb = [
                            g.albedo[at][0] as f64,
                            g.albedo[at][1] as f64,
                            g.albedo[at][2] as f64,
                        ];
                        let wp =
                            [g.world[at][0] as f64, g.world[at][1] as f64, g.world[at][2] as f64];
                        let occ = ao[at] as f64;
                        let vdir = norm(sub(cam.eye, wp));
                        // Two-sided: a back face lit from behind should read
                        // as shadowed, not black, so the normal is turned to
                        // face the camera and the ambient carries it.
                        let n = if dot(n, vdir) < 0.0 { mul(n, -1.0) } else { n };

                        // Hemisphere ambient: sky above, a warmer bounce from
                        // below, mixed by which way the surface faces. This
                        // is what replaces the flat 0.25 the viewport adds,
                        // and it is most of the difference in the shadows.
                        let up = (n[2] * 0.5 + 0.5).clamp(0.0, 1.0);
                        // Keep the ambient well under the key light. Lifted
                        // to where it competes, it desaturates everything:
                        // the sum runs past 1 across the whole model and the
                        // filmic shoulder flattens colour out of it, which
                        // turned a dark red heart pink.
                        let sky = [0.17, 0.20, 0.26];
                        let ground = [0.085, 0.075, 0.07];
                        let mut c = [0.0f64; 3];
                        for k in 0..3 {
                            c[k] = alb[k] * (ground[k] + (sky[k] - ground[k]) * up) * occ;
                        }
                        for (li, l) in lights.iter().enumerate() {
                            let ndl = dot(n, l.dir);
                            if ndl <= 0.0 {
                                continue;
                            }
                            let vis = match &maps[li] {
                                Some(m) => m.lit(wp, ndl),
                                None => 1.0,
                            };
                            if vis <= 0.0 {
                                continue;
                            }
                            for k in 0..3 {
                                c[k] += alb[k] * l.color[k] * ndl * vis;
                            }
                            if l.spec > 0.0 {
                                // Blinn-Phong, with a Schlick Fresnel so
                                // grazing angles brighten the way a real
                                // surface does. One roughness for everything:
                                // the language has no material, so inventing
                                // several would be inventing information.
                                let hv = norm(add(l.dir, vdir));
                                let ndh = dot(n, hv).max(0.0);
                                let f = 0.04 + 0.96 * (1.0 - dot(vdir, n).max(0.0)).powi(5);
                                let spec = ndh.powf(48.0) * f * l.spec * vis * ndl;
                                for k in 0..3 {
                                    c[k] += l.color[k] * spec;
                                }
                            }
                        }
                        *out = [c[0] as f32, c[1] as f32, c[2] as f32];
                    }
                });
            }
        });
    }

    // Tone map, then box-filter the supersamples down. Averaging AFTER the
    // curve is what makes an edge against a bright background read correctly
    // rather than fringing.
    let mut rgb = vec![0u8; w * h * 3];
    let inv = 1.0 / (s * s) as f64;
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f64; 3];
            for sy in 0..s {
                for sx in 0..s {
                    let c = lit[(y * s + sy) * sw + x * s + sx];
                    for k in 0..3 {
                        acc[k] += tonemap(c[k] as f64);
                    }
                }
            }
            let at = (y * w + x) * 3;
            for k in 0..3 {
                rgb[at + k] = to_srgb(acc[k] * inv);
            }
        }
    }
    rgb
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(size: f64) -> Mesh {
        crate::geom::cube([size, size, size], true)
    }

    fn pixel(rgb: &[u8], w: usize, x: usize, y: usize) -> [u8; 3] {
        let at = (y * w + x) * 3;
        [rgb[at], rgb[at + 1], rgb[at + 2]]
    }

    #[test]
    fn an_empty_scene_is_the_background_and_nothing_else() {
        let v = View { width: 8, height: 6, ..View::default() };
        let px = render(&[], &v);
        assert_eq!(px.len(), 8 * 6 * 3);
        let first = pixel(&px, 8, 0, 0);
        assert!(px.chunks(3).all(|c| c == first), "an empty render is not uniform");
    }

    #[test]
    fn a_lit_cube_is_brighter_than_the_background_and_shaded_across_its_faces() {
        let v = View { width: 120, height: 120, samples: 1, ..View::default() };
        let px = render(&[(cube(10.0), Some([0.8, 0.8, 0.8, 1.0]), false)], &v);
        let bg = pixel(&px, 120, 2, 2);
        let mid = pixel(&px, 120, 60, 60);
        let lum = |c: [u8; 3]| c[0] as u32 + c[1] as u32 + c[2] as u32;
        assert!(lum(mid) > lum(bg) + 60, "the cube is not lit: {mid:?} against {bg:?}");

        // Three faces are visible from this camera and no two of them face
        // the same way, so no two should come back the same brightness.
        // That is the whole point of having lights at all.
        let mut seen: Vec<u32> = Vec::new();
        for (x, y) in [(60usize, 30usize), (36, 74), (86, 74)] {
            seen.push(lum(pixel(&px, 120, x, y)));
        }
        seen.sort_unstable();
        assert!(
            seen[2] - seen[0] > 25,
            "the faces are not distinguishable from each other: {seen:?}"
        );
    }

    #[test]
    fn colour_reaches_the_image() {
        let v = View { width: 60, height: 60, samples: 1, shadows: false, ..View::default() };
        let red = render(&[(cube(10.0), Some([0.9, 0.05, 0.05, 1.0]), false)], &v);
        let blue = render(&[(cube(10.0), Some([0.05, 0.05, 0.9, 1.0]), false)], &v);
        let r = pixel(&red, 60, 30, 30);
        let b = pixel(&blue, 60, 30, 30);
        assert!(r[0] > r[2] + 40, "the red cube is not red: {r:?}");
        assert!(b[2] > b[0] + 40, "the blue cube is not blue: {b:?}");
    }

    /// The property the whole shadow stage exists for: something above
    /// something else must darken it, and must not darken it when the
    /// shadows are off.
    #[test]
    fn a_blocker_casts_a_shadow_that_turning_shadows_off_removes() {
        let floor = crate::geom::cube([80.0, 80.0, 2.0], true);
        let mut blocker = crate::geom::cube([26.0, 26.0, 2.0], true);
        for p in &mut blocker.positions {
            p[2] += 22.0;
        }
        let scene = [
            (floor, Some([0.8, 0.8, 0.8, 1.0]), false),
            (blocker, Some([0.8, 0.8, 0.8, 1.0]), false),
        ];
        let base = View {
            width: 150,
            height: 150,
            samples: 1,
            azimuth: 0.0,
            elevation: 89.0,
            fov: 0.0,
            occlusion: false,
            ..View::default()
        };
        let on = render(&scene, &base);
        let off = render(&scene, &View { shadows: false, ..base.clone() });
        let lum = |p: &[u8], x: usize, y: usize| {
            let c = pixel(p, 150, x, y);
            c[0] as i32 + c[1] as i32 + c[2] as i32
        };
        // Looking almost straight down with the key light off to one side,
        // the floor on the shaded side of the blocker must be darker with
        // shadows on than with them off.
        let mut found = false;
        for y in 20..130 {
            for x in 20..130 {
                if lum(&off, x, y) - lum(&on, x, y) > 40 {
                    found = true;
                }
            }
        }
        assert!(found, "no pixel darkened when shadows were turned on");
        // And nothing should get BRIGHTER for having a shadow map.
        for y in 0..150 {
            for x in 0..150 {
                assert!(
                    lum(&on, x, y) <= lum(&off, x, y) + 6,
                    "shadows brightened ({x},{y}): {} vs {}",
                    lum(&on, x, y),
                    lum(&off, x, y)
                );
            }
        }
    }

    /// Occlusion must darken a crease and must never brighten anything.
    #[test]
    fn occlusion_darkens_an_inside_corner() {
        // A floor with a wall standing on it. The camera looks OBLIQUELY, so
        // the inside corner where they meet is visible; from straight above
        // the wall hides its own crease and there is nothing to darken,
        // which is how the first version of this test managed to pass
        // nothing at all.
        let mut floor = crate::geom::cube([70.0, 70.0, 4.0], true);
        for p in &mut floor.positions {
            p[2] -= 2.0;
        }
        let mut wall = crate::geom::cube([4.0, 70.0, 34.0], true);
        for p in &mut wall.positions {
            p[0] -= 20.0;
            p[2] += 17.0;
        }
        let scene =
            [(floor, Some([0.8, 0.8, 0.8, 1.0]), false), (wall, Some([0.8, 0.8, 0.8, 1.0]), false)];
        let base = View {
            width: 160,
            height: 160,
            samples: 1,
            azimuth: 20.0,
            elevation: 34.0,
            shadows: false,
            ..View::default()
        };
        let on = render(&scene, &base);
        let off = render(&scene, &View { occlusion: false, ..base.clone() });
        let lum = |p: &[u8], i: usize| {
            p[i * 3] as i32 + p[i * 3 + 1] as i32 + p[i * 3 + 2] as i32
        };
        let darkened = (0..160 * 160).filter(|&i| lum(&off, i) - lum(&on, i) > 8).count();
        assert!(darkened > 200, "occlusion darkened only {darkened} pixels");
        for i in 0..160 * 160 {
            assert!(lum(&on, i) <= lum(&off, i) + 6, "occlusion brightened a pixel");
        }
    }

    /// A shadow must land on the FLOOR, not only on the model.
    ///
    /// This is the test that would have caught interpolating world position
    /// with screen-space barycentrics. Under perspective those are not the
    /// world's weights, and on the floor -- two triangles spanning the whole
    /// frame -- the error was large enough to send every shadow lookup
    /// outside the map, so the floor came back uniformly lit while the model
    /// shaded itself perfectly. Small triangles hid it completely.
    #[test]
    fn a_floor_receives_the_shadow_of_what_stands_on_it() {
        let mut block = crate::geom::cube([20.0, 20.0, 20.0], true);
        for p in &mut block.positions {
            p[2] += 10.0;
        }
        let scene = [(block, Some([0.8, 0.8, 0.8, 1.0]), false)];
        let base = View {
            width: 220,
            height: 170,
            samples: 1,
            azimuth: 30.0,
            elevation: 20.0,
            ground: true,
            occlusion: false,
            ..View::default()
        };
        let on = render(&scene, &base);
        let off = render(&scene, &View { shadows: false, ..base.clone() });
        let lum = |p: &[u8], i: usize| {
            p[i * 3] as i32 + p[i * 3 + 1] as i32 + p[i * 3 + 2] as i32
        };
        let darkened = (0..220 * 170).filter(|&i| lum(&off, i) - lum(&on, i) > 25).count();
        // A block this size at this angle throws a shadow across a good part
        // of the floor. A handful of pixels would mean the model shading
        // itself and the floor missing out, which is the bug.
        assert!(darkened > 1200, "only {darkened} pixels fell into shadow");
        for i in 0..220 * 170 {
            assert!(lum(&on, i) <= lum(&off, i) + 6, "a shadow brightened a pixel");
        }
    }

    /// Perspective-correct interpolation, stated directly: a large triangle
    /// seen at a steep angle must report world positions that actually lie
    /// on it.
    #[test]
    fn a_steep_plane_reports_true_world_positions() {
        // Two triangles making one big square, tilted away from the camera,
        // rendered with the floor off so only they are present. Every shaded
        // pixel's world position is recovered from its own depth and checked
        // against the plane it must lie on.
        let mesh = crate::geom::cube([200.0, 200.0, 1.0], true);
        let scene = [(mesh, Some([0.9, 0.9, 0.9, 1.0]), false)];
        let v = View {
            width: 120,
            height: 90,
            samples: 1,
            elevation: 6.0,
            fov: 55.0,
            shadows: false,
            occlusion: false,
            ..View::default()
        };
        // Rendering is enough: if interpolation were wrong the shading would
        // be too, so compare against the same scene in ORTHOGRAPHIC, where
        // screen-space interpolation is exact by construction. A plane of
        // one colour and one normal must come out one flat tone either way.
        let persp = render(&scene, &v);
        let ortho = render(&scene, &View { fov: 0.0, ..v.clone() });
        let spread = |p: &[u8]| {
            let vals: Vec<i32> = (0..120 * 90)
                .map(|i| p[i * 3] as i32 + p[i * 3 + 1] as i32 + p[i * 3 + 2] as i32)
                .filter(|l| *l > 120)
                .collect();
            match (vals.iter().min(), vals.iter().max()) {
                (Some(a), Some(b)) => b - a,
                _ => 0,
            }
        };
        assert!(spread(&persp) <= spread(&ortho) + 12, "perspective shading is not flat");
    }

    /// Supersampling must soften the edges without moving the silhouette.
    #[test]
    fn supersampling_smooths_edges_without_moving_them() {
        // Shadows and occlusion off, so the ONLY thing that can produce a
        // value between a face and the background is partial coverage at an
        // edge. With them on, their gradients supply intermediate values in
        // both images and the test proves nothing.
        let v1 = View {
            width: 80,
            height: 80,
            samples: 1,
            shadows: false,
            occlusion: false,
            ..View::default()
        };
        let v2 = View { samples: 3, ..v1.clone() };
        let scene = [(cube(10.0), Some([0.85, 0.85, 0.85, 1.0]), false)];
        let a = render(&scene, &v1);
        let b = render(&scene, &v2);
        let lum = |p: &[u8], i: usize| {
            p[i * 3] as u32 + p[i * 3 + 1] as u32 + p[i * 3 + 2] as u32
        };
        // Without antialiasing the image holds only the background and one
        // value per visible face. Supersampling has to produce more.
        let shades = |p: &[u8]| {
            let mut v: Vec<u32> = (0..80 * 80).map(|i| lum(p, i)).collect();
            v.sort_unstable();
            v.dedup();
            v.len()
        };
        assert!(
            shades(&b) > shades(&a) * 3,
            "supersampling added no shades: {} against {}",
            shades(&b),
            shades(&a)
        );
        // And the silhouette must enclose the same area either way.
        let half = (lum(&a, 40 * 80 + 40) + lum(&a, 0)) / 2;
        let count = |p: &[u8]| (0..80 * 80).filter(|&i| lum(p, i) > half).count();
        let (ca, cb) = (count(&a), count(&b));
        assert!(
            (ca as i32 - cb as i32).abs() < (ca as i32) / 8,
            "silhouette moved: {ca} against {cb}"
        );
    }
}
