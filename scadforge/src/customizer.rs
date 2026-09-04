//! Customizer parameter model — the comment-based parameter surface.
//!
//! Scans the MAIN file's source for top-level assignments with a LITERAL
//! right-hand side (number / bool / string / vector-of-numbers) appearing
//! BEFORE the first module/function definition. A trailing `// [...]` comment
//! selects the widget (slider / dropdown / …); a `// description` line just
//! above supplies the label; a standalone `/* [Group] */` block comment opens
//! a tab (`[Hidden]` hides, `[Global]` shows everywhere). Values a caller
//! overrides are re-applied as trailing top-level assignments (last-write-
//! wins), the same mechanism the GUI, presets, and CLI `-D` use.
//!
//! Line-based and preview-grade: parameters are the simple one-liners the
//! Customizer grammar targets.

#[derive(Debug, Clone, PartialEq)]
pub enum Widget {
    Spinbox,
    Checkbox,
    Textbox,
    Slider { min: f64, step: Option<f64>, max: f64 },
    /// (value, label) options; value is the literal as written.
    Dropdown(Vec<(String, String)>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Number,
    Bool,
    String,
    Vector,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub group: String, // "" = default tab; "Hidden" hidden; "Global" everywhere
    pub description: String,
    pub value: String, // default RHS as an OpenSCAD literal string
    pub kind: Kind,
    pub widget: Widget,
}

/// Parse the customizable parameters from the source (in first-appearance
/// order). Stops at the first top-level module/function definition.
pub fn parse(source: &str) -> Vec<Parameter> {
    let mut params = Vec::new();
    let mut group = String::new();
    let mut pending_desc = String::new();
    for raw in source.lines() {
        let line = raw.trim();
        // A standalone /* [Group] */ block comment opens a section.
        if let Some(g) = section_name(line) {
            group = g;
            pending_desc.clear();
            continue;
        }
        // A `// text` line (not a widget) becomes the next parameter's label.
        if let Some(rest) = line.strip_prefix("//") {
            let t = rest.trim();
            if !t.starts_with('[') {
                pending_desc = t.to_string();
            }
            continue;
        }
        if line.is_empty() {
            continue;
        }
        // Stop scanning at the first top-level definition.
        if line.starts_with("module ") || line.starts_with("function ") {
            break;
        }
        if let Some(p) = parse_assignment(line, &group, &pending_desc) {
            params.push(p);
        }
        pending_desc.clear();
    }
    params
}

/// `/* [Name] */` (the whole block comment is just the bracketed name).
fn section_name(line: &str) -> Option<String> {
    let inner = line.strip_prefix("/*")?.strip_suffix("*/")?.trim();
    let name = inner.strip_prefix('[')?.strip_suffix(']')?.trim();
    Some(name.to_string())
}

/// Parse `name = <literal> ; // [widget]` into a Parameter, or None if the RHS
/// is not a literal (a computed expression gets no widget).
fn parse_assignment(line: &str, group: &str, desc: &str) -> Option<Parameter> {
    let eq = line.find('=')?;
    let name = line[..eq].trim();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') {
        return None;
    }
    let after = &line[eq + 1..];
    let semi = after.find(';')?;
    let rhs = after[..semi].trim();
    // Split any trailing comment off the tail after ';'.
    let tail = after[semi + 1..].trim();
    let widget_src = tail.strip_prefix("//").map(str::trim).unwrap_or("");

    let (kind, value) = classify_literal(rhs)?;
    let widget = widget_for(&kind, widget_src);
    Some(Parameter {
        name: name.to_string(),
        group: group.to_string(),
        description: desc.to_string(),
        value,
        kind,
        widget,
    })
}

