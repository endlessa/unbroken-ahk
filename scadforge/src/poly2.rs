//! 2D geometry: closed contours with even-odd fill, and an ear-clipping
//! triangulator (holes handled by bridging) that turns a region into the
//! flat z=0 fill mesh the viewer draws and the extrusion caps reuse.
//!
//! From scratch, zero dependencies. Preview-grade: it assumes reasonably
//! clean input (our primitives produce it) and resolves nested holes by
//! even-odd depth; it does not fully sanitize self-intersecting contours.

pub type Vec2 = [f64; 2];

/// How a contour set decides what is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    /// OpenSCAD's own rule, and the one every 2D boolean result comes back
    /// in: a point covered by an odd number of contours is inside, so a path
    /// inside an outline is a hole and one inside a hole is an island.
    EvenOdd,
    /// The rule a typeface is drawn under. A contour wound WITH the outline
    /// direction is solid and one wound against it is a counter, which lets
    /// a glyph be drawn as several OVERLAPPING same-direction strokes --
    /// this face draws 'B' as a stem rectangle laid across a bowl path and
    /// 'A' as three crossing bars. Even-odd punches a hole through every one
    /// of those overlaps; the reference says glyph outlines are unioned and
    /// counters follow the font's winding, which is this.
    Font,
}

/// A 2D region: one or more closed contours (first point NOT repeated),
/// filled by `fill`.
#[derive(Debug, Clone, PartialEq)]
pub struct Poly2 {
    pub contours: Vec<Vec<Vec2>>,
    pub fill: Fill,
}

impl Poly2 {
    /// An even-odd region — primitives, `polygon()`, and every boolean result.
    pub fn new(contours: Vec<Vec<Vec2>>) -> Poly2 {
        Poly2 { contours, fill: Fill::EvenOdd }
    }

    /// A region of raw glyph outlines, filled by the font's winding.
    pub fn new_font(contours: Vec<Vec<Vec2>>) -> Poly2 {
        Poly2 { contours, fill: Fill::Font }
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

/// Drop zero-length edges. A TrueType contour closes by repeating its first
/// point and flattening a curve can land two samples on top of each other;
/// left in, each repeat costs a wall quad with no area and no usable normal
/// (the viewer's per-face normal is a cross product, so a collapsed face
/// shades as a dark sliver) and hands the triangulator a phantom corner.
fn dedup_closed(c: &[Vec2]) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::with_capacity(c.len());
    for &p in c {
        if out.last().map_or(true, |&q| q != p) {
            out.push(p);
        }
    }
    while out.len() > 1 && out[0] == out[out.len() - 1] {
        out.pop();
    }
    out
}

/// The region's contours, deduplicated and with the unusable ones dropped.
fn clean_contours(poly: &Poly2) -> Vec<Vec<Vec2>> {
    poly.contours
        .iter()
        .filter(|c| c.len() >= 3)
        .map(|c| dedup_closed(c))
        .filter(|c| c.len() >= 3)
        .collect()
}

/// A point strictly INSIDE a simple contour, for containment tests.
///
/// Using a vertex instead (which is what this used to do) decides nothing
/// the moment two contours touch, and in a font they touch constantly: 'B'
/// is a bowl path whose first point lands on the stem rectangle laid over
/// it, so the bowl tested as nested in the stem and the letter filled inside
/// out. The rightmost vertex of any simple contour is a convex corner, so a
/// short step along its interior bisector is inside by construction.
fn interior_point(c: &[Vec2]) -> Vec2 {
    let n = c.len();
    let k = (1..n).fold(0usize, |best, i| {
        if c[i][0] > c[best][0] || (c[i][0] == c[best][0] && c[i][1] < c[best][1]) {
            i
        } else {
            best
        }
    });
    let v = c[k];
    let unit = |d: Vec2| {
        let l = (d[0] * d[0] + d[1] * d[1]).sqrt();
        if l > 0.0 { ([d[0] / l, d[1] / l], l) } else { ([0.0, 0.0], 0.0) }
    };
    let (u1, l1) = unit(sub(c[(k + n - 1) % n], v));
    let (u2, l2) = unit(sub(c[(k + 1) % n], v));
    // The bisector of the wedge between the two edges points into the wedge,
    // and at a convex corner the wedge IS the interior — no orientation
    // needed. Straight-through corners fall back to the edge normal on the
    // filled side, which the signed area picks out.
    let mut dir = [u1[0] + u2[0], u1[1] + u2[1]];
    if dir[0] * dir[0] + dir[1] * dir[1] < 1e-18 {
        let turn = if signed_area2(c) >= 0.0 { 1.0 } else { -1.0 };
        dir = [-u2[1] * turn, u2[0] * turn];
    }
    let (dir, _) = unit(dir);
    let reach = if l1 > 0.0 && l2 > 0.0 { l1.min(l2) } else { l1.max(l2) };
    for f in [1e-3, 1e-2, 1e-4, 1e-6] {
        let p = [v[0] + dir[0] * reach * f, v[1] + dir[1] * reach * f];
        if point_in_one(c, p) {
            return p;
        }
    }
    // Last resort for a contour too degenerate for the bisector: a short
    // scan of skip-one chords, then simply the first vertex.
    for i in 0..n {
        let q = c[(i + 2) % n];
        let p = [(c[i][0] + q[0]) / 2.0, (c[i][1] + q[1]) / 2.0];
        if point_in_one(c, p) {
            return p;
        }
    }
    c[0]
}

/// Group even-odd contours into (solid, its holes). Indices are into
/// `clean`; every contour lands in exactly one group, either as the solid
/// or as one solid's hole. The font rule needs none of this — its fill and
/// its walls both read the winding directly, which is the only thing that
/// survives an outline crossing itself.
fn groups(clean: &[Vec<Vec2>]) -> Vec<(usize, Vec<usize>)> {
    let n = clean.len();
    let inner: Vec<Vec2> = clean.iter().map(|c| interior_point(c)).collect();
    let inside = |i: usize, j: usize| i != j && point_in_one(&clean[j], inner[i]);
    // Nesting depth: how many OTHER contours contain this one.
    let depth: Vec<usize> = (0..n).map(|i| (0..n).filter(|&j| inside(i, j)).count()).collect();
    (0..n)
        .filter(|&i| depth[i] % 2 == 0)
        .map(|i| {
            let holes = (0..n).filter(|&j| depth[j] == depth[i] + 1 && inside(j, i)).collect();
            (i, holes)
        })
        .collect()
}

/// Group contours that could possibly interact, by transitive overlap of
/// their bounding boxes. Winding, crossings and burial are all local, so a
/// line of text is a row of independent letters — without this the sweep
/// and the wall split both compare every edge in the string against every
/// other, which is quadratic in the length of the text rather than in the
/// size of one glyph.
fn components(contours: &[Vec<Vec2>]) -> Vec<Vec<usize>> {
    let n = contours.len();
    let bb: Vec<[f64; 4]> = contours
        .iter()
        .map(|c| {
            let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
            for p in c {
                b[0] = b[0].min(p[0]);
                b[1] = b[1].min(p[1]);
                b[2] = b[2].max(p[0]);
                b[3] = b[3].max(p[1]);
            }
            b
        })
        .collect();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut Vec<usize>, mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            if bb[i][0] > bb[j][2] || bb[i][2] < bb[j][0] || bb[i][1] > bb[j][3] || bb[i][3] < bb[j][1]
            {
                continue;
            }
            let (a, b) = (find(&mut parent, i), find(&mut parent, j));
            if a != b {
                parent[a] = b;
            }
        }
    }
    let mut by_root: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    let mut order: Vec<usize> = Vec::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        if !by_root.contains_key(&r) {
            order.push(r);
        }
        by_root.entry(r).or_default().push(i);
    }
    order.into_iter().filter_map(|r| by_root.remove(&r)).collect()
}

