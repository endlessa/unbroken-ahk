//! Minimal ZIP container — from scratch, zero dependencies. Enough to write and
//! read the OPC (ZIP) archives a `.3mf` is: a handful of named entries.
//!
//! The writer stores entries uncompressed (method 0) — always valid ZIP, and a
//! 3MF reader accepts it. The reader handles both stored (method 0) and
//! DEFLATE (method 8, via `crate::deflate::inflate`) entries, since real-world
//! `.3mf` files deflate their parts. Everything is bounds-checked: malformed or
//! truncated input yields the entries that parse, never a panic.

use crate::deflate;

/// CRC-32 (IEEE 802.3 polynomial 0xEDB88320), the checksum ZIP entries carry.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn u16le(v: u16, out: &mut Vec<u8>) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn u32le(v: u32, out: &mut Vec<u8>) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Write named entries as one STORED (uncompressed) ZIP archive.
pub fn write_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    // (local-header offset, crc, size, name) per entry, for the central dir.
    let mut records: Vec<(u32, u32, u32, &str)> = Vec::new();
    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let size = data.len() as u32;
        // Local file header.
        u32le(0x0403_4b50, &mut out);
        u16le(20, &mut out); // version needed
        u16le(0, &mut out); // flags
        u16le(0, &mut out); // method 0 = stored
        u16le(0, &mut out); // mod time
        u16le(0x21, &mut out); // mod date (1980-01-01)
        u32le(crc, &mut out);
        u32le(size, &mut out); // compressed size (== uncompressed for stored)
        u32le(size, &mut out); // uncompressed size
        u16le(name.len() as u16, &mut out);
        u16le(0, &mut out); // extra length
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        records.push((offset, crc, size, name));
    }
    // Central directory.
    let cd_start = out.len() as u32;
    for (offset, crc, size, name) in &records {
        u32le(0x0201_4b50, &mut out);
        u16le(20, &mut out); // version made by
        u16le(20, &mut out); // version needed
        u16le(0, &mut out); // flags
        u16le(0, &mut out); // method
        u16le(0, &mut out); // mod time
        u16le(0x21, &mut out); // mod date
        u32le(*crc, &mut out);
        u32le(*size, &mut out);
        u32le(*size, &mut out);
        u16le(name.len() as u16, &mut out);
        u16le(0, &mut out); // extra
        u16le(0, &mut out); // comment
        u16le(0, &mut out); // disk number start
        u16le(0, &mut out); // internal attrs
        u32le(0, &mut out); // external attrs
        u32le(*offset, &mut out);
        out.extend_from_slice(name.as_bytes());
    }
    let cd_size = out.len() as u32 - cd_start;
    // End of central directory record.
    u32le(0x0605_4b50, &mut out);
    u16le(0, &mut out); // this disk
    u16le(0, &mut out); // cd start disk
    u16le(records.len() as u16, &mut out);
    u16le(records.len() as u16, &mut out);
    u32le(cd_size, &mut out);
    u32le(cd_start, &mut out);
    u16le(0, &mut out); // comment length
    out
}

