//! Mesh import/export: STL (ASCII + binary) and OFF, from scratch.
//!
//! Geometry only — the 2021.01 mesh formats carry no units, names, or color
//! beyond fixed boilerplate, so a round-trip preserves the triangle set (up
//! to vertex welding), which is exactly the equivalence the reference
//! promises (mesh-level, never byte-level). `%` background geometry is
//! excluded from exports and `#` highlight geometry is included — that
//! selection happens in the caller, which hands us the already-merged solid.

use crate::geom::Mesh;
use crate::poly2::Poly2;
use std::collections::HashMap;

// -- 2D vector formats (SVG / DXF) ------------------------------------------

fn regions_bbox(regions: &[Poly2]) -> ([f64; 2], [f64; 2]) {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for r in regions {
        for c in &r.contours {
            for p in c {
                lo[0] = lo[0].min(p[0]);
                lo[1] = lo[1].min(p[1]);
                hi[0] = hi[0].max(p[0]);
                hi[1] = hi[1].max(p[1]);
            }
        }
    }
    if !lo[0].is_finite() {
        ([0.0, 0.0], [0.0, 0.0])
    } else {
        (lo, hi)
    }
}

/// Write a 2D region set as one SVG document: every contour becomes a closed
/// `<path>` subpath and the whole set is one filled path with
/// `fill-rule="evenodd"` (so holes read correctly). The Y axis is flipped so
/// the drawing appears upright in an SVG viewer, matching the import
/// convention. Coordinates are millimetres.
pub fn write_svg(regions: &[Poly2]) -> String {
    let (lo, hi) = regions_bbox(regions);
    let (w, h) = (hi[0] - lo[0], hi[1] - lo[1]);
    let mut d = String::new();
    for r in regions {
        for c in &r.contours {
            if c.len() < 3 {
                continue;
            }
            for (i, p) in c.iter().enumerate() {
                // Flip Y: SVG's axis points down.
                let (x, y) = (p[0], -p[1]);
                d.push_str(if i == 0 { "M" } else { "L" });
                d.push_str(&format!("{} {} ", fmt_coord(x), fmt_coord(y)));
            }
            d.push_str("Z ");
        }
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}mm\" height=\"{}mm\" \
         viewBox=\"{} {} {} {}\">\n\
         <path d=\"{}\" fill-rule=\"evenodd\" fill=\"lightgray\" stroke=\"black\" stroke-width=\"0.5\"/>\n\
         </svg>\n",
        fmt_coord(w),
        fmt_coord(h),
        fmt_coord(lo[0]),
        fmt_coord(-hi[1]),
        fmt_coord(w),
        fmt_coord(h),
        d.trim_end()
    )
}

/// Write a 2D region set as a minimal one-page vector PDF (new in 2021.01).
/// Zero-dependency and uncompressed — legal PDF. The page is sized to the
/// geometry's bounding box plus a small margin; coordinates convert from
/// millimetres to PostScript points (1 mm = 72/25.4 pt). PDF's Y axis points
/// up (like OpenSCAD), so no flip is needed. Every contour is one subpath and
/// the whole set is filled even-odd (holes read correctly) and stroked.
///
/// The cross-reference table needs exact byte offsets, so the document is
/// assembled object-by-object while tracking the running byte length. Output
/// is pure ASCII, so it rides the same text export path as SVG/DXF.
pub fn write_pdf(regions: &[Poly2]) -> String {
    const K: f64 = 72.0 / 25.4; // mm → pt
    const MARGIN: f64 = 5.0 * 72.0 / 25.4; // 5 mm in points
    let (lo, hi) = regions_bbox(regions);
    let w = (hi[0] - lo[0]) * K + 2.0 * MARGIN;
    let h = (hi[1] - lo[1]) * K + 2.0 * MARGIN;
    // Build the content stream: path ops in page points (origin bottom-left).
    let mut cs = String::from("0.8 0.8 0.8 rg 0 0 0 RG 0.5 w\n");
    for r in regions {
        for c in &r.contours {
            if c.len() < 3 {
                continue;
            }
            for (i, p) in c.iter().enumerate() {
                let x = (p[0] - lo[0]) * K + MARGIN;
                let y = (p[1] - lo[1]) * K + MARGIN;
                cs.push_str(&format!("{} {} {}\n", fmt_coord(x), fmt_coord(y), if i == 0 { "m" } else { "l" }));
            }
            cs.push_str("h\n");
        }
    }
    // Even-odd fill then stroke the same path.
    cs.push_str("B*\n");

    // Assemble objects, recording each one's byte offset for the xref table.
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents 4 0 R /Resources << >> >>",
            fmt_coord(w),
            fmt_coord(h)
        ),
        format!("<< /Length {} >>\nstream\n{}endstream", cs.len(), cs),
    ];
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.push_str(&format!("{} 0 obj\n{}\nendobj\n", i + 1, body));
    }
    let xref_pos = out.len();
    let n = objects.len() + 1; // +1 for the free object 0
    out.push_str(&format!("xref\n0 {}\n", n));
    out.push_str("0000000000 65535 f \n");
    for off in &offsets {
        out.push_str(&format!("{:010} 00000 n \n", off));
    }
    out.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        n, xref_pos
    ));
    out
}