/// Recognize a literal RHS and its kind. Anything with an operator, call, or
/// bare identifier is not a literal → no widget.
fn classify_literal(rhs: &str) -> Option<(Kind, String)> {
    if rhs.is_empty() {
        return None;
    }
    if rhs == "true" || rhs == "false" {
        return Some((Kind::Bool, rhs.to_string()));
    }
    if is_string_literal(rhs) {
        // A SINGLE string literal — the only unescaped `"` are the outer two.
        // This rejects concatenations (`"a" + "b"`) and, crucially, any value
        // that closes the string early to smuggle statements after it
        // (`"a"; cube(9); //"`), keeping the RHS a bare literal token.
        return Some((Kind::String, rhs.to_string()));
    }
    if rhs.starts_with('[') && rhs.ends_with(']') {
        // A vector of NUMBER literals only.
        let inner = &rhs[1..rhs.len() - 1];
        if inner.split(',').all(|e| is_number_literal(e.trim())) && !inner.trim().is_empty() {
            return Some((Kind::Vector, rhs.to_string()));
        }
        return None;
    }
    if is_number_literal(rhs) {
        return Some((Kind::Number, rhs.to_string()));
    }
    None
}

/// A plain decimal number literal (optionally signed, with a fraction or
/// exponent) — but NOT an expression like `1+2` or `w`.
fn is_number_literal(s: &str) -> bool {
    !s.is_empty() && s.parse::<f64>().is_ok()
}

/// Is `s` exactly ONE double-quoted string literal — outer quotes only, with
/// every interior `"` backslash-escaped and the closing quote unescaped? This
/// is what makes `classify_literal` safe: a value like `"a"; cube(9); //"`
/// closes its string at the second `"`, so it is rejected rather than pasted
/// into the RHS as if it were a literal.
fn is_string_literal(s: &str) -> bool {
    let b = s.as_bytes();
    let n = b.len();
    if n < 2 || b[0] != b'"' || b[n - 1] != b'"' {
        return false;
    }
    let mut i = 1;
    while i < n - 1 {
        match b[i] {
            b'\\' => i += 2,       // an escape consumes the next byte
            b'"' => return false,  // an unescaped quote closes the string early
            _ => i += 1,
        }
    }
    // If the final `"` was reached only because an escape ran off the end, the
    // closing quote was actually escaped (e.g. `"\"`) → not a closed literal.
    i == n - 1
}

/// Choose the widget from the parameter kind and the trailing bracket comment.
fn widget_for(kind: &Kind, comment: &str) -> Widget {
    match kind {
        Kind::Bool => Widget::Checkbox,
        Kind::String => match parse_bracket(comment) {
            Some(items) => Widget::Dropdown(items),
            None => Widget::Textbox,
        },
        Kind::Number | Kind::Vector => {
            let inner = match bracket_inner(comment) {
                Some(i) => i,
                None => return Widget::Spinbox,
            };
            // A range "[min:max]" / "[min:step:max]" / "[max]" → slider;
            // otherwise a value list → dropdown.
            if inner.contains(':') {
                let parts: Vec<f64> =
                    inner.split(':').filter_map(|p| p.trim().parse::<f64>().ok()).collect();
                match parts.len() {
                    2 => return Widget::Slider { min: parts[0], step: None, max: parts[1] },
                    3 => {
                        return Widget::Slider { min: parts[0], step: Some(parts[1]), max: parts[2] }
                    }
                    _ => {}
                }
            }
            if let Some(v) = inner.trim().parse::<f64>().ok().filter(|_| !inner.contains(',')) {
                return Widget::Slider { min: 0.0, step: None, max: v }; // "[max]"
            }
            match parse_bracket(comment) {
                Some(items) => Widget::Dropdown(items),
                None => Widget::Spinbox,
            }
        }
    }
}

