//! 2D geometry: closed contours with even-odd fill, and an ear-clipping
//! triangulator (holes handled by bridging) that turns a region into the
//! flat z=0 fill mesh the viewer draws and the extrusion caps reuse.
//!
//! From scratch, zero dependencies. Preview-grade: it assumes reasonably
//! clean input (our primitives produce it) and resolves nested holes by
//! even-odd depth; it does not fully sanitize self-intersecting contours.

pub type Vec2 = [f64; 2];

/// A 2D region: one or more closed contours (first point NOT repeated).
/// Membership is even-odd — a point enclosed by an odd number of contours
/// is inside, so a contour nested in a solid is a hole and one nested in a
/// hole is an island.
#[derive(Debug, Clone, PartialEq)]
pub struct Poly2 {
    pub contours: Vec<Vec<Vec2>>,
}

impl Poly2 {
    pub fn new(contours: Vec<Vec<Vec2>>) -> Poly2 {
        Poly2 { contours }
    }

    pub fn is_empty(&self) -> bool {
        self.contours.iter().all(|c| c.len() < 3)
    }
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    [a[0] - b[0], a[1] - b[1]]
}

/// Twice the signed area of a contour (positive = CCW).
pub fn signed_area2(contour: &[Vec2]) -> f64 {
    let mut s = 0.0;
    for i in 0..contour.len() {
        let a = contour[i];
        let b = contour[(i + 1) % contour.len()];
        s += a[0] * b[1] - b[0] * a[1];
    }
    s
}

/// Is `p` inside contour `c`? (ray cast to +x, odd crossings = inside).
fn point_in_one(c: &[Vec2], p: Vec2) -> bool {
    let n = c.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (c[i], c[j]);
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let t = (p[1] - a[1]) / (b[1] - a[1]);
            if p[0] < a[0] + t * (b[0] - a[0]) {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Triangulate an even-odd region into (vertices, triangles). Contours are
/// grouped by nesting depth: even-depth contours are solid outlines, the
/// odd-depth contours directly inside them are holes. Each solid + its
/// holes is bridged into one simple polygon and ear-clipped.
pub fn triangulate(poly: &Poly2) -> (Vec<Vec2>, Vec<[u32; 3]>) {
    let contours: Vec<&Vec<Vec2>> = poly.contours.iter().filter(|c| c.len() >= 3).collect();
    if contours.is_empty() {
        return (Vec::new(), Vec::new());
    }
    // Nesting depth of each contour = how many OTHER contours contain a
    // representative point of it.
    let depth: Vec<usize> = contours
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let p = c[0];
            contours
                .iter()
                .enumerate()
                .filter(|&(j, other)| j != i && point_in_one(other, p))
                .count()
        })
        .collect();

    let mut positions: Vec<Vec2> = Vec::new();
    let mut tris: Vec<[u32; 3]> = Vec::new();
    // Each even-depth contour is a solid; its holes are the odd-depth
    // contours nested exactly one level deeper inside it.
    for (i, outer) in contours.iter().enumerate() {
        if depth[i] % 2 != 0 {
            continue; // a hole (or island's hole) — handled by its parent
        }
        let holes: Vec<Vec<Vec2>> = contours
            .iter()
            .enumerate()
            .filter(|&(j, h)| {
                depth[j] == depth[i] + 1 && point_in_one(outer, h[0]) && j != i
            })
            .map(|(_, h)| (*h).clone())
            .collect();
        let merged = bridge_holes(ccw((*outer).clone()), holes);
        let base = positions.len() as u32;
        positions.extend_from_slice(&merged);
        for t in ear_clip(&merged) {
            tris.push([base + t[0], base + t[1], base + t[2]]);
        }
    }
    (positions, tris)
}

/// Ensure CCW orientation (positive area).
fn ccw(mut c: Vec<Vec2>) -> Vec<Vec2> {
    if signed_area2(&c) < 0.0 {
        c.reverse();
    }
    c
}

