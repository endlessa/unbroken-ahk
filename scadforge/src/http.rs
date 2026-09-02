//! Minimal hand-rolled HTTP server bound to localhost only.
//!
//! Two routes: GET / serves the embedded single-page app, POST /render
//! takes SCAD source as the request body and returns mesh JSON. Request
//! handling is factored as text-in/text-out so tests exercise it without
//! sockets.

use crate::eval;
use crate::parser;
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
    match (method, path) {
        ("GET", "/") => Response {
            status: "200 OK",
            content_type: "text/html; charset=utf-8",
            body: PAGE.to_string(),
        },
        ("POST", "/render") => Response {
            status: "200 OK",
            content_type: "application/json",
            body: render_json(body),
        },
        _ => Response {
            status: "404 Not Found",
            content_type: "text/plain; charset=utf-8",
            body: "not found".into(),
        },
    }
}

/// Compile + evaluate source into the viewer's mesh JSON:
/// {"meshes": [{"positions": [x,y,z,...], "indices": [...], "color": [r,g,b,a]}],
///  "warnings": [...], "echoes": [...], "error": "..."?}
pub fn render_json(source: &str) -> String {
    let mut pairs: Vec<(&str, JsonValue)> = Vec::new();
    match parser::parse(source) {
        Ok(program) => {
            let out = eval::evaluate(&program);
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
                    let color = s.color.unwrap_or([0.83, 0.71, 0.28, 1.0]); // default gold
                    obj(vec![
                        ("positions", JsonValue::Array(positions)),
                        ("indices", JsonValue::Array(indices)),
                        (
                            "color",
                            JsonValue::Array(color.iter().map(|&c| JsonValue::Number(c)).collect()),
                        ),
                    ])
                })
                .collect();
            pairs.push(("meshes", JsonValue::Array(meshes)));
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
        Err(e) => {
            pairs.push(("meshes", JsonValue::Array(Vec::new())));
            pairs.push(("warnings", JsonValue::Array(Vec::new())));
            pairs.push(("echoes", JsonValue::Array(Vec::new())));
            pairs.push(("error", str_val(&e)));
        }
    }
    to_json_compact(&obj(pairs))
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
        let json = render_json("cube(2);");
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
        let v = parse_json(&render_json("cube(")).unwrap();
        assert!(v.get_str("error").unwrap().contains("expression"));
        assert!(v.get("meshes").unwrap().as_array().unwrap().is_empty());

        let v = parse_json(&render_json("frob(1); cube(1);")).unwrap();
        assert!(v.get("error").is_none());
        assert_eq!(v.get("meshes").unwrap().as_array().unwrap().len(), 1);
        let warnings = v.get("warnings").unwrap().as_array().unwrap();
        assert!(warnings.iter().any(|w| w.as_str().unwrap().contains("frob")));
    }
}
