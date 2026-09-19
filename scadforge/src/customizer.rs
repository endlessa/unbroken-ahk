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
    /// How many widgets this parameter needs: 1 for a scalar, and for a
    /// vector its component count. The reference: "vector (<=4 numeric
    /// elements) -> one spinner/slider per component, with a single range
    /// comment applying to every component". A single scalar widget for a
    /// vector was worse than none — it displayed the wrong value and every
    /// edit it produced was rejected as a kind mismatch.
    pub components: usize,
}

/// One line after comment removal: the code, the text of a `//` comment that
/// ran to end of line, the contents of every `/* ... */` block that CLOSED on
/// this line, and the net brace delta of the code (strings excluded).
struct LineScan {
    code: String,
    line_comment: Option<String>,
    blocks: Vec<String>,
    braces: i32,
}

/// Split one line into code and comments, carrying block-comment state across
/// lines in `in_block`/`block_acc`. String literals are respected, so a `//`
/// or `{` inside one is not a comment or a brace.
///
/// Bytes are moved, never re-encoded, so UTF-8 in a description or a label
/// survives: every branch pushes whole bytes in order, and the only bytes ever
/// matched are ASCII, which can never occur inside a multi-byte sequence.
fn scan_line(line: &str, in_block: &mut bool, block_acc: &mut Vec<u8>) -> LineScan {
    let b = line.as_bytes();
    let mut code: Vec<u8> = Vec::with_capacity(b.len());
    let mut line_comment = None;
    let mut blocks = Vec::new();
    let mut braces = 0i32;
    let mut in_str = false;
    let mut i = 0;
    while i < b.len() {
        if *in_block {
            if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                *in_block = false;
                blocks.push(String::from_utf8(std::mem::take(block_acc)).unwrap_or_default());
                i += 2;
            } else {
                block_acc.push(b[i]);
                i += 1;
            }
            continue;
        }
        if in_str {
            code.push(b[i]);
            if b[i] == b'\\' {
                if let Some(&n) = b.get(i + 1) {
                    code.push(n);
                }
                i += 2;
            } else {
                if b[i] == b'"' {
                    in_str = false;
                }
                i += 1;
            }
            continue;
        }
        match b[i] {
            b'"' => {
                in_str = true;
                code.push(b'"');
                i += 1;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                line_comment = Some(line[i + 2..].to_string());
                break;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                *in_block = true;
                block_acc.clear();
                i += 2;
            }
            c => {
                if c == b'{' {
                    braces += 1;
                } else if c == b'}' {
                    braces -= 1;
                }
                code.push(c);
                i += 1;
            }
        }
    }
    LineScan {
        code: String::from_utf8(code).unwrap_or_default(),
        line_comment,
        blocks,
        braces,
    }
}

/// Parse the customizable parameters from the source (in first-appearance
/// order). Stops at the first top-level module/function definition.
///
/// Only TOP-LEVEL assignments count. The scan used to be a bare line split
/// with no lexical state at all, so an assignment commented out inside a
/// `/* ... */` block became a phantom parameter (and an override was written
/// INTO the comment), and one nested in an `if`/`for` block was offered as
/// though it were customizable although the reference states flatly that
/// "Parameters inside 'if' blocks or modules are never customizable".
pub fn parse(source: &str) -> Vec<Parameter> {
    // Every top-level assignment in order: the name, the Parameter if its RHS
    // was a literal, and whether the line carried a widget comment.
    let mut found: Vec<(String, Option<Parameter>, bool)> = Vec::new();
    let mut group = String::new();
    let mut pending_desc = String::new();
    let mut in_block = false;
    let mut block_acc: Vec<u8> = Vec::new();
    let mut depth = 0i32;
    for raw in source.lines() {
        let scan = scan_line(raw, &mut in_block, &mut block_acc);
        // A block comment whose whole content is `[Name]` opens a section.
        for blk in &scan.blocks {
            let t = blk.trim();
            if let Some(name) = t.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
                group = name.trim().to_string();
                pending_desc.clear();
            }
        }
        let code = scan.code.trim();
        if code.is_empty() {
            // A `// text` line (not a widget) labels the next parameter.
            if let Some(c) = &scan.line_comment {
                let t = c.trim();
                if !t.starts_with('[') {
                    pending_desc = t.to_string();
                }
            }
            continue;
        }
        // Stop scanning at the first top-level definition.
        if depth == 0 && (code.starts_with("module ") || code.starts_with("function ")) {
            break;
        }
        let here = depth;
        depth = (depth + scan.braces).max(0);
        if here == 0 {
            if let Some(hit) = parse_assignment(
                code,
                scan.line_comment.as_deref().unwrap_or(""),
                &group,
                &pending_desc,
            ) {
                found.push(hit);
            }
        }
        pending_desc.clear();
    }
    one_per_name(found)
}