/// Merge holes into an outer contour by cutting a bridge from each hole's
/// rightmost vertex to a visible outer vertex, producing one weakly-simple
/// CCW polygon. Holes are inserted CW (reversed) so the bridge seams close.
fn bridge_holes(outer: Vec<Vec2>, mut holes: Vec<Vec<Vec2>>) -> Vec<Vec2> {
    if holes.is_empty() {
        return outer;
    }
    // Process holes right-to-left by their rightmost vertex, so earlier
    // bridges don't block later ones.
    holes.sort_by(|a, b| rightmost(b).partial_cmp(&rightmost(a)).unwrap_or(std::cmp::Ordering::Equal));
    let mut poly = outer;
    for hole in holes {
        let hole = cw(hole); // holes wind opposite the outer
        let hi = rightmost_index(&hole);
        // Connect the hole's rightmost vertex to the nearest outer vertex
        // (a cheap, robust-enough visibility proxy for preview fill).
        let hp = hole[hi];
        let oi = (0..poly.len())
            .min_by(|&a, &b| {
                dist2(poly[a], hp).partial_cmp(&dist2(poly[b], hp)).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        // Splice: outer[0..=oi], hole[hi..]+hole[..=hi], outer[oi..].
        let mut merged = Vec::with_capacity(poly.len() + hole.len() + 2);
        merged.extend_from_slice(&poly[..=oi]);
        for k in 0..=hole.len() {
            merged.push(hole[(hi + k) % hole.len()]);
        }
        merged.extend_from_slice(&poly[oi..]);
        poly = merged;
    }
    poly
}

fn cw(c: Vec<Vec2>) -> Vec<Vec2> {
    let mut c = c;
    if signed_area2(&c) > 0.0 {
        c.reverse();
    }
    c
}

fn rightmost(c: &[Vec2]) -> f64 {
    c.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max)
}

fn rightmost_index(c: &[Vec2]) -> usize {
    (0..c.len()).max_by(|&a, &b| c[a][0].partial_cmp(&c[b][0]).unwrap_or(std::cmp::Ordering::Equal)).unwrap()
}

fn dist2(a: Vec2, b: Vec2) -> f64 {
    let d = sub(a, b);
    d[0] * d[0] + d[1] * d[1]
}

fn cross(o: Vec2, a: Vec2, b: Vec2) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

/// Ear-clipping triangulation of a CCW simple polygon; returns triangles
/// as index triples into the input.
fn ear_clip(poly: &[Vec2]) -> Vec<[u32; 3]> {
    let n = poly.len();
    let mut tris = Vec::new();
    if n < 3 {
        return tris;
    }
    let mut idx: Vec<usize> = (0..n).collect();
    let mut guard = 0;
    while idx.len() > 3 {
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            let ia = idx[(i + m - 1) % m];
            let ib = idx[i];
            let ic = idx[(i + 1) % m];
            let (a, b, c) = (poly[ia], poly[ib], poly[ic]);
            // Convex corner?
            if cross(a, b, c) <= 0.0 {
                continue;
            }
            // No other vertex inside triangle abc? Skip vertices that
            // COINCIDE with a corner — hole bridging duplicates a vertex
            // position, and a point on the boundary would otherwise
            // falsely block every ear at the seam.
            let mut ok = true;
            for &j in &idx {
                if j == ia || j == ib || j == ic {
                    continue;
                }
                let pj = poly[j];
                if pj == a || pj == b || pj == c {
                    continue;
                }
                if in_triangle(pj, a, b, c) {
                    ok = false;
                    break;
                }
            }
            if ok {
                tris.push([ia as u32, ib as u32, ic as u32]);
                idx.remove(i);
                clipped = true;
                break;
            }
        }
        guard += 1;
        if !clipped || guard > n + 2 {
            break; // degenerate / self-intersecting — stop gracefully
        }
    }
    if idx.len() == 3 {
        tris.push([idx[0] as u32, idx[1] as u32, idx[2] as u32]);
    }
    tris
}

fn in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let d1 = cross(a, b, p);
    let d2 = cross(b, c, p);
    let d3 = cross(c, a, p);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}

// -- Primitives -------------------------------------------------------------

