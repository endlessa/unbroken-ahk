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

use crate::ast::{Expr, Param, VecItem};
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
    // (resolved file, its dir, the path exactly as the directive spelled it)
    let mut uses: Vec<(PathBuf, PathBuf, String)> = Vec::new();
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
    // resolved first). Its top-level geometry is dropped; its top-level
    // variables come across in a private namespace.
    let mut used_defs: Vec<Stmt> = Vec::new();
    let mut used_seen: HashSet<PathBuf> = HashSet::new();
    let mut used_tag = 0usize;
    for (path, dir, spelled) in uses {
        if !used_seen.insert(path.clone()) {
            continue; // using the same file twice is idempotent
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                // The directive's own text, not the resolved absolute path:
                // a user who wrote `use <lib.scad>` gets that name back.
                warnings.push(format!("WARNING: Can't open library '{}'.", spelled));
                continue;
            }
        };
        let mut inner_seen = HashSet::new();
        let mut inner_uses = Vec::new();
        let inner = inline_includes(
            &text, &dir, &root, &mut inner_seen, &mut warnings, &mut inner_uses, &mut budget, 0,
        );
        match parser::parse(&inner) {
            Ok(mut stmts) => {
                // The used file's own top-level constants come across too,
                // renamed into a private namespace so its definitions can
                // see them while the user cannot.
                privatize(&mut stmts, used_tag);
                used_tag += 1;
                for s in stmts {
                    if matches!(
                        s,
                        Stmt::ModuleDef { .. } | Stmt::FunctionDef { .. } | Stmt::Assign { .. }
                    ) {
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
    uses: &mut Vec<(PathBuf, PathBuf, String)>,
    budget: &mut Budget,
    depth: usize,
) -> String {
    if depth > MAX_INCLUDE_DEPTH {
        warn_once(budget, warnings, "WARNING: include nesting too deep; truncated.");
        return String::new();
    }
    let b = source.as_bytes();
    let mut out: Vec<u8> = Vec::new();
    let mut truncated = false;
    let mut i = 0;
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
            truncated = true;
            break;
        }
        let c = b[i];
        match st {
            St::Normal => {
                // `include <f>` is a LEXICAL token, not a line-oriented
                // directive: the reference documents it inside module bodies
                // and blocks, so `cube(1); include <lib.scad>` has to work.
                // Requiring a token boundary before the keyword is what keeps
                // `x = myinclude <3;` from being mistaken for one.
                if (c == b'i' || c == b'u') && at_token_start(b, i) {
                    if let Some((kind, path, consumed)) = match_directive_len(source, i) {
                        process_directive(
                            kind, &path, base, root, seen, warnings, uses, budget, depth, &mut out,
                        );
                        i += consumed;
                        continue;
                    }
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
    // A truncation must keep what it has, not throw it away. Cut back to the
    // last complete LINE: that both lands on a UTF-8 boundary and leaves whole
    // statements, so the surviving prefix still parses. Handing the raw cut to
    // `String::from_utf8(..).unwrap_or_default()` discarded the ENTIRE
    // expansion whenever the cap fell inside a multi-byte character — a
    // truncation warning followed by a completely empty file.
    if truncated {
        match out.iter().rposition(|&b| b == b'\n') {
            Some(nl) => out.truncate(nl + 1),
            None => out.clear(),
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

/// True when byte `i` begins a token: the byte before it cannot continue an
/// identifier. Without this a directive keyword would be matched inside a
/// longer name.
fn at_token_start(b: &[u8], i: usize) -> bool {
    match i.checked_sub(1).and_then(|k| b.get(k)) {
        None => true,
        Some(&p) => !(p.is_ascii_alphanumeric() || p == b'_' || p == b'$'),
    }
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
    uses: &mut Vec<(PathBuf, PathBuf, String)>,
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
                uses.push((resolved, dir, path.to_string()));
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

    #[test]
    fn a_directive_is_a_token_not_a_line() {
        // The reference documents `include` inside module bodies and blocks,
        // so it cannot be anchored to the start of a line. Anchoring it there
        // made `cube(1); include <lib.scad>` a parse error.
        let dir = std::env::temp_dir().join(format!("sfpre{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("lib.scad"), "module libthing() { sphere(3); }\n").unwrap();
        let root = dir.canonicalize().unwrap();
        let run = |src: &str| {
            let mut seen = HashSet::new();
            let mut warns = Vec::new();
            let mut uses = Vec::new();
            let mut budget = Budget { expansions: 0, capped: false };
            let out =
                inline_includes(src, &root, &root, &mut seen, &mut warns, &mut uses, &mut budget, 0);
            (out, warns)
        };
        for src in [
            "cube(1); include <lib.scad>\n",
            "module outer() { include <lib.scad> libthing(); }\n",
        ] {
            let (out, warns) = run(src);
            assert!(out.contains("module libthing"), "not expanded: {:?}", out);
            assert!(warns.is_empty(), "{:?}", warns);
        }
        // ...but the keyword still has to start a token: these are identifiers.
        let (out, warns) = run("myinclude = 3; useful = 4;\n");
        assert_eq!(out, "myinclude = 3; useful = 4;\n");
        assert!(warns.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_library_is_named_as_the_script_spelled_it() {
        // The warning used to print the RESOLVED absolute path, so a user who
        // wrote `use <nope.scad>` was shown a machine-specific /tmp/... string.
        let dir = std::env::temp_dir().join(format!("sfpre2{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("main.scad"), "use <nope.scad>\ncube(1);\n").unwrap();
        let r = resolve(&std::fs::read_to_string(dir.join("main.scad")).unwrap(), &dir);
        assert!(
            r.warnings.iter().any(|w| w == "WARNING: Can't open library 'nope.scad'."),
            "{:?}",
            r.warnings
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_oversized_expansion_truncates_to_whole_lines_and_keeps_them() {
        // The cap can fall inside a multi-byte character. Handing that raw cut
        // to `String::from_utf8(..).unwrap_or_default()` threw the WHOLE
        // expansion away: a truncation warning and a completely empty file.
        let dir = std::env::temp_dir().join(format!("sfpre3{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let pad = format!("// {}\n", "\u{4e2d}".repeat(300));
        let mut huge = String::from("cube(3);\n");
        huge.push_str(&pad.repeat(MAX_OUTPUT_BYTES / pad.len() + 200));
        std::fs::write(dir.join("huge.scad"), &huge).unwrap();
        std::fs::write(dir.join("main.scad"), "include <huge.scad>\n").unwrap();
        let r = resolve(&std::fs::read_to_string(dir.join("main.scad")).unwrap(), &dir);
        assert!(
            r.warnings.iter().any(|w| w.contains("too large")),
            "expected the truncation warning: {:?}",
            r.warnings
        );
        assert!(r.error.is_none(), "the surviving prefix must still parse: {:?}", r.error);
        assert!(!r.program.is_empty(), "the geometry before the cut must survive");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// -- use<> private constants ------------------------------------------------

/// Rewrite a used file's own top-level variables into a private namespace.
///
/// The reference: "Since 2019.05 its top-level variable assignments ARE
/// evaluated — in the used file's own private scope — so its
/// functions/modules see their own file's constants; those variables are
/// invisible to and NOT overridable by the user". Only the definitions used
/// to cross the `use` boundary, so a library's modules saw `undef` for every
/// constant the library declared about itself — which is most libraries.
///
/// Carrying the assignments over as-is would make them the user's variables:
/// visible, collide-able, and overridable by a later assignment or -D. So
/// each is renamed to a spelling the user cannot write, and references to it
/// inside that same file's definitions are renamed to match. A name the used
/// file does NOT declare is left alone, so `$fn` and genuinely global
/// identifiers still resolve outward.
fn privatize(stmts: &mut [Stmt], tag: usize) {
    let owned: HashSet<String> = stmts
        .iter()
        .filter_map(|s| match s {
            Stmt::Assign { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    if owned.is_empty() {
        return;
    }
    let pre = format!("__use{}__", tag);
    let mut shadow: Vec<HashSet<String>> = Vec::new();
    for s in stmts.iter_mut() {
        if let Stmt::Assign { name, .. } = s {
            *name = format!("{}{}", pre, name);
        }
        rn_stmt(s, &owned, &mut shadow, &pre);
    }
}

/// Is `name` rebound by an enclosing binder, so it is not the file's own?
fn masked(name: &str, shadow: &[HashSet<String>]) -> bool {
    shadow.iter().any(|f| f.contains(name))
}

/// Assignments are hoisted to the top of their scope, so a body's own
/// assignments shadow the file's constants throughout that body.
fn body_frame(body: &[Stmt]) -> HashSet<String> {
    body.iter()
        .filter_map(|s| match s {
            Stmt::Assign { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn rn_body(body: &mut Vec<Stmt>, owned: &HashSet<String>, shadow: &mut Vec<HashSet<String>>, pre: &str) {
    shadow.push(body_frame(body));
    for s in body.iter_mut() {
        rn_stmt(s, owned, shadow, pre);
    }
    shadow.pop();
}

fn rn_binds(
    binds: &mut Vec<(String, Expr)>,
    owned: &HashSet<String>,
    shadow: &mut Vec<HashSet<String>>,
    pre: &str,
) -> HashSet<String> {
    // Bindings are sequential: each value sees the earlier names, not its own.
    let mut frame = HashSet::new();
    for (n, v) in binds.iter_mut() {
        shadow.push(frame.clone());
        rn_expr(v, owned, shadow, pre);
        shadow.pop();
        frame.insert(n.clone());
    }
    frame
}

fn rn_params(params: &mut [Param], owned: &HashSet<String>, shadow: &mut Vec<HashSet<String>>, pre: &str) -> HashSet<String> {
    let mut frame = HashSet::new();
    for p in params.iter_mut() {
        if let Some(d) = &mut p.default {
            shadow.push(frame.clone());
            rn_expr(d, owned, shadow, pre);
            shadow.pop();
        }
        frame.insert(p.name.clone());
    }
    frame
}

fn rn_stmt(s: &mut Stmt, owned: &HashSet<String>, shadow: &mut Vec<HashSet<String>>, pre: &str) {
    match s {
        Stmt::Modified { stmt, .. } => rn_stmt(stmt, owned, shadow, pre),
        Stmt::Assign { value, .. } => rn_expr(value, owned, shadow, pre),
        Stmt::Call { args, children, .. } => {
            for a in args.iter_mut() {
                rn_expr(&mut a.value, owned, shadow, pre);
            }
            rn_body(children, owned, shadow, pre);
        }
        Stmt::For { bindings, body } | Stmt::IntersectionFor { bindings, body } => {
            let frame = rn_binds(bindings, owned, shadow, pre);
            shadow.push(frame);
            rn_body(body, owned, shadow, pre);
            shadow.pop();
        }
        Stmt::If { cond, then, els } => {
            rn_expr(cond, owned, shadow, pre);
            rn_body(then, owned, shadow, pre);
            rn_body(els, owned, shadow, pre);
        }
        Stmt::Let { bindings, body, .. } => {
            let frame = rn_binds(bindings, owned, shadow, pre);
            shadow.push(frame);
            rn_body(body, owned, shadow, pre);
            shadow.pop();
        }
        Stmt::ModuleDef { params, body, .. } => {
            let frame = rn_params(params, owned, shadow, pre);
            shadow.push(frame);
            rn_body(body, owned, shadow, pre);
            shadow.pop();
        }
        Stmt::FunctionDef { params, body, .. } => {
            let frame = rn_params(params, owned, shadow, pre);
            shadow.push(frame);
            rn_expr(body, owned, shadow, pre);
            shadow.pop();
        }
        Stmt::Block(body) => rn_body(body, owned, shadow, pre),
    }
}

fn rn_items(items: &mut Vec<VecItem>, owned: &HashSet<String>, shadow: &mut Vec<HashSet<String>>, pre: &str) {
    for it in items.iter_mut() {
        match it {
            VecItem::One(e) | VecItem::Each(e) => rn_expr(e, owned, shadow, pre),
            VecItem::CFor { bindings, rest } => {
                let frame = rn_binds(bindings, owned, shadow, pre);
                shadow.push(frame);
                rn_items(rest, owned, shadow, pre);
                shadow.pop();
            }
            VecItem::CForC { inits, cond, updates, rest } => {
                let frame = rn_binds(inits, owned, shadow, pre);
                shadow.push(frame);
                rn_expr(cond, owned, shadow, pre);
                for (_, v) in updates.iter_mut() {
                    rn_expr(v, owned, shadow, pre);
                }
                rn_items(rest, owned, shadow, pre);
                shadow.pop();
            }
            VecItem::CIf { cond, then, els } => {
                rn_expr(cond, owned, shadow, pre);
                rn_items(then, owned, shadow, pre);
                rn_items(els, owned, shadow, pre);
            }
            VecItem::CLet { bindings, rest } => {
                let frame = rn_binds(bindings, owned, shadow, pre);
                shadow.push(frame);
                rn_items(rest, owned, shadow, pre);
                shadow.pop();
            }
        }
    }
}

fn rn_expr(e: &mut Expr, owned: &HashSet<String>, shadow: &mut Vec<HashSet<String>>, pre: &str) {
    match e {
        Expr::Ident(n) => {
            if owned.contains(n.as_str()) && !masked(n, shadow) {
                *n = format!("{}{}", pre, n);
            }
        }
        Expr::Num(_) | Expr::Bool(_) | Expr::Str(_) | Expr::Undef => {}
        Expr::Vector(items) => rn_items(items, owned, shadow, pre),
        Expr::Range { start, step, end } => {
            rn_expr(start, owned, shadow, pre);
            if let Some(s) = step {
                rn_expr(s, owned, shadow, pre);
            }
            rn_expr(end, owned, shadow, pre);
        }
        Expr::Binary { lhs, rhs, .. } => {
            rn_expr(lhs, owned, shadow, pre);
            rn_expr(rhs, owned, shadow, pre);
        }
        Expr::Neg(x) | Expr::Pos(x) | Expr::Not(x) | Expr::Paren(x) => rn_expr(x, owned, shadow, pre),
        Expr::Ternary { cond, then, els } => {
            rn_expr(cond, owned, shadow, pre);
            rn_expr(then, owned, shadow, pre);
            rn_expr(els, owned, shadow, pre);
        }
        Expr::Index { base, index } => {
            rn_expr(base, owned, shadow, pre);
            rn_expr(index, owned, shadow, pre);
        }
        // The member NAME is a field, not a variable.
        Expr::Member { base, .. } => rn_expr(base, owned, shadow, pre),
        // The callee NAME lives in the function namespace, not the variable
        // one, and an argument's name is the callee's parameter.
        Expr::Call { args, .. } => {
            for a in args.iter_mut() {
                rn_expr(&mut a.value, owned, shadow, pre);
            }
        }
        Expr::CallValue { callee, args } => {
            rn_expr(callee, owned, shadow, pre);
            for a in args.iter_mut() {
                rn_expr(&mut a.value, owned, shadow, pre);
            }
        }
        Expr::Let { bindings, body } => {
            let frame = rn_binds(bindings, owned, shadow, pre);
            shadow.push(frame);
            rn_expr(body, owned, shadow, pre);
            shadow.pop();
        }
        Expr::EchoExpr { args, body } | Expr::AssertExpr { args, body } => {
            for a in args.iter_mut() {
                rn_expr(&mut a.value, owned, shadow, pre);
            }
            rn_expr(body, owned, shadow, pre);
        }
        Expr::FnLiteral { params, body } => {
            let frame = rn_params(params, owned, shadow, pre);
            shadow.push(frame);
            rn_expr(body, owned, shadow, pre);
            shadow.pop();
        }
    }
}
