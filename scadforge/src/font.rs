//! From-scratch TrueType glyph-outline parser.
//!
//! Enough of the sfnt/`glyf` format to turn a character into flattened 2D
//! contours in font units: the table directory, `head` (units-per-em, loca
//! format), `maxp` (glyph count), `hhea`+`hmtx` (advance widths), a `cmap`
//! (format 4 or 12) for character→glyph mapping, `loca` (glyph offsets), and
//! `glyf` (simple + composite outlines, quadratic Béziers flattened to line
//! segments). No hinting, no shaping — advances come straight from `hmtx`.
//! Zero dependencies; the default face (Instrument Sans, SIL OFL) is embedded.

use std::collections::HashMap;
use std::sync::OnceLock;

/// The bundled default sans face (SIL Open Font License — see fonts/OFL.txt).
const DEFAULT_TTF: &[u8] = include_bytes!("../fonts/InstrumentSans-Regular.ttf");

pub struct Font {
    data: Vec<u8>,
    pub units_per_em: f64,
    pub ascent: f64,
    pub descent: f64,
    num_glyphs: u16,
    loca: Vec<u32>,
    glyf: usize,
    /// glyph id → advance width in font units.
    advances: Vec<u16>,
    cmap: HashMap<u32, u16>,
}

fn be16(d: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([d[o], d[o + 1]])
}
fn bei16(d: &[u8], o: usize) -> i16 {
    i16::from_be_bytes([d[o], d[o + 1]])
}
fn be32(d: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

impl Font {
    pub fn parse(data: Vec<u8>) -> Option<Font> {
        if data.len() < 12 {
            return None;
        }
        let num_tables = be16(&data, 4) as usize;
        let mut tables: HashMap<[u8; 4], (usize, usize)> = HashMap::new();
        for i in 0..num_tables {
            let rec = 12 + i * 16;
            if rec + 16 > data.len() {
                return None;
            }
            let tag = [data[rec], data[rec + 1], data[rec + 2], data[rec + 3]];
            let off = be32(&data, rec + 8) as usize;
            let len = be32(&data, rec + 12) as usize;
            tables.insert(tag, (off, len));
        }
        let get = |t: &[u8; 4]| tables.get(t).copied();
        let (head, _) = get(b"head")?;
        let (maxp, _) = get(b"maxp")?;
        let (hhea, _) = get(b"hhea")?;
        let (hmtx, _) = get(b"hmtx")?;
        let (loca_off, _) = get(b"loca")?;
        let (glyf, _) = get(b"glyf")?;
        let (cmap_off, _) = get(b"cmap")?;

        let units_per_em = be16(&data, head + 18) as f64;
        let index_to_loc = bei16(&data, head + 50); // 0 = short, 1 = long
        let ascent = bei16(&data, hhea + 4) as f64;
        let descent = bei16(&data, hhea + 6) as f64;
        let num_h_metrics = be16(&data, hhea + 34) as usize;
        let num_glyphs = be16(&data, maxp + 4);

        // loca: numGlyphs + 1 offsets into glyf.
        let mut loca = Vec::with_capacity(num_glyphs as usize + 1);
        for i in 0..=num_glyphs as usize {
            let v = if index_to_loc == 0 {
                be16(&data, loca_off + i * 2) as u32 * 2
            } else {
                be32(&data, loca_off + i * 4)
            };
            loca.push(v);
        }

        // hmtx advances: the first num_h_metrics are explicit; the rest reuse
        // the last advance (monospaced tail).
        let mut advances = Vec::with_capacity(num_glyphs as usize);
        let mut last = 0u16;
        for g in 0..num_glyphs as usize {
            if g < num_h_metrics {
                last = be16(&data, hmtx + g * 4);
            }
            advances.push(last);
        }

        let cmap = parse_cmap(&data, cmap_off).unwrap_or_default();

        Some(Font {
            data,
            units_per_em: if units_per_em > 0.0 { units_per_em } else { 1000.0 },
            ascent,
            descent,
            num_glyphs,
            loca,
            glyf,
            advances,
            cmap,
        })
    }

    pub fn glyph_id(&self, ch: char) -> u16 {
        self.cmap.get(&(ch as u32)).copied().unwrap_or(0)
    }

    pub fn advance(&self, gid: u16) -> f64 {
        self.advances.get(gid as usize).copied().unwrap_or(0) as f64
    }

    /// Flattened contours of a glyph in font units, curves subdivided into
    /// `steps` segments each. Composite glyphs are resolved recursively.
    pub fn glyph_contours(&self, gid: u16, steps: usize) -> Vec<Vec<[f64; 2]>> {
        let mut out = Vec::new();
        self.append_glyph(gid, steps, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], 0, &mut out);
        out
    }

    fn append_glyph(
        &self,
        gid: u16,
        steps: usize,
        m: [f64; 6],
        depth: usize,
        out: &mut Vec<Vec<[f64; 2]>>,
    ) {
        if depth > 8 || gid >= self.num_glyphs {
            return;
        }
        let (start, end) = (self.loca[gid as usize] as usize, self.loca[gid as usize + 1] as usize);
        if end <= start {
            return; // empty glyph (e.g. space)
        }
        let g = self.glyf + start;
        let d = &self.data;
        if g + 10 > d.len() {
            return;
        }
        let n_contours = bei16(d, g);
        if n_contours >= 0 {
            self.simple_glyph(g, n_contours as usize, steps, m, out);
        } else {
            self.composite_glyph(g, steps, m, depth, out);
        }
    }

    fn simple_glyph(&self, g: usize, n: usize, steps: usize, m: [f64; 6], out: &mut Vec<Vec<[f64; 2]>>) {
        let d = &self.data;
        let mut o = g + 10;
        let mut ends = Vec::with_capacity(n);
        for _ in 0..n {
            ends.push(be16(d, o) as usize);
            o += 2;
        }
        let num_points = ends.last().map_or(0, |&e| e + 1);
        let instr_len = be16(d, o) as usize;
        o += 2 + instr_len;
        // Flags (with repeat).
        let mut flags = Vec::with_capacity(num_points);
        while flags.len() < num_points {
            let f = d[o];
            o += 1;
            flags.push(f);
            if f & 0x08 != 0 {
                let r = d[o];
                o += 1;
                for _ in 0..r {
                    flags.push(f);
                }
            }
        }
        flags.truncate(num_points);
        // X then Y coordinates (delta-encoded, byte or word per the flags).
        let mut xs = Vec::with_capacity(num_points);
        let mut x = 0i32;
        for &f in &flags {
            if f & 0x02 != 0 {
                let dx = d[o] as i32;
                o += 1;
                x += if f & 0x10 != 0 { dx } else { -dx };
            } else if f & 0x10 == 0 {
                x += bei16(d, o) as i32;
                o += 2;
            }
            xs.push(x as f64);
        }
        let mut ys = Vec::with_capacity(num_points);
        let mut y = 0i32;
        for &f in &flags {
            if f & 0x04 != 0 {
                let dy = d[o] as i32;
                o += 1;
                y += if f & 0x20 != 0 { dy } else { -dy };
            } else if f & 0x20 == 0 {
                y += bei16(d, o) as i32;
                o += 2;
            }
            ys.push(y as f64);
        }
        // Walk each contour, flattening quadratic Béziers. Consecutive
        // off-curve points imply an on-curve midpoint between them.
        let mut start = 0usize;
        for &e in &ends {
            if e < start || e >= num_points {
                break;
            }
            let pts: Vec<(f64, f64, bool)> = (start..=e)
                .map(|i| (xs[i], ys[i], flags[i] & 0x01 != 0))
                .collect();
            start = e + 1;
            if pts.len() < 2 {
                continue;
            }
            out.push(flatten_contour(&pts, steps, m));
        }
    }

    fn composite_glyph(&self, g: usize, steps: usize, m: [f64; 6], depth: usize, out: &mut Vec<Vec<[f64; 2]>>) {
        let d = &self.data;
        let mut o = g + 10;
        loop {
            if o + 4 > d.len() {
                return;
            }
            let flags = be16(d, o);
            let comp_gid = be16(d, o + 2);
            o += 4;
            // Arguments: xy offsets (we ignore point-matching, ARGS_ARE_XY unset).
            let (dx, dy) = if flags & 0x0001 != 0 {
                let a = bei16(d, o) as f64;
                let b = bei16(d, o + 2) as f64;
                o += 4;
                (a, b)
            } else {
                let a = (d[o] as i8) as f64;
                let b = (d[o + 1] as i8) as f64;
                o += 2;
                (a, b)
            };
            // Optional 2×2 transform.
            let (mut a, mut b, mut c, mut dd) = (1.0, 0.0, 0.0, 1.0);
            if flags & 0x0008 != 0 {
                a = f2dot14(d, o);
                dd = a;
                o += 2;
            } else if flags & 0x0040 != 0 {
                a = f2dot14(d, o);
                dd = f2dot14(d, o + 2);
                o += 4;
            } else if flags & 0x0080 != 0 {
                a = f2dot14(d, o);
                b = f2dot14(d, o + 2);
                c = f2dot14(d, o + 4);
                dd = f2dot14(d, o + 6);
                o += 8;
            }
            // Compose the component transform with the parent matrix `m`.
            let sub = [a, b, c, dd, dx, dy];
            let combined = mul_affine(m, sub);
            self.append_glyph(comp_gid, steps, combined, depth + 1, out);
            if flags & 0x0020 == 0 {
                break; // no MORE_COMPONENTS
            }
        }
    }
}

