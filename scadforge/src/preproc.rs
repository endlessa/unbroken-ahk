//! `include` / `use` directive resolution — a source-level pass run before
//! evaluation.
//!
//! `include <path>` textually pastes another file's content (recursively);
//! its geometry runs and its definitions and top-level assignments join the
//! including scope, so a later same-name assignment in the main file wins
//! (last-write-wins covers the common override-after-include idiom; the
//! 2019.05 "main wins even before the include" rule is a documented partial).
//! `use <path>` parses another file and exposes only its top-level
//! module/function DEFINITIONS (geometry and top-level variables are not run);
//! a same-name definition in the using file shadows it.
//!
//! Paths resolve relative to the directory of the file containing the
//! directive and are sandboxed: absolute paths and `..` traversal are refused
//! (the server is localhost but hosted). Self/mutual includes are cycle-
//! guarded. Missing files warn and are skipped, never fatal.

use crate::ast::Stmt;
use crate::parser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct Resolved {
    pub program: Vec<Stmt>,
    pub warnings: Vec<String>,
    /// A parse error in the main (post-include) source; included/used files
    /// that fail to parse warn instead of aborting the whole design.
    pub error: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Include,
    Use,
}

/// DoS budget for include expansion (this pass runs on the default stack,
/// BEFORE the evaluator's big-stack thread, so an adversarial include DAG
/// must not blow the stack, fan out combinatorially, or amplify the body).
const MAX_INCLUDE_DEPTH: usize = 40;
const MAX_INCLUDE_EXPANSIONS: usize = 2_000;
const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

struct Budget {
    expansions: usize,
    capped: bool,
}

/// Resolve every include/use in `source` (whose directory is `base`) and parse
/// the result into one program, with the definitions of every `use`d file
/// prepended (so local definitions shadow them via last-write-wins).
pub fn resolve(source: &str, base: &Path) -> Resolved {
    let mut warnings = Vec::new();
    let mut uses: Vec<(PathBuf, PathBuf)> = Vec::new(); // (resolved file, its dir)
    let mut seen = HashSet::new();
    let mut budget = Budget { expansions: 0, capped: false };
    // The containment root: no include/use may resolve outside this subtree
    // (canonicalized, so symlinks can't escape it either).
    let root = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
    let inlined =
        inline_includes(source, base, &root, &mut seen, &mut warnings, &mut uses, &mut budget, 0);

    let main = match parser::parse(&inlined) {
        Ok(p) => p,
        Err(e) => {
            return Resolved { program: Vec::new(), warnings, error: Some(e) };
        }
    };

    // Collect the definitions exported by every use'd file (its own includes
    // resolved first). Geometry and top-level variables are dropped.
    let mut used_defs: Vec<Stmt> = Vec::new();
    let mut used_seen: HashSet<PathBuf> = HashSet::new();
    for (path, dir) in uses {
        if !used_seen.insert(path.clone()) {
            continue; // using the same file twice is idempotent
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                warnings.push(format!("WARNING: Can't open library '{}'.", display(&path)));
                continue;
            }
        };
        let mut inner_seen = HashSet::new();
        let mut inner_uses = Vec::new();
        let inner = inline_includes(
            &text, &dir, &root, &mut inner_seen, &mut warnings, &mut inner_uses, &mut budget, 0,
        );
        match parser::parse(&inner) {
            Ok(stmts) => {
                for s in stmts {
                    if matches!(s, Stmt::ModuleDef { .. } | Stmt::FunctionDef { .. }) {
                        used_defs.push(s);
                    }
                }
            }
            Err(e) => {
                warnings.push(format!("WARNING: parse error in used file '{}': {}", display(&path), e));
            }
        }
    }

    let mut program = used_defs;
    program.extend(main);
    Resolved { program, warnings, error: None }
}

