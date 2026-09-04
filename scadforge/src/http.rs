//! Minimal hand-rolled HTTP server bound to localhost only.
//!
//! Two routes: GET / serves the embedded single-page app, POST /render
//! takes SCAD source as the request body and returns mesh JSON. Request
//! handling is factored as text-in/text-out so tests exercise it without
//! sockets.

use crate::customizer::{self, Kind, Widget};
use crate::eval;
use std::io::{Read, Write};
use std::net::TcpListener;
use unbroken_test_platform::json::{obj, str_val, to_json_compact, JsonValue};

const PAGE: &str = include_str!("../web/index.html");

/// Cap request bodies (a render request is a script, not a dataset).
const MAX_BODY: usize = 1_000_000;

pub struct Response {
    pub status: &'static str,
    pub content_type: &'static str,
    pub body: String,
}

/// Route one parsed request. Pure: no I/O.
pub fn handle(method: &str, path: &str, body: &str) -> Response {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    match (method, route) {
        ("GET", "/") => Response {
            status: "200 OK",
            content_type: "text/html; charset=utf-8",
            body: PAGE.to_string(),
        },
        ("POST", "/render") => Response {
            status: "200 OK",
            content_type: "application/json",
            body: render_json(body, &overrides_from_query(query)),
        },
        // Export the current source's solid geometry as a downloadable mesh
        // (ASCII STL or OFF). Text formats only over HTTP; binary STL is
        // available programmatically via io::write_stl_binary.
        ("POST", "/export") => export_response(query, body),
        _ => Response {
            status: "404 Not Found",
            content_type: "text/plain; charset=utf-8",
            body: "not found".into(),
        },
    }
}

/// Look up a `key=value` pair in a `&`-separated query string.
fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
}

/// Collect the Customizer overrides from the query string. The UI sends one
/// `p=<urlenc(name=literal)>` param per changed widget (e.g.
/// `p=size%3D42&p=mode%3D%22round%22`); each is percent-decoded and split on
/// the first `=` into (name, literal). The literal is validated downstream by
/// `customizer::apply_overrides`, so a bad pair here is harmless.
fn overrides_from_query(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .filter(|(k, _)| *k == "p")
        .filter_map(|(_, v)| {
            let decoded = percent_decode(v);
            decoded.split_once('=').map(|(n, lit)| (n.trim().to_string(), lit.trim().to_string()))
        })
        .collect()
}

/// Decode `application/x-www-form-urlencoded` text: `+` → space and `%XX` →
/// the byte. Invalid escapes are passed through literally (lenient, since a
/// malformed override is dropped later, never executed).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                match (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    (Some(hi), Some(lo)) => {
                        out.push((hi << 4) | lo);
                        i += 3;
                    }
                    _ => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn export_response(query: &str, source: &str) -> Response {
    let format = query_param(query, "format").unwrap_or("stl");
    let base = std::env::current_dir().unwrap_or_else(|_| ".".into());
    // Export the design as the Customizer currently has it: the same override
    // injection the /render path uses, so a downloaded mesh matches the preview.
    let effective = customizer::apply_overrides(source, &overrides_from_query(query));
    let out = eval::evaluate_source(&effective, &base);
    // `.echo` returns the console stream (ECHO + diagnostics), and captures it
    // even when a fatal error halted evaluation — so it is handled before the
    // error check that the geometry exports use.
    if format == "echo" {
        return Response {
            status: "200 OK",
            content_type: "text/plain; charset=utf-8",
            body: eval::echo_stream(&out),
        };
    }
    if let Some(err) = &out.error {
        return Response {
            status: "422 Unprocessable Entity",
            content_type: "text/plain; charset=utf-8",
            body: err.clone(),
        };
    }
    // 3MF is binary (ZIP); the String-bodied HTTP response can't carry it, so
    // it is a CLI-only export. Say so rather than silently returning STL.
    if format == "3mf" {
        return Response {
            status: "415 Unsupported Media Type",
            content_type: "text/plain; charset=utf-8",
            body: "3MF is a binary format; export it with the CLI: \
                   scadforge -o model.3mf input.scad"
                .into(),
        };
    }
    // Normalize the tag (default STL) so it matches eval::export_string, then
    // pair each format with its MIME type. 2D vector formats export the 2D
    // outlines; mesh formats export the solid geometry.
    let tag = match format {
        "svg" | "dxf" | "pdf" | "off" | "amf" | "stl" => format,
        _ => "stl",
    };
    let content_type = match tag {
        "svg" => "image/svg+xml",
        "dxf" => "application/dxf",
        "pdf" => "application/pdf",
        "off" => "text/plain; charset=utf-8",
        "amf" => "application/x-amf",
        _ => "model/stl",
    };
    match eval::export_string(&out, tag) {
        Ok(body) => Response { status: "200 OK", content_type, body },
        Err(e) => Response {
            status: "422 Unprocessable Entity",
            content_type: "text/plain; charset=utf-8",
            body: e,
        },
    }
}