// -- Non-zero sweep fill ----------------------------------------------------

/// One non-horizontal edge, normalised to run upward, remembering which way
/// the outline actually crossed it (+1 up, -1 down) so the sweep can count
/// winding rather than just crossings.
#[derive(Clone, Copy)]
struct SwEdge {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    dir: i32,
}

impl SwEdge {
    fn x_at(&self, y: f64) -> f64 {
        let dy = self.y1 - self.y0;
        if dy <= 0.0 {
            self.x0
        } else {
            self.x0 + (y - self.y0) * (self.x1 - self.x0) / dy
        }
    }
}

/// y of a proper crossing between two edges, if they cross strictly between
/// their endpoints. Touching at an endpoint is already a scanline (every
/// vertex is one), so only interior crossings need reporting.
fn crossing_y(a: &SwEdge, b: &SwEdge) -> Option<f64> {
    let (px, py) = (a.x0, a.y0);
    let (rx, ry) = (a.x1 - a.x0, a.y1 - a.y0);
    let (qx, qy) = (b.x0, b.y0);
    let (sx, sy) = (b.x1 - b.x0, b.y1 - b.y0);
    let d = rx * sy - ry * sx;
    if d == 0.0 {
        return None; // parallel or collinear: no new scanline
    }
    let (wx, wy) = (qx - px, qy - py);
    let t = (wx * sy - wy * sx) / d;
    let u = (wx * ry - wy * rx) / d;
    if t > 0.0 && t < 1.0 && u > 0.0 && u < 1.0 {
        Some(py + t * ry)
    } else {
        None
    }
}

/// Fill a contour set by the NON-ZERO winding rule, with a horizontal sweep.
///
/// This is the only way to fill an outline that crosses itself, and glyph
/// outlines cross themselves constantly: this face draws '4' as a single
/// 14-point path that crosses itself six times, leaving the counter as the
/// one pocket the winding cancels in, and the 'e' the same way. Ear clipping
/// asks a local question — is this corner an ear — and a local question
/// cannot see a crossing on the far side of the glyph, so it fills those
/// counters solid.
///
/// Every vertex y and every edge crossing is a scanline. Between two
/// consecutive scanlines no edge begins, ends or crosses another, so the
/// band is cut cleanly into trapezoids by the edges spanning it sorted by
/// x, and the running winding says which gaps are ink.
fn sweep_fill(contours: &[Vec<Vec2>]) -> (Vec<Vec2>, Vec<[u32; 3]>) {
    let mut positions: Vec<Vec2> = Vec::new();
    let mut tris: Vec<[u32; 3]> = Vec::new();
    for group in components(contours) {
        let part: Vec<Vec<Vec2>> = group.into_iter().map(|i| contours[i].clone()).collect();
        let (p, t) = sweep_component(&part);
        let base = positions.len() as u32;
        positions.extend_from_slice(&p);
        tris.extend(t.into_iter().map(|x| [base + x[0], base + x[1], base + x[2]]));
    }
    (positions, tris)
}

fn sweep_component(contours: &[Vec<Vec2>]) -> (Vec<Vec2>, Vec<[u32; 3]>) {
    let mut edges: Vec<SwEdge> = Vec::new();
    let (mut ylo, mut yhi) = (f64::INFINITY, f64::NEG_INFINITY);
    for c in contours {
        let n = c.len();
        for i in 0..n {
            let (a, b) = (c[i], c[(i + 1) % n]);
            ylo = ylo.min(a[1]);
            yhi = yhi.max(a[1]);
            if a[1] == b[1] {
                continue; // horizontal edges cross no scanline
            }
            edges.push(if a[1] < b[1] {
                SwEdge { x0: a[0], y0: a[1], x1: b[0], y1: b[1], dir: 1 }
            } else {
                SwEdge { x0: b[0], y0: b[1], x1: a[0], y1: a[1], dir: -1 }
            });
        }
    }
    if edges.is_empty() || !(yhi > ylo) {
        return (Vec::new(), Vec::new());
    }
    let span = yhi - ylo;
    let tol = span * 1e-9;

    let mut ys: Vec<f64> = Vec::with_capacity(edges.len() * 2);
    for c in contours {
        for p in c {
            ys.push(p[1]);
        }
    }
    for i in 0..edges.len() {
        for j in i + 1..edges.len() {
            if let Some(y) = crossing_y(&edges[i], &edges[j]) {
                ys.push(y);
            }
        }
    }
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    ys.dedup_by(|a, b| (*a - *b).abs() <= tol);

    let mut positions: Vec<Vec2> = Vec::new();
    let mut tris: Vec<[u32; 3]> = Vec::new();
    let mut act: Vec<(f64, usize)> = Vec::new();
    for w in ys.windows(2) {
        let (ya, yb) = (w[0], w[1]);
        if yb - ya <= tol {
            continue;
        }
        let ym = 0.5 * (ya + yb);
        act.clear();
        for (i, e) in edges.iter().enumerate() {
            if e.y0 <= ya + tol && e.y1 >= yb - tol {
                act.push((e.x_at(ym), i));
            }
        }
        act.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut wind = 0i32;
        for k in 0..act.len().saturating_sub(1) {
            wind += edges[act[k].1].dir;
            if wind == 0 {
                continue; // a gap in the ink
            }
            let (l, r) = (&edges[act[k].1], &edges[act[k + 1].1]);
            // CCW: right along the bottom, up the right edge, left along the
            // top, down the left edge.
            let quad = [
                [l.x_at(ya), ya],
                [r.x_at(ya), ya],
                [r.x_at(yb), yb],
                [l.x_at(yb), yb],
            ];
            let base = positions.len() as u32;
            let mut used = false;
            for t in [[0usize, 1, 2], [0, 2, 3]] {
                let (a, b, c) = (quad[t[0]], quad[t[1]], quad[t[2]]);
                if cross(a, b, c) > 0.0 {
                    if !used {
                        positions.extend_from_slice(&quad);
                        used = true;
                    }
                    tris.push([base + t[0] as u32, base + t[1] as u32, base + t[2] as u32]);
                }
            }
        }
    }
    (positions, tris)
}