/// Write a 2D region set as a minimal DXF: one closed LWPOLYLINE entity per
/// contour (the entity CAM tools read most reliably). Millimetre units.
pub fn write_dxf_2d(regions: &[Poly2]) -> String {
    let mut s = String::from("0\nSECTION\n2\nENTITIES\n");
    for r in regions {
        for c in &r.contours {
            if c.len() < 3 {
                continue;
            }
            s.push_str("0\nLWPOLYLINE\n8\n0\n"); // entity, layer 0
            s.push_str(&format!("90\n{}\n70\n1\n", c.len())); // vertex count, closed
            for p in c {
                s.push_str(&format!("10\n{}\n20\n{}\n", fmt_coord(p[0]), fmt_coord(p[1])));
            }
        }
    }
    s.push_str("0\nENDSEC\n0\nEOF\n");
    s
}

/// Read a DXF into a 2D region: LINE / LWPOLYLINE / POLYLINE edges are
/// stitched into closed loops by endpoint matching, and CIRCLE / ARC are
/// tessellated with the given fragment parameters. Unsupported entities are
/// skipped with a warning. Z is ignored (2D only).
pub fn read_dxf(text: &str, fn_: f64, fa: f64, fs: f64) -> (Poly2, Vec<String>) {
    let mut warnings = Vec::new();
    let mut contours: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut segs: Vec<([f64; 2], [f64; 2])> = Vec::new();
    let mut unsupported: std::collections::HashSet<String> = std::collections::HashSet::new();

    // DXF is a stream of (group-code, value) line pairs.
    let mut pairs: Vec<(i64, String)> = Vec::new();
    let mut lines = text.lines();
    while let (Some(code), Some(val)) = (lines.next(), lines.next()) {
        if let Ok(c) = code.trim().parse::<i64>() {
            pairs.push((c, val.trim().to_string()));
        }
    }
    // Walk entities: an entity starts at a `0` code naming its type.
    let mut i = 0;
    while i < pairs.len() {
        if pairs[i].0 != 0 {
            i += 1;
            continue;
        }
        let etype = pairs[i].1.to_ascii_uppercase();
        i += 1;
        // Collect this entity's codes until the next `0`.
        let start = i;
        while i < pairs.len() && pairs[i].0 != 0 {
            i += 1;
        }
        let body = &pairs[start..i];
        match etype.as_str() {
            "LINE" => {
                let (mut x1, mut y1, mut x2, mut y2) = (0.0, 0.0, 0.0, 0.0);
                for (c, v) in body {
                    let f = v.parse::<f64>().unwrap_or(0.0);
                    match c {
                        10 => x1 = f,
                        20 => y1 = f,
                        11 => x2 = f,
                        21 => y2 = f,
                        _ => {}
                    }
                }
                segs.push(([x1, y1], [x2, y2]));
            }
            "LWPOLYLINE" | "POLYLINE" => {
                let mut verts: Vec<[f64; 2]> = Vec::new();
                let mut closed = false;
                let mut cur_x = None;
                for (c, v) in body {
                    let f = v.parse::<f64>().unwrap_or(0.0);
                    match c {
                        70 => closed = (f as i64) & 1 != 0,
                        10 => cur_x = Some(f),
                        20 => {
                            if let Some(x) = cur_x.take() {
                                verts.push([x, f]);
                            }
                        }
                        _ => {}
                    }
                }
                if verts.len() >= 2 {
                    if closed && verts.len() >= 3 {
                        contours.push(verts);
                    } else {
                        for w in verts.windows(2) {
                            segs.push((w[0], w[1]));
                        }
                    }
                }
            }
            "CIRCLE" => {
                let (mut cx, mut cy, mut r) = (0.0, 0.0, 0.0);
                for (c, v) in body {
                    let f = v.parse::<f64>().unwrap_or(0.0);
                    match c {
                        10 => cx = f,
                        20 => cy = f,
                        40 => r = f,
                        _ => {}
                    }
                }
                if r > 0.0 {
                    let n = crate::geom::fragments(r, fn_, fa, fs).max(3);
                    let ring = (0..n)
                        .map(|k| {
                            let a = std::f64::consts::TAU * k as f64 / n as f64;
                            [cx + r * a.cos(), cy + r * a.sin()]
                        })
                        .collect();
                    contours.push(ring);
                }
            }
            "ARC" => {
                let (mut cx, mut cy, mut r, mut a0, mut a1) = (0.0, 0.0, 0.0, 0.0, 0.0);
                for (c, v) in body {
                    let f = v.parse::<f64>().unwrap_or(0.0);
                    match c {
                        10 => cx = f,
                        20 => cy = f,
                        40 => r = f,
                        50 => a0 = f,
                        51 => a1 = f,
                        _ => {}
                    }
                }
                if r > 0.0 {
                    let sweep = if a1 >= a0 { a1 - a0 } else { a1 + 360.0 - a0 };
                    let n = (crate::geom::fragments(r, fn_, fa, fs) as f64 * sweep / 360.0)
                        .ceil()
                        .max(1.0) as u32;
                    let mut prev: Option<[f64; 2]> = None;
                    for k in 0..=n {
                        let a = (a0 + sweep * k as f64 / n as f64).to_radians();
                        let p = [cx + r * a.cos(), cy + r * a.sin()];
                        if let Some(q) = prev {
                            segs.push((q, p));
                        }
                        prev = Some(p);
                    }
                }
            }
            "SECTION" | "ENDSEC" | "EOF" | "TABLE" | "ENDTAB" | "VERTEX" | "SEQEND"
            | "BLOCK" | "ENDBLK" | "" => {}
            other => {
                unsupported.insert(other.to_string());
            }
        }
    }
    // Stitch the loose segments (LINE / ARC / open polylines) into closed loops.
    contours.extend(stitch_loops(&segs));
    for u in unsupported {
        warnings.push(format!("WARNING: DXF entity '{}' is not supported; skipped.", u));
    }
    (Poly2::new(contours), warnings)
}