/// Apply caller overrides to the source. Each parameter's declaration line is
/// rewritten IN PLACE — its right-hand side is replaced with the override value
/// while the leading `name =`, the trailing `;`, and the `// [widget]` comment
/// are preserved. This is what the reference Customizer does: the value shown in
/// the panel IS the value that renders, with no duplicate assignment and no
/// "variable reassigned" diagnostic. It works because the panel edits the same
/// slot the file declares. This is the GUI / preset / CLI `-D` mechanism.
///
/// Overrides are validated against the parsed parameter model: an override is
/// honored only when its `name` is a real customizer parameter AND its `value`
/// is a literal of that parameter's kind. So a malformed or hostile value can
/// never smuggle arbitrary statements into the source — the replacement is
/// always a bare literal token in the RHS position. Unknown names and type
/// mismatches are dropped. `overrides` is `(name, literal)` where the literal is
/// written as OpenSCAD source (`20`, `true`, `"round"`, `[1, 2, 3]`).
pub fn apply_overrides(source: &str, overrides: &[(String, String)]) -> String {
    if overrides.is_empty() {
        return source.to_string();
    }
    let params = parse(source);
    // Validate each override and reduce to (name -> replacement literal). A
    // later override for the same name wins (matches last-write-wins).
    let mut repl: Vec<(String, String)> = Vec::new();
    for (name, raw) in overrides {
        let Some(p) = params.iter().find(|p| &p.name == name) else { continue };
        let Some((kind, lit)) = classify_literal(raw.trim()) else { continue };
        if kind != p.kind {
            continue;
        }
        // A literal is rewritten onto a single line; a raw newline (or CR) would
        // split it and corrupt the source, so reject one outright.
        if lit.contains('\n') || lit.contains('\r') {
            continue;
        }
        match repl.iter_mut().find(|(n, _)| n == &p.name) {
            Some(slot) => slot.1 = lit,
            None => repl.push((p.name.clone(), lit)),
        }
    }
    if repl.is_empty() {
        return source.to_string();
    }
    // Walk the lines up to the first module/function (the same window `parse`
    // scans) and rewrite each matched declaration line's RHS once.
    let mut applied = vec![false; repl.len()];
    let mut out_lines: Vec<String> = Vec::with_capacity(source.lines().count() + 1);
    let mut stop = false;
    for raw in source.lines() {
        if !stop {
            let t = raw.trim_start();
            if t.starts_with("module ") || t.starts_with("function ") {
                stop = true;
            } else if let Some(idx) =
                repl.iter().position(|(n, _)| !applied_at(&applied, &repl, n) && line_assigns(t, n))
            {
                out_lines.push(rewrite_rhs(raw, &repl[idx].1));
                applied[idx] = true;
                continue;
            }
        }
        out_lines.push(raw.to_string());
    }
    let mut out = out_lines.join("\n");
    if source.ends_with('\n') {
        out.push('\n');
    }
    // Fallback: any validated override whose line wasn't found (it always
    // should be, since it came from `parse`) is appended so it still applies.
    let mut tail = String::new();
    for (i, (n, lit)) in repl.iter().enumerate() {
        if !applied[i] {
            tail.push_str(&format!("{} = {};\n", n, lit));
        }
    }
    if !tail.is_empty() {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&tail);
    }
    out
}

/// A named parameter set from a Customizer preset file: the set's name and its
/// `(parameter name, value-string)` pairs. Values are stored as the reference's
/// `<file>.json` sidecar records them — bare strings: `"20"`, `"true"`,
/// `"round"` (no surrounding quotes), `"[1, 2, 3]"`.
pub type PresetSet = (String, Vec<(String, String)>);

/// Parse the `parameterSets` map of a Customizer preset JSON into named sets,
/// preserving order. A missing/misshapen `parameterSets`, or unparseable JSON,
/// yields no sets (a corrupt sidecar simply offers nothing — never an error).
/// Each value is normalized to its stored string form whether the file wrote it
/// as a JSON string (the reference form) or, leniently, as a native number/bool.
pub fn parse_presets(json: &str) -> Vec<PresetSet> {
    let Ok(root) = unbroken_test_platform::json::parse_json(json) else { return Vec::new() };
    let Some(sets) = root.get("parameterSets").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (set_name, body) in sets {
        let Some(fields) = body.as_object() else { continue };
        let mut params = Vec::new();
        for (pname, pval) in fields {
            params.push((pname.clone(), json_value_as_string(pval)));
        }
        out.push((set_name.clone(), params));
    }
    out
}

