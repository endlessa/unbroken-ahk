//! Mesh import/export: STL (ASCII + binary) and OFF, from scratch.
//!
//! Geometry only — the 2021.01 mesh formats carry no units, names, or color
//! beyond fixed boilerplate, so a round-trip preserves the triangle set (up
//! to vertex welding), which is exactly the equivalence the reference
//! promises (mesh-level, never byte-level). `%` background geometry is
//! excluded from exports and `#` highlight geometry is included — that
//! selection happens in the caller, which hands us the already-merged solid.

use crate::geom::Mesh;
use std::collections::HashMap;

// -- Export -----------------------------------------------------------------

/// Format a coordinate the way the reference's ASCII exporters do: up to 6
/// significant digits, no trailing zeros, no exponent for ordinary ranges.
fn fmt_coord(x: f64) -> String {
    if x == 0.0 {
        return "0".to_string(); // avoid "-0"
    }
    let mut s = format!("{:.6}", x);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

/// Per-triangle geometric normal (recomputed from winding — stored STL
/// normals are ignored on import, and OpenSCAD recomputes on export).
fn triangle_normal(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> [f64; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len < 1e-12 {
        [0.0, 0.0, 0.0]
    } else {
        [n[0] / len, n[1] / len, n[2] / len]
    }
}

/// ASCII STL. Header/trailer name is fixed boilerplate; facet normals are
/// recomputed from winding.
pub fn write_stl_ascii(mesh: &Mesh) -> String {
    let mut s = String::from("solid scadforge_model\n");
    for t in &mesh.tris {
        let a = mesh.positions[t[0] as usize];
        let b = mesh.positions[t[1] as usize];
        let c = mesh.positions[t[2] as usize];
        let n = triangle_normal(a, b, c);
        s.push_str(&format!(
            "  facet normal {} {} {}\n    outer loop\n",
            fmt_coord(n[0]),
            fmt_coord(n[1]),
            fmt_coord(n[2])
        ));
        for v in [a, b, c] {
            s.push_str(&format!(
                "      vertex {} {} {}\n",
                fmt_coord(v[0]),
                fmt_coord(v[1]),
                fmt_coord(v[2])
            ));
        }
        s.push_str("    endloop\n  endfacet\n");
    }
    s.push_str("endsolid scadforge_model\n");
    s
}

/// Binary STL: 80-byte header, u32 triangle count, then 50-byte records
/// (float32 normal + 3 float32 vertices + u16 attribute, all little-endian).
pub fn write_stl_binary(mesh: &Mesh) -> Vec<u8> {
    let mut out = Vec::with_capacity(84 + mesh.tris.len() * 50);
    let mut header = [0u8; 80];
    let tag = b"scadforge binary STL";
    header[..tag.len()].copy_from_slice(tag);
    out.extend_from_slice(&header);
    out.extend_from_slice(&(mesh.tris.len() as u32).to_le_bytes());
    let push_f32 = |out: &mut Vec<u8>, x: f64| out.extend_from_slice(&(x as f32).to_le_bytes());
    for t in &mesh.tris {
        let a = mesh.positions[t[0] as usize];
        let b = mesh.positions[t[1] as usize];
        let c = mesh.positions[t[2] as usize];
        let n = triangle_normal(a, b, c);
        for k in 0..3 {
            push_f32(&mut out, n[k]);
        }
        for v in [a, b, c] {
            for k in 0..3 {
                push_f32(&mut out, v[k]);
            }
        }
        out.extend_from_slice(&0u16.to_le_bytes()); // attribute byte count
    }
    out
}

/// OFF (Geomview): 'OFF', a counts line, vertex lines, then face lines that
/// begin with the vertex count. We emit triangles (count 3).
pub fn write_off(mesh: &Mesh) -> String {
    let mut s = String::from("OFF\n");
    s.push_str(&format!("{} {} 0\n", mesh.positions.len(), mesh.tris.len()));
    for p in &mesh.positions {
        s.push_str(&format!("{} {} {}\n", fmt_coord(p[0]), fmt_coord(p[1]), fmt_coord(p[2])));
    }
    for t in &mesh.tris {
        s.push_str(&format!("3 {} {} {}\n", t[0], t[1], t[2]));
    }
    s
}

// -- Import -----------------------------------------------------------------

/// Weld coincident vertices by EXACT position (STL stores each triangle's
/// three vertices independently; matching by position stitches the mesh, per
/// the reference).
struct Welder {
    map: HashMap<[u64; 3], u32>,
    positions: Vec<[f64; 3]>,
}

impl Welder {
    fn new() -> Welder {
        Welder { map: HashMap::new(), positions: Vec::new() }
    }
    fn intern(&mut self, p: [f64; 3]) -> u32 {
        let key = [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()];
        if let Some(&i) = self.map.get(&key) {
            return i;
        }
        let i = self.positions.len() as u32;
        self.positions.push(p);
        self.map.insert(key, i);
        i
    }
}

/// True when the byte length matches the binary-STL layout for the triangle
/// count stored at offset 80 — the robust sniff, since a binary file's
/// 80-byte header can itself begin with "solid".
fn looks_like_binary_stl(bytes: &[u8]) -> bool {
    if bytes.len() < 84 {
        return false;
    }
    let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    bytes.len() == 84 + count * 50
}

/// Read an STL, autodetecting ASCII vs binary. Degenerate (zero-area)
/// triangles are dropped. Stored normals are ignored.
pub fn read_stl(bytes: &[u8]) -> Result<Mesh, String> {
    if looks_like_binary_stl(bytes) {
        read_stl_binary(bytes)
    } else if bytes.len() >= 5 && &bytes[..5] == b"solid" {
        // Try ASCII; a mis-sniffed binary that happens to start with "solid"
        // but failed the size check falls back to binary — including the case
        // where the ASCII parse "succeeds" but finds no facets (a binary body).
        match read_stl_ascii(bytes) {
            Ok(m) if !m.tris.is_empty() => Ok(m),
            _ => match read_stl_binary(bytes) {
                Ok(b) if !b.tris.is_empty() => Ok(b),
                _ => read_stl_ascii(bytes), // neither found geometry; keep ASCII result
            },
        }
    } else {
        read_stl_binary(bytes)
    }
}

fn read_stl_binary(bytes: &[u8]) -> Result<Mesh, String> {
    if bytes.len() < 84 {
        return Err("binary STL too short".into());
    }
    let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    // NEVER pre-allocate the untrusted count: a tiny file can claim billions of
    // triangles (OOM/abort). Cap the reservation at what the bytes can hold.
    let max_possible = bytes.len().saturating_sub(84) / 50;
    let mut w = Welder::new();
    let mut tris = Vec::with_capacity(count.min(max_possible));
    let mut off = 84;
    let rf = |b: &[u8], o: usize| f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as f64;
    for _ in 0..count {
        if off + 50 > bytes.len() {
            break; // truncated — keep what parsed
        }
        // Skip the 12-byte stored normal; recompute from winding on export.
        let mut v = [[0.0; 3]; 3];
        for (k, vk) in v.iter_mut().enumerate() {
            let base = off + 12 + k * 12;
            *vk = [rf(bytes, base), rf(bytes, base + 4), rf(bytes, base + 8)];
        }
        push_triangle(&mut w, &mut tris, v[0], v[1], v[2]);
        off += 50;
    }
    Ok(Mesh { positions: w.positions, tris })
}

fn read_stl_ascii(bytes: &[u8]) -> Result<Mesh, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "ASCII STL is not valid UTF-8".to_string())?;
    let mut w = Welder::new();
    let mut tris = Vec::new();
    let mut loop_verts: Vec<[f64; 3]> = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("vertex") => {
                let c: Vec<f64> = it.filter_map(|t| t.parse().ok()).collect();
                if c.len() != 3 {
                    return Err(format!("malformed STL vertex: {:?}", line));
                }
                loop_verts.push([c[0], c[1], c[2]]);
            }
            Some("endloop") => {
                if loop_verts.len() == 3 {
                    push_triangle(&mut w, &mut tris, loop_verts[0], loop_verts[1], loop_verts[2]);
                }
                loop_verts.clear();
            }
            _ => {}
        }
    }
    Ok(Mesh { positions: w.positions, tris })
}