/// Compile + evaluate source into the viewer's mesh JSON:
/// {"meshes": [{"positions": [x,y,z,...], "indices": [...], "color": [r,g,b,a]}],
///  "parameters": [...], "warnings": [...], "echoes": [...], "error": "..."?}
///
/// `overrides` are the Customizer's current widget values `(name, literal)`;
/// they are injected as trailing assignments (last-write-wins) before eval.
/// The `parameters` model is parsed from the ORIGINAL source, so the panel
/// keeps showing the declared widgets and their defaults while the preview
/// reflects the overridden values.
pub fn render_json(source: &str, overrides: &[(String, String)]) -> String {
    let mut pairs: Vec<(&str, JsonValue)> = Vec::new();
    {
            let base = std::env::current_dir().unwrap_or_else(|_| ".".into());
            let effective = customizer::apply_overrides(source, overrides);
            let out = eval::evaluate_source(&effective, &base);
            let meshes: Vec<JsonValue> = out
                .shapes
                .iter()
                .map(|s| {
                    let positions: Vec<JsonValue> = s
                        .mesh
                        .positions
                        .iter()
                        .flat_map(|p| p.iter().map(|&c| JsonValue::Number(c)))
                        .collect();
                    let indices: Vec<JsonValue> = s
                        .mesh
                        .tris
                        .iter()
                        .flat_map(|t| t.iter().map(|&i| JsonValue::Number(i as f64)))
                        .collect();
                    // Uncolored geometry defaults to gold; uncolored `%`
                    // background ghosts default to gray (an explicit color()
                    // inside a `%` subtree is preserved and tints the ghost).
                    let default = if s.background {
                        [0.6, 0.6, 0.6, 1.0]
                    } else {
                        [0.83, 0.71, 0.28, 1.0]
                    };
                    let color = s.color.unwrap_or(default);
                    obj(vec![
                        ("positions", JsonValue::Array(positions)),
                        ("indices", JsonValue::Array(indices)),
                        (
                            "color",
                            JsonValue::Array(color.iter().map(|&c| JsonValue::Number(c)).collect()),
                        ),
                        // Modifier-character display state: `#` draws tinted,
                        // `%` draws as a translucent ghost (background).
                        ("highlight", JsonValue::Bool(s.highlight)),
                        ("background", JsonValue::Bool(s.background)),
                    ])
                })
                .collect();
            pairs.push(("meshes", JsonValue::Array(meshes)));
            pairs.push(("parameters", parameters_json(source)));
            pairs.push((
                "warnings",
                JsonValue::Array(out.warnings.iter().map(|w| str_val(w)).collect()),
            ));
            pairs.push((
                "echoes",
                JsonValue::Array(out.echoes.iter().map(|e| str_val(e)).collect()),
            ));
            if let Some(err) = &out.error {
                pairs.push(("error", str_val(err)));
            }
    }
    to_json_compact(&obj(pairs))
}

