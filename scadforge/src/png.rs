//! PNG encoding (RFC 2083) — from scratch, zero dependencies.
//!
//! Only what a rendered image needs: 8-bit truecolour, no interlacing, no
//! palette, no ancillary chunks. The bytes that do the work are the row
//! FILTERS, which turn a smoothly shaded picture into mostly small numbers
//! before `deflate` ever sees it; the compressor is this project's own, in
//! `deflate.rs`, and the CRC is the one `zip.rs` already computes for ZIP
//! entries. Nothing here is borrowed from libpng or zlib.

use crate::deflate::zlib;
use crate::zip::crc32;

fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], body: &[u8]) {
    out.extend((body.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend(tag);
    out.extend(body);
    let sum = crc32(&out[start..]);
    out.extend(sum.to_be_bytes());
}

/// The five filters PNG defines, applied to one row against the row above.
///
/// Each predicts a byte from its neighbours and stores the difference, so a
/// gradient becomes a run of near-zeros. Which one wins varies row by row,
/// so all five are tried and the one with the smallest total excursion from
/// zero is kept. That heuristic is the one the specification suggests, and
/// it is cheap: five passes over a row costs nothing beside the compression
/// that follows.
fn filter_row(cur: &[u8], up: &[u8], bpp: usize, out: &mut Vec<u8>) {
    let n = cur.len();
    let left = |b: &[u8], i: usize| if i >= bpp { b[i - bpp] as i32 } else { 0 };
    let upleft = |b: &[u8], i: usize| if i >= bpp { b[i - bpp] as i32 } else { 0 };

    let mut best: Option<(u64, u8, Vec<u8>)> = None;
    for f in 0u8..5 {
        let mut row = Vec::with_capacity(n);
        for i in 0..n {
            let a = left(cur, i);
            let b = up[i] as i32;
            let c = upleft(up, i);
            let pred = match f {
                0 => 0,
                1 => a,
                2 => b,
                3 => (a + b) / 2,
                _ => {
                    // Paeth: pick whichever of left, above, above-left the
                    // linear estimate a + b - c comes closest to.
                    let p = a + b - c;
                    let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
                    if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c
                    }
                }
            };
            row.push(((cur[i] as i32 - pred) & 0xff) as u8);
        }
        // Sum of distances from zero, counting 255 as -1: the specification's
        // own measure of how well a filter flattened the row.
        let cost: u64 = row.iter().map(|&v| if v < 128 { v as u64 } else { 256 - v as u64 }).sum();
        if best.as_ref().is_none_or(|(c, _, _)| cost < *c) {
            best = Some((cost, f, row));
        }
    }
    let (_, f, row) = best.unwrap();
    out.push(f);
    out.extend(row);
}

/// Encode 8-bit RGB pixels, row-major, `w * h * 3` bytes, as a PNG.
pub fn encode_rgb(w: usize, h: usize, rgb: &[u8]) -> Vec<u8> {
    assert_eq!(rgb.len(), w * h * 3, "pixel buffer is not w*h*3");
    let mut raw = Vec::with_capacity(h * (w * 3 + 1));
    let zero = vec![0u8; w * 3];
    for y in 0..h {
        let cur = &rgb[y * w * 3..(y + 1) * w * 3];
        let up = if y == 0 { &zero[..] } else { &rgb[(y - 1) * w * 3..y * w * 3] };
        filter_row(cur, up, 3, &mut raw);
    }
    let mut out = Vec::new();
    out.extend([0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend((w as u32).to_be_bytes());
    ihdr.extend((h as u32).to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]); // 8 bits, truecolour, deflate, adaptive, no interlace
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deflate::inflate;

    /// Decode our own PNG far enough to get the pixels back: chunk walk,
    /// inflate, then undo the filters. If they survive that they are a
    /// conforming image, because every step is the specification read in
    /// reverse.
    fn decode(png: &[u8]) -> (usize, usize, Vec<u8>) {
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        let (mut i, mut w, mut h, mut idat) = (8usize, 0usize, 0usize, Vec::new());
        while i + 8 <= png.len() {
            let len = u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]) as usize;
            let tag = &png[i + 4..i + 8];
            let body = &png[i + 8..i + 8 + len];
            // Every chunk carries a CRC over its tag and body.
            let want = u32::from_be_bytes([
                png[i + 8 + len],
                png[i + 9 + len],
                png[i + 10 + len],
                png[i + 11 + len],
            ]);
            assert_eq!(crc32(&png[i + 4..i + 8 + len]), want, "crc on {:?}", tag);
            if tag == b"IHDR" {
                w = u32::from_be_bytes([body[0], body[1], body[2], body[3]]) as usize;
                h = u32::from_be_bytes([body[4], body[5], body[6], body[7]]) as usize;
                assert_eq!(&body[8..], &[8, 2, 0, 0, 0]);
            } else if tag == b"IDAT" {
                idat.extend(body);
            }
            i += 12 + len;
        }
        let raw = inflate(&idat[2..idat.len() - 4], None).expect("IDAT inflates");
        let stride = w * 3;
        let mut px = vec![0u8; w * h * 3];
        for y in 0..h {
            let f = raw[y * (stride + 1)];
            for x in 0..stride {
                let v = raw[y * (stride + 1) + 1 + x] as i32;
                let a = if x >= 3 { px[y * stride + x - 3] as i32 } else { 0 };
                let b = if y > 0 { px[(y - 1) * stride + x] as i32 } else { 0 };
                let c = if x >= 3 && y > 0 { px[(y - 1) * stride + x - 3] as i32 } else { 0 };
                let pred = match f {
                    0 => 0,
                    1 => a,
                    2 => b,
                    3 => (a + b) / 2,
                    _ => {
                        let p = a + b - c;
                        let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
                        if pa <= pb && pa <= pc {
                            a
                        } else if pb <= pc {
                            b
                        } else {
                            c
                        }
                    }
                };
                px[y * stride + x] = ((v + pred) & 0xff) as u8;
            }
        }
        (w, h, px)
    }

    #[test]
    fn a_png_decodes_back_to_the_pixels_it_was_given() {
        // A picture with a flat region, a gradient and sharp edges, so every
        // filter gets chosen somewhere.
        let (w, h) = (61usize, 37usize);
        let mut px = Vec::with_capacity(w * h * 3);
        for y in 0..h {
            for x in 0..w {
                let edge = (x / 7 + y / 5) % 2 == 0;
                px.push(if edge { (x * 255 / w) as u8 } else { 17 });
                px.push((y * 255 / h) as u8);
                px.push(if edge { 200 } else { 17 });
            }
        }
        let png = encode_rgb(w, h, &px);
        let (dw, dh, back) = decode(&png);
        assert_eq!((dw, dh), (w, h));
        assert!(back == px, "pixels differ");
    }

    #[test]
    fn a_flat_image_is_small() {
        // The property that makes this usable: a rendered part on an empty
        // background must not cost a byte a pixel.
        //
        // It does not go to nothing, and the floor is DEFLATE's own: a match
        // carries at most 258 bytes, so 360,000 identical bytes need about
        // 1,400 of them whatever the compressor does, at roughly 25 bits
        // each. Three kilobytes is near that floor; the test guards the
        // order of magnitude, which is what actually matters.
        let (w, h) = (400usize, 300usize);
        let png = encode_rgb(w, h, &vec![14u8; w * h * 3]);
        let raw = w * h * 3;
        assert!(png.len() * 100 < raw, "flat 400x300 came to {} bytes of {raw}", png.len());
        let (_, _, back) = decode(&png);
        assert!(back.iter().all(|&v| v == 14));
    }

    #[test]
    fn one_pixel_and_one_row_are_still_valid() {
        let png = encode_rgb(1, 1, &[9, 8, 7]);
        assert_eq!(decode(&png), (1, 1, vec![9, 8, 7]));
        let row: Vec<u8> = (0..30u8).collect();
        let png = encode_rgb(10, 1, &row);
        assert_eq!(decode(&png).2, row);
    }
}