/// Read an OFF text mesh. Comments (`#`) and blank lines are skipped;
/// polygonal faces are fan-triangulated.
pub fn read_off(text: &str) -> Result<Mesh, String> {
    // Tokenize, dropping comments and blank lines. The optional leading 'OFF'
    // may be glued to the counts line (e.g. "OFF") on its own.
    let mut toks: Vec<f64> = Vec::new();
    let mut saw_off = false;
    let mut raw: Vec<&str> = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        for t in line.split_whitespace() {
            if !saw_off && t.eq_ignore_ascii_case("OFF") {
                saw_off = true;
                continue;
            }
            raw.push(t);
        }
    }
    for t in &raw {
        toks.push(t.parse().unwrap_or(f64::NAN));
    }
    if toks.len() < 3 {
        return Err("OFF: missing counts line".into());
    }
    // `f64 as usize` saturates (inf/huge → usize::MAX), so the declared counts
    // are untrusted; never reserve more than the token stream can supply (each
    // vertex needs 3 tokens, each face ≥ 4). This bounds the allocation to the
    // input size — a "4000000000 0 0" header can't OOM the process.
    let count = |t: f64| if t.is_finite() && t >= 0.0 { t as usize } else { 0 };
    let nv = count(toks[0]);
    let nf = count(toks[1]);
    let mut idx = 3; // vertices start after "nv nf ne"
    let mut positions = Vec::with_capacity(nv.min(toks.len() / 3));
    for _ in 0..nv {
        if idx + 3 > toks.len() {
            return Err("OFF: truncated vertex list".into());
        }
        let fin = |x: f64| if x.is_finite() { x } else { 0.0 };
        positions.push([fin(toks[idx]), fin(toks[idx + 1]), fin(toks[idx + 2])]);
        idx += 3;
    }
    let mut tris = Vec::with_capacity(nf.min(toks.len() / 4));
    for _ in 0..nf {
        if idx >= toks.len() {
            break;
        }
        let k = count(toks[idx]);
        idx += 1;
        // Checked against remaining tokens (idx + k can overflow if k saturates
        // to usize::MAX, wrapping the bound and forcing an OOB index).
        if k > toks.len().saturating_sub(idx) {
            break;
        }
        let face: Vec<u32> = (0..k).map(|j| toks[idx + j] as u32).collect();
        idx += k;
        // Fan-triangulate, bounds-checking indices.
        for j in 1..k.saturating_sub(1) {
            let (a, b, c) = (face[0], face[j], face[j + 1]);
            if (a as usize) < nv && (b as usize) < nv && (c as usize) < nv {
                tris.push([a, b, c]);
            }
        }
    }
    Ok(Mesh { positions, tris })
}

