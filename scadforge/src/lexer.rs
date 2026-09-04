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
                let mut s = String::new();
                loop {
                    match b.get(i) {
                        Some(b'"') => {
                            i += 1;
                            break;
                        }
                        Some(b'\\') => match b.get(i + 1) {
                            Some(b'"') => {
                                s.push('"');
                                i += 2;
                            }
                            Some(b'\\') => {
                                s.push('\\');
                                i += 2;
                            }
                            Some(b'n') => {
                                s.push('\n');
                                i += 2;
                            }
                            Some(b't') => {
                                s.push('\t');
                                i += 2;
                            }
                            _ => return Err(format!("bad escape in string at byte {}", i)),
                        },
                        Some(&ch) if ch < 0x80 => {
                            s.push(ch as char);
                            i += 1;
                        }
                        Some(_) => {
                            // Multi-byte UTF-8: decode the whole character so
                            // string values hold code points, not raw bytes.
                            let ch = src[i..].chars().next().unwrap();
                            s.push(ch);
                            i += ch.len_utf8();
                        }
                        None => return Err(format!("unterminated string at byte {}", start)),
                    }
                }
                out.push(Token { tok: Tok::Str(s), pos: start });
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
    fn block_comments_and_errors_carry_positions() {
        assert!(lex("/* a\nb */ cube(1);").is_ok());
        let err = lex("cube(1) @").unwrap_err();
        assert!(err.contains("'@'") && err.contains("byte 8"), "got: {}", err);
        assert!(lex("\"unterminated").unwrap_err().contains("unterminated string"));
        assert!(lex("/* open").unwrap_err().contains("unterminated block comment"));
    }
}