/// Chain undirected segments into closed loops by endpoint matching (grid-
/// quantized). Open chains are dropped.
fn stitch_loops(segs: &[([f64; 2], [f64; 2])]) -> Vec<Vec<[f64; 2]>> {
    const SNAP: f64 = 1e-4;
    let key = |p: [f64; 2]| ((p[0] / SNAP).round() as i64, (p[1] / SNAP).round() as i64);
    // adjacency: point key -> list of (segment index, endpoint 0/1)
    let mut adj: HashMap<(i64, i64), Vec<(usize, usize)>> = HashMap::new();
    for (i, s) in segs.iter().enumerate() {
        if key(s.0) == key(s.1) {
            continue; // zero-length
        }
        adj.entry(key(s.0)).or_default().push((i, 0));
        adj.entry(key(s.1)).or_default().push((i, 1));
    }
    let ends = |s: &([f64; 2], [f64; 2]), e: usize| if e == 0 { (s.0, s.1) } else { (s.1, s.0) };
    let mut used = vec![false; segs.len()];
    let mut loops = Vec::new();
    for start in 0..segs.len() {
        if used[start] {
            continue;
        }
        let (a0, _) = ends(&segs[start], 0);
        let mut loop_pts = vec![a0];
        let mut cur = start;
        let mut from_end = 0usize;
        let mut ok = false;
        for _ in 0..segs.len() + 1 {
            used[cur] = true;
            let (_, tail) = ends(&segs[cur], from_end);
            loop_pts.push(tail);
            if key(tail) == key(a0) {
                loop_pts.pop(); // closed: drop the duplicated start
                ok = loop_pts.len() >= 3;
                break;
            }
            // find an unused segment continuing from `tail`
            let next = adj.get(&key(tail)).and_then(|cands| {
                cands.iter().find(|&&(si, _)| !used[si]).copied()
            });
            match next {
                Some((si, e)) => {
                    cur = si;
                    from_end = e;
                }
                None => break, // open chain — dropped
            }
        }
        if ok {
            loops.push(loop_pts);
        }
    }
    loops
}

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