/// F2Dot14 fixed-point (composite-glyph scale).
fn f2dot14(d: &[u8], o: usize) -> f64 {
    bei16(d, o) as f64 / 16384.0
}

/// Compose affine `outer` ∘ `inner` (each [a,b,c,d,e,f] mapping
/// x' = a·x + c·y + e, y' = b·x + d·y + f).
fn mul_affine(outer: [f64; 6], inner: [f64; 6]) -> [f64; 6] {
    let [a1, b1, c1, d1, e1, f1] = outer;
    let [a2, b2, c2, d2, e2, f2] = inner;
    [
        a1 * a2 + c1 * b2,
        b1 * a2 + d1 * b2,
        a1 * c2 + c1 * d2,
        b1 * c2 + d1 * d2,
        a1 * e2 + c1 * f2 + e1,
        b1 * e2 + d1 * f2 + f1,
    ]
}

fn apply(m: [f64; 6], x: f64, y: f64) -> [f64; 2] {
    [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]]
}

/// Flatten one TrueType contour (on/off-curve points) into a polyline, with
/// each quadratic Bézier split into `steps` segments, transformed by `m`.
fn flatten_contour(pts: &[(f64, f64, bool)], steps: usize, m: [f64; 6]) -> Vec<[f64; 2]> {
    let n = pts.len();
    // Establish a starting on-curve point (synthesize one if the contour
    // begins off-curve, per the spec).
    let (mut cx, mut cy);
    let mut start_idx = 0;
    if pts[0].2 {
        cx = pts[0].0;
        cy = pts[0].1;
        start_idx = 1;
    } else if pts[n - 1].2 {
        cx = pts[n - 1].0;
        cy = pts[n - 1].1;
    } else {
        cx = (pts[0].0 + pts[n - 1].0) / 2.0;
        cy = (pts[0].1 + pts[n - 1].1) / 2.0;
    }
    let mut contour = vec![apply(m, cx, cy)];
    let steps = steps.max(1);
    let mut i = start_idx;
    let mut count = 0;
    while count < n {
        let (px, py, on) = pts[(i) % n];
        if on {
            contour.push(apply(m, px, py));
            cx = px;
            cy = py;
            i += 1;
            count += 1;
        } else {
            // Off-curve control: the next on-curve point ends the quad; if the
            // next is also off-curve, an on-curve midpoint is implied.
            let (nx, ny, non) = pts[(i + 1) % n];
            let (ex, ey) = if non {
                (nx, ny)
            } else {
                ((px + nx) / 2.0, (py + ny) / 2.0)
            };
            for s in 1..=steps {
                let t = s as f64 / steps as f64;
                let u = 1.0 - t;
                let qx = u * u * cx + 2.0 * u * t * px + t * t * ex;
                let qy = u * u * cy + 2.0 * u * t * py + t * t * ey;
                contour.push(apply(m, qx, qy));
            }
            cx = ex;
            cy = ey;
            i += 1;
            count += 1;
            if non {
                i += 1;
                count += 1;
            }
        }
    }
    contour
}