/// Intern a triangle's vertices and push it, dropping degenerate (zero-area)
/// or non-finite triangles so they can't seed downstream CSG errors.
fn push_triangle(w: &mut Welder, tris: &mut Vec<[u32; 3]>, a: [f64; 3], b: [f64; 3], c: [f64; 3]) {
    if [a, b, c].iter().flatten().any(|x| !x.is_finite()) {
        return; // a NaN/inf coordinate (triangle_normal wouldn't catch NaN)
    }
    if triangle_normal(a, b, c) == [0.0, 0.0, 0.0] {
        return; // zero-area / collinear
    }
    let (ia, ib, ic) = (w.intern(a), w.intern(b), w.intern(c));
    if ia != ib && ib != ic && ia != ic {
        tris.push([ia, ib, ic]);
    }
}

// -- surface() heightmap -----------------------------------------------------

/// Parse a `surface()` text grid: whitespace-separated numeric heights, one
/// row per non-comment line (`#`-led lines and blank lines skipped). Rows are
/// padded to the widest with zeros so the grid is rectangular.
pub fn parse_surface_text(text: &str) -> Vec<Vec<f64>> {
    let mut grid: Vec<Vec<f64>> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let row: Vec<f64> = line.split_whitespace().filter_map(|t| t.parse().ok()).collect();
        if !row.is_empty() {
            grid.push(row);
        }
    }
    let cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
    for r in &mut grid {
        r.resize(cols, 0.0);
    }
    grid
}

