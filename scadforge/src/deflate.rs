//! DEFLATE decompression (RFC 1951) — from scratch, zero dependencies.
//!
//! Enough to read the entries a real `.3mf` (ZIP) stores: block types 0
//! (stored), 1 (fixed Huffman), and 2 (dynamic Huffman), with LZ77 back-
//! references resolved against a growing output window. Bits are packed
//! LSB-first within each byte; Huffman codes are read MSB-first (one bit at a
//! time), which the canonical decoder below relies on.
//!
//! The decoder is a clean-room implementation of the standard canonical-
//! Huffman algorithm (counts + length-sorted symbols, as described in the RFC
//! and DEFLATE tutorials); it reads no zlib/gzip source.

const MAX_BITS: usize = 15;

struct BitReader<'a> {
    data: &'a [u8],
    byte: usize,
    bit: u32, // 0..=7, next bit position within data[byte]
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, byte: 0, bit: 0 }
    }
    /// Read one bit (LSB-first within the byte). Returns None past the end.
    fn bit(&mut self) -> Option<u32> {
        if self.byte >= self.data.len() {
            return None;
        }
        let b = (self.data[self.byte] >> self.bit) & 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.byte += 1;
        }
        Some(b as u32)
    }
    /// Read `n` bits as an integer, LSB-first (the extra-bits convention).
    fn bits(&mut self, n: u32) -> Option<u32> {
        let mut v = 0u32;
        for i in 0..n {
            v |= self.bit()? << i;
        }
        Some(v)
    }
    /// Discard the rest of the current byte (for a stored block).
    fn align(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
            self.byte += 1;
        }
    }
}

/// A canonical Huffman table: how many codes of each length, and the symbols
/// in canonical (length, then value) order.
struct Huffman {
    count: [u16; MAX_BITS + 1],
    symbol: Vec<u16>,
}

impl Huffman {
    /// Build from a per-symbol code-length list (0 = symbol unused).
    fn new(lengths: &[u16]) -> Huffman {
        let mut count = [0u16; MAX_BITS + 1];
        for &l in lengths {
            count[l as usize] += 1;
        }
        count[0] = 0; // length-0 symbols are absent
        // Starting index of each length within the symbol array.
        let mut offsets = [0u16; MAX_BITS + 2];
        for len in 1..=MAX_BITS {
            offsets[len + 1] = offsets[len] + count[len];
        }
        let mut symbol = vec![0u16; lengths.len()];
        for (sym, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbol[offsets[l as usize] as usize] = sym as u16;
                offsets[l as usize] += 1;
            }
        }
        Huffman { count, symbol }
    }

    /// Decode one symbol from the bit stream (walking codes bit by bit).
    fn decode(&self, r: &mut BitReader) -> Option<u16> {
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: i32 = 0;
        for len in 1..=MAX_BITS {
            code |= r.bit()? as i32;
            let cnt = self.count[len] as i32;
            if code - first < cnt {
                return Some(self.symbol[(index + (code - first)) as usize]);
            }
            index += cnt;
            first += cnt;
            first <<= 1;
            code <<= 1;
        }
        None
    }
}

// Length codes 257..=285: base length and extra bits.
const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
// Distance codes 0..=29: base distance and extra bits.
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// Inflate a raw DEFLATE stream. Returns the decompressed bytes, or None on any
/// malformed input (truncation, a bad symbol, a back-reference before the
/// output start). An optional `expected` size pre-reserves the output buffer.
pub fn inflate(data: &[u8], expected: Option<usize>) -> Option<Vec<u8>> {
    let mut r = BitReader::new(data);
    let mut out: Vec<u8> = Vec::with_capacity(expected.unwrap_or(0).min(64 << 20));
    // A hard cap so a crafted stream can't exhaust memory in a public preview.
    const MAX_OUT: usize = 128 << 20;
    loop {
        let bfinal = r.bit()?;
        let btype = r.bits(2)?;
        match btype {
            0 => {
                r.align();
                let len = r.bits(16)?;
                let nlen = r.bits(16)?;
                // A valid stored block has NLEN = one's-complement of LEN.
                if nlen != (!len & 0xFFFF) {
                    return None;
                }
                let len = len as usize;
                for _ in 0..len {
                    out.push(r.bits(8)? as u8);
                    if out.len() > MAX_OUT {
                        return None;
                    }
                }
            }
            1 => inflate_block(&mut r, &mut out, &fixed_lit(), &fixed_dist(), MAX_OUT)?,
            2 => {
                let (lit, dist) = dynamic_tables(&mut r)?;
                inflate_block(&mut r, &mut out, &lit, &dist, MAX_OUT)?;
            }
            _ => return None, // reserved block type
        }
        if bfinal == 1 {
            break;
        }
    }
    Some(out)
}