/// Collapse repeated assignments to ONE parameter per name.
///
/// The value comes from the LAST top-level assignment, because that is the
/// write whose value every read sees; the annotations come from the first
/// occurrence that supplied them, because that is where people write them.
/// A name whose last assignment is NOT a literal drops out entirely: a
/// computed value overwrites whatever the panel would set, so the variable is
/// not customizable. Emitting one row per ASSIGNMENT put the same name in the
/// panel twice, sharing a single override slot, and the override was written
/// into the dead first slot while the later assignment still won.
fn one_per_name(found: Vec<(String, Option<Parameter>, bool)>) -> Vec<Parameter> {
    let mut out: Vec<(String, Option<Parameter>)> = Vec::new();
    for (name, param, had_comment) in found {
        match out.iter_mut().find(|(n, _)| *n == name) {
            Some(slot) => {
                let first = slot.0.clone();
                let earlier = slot.1.take();
                slot.1 = param.map(|mut p| {
                    if let Some(a) = earlier {
                        if p.description.is_empty() {
                            p.description = a.description;
                        }
                        if p.group.is_empty() {
                            p.group = a.group;
                        }
                        // Inherit the earlier widget only when this line had
                        // no comment of its own AND the kind still matches.
                        if !had_comment && a.kind == p.kind {
                            p.widget = a.widget;
                        }
                    }
                    p.name = first;
                    p
                });
            }
            None => out.push((name, param)),
        }
    }
    out.into_iter().filter_map(|(_, p)| p).collect()
}

/// Parse `name = <literal>;` plus its trailing `// [widget]` comment. Returns
/// the name even when the RHS is not a literal, so `parse` can tell that the
/// name was reassigned to something computed.
fn parse_assignment(
    code: &str,
    widget_src: &str,
    group: &str,
    desc: &str,
) -> Option<(String, Option<Parameter>, bool)> {
    let eq = code.find('=')?;
    // `==`, `<=`, `>=` and `!=` are comparisons, not assignments.
    if code.as_bytes().get(eq + 1) == Some(&b'=')
        || matches!(code.as_bytes().get(eq.wrapping_sub(1)), Some(b'<') | Some(b'>') | Some(b'!') | Some(b'='))
    {
        return None;
    }
    let name = code[..eq].trim();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') {
        return None;
    }
    let after = &code[eq + 1..];
    let semi = after.find(';')?;
    let rhs = after[..semi].trim();
    let widget_src = widget_src.trim();
    let had_comment = widget_src.starts_with('[');

    let Some((kind, value)) = classify_literal(rhs) else {
        return Some((name.to_string(), None, had_comment));
    };
    let components = if kind == Kind::Vector { vector_len(&value) } else { 1 };
    let widget = widget_for(&kind, widget_src);
    Some((
        name.to_string(),
        Some(Parameter {
            name: name.to_string(),
            group: group.to_string(),
            description: desc.to_string(),
            value,
            kind,
            widget,
            components,
        }),
        had_comment,
    ))
}

