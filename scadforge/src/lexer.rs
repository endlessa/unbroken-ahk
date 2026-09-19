//! Hand-rolled lexer for the SCAD-compatible language subset.
//!
//! Zero dependencies. Produces a flat token stream with byte offsets so
//! the parser can report positions. `$`-prefixed identifiers (special
//! variables like $fn) lex as ordinary identifiers.

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Num(f64),
    Ident(String),
    Str(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Colon,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Question,
    Dot,
    Bang,
    /// `#` — the highlight/debug modifier prefix (no expression meaning).
    Hash,
    Lt,
    Le,
    Gt,
    Ge,
    EqEq,
    NotEq,
    AndAnd,
    OrOr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    /// Byte offset in the source, for error messages.
    pub pos: usize,
}

/// Exactly `n` hex digits at `at`, as a number — or None if they are not
/// there or not all hex.
fn hex_at(b: &[u8], at: usize, n: usize) -> Option<u32> {
    b.get(at..at + n)
        .filter(|h| h.iter().all(u8::is_ascii_hexdigit))
        .and_then(|h| std::str::from_utf8(h).ok())
        .and_then(|h| u32::from_str_radix(h, 16).ok())
}

/// Append one code point's UTF-8 bytes.
fn push_utf8(buf: &mut Vec<u8>, c: char) {
    let mut tmp = [0u8; 4];
    buf.extend_from_slice(c.encode_utf8(&mut tmp).as_bytes());
}