/// Parse a `cmap` table, choosing a Unicode subtable and reading format 4 or
/// 12 into a char→glyph map.
fn parse_cmap(d: &[u8], cmap: usize) -> Option<HashMap<u32, u16>> {
    let n = be16(d, cmap + 2) as usize;
    let mut best: Option<usize> = None;
    let mut best_rank = -1i32;
    for i in 0..n {
        let rec = cmap + 4 + i * 8;
        let plat = be16(d, rec);
        let enc = be16(d, rec + 2);
        let off = be32(d, rec + 4) as usize;
        // Prefer Windows Unicode BMP/full, then Unicode platform.
        let rank = match (plat, enc) {
            (3, 10) => 5,
            (3, 1) => 4,
            (0, _) => 3,
            (3, 0) => 1,
            _ => 0,
        };
        if rank > best_rank {
            best_rank = rank;
            best = Some(cmap + off);
        }
    }
    let sub = best?;
    match be16(d, sub) {
        4 => parse_cmap4(d, sub),
        12 => parse_cmap12(d, sub),
        _ => None,
    }
}

fn parse_cmap4(d: &[u8], sub: usize) -> Option<HashMap<u32, u16>> {
    let segx2 = be16(d, sub + 6) as usize;
    let segs = segx2 / 2;
    let end = sub + 14;
    let start = end + segx2 + 2;
    let delta = start + segx2;
    let range = delta + segx2;
    let mut map = HashMap::new();
    for s in 0..segs {
        let end_code = be16(d, end + s * 2);
        let start_code = be16(d, start + s * 2);
        let id_delta = be16(d, delta + s * 2);
        let id_range = be16(d, range + s * 2) as usize;
        if start_code > end_code {
            continue;
        }
        for c in start_code..=end_code {
            if c == 0xffff {
                break;
            }
            let gid = if id_range == 0 {
                (c as u32 + id_delta as u32) as u16
            } else {
                // idRangeOffset indexes into the glyphIdArray that follows.
                let gi = range + s * 2 + id_range + (c - start_code) as usize * 2;
                if gi + 1 >= d.len() {
                    continue;
                }
                let g = be16(d, gi);
                if g == 0 {
                    0
                } else {
                    (g as u32 + id_delta as u32) as u16
                }
            };
            if gid != 0 {
                map.insert(c as u32, gid);
            }
        }
    }
    Some(map)
}