/// Serialize the Customizer parameter model for the UI panel. Each entry:
/// {"name","group","description","value","kind","widget":{...}} where the
/// widget object carries its type-specific fields (slider bounds, dropdown
/// options). `group` drives the tabbed layout ("" default, "Hidden" hidden,
/// "Global" everywhere).
fn parameters_json(source: &str) -> JsonValue {
    let params = customizer::parse(source);
    JsonValue::Array(
        params
            .iter()
            .map(|p| {
                let kind = match p.kind {
                    Kind::Number => "number",
                    Kind::Bool => "bool",
                    Kind::String => "string",
                    Kind::Vector => "vector",
                };
                obj(vec![
                    ("name", str_val(&p.name)),
                    ("group", str_val(&p.group)),
                    ("description", str_val(&p.description)),
                    ("value", str_val(&p.value)),
                    ("kind", str_val(kind)),
                    ("widget", widget_json(&p.widget)),
                ])
            })
            .collect(),
    )
}

fn widget_json(w: &Widget) -> JsonValue {
    match w {
        Widget::Spinbox => obj(vec![("type", str_val("spinbox"))]),
        Widget::Checkbox => obj(vec![("type", str_val("checkbox"))]),
        Widget::Textbox => obj(vec![("type", str_val("textbox"))]),
        Widget::Slider { min, step, max } => {
            let mut fields = vec![
                ("type", str_val("slider")),
                ("min", JsonValue::Number(*min)),
                ("max", JsonValue::Number(*max)),
            ];
            fields.push(("step", match step {
                Some(s) => JsonValue::Number(*s),
                None => JsonValue::Null,
            }));
            obj(fields)
        }
        Widget::Dropdown(items) => obj(vec![
            ("type", str_val("dropdown")),
            (
                "options",
                JsonValue::Array(
                    items
                        .iter()
                        .map(|(v, l)| obj(vec![("value", str_val(v)), ("label", str_val(l))]))
                        .collect(),
                ),
            ),
        ]),
    }
}

/// Blocking accept loop on 127.0.0.1 — localhost only, by design.
pub fn serve(port: u16) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    eprintln!("scadforge listening on http://127.0.0.1:{}/", port);
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        std::thread::spawn(move || {
            let _ = serve_one(stream);
        });
    }
    Ok(())
}