pub fn lex(src: &str) -> Result<Vec<Token>, String> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        match c {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let start = i;
                i += 2;
                loop {
                    match b.get(i) {
                        Some(b'*') if b.get(i + 1) == Some(&b'/') => {
                            i += 2;
                            break;
                        }
                        Some(_) => i += 1,
                        None => return Err(format!("unterminated block comment at byte {}", start)),
                    }
                }
            }
            b'(' => push1(&mut out, Tok::LParen, &mut i),
            b')' => push1(&mut out, Tok::RParen, &mut i),
            b'{' => push1(&mut out, Tok::LBrace, &mut i),
            b'}' => push1(&mut out, Tok::RBrace, &mut i),
            b'[' => push1(&mut out, Tok::LBracket, &mut i),
            b']' => push1(&mut out, Tok::RBracket, &mut i),
            b',' => push1(&mut out, Tok::Comma, &mut i),
            b';' => push1(&mut out, Tok::Semi, &mut i),
            b':' => push1(&mut out, Tok::Colon, &mut i),
            b'=' if b.get(i + 1) == Some(&b'=') => push2(&mut out, Tok::EqEq, &mut i),
            b'=' => push1(&mut out, Tok::Assign, &mut i),
            b'!' if b.get(i + 1) == Some(&b'=') => push2(&mut out, Tok::NotEq, &mut i),
            b'!' => push1(&mut out, Tok::Bang, &mut i),
            b'<' if b.get(i + 1) == Some(&b'=') => push2(&mut out, Tok::Le, &mut i),
            b'<' => push1(&mut out, Tok::Lt, &mut i),
            b'>' if b.get(i + 1) == Some(&b'=') => push2(&mut out, Tok::Ge, &mut i),
            b'>' => push1(&mut out, Tok::Gt, &mut i),
            b'&' if b.get(i + 1) == Some(&b'&') => push2(&mut out, Tok::AndAnd, &mut i),
            b'|' if b.get(i + 1) == Some(&b'|') => push2(&mut out, Tok::OrOr, &mut i),
            b'?' => push1(&mut out, Tok::Question, &mut i),
            b'^' => push1(&mut out, Tok::Caret, &mut i),
            b'+' => push1(&mut out, Tok::Plus, &mut i),
            b'-' => push1(&mut out, Tok::Minus, &mut i),
            b'*' => push1(&mut out, Tok::Star, &mut i),
            b'/' => push1(&mut out, Tok::Slash, &mut i),
            b'%' => push1(&mut out, Tok::Percent, &mut i),
            b'#' => push1(&mut out, Tok::Hash, &mut i),
            // '.' followed by a digit starts a number (.5); otherwise it
            // is member access (v.x).
            b'.' if !matches!(b.get(i + 1), Some(d) if d.is_ascii_digit()) => {
                push1(&mut out, Tok::Dot, &mut i)
            }
            b'"' => {
                let start = i;
                i += 1;
                // BYTES, not chars. `\x##` is a BYTE escape per the reference
                // ("two hex digits, one byte"), which is what makes a UTF-8
                // sequence spelled out byte by byte — "\xc3\xa9" for "é" —
                // come back as ONE code point. Pushing char::from_u32(0xC3)
                // instead produced two, and the code's own comment said
                // "one byte" while doing otherwise.
                let mut buf: Vec<u8> = Vec::new();
                loop {
                    match b.get(i) {
                        Some(b'"') => {
                            i += 1;
                            break;
                        }
                        Some(b'\\') => match b.get(i + 1) {
                            Some(b'"') => {
                                buf.push(b'"');
                                i += 2;
                            }
                            Some(b'\\') => {
                                buf.push(b'\\');
                                i += 2;
                            }
                            Some(b'n') => {
                                buf.push(b'\n');
                                i += 2;
                            }
                            Some(b't') => {
                                buf.push(b'\t');
                                i += 2;
                            }
                            Some(b'r') => {
                                buf.push(b'\r');
                                i += 2;
                            }
                            // `\xNN` — exactly two hex digits → ONE byte.
                            Some(b'x') => match hex_at(b, i + 2, 2) {
                                Some(v) => {
                                    buf.push(v as u8);
                                    i += 4;
                                }
                                None => {
                                    buf.push(b'\\');
                                    i += 1;
                                }
                            },
                            // `\uXXXX` / `\UXXXXXX` — a code point, encoded.
                            Some(b'u') => match hex_at(b, i + 2, 4).and_then(char::from_u32) {
                                Some(c) => {
                                    push_utf8(&mut buf, c);
                                    i += 6;
                                }
                                None => {
                                    buf.push(b'\\');
                                    i += 1;
                                }
                            },
                            Some(b'U') => match hex_at(b, i + 2, 6).and_then(char::from_u32) {
                                Some(c) => {
                                    push_utf8(&mut buf, c);
                                    i += 8;
                                }
                                None => {
                                    buf.push(b'\\');
                                    i += 1;
                                }
                            },
                            // An unrecognized escape KEEPS ITS CHARACTERS: the
                            // backslash goes in here and the next turn of the
                            // loop copies the character after it. A lex error
                            // is fatal to the entire file, so one stray `\q` —
                            // or the backslashes in
                            // import("C:\Users\me\part.stl") — used to produce
                            // no geometry, no echoes and no output at all. The
                            // reference lists exactly ONE string-literal error,
                            // the unterminated string, and believes an unknown
                            // escape keeps the character.
                            Some(_) => {
                                buf.push(b'\\');
                                i += 1;
                            }
                            None => return Err(format!("unterminated string at byte {}", start)),
                        },
                        Some(&ch) => {
                            // Every other byte is copied through, so a
                            // multi-byte character survives intact without
                            // being decoded and re-encoded.
                            buf.push(ch);
                            i += 1;
                        }
                        None => return Err(format!("unterminated string at byte {}", start)),
                    }
                }
                // `\x` can spell a byte that is not valid UTF-8 on its own;
                // replace such a run rather than losing the whole string.
                out.push(Token {
                    tok: Tok::Str(String::from_utf8_lossy(&buf).into_owned()),
                    pos: start,
                });
            }
            b'0'..=b'9' | b'.' => {
                let start = i;
                while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                    i += 1;
                }
                // Exponent part: 1e5, 2.5e-3.
                if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
                    let mut j = i + 1;
                    if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                        j += 1;
                    }
                    if j < b.len() && b[j].is_ascii_digit() {
                        i = j;
                        while i < b.len() && b[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                }
                let text = &src[start..i];
                let n: f64 = text
                    .parse()
                    .map_err(|_| format!("bad number '{}' at byte {}", text, start))?;
                out.push(Token { tok: Tok::Num(n), pos: start });
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'$' => {
                let start = i;
                i += 1;
                while i < b.len()
                    && (b[i].is_ascii_alphanumeric() || b[i] == b'_')
                {
                    i += 1;
                }
                // "$ alone is not an identifier" — the reference states it
                // flatly. A bare `$` was lexed as the identifier "$", and
                // since every $-name resolves to a silent undef it was then
                // accepted both as an expression and as an assignment target
                // with no diagnostic anywhere.
                if b[start] == b'$' && i == start + 1 {
                    return Err(format!("'$' alone is not an identifier, at byte {}", start));
                }
                out.push(Token { tok: Tok::Ident(src[start..i].to_string()), pos: start });
            }
            other => {
                return Err(format!(
                    "unexpected character '{}' at byte {}",
                    other as char, i
                ))
            }
        }
    }
    Ok(out)
}