/// AMF (XML mesh): one object with a vertex list and a triangle volume.
/// Geometry only — no units/color metadata (unit attribute is millimetre).
pub fn write_amf(mesh: &Mesh) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<amf unit=\"millimeter\">\n  <object id=\"0\">\n    <mesh>\n      <vertices>\n",
    );
    for p in &mesh.positions {
        s.push_str(&format!(
            "        <vertex><coordinates><x>{}</x><y>{}</y><z>{}</z></coordinates></vertex>\n",
            fmt_coord(p[0]),
            fmt_coord(p[1]),
            fmt_coord(p[2])
        ));
    }
    s.push_str("      </vertices>\n      <volume>\n");
    for t in &mesh.tris {
        s.push_str(&format!(
            "        <triangle><v1>{}</v1><v2>{}</v2><v3>{}</v3></triangle>\n",
            t[0], t[1], t[2]
        ));
    }
    s.push_str("      </volume>\n    </mesh>\n  </object>\n</amf>\n");
    s
}

/// Read an AMF mesh with a minimal tag scanner (no XML dependency): every
/// `<vertex>` contributes a point, every `<triangle>` a face; all objects and
/// volumes union into one mesh. Out-of-range triangle indices are dropped.
pub fn read_amf(text: &str) -> Mesh {
    fn between(s: &str, open: &str, close: &str) -> Option<f64> {
        let a = s.find(open)? + open.len();
        let b = s[a..].find(close)? + a;
        s[a..b].trim().parse().ok()
    }
    let mut positions = Vec::new();
    let mut scan = text;
    while let Some(p) = scan.find("<vertex>") {
        let rest = &scan[p + 8..];
        let end = rest.find("</vertex>").unwrap_or(rest.len());
        let chunk = &rest[..end];
        match (between(chunk, "<x>", "</x>"), between(chunk, "<y>", "</y>"), between(chunk, "<z>", "</z>")) {
            (Some(x), Some(y), Some(z)) if x.is_finite() && y.is_finite() && z.is_finite() => {
                positions.push([x, y, z]);
            }
            _ => {}
        }
        scan = &rest[end..];
    }
    let n = positions.len();
    let mut tris = Vec::new();
    let mut scan = text;
    while let Some(p) = scan.find("<triangle>") {
        let rest = &scan[p + 10..];
        let end = rest.find("</triangle>").unwrap_or(rest.len());
        let chunk = &rest[..end];
        let idx = |tag_o: &str, tag_c: &str| between(chunk, tag_o, tag_c).map(|v| v as usize);
        if let (Some(a), Some(b), Some(c)) = (idx("<v1>", "</v1>"), idx("<v2>", "</v2>"), idx("<v3>", "</v3>")) {
            if a < n && b < n && c < n && a != b && b != c && a != c {
                tris.push([a as u32, b as u32, c as u32]);
            }
        }
        scan = &rest[end..];
    }
    Mesh { positions, tris }
}

/// Write a mesh as a `.3mf` (3D Manufacturing Format): an OPC/ZIP container
/// holding `[Content_Types].xml`, `_rels/.rels`, and `3D/3dmodel.model`. The
/// model XML lists the mesh's vertices and triangles (geometry only, matching
/// 2021.01's 3MF support). Entries are stored uncompressed — valid ZIP that any
/// conforming reader accepts.
pub fn write_3mf(mesh: &Mesh) -> Vec<u8> {
    let mut m = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    m.push_str(
        "<model unit=\"millimeter\" xml:lang=\"en-US\" \
         xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\">\n \
         <resources>\n  <object id=\"1\" type=\"model\">\n   <mesh>\n    <vertices>\n",
    );
    for p in &mesh.positions {
        m.push_str(&format!(
            "     <vertex x=\"{}\" y=\"{}\" z=\"{}\"/>\n",
            fmt_coord(p[0]),
            fmt_coord(p[1]),
            fmt_coord(p[2])
        ));
    }
    m.push_str("    </vertices>\n    <triangles>\n");
    for t in &mesh.tris {
        m.push_str(&format!("     <triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>\n", t[0], t[1], t[2]));
    }
    m.push_str(
        "    </triangles>\n   </mesh>\n  </object>\n </resources>\n \
         <build>\n  <item objectid=\"1\"/>\n </build>\n</model>\n",
    );

    const CONTENT_TYPES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\n\
        <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\n\
        <Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/>\n\
        </Types>\n";
    const RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n\
        <Relationship Target=\"/3D/3dmodel.model\" Id=\"rel0\" \
        Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/>\n\
        </Relationships>\n";

    crate::zip::write_zip(&[
        ("[Content_Types].xml", CONTENT_TYPES.as_bytes()),
        ("_rels/.rels", RELS.as_bytes()),
        ("3D/3dmodel.model", m.as_bytes()),
    ])
}