/// square([x, y], center): one CCW contour, occupying [0,x]×[0,y] or
/// centered. Zero/negative/non-finite size → empty.
pub fn square(size: Vec2, center: bool) -> Poly2 {
    if !(size[0] > 0.0) || !(size[1] > 0.0) {
        return Poly2::new(Vec::new());
    }
    let (ox, oy) = if center { (-size[0] / 2.0, -size[1] / 2.0) } else { (0.0, 0.0) };
    let (ex, ey) = (ox + size[0], oy + size[1]);
    Poly2::new(vec![vec![[ox, oy], [ex, oy], [ex, ey], [ox, ey]]])
}

/// circle(r, n): an n-gon inscribed in radius r, vertex 0 at (r,0), CCW.
pub fn circle(r: f64, n: u32) -> Poly2 {
    if !(r > 0.0) || n < 3 {
        return Poly2::new(Vec::new());
    }
    let contour = (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            [r * a.cos(), r * a.sin()]
        })
        .collect();
    Poly2::new(vec![contour])
}

/// polygon(points, paths): contours addressed by index. With no paths the
/// single points list is the sole contour. Out-of-range indices drop the
/// offending path; a path with fewer than 3 points is dropped.
pub fn polygon(points: &[Vec2], paths: Option<&[Vec<usize>]>) -> (Poly2, Vec<String>) {
    let mut warnings = Vec::new();
    let contours = match paths {
        None => {
            if points.len() >= 3 {
                vec![points.to_vec()]
            } else {
                Vec::new()
            }
        }
        Some(paths) => {
            let mut out = Vec::new();
            for path in paths {
                if let Some(&bad) = path.iter().find(|&&i| i >= points.len()) {
                    warnings.push(format!("polygon: point index {} out of range; path dropped", bad));
                    continue;
                }
                if path.len() < 3 {
                    continue;
                }
                out.push(path.iter().map(|&i| points[i]).collect());
            }
            out
        }
    };
    (Poly2::new(contours), warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(tris: &[[u32; 3]], v: &[Vec2]) -> f64 {
        tris.iter()
            .map(|t| {
                cross(v[t[0] as usize], v[t[1] as usize], v[t[2] as usize]).abs() / 2.0
            })
            .sum()
    }

    #[test]
    fn square_and_circle_have_the_right_area() {
        let (v, t) = triangulate(&square([4.0, 3.0], false));
        assert!((area(&t, &v) - 12.0).abs() < 1e-9);
        // A 64-gon of radius 5 is close to π·25.
        let (v, t) = triangulate(&circle(5.0, 64));
        assert!((area(&t, &v) - std::f64::consts::PI * 25.0).abs() < 0.2);
        // circle vertex 0 is exactly (r, 0).
        assert_eq!(circle(5.0, 8).contours[0][0], [5.0, 0.0]);
    }

    #[test]
    fn concave_polygon_triangulates_to_its_area() {
        // An L-shape (concave): area = 2x2 square minus a 1x1 bite = 3.
        let l = Poly2::new(vec![vec![
            [0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0], [1.0, 2.0], [0.0, 2.0],
        ]]);
        let (v, t) = triangulate(&l);
        assert!((area(&t, &v) - 3.0).abs() < 1e-9, "area {}", area(&t, &v));
    }

    #[test]
    fn hole_is_subtracted_by_even_odd_nesting() {
        // A 10×10 square with a 4×4 square hole centered inside: area 84.
        let outer = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let hole = vec![[3.0, 3.0], [7.0, 3.0], [7.0, 7.0], [3.0, 7.0]];
        let (v, t) = triangulate(&Poly2::new(vec![outer, hole]));
        assert!((area(&t, &v) - 84.0).abs() < 1e-6, "area {}", area(&t, &v));
    }

    #[test]
    fn degenerate_and_empty_inputs_are_safe() {
        assert!(square([0.0, 5.0], false).is_empty());
        assert!(circle(-1.0, 20).is_empty());
        let (p, _) = polygon(&[[0.0, 0.0], [1.0, 0.0]], None); // < 3 points
        assert!(p.is_empty());
        let (v, t) = triangulate(&Poly2::new(Vec::new()));
        assert!(v.is_empty() && t.is_empty());
    }
}