fn serve_one(mut stream: std::net::TcpStream) -> std::io::Result<()> {
    // Read headers.
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_header_end(&buf) {
            break pos;
        }
        if buf.len() > 64 * 1024 {
            return respond(&mut stream, "431 Request Header Fields Too Large", "text/plain", "");
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let content_length: usize = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse().ok())
        .unwrap_or(0);
    if content_length > MAX_BODY {
        return respond(&mut stream, "413 Payload Too Large", "text/plain", "body too large");
    }
    let mut body = buf[header_end + 4..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(content_length);
    let body = String::from_utf8_lossy(&body).to_string();

    let resp = handle(&method, &path, &body);
    respond(&mut stream, resp.status, resp.content_type, &resp.body)
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn respond(
    stream: &mut std::net::TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        content_type,
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use unbroken_test_platform::json::parse_json;

    #[test]
    fn index_serves_the_app_and_unknown_paths_404() {
        let r = handle("GET", "/", "");
        assert_eq!(r.status, "200 OK");
        assert!(r.body.contains("<canvas"), "page must embed the viewport");
        assert_eq!(handle("GET", "/nope", "").status, "404 Not Found");
        assert_eq!(handle("DELETE", "/render", "").status, "404 Not Found");
    }

    #[test]
    fn render_returns_meshes_for_valid_source() {
        let json = render_json("cube(2);", &[]);
        let v = parse_json(&json).unwrap();
        let meshes = v.get("meshes").unwrap().as_array().unwrap();
        assert_eq!(meshes.len(), 1);
        // 8 vertices * 3 components, 12 triangles * 3 indices.
        assert_eq!(meshes[0].get("positions").unwrap().as_array().unwrap().len(), 24);
        assert_eq!(meshes[0].get("indices").unwrap().as_array().unwrap().len(), 36);
        assert!(v.get("error").is_none());
    }

    #[test]
    fn render_reports_parse_errors_and_warnings() {
        let v = parse_json(&render_json("cube(", &[])).unwrap();
        assert!(v.get_str("error").unwrap().contains("expression"));
        assert!(v.get("meshes").unwrap().as_array().unwrap().is_empty());

        let v = parse_json(&render_json("frob(1); cube(1);", &[])).unwrap();
        assert!(v.get("error").is_none());
        assert_eq!(v.get("meshes").unwrap().as_array().unwrap().len(), 1);
        let warnings = v.get("warnings").unwrap().as_array().unwrap();
        assert!(warnings.iter().any(|w| w.as_str().unwrap().contains("frob")));
    }

    #[test]
    fn render_exposes_parameter_model_and_applies_overrides() {
        // A cube whose size is a customizer parameter with a slider.
        let src = "// Cube edge\nsize = 2; // [1:10]\ncube(size);";
        let v = parse_json(&render_json(src, &[])).unwrap();
        let params = v.get("parameters").unwrap().as_array().unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].get_str("name").unwrap(), "size");
        assert_eq!(params[0].get_str("description").unwrap(), "Cube edge");
        assert_eq!(params[0].get("widget").unwrap().get_str("type").unwrap(), "slider");
        // Default render: an edge-2 cube → 8 verts.
        let n_default =
            v.get("meshes").unwrap().as_array().unwrap()[0].get("positions").unwrap().as_array().unwrap().len();
        assert_eq!(n_default, 24);

        // Override the size to 6; the mesh must reflect the bigger cube, and the
        // parameter model still reports the DECLARED default (2), not 6.
        let ov = vec![("size".to_string(), "6".to_string())];
        let v = parse_json(&render_json(src, &ov)).unwrap();
        assert_eq!(v.get("parameters").unwrap().as_array().unwrap()[0].get_str("value").unwrap(), "2");
        let m = v.get("meshes").unwrap().as_array().unwrap()[0].get("positions").unwrap().as_array().unwrap();
        // Max coordinate is now 6, proving the override took effect.
        let maxc = m.iter().map(|c| c.as_f64().unwrap()).fold(0.0_f64, f64::max);
        assert_eq!(maxc, 6.0);
    }

    #[test]
    fn override_query_parsing_decodes_and_applies() {
        // The transport path: overrides ride in the query string as
        // `p=<urlenc(name=literal)>`, percent-decoded before injection.
        let src = "n = 1; // [0:10]\nm = \"a\";\ncube([n, 1, 1]);";
        let query = "p=n%3D7&p=m%3D%22b%22";
        let r = handle("POST", &format!("/render?{}", query), src);
        let v = parse_json(&r.body).unwrap();
        let m = v.get("meshes").unwrap().as_array().unwrap()[0].get("positions").unwrap().as_array().unwrap();
        let maxc = m.iter().map(|c| c.as_f64().unwrap()).fold(0.0_f64, f64::max);
        assert_eq!(maxc, 7.0);
    }

    #[test]
    fn percent_decode_handles_escapes_and_plus() {
        assert_eq!(percent_decode("a%3Db"), "a=b");
        assert_eq!(percent_decode("%22round%22"), "\"round\"");
        assert_eq!(percent_decode("one+two"), "one two");
        assert_eq!(percent_decode("%5B1%2C2%5D"), "[1,2]");
        // A dangling/invalid escape passes through literally.
        assert_eq!(percent_decode("50%"), "50%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }
}