/// Triangulate a region into (vertices, triangles). An even-odd region
/// bridges each solid contour to its holes into one weakly-simple polygon
/// and ear-clips it; a font region goes to the non-zero sweep instead,
/// which is what its self-crossing outlines need.
pub fn triangulate(poly: &Poly2) -> (Vec<Vec2>, Vec<[u32; 3]>) {
    let clean = clean_contours(poly);
    if clean.is_empty() {
        return (Vec::new(), Vec::new());
    }
    if poly.fill == Fill::Font {
        return sweep_fill(&clean);
    }
    // The nesting classifier below cannot describe contours that CROSS, so
    // they are resolved into equivalent nested ones first. A region that
    // does not cross comes back unchanged.
    let resolved;
    let clean = if has_crossing(&clean) {
        resolved = sanitize(poly);
        clean_contours(&resolved)
    } else {
        clean
    };
    let mut positions: Vec<Vec2> = Vec::new();
    let mut tris: Vec<[u32; 3]> = Vec::new();
    for (i, holes) in groups(&clean) {
        let hs: Vec<Vec<Vec2>> = holes.iter().map(|&j| clean[j].clone()).collect();
        let merged = bridge_holes(ccw(clean[i].clone()), hs);
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
        let hp = hole[hi];
        let oi = visible_index(&poly, hp);
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

/// Index of a vertex of CCW `poly` that the hole vertex `m` can see.
///
/// Shoot +x from `m`, take the edge the ray first hits, and use that edge's
/// farther-right endpoint P — unless a reflex vertex of the outline falls
/// inside triangle (m, I, P), in which case the reflex vertex at the
/// smallest angle off the ray is the one in the way and the one to bridge
/// to. Picking the NEAREST vertex instead (what this did) cuts the bridge
/// straight through the outline whenever the nearest vertex is across a
/// bowl from the counter, and the ear clipper then triangulates a polygon
/// that crosses itself: that is the flap over the 'G' and the cross-hatch
/// through the counters of '8'.
fn visible_index(poly: &[Vec2], m: Vec2) -> usize {
    let n = poly.len();
    let mut best_x = f64::INFINITY;
    let mut hit: Option<usize> = None; // index of the edge's far-right end
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        // Half-open straddle, so a vertex exactly at m's height counts once.
        if (a[1] > m[1]) == (b[1] > m[1]) {
            continue;
        }
        let t = (m[1] - a[1]) / (b[1] - a[1]);
        let x = a[0] + t * (b[0] - a[0]);
        if x >= m[0] && x < best_x {
            best_x = x;
            hit = Some(if a[0] > b[0] { i } else { (i + 1) % n });
        }
    }
    let p = match hit {
        Some(p) => p,
        // The hole is not enclosed (shouldn't happen once classified), so
        // fall back to the nearest vertex rather than failing to bridge.
        None => {
            return (0..n)
                .min_by(|&a, &b| {
                    dist2(poly[a], m).partial_cmp(&dist2(poly[b], m)).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap()
        }
    };
    let ray = [best_x, m[1]];
    if dist2(ray, poly[p]) <= 1e-18 {
        return p; // the ray ran straight into a vertex
    }
    // Reflex vertices inside (m, I, P) block the straight bridge to P.
    let mut chosen = p;
    let mut best_cos = f64::NEG_INFINITY;
    let mut best_d = f64::INFINITY;
    for i in 0..n {
        if i == p {
            continue;
        }
        let v = poly[i];
        let (a, c) = (poly[(i + n - 1) % n], poly[(i + 1) % n]);
        if cross(a, v, c) > 0.0 {
            continue; // convex, cannot block
        }
        if !in_triangle(v, m, ray, poly[p]) {
            continue;
        }
        let d = sub(v, m);
        let l = (d[0] * d[0] + d[1] * d[1]).sqrt();
        if l <= 0.0 {
            continue;
        }
        let cos = d[0] / l;
        if cos > best_cos || (cos == best_cos && l < best_d) {
            best_cos = cos;
            best_d = l;
            chosen = i;
        }
    }
    chosen
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

/// Ear-clipping triangulation of a CCW polygon; returns triangles as index
/// triples into the input.
///
/// Every pass removes a vertex, so the polygon is always fully consumed.
/// The old loop abandoned the rest of the ring the first time no clean ear
/// turned up, which on a bridged glyph left a facet simply missing — a hole
/// straight through the letter with the far cap showing through it.
fn ear_clip(poly: &[Vec2]) -> Vec<[u32; 3]> {
    let n = poly.len();
    let mut tris = Vec::new();
    if n < 3 {
        return tris;
    }
    // Area tolerance relative to the outline's own size: glyphs arrive in em
    // units in the hundreds, where a fixed epsilon either misses slivers or
    // eats real corners.
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for p in poly {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-12);
    let eps = span * span * 1e-13;

    let mut idx: Vec<usize> = (0..n).collect();
    while idx.len() > 3 {
        let m = idx.len();
        let corner = |i: usize| {
            let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            (ia, ib, ic, poly[ia], poly[ib], poly[ic])
        };
        // 1. A corner with no area is noise, not an ear: drop the vertex and
        //    emit nothing. Flattened curves leave collinear runs all along a
        //    straight stem, and a bridge seam doubles a vertex outright.
        if let Some(i) = (0..m).find(|&i| {
            let (_, _, _, a, b, c) = corner(i);
            cross(a, b, c).abs() <= eps
        }) {
            idx.remove(i);
            continue;
        }
        // 2. A true ear: convex, with no other vertex of the ring inside it.
        //    Vertices that COINCIDE with a corner are skipped — a bridge
        //    seam duplicates a position, and a point on the boundary would
        //    otherwise block every ear along the seam.
        let ear = (0..m).find(|&i| {
            let (ia, ib, ic, a, b, c) = corner(i);
            if cross(a, b, c) <= eps {
                return false;
            }
            !idx.iter().any(|&j| {
                if j == ia || j == ib || j == ic {
                    return false;
                }
                let pj = poly[j];
                pj != a && pj != b && pj != c && in_triangle(pj, a, b, c)
            })
        });
        if let Some(i) = ear {
            let (ia, ib, ic, ..) = corner(i);
            tris.push([ia as u32, ib as u32, ic as u32]);
            idx.remove(i);
            continue;
        }
        // 3. No clean ear — the ring touches itself, which a bridge seam can
        //    genuinely produce. Take the widest convex corner anyway: a
        //    slightly overlapping triangle is invisible on a flat cap, where
        //    walking away leaves a hole that is not.
        let mut pick = 0usize;
        let mut best = f64::NEG_INFINITY;
        for i in 0..m {
            let (_, _, _, a, b, c) = corner(i);
            let v = cross(a, b, c);
            if v > best {
                best = v;
                pick = i;
            }
        }
        if best > eps {
            let (ia, ib, ic, ..) = corner(pick);
            tris.push([ia as u32, ib as u32, ic as u32]);
        }
        idx.remove(pick);
    }
    if idx.len() == 3 {
        let (a, b, c) = (poly[idx[0]], poly[idx[1]], poly[idx[2]]);
        if cross(a, b, c).abs() > eps {
            tris.push([idx[0] as u32, idx[1] as u32, idx[2] as u32]);
        }
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

// -- Extrusions (2D → 3D) ---------------------------------------------------

type Mesh3 = (Vec<[f64; 3]>, Vec<[u32; 3]>);

/// Contours re-wound so the filled region is always on the LEFT of each
/// directed edge: a solid outline → CCW, one of its holes → CW. Extrusion
/// walls built from these face outward on solids and inward on holes, and
/// the 2D boolean kernel reads the same orientation to build its boundary
/// segments. Grouping follows the region's own fill rule, so a font's
/// overlapping strokes stay solid instead of cancelling each other.
pub fn oriented_contours(poly: &Poly2) -> Vec<Vec<Vec2>> {
    let clean = clean_contours(poly);
    if clean.is_empty() {
        return Vec::new();
    }
    if poly.fill == Fill::Font {
        // The whole outline set shares one drawing direction, and under the
        // non-zero rule that direction already puts the ink on one side of
        // every edge -- counters included, and self-crossing paths too,
        // where a per-contour nesting guess has nothing to hold on to.
        // TrueType runs outer contours clockwise, so the set is reversed
        // wholesale when its net area says the ink is on the right.
        let total: f64 = clean.iter().map(|c| signed_area2(c)).sum();
        return clean
            .into_iter()
            .map(|c| {
                if total < 0.0 {
                    c.into_iter().rev().collect()
                } else {
                    c
                }
            })
            .collect();
    }
    let mut out: Vec<Option<Vec<Vec2>>> = vec![None; clean.len()];
    for (i, holes) in groups(&clean) {
        out[i] = Some(ccw(clean[i].clone()));
        for j in holes {
            out[j] = Some(cw(clean[j].clone()));
        }
    }
    // Grouping claims every contour, but never emit fewer walls than there
    // are outlines: an unclaimed one keeps the direction it arrived in.
    out.into_iter()
        .enumerate()
        .map(|(i, c)| c.unwrap_or_else(|| clean[i].clone()))
        .collect()
}

/// Parameter of a proper crossing of segment `e` by segment `o`, plus the
/// parameters of `o`'s endpoints where they land in the middle of `e` (one
/// stroke's corner meeting another's flank).
fn split_params(e: &[Vec2; 2], o: &[Vec2; 2], tol: f64, out: &mut Vec<f64>) {
    let (rx, ry) = (e[1][0] - e[0][0], e[1][1] - e[0][1]);
    let (sx, sy) = (o[1][0] - o[0][0], o[1][1] - o[0][1]);
    let d = rx * sy - ry * sx;
    let (wx, wy) = (o[0][0] - e[0][0], o[0][1] - e[0][1]);
    if d != 0.0 {
        let t = (wx * sy - wy * sx) / d;
        let u = (wx * ry - wy * rx) / d;
        if t > 0.0 && t < 1.0 && u > 0.0 && u < 1.0 {
            out.push(t);
        }
    }
    let l2 = rx * rx + ry * ry;
    if l2 <= 0.0 {
        return;
    }
    for q in o {
        let t = ((q[0] - e[0][0]) * rx + (q[1] - e[0][1]) * ry) / l2;
        if t <= 0.0 || t >= 1.0 {
            continue;
        }
        let (px, py) = (e[0][0] + t * rx - q[0], e[0][1] + t * ry - q[1]);
        if px * px + py * py <= tol * tol {
            out.push(t);
        }
    }
}

/// Is `p` in the filled region, under `fill`?
fn filled_at(clean: &[Vec<Vec2>], fill: Fill, p: Vec2) -> bool {
    match fill {
        // "Covered by an odd number of boundaries is inside" — and a single
        // contour that crosses itself already answers even-odd for its own
        // lobes, because point_in_one counts ray crossings.
        Fill::EvenOdd => clean.iter().filter(|c| point_in_one(c, p)).count() % 2 == 1,
        Fill::Font => {
            let mut w = 0;
            for c in clean {
                let n = c.len();
                for i in 0..n {
                    let (a, b) = (c[i], c[(i + 1) % n]);
                    if a[1] <= p[1] {
                        if b[1] > p[1] && cross(a, b, p) > 0.0 {
                            w += 1;
                        }
                    } else if b[1] <= p[1] && cross(a, b, p) < 0.0 {
                        w -= 1;
                    }
                }
            }
            w != 0
        }
    }
}

/// Do two segments cross strictly inside both? Endpoint touches don't count
/// — a contour nested in another may touch it, and nesting handles that
/// perfectly well. A genuine crossing is the thing nesting cannot describe.
fn seg_cross(p: &[Vec2; 2], q: &[Vec2; 2]) -> bool {
    let o = |a: Vec2, b: Vec2, c: Vec2| {
        let v = cross(a, b, c);
        if v > 0.0 {
            1
        } else if v < 0.0 {
            -1
        } else {
            0
        }
    };
    let (a, b) = (o(p[0], p[1], q[0]), o(p[0], p[1], q[1]));
    let (c, d) = (o(q[0], q[1], p[0]), o(q[0], q[1], p[1]));
    a != 0 && b != 0 && c != 0 && d != 0 && a != b && c != d
}

/// Does any edge of the region cross any other? A sweep ordered by the
/// edges' lower y, so the common clean region costs about a sort rather
/// than every pair.
fn has_crossing(clean: &[Vec<Vec2>]) -> bool {
    let mut segs: Vec<[Vec2; 2]> = Vec::new();
    for c in clean {
        let n = c.len();
        for i in 0..n {
            segs.push([c[i], c[(i + 1) % n]]);
        }
    }
    let ylo = |s: &[Vec2; 2]| s[0][1].min(s[1][1]);
    let yhi = |s: &[Vec2; 2]| s[0][1].max(s[1][1]);
    let mut order: Vec<usize> = (0..segs.len()).collect();
    order.sort_by(|&a, &b| {
        ylo(&segs[a]).partial_cmp(&ylo(&segs[b])).unwrap_or(std::cmp::Ordering::Equal)
    });
    for (k, &i) in order.iter().enumerate() {
        let top = yhi(&segs[i]);
        for &j in &order[k + 1..] {
            if ylo(&segs[j]) > top {
                break; // everything later starts above this edge
            }
            if segs[i][0][0].min(segs[i][1][0]) > segs[j][0][0].max(segs[j][1][0])
                || segs[i][0][0].max(segs[i][1][0]) < segs[j][0][0].min(segs[j][1][0])
            {
                continue;
            }
            if seg_cross(&segs[i], &segs[j]) {
                return true;
            }
        }
    }
    false
}

/// The region's true boundary as directed segments, ink always on the LEFT.
///
/// Every edge is cut at its crossings with every other, and each piece is
/// kept only where the fill actually CHANGES across it — which settles both
/// the question of whether it is boundary at all and which way round it
/// goes, without ever asking whether one contour is nested in another. That
/// matters because nesting is not a question a crossing has an answer to:
/// of two sibling squares laid half across each other, neither is inside
/// the other, and of a path that crosses itself there are not two contours
/// to compare. Glyphs are the extreme case — 'B' is a stem rectangle laid
/// across a bowl path, '4' crosses itself six times — but polygon() allows
/// exactly the same thing and the reference pins the answer (a bowtie is
/// two triangles).
///
/// A piece buried in ink is dropped, which is what keeps an extrusion from
/// raising a wall inside its own solid whose top edge lies in the cap plane
/// — the two then fight for the same depth and the seam shows as a hairline
/// ruled across the face.
fn boundary_segments(clean: &[Vec<Vec2>], fill: Fill) -> Vec<[Vec2; 2]> {
    let mut segs: Vec<[Vec2; 2]> = Vec::new();
    for c in clean {
        let n = c.len();
        for i in 0..n {
            segs.push([c[i], c[(i + 1) % n]]);
        }
    }
    if segs.is_empty() {
        return segs;
    }
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for s in &segs {
        for q in s {
            for k in 0..2 {
                lo[k] = lo[k].min(q[k]);
                hi[k] = hi[k].max(q[k]);
            }
        }
    }
    let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-12);
    let tol = span * 1e-9;
    let probe = span * 1e-6;
    // Per-segment bounds, so the quadratic pass skips all but the handful of
    // edges that could actually meet this one.
    let bbox: Vec<[f64; 4]> = segs
        .iter()
        .map(|s| {
            [
                s[0][0].min(s[1][0]) - tol,
                s[0][1].min(s[1][1]) - tol,
                s[0][0].max(s[1][0]) + tol,
                s[0][1].max(s[1][1]) + tol,
            ]
        })
        .collect();

    let mut out: Vec<[Vec2; 2]> = Vec::with_capacity(segs.len());
    let mut ts: Vec<f64> = Vec::new();
    for i in 0..segs.len() {
        ts.clear();
        ts.push(0.0);
        ts.push(1.0);
        for j in 0..segs.len() {
            if i == j
                || bbox[i][0] > bbox[j][2]
                || bbox[i][2] < bbox[j][0]
                || bbox[i][1] > bbox[j][3]
                || bbox[i][3] < bbox[j][1]
            {
                continue;
            }
            split_params(&segs[i], &segs[j], tol, &mut ts);
        }
        ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let (a, b) = (segs[i][0], segs[i][1]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 0.0 {
            continue;
        }
        let nl = [-dy / len, dx / len]; // left-hand normal
        for w in ts.windows(2) {
            let (t0, t1) = (w[0], w[1]);
            if (t1 - t0) * len <= tol {
                continue;
            }
            let tm = 0.5 * (t0 + t1);
            let m = [a[0] + tm * dx, a[1] + tm * dy];
            let left = filled_at(clean, fill, [m[0] + nl[0] * probe, m[1] + nl[1] * probe]);
            let right = filled_at(clean, fill, [m[0] - nl[0] * probe, m[1] - nl[1] * probe]);
            if left == right {
                continue; // ink on both sides, or neither: not boundary
            }
            let (p0, p1) = (
                [a[0] + t0 * dx, a[1] + t0 * dy],
                [a[0] + t1 * dx, a[1] + t1 * dy],
            );
            out.push(if left { [p0, p1] } else { [p1, p0] });
        }
    }
    // Two contours can share a stretch of boundary exactly — the top bar of
    // 'F' lies flush with the top of its stem — and each contributes the
    // same piece. Kept twice the wall is built twice and the solid no longer
    // closes; kept once in each direction the ink is on both sides and it
    // was never boundary at all. Folding by direction settles both.
    let snap = span * 1e-7;
    let key = |p: Vec2| [(p[0] / snap).round() as i64, (p[1] / snap).round() as i64];
    let mut at: std::collections::HashMap<[i64; 4], usize> = std::collections::HashMap::new();
    let mut fold: Vec<([Vec2; 2], i32)> = Vec::new();
    for s in out {
        let (ka, kb) = (key(s[0]), key(s[1]));
        let fwd = (ka[0], ka[1]) <= (kb[0], kb[1]);
        let k = if fwd {
            [ka[0], ka[1], kb[0], kb[1]]
        } else {
            [kb[0], kb[1], ka[0], ka[1]]
        };
        let e = *at.entry(k).or_insert_with(|| {
            fold.push((if fwd { s } else { [s[1], s[0]] }, 0));
            fold.len() - 1
        });
        fold[e].1 += if fwd { 1 } else { -1 };
    }
    fold.into_iter()
        .filter_map(|(s, n)| match n {
            0 => None,
            n if n > 0 => Some(s),
            _ => Some([s[1], s[0]]),
        })
        .collect()
}

/// Chain directed boundary segments (ink on the left) into closed contours.
/// Where more than one edge leaves a junction, take the sharpest RIGHT turn:
/// that hugs the filled side and traces loops that do not cross each other.
fn chain_loops(segs: &[[Vec2; 2]], snap: f64) -> Vec<Vec<Vec2>> {
    let mut verts: Vec<Vec2> = Vec::new();
    let mut ids: std::collections::HashMap<[i64; 2], usize> = std::collections::HashMap::new();
    let id = |p: Vec2, verts: &mut Vec<Vec2>, ids: &mut std::collections::HashMap<[i64; 2], usize>| {
        let k = [(p[0] / snap).round() as i64, (p[1] / snap).round() as i64];
        *ids.entry(k).or_insert_with(|| {
            verts.push(p);
            verts.len() - 1
        })
    };
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for s in segs {
        let (u, v) = (id(s[0], &mut verts, &mut ids), id(s[1], &mut verts, &mut ids));
        if u != v {
            edges.push((u, v));
        }
    }
    let mut out_of: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for (i, &(u, _)) in edges.iter().enumerate() {
        out_of.entry(u).or_default().push(i);
    }
    // Clockwise angle in [0, tau) to rotate `from` onto `to`.
    let cw = |from: Vec2, to: Vec2| {
        let mut d = from[1].atan2(from[0]) - to[1].atan2(to[0]);
        while d < 0.0 {
            d += std::f64::consts::TAU;
        }
        d
    };
    let mut used = vec![false; edges.len()];
    let mut loops: Vec<Vec<Vec2>> = Vec::new();
    for seed in 0..edges.len() {
        if used[seed] {
            continue;
        }
        let start = edges[seed].0;
        let mut cur = seed;
        used[cur] = true;
        let mut pts = vec![verts[start]];
        let mut steps = 0usize;
        loop {
            let (from, v) = edges[cur];
            if v == start {
                break;
            }
            pts.push(verts[v]);
            // Sharpest right turn from the way we came.
            let r = sub(verts[from], verts[v]);
            let mut best: Option<usize> = None;
            let mut best_score = f64::INFINITY;
            for &e in out_of.get(&v).map(|v| &v[..]).unwrap_or(&[]) {
                if used[e] {
                    continue;
                }
                let mut a = cw(r, sub(verts[edges[e].1], verts[v]));
                if a < 1e-9 {
                    a = std::f64::consts::TAU; // doubling straight back: last resort
                }
                if a < best_score {
                    best_score = a;
                    best = Some(e);
                }
            }
            match best {
                Some(e) => {
                    used[e] = true;
                    cur = e;
                }
                None => {
                    pts.clear(); // an open chain: no loop to keep
                    break;
                }
            }
            steps += 1;
            if steps > edges.len() + 1 {
                pts.clear();
                break;
            }
        }
        if pts.len() >= 3 && signed_area2(&pts).abs() > 0.0 {
            loops.push(pts);
        }
    }
    loops
}

/// Resolve a region into simple, properly nested contours enclosing exactly
/// the same filled area, each wound with the fill on its LEFT.
///
/// Everything downstream of a 2D region — the nesting classifier, the
/// boolean BSP, offset — assumes contours that do not cross. Crossing ones
/// are legal input: the reference says self-intersecting outlines are
/// resolved by the 2D kernel even-odd, a bowtie becoming two triangles. So
/// the crossings are cut in and the boundary re-traced. A region that does
/// not cross itself is already nested and comes back untouched, vertex for
/// vertex, so the common case pays only for the sweep that proves it.
pub fn sanitize(poly: &Poly2) -> Poly2 {
    let clean = clean_contours(poly);
    if clean.is_empty() {
        return Poly2::new(Vec::new());
    }
    if poly.fill == Fill::EvenOdd && !has_crossing(&clean) {
        return Poly2::new(clean);
    }
    let mut loops = Vec::new();
    for g in components(&clean) {
        let part: Vec<Vec<Vec2>> = g.into_iter().map(|i| clean[i].clone()).collect();
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for c in &part {
            for q in c {
                for k in 0..2 {
                    lo[k] = lo[k].min(q[k]);
                    hi[k] = hi[k].max(q[k]);
                }
            }
        }
        let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-12);
        loops.extend(chain_loops(&boundary_segments(&part, poly.fill), span * 1e-7));
    }
    Poly2::new(loops)
}

/// Directed wall segments for an extrusion, ink always on the LEFT.
///
/// A region whose contours do not cross already carries its own boundary,
/// so it passes straight through. One whose contours DO cross — a font's
/// always, a self-intersecting polygon() sometimes — has its boundary
/// re-derived, which both orients each piece and drops the stretches that
/// run buried through ink.
pub fn wall_segments(poly: &Poly2) -> Vec<[Vec2; 2]> {
    let clean = clean_contours(poly);
    if clean.is_empty() {
        return Vec::new();
    }
    if poly.fill == Fill::EvenOdd && !has_crossing(&clean) {
        let mut segs = Vec::new();
        for c in oriented_contours(poly) {
            let n = c.len();
            for i in 0..n {
                segs.push([c[i], c[(i + 1) % n]]);
            }
        }
        return segs;
    }
    let mut out = Vec::new();
    for g in components(&clean) {
        let part: Vec<Vec<Vec2>> = g.into_iter().map(|i| clean[i].clone()).collect();
        out.extend(boundary_segments(&part, poly.fill));
    }
    out
}

/// Emit one wall triangle, unless it has collapsed. A face with no area has
/// no cross product either, so the viewer's per-face normal comes out as
/// zero and the triangle shades as a dark sliver across whatever it lies
/// on — worth the check on a ring that a scale=0 taper or a repeated
/// outline point can pinch flat.
fn push_tri(tris: &mut Vec<[u32; 3]>, q: &[[f64; 3]; 4], base: u32, k: [usize; 3]) {
    let (a, b, c) = (q[k[0]], q[k[1]], q[k[2]]);
    let (u, v) = (
        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
    );
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    if n[0] * n[0] + n[1] * n[1] + n[2] * n[2] > 0.0 {
        tris.push([base + k[0] as u32, base + k[1] as u32, base + k[2] as u32]);
    }
}

/// linear_extrude: stamp the region as slices+1 rings from z0 to z0+height,
/// each ring scaled (lerp 1→scale) then rotated (-twist·t degrees) about
/// the origin. Walls connect consecutive rings; the base triangulation
/// forms the bottom (reversed) and top caps. Negative scale clamps to 0.
pub fn extrude_linear(
    poly: &Poly2,
    height: f64,
    center: bool,
    twist: f64,
    slices: usize,
    scale: Vec2,
) -> Mesh3 {
    let slices = slices.max(1);
    let scale = [scale[0].max(0.0), scale[1].max(0.0)];
    let z0 = if center { -height / 2.0 } else { 0.0 };
    let ring = |p: Vec2, t: f64| -> [f64; 3] {
        let sx = 1.0 + (scale[0] - 1.0) * t;
        let sy = 1.0 + (scale[1] - 1.0) * t;
        let (x, y) = (p[0] * sx, p[1] * sy);
        let ang = (-twist * t).to_radians();
        let (c, s) = (ang.cos(), ang.sin());
        [x * c - y * s, x * s + y * c, z0 + t * height]
    };
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut tris: Vec<[u32; 3]> = Vec::new();
    let (cap2, cap_tris) = triangulate(poly);
    // Bottom cap at t=0, reversed to face -z.
    let base = positions.len() as u32;
    positions.extend(cap2.iter().map(|v| ring(*v, 0.0)));
    tris.extend(cap_tris.iter().map(|t| [base + t[0], base + t[2], base + t[1]]));
    // Top cap at t=1, facing +z.
    let base = positions.len() as u32;
    positions.extend(cap2.iter().map(|v| ring(*v, 1.0)));
    tris.extend(cap_tris.iter().map(|t| [base + t[0], base + t[1], base + t[2]]));
    // Side walls, one quad per boundary segment per slice (segments carry
    // the ink on their left, so hole walls face inward).
    for seg in &wall_segments(poly) {
        for i in 0..slices {
            let (t0, t1) = (i as f64 / slices as f64, (i + 1) as f64 / slices as f64);
            let q = [
                ring(seg[0], t0),
                ring(seg[1], t0),
                ring(seg[1], t1),
                ring(seg[0], t1),
            ];
            let b = positions.len() as u32;
            positions.extend_from_slice(&q);
            push_tri(&mut tris, &q, b, [0, 1, 2]);
            push_tri(&mut tris, &q, b, [0, 2, 3]);
        }
    }
    (positions, tris)
}

/// rotate_extrude: revolve the profile around Z — (x, y) maps to
/// (x·cosθ, x·sinθ, y). All profile x must share one sign (else Err). A
/// full sweep welds; a partial sweep adds flat caps at θ=0 and θ=angle.
pub fn extrude_rotate(poly: &Poly2, angle_deg: f64, frags: usize) -> Result<Mesh3, String> {
    let frags = frags.max(1);
    let mut minx = f64::INFINITY;
    let mut maxx = f64::NEG_INFINITY;
    for c in &poly.contours {
        for p in c {
            minx = minx.min(p[0]);
            maxx = maxx.max(p[0]);
        }
    }
    if minx < -1e-9 && maxx > 1e-9 {
        return Err(format!(
            "all points for rotate_extrude() must have the same X coordinate sign \
             (range is {:.2} -> {:.2})",
            minx, maxx
        ));
    }
    let full = angle_deg.abs() >= 360.0 - 1e-9;
    let sweep = if full { 360.0 } else { angle_deg };
    let angle_at = |k: usize| (sweep * k as f64 / frags as f64).to_radians();
    let revolve = |p: Vec2, theta: f64| [p[0] * theta.cos(), p[0] * theta.sin(), p[1]];
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut tris: Vec<[u32; 3]> = Vec::new();
    for edge in &wall_segments(poly) {
        for seg in 0..frags {
            let (th0, th1) = (angle_at(seg), angle_at(seg + 1));
            {
                let q = [
                    revolve(edge[0], th0),
                    revolve(edge[1], th0),
                    revolve(edge[1], th1),
                    revolve(edge[0], th1),
                ];
                let b = positions.len() as u32;
                positions.extend_from_slice(&q);
                // Winding: walking the profile CCW puts the outer wall's
                // edge going +z, and sweeping puts the next edge at +theta;
                // (+z) x (+theta) points at the axis, so the naive order
                // gives an inside-out solid. Both triangles are reversed.
                // Winding: walking the profile CCW puts the outer wall's
                // edge going +z, and sweeping puts the next edge at +theta;
                // (+z) x (+theta) points at the axis, so the naive order
                // gives an inside-out solid. Both triangles are reversed.
                push_tri(&mut tris, &q, b, [0, 2, 1]);
                push_tri(&mut tris, &q, b, [0, 3, 2]);
            }
        }
    }
    if !full {
        let (cap2, cap_tris) = triangulate(poly);
        // Cap at θ=0 faces the -θ side; a CCW cap triangle already maps to
        // a -y normal there, so it is the θ=angle cap that gets reversed.
        let base = positions.len() as u32;
        positions.extend(cap2.iter().map(|v| revolve(*v, angle_at(0))));
        tris.extend(cap_tris.iter().map(|t| [base + t[0], base + t[1], base + t[2]]));
        // Cap at θ=angle.
        let base = positions.len() as u32;
        positions.extend(cap2.iter().map(|v| revolve(*v, angle_at(frags))));
        tris.extend(cap_tris.iter().map(|t| [base + t[0], base + t[2], base + t[1]]));
    }
    Ok((positions, tris))
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

    /// The reference pins this: "Self-intersection resolution is even-odd,
    /// not nonzero — polygon(bowtie) yields two triangles" (/7/edge_cases/8),
    /// and "a point covered by an odd number of boundaries is inside"
    /// (/7/semantics). Neither is a question the NESTING classifier can
    /// answer — of a path that crosses itself there are not two contours to
    /// compare, and of two squares laid half across each other neither is
    /// inside the other — so both used to come back with the overlap added
    /// instead of cancelled.
    #[test]
    fn crossing_contours_resolve_even_odd() {
        let area = |p: &Poly2| -> f64 {
            let (v, t) = triangulate(p);
            t.iter()
                .map(|tr| {
                    let (a, b, c) = (v[tr[0] as usize], v[tr[1] as usize], v[tr[2] as usize]);
                    ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5
                })
                .sum()
        };
        // A bowtie: two triangles of 25, not one 100-unit square.
        let bowtie = Poly2::new(vec![vec![[0.0, 0.0], [10.0, 0.0], [0.0, 10.0], [10.0, 10.0]]]);
        assert!((area(&bowtie) - 50.0).abs() < 1e-9, "bowtie filled {}", area(&bowtie));
        // Two sibling squares overlapping by 25: union 175, less the overlap
        // counted a second time = 150.
        let pair = Poly2::new(vec![
            vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            vec![[5.0, 5.0], [15.0, 5.0], [15.0, 15.0], [5.0, 15.0]],
        ]);
        assert!((area(&pair) - 150.0).abs() < 1e-9, "overlapping pair filled {}", area(&pair));
        // And they extrude to a closed solid of exactly that area times height.
        for (name, region, want) in [("bowtie", bowtie, 50.0), ("pair", pair, 150.0)] {
            let h = 4.0;
            let (p, t) = extrude_linear(&region, h, false, 0.0, 1, [1.0, 1.0]);
            let vol: f64 = t
                .iter()
                .map(|tr| {
                    let (a, b, c) = (p[tr[0] as usize], p[tr[1] as usize], p[tr[2] as usize]);
                    (a[0] * (b[1] * c[2] - b[2] * c[1]) + a[1] * (b[2] * c[0] - b[0] * c[2])
                        + a[2] * (b[0] * c[1] - b[1] * c[0]))
                        / 6.0
                })
                .sum();
            assert!(
                (vol - want * h).abs() < 1e-6,
                "{} extrudes to {}, not {}",
                name,
                vol,
                want * h
            );
        }
    }

    /// A region whose contours do not cross is already nested, and must come
    /// back from the sanitizer vertex for vertex — the reference observes
    /// square()'s outline as exactly four vertices, and every clean region
    /// pays only for the sweep that proves it needs nothing.
    #[test]
    fn sanitize_leaves_a_clean_region_alone() {
        let p = Poly2::new(vec![
            vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            vec![[3.0, 3.0], [3.0, 7.0], [7.0, 7.0], [7.0, 3.0]],
        ]);
        assert_eq!(sanitize(&p).contours, p.contours);
        assert_eq!(sanitize(&circle(5.0, 12)).contours, circle(5.0, 12).contours);
    }

    /// Glyph outlines are not the clean nested contours a 2D primitive
    /// gives: this face draws 'B' as a stem rectangle laid ACROSS a bowl
    /// path, 'A' as three crossing bars, and '4' and 'e' as a single path
    /// that crosses itself and leaves the counter as the pocket its winding
    /// cancels in. Filling them by even-odd punched a hole through every
    /// overlap and ear clipping filled every self-crossed counter solid, so
    /// the fill is checked against the non-zero winding itself, sampled.
    #[test]
    fn glyph_fill_matches_the_fonts_own_winding() {
        let f = crate::font::default_font().expect("bundled font");
        for ch in ['A', 'B', 'G', '4', 'e', '8'] {
            let poly = Poly2::new_font(f.glyph_contours(f.glyph_id(ch), 6));
            let cs: Vec<Vec<Vec2>> = poly.contours.clone();
            let (v, t) = triangulate(&poly);
            // Non-zero winding of the raw outline, the answer a rasterizer
            // would give.
            let wind = |p: Vec2| -> i32 {
                let mut w = 0;
                for c in &cs {
                    let n = c.len();
                    for i in 0..n {
                        let (a, b) = (c[i], c[(i + 1) % n]);
                        if a[1] <= p[1] {
                            if b[1] > p[1] && cross(a, b, p) > 0.0 {
                                w += 1;
                            }
                        } else if b[1] <= p[1] && cross(a, b, p) < 0.0 {
                            w -= 1;
                        }
                    }
                }
                w
            };
            let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
            for c in &cs {
                for q in c {
                    for k in 0..2 {
                        lo[k] = lo[k].min(q[k]);
                        hi[k] = hi[k].max(q[k]);
                    }
                }
            }
            let n = 60;
            let cell = ((hi[0] - lo[0]) / n as f64).max((hi[1] - lo[1]) / n as f64);
            let mut bad = 0;
            for gy in 0..n {
                for gx in 0..n {
                    let p = [
                        lo[0] + (hi[0] - lo[0]) * (gx as f64 + 0.5) / n as f64,
                        lo[1] + (hi[1] - lo[1]) * (gy as f64 + 0.5) / n as f64,
                    ];
                    let filled = t.iter().any(|tr| {
                        in_triangle(p, v[tr[0] as usize], v[tr[1] as usize], v[tr[2] as usize])
                    });
                    if filled == (wind(p) != 0) {
                        continue;
                    }
                    // A sample landing ON an edge may go either way; anything
                    // further out is a real hole in the letter or ink spilled
                    // outside it.
                    let mut d = f64::INFINITY;
                    for c in &cs {
                        let m = c.len();
                        for i in 0..m {
                            let (a, b) = (c[i], c[(i + 1) % m]);
                            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                            let l2 = dx * dx + dy * dy;
                            let u = if l2 > 0.0 {
                                (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            d = d.min(dist2([a[0] + u * dx, a[1] + u * dy], p).sqrt());
                        }
                    }
                    if d > cell {
                        bad += 1;
                    }
                }
            }
            assert_eq!(bad, 0, "'{}' fill disagrees with its own winding at {} samples", ch, bad);
        }
    }

    /// A collapsed triangle has no cross product, so the viewer's per-face
    /// normal comes out zero and it shades as a dark sliver laid across the
    /// letter. Every TrueType contour used to arrive closed on a repeat of
    /// its first point, which cost two of them per contour in the walls
    /// alone.
    #[test]
    fn extruded_text_emits_no_collapsed_faces() {
        let f = crate::font::default_font().expect("bundled font");
        let mut contours = Vec::new();
        let mut pen = 0.0;
        for ch in "ABGORS8@e4".chars() {
            let gid = f.glyph_id(ch);
            for c in f.glyph_contours(gid, 6) {
                contours.push(c.iter().map(|p| [p[0] + pen, p[1]]).collect::<Vec<Vec2>>());
            }
            pen += f.advance(gid);
        }
        let (pos, tris) = extrude_linear(&Poly2::new_font(contours), 60.0, false, 0.0, 1, [1.0, 1.0]);
        assert!(!tris.is_empty());
        for t in &tris {
            let (a, b, c) = (pos[t[0] as usize], pos[t[1] as usize], pos[t[2] as usize]);
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            assert!(
                n[0] * n[0] + n[1] * n[1] + n[2] * n[2] > 0.0,
                "collapsed face {:?} {:?} {:?}",
                a,
                b,
                c
            );
        }
    }

    /// The real proof that the walls trace the true boundary and the caps
    /// the true fill: the extruded letters must close into a solid whose
    /// volume is exactly its own ink area times its height. A wall left
    /// buried in the ink, or one missing from an edge, breaks it.
    #[test]
    fn extruded_text_closes_to_area_times_height() {
        let f = crate::font::default_font().expect("bundled font");
        let mut contours = Vec::new();
        let mut pen = 0.0;
        for ch in "ABGe48@Rm".chars() {
            let gid = f.glyph_id(ch);
            for c in f.glyph_contours(gid, 6) {
                contours.push(c.iter().map(|p| [p[0] + pen, p[1]]).collect::<Vec<Vec2>>());
            }
            pen += f.advance(gid);
        }
        let poly = Poly2::new_font(contours);
        let (cv, ct) = triangulate(&poly);
        let area: f64 = ct
            .iter()
            .map(|t| {
                let (a, b, c) = (cv[t[0] as usize], cv[t[1] as usize], cv[t[2] as usize]);
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) * 0.5
            })
            .sum();
        let h = 40.0;
        let (p, t) = extrude_linear(&poly, h, false, 0.0, 1, [1.0, 1.0]);
        let vol: f64 = t
            .iter()
            .map(|tr| {
                let (a, b, c) = (p[tr[0] as usize], p[tr[1] as usize], p[tr[2] as usize]);
                (a[0] * (b[1] * c[2] - b[2] * c[1]) + a[1] * (b[2] * c[0] - b[0] * c[2])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum();
        assert!(area > 0.0, "cap area {}", area);
        assert!(
            (vol - area * h).abs() < area * h * 1e-6,
            "extruded text is not closed: volume {} vs area*height {}",
            vol,
            area * h
        );
    }

    /// Ear clipping used to give up on the whole remaining ring the first
    /// time no clean ear turned up, leaving a facet simply missing. A comb
    /// with a bridged hole is the shape that did it.
    #[test]
    fn ear_clipping_never_abandons_a_ring() {
        let outer = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let hole = vec![[3.0, 3.0], [3.0, 7.0], [7.0, 7.0], [7.0, 3.0]];
        let p = Poly2::new(vec![outer, hole]);
        let (v, t) = triangulate(&p);
        let a: f64 = t
            .iter()
            .map(|tr| {
                let (a, b, c) = (v[tr[0] as usize], v[tr[1] as usize], v[tr[2] as usize]);
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5
            })
            .sum();
        assert!((a - 84.0).abs() < 1e-9, "square with a hole should fill 84, got {}", a);
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

    fn signed_volume(pos: &[[f64; 3]], tris: &[[u32; 3]]) -> f64 {
        let mut v = 0.0;
        for t in tris {
            let a = pos[t[0] as usize];
            let b = pos[t[1] as usize];
            let c = pos[t[2] as usize];
            v += (a[0] * (b[1] * c[2] - b[2] * c[1]) + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0;
        }
        v
    }

    #[test]
    fn linear_extrude_makes_an_outward_prism() {
        // A 3×2 square extruded 5 tall = a 30-unit box, outward-wound.
        let (p, t) = extrude_linear(&square([3.0, 2.0], false), 5.0, false, 0.0, 1, [1.0, 1.0]);
        assert!((signed_volume(&p, &t) - 30.0).abs() < 1e-9, "vol {}", signed_volume(&p, &t));
        // A ring (square with a hole) extruded stays hollow: area 84 × h.
        let ring = Poly2::new(vec![
            vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            vec![[3.0, 3.0], [7.0, 3.0], [7.0, 7.0], [3.0, 7.0]],
        ]);
        let (p, t) = extrude_linear(&ring, 2.0, false, 0.0, 1, [1.0, 1.0]);
        // Signed, not |signed|: an inside-out solid has the right magnitude
        // and the wrong sign, and comparing magnitudes cannot tell them apart.
        assert!((signed_volume(&p, &t) - 84.0 * 2.0).abs() < 1e-6, "prism volume {}", signed_volume(&p, &t));
    }

    #[test]
    fn rotate_extrude_revolves_a_washer() {
        // A unit-tall rectangle at radius [2,4], revolved 360° → a washer
        // of volume π(4² − 2²)·1 = 12π.
        let rect = Poly2::new(vec![vec![[2.0, 0.0], [4.0, 0.0], [4.0, 1.0], [2.0, 1.0]]]);
        let (p, t) = extrude_rotate(&rect, 360.0, 128).unwrap();
        let want = std::f64::consts::PI * 12.0;
        let got = signed_volume(&p, &t);
        assert!((got - want).abs() < 0.2, "washer volume {got}, want {want}");
        // A profile straddling X=0 is an error.
        let bad = Poly2::new(vec![vec![[-1.0, 0.0], [1.0, 0.0], [0.0, 1.0]]]);
        assert!(extrude_rotate(&bad, 360.0, 16).is_err());
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