/// Return `source` with each `include` directive replaced by the (recursively
/// resolved) content of the referenced file, `use` directives removed and
/// recorded in `uses`, and everything else passed through unchanged.
///
/// The scan is CONTEXT-AWARE: it tracks string-literal and comment state so a
/// line that merely LOOKS like a directive inside a `"..."` string or a
/// `/*...*/` / `//` comment is inert (the earlier line-level scan let a hidden
/// `include <secret>` read and exfiltrate in-tree files). Only the directive
/// tokens are consumed, so trailing code on the same line survives. Expansion
/// is bounded (depth / count / output size) so an adversarial include DAG
/// cannot blow the stack, fan out combinatorially, or amplify the body.
#[allow(clippy::too_many_arguments)]
fn inline_includes(
    source: &str,
    base: &Path,
    root: &Path,
    seen: &mut HashSet<PathBuf>,
    warnings: &mut Vec<String>,
    uses: &mut Vec<(PathBuf, PathBuf)>,
    budget: &mut Budget,
    depth: usize,
) -> String {
    if depth > MAX_INCLUDE_DEPTH {
        warn_once(budget, warnings, "WARNING: include nesting too deep; truncated.");
        return String::new();
    }
    let b = source.as_bytes();
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0;
    let mut line_start = true;
    #[derive(PartialEq)]
    enum St {
        Normal,
        Str,
        Line,
        Block,
    }
    let mut st = St::Normal;
    while i < b.len() {
        if out.len() > MAX_OUTPUT_BYTES {
            warn_once(budget, warnings, "WARNING: include expansion too large; truncated.");
            break;
        }
        let c = b[i];
        match st {
            St::Normal => {
                if line_start && !c.is_ascii_whitespace() {
                    if let Some((kind, path, consumed)) = match_directive_len(source, i) {
                        process_directive(
                            kind, &path, base, root, seen, warnings, uses, budget, depth, &mut out,
                        );
                        i += consumed;
                        line_start = false;
                        continue;
                    }
                    line_start = false; // committed to code on this line
                }
                if c == b'"' {
                    st = St::Str;
                    out.push(c);
                    i += 1;
                } else if c == b'/' && b.get(i + 1) == Some(&b'/') {
                    st = St::Line;
                    out.extend_from_slice(b"//");
                    i += 2;
                } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
                    st = St::Block;
                    out.extend_from_slice(b"/*");
                    i += 2;
                } else if c == b'\n' {
                    line_start = true;
                    out.push(c);
                    i += 1;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
            St::Str => {
                if c == b'\\' {
                    out.push(c);
                    if let Some(&n) = b.get(i + 1) {
                        out.push(n);
                    }
                    i += 2;
                } else {
                    if c == b'"' {
                        st = St::Normal;
                    }
                    out.push(c);
                    i += 1;
                }
            }
            St::Line => {
                if c == b'\n' {
                    st = St::Normal;
                    line_start = true;
                }
                out.push(c);
                i += 1;
            }
            St::Block => {
                if c == b'*' && b.get(i + 1) == Some(&b'/') {
                    st = St::Normal;
                    out.extend_from_slice(b"*/");
                    i += 2;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

fn warn_once(budget: &mut Budget, warnings: &mut Vec<String>, msg: &str) {
    if !budget.capped {
        budget.capped = true;
        warnings.push(msg.to_string());
    }
}

#[allow(clippy::too_many_arguments)]
fn process_directive(
    kind: Kind,
    path: &str,
    base: &Path,
    root: &Path,
    seen: &mut HashSet<PathBuf>,
    warnings: &mut Vec<String>,
    uses: &mut Vec<(PathBuf, PathBuf)>,
    budget: &mut Budget,
    depth: usize,
    out: &mut Vec<u8>,
) {
    match kind {
        Kind::Include => {
            let resolved = match resolve_contained(base, root, path) {
                Some(p) => p,
                None => {
                    warnings.push(format!("WARNING: Can't open include file '{}'.", path));
                    return;
                }
            };
            if seen.contains(&resolved) {
                return; // active-ancestor cycle: skip to avoid infinite expansion
            }
            budget.expansions += 1;
            if budget.expansions > MAX_INCLUDE_EXPANSIONS {
                warn_once(budget, warnings, "WARNING: too many include expansions; truncated.");
                return;
            }
            match std::fs::read_to_string(&resolved) {
                Ok(text) => {
                    seen.insert(resolved.clone());
                    let dir = resolved.parent().unwrap_or(base).to_path_buf();
                    let inner =
                        inline_includes(&text, &dir, root, seen, warnings, uses, budget, depth + 1);
                    out.extend_from_slice(inner.as_bytes());
                    seen.remove(&resolved);
                }
                Err(_) => {
                    warnings.push(format!("WARNING: Can't open include file '{}'.", path));
                }
            }
        }
        Kind::Use => match resolve_contained(base, root, path) {
            Some(resolved) => {
                let dir = resolved.parent().unwrap_or(base).to_path_buf();
                uses.push((resolved, dir));
            }
            None => warnings.push(format!("WARNING: Can't open library '{}'.", path)),
        },
    }
}

/// Match an `include <path>` / `use <path>` directive at byte offset `i`
/// (which must be the first non-whitespace char of a line, in normal context).
/// Returns the kind, the path, and the number of bytes the directive tokens
/// occupy (so the caller can consume exactly them and keep any trailing code).
fn match_directive_len(src: &str, i: usize) -> Option<(Kind, String, usize)> {
    let s = src.get(i..)?;
    for (kw, kind) in [("include", Kind::Include), ("use", Kind::Use)] {
        if let Some(after) = s.strip_prefix(kw) {
            let trimmed = after.trim_start();
            let ws = after.len() - trimmed.len();
            // The keyword must be followed (after optional whitespace) by the
            // `<path>` bracket — so `used`, `useful`, `includeme` are not it.
            if let Some(rest) = trimmed.strip_prefix('<') {
                if let Some(end) = rest.find('>') {
                    let path = rest[..end].trim().to_string();
                    let consumed = kw.len() + ws + 1 + end + 1;
                    return Some((kind, path, consumed));
                }
            }
        }
    }
    None
}

/// Resolve a directive path against `base`, then require the canonicalized
/// target to stay under `root` — refusing absolute paths, `..` traversal, and
/// symlink escapes. A missing file returns its lexical path so the read fails
/// and warns (a nonexistent file is not a containment breach).
fn resolve_contained(base: &Path, root: &Path, path: &str) -> Option<PathBuf> {
    if path.is_empty() {
        return None;
    }
    let p = Path::new(path);
    if p.is_absolute() || p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return None;
    }
    let joined = base.join(p);
    match joined.canonicalize() {
        Ok(canon) => {
            if canon.starts_with(root) {
                Some(canon)
            } else {
                None // symlink (or alias) escaping the sandbox root
            }
        }
        Err(_) => Some(joined), // missing file: let the read fail and warn
    }
}

fn display(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md(line: &str) -> Option<(Kind, String)> {
        // Directive detection at a line start (offset 0), matching how the
        // scanner probes the first non-whitespace char.
        let t = line.trim_start();
        let off = line.len() - t.len();
        match_directive_len(line, off).map(|(k, p, _)| (k, p))
    }

    #[test]
    fn directive_recognition() {
        assert!(matches!(md("include <a.scad>"), Some((Kind::Include, _))));
        assert!(matches!(md("  use <lib/b.scad> // note"), Some((Kind::Use, _))));
        assert_eq!(md("use <x.scad>").unwrap().1, "x.scad");
        assert!(md("used = 1;").is_none());
        assert!(md("cube(1);").is_none());
        assert!(md("useful <x>").is_none());
        assert!(md("includeme <x>").is_none());
    }

    #[test]
    fn path_sandbox_refuses_escapes() {
        let base = Path::new("/tmp/x");
        let root = Path::new("/tmp/x");
        // A non-existent in-sandbox path returns its lexical form (read warns).
        assert!(resolve_contained(base, root, "lib/a.scad").is_some());
        assert!(resolve_contained(base, root, "/etc/passwd").is_none());
        assert!(resolve_contained(base, root, "../secret.scad").is_none());
        assert!(resolve_contained(base, root, "a/../../b.scad").is_none());
    }

    #[test]
    fn directive_inside_string_or_comment_is_inert() {
        // The security fix: an `include` hidden in a string literal or comment
        // must NOT trigger a file read (it stays verbatim in the output).
        let base = Path::new(".");
        let root = Path::new(".");
        let mut seen = HashSet::new();
        let mut warns = Vec::new();
        let mut uses = Vec::new();
        let mut budget = Budget { expansions: 0, capped: false };
        let src = "x = \"A\ninclude <secret.txt>\nB\";\n/*\ninclude <also.txt>\n*/\ncube(1);";
        let out = inline_includes(src, base, root, &mut seen, &mut warns, &mut uses, &mut budget, 0);
        assert!(out.contains("include <secret.txt>"), "string-embedded directive stays literal");
        assert!(out.contains("include <also.txt>"), "comment-embedded directive stays literal");
        assert!(warns.is_empty(), "no file read attempted: {:?}", warns);
        // Trailing code after a real directive survives.
        assert_eq!(md("include <a.scad>").is_some() as u8, 1);
    }
}