/// Build a closed 3D solid from a height grid: the top surface follows the
/// samples (one unit per sample in X/Y), a flat base closes it below the
/// minimum height, and skirt walls join the two. `center` centres the grid on
/// the origin; `invert` negates the heights. The first row maps to the largest
/// Y (text top = far side), matching the reference. Output is OUTWARD-wound
/// (positive signed volume), like every other primitive.
pub fn heightmap_solid(grid: &[Vec<f64>], center: bool, invert: bool) -> Mesh {
    let rows = grid.len();
    let cols = grid.first().map_or(0, |r| r.len());
    if rows < 2 || cols < 2 {
        return Mesh::empty(); // need at least a 2×2 grid for any area
    }
    let h = |r: usize, c: usize| {
        let v = grid[r].get(c).copied().unwrap_or(0.0);
        let v = if v.is_finite() { v } else { 0.0 }; // a NaN/inf sample → 0
        if invert { -v } else { v }
    };
    let mut min_h = f64::INFINITY;
    for r in 0..rows {
        for c in 0..cols {
            min_h = min_h.min(h(r, c));
        }
    }
    // The base plane sits strictly BELOW the lowest sample so boundary cells at
    // the minimum still get non-degenerate walls (a flat grid would otherwise
    // collapse the solid to zero-area triangles).
    let base = min_h - 1.0;
    let (ox, oy) = if center {
        ((cols - 1) as f64 / 2.0, (rows - 1) as f64 / 2.0)
    } else {
        (0.0, 0.0)
    };
    let x = |c: usize| c as f64 - ox;
    let y = |r: usize| (rows - 1 - r) as f64 - oy; // first row → largest Y
    let mut positions: Vec<[f64; 3]> = Vec::with_capacity(rows * cols * 2);
    let top = |r: usize, c: usize| (r * cols + c) as u32;
    let bot_base = (rows * cols) as u32;
    let bot = |r: usize, c: usize| bot_base + (r * cols + c) as u32;
    for r in 0..rows {
        for c in 0..cols {
            positions.push([x(c), y(r), h(r, c)]);
        }
    }
    for r in 0..rows {
        for c in 0..cols {
            positions.push([x(c), y(r), base]);
        }
    }
    let mut tris: Vec<[u32; 3]> = Vec::new();
    for r in 0..rows - 1 {
        for c in 0..cols - 1 {
            // Top surface (faces +Z, CCW seen from above).
            tris.push([top(r, c), top(r, c + 1), top(r + 1, c + 1)]);
            tris.push([top(r, c), top(r + 1, c + 1), top(r + 1, c)]);
            // Base (faces −Z, reversed winding).
            tris.push([bot(r, c), bot(r + 1, c + 1), bot(r, c + 1)]);
            tris.push([bot(r, c), bot(r + 1, c), bot(r + 1, c + 1)]);
        }
    }
    // Skirt walls around the perimeter (top edge down to the base edge).
    let mut wall = |a: u32, b: u32| {
        // a,b are adjacent TOP perimeter vertices; connect to their base twins.
        let (abz, bbz) = (a + bot_base, b + bot_base);
        tris.push([a, b, bbz]);
        tris.push([a, bbz, abz]);
    };
    for c in 0..cols - 1 {
        wall(top(0, c + 1), top(0, c)); // far edge (r=0)
        wall(top(rows - 1, c), top(rows - 1, c + 1)); // near edge
    }
    for r in 0..rows - 1 {
        wall(top(r, 0), top(r + 1, 0)); // left edge
        wall(top(r + 1, cols - 1), top(r, cols - 1)); // right edge
    }
    // The winding above is globally INWARD (top faces −Z); flip every triangle
    // so the solid is outward-wound / positive-volume like every primitive.
    for t in &mut tris {
        t.swap(1, 2);
    }
    Mesh { positions, tris }
}