fn push1(out: &mut Vec<Token>, tok: Tok, i: &mut usize) {
    out.push(Token { tok, pos: *i });
    *i += 1;
}

fn push2(out: &mut Vec<Token>, tok: Tok, i: &mut usize) {
    out.push(Token { tok, pos: *i });
    *i += 2;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_a_representative_call() {
        let toks = lex("translate([1, 2.5, -3]) cube(1); // c").unwrap();
        let kinds: Vec<&Tok> = toks.iter().map(|t| &t.tok).collect();
        assert!(matches!(kinds[0], Tok::Ident(s) if s == "translate"));
        assert!(kinds.contains(&&Tok::Num(2.5)));
        // The trailing comment vanishes; '-' lexes as its own token.
        assert!(kinds.contains(&&Tok::Minus));
        assert_eq!(*kinds.last().unwrap(), &Tok::Semi);
    }

    #[test]
    fn dollar_idents_and_exponents() {
        let toks = lex("$fn = 1e2;").unwrap();
        assert!(matches!(&toks[0].tok, Tok::Ident(s) if s == "$fn"));
        assert!(matches!(&toks[2].tok, Tok::Num(n) if *n == 100.0));
    }

    #[test]
    fn string_escapes_cover_the_reference_set() {
        let toks = lex("\"a\\tb\\nc\\rd\\\\e\\\"f\";").unwrap();
        match &toks[0].tok {
            Tok::Str(s) => assert_eq!(s, "a\tb\nc\rd\\e\"f"),
            other => panic!("expected string, got {:?}", other),
        }
        // \uXXXX decodes a Unicode code point (Ω = U+03A9, é = U+00E9).
        match &lex("\"\\u03a9\\u00e9\";").unwrap()[0].tok {
            Tok::Str(s) => assert_eq!(s, "Ωé"),
            other => panic!("expected string, got {:?}", other),
        }
        // `\x##` is a BYTE escape, so a UTF-8 sequence spelled out byte by
        // byte comes back as one code point.
        match &lex("\"\\xc3\\xa9\";").unwrap()[0].tok {
            Tok::Str(s) => assert_eq!(s, "é"),
            other => panic!("expected string, got {:?}", other),
        }
        match &lex("\"\\x41\";").unwrap()[0].tok {
            Tok::Str(s) => assert_eq!(s, "A"),
            other => panic!("expected string, got {:?}", other),
        }
        // A malformed or unknown escape KEEPS ITS CHARACTERS. It used to be a
        // fatal lex error, and a lex error kills the whole file: one stray
        // `\q`, or the backslashes in a Windows path, produced no geometry, no
        // echoes and no output at all. The reference lists exactly one
        // string-literal error, the unterminated string.
        for (src, want) in [
            ("\"\\u03z9\";", "\\u03z9"),
            ("\"\\u03\";", "\\u03"),
            ("\"\\q\";", "\\q"),
            ("\"C:\\Users\\me\\part.stl\";", "C:\\Users\\me\\part.stl"),
            ("\"\\xzz\";", "\\xzz"),
        ] {
            match &lex(src).unwrap_or_else(|e| panic!("{src} must lex, got {e}"))[0].tok {
                Tok::Str(s) => assert_eq!(s, want, "{src}"),
                other => panic!("expected string, got {:?}", other),
            }
        }
    }

    #[test]
    fn a_bare_dollar_is_not_an_identifier() {
        // The reference states it flatly, with no VERIFY marker. A bare `$`
        // lexed as the identifier "$", and since every $-name resolves to a
        // silent undef it was then accepted both as an expression and as an
        // assignment target with no diagnostic anywhere.
        for src in ["echo($);", "$ = 7;", "x = $ + 1;"] {
            let e = lex(src).unwrap_err();
            assert!(e.contains("'$' alone"), "{src} gave: {e}");
        }
        // Real $-names are untouched.
        for src in ["echo($fn);", "$t = 1;", "f($_a);", "$fn2 = 3;"] {
            assert!(lex(src).is_ok(), "{src} must lex");
        }
    }

    #[test]
    fn block_comments_and_errors_carry_positions() {
        assert!(lex("/* a\nb */ cube(1);").is_ok());
        let err = lex("cube(1) @").unwrap_err();
        assert!(err.contains("'@'") && err.contains("byte 8"), "got: {}", err);
        assert!(lex("\"unterminated").unwrap_err().contains("unterminated string"));
        assert!(lex("/* open").unwrap_err().contains("unterminated block comment"));
    }
}