/// Read a `.3mf`: unzip (inflating deflated parts), find the `.model` part, and
/// parse its `<vertex>`/`<triangle>` elements into one mesh. Geometry only.
/// Out-of-range or degenerate triangles and non-finite vertices are dropped,
/// mirroring the AMF reader.
pub fn read_3mf(bytes: &[u8]) -> Result<Mesh, String> {
    let entries = crate::zip::read_zip(bytes);
    if entries.is_empty() {
        return Err("empty or unreadable 3MF archive".into());
    }
    let model = entries
        .iter()
        .find(|(n, _)| n.to_ascii_lowercase().ends_with(".model"))
        .map(|(_, b)| String::from_utf8_lossy(b).into_owned())
        .ok_or_else(|| "3MF archive has no .model part".to_string())?;
    Ok(parse_3mf_model(&model))
}

/// Parse the 3MF model XML's vertices and triangles (attribute-based).
fn parse_3mf_model(xml: &str) -> Mesh {
    // Extract the attribute substring of each `<tag ...>` occurrence and read
    // its attributes via the shared, boundary-safe attribute reader.
    let attr = crate::svg::attr;
    let num = |chunk: &str, k: &str| attr(chunk, k).and_then(|v| v.trim().parse::<f64>().ok());

    let mut positions = Vec::new();
    let mut scan = xml;
    while let Some(p) = scan.find("<vertex") {
        let rest = &scan[p + 7..];
        let end = rest.find('>').unwrap_or(rest.len());
        let chunk = &rest[..end];
        if let (Some(x), Some(y), Some(z)) = (num(chunk, "x"), num(chunk, "y"), num(chunk, "z")) {
            if x.is_finite() && y.is_finite() && z.is_finite() {
                positions.push([x, y, z]);
            }
        }
        scan = &rest[end..];
    }
    let n = positions.len();
    let mut tris = Vec::new();
    let mut scan = xml;
    while let Some(p) = scan.find("<triangle") {
        let rest = &scan[p + 9..];
        let end = rest.find('>').unwrap_or(rest.len());
        let chunk = &rest[..end];
        let idx = |k: &str| attr(chunk, k).and_then(|v| v.trim().parse::<usize>().ok());
        if let (Some(a), Some(b), Some(c)) = (idx("v1"), idx("v2"), idx("v3")) {
            if a < n && b < n && c < n && a != b && b != c && a != c {
                tris.push([a as u32, b as u32, c as u32]);
            }
        }
        scan = &rest[end..];
    }
    Mesh { positions, tris }
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
    Amf,
}