fn parse_cmap12(d: &[u8], sub: usize) -> Option<HashMap<u32, u16>> {
    let ngroups = be32(d, sub + 12) as usize;
    let mut map = HashMap::new();
    for i in 0..ngroups {
        let g = sub + 16 + i * 12;
        if g + 12 > d.len() {
            break;
        }
        let start = be32(d, g);
        let end = be32(d, g + 4);
        let sgid = be32(d, g + 8);
        for c in start..=end.min(start + 65535) {
            map.insert(c, (sgid + (c - start)) as u16);
        }
    }
    Some(map)
}

/// The bundled default face, parsed once.
pub fn default_font() -> Option<&'static Font> {
    static FONT: OnceLock<Option<Font>> = OnceLock::new();
    FONT.get_or_init(|| Font::parse(DEFAULT_TTF.to_vec())).as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_font_parses_and_has_metrics() {
        let f = default_font().expect("embedded font must parse");
        assert!(f.units_per_em > 0.0);
        assert!(f.ascent > 0.0 && f.descent < 0.0, "asc {} desc {}", f.ascent, f.descent);
    }

    #[test]
    fn letters_map_to_glyphs_with_advance_and_outline() {
        let f = default_font().unwrap();
        // Space maps to a glyph with a positive advance but no contours.
        let sp = f.glyph_id(' ');
        assert!(f.advance(sp) > 0.0, "space advances");
        assert!(f.glyph_contours(sp, 6).is_empty(), "space has no ink");
        // 'A' has an advance and at least one contour.
        let a = f.glyph_id('A');
        assert!(a != 0, "'A' is in the cmap");
        assert!(f.advance(a) > 0.0);
        let ca = f.glyph_contours(a, 6);
        assert!(!ca.is_empty() && ca.iter().all(|c| c.len() >= 3), "'A' has real outlines");
        // 'O' has a counter → at least two contours (outer + hole).
        let o = f.glyph_contours(f.glyph_id('O'), 8);
        assert!(o.len() >= 2, "'O' has a counter (two contours), got {}", o.len());
    }

    #[test]
    fn unknown_char_is_notdef_glyph_zero() {
        let f = default_font().unwrap();
        // A private-use codepoint the font won't have.
        assert_eq!(f.glyph_id('\u{E000}'), 0);
    }
}