fn rd_u16(d: &[u8], off: usize) -> Option<u16> {
    d.get(off..off.checked_add(2)?).map(|s| u16::from_le_bytes([s[0], s[1]]))
}
fn rd_u32(d: &[u8], off: usize) -> Option<u32> {
    d.get(off..off.checked_add(4)?).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Read a ZIP archive into `(name, bytes)` pairs, decompressing stored and
/// DEFLATE entries. Entries that fail to parse or decompress are skipped.
pub fn read_zip(d: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    // Find the End-Of-Central-Directory record by scanning back for its sig.
    let eocd = match find_eocd(d) {
        Some(p) => p,
        None => return out,
    };
    let count = match rd_u16(d, eocd + 10) {
        Some(c) => c as usize,
        None => return out,
    };
    let mut cd = match rd_u32(d, eocd + 16) {
        Some(o) => o as usize,
        None => return out,
    };
    // A global cap on the SUM of decompressed bytes across all entries. The
    // per-stream inflate cap does not bound the total, and a crafted archive
    // can point thousands of central-directory records at one small,
    // highly-compressible stream — so budget the whole archive here.
    let mut budget: usize = 256 << 20;
    for _ in 0..count {
        // Central directory header.
        if rd_u32(d, cd) != Some(0x0201_4b50) {
            break;
        }
        let method = match rd_u16(d, cd + 10) {
            Some(m) => m,
            None => break,
        };
        let crc = rd_u32(d, cd + 16).unwrap_or(0);
        let comp_size = match rd_u32(d, cd + 20) {
            Some(s) => s as usize,
            None => break,
        };
        let name_len = rd_u16(d, cd + 28).unwrap_or(0) as usize;
        let extra_len = rd_u16(d, cd + 30).unwrap_or(0) as usize;
        let comment_len = rd_u16(d, cd + 32).unwrap_or(0) as usize;
        let local_off = match rd_u32(d, cd + 42) {
            Some(o) => o as usize,
            None => break,
        };
        let name = cd
            .checked_add(46)
            .and_then(|s| d.get(s..s.checked_add(name_len)?))
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        // Local header at local_off: recompute the data start from ITS own
        // name/extra lengths (they can differ from the central dir's).
        if rd_u32(d, local_off) == Some(0x0403_4b50) {
            let lname = rd_u16(d, local_off + 26).unwrap_or(0) as usize;
            let lextra = rd_u16(d, local_off + 28).unwrap_or(0) as usize;
            // All-checked arithmetic so a hostile offset can't wrap `usize`
            // (a concern only on 32-bit targets, but free to be correct).
            let data_start = local_off
                .checked_add(30)
                .and_then(|s| s.checked_add(lname))
                .and_then(|s| s.checked_add(lextra));
            let raw = data_start
                .and_then(|s| s.checked_add(comp_size).map(|e| (s, e)))
                .and_then(|(s, e)| d.get(s..e));
            if let Some(raw) = raw {
                let bytes = match method {
                    0 => Some(raw.to_vec()),
                    8 => deflate::inflate(raw, None),
                    _ => None,
                };
                // Verify the CRC-32 (from the central directory) — this both
                // catches corruption and rejects an entry whose local-header
                // data range disagrees with its central record.
                if let Some(b) = bytes {
                    if b.len() <= budget && crc32(&b) == crc {
                        budget -= b.len();
                        out.push((name, b));
                    }
                }
            }
        }
        cd += 46 + name_len + extra_len + comment_len;
    }
    out
}

/// Scan backwards for the EOCD signature (0x06054b50). The trailing comment is
/// almost always empty, so this is a short scan near the end.
fn find_eocd(d: &[u8]) -> Option<usize> {
    if d.len() < 22 {
        return None;
    }
    let start = d.len().saturating_sub(22 + 0xFFFF);
    for i in (start..=d.len() - 22).rev() {
        if rd_u32(d, i) == Some(0x0605_4b50) {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_known_vector() {
        // The canonical CRC-32 of "123456789" is 0xCBF43926.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn stored_zip_round_trips_multiple_entries() {
        let entries: [(&str, &[u8]); 3] = [
            ("[Content_Types].xml", b"<Types/>"),
            ("_rels/.rels", b"<Relationships/>"),
            ("3D/3dmodel.model", b"<model>hello</model>"),
        ];
        let bytes = write_zip(&entries);
        let back = read_zip(&bytes);
        assert_eq!(back.len(), 3);
        assert_eq!(back[0].0, "[Content_Types].xml");
        assert_eq!(back[2].0, "3D/3dmodel.model");
        assert_eq!(back[2].1, b"<model>hello</model>");
    }

    #[test]
    fn reads_a_deflated_entry_from_python_zip() {
        // A real ZIP produced by Python's zipfile with ZIP_DEFLATED for one
        // entry "m" = "AAAA...(64)". Exercises the method-8 (inflate) path.
        // Generated: zf.writestr(ZipInfo('m'), b'A'*64, ZIP_DEFLATED)
        let z = python_deflated_zip();
        let back = read_zip(&z);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].0, "m");
        assert_eq!(back[0].1, vec![b'A'; 64]);
    }

    #[test]
    fn corrupted_entry_data_fails_crc_and_is_skipped() {
        // A valid two-entry archive; flip a byte of the first entry's stored
        // data. Its CRC no longer matches, so read_zip drops it (and any
        // crafted local/central size disagreement is caught the same way).
        let mut z = write_zip(&[("a", b"hello world"), ("b", b"kept")]);
        // The first entry's data begins right after its 30-byte local header
        // plus the 1-byte name "a".
        let data_pos = 30 + 1;
        z[data_pos] ^= 0xFF;
        let back = read_zip(&z);
        assert_eq!(back.len(), 1, "corrupted entry dropped, valid one kept");
        assert_eq!(back[0].0, "b");
        assert_eq!(back[0].1, b"kept");
    }

    #[test]
    fn truncated_or_garbage_zip_is_empty_not_panic() {
        assert!(read_zip(b"not a zip").is_empty());
        assert!(read_zip(&[]).is_empty());
        // A valid archive with its tail chopped off.
        let mut z = write_zip(&[("a", b"data")]);
        z.truncate(z.len() - 10);
        let _ = read_zip(&z); // must not panic
    }

    // A ZIP with one DEFLATE-compressed entry "m" = "A"×64, produced by
    // Python's zipfile (ZIP_DEFLATED).
    fn python_deflated_zip() -> Vec<u8> {
        vec![
            0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00,
            0x3c, 0x62, 0x4c, 0x41, 0x06, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x6d, 0x73, 0x74, 0xa4, 0x0c, 0x00, 0x00, 0x50, 0x4b, 0x01, 0x02, 0x14,
            0x03, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x21, 0x00, 0x3c, 0x62, 0x4c,
            0x41, 0x06, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x6d,
            0x50, 0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x2f, 0x00,
            0x00, 0x00, 0x25, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]
    }
}