pub fn format_from_ext(path: &str) -> Option<MeshFormat> {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "stl" => Some(MeshFormat::Stl),
        "off" => Some(MeshFormat::Off),
        "amf" => Some(MeshFormat::Amf),
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
    fn amf_round_trips_a_cube() {
        let cube = geom::cube([2.0, 3.0, 4.0], false); // volume 24
        let xml = write_amf(&cube);
        assert!(xml.contains("<amf") && xml.contains("<triangle>"));
        let back = read_amf(&xml);
        assert_eq!(tri_count(&back), 12);
        assert_eq!(back.positions.len(), 8);
        assert!((signed_volume(&back) - 24.0).abs() < 1e-6, "vol {}", signed_volume(&back));
        // A triangle referencing an out-of-range vertex is dropped.
        let bad = "<amf><object><mesh><vertices>\
            <vertex><coordinates><x>0</x><y>0</y><z>0</z></coordinates></vertex>\
            </vertices><volume><triangle><v1>0</v1><v2>1</v2><v3>99</v3></triangle></volume>\
            </mesh></object></amf>";
        assert_eq!(tri_count(&read_amf(bad)), 0);
    }

    #[test]
    fn threemf_round_trips_a_cube() {
        let cube = geom::cube([2.0, 3.0, 4.0], false); // volume 24
        let bytes = write_3mf(&cube);
        // It is a real ZIP with the three OPC parts.
        let parts = crate::zip::read_zip(&bytes);
        assert!(parts.iter().any(|(n, _)| n == "[Content_Types].xml"));
        assert!(parts.iter().any(|(n, _)| n == "_rels/.rels"));
        assert!(parts.iter().any(|(n, _)| n == "3D/3dmodel.model"));
        // And it reads back to the same mesh.
        let back = read_3mf(&bytes).unwrap();
        assert_eq!(tri_count(&back), 12);
        assert_eq!(back.positions.len(), 8);
        assert!((signed_volume(&back) - 24.0).abs() < 1e-6, "vol {}", signed_volume(&back));
        // Reading garbage is an error, not a panic.
        assert!(read_3mf(b"not a zip").is_err());
        // A triangle referencing an out-of-range vertex is dropped.
        let onlyvtx = write_3mf(&Mesh { positions: vec![[0.0; 3]], tris: vec![] });
        assert_eq!(tri_count(&read_3mf(&onlyvtx).unwrap()), 0);
    }

    #[test]
    fn reads_a_deflate_compressed_3mf_from_python() {
        // A real .3mf produced by Python's zipfile with ZIP_DEFLATED — a
        // tetrahedron (4 vertices, 4 triangles). Pins the full import stack:
        // ZIP central directory → inflate → 3MF model-XML attribute parse.
        let z: &[u8] = &[
            0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x89, 0x7e, 0x24, 0x5d,
            0xee, 0x47, 0x58, 0x66, 0x1f, 0x00, 0x00, 0x00, 0x1d, 0x00, 0x00, 0x00, 0x13, 0x00,
            0x00, 0x00, 0x5b, 0x43, 0x6f, 0x6e, 0x74, 0x65, 0x6e, 0x74, 0x5f, 0x54, 0x79, 0x70,
            0x65, 0x73, 0x5d, 0x2e, 0x78, 0x6d, 0x6c, 0xb3, 0xb1, 0xaf, 0xc8, 0xcd, 0x51, 0x28,
            0x4b, 0x2d, 0x2a, 0xce, 0xcc, 0xcf, 0xb3, 0x55, 0x32, 0xd4, 0x33, 0x50, 0xb2, 0xb7,
            0xb3, 0x09, 0xa9, 0x2c, 0x48, 0x2d, 0xd6, 0xb7, 0x03, 0x00, 0x50, 0x4b, 0x03, 0x04,
            0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x89, 0x7e, 0x24, 0x5d, 0x61, 0x72, 0x5a, 0x9e,
            0x26, 0x00, 0x00, 0x00, 0x25, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x5f, 0x72,
            0x65, 0x6c, 0x73, 0x2f, 0x2e, 0x72, 0x65, 0x6c, 0x73, 0xb3, 0xb1, 0xaf, 0xc8, 0xcd,
            0x51, 0x28, 0x4b, 0x2d, 0x2a, 0xce, 0xcc, 0xcf, 0xb3, 0x55, 0x32, 0xd4, 0x33, 0x50,
            0xb2, 0xb7, 0xb3, 0x09, 0x4a, 0xcd, 0x49, 0x2c, 0x01, 0x0a, 0x14, 0x67, 0x64, 0x16,
            0x14, 0xeb, 0xdb, 0x01, 0x00, 0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x89, 0x7e, 0x24, 0x5d, 0x50, 0xc9, 0x0c, 0x87, 0xb9, 0x00, 0x00, 0x00, 0xba,
            0x01, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x33, 0x44, 0x2f, 0x33, 0x64, 0x6d, 0x6f,
            0x64, 0x65, 0x6c, 0x2e, 0x6d, 0x6f, 0x64, 0x65, 0x6c, 0x75, 0x90, 0x5d, 0x0e, 0x83,
            0x20, 0x10, 0x84, 0xaf, 0x42, 0x38, 0x40, 0x11, 0x7d, 0x05, 0x3c, 0x4b, 0xd5, 0x4d,
            0xbb, 0x0d, 0x48, 0x03, 0x68, 0xb4, 0xa7, 0x2f, 0x3f, 0x26, 0xa5, 0x69, 0x7d, 0x59,
            0x36, 0xf9, 0x66, 0x66, 0xc9, 0x88, 0x7e, 0x33, 0x9a, 0xac, 0xe0, 0x3c, 0xda, 0x59,
            0x52, 0x7e, 0x69, 0x68, 0xaf, 0x84, 0xb1, 0x13, 0x68, 0xb2, 0xcc, 0x18, 0x24, 0x35,
            0xa8, 0x35, 0x1a, 0x08, 0xe0, 0xa8, 0x12, 0x0e, 0xbc, 0x5d, 0xdc, 0x08, 0x5e, 0x09,
            0x3b, 0x3c, 0x60, 0x0c, 0x04, 0xa7, 0xe8, 0xa2, 0x24, 0xec, 0x4f, 0x88, 0xda, 0xe4,
            0x8b, 0x32, 0x03, 0xfe, 0xae, 0x44, 0x4c, 0x0d, 0x98, 0xb5, 0x69, 0x83, 0x8d, 0x6c,
            0x92, 0x36, 0x94, 0xec, 0x79, 0xbe, 0xd2, 0x64, 0x35, 0xe2, 0xe7, 0xa8, 0xb8, 0xf8,
            0x39, 0x2a, 0x2e, 0x9e, 0x10, 0xfb, 0x9c, 0x0d, 0x0e, 0xaf, 0xf3, 0x4d, 0xd7, 0x2b,
            0x59, 0x79, 0x56, 0xaf, 0xad, 0xa4, 0x6d, 0x7c, 0xba, 0xc3, 0xf5, 0x8f, 0xf3, 0xc2,
            0xbb, 0x33, 0xde, 0x15, 0xde, 0xfe, 0x70, 0xfe, 0x95, 0x9f, 0xfd, 0xac, 0xfa, 0x0b,
            0x2b, 0xf5, 0xb0, 0xd2, 0x60, 0x5c, 0xaa, 0x56, 0x87, 0x05, 0xf5, 0xa4, 0x04, 0x06,
            0x30, 0xa4, 0xf0, 0x52, 0x70, 0x8a, 0x38, 0x18, 0xcb, 0x2d, 0xab, 0x37, 0x50, 0x4b,
            0x01, 0x02, 0x14, 0x03, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x89, 0x7e, 0x24, 0x5d,
            0xee, 0x47, 0x58, 0x66, 0x1f, 0x00, 0x00, 0x00, 0x1d, 0x00, 0x00, 0x00, 0x13, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x5b, 0x43, 0x6f, 0x6e, 0x74, 0x65, 0x6e, 0x74, 0x5f, 0x54, 0x79, 0x70,
            0x65, 0x73, 0x5d, 0x2e, 0x78, 0x6d, 0x6c, 0x50, 0x4b, 0x01, 0x02, 0x14, 0x03, 0x14,
            0x00, 0x00, 0x00, 0x08, 0x00, 0x89, 0x7e, 0x24, 0x5d, 0x61, 0x72, 0x5a, 0x9e, 0x26,
            0x00, 0x00, 0x00, 0x25, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x50, 0x00, 0x00, 0x00, 0x5f, 0x72, 0x65,
            0x6c, 0x73, 0x2f, 0x2e, 0x72, 0x65, 0x6c, 0x73, 0x50, 0x4b, 0x01, 0x02, 0x14, 0x03,
            0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x89, 0x7e, 0x24, 0x5d, 0x50, 0xc9, 0x0c, 0x87,
            0xb9, 0x00, 0x00, 0x00, 0xba, 0x01, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x9f, 0x00, 0x00, 0x00, 0x33, 0x44,
            0x2f, 0x33, 0x64, 0x6d, 0x6f, 0x64, 0x65, 0x6c, 0x2e, 0x6d, 0x6f, 0x64, 0x65, 0x6c,
            0x50, 0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x03, 0x00, 0xb8, 0x00,
            0x00, 0x00, 0x86, 0x01, 0x00, 0x00, 0x00, 0x00,
        ];
        let m = read_3mf(z).expect("deflate-compressed 3MF reads");
        assert_eq!(m.positions.len(), 4, "tetra vertices");
        assert_eq!(tri_count(&m), 4, "tetra triangles");
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

    fn area2(poly: &Poly2) -> f64 {
        let (v, t) = crate::poly2::triangulate(poly);
        t.iter()
            .map(|tri| {
                let (a, b, c) = (v[tri[0] as usize], v[tri[1] as usize], v[tri[2] as usize]);
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.0
            })
            .sum()
    }

    #[test]
    fn dxf_round_trips_a_square_with_a_hole() {
        // A 10×10 square with a 4×4 hole (area 84).
        let ring = Poly2::new(vec![
            vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            vec![[3.0, 3.0], [7.0, 3.0], [7.0, 7.0], [3.0, 7.0]],
        ]);
        let dxf = write_dxf_2d(std::slice::from_ref(&ring));
        assert!(dxf.contains("LWPOLYLINE") && dxf.contains("EOF"));
        let (back, warns) = read_dxf(&dxf, 0.0, 12.0, 2.0);
        assert!(warns.is_empty());
        assert_eq!(back.contours.len(), 2, "outer + hole");
        assert!((area2(&back) - 84.0).abs() < 1e-6, "area {}", area2(&back));
    }

    #[test]
    fn dxf_stitches_lines_into_a_loop_and_tessellates_circles() {
        // Four LINE entities forming a 5×5 square, plus a CIRCLE.
        let dxf = "0\nSECTION\n2\nENTITIES\n\
            0\nLINE\n10\n0\n20\n0\n11\n5\n21\n0\n\
            0\nLINE\n10\n5\n20\n0\n11\n5\n21\n5\n\
            0\nLINE\n10\n5\n20\n5\n11\n0\n21\n5\n\
            0\nLINE\n10\n0\n20\n5\n11\n0\n21\n0\n\
            0\nCIRCLE\n10\n20\n20\n0\n40\n3\n\
            0\nSPLINE\n\
            0\nENDSEC\n0\nEOF\n";
        let (poly, warns) = read_dxf(dxf, 32.0, 12.0, 2.0);
        assert_eq!(poly.contours.len(), 2, "the square loop + the circle");
        // area = 25 (square) + ~π·9 (circle) ≈ 53.3
        assert!((area2(&poly) - (25.0 + std::f64::consts::PI * 9.0)).abs() < 0.5, "area {}", area2(&poly));
        assert!(warns.iter().any(|w| w.contains("SPLINE")), "unsupported entity warns");
    }

    #[test]
    fn svg_export_emits_a_filled_evenodd_path() {
        let ring = Poly2::new(vec![
            vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            vec![[3.0, 3.0], [7.0, 3.0], [7.0, 7.0], [3.0, 7.0]],
        ]);
        let svg = write_svg(std::slice::from_ref(&ring));
        assert!(svg.contains("<svg") && svg.contains("fill-rule=\"evenodd\""));
        assert!(svg.contains("width=\"10mm\"") && svg.contains("height=\"10mm\""));
        // Two subpaths (outer + hole), each closed with Z.
        assert_eq!(svg.matches('Z').count(), 2);
    }

    #[test]
    fn format_detection_is_case_insensitive() {
        assert_eq!(format_from_ext("part.STL"), Some(MeshFormat::Stl));
        assert_eq!(format_from_ext("/a/b/Model.off"), Some(MeshFormat::Off));
        assert_eq!(format_from_ext("thing.3mf"), None);
    }

    #[test]
    fn pdf_export_is_well_formed_with_correct_xref_offsets() {
        let ring = Poly2::new(vec![
            vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            vec![[3.0, 3.0], [7.0, 3.0], [7.0, 7.0], [3.0, 7.0]],
        ]);
        let pdf = write_pdf(std::slice::from_ref(&ring));
        assert!(pdf.starts_with("%PDF-1.4"));
        assert!(pdf.trim_end().ends_with("%%EOF"));
        assert!(pdf.contains("/MediaBox"));
        assert!(pdf.contains("B*")); // even-odd fill + stroke
        // Two closed subpaths (outer + hole).
        assert_eq!(pdf.matches(" h\n").count() + pdf.matches("\nh\n").count(), 2);

        // The xref offsets must point exactly at each "N 0 obj" — a wrong offset
        // makes the PDF unreadable, so verify the table against the bytes.
        let xref_pos = pdf.rfind("\nxref\n").unwrap() + 1;
        let table = &pdf[xref_pos..];
        let lines: Vec<&str> = table.lines().collect();
        // lines: "xref", "0 5", "0000000000 65535 f", then 4 object entries.
        assert_eq!(lines[1], "0 5");
        for (i, entry) in lines[3..7].iter().enumerate() {
            let off: usize = entry.split_whitespace().next().unwrap().parse().unwrap();
            let expect = format!("{} 0 obj", i + 1);
            assert!(pdf[off..].starts_with(&expect), "obj {} at offset {}: {:?}", i + 1, off, &pdf[off..off + 10]);
        }
        // startxref must point at the "xref" keyword.
        let sx: usize = pdf.rsplit("startxref\n").next().unwrap().split_whitespace().next().unwrap().parse().unwrap();
        assert!(pdf[sx..].starts_with("xref\n"), "startxref points at xref");

        // The stream /Length must equal the actual byte count between
        // `stream\n` and `endstream` — a wrong length makes the PDF unreadable.
        let len_kw = pdf.find("/Length ").unwrap();
        let declared: usize = pdf[len_kw + 8..].split_whitespace().next().unwrap().parse().unwrap();
        let s0 = pdf.find("stream\n").unwrap() + "stream\n".len();
        let s1 = pdf.find("endstream").unwrap();
        assert_eq!(declared, s1 - s0, "declared /Length matches the stream bytes");
    }
}