/// Decode one compressed block's symbols into `out`.
fn inflate_block(
    r: &mut BitReader,
    out: &mut Vec<u8>,
    lit: &Huffman,
    dist: &Huffman,
    max_out: usize,
) -> Option<()> {
    loop {
        let sym = lit.decode(r)?;
        if sym == 256 {
            return Some(()); // end of block
        }
        if sym < 256 {
            out.push(sym as u8);
            if out.len() > max_out {
                return None;
            }
            continue;
        }
        // A length/distance pair.
        let li = (sym - 257) as usize;
        if li >= LEN_BASE.len() {
            return None;
        }
        let length = LEN_BASE[li] as usize + r.bits(LEN_EXTRA[li])? as usize;
        let dsym = dist.decode(r)? as usize;
        if dsym >= DIST_BASE.len() {
            return None;
        }
        let distance = DIST_BASE[dsym] as usize + r.bits(DIST_EXTRA[dsym])? as usize;
        if distance == 0 || distance > out.len() {
            return None; // reference before the output start
        }
        let start = out.len() - distance;
        for k in 0..length {
            let b = out[start + k];
            out.push(b);
        }
        if out.len() > max_out {
            return None;
        }
    }
}

/// The fixed literal/length code lengths (RFC 1951 §3.2.6).
fn fixed_lit() -> Huffman {
    let mut lengths = [0u16; 288];
    for (i, l) in lengths.iter_mut().enumerate() {
        *l = match i {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    Huffman::new(&lengths)
}

fn fixed_dist() -> Huffman {
    Huffman::new(&[5u16; 30])
}

/// Read the dynamic Huffman tables that precede a type-2 block.
fn dynamic_tables(r: &mut BitReader) -> Option<(Huffman, Huffman)> {
    let hlit = r.bits(5)? as usize + 257;
    let hdist = r.bits(5)? as usize + 1;
    let hclen = r.bits(4)? as usize + 4;
    // Code-length code lengths, in the RFC's shuffled order.
    const ORDER: [usize; 19] =
        [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
    let mut cl_lengths = [0u16; 19];
    for i in 0..hclen {
        cl_lengths[ORDER[i]] = r.bits(3)? as u16;
    }
    let cl = Huffman::new(&cl_lengths);
    // Decode the lit+dist code lengths with run-length expansion.
    let total = hlit + hdist;
    let mut lengths = vec![0u16; total];
    let mut i = 0;
    while i < total {
        let sym = cl.decode(r)?;
        match sym {
            0..=15 => {
                lengths[i] = sym;
                i += 1;
            }
            16 => {
                // Repeat the previous length 3..=6 times.
                if i == 0 {
                    return None;
                }
                let n = 3 + r.bits(2)? as usize;
                let prev = lengths[i - 1];
                for _ in 0..n {
                    if i >= total {
                        return None;
                    }
                    lengths[i] = prev;
                    i += 1;
                }
            }
            17 => {
                let n = 3 + r.bits(3)? as usize;
                for _ in 0..n {
                    if i >= total {
                        return None;
                    }
                    lengths[i] = 0;
                    i += 1;
                }
            }
            18 => {
                let n = 11 + r.bits(7)? as usize;
                for _ in 0..n {
                    if i >= total {
                        return None;
                    }
                    lengths[i] = 0;
                    i += 1;
                }
            }
            _ => return None,
        }
    }
    let lit = Huffman::new(&lengths[..hlit]);
    let dist = Huffman::new(&lengths[hlit..]);
    Some((lit, dist))
}

#[cfg(test)]
mod tests {
    use super::*;

    // A raw DEFLATE stream for "hello, deflate! hello, deflate!\n", produced by
    // Python zlib.compressobj(9, DEFLATED, -15) — a fixed-Huffman block with a
    // back-reference for the repeated phrase. Pins the Huffman + LZ77 path.
    const HELLO: &[u8] = &[
        0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0xd7, 0x51, 0x48, 0x49, 0x4d, 0xcb, 0x49, 0x2c, 0x49, 0x55,
        0x54, 0xc8, 0x40, 0xe5, 0x73, 0x01, 0x00,
    ];

    #[test]
    fn inflates_a_backreferenced_stream() {
        let out = inflate(HELLO, None).expect("inflate ok");
        assert_eq!(String::from_utf8_lossy(&out), "hello, deflate! hello, deflate!\n");
    }

    // A raw DEFLATE *dynamic-Huffman* (BTYPE=2) stream: Python-compressed 40
    // repeated "<vertex .../>" fragments (1352 bytes → 219). Exercises the
    // dynamic code-length tables + run-length expansion path that real 3MF
    // meshes use.
    const DYNAMIC: &[u8] = &[
        0x75, 0xd3, 0x4b, 0x0a, 0xc2, 0x40, 0x10, 0x45, 0xd1, 0xad, 0x48, 0x16, 0xa0, 0xa9, 0xaa,
        0xfe, 0x42, 0x74, 0x37, 0x6e, 0x40, 0x44, 0xd4, 0xd5, 0x1b, 0xa1, 0x5f, 0x8b, 0x83, 0x3b,
        0x4a, 0xc3, 0xbb, 0xa3, 0x43, 0x6a, 0x7b, 0x5c, 0x6f, 0xf7, 0xeb, 0xf3, 0xf0, 0x3c, 0x2f,
        0xeb, 0x31, 0x2f, 0x87, 0xd7, 0xf7, 0xeb, 0xfb, 0xe3, 0xbd, 0x3f, 0x96, 0xd3, 0x65, 0xfb,
        0xed, 0x36, 0xf6, 0x80, 0xdd, 0xc7, 0x5e, 0x60, 0x8f, 0xb1, 0x77, 0xd8, 0xd3, 0xd8, 0xcd,
        0x21, 0xc8, 0x0a, 0x32, 0x04, 0x45, 0x41, 0x83, 0xa0, 0x8e, 0xc0, 0x0d, 0x82, 0xa6, 0x20,
        0x41, 0xd0, 0x15, 0x54, 0x52, 0x12, 0x63, 0xa0, 0xe3, 0x84, 0x24, 0x49, 0x13, 0x65, 0x90,
        0xa5, 0x09, 0x33, 0x48, 0xd3, 0xc4, 0x99, 0x88, 0xd3, 0xe4, 0x99, 0xc8, 0xd3, 0x04, 0x9a,
        0x08, 0xd4, 0x24, 0x9a, 0x49, 0xd4, 0x44, 0x9a, 0x89, 0xd4, 0x64, 0x9a, 0xc9, 0xd4, 0x65,
        0x5a, 0xc8, 0xd4, 0x65, 0x5a, 0xf0, 0xef, 0x9c, 0xbf, 0x27, 0x99, 0xba, 0x4c, 0x0b, 0x99,
        0xba, 0x4c, 0x2b, 0x99, 0xba, 0x4c, 0x2b, 0x99, 0xba, 0x4c, 0x2b, 0x99, 0xba, 0x4c, 0x1b,
        0x99, 0xba, 0x4c, 0x1b, 0x99, 0xba, 0x4c, 0x1b, 0x99, 0x86, 0x4c, 0x3b, 0x99, 0x86, 0x4c,
        0x3b, 0x99, 0x86, 0x4c, 0x3b, 0xde, 0xfc, 0x3c, 0x7a, 0x32, 0x8d, 0x79, 0xf6, 0x2b, 0xa1,
        0xc6, 0x3c, 0xfc, 0x95, 0x54, 0x63, 0x9e, 0xfe, 0x4a, 0xac, 0x21, 0x56, 0x33, 0x72, 0x8d,
        0x36, 0x13, 0x82, 0x8d, 0x3e, 0x93, 0x3f, 0xd9, 0x0f,
    ];

    #[test]
    fn inflates_a_dynamic_huffman_block() {
        let out = inflate(DYNAMIC, Some(1352)).expect("inflate dynamic ok");
        assert_eq!(out.len(), 1352);
        let s = String::from_utf8_lossy(&out);
        assert!(s.starts_with("<vertex x=\"0.5\" y=\"0.25\" z=\"0\"/>"));
        assert!(s.contains("<vertex x=\"39.5\" y=\"117.25\" z=\"0\"/>"));
    }

    #[test]
    fn stored_block_round_trips() {
        // A hand-built stored (uncompressed) block: BFINAL=1, BTYPE=00, then
        // byte-aligned LEN=5, NLEN=~5, "abcde".
        let data = [
            0x01, // BFINAL=1, BTYPE=00 (bits: 1 then 00)
            0x05, 0x00, // LEN = 5
            0xfa, 0xff, // NLEN = ~5
            b'a', b'b', b'c', b'd', b'e',
        ];
        let out = inflate(&data, Some(5)).unwrap();
        assert_eq!(&out, b"abcde");
    }

    #[test]
    fn truncated_input_is_none_not_panic() {
        assert!(inflate(&[0x0d, 0xc6], None).is_none());
        assert!(inflate(&[], None).is_none());
    }
}

// ===================================================================
//  COMPRESSION (RFC 1951, block type 1 — fixed Huffman)
//
//  Only as much as a PNG needs, which is one stream of fixed-Huffman
//  blocks with LZ77 back-references. Dynamic Huffman would compress
//  better and costs a code-length tree, two more Huffman tables and a
//  second pass; fixed codes are in the RFC as constants and need none of
//  that, and on rendered images the win from matching runs of identical
//  filtered bytes dwarfs the win from optimal symbol codes.
//
//  It is checked against `inflate` above rather than against a reference
//  library: the decoder was written first, from the RFC, and a stream the
//  decoder reproduces byte for byte is a correct stream whatever wrote it.
// ===================================================================

/// The length codes 257..=285: (code, extra bits, smallest length coded).
const LENGTH_CODES: [(u16, u32, u16); 29] = [
    (257, 0, 3),
    (258, 0, 4),
    (259, 0, 5),
    (260, 0, 6),
    (261, 0, 7),
    (262, 0, 8),
    (263, 0, 9),
    (264, 0, 10),
    (265, 1, 11),
    (266, 1, 13),
    (267, 1, 15),
    (268, 1, 17),
    (269, 2, 19),
    (270, 2, 23),
    (271, 2, 27),
    (272, 2, 31),
    (273, 3, 35),
    (274, 3, 43),
    (275, 3, 51),
    (276, 3, 59),
    (277, 4, 67),
    (278, 4, 83),
    (279, 4, 99),
    (280, 4, 115),
    (281, 5, 131),
    (282, 5, 163),
    (283, 5, 195),
    (284, 5, 227),
    (285, 0, 258),
];

/// The distance codes 0..=29: (extra bits, smallest distance coded).
const DIST_CODES: [(u32, u16); 30] = [
    (0, 1),
    (0, 2),
    (0, 3),
    (0, 4),
    (1, 5),
    (1, 7),
    (2, 9),
    (2, 13),
    (3, 17),
    (3, 25),
    (4, 33),
    (4, 49),
    (5, 65),
    (5, 97),
    (6, 129),
    (6, 193),
    (7, 257),
    (7, 385),
    (8, 513),
    (8, 769),
    (9, 1025),
    (9, 1537),
    (10, 2049),
    (10, 3073),
    (11, 4097),
    (11, 6145),
    (12, 8193),
    (12, 12289),
    (13, 16385),
    (13, 24577),
];

struct BitWriter {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl BitWriter {
    fn new() -> BitWriter {
        BitWriter { out: Vec::new(), acc: 0, n: 0 }
    }
    /// `bits` of `v`, LSB first — how the stream itself is packed.
    fn put(&mut self, v: u32, bits: u32) {
        self.acc |= v << self.n;
        self.n += bits;
        while self.n >= 8 {
            self.out.push((self.acc & 0xff) as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
    /// A Huffman code, MSB first — the one place the order reverses, and
    /// the reason a decoder reading bit by bit finds the tree.
    fn put_code(&mut self, code: u32, bits: u32) {
        for i in (0..bits).rev() {
            self.put((code >> i) & 1, 1);
        }
    }
    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push((self.acc & 0xff) as u8);
        }
        self.out
    }
}

/// Emit one literal/length symbol in the fixed code.
fn put_litlen(w: &mut BitWriter, sym: u16) {
    match sym {
        0..=143 => w.put_code(0x30 + sym as u32, 8),
        144..=255 => w.put_code(0x190 + (sym as u32 - 144), 9),
        256..=279 => w.put_code(sym as u32 - 256, 7),
        _ => w.put_code(0xc0 + (sym as u32 - 280), 8),
    }
}

const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
const WINDOW: usize = 32768;
const HASH_BITS: usize = 15;
const CHAIN: usize = 32;

/// Compress with DEFLATE, fixed Huffman, LZ77 over a 32 KiB window.
pub fn deflate(data: &[u8]) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.put(1, 1); // final block
    w.put(1, 2); // type 1, fixed Huffman

    // A hash of three bytes to the most recent position that started with
    // them, and a chain of earlier positions from each. Greedy, shortest
    // useful chain: this is a compressor for pictures of machine parts, not
    // an archiver.
    let mut head = vec![u32::MAX; 1 << HASH_BITS];
    let mut prev = vec![u32::MAX; data.len().max(1)];
    let hash = |d: &[u8], i: usize| -> usize {
        ((d[i] as usize) << 10 ^ (d[i + 1] as usize) << 5 ^ d[i + 2] as usize) & ((1 << HASH_BITS) - 1)
    };

    let mut i = 0usize;
    while i < data.len() {
        let mut best_len = 0usize;
        let mut best_dist = 0usize;
        if i + MIN_MATCH <= data.len() {
            let h = hash(data, i);
            let mut cand = head[h];
            let limit = i.saturating_sub(WINDOW);
            let mut tries = CHAIN;
            while cand != u32::MAX && (cand as usize) >= limit && tries > 0 {
                let c = cand as usize;
                let max = MAX_MATCH.min(data.len() - i);
                let mut l = 0;
                while l < max && data[c + l] == data[i + l] {
                    l += 1;
                }
                if l > best_len {
                    best_len = l;
                    best_dist = i - c;
                    if l == max {
                        break;
                    }
                }
                cand = prev[c];
                tries -= 1;
            }
        }
        // A three-byte match costs about 24 bits and saves three literals,
        // so a distant one is not worth its distance code; take the literal.
        let take = best_len >= MIN_MATCH && !(best_len == 3 && best_dist > 4096);
        let step = if take { best_len } else { 1 };

        // Register exactly the positions this step CONSUMES, and only after
        // the decision is made. Registering the match's whole span and then
        // rejecting the match left positions ahead of `i` in the chains, so
        // the next round found itself there and computed a distance of zero,
        // which is not a distance DEFLATE can encode. It showed on
        // incompressible input, where rejected three-byte matches are common
        // and matches worth taking are not.
        for k in 0..step {
            if i + k + MIN_MATCH <= data.len() {
                let hh = hash(data, i + k);
                prev[i + k] = head[hh];
                head[hh] = (i + k) as u32;
            }
        }

        if take {
            let (code, extra, base) =
                *LENGTH_CODES.iter().rev().find(|(_, _, b)| *b as usize <= best_len).unwrap();
            put_litlen(&mut w, code);
            if extra > 0 {
                w.put((best_len as u32) - base as u32, extra);
            }
            let dcode =
                DIST_CODES.iter().rposition(|(_, b)| *b as usize <= best_dist).unwrap();
            let (dextra, dbase) = DIST_CODES[dcode];
            w.put_code(dcode as u32, 5);
            if dextra > 0 {
                w.put((best_dist as u32) - dbase as u32, dextra);
            }
        } else {
            put_litlen(&mut w, data[i] as u16);
        }
        i += step;
    }
    put_litlen(&mut w, 256); // end of block
    w.finish()
}

/// Adler-32, the checksum a zlib stream carries.
pub fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// Wrap a DEFLATE stream in the zlib envelope PNG asks for.
pub fn zlib(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01]; // deflate, 32K window, no preset dictionary
    out.extend(deflate(data));
    out.extend(adler32(data).to_be_bytes());
    out
}

#[cfg(test)]
mod compress_tests {
    use super::*;

    /// The compressor is checked against this file's own decompressor, and
    /// that is a real check rather than a circular one: the decoder was
    /// written first, from RFC 1951, to read `.3mf` files produced by other
    /// programs. A stream it reproduces byte for byte is a conforming stream
    /// whatever wrote it.
    fn roundtrip(data: &[u8]) {
        let packed = deflate(data);
        let back = inflate(&packed, Some(data.len()))
            .unwrap_or_else(|| panic!("inflate refused {} bytes of output", data.len()));
        assert_eq!(back.len(), data.len(), "length");
        assert!(back == data, "content differs");
    }

    #[test]
    fn deflate_streams_read_back_through_our_own_inflate() {
        roundtrip(b"");
        roundtrip(b"a");
        roundtrip(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        roundtrip(b"the quick brown fox jumps over the lazy dog");
        // Every byte value, so the 8-bit and 9-bit halves of the fixed
        // literal code are both exercised.
        let all: Vec<u8> = (0..=255u8).collect();
        roundtrip(&all);
        // A long repeat, which is what a rendered image's background is and
        // the only reason this compresses at all.
        roundtrip(&vec![0u8; 100_000]);
        // Matches at every distance class, built by repeating a block of
        // growing size so back-references of many lengths occur.
        let mut grow = Vec::new();
        for n in 1..300usize {
            let chunk: Vec<u8> = (0..n).map(|i| (i * 31 % 251) as u8).collect();
            grow.extend(&chunk);
            grow.extend(&chunk);
        }
        roundtrip(&grow);
        // Something incompressible: a stream where matching never pays.
        let mut x = 0u32;
        let noise: Vec<u8> = (0..50_000)
            .map(|_| {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (x >> 24) as u8
            })
            .collect();
        roundtrip(&noise);
    }

    #[test]
    fn a_run_of_one_byte_compresses_to_almost_nothing() {
        // The property the PNG writer depends on. Without it an image of a
        // machine part on a flat background would be written at full size.
        let flat = vec![7u8; 200_000];
        let packed = deflate(&flat);
        assert!(packed.len() < flat.len() / 100, "200 KB of one byte became {}", packed.len());
    }

    #[test]
    fn the_zlib_envelope_carries_a_correct_adler32() {
        // Adler-32 of "abc" is 0x024d0127, which the RFC gives as its worked
        // example.
        assert_eq!(adler32(b"abc"), 0x024d_0127);
        let z = zlib(b"hello hello hello hello");
        assert_eq!(&z[..2], &[0x78, 0x01]);
        let n = z.len();
        let carried = u32::from_be_bytes([z[n - 4], z[n - 3], z[n - 2], z[n - 1]]);
        assert_eq!(carried, adler32(b"hello hello hello hello"));
        assert_eq!(inflate(&z[2..n - 4], None).unwrap(), b"hello hello hello hello");
    }
}
