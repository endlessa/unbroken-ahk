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
        // but failed the size check falls back to binary.
        read_stl_ascii(bytes).or_else(|_| read_stl_binary(bytes))
    } else {
        read_stl_binary(bytes)
    }
}

fn read_stl_binary(bytes: &[u8]) -> Result<Mesh, String> {
    if bytes.len() < 84 {
        return Err("binary STL too short".into());
    }
    let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    let mut w = Welder::new();
    let mut tris = Vec::with_capacity(count);
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
    let nv = toks[0] as usize;
    let nf = toks[1] as usize;
    // vertices start at index 3 (after nv nf ne)
    let mut idx = 3;
    let mut positions = Vec::with_capacity(nv);
    for _ in 0..nv {
        if idx + 3 > toks.len() {
            return Err("OFF: truncated vertex list".into());
        }
        positions.push([toks[idx], toks[idx + 1], toks[idx + 2]]);
        idx += 3;
    }
    let mut tris = Vec::with_capacity(nf);
    for _ in 0..nf {
        if idx >= toks.len() {
            break;
        }
        let k = toks[idx] as usize;
        idx += 1;
        if idx + k > toks.len() {
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
/// triangles so they can't seed downstream CSG errors.
fn push_triangle(w: &mut Welder, tris: &mut Vec<[u32; 3]>, a: [f64; 3], b: [f64; 3], c: [f64; 3]) {
    if triangle_normal(a, b, c) == [0.0, 0.0, 0.0] {
        return; // zero-area / collinear
    }
    let (ia, ib, ic) = (w.intern(a), w.intern(b), w.intern(c));
    if ia != ib && ib != ic && ia != ic {
        tris.push([ia, ib, ic]);
    }
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

    #[test]
    fn format_detection_is_case_insensitive() {
        assert_eq!(format_from_ext("part.STL"), Some(MeshFormat::Stl));
        assert_eq!(format_from_ext("/a/b/Model.off"), Some(MeshFormat::Off));
        assert_eq!(format_from_ext("thing.3mf"), None);
    }
}