/// The stored string form of a preset value: a JSON string verbatim, a number
/// via the project's canonical formatter-free `{}`? No — plain `to_string`
/// would print `20` for 20.0. Use a compact numeric rendering; bools become
/// `true`/`false`; anything else falls back to its compact JSON text.
fn json_value_as_string(v: &unbroken_test_platform::json::JsonValue) -> String {
    use unbroken_test_platform::json::JsonValue;
    match v {
        JsonValue::Str(s) => s.clone(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => {
            if n.fract() == 0.0 && n.is_finite() {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        other => unbroken_test_platform::json::to_json_compact(other),
    }
}

/// Serialize named parameter sets to the reference preset-JSON shape:
/// `{"parameterSets": {name: {param: "value", ...}, ...}, "fileFormatVersion": "1"}`.
/// Values are written as JSON strings, matching how the reference records them.
pub fn write_presets(sets: &[PresetSet]) -> String {
    use unbroken_test_platform::json::{obj, str_val, to_json_pretty, JsonValue};
    let set_objs: Vec<(String, JsonValue)> = sets
        .iter()
        .map(|(name, params)| {
            let fields: Vec<(&str, JsonValue)> =
                params.iter().map(|(k, val)| (k.as_str(), str_val(val))).collect();
            (name.clone(), obj(fields))
        })
        .collect();
    let root = obj(vec![
        ("parameterSets", JsonValue::Object(set_objs)),
        ("fileFormatVersion", str_val("1")),
    ]);
    to_json_pretty(&root)
}

/// Convert one preset set's stored value-strings into overrides for `source`
/// (each value promoted to a literal of the parameter's kind). The result is
/// fed to `apply_overrides`, which re-validates, so an entry naming an unknown
/// parameter or carrying a bad value is harmlessly dropped there.
pub fn preset_to_overrides(source: &str, set: &[(String, String)]) -> Vec<(String, String)> {
    let params = parse(source);
    set.iter()
        .filter_map(|(name, valstr)| {
            let p = params.iter().find(|p| &p.name == name)?;
            Some((name.clone(), value_string_to_literal(&p.kind, valstr)))
        })
        .collect()
}

/// Promote a preset's stored value-string to an OpenSCAD literal of `kind`.
/// A String parameter's value is quoted (unless already quoted); numbers,
/// bools, and vectors are already in literal form.
fn value_string_to_literal(kind: &Kind, valstr: &str) -> String {
    match kind {
        Kind::String => {
            let t = valstr.trim();
            if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
                t.to_string()
            } else {
                // Quote and escape (backslash and double-quote).
                let esc = valstr.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\"{}\"", esc)
            }
        }
        _ => valstr.trim().to_string(),
    }
}

/// Has the override for `name` already been applied on an earlier line?
fn applied_at(applied: &[bool], repl: &[(String, String)], name: &str) -> bool {
    repl.iter().position(|(n, _)| n == name).map(|i| applied[i]).unwrap_or(true)
}

/// Does this (already left-trimmed) line assign to exactly `name`?
fn line_assigns(trimmed: &str, name: &str) -> bool {
    parse_assignment(trimmed, "", "").map(|p| p.name == name).unwrap_or(false)
}

/// Replace the RHS of `raw` (`indent name = <rhs> ; tail`) with `new_lit`,
/// preserving the name, the `;`, and any trailing comment.
fn rewrite_rhs(raw: &str, new_lit: &str) -> String {
    let Some(eq) = raw.find('=') else { return raw.to_string() };
    let after = &raw[eq + 1..];
    let Some(semi) = after.find(';') else { return raw.to_string() };
    format!("{}= {}{}", &raw[..eq], new_lit, &after[semi..])
}

fn bracket_inner(comment: &str) -> Option<&str> {
    comment.trim().strip_prefix('[')?.strip_suffix(']')
}

/// Parse a `[a, b, c]` or `[10:Small, 20:Large]` value list into (value,
/// label) pairs. Labels default to the value.
fn parse_bracket(comment: &str) -> Option<Vec<(String, String)>> {
    let inner = bracket_inner(comment)?;
    if inner.contains(':') && inner.split(',').count() == 1 && !inner.contains(',') {
        // A single "min:max" range is not a dropdown.
        // (handled by the caller as a slider)
    }
    let mut out = Vec::new();
    for item in inner.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        // "value:label" (label may contain spaces).
        if let Some((v, l)) = item.split_once(':') {
            // Only treat as labeled if the label isn't purely another number
            // forming a range (that case is a slider, handled earlier).
            out.push((v.trim().to_string(), l.trim().to_string()));
        } else {
            out.push((item.to_string(), item.to_string()));
        }
    }
    if out.len() >= 2 || (out.len() == 1 && inner.contains(',')) {
        Some(out)
    } else if out.len() == 1 && inner.contains(':') {
        Some(out) // a single labeled entry
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_typed_parameters_with_widgets_and_groups() {
        let src = "\
// Diameter of the base\n\
diameter = 20; // [10:50]\n\
height = 5;\n\
smooth = true;\n\
label = \"hi\";\n\
mode = \"round\"; // [round, sharp]\n\
detail = 12; // [1:1:64]\n\
size = [10, 20, 30];\n\
/* [Advanced] */\n\
tol = 0.2;\n\
/* [Hidden] */\n\
seed = 7;\n\
computed = height * 2;\n\
module part() { cube(1); }\n\
after = 99;\n"; // after the first module → not scanned
        let ps = parse(src);
        let by = |n: &str| ps.iter().find(|p| p.name == n);
        assert_eq!(by("diameter").unwrap().description, "Diameter of the base");
        assert!(matches!(by("diameter").unwrap().widget, Widget::Slider { min, max, .. } if min == 10.0 && max == 50.0));
        assert!(matches!(by("height").unwrap().widget, Widget::Spinbox));
        assert!(matches!(by("smooth").unwrap().widget, Widget::Checkbox));
        assert!(matches!(by("label").unwrap().widget, Widget::Textbox));
        assert!(matches!(by("mode").unwrap().widget, Widget::Dropdown(_)));
        assert!(matches!(by("detail").unwrap().widget, Widget::Slider { step: Some(s), .. } if s == 1.0));
        assert_eq!(by("size").unwrap().kind, Kind::Vector);
        assert_eq!(by("tol").unwrap().group, "Advanced");
        assert_eq!(by("seed").unwrap().group, "Hidden");
        // A computed RHS is NOT a parameter; nothing after the first module is scanned.
        assert!(by("computed").is_none());
        assert!(by("after").is_none());
    }

    #[test]
    fn labeled_dropdown_and_negative_and_single_max() {
        let ps = parse("n = 10; // [10:Small, 20:Large]\noff = -5;\nq = 8; // [64]\n");
        match &ps.iter().find(|p| p.name == "n").unwrap().widget {
            Widget::Dropdown(items) => {
                assert_eq!(items[0], ("10".into(), "Small".into()));
                assert_eq!(items[1], ("20".into(), "Large".into()));
            }
            w => panic!("expected dropdown, got {:?}", w),
        }
        // Negative literal still gets a widget (spinbox).
        assert_eq!(ps.iter().find(|p| p.name == "off").unwrap().value, "-5");
        assert!(matches!(ps.iter().find(|p| p.name == "q").unwrap().widget, Widget::Slider { min, max, .. } if min == 0.0 && max == 64.0));
    }

    #[test]
    fn overrides_are_validated_and_rewritten_in_place() {
        let src = "size = 10; // [1:50]\nname = \"a\";\nflag = false;\nvec = [1, 2];\n";
        let ov = vec![
            ("size".to_string(), "42".to_string()),          // ok: number → number
            ("name".to_string(), "\"b\"".to_string()),       // ok: string → string
            ("flag".to_string(), "true".to_string()),        // ok: bool → bool
            ("vec".to_string(), "[3, 4, 5]".to_string()),    // ok: vector → vector
            ("size".to_string(), "cube(9); size".to_string()), // rejected: not a literal
            ("name".to_string(), "5".to_string()),            // rejected: kind mismatch
            ("ghost".to_string(), "1".to_string()),           // rejected: unknown name
        ];
        let out = apply_overrides(src, &ov);
        // Each declaration line is rewritten IN PLACE — RHS replaced, widget
        // comment preserved, and NO duplicate/appended assignment (so no
        // "reassigned" warning at eval time).
        assert!(out.contains("size = 42; // [1:50]"), "RHS rewritten, comment kept: {out}");
        assert!(out.contains("name = \"b\";"));
        assert!(out.contains("flag = true;"));
        assert!(out.contains("vec = [3, 4, 5];"));
        // The original defaults are gone (replaced, not shadowed).
        assert!(!out.contains("size = 10"));
        assert!(!out.contains("[1, 2]"));
        // Nothing hostile or mismatched leaks through.
        assert!(!out.contains("cube(9)"));
        assert!(!out.contains("name = 5;"));
        assert!(!out.contains("ghost"));
        // Exactly one assignment per name (no appended duplicate).
        assert_eq!(out.matches("size =").count(), 1);
        // No overrides → source returned unchanged.
        assert_eq!(apply_overrides(src, &[]), src);
    }

    #[test]
    fn preset_json_parses_selects_and_round_trips() {
        let json = "{\
            \"parameterSets\": {\
              \"Small\": {\"width\": \"20\", \"label\": \"a\", \"on\": \"false\"},\
              \"Large\": {\"width\": \"100\", \"label\": \"b\", \"on\": \"true\"}\
            },\
            \"fileFormatVersion\": \"1\"}";
        let sets = parse_presets(json);
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[0].0, "Small");
        assert_eq!(sets[1].0, "Large");
        let large = &sets.iter().find(|s| s.0 == "Large").unwrap().1;
        assert!(large.iter().any(|(n, v)| n == "width" && v == "100"));

        // Promote a set to overrides against a source: the string param gets
        // quoted, the number/bool stay literal.
        let src = "width = 1; // [0:200]\nlabel = \"x\";\non = false;\ncube(width);";
        let ov = preset_to_overrides(src, large);
        assert!(ov.contains(&("width".to_string(), "100".to_string())));
        assert!(ov.contains(&("label".to_string(), "\"b\"".to_string())));
        assert!(ov.contains(&("on".to_string(), "true".to_string())));
        // And they apply in place.
        let out = apply_overrides(src, &ov);
        assert!(out.contains("width = 100;"));
        assert!(out.contains("label = \"b\";"));
        assert!(out.contains("on = true;"));

        // Round-trip: write the parsed sets back and re-parse to the same data.
        let reparsed = parse_presets(&write_presets(&sets));
        assert_eq!(reparsed, sets);

        // Junk / missing map → no sets, no panic.
        assert!(parse_presets("not json").is_empty());
        assert!(parse_presets("{\"x\":1}").is_empty());
    }

    #[test]
    fn preset_accepts_native_json_scalars_too() {
        // A lenient sidecar that stored raw numbers/bools instead of strings.
        let sets = parse_presets("{\"parameterSets\":{\"S\":{\"n\": 20, \"b\": true}}}");
        let s = &sets[0].1;
        assert!(s.contains(&("n".to_string(), "20".to_string())));
        assert!(s.contains(&("b".to_string(), "true".to_string())));
    }

    #[test]
    fn string_override_cannot_smuggle_statements() {
        // A String parameter whose override value tries to close the string
        // early and append a statement must be REJECTED (not a single literal),
        // so the declaration is left at its default and nothing is injected.
        let src = "name = \"a\"; // a label\ncube(1);";
        let attacks = [
            "\"a\"; cube(999); //",       // close early, append a call
            "\"a\" + \"b\"",                // concatenation is not a literal
            "\"a\"\ncube(9); x=\"",       // embedded newline
            "\"\\\"",                        // the closing quote is escaped → unterminated
        ];
        for a in attacks {
            let out = apply_overrides(src, &[("name".to_string(), a.to_string())]);
            assert!(!out.contains("cube(999)"), "injection via {:?}: {}", a, out);
            assert!(!out.contains("cube(9)"), "injection via {:?}: {}", a, out);
            // The original default declaration survives unchanged.
            assert!(out.contains("name = \"a\";"), "default kept for {:?}", a);
        }
        // A legitimate string with an escaped interior quote IS accepted.
        let ok = apply_overrides(src, &[("name".to_string(), "\"a\\\"b\"".to_string())]);
        assert!(ok.contains("name = \"a\\\"b\";"), "valid escaped quote applied: {ok}");
        // classify_literal itself no longer mistakes a concatenation for a literal.
        assert!(classify_literal("\"a\" + \"b\"").is_none());
        assert_eq!(classify_literal("\"round\"").unwrap().0, Kind::String);
        assert_eq!(classify_literal("\"\"").unwrap().0, Kind::String); // empty string ok
    }

    #[test]
    fn override_after_first_module_is_not_touched() {
        // A same-named assignment BELOW the first module is out of the
        // customizer window and must be left alone.
        let src = "size = 1; // [0:10]\nmodule m() { }\nsize = 2;\n";
        let out = apply_overrides(src, &[("size".to_string(), "9".to_string())]);
        assert!(out.contains("size = 9; // [0:10]"));
        assert!(out.contains("size = 2;")); // the post-module line is preserved verbatim
    }
}
