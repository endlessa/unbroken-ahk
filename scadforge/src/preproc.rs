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

/// Resolve every include/use in `source` (whose directory is `base`) and parse
/// the result into one program, with the definitions of every `use`d file
/// prepended (so local definitions shadow them via last-write-wins).
pub fn resolve(source: &str, base: &Path) -> Resolved {
    let mut warnings = Vec::new();
    let mut uses: Vec<(PathBuf, PathBuf)> = Vec::new(); // (resolved file, its dir)
    let mut seen = HashSet::new();
    let inlined = inline_includes(source, base, &mut seen, &mut warnings, &mut uses);

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
        let inner = inline_includes(&text, &dir, &mut inner_seen, &mut warnings, &mut inner_uses);
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
fn inline_includes(
    source: &str,
    base: &Path,
    seen: &mut HashSet<PathBuf>,
    warnings: &mut Vec<String>,
    uses: &mut Vec<(PathBuf, PathBuf)>,
) -> String {
    let mut out = String::new();
    for line in source.lines() {
        match parse_directive(line) {
            Some((Kind::Include, path)) => {
                let resolved = match resolve_path(base, &path) {
                    Some(p) => p,
                    None => {
                        warnings.push(format!("WARNING: Can't open include file '{}'.", path));
                        out.push('\n');
                        continue;
                    }
                };
                if seen.contains(&resolved) {
                    // Cycle (or repeat within this chain): skip to avoid an
                    // infinite expansion. Plain double-include across separate
                    // chains still re-executes.
                    out.push('\n');
                    continue;
                }
                match std::fs::read_to_string(&resolved) {
                    Ok(text) => {
                        seen.insert(resolved.clone());
                        let dir = resolved.parent().unwrap_or(base).to_path_buf();
                        let inner = inline_includes(&text, &dir, seen, warnings, uses);
                        out.push_str(&inner);
                        out.push('\n');
                        seen.remove(&resolved);
                    }
                    Err(_) => {
                        warnings.push(format!("WARNING: Can't open include file '{}'.", path));
                        out.push('\n');
                    }
                }
            }
            Some((Kind::Use, path)) => {
                match resolve_path(base, &path) {
                    Some(resolved) => {
                        let dir = resolved.parent().unwrap_or(base).to_path_buf();
                        uses.push((resolved, dir));
                    }
                    None => warnings.push(format!("WARNING: Can't open library '{}'.", path)),
                }
                out.push('\n'); // keep line numbers stable
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}

/// Recognize an `include <path>` / `use <path>` directive line. Returns the
/// kind and the raw path. A comment line or a bare identifier that merely
/// starts with "use"/"include" is not a directive.
fn parse_directive(line: &str) -> Option<(Kind, String)> {
    let t = line.trim_start();
    if t.starts_with("//") || t.starts_with('#') {
        return None;
    }
    for (kw, kind) in [("include", Kind::Include), ("use", Kind::Use)] {
        if let Some(rest) = t.strip_prefix(kw) {
            // A word boundary: the keyword must be followed by whitespace or
            // directly by '<' (so `used = 1;` is not a directive).
            let rest = rest.trim_start();
            if rest.starts_with('<') {
                if let Some(end) = rest.find('>') {
                    return Some((kind, rest[1..end].trim().to_string()));
                }
            }
            // `strip_prefix` matched but this isn't a directive; only bail out
            // of the include branch, still let `use` be tried and vice versa.
        }
    }
    None
}

/// Resolve a directive path against `base`, refusing absolute paths and any
/// `..` traversal (the sandbox). None => refused.
fn resolve_path(base: &Path, path: &str) -> Option<PathBuf> {
    if path.is_empty() {
        return None;
    }
    let p = Path::new(path);
    if p.is_absolute() || p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return None;
    }
    Some(base.join(p))
}

fn display(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directive_recognition() {
        assert!(matches!(parse_directive("include <a.scad>"), Some((Kind::Include, _))));
        assert!(matches!(parse_directive("  use <lib/b.scad> // note"), Some((Kind::Use, _))));
        assert_eq!(parse_directive("use <x.scad>").unwrap().1, "x.scad");
        // Not directives:
        assert!(parse_directive("used = 1;").is_none());
        assert!(parse_directive("// include <a.scad>").is_none());
        assert!(parse_directive("cube(1);").is_none());
        assert!(parse_directive("a = b < c;").is_none());
    }

    #[test]
    fn path_sandbox_refuses_escapes() {
        let base = Path::new("/tmp/x");
        assert!(resolve_path(base, "lib/a.scad").is_some());
        assert!(resolve_path(base, "/etc/passwd").is_none());
        assert!(resolve_path(base, "../secret.scad").is_none());
        assert!(resolve_path(base, "a/../../b.scad").is_none());
    }
}