/// How many components a vector literal has (it is already known to be a
/// bracketed list of number literals).
fn vector_len(lit: &str) -> usize {
    lit.trim_start_matches('[').trim_end_matches(']').split(',').count()
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

/// A decimal number literal as the LEXER defines one — digits with an optional
/// fraction and exponent — plus an optional leading `-`, since the reference
/// counts `a = -5;` as a literal that gets a widget.
///
/// Emphatically NOT `s.parse::<f64>().is_ok()`: Rust's float parser also
/// accepts "inf", "-inf", "infinity" and "nan", case-insensitively, and this
/// language has no literal for either ("they arise only from arithmetic").
/// The lexer lexes them as ordinary identifiers, so an override of `nan` was
/// written into the source as a bare name and evaluated to undef with an
/// unknown-variable warning, and a source line `x = inf;` was shown in the
/// panel as a number parameter.
fn is_number_literal(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = if b.first() == Some(&b'-') { 1 } else { 0 };
    let mant_start = i;
    while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
        i += 1;
    }
    let mant = &s[mant_start..i];
    // At least one digit, and at most one '.': "1.2.3" lexes as a single token
    // but is not a number.
    if !mant.bytes().any(|c| c.is_ascii_digit()) || mant.bytes().filter(|&c| c == b'.').count() > 1
    {
        return false;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let digits = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == digits {
            return false; // "1e", "1e+"
        }
    }
    // Nothing left over, and the value has to be usable as a number.
    i == b.len() && s.parse::<f64>().is_ok_and(f64::is_finite)
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
        // A vector shares one widget spec across its components; `components`
        // on the Parameter says how many to render.
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
/// Every override's VALUE must be a bare literal, and a value for a name that
/// is a known parameter must also match that parameter's kind. That is what
/// keeps a malformed or hostile value from smuggling statements into the
/// source: the replacement is always a single literal token in RHS position.
///
/// The NAME, though, may be anything. The reference lists `-D '$fn=64'`,
/// `-D '$t=0.5'` and `-D '$preview=false'` as standard automation, and says an
/// override of a name the file never defines simply "creates a new top-level
/// variable". Requiring the name to be in the parameter model silently dropped
/// all of those — a $-special is never a customizer parameter — so `-D $fn=6`
/// left $fn at 0 with no diagnostic anywhere. A name with no declaration line
/// to rewrite is appended instead, which is the reference's own model ("as if
/// assigned at the end of the top-level scope").
///
/// `overrides` is `(name, literal)` where the literal is written as OpenSCAD
/// source (`20`, `true`, `"round"`, `[1, 2, 3]`).
pub fn apply_overrides(source: &str, overrides: &[(String, String)]) -> String {
    if overrides.is_empty() {
        return source.to_string();
    }
    let params = parse(source);
    // Validate each override and reduce to (name -> replacement literal). A
    // later override for the same name wins (matches last-write-wins).
    let mut repl: Vec<(String, String)> = Vec::new();
    for (name, raw) in overrides {
        let name = name.trim();
        if name.is_empty()
            || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$')
            || name.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            continue; // not an identifier: never paste it into the source
        }
        let Some((kind, lit)) = classify_literal(raw.trim()) else { continue };
        // A declared parameter keeps its kind, so the panel round-trip cannot
        // put a string where a slider was. An undeclared name (a $-special,
        // or one the file never assigns) has no kind to match.
        if let Some(p) = params.iter().find(|p| p.name == name) {
            if kind != p.kind {
                continue;
            }
        }
        // A literal is rewritten onto a single line; a raw newline (or CR) would
        // split it and corrupt the source, so reject one outright.
        if lit.contains('\n') || lit.contains('\r') {
            continue;
        }
        match repl.iter_mut().find(|(n, _)| n == name) {
            Some(slot) => slot.1 = lit,
            None => repl.push((name.to_string(), lit)),
        }
    }
    if repl.is_empty() {
        return source.to_string();
    }
    // Find each name's LAST top-level declaration line, using the same
    // lexical scan `parse` uses so a line inside a block comment or nested in
    // an if/for block is never a candidate.
    //
    // The LAST, not the first: `size = 10; size = 20;` binds at the first slot
    // but takes the later write's value, so rewriting the first one left the
    // second still winning and the panel did nothing at all.
    let mut target = vec![usize::MAX; repl.len()];
    {
        let mut in_block = false;
        let mut block_acc: Vec<u8> = Vec::new();
        let mut depth = 0i32;
        for (ln, raw) in source.lines().enumerate() {
            let scan = scan_line(raw, &mut in_block, &mut block_acc);
            let code = scan.code.trim();
            if code.is_empty() {
                continue;
            }
            if depth == 0 && (code.starts_with("module ") || code.starts_with("function ")) {
                break;
            }
            let here = depth;
            depth = (depth + scan.braces).max(0);
            if here != 0 {
                continue;
            }
            if let Some(idx) = repl.iter().position(|(n, _)| line_assigns(code, n)) {
                target[idx] = ln;
            }
        }
    }
    let mut out_lines: Vec<String> = Vec::with_capacity(source.lines().count() + 1);
    for (ln, raw) in source.lines().enumerate() {
        match target.iter().position(|&t| t == ln) {
            Some(idx) => out_lines.push(rewrite_rhs(raw, &repl[idx].1)),
            None => out_lines.push(raw.to_string()),
        }
    }
    let mut out = out_lines.join("\n");
    if source.ends_with('\n') {
        out.push('\n');
    }
    // A name with no declaration line to rewrite — a $-special, or one the
    // file never assigns — is appended instead, which is the reference's own
    // model: "as if assigned at the end of the top-level scope".
    let mut tail = String::new();
    for (i, (n, lit)) in repl.iter().enumerate() {
        if target[i] == usize::MAX {
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

/// Does this (comment-stripped, trimmed) line assign to exactly `name`? True
/// for a computed RHS too: the line still holds that name's value, and it is
/// the one to rewrite.
fn line_assigns(code: &str, name: &str) -> bool {
    parse_assignment(code, "", "", "").is_some_and(|(n, _, _)| n == name)
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
    fn only_top_level_assignments_are_parameters() {
        // The scan had no lexical state at all, so an assignment commented out
        // in a /* */ block became a phantom parameter — and an override was
        // written INTO the comment — and one nested in an if/for block was
        // offered although the reference states flatly that "Parameters inside
        // 'if' blocks or modules are never customizable".
        let names = |src: &str| parse(src).into_iter().map(|p| p.name).collect::<Vec<_>>();
        assert!(names("/*\nold = 10;\n*/\ncube(5);\n").is_empty(), "block comment");
        assert!(names("/* a\n * b\n */\nx = 1;\n") == vec!["x"], "multi-line block then code");
        assert_eq!(
            names("show = true;\nif (show) {\n  // Lid\n  lid_t = 2; // [1:5]\n}\n"),
            vec!["show"],
            "an if body is not top level"
        );
        assert_eq!(
            names("a = 1;\nfor (i = [0:2]) {\n  b = i;\n}\nc = 3;\n"),
            vec!["a", "c"],
            "a for body is not top level, and the scan resumes after it"
        );
        // A brace inside a STRING must not open a block.
        assert_eq!(names("label = \"{\";\ngap = 3; // [1:9]\n"), vec!["label", "gap"]);
        // A `//` inside a string is not a comment either.
        assert_eq!(names("u = \"http://x\";\nv = 2;\n"), vec!["u", "v"]);
        // Sections and Unicode descriptions still work through the new scan.
        let p = parse("/* [Größe] */\n// Höhe über NN\nh = 3; // [0:9]\n");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].group, "Größe");
        assert_eq!(p[0].description, "Höhe über NN");
        assert_eq!(p[0].widget, Widget::Slider { min: 0.0, step: None, max: 9.0 });
    }

    #[test]
    fn a_name_assigned_twice_gets_one_row_bound_to_the_winning_slot() {
        // Two rows shared one override key, and the override was written into
        // the dead FIRST slot while the later assignment still won: the user
        // moved the slider and nothing changed, with no diagnostic.
        let src = "size = 10; // [1:50]\nsize = 20;\necho(size);\n";
        let p = parse(src);
        assert_eq!(p.len(), 1, "one row per NAME: {p:?}");
        assert_eq!(p[0].value, "20", "the winning write supplies the value");
        assert_eq!(
            p[0].widget,
            Widget::Slider { min: 1.0, step: None, max: 50.0 },
            "the annotation is inherited from where it was written"
        );
        // The override must land on the slot that wins.
        let out = apply_overrides(src, &[("size".into(), "42".into())]);
        assert!(out.contains("size = 42;"), "{out:?}");
        assert!(out.contains("size = 10; // [1:50]"), "the annotated line is untouched: {out:?}");

        // A name whose LAST assignment is computed is not customizable: that
        // value overwrites anything the panel could set.
        assert!(parse("w = 5; // [1:9]\nw = h * 2;\n").is_empty());
    }

    #[test]
    fn a_vector_parameter_gets_one_widget_per_component() {
        // widget_for treated Vector exactly like Number, so a vector got a
        // single scalar slider: it displayed NaN's midpoint and every value it
        // produced was rejected downstream as a kind mismatch.
        let p = parse("size = [20, 30]; // [10:100]\n");
        assert_eq!(p[0].kind, Kind::Vector);
        assert_eq!(p[0].components, 2);
        assert_eq!(p[0].widget, Widget::Slider { min: 10.0, step: None, max: 100.0 });
        assert_eq!(parse("d = [1,2,3,4]; // [0:9]\n")[0].components, 4);
        assert_eq!(parse("x = 5;\n")[0].components, 1);
        // A vector value round-trips; a scalar for a vector is still refused.
        let src = "size = [20, 30]; // [10:100]\n";
        assert!(apply_overrides(src, &[("size".into(), "[44, 55]".into())]).contains("[44, 55]"));
        assert_eq!(apply_overrides(src, &[("size".into(), "55".into())]), src);
    }

    #[test]
    fn an_override_may_name_a_special_or_an_undeclared_variable() {
        // Overrides were validated against the parameter MODEL, so a name that
        // is not a customizer parameter was dropped — and a $-special never is
        // one. `-D '$fn=64'`, which the reference lists as standard automation,
        // did nothing at all and said nothing about it.
        let out = apply_overrides("echo($fn);\ncylinder(h = 10, r = 5);\n", &[("$fn".into(), "6".into())]);
        assert!(out.ends_with("$fn = 6;\n"), "appended as a trailing assignment: {out:?}");
        // A name the file never assigns becomes a new top-level variable.
        assert!(apply_overrides("cube(1);\n", &[("nw".into(), "3".into())]).contains("nw = 3;"));
        // A file that DOES assign the special has that line rewritten instead.
        let out = apply_overrides("$fn = 12;\nsphere(4);\n", &[("$fn".into(), "6".into())]);
        assert!(out.starts_with("$fn = 6;"), "{out:?}");
        assert_eq!(out.matches("$fn =").count(), 1, "no duplicate assignment: {out:?}");
        // The VALUE is still strictly a literal, so nothing can be smuggled in.
        for hostile in ["1; cube(9); //", "\"a\"; cube(9); //", "a+b", "f(1)", "[1,2]; x=1; //"] {
            assert_eq!(
                apply_overrides("cube(1);\n", &[("z".into(), hostile.into())]),
                "cube(1);\n",
                "hostile value accepted: {hostile:?}"
            );
        }
        // ...and so is the NAME.
        for bad in ["a; cube(9)", "1abc", "", "a b"] {
            assert_eq!(apply_overrides("cube(1);\n", &[(bad.into(), "3".into())]), "cube(1);\n");
        }
    }

    #[test]
    fn inf_and_nan_are_not_number_literals() {
        // is_number_literal was `s.parse::<f64>().is_ok()`, and Rust's parser
        // accepts inf/nan. This language has no literal for either, so the
        // lexer reads them as identifiers: an override of `nan` was written in
        // as a bare name and evaluated to undef.
        for bad in ["inf", "-inf", "Infinity", "infinity", "nan", "NaN", "1e", "1e+", "1.2.3", "", "-", "."] {
            assert!(!is_number_literal(bad), "{bad:?} must not be a number literal");
            assert_eq!(classify_literal(bad), None, "{bad:?}");
        }
        for good in ["0", "5", "-5", "1.5", ".5", "-.5", "1e-3", "2.5E+6", "123."] {
            assert!(is_number_literal(good), "{good:?} must be a number literal");
        }
        // `x = inf;` gets no widget, and an inf/nan override is dropped.
        assert!(parse("x = inf;\n").is_empty());
        assert_eq!(apply_overrides("x = 1;\n", &[("x".into(), "nan".into())]), "x = 1;\n");
    }

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
            ("ghost".to_string(), "1".to_string()),           // APPENDED: a new variable
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
        // A name the file never assigns is a NEW top-level variable, appended,
        // which is the reference's model for -D. Dropping it silently was the
        // same rule that made `-D '$fn=64'` a no-op.
        assert!(out.contains("ghost = 1;"), "{out}");
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