/// The 3D mesh format implied by a file path's extension (case-insensitive).
#[derive(PartialEq, Debug)]
pub enum MeshFormat {
    Stl,
    Off,
}

pub fn format_from_ext(path: &str) -> Option<MeshFormat> {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "stl" => Some(MeshFormat::Stl),
        "off" => Some(MeshFormat::Off),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom;

    fn tri_count(m: &Mesh) -> usize {
        m.tris.len()
    }

    fn signed_volume(m: &Mesh) -> f64 {
        let mut v = 0.0;
        for t in &m.tris {
            let a = m.positions[t[0] as usize];
            let b = m.positions[t[1] as usize];
            let c = m.positions[t[2] as usize];
            v += (a[0] * (b[1] * c[2] - b[2] * c[1]) + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0;
        }
        v.abs()
    }

    #[test]
    fn stl_ascii_round_trips_a_cube() {
        let cube = geom::cube([3.0, 4.0, 5.0], false); // volume 60
        let text = write_stl_ascii(&cube);
        assert!(text.starts_with("solid scadforge_model"));
        assert!(text.trim_end().ends_with("endsolid scadforge_model"));
        let back = read_stl(text.as_bytes()).unwrap();
        // 12 triangles either way; welding gives 8 unique corners.
        assert_eq!(tri_count(&back), 12);
        assert_eq!(back.positions.len(), 8, "welded to 8 cube corners");
        assert!((signed_volume(&back) - 60.0).abs() < 1e-4, "vol {}", signed_volume(&back));
    }

    #[test]
    fn stl_binary_round_trips_a_cube() {
        let cube = geom::cube([2.0, 2.0, 2.0], true); // volume 8
        let bytes = write_stl_binary(&cube);
        assert!(looks_like_binary_stl(&bytes), "size sniff should classify it binary");
        let back = read_stl(&bytes).unwrap();
        assert_eq!(tri_count(&back), 12);
        assert_eq!(back.positions.len(), 8);
        assert!((signed_volume(&back) - 8.0).abs() < 1e-4);
    }

    #[test]
    fn off_round_trips_a_sphere() {
        let s = geom::sphere(5.0, 16);
        let want = signed_volume(&s);
        let text = write_off(&s);
        assert!(text.starts_with("OFF\n"));
        let back = read_off(&text).unwrap();
        assert_eq!(tri_count(&back), tri_count(&s));
        // 6-decimal coordinate formatting perturbs the volume at the ~1e-4
        // level on a radius-5 sphere; mesh-level (not byte-level) equivalence.
        assert!((signed_volume(&back) - want).abs() < 1e-3, "vol {} want {}", signed_volume(&back), want);
        // Comments and an n-gon face fan-triangulate.
        let quad = "OFF\n# a unit square as one quad\n4 1 0\n0 0 0\n1 0 0\n1 1 0\n0 1 0\n4 0 1 2 3\n";
        let m = read_off(quad).unwrap();
        assert_eq!(tri_count(&m), 2, "quad → two triangles");
    }

    #[test]
    fn binary_stl_starting_with_solid_still_loads() {
        // A binary STL whose 80-byte header begins with "solid" must NOT be
        // mis-parsed as ASCII — the size sniff catches it.
        let mut bytes = write_stl_binary(&geom::cube([1.0, 1.0, 1.0], false));
        let tag = b"solid like an ascii file but actually binary";
        bytes[..tag.len()].copy_from_slice(tag);
        let back = read_stl(&bytes).unwrap();
        assert_eq!(tri_count(&back), 12);
    }

    #[test]
    fn degenerate_triangles_are_dropped_on_import() {
        // Two real triangles plus a zero-area one.
        let ascii = "solid t\n\
            facet normal 0 0 1\n outer loop\n vertex 0 0 0\n vertex 1 0 0\n vertex 0 1 0\n endloop\n endfacet\n\
            facet normal 0 0 0\n outer loop\n vertex 0 0 0\n vertex 1 0 0\n vertex 2 0 0\n endloop\n endfacet\n\
            endsolid t\n";
        let m = read_stl(ascii.as_bytes()).unwrap();
        assert_eq!(tri_count(&m), 1, "the collinear facet is dropped");
    }

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

    /// SIGNED volume (no abs) — detects an inverted (inside-out) mesh.
    fn raw_signed_volume(m: &Mesh) -> f64 {
        let mut v = 0.0;
        for t in &m.tris {
            let a = m.positions[t[0] as usize];
            let b = m.positions[t[1] as usize];
            let c = m.positions[t[2] as usize];
            v += (a[0] * (b[1] * c[2] - b[2] * c[1]) + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0;
        }
        v
    }

    #[test]
    fn surface_builds_a_closed_heightmap_solid() {
        let grid = parse_surface_text("# a little hill\n0 0 0\n0 3 0\n0 0 0\n");
        assert_eq!(grid.len(), 3);
        assert_eq!(grid[1], vec![0.0, 3.0, 0.0]);
        let m = heightmap_solid(&grid, false, false);
        assert!(is_closed_manifold(&m), "surface must be a closed 2-manifold");
        // OUTWARD-wound: the raw signed volume must be POSITIVE, like every
        // other primitive (the earlier winding was inside-out / negative).
        assert!(raw_signed_volume(&m) > 0.0, "outward-wound, got {}", raw_signed_volume(&m));
        // A FLAT grid must still be a non-degenerate closed solid (the base sits
        // below the samples, so walls have height).
        let flat = heightmap_solid(&parse_surface_text("2 2\n2 2\n"), false, false);
        assert!(is_closed_manifold(&flat) && raw_signed_volume(&flat) > 0.0, "flat grid non-degenerate");
        // Ragged rows pad with zeros to a rectangular grid.
        assert_eq!(parse_surface_text("1 2 3\n4 5\n")[1], vec![4.0, 5.0, 0.0]);
        // invert stays a valid outward closed solid.
        let mi = heightmap_solid(&grid, true, true);
        assert!(is_closed_manifold(&mi) && raw_signed_volume(&mi) > 0.0);
    }

    #[test]
    fn malicious_mesh_headers_do_not_oom_or_panic() {
        // A tiny binary STL claiming ~4 billion triangles must not allocate on
        // the untrusted count (it is bounded to what the bytes can hold).
        let mut bin = vec![0u8; 84];
        bin[80..84].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        assert!(read_stl(&bin).unwrap().tris.is_empty());
        // OFF with absurd counts / inf tokens must not OOM, overflow, or index
        // out of bounds — returning Err or an empty mesh are both acceptable
        // (reaching these asserts at all proves no panic/abort occurred).
        let ok = |r: Result<Mesh, String>| r.map_or(true, |m| m.tris.is_empty());
        assert!(ok(read_off("OFF\n4000000000 0 0\n")));
        assert!(ok(read_off("OFF\n0 4000000000 0\n")));
        assert!(ok(read_off("OFF\n1 1 0\n0 0 0\n1e300\n"))); // face-size token saturates usize
        assert!(ok(read_off("OFF\n1e309 0 0\n"))); // inf vertex count
    }

    #[test]
    fn format_detection_is_case_insensitive() {
        assert_eq!(format_from_ext("part.STL"), Some(MeshFormat::Stl));
        assert_eq!(format_from_ext("/a/b/Model.off"), Some(MeshFormat::Off));
        assert_eq!(format_from_ext("thing.3mf"), None);
    }
}
