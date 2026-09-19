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

/// How long a connection may go without sending anything before it is
/// dropped. Generous for a local viewer typing a request, short enough that
/// a stalled socket cannot hold a thread indefinitely.
const READ_TIMEOUT_SECS: u64 = 30;

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
            body: render_json_with_camera(body, &overrides_from_query(query), camera_from_query(query)),
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
    // `.csg` needs the evaluator to record the instantiation tree as it runs.
    // An export is a RENDER, not a preview: `$preview` must be false so
    // `$fn = $preview ? 24 : 120` gives the fine mesh in the downloaded file.
    let out = eval::evaluate_source_for(&effective, &base, format == "csg", eval::Mode::Render);
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
        "svg" | "dxf" | "pdf" | "off" | "amf" | "stl" | "csg" => format,
        _ => "stl",
    };
    let content_type = match tag {
        "svg" => "image/svg+xml",
        "dxf" => "application/dxf",
        "pdf" => "application/pdf",
        "off" => "text/plain; charset=utf-8",
        "amf" => "application/x-amf",
        "csg" => "text/plain; charset=utf-8",
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
/// The viewport's live camera, as the `$vp*` quartet. The viewer sends where
/// it currently is so a script that READS `$vpr` sees the real camera, and
/// the response reports where the camera should end up so a top-level
/// `$vpr = ...` can move it — the reference's camera-animation idiom.
fn camera_from_query(query: &str) -> eval::Camera {
    let num = |key: &str, d: f64| {
        query_param(query, key).and_then(|v| v.parse::<f64>().ok()).filter(|n| n.is_finite()).unwrap_or(d)
    };
    let d = eval::Camera::DEFAULT;
    eval::Camera {
        rot: [num("vpr0", d.rot[0]), num("vpr1", d.rot[1]), num("vpr2", d.rot[2])],
        trans: [num("vpt0", d.trans[0]), num("vpt1", d.trans[1]), num("vpt2", d.trans[2])],
        dist: num("vpd", d.dist),
        fov: num("vpf", d.fov),
    }
}

pub fn render_json(source: &str, overrides: &[(String, String)]) -> String {
    render_json_with_camera(source, overrides, eval::Camera::DEFAULT)
}

pub fn render_json_with_camera(
    source: &str,
    overrides: &[(String, String)],
    camera: eval::Camera,
) -> String {
    let mut pairs: Vec<(&str, JsonValue)> = Vec::new();
    {
            let base = std::env::current_dir().unwrap_or_else(|_| ".".into());
            let effective = customizer::apply_overrides(source, overrides);
            let out = eval::evaluate_source_with_camera(
                &effective,
                &base,
                false,
                eval::Mode::Preview,
                camera,
            );
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
            // Where the camera should be after the compile. Equal to what the
            // viewer sent unless the script assigned a $vp* at top level.
            let cam = out.camera.unwrap_or(camera);
            pairs.push((
                "camera",
                JsonValue::Array(vec![
                    JsonValue::Number(cam.rot[0]),
                    JsonValue::Number(cam.rot[1]),
                    JsonValue::Number(cam.rot[2]),
                    JsonValue::Number(cam.trans[0]),
                    JsonValue::Number(cam.trans[1]),
                    JsonValue::Number(cam.trans[2]),
                    JsonValue::Number(cam.dist),
                    JsonValue::Number(cam.fov),
                ]),
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
            // A client that connects and then stalls — a crashed viewer, a
            // half-open socket after a network drop, a probe that never
            // sends a byte — must not park this thread forever. Neither
            // read loop in serve_one has any escape other than EOF, so five
            // such connections permanently cost five OS threads.
            let t = Some(std::time::Duration::from_secs(READ_TIMEOUT_SECS));
            let _ = stream.set_read_timeout(t);
            let _ = stream.set_write_timeout(t);
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
    let request_line = head.split("\r\n").next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let header = |name: &str| {
        head.split("\r\n")
            .skip(1)
            .filter_map(|l| l.split_once(':'))
            .find(|(k, _)| k.trim().eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim())
    };
    // A body framed a way we do not implement, or not framed at all, used to
    // be truncated to NOTHING: the script vanished and the response was a
    // successful empty render, indistinguishable from a real empty design.
    // Refuse it instead of lying about it.
    if header("transfer-encoding").is_some() {
        return respond(
            &mut stream,
            "501 Not Implemented",
            "text/plain",
            "transfer encodings are not supported; send a Content-Length",
        );
    }
    let declared = header("content-length").and_then(|v| v.parse::<usize>().ok());
    if method.eq_ignore_ascii_case("POST") && declared.is_none() {
        return respond(
            &mut stream,
            "411 Length Required",
            "text/plain",
            "POST requires a Content-Length header",
        );
    }
    let content_length: usize = declared.unwrap_or(0);
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

    /// Drive a real socket through `serve` on an ephemeral port and return the
    /// raw response. The framing and timeout rules live in `serve_one`, below
    /// `handle`, so `handle` alone cannot reach them.
    fn raw_request(port: u16, raw: &[u8], read_all: bool) -> String {
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
        s.write_all(raw).unwrap();
        let mut out = Vec::new();
        let mut chunk = [0u8; 4096];
        while let Ok(n) = s.read(&mut chunk) {
            if n == 0 {
                break;
            }
            out.extend_from_slice(&chunk[..n]);
            if !read_all && out.len() > 2048 {
                break;
            }
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    fn spawn_server() -> u16 {
        // Bind first to learn a free port, drop it, then let `serve` take it.
        let port = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
        std::thread::spawn(move || {
            let _ = serve(port);
        });
        for _ in 0..200 {
            if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                return port;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        panic!("server never came up on port {port}");
    }

    #[test]
    fn the_viewer_is_wired_to_the_camera_channel() {
        // The server reported Camera::DEFAULT in every response and parsed
        // vpr0..vpf out of every query, but the viewer sent neither and read
        // back neither: the whole $vp* feature was dead in the UI. These
        // pin the two ends together — the viewer's startup view has to BE
        // Camera::DEFAULT, or echo($vpr) lies before the user touches
        // anything.
        let page = handle("GET", "/", "").body;
        let d = eval::Camera::DEFAULT;
        let start = format!("let yaw = (-90 - {}) * D2R, pitch = (90 - {}) * D2R;", d.rot[2], d.rot[0]);
        assert!(page.contains(&start), "viewer startup view drifted from Camera::DEFAULT; want `{start}`");
        let fov = format!("let fov = {} * D2R;", d.fov);
        assert!(page.contains(&fov), "viewer field of view drifted from the $vpf default; want `{fov}`");
        assert!(page.contains("function cameraVp"), "viewer has no camera-to-$vp* mapping");
        for k in ["vpr0", "vpr1", "vpr2", "vpt0", "vpt1", "vpt2", "vpd", "vpf"] {
            assert!(page.contains(k), "viewer never sends {k}, so a script reading it sees the default");
        }
        assert!(page.contains("applyVp(got, sentVp)"), "viewer ignores the camera the server returns");
    }

    #[test]
    fn an_unframed_or_chunked_body_is_refused_not_silently_dropped() {
        // Both used to be truncated to nothing and answered "200 OK" with an
        // empty render — a posted script vanishing without a word.
        let port = spawn_server();
        let chunked = raw_request(
            port,
            b"POST /render HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n9\r\ncube(10);\r\n0\r\n\r\n",
            false,
        );
        assert!(chunked.starts_with("HTTP/1.1 501"), "chunked: {}", &chunked[..40.min(chunked.len())]);
        let unframed = raw_request(port, b"POST /render HTTP/1.1\r\nHost: x\r\n\r\ncube(10);", false);
        assert!(unframed.starts_with("HTTP/1.1 411"), "unframed: {}", &unframed[..40.min(unframed.len())]);

        // A properly framed POST still renders, an explicitly empty one is
        // still a legitimate empty render, and a GET needs no Content-Length.
        let body = "cube(10);";
        let ok = raw_request(
            port,
            format!("POST /render HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{}", body.len(), body)
                .as_bytes(),
            true,
        );
        assert!(ok.starts_with("HTTP/1.1 200"), "framed POST: {}", &ok[..40.min(ok.len())]);
        assert!(ok.contains("\"positions\""), "framed POST must render geometry");
        let empty = raw_request(port, b"POST /render HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n", true);
        assert!(empty.starts_with("HTTP/1.1 200"), "empty POST: {}", &empty[..40.min(empty.len())]);
        let get = raw_request(port, b"GET / HTTP/1.1\r\nHost: x\r\n\r\n", false);
        assert!(get.starts_with("HTTP/1.1 200"), "GET: {}", &get[..40.min(get.len())]);
    }

    #[test]
    fn the_viewport_camera_survives_the_query_round_trip() {
        // The viewer sends the live camera as vpr0..vpf so a script READING
        // $vpr sees where the user orbited to, and the response carries where
        // the camera should end up so a top-level assignment can move it.
        let c = camera_from_query("vpr0=70&vpr1=0&vpr2=15&vpt0=1&vpt1=2&vpt2=3&vpd=250&vpf=45");
        assert_eq!(c.rot, [70.0, 0.0, 15.0]);
        assert_eq!(c.trans, [1.0, 2.0, 3.0]);
        assert_eq!((c.dist, c.fov), (250.0, 45.0));
        // A missing or unparseable key falls back to that component's default,
        // never to NaN.
        let d = camera_from_query("vpr0=nope&vpd=");
        assert_eq!(d.rot, eval::Camera::DEFAULT.rot);
        assert_eq!(d.dist, eval::Camera::DEFAULT.dist);

        let read = render_json_with_camera("echo($vpr); echo($vpf);", &[], c);
        assert!(read.contains("[70, 0, 15]"), "a read must see the live camera: {read}");
        assert!(read.contains("ECHO: 45"), "$vpf too: {read}");
        // A top-level assignment comes back in `camera` so the viewer can move.
        let moved = render_json_with_camera("$vpr = [12, 0, 34]; cube(1);", &[], c);
        let v = parse_json(&moved).unwrap();
        let cam = v.get("camera").unwrap().as_array().unwrap();
        assert_eq!(cam[0].as_f64().unwrap(), 12.0);
        assert_eq!(cam[2].as_f64().unwrap(), 34.0);
        // ...and with no assignment it is exactly what the viewer sent, which
        // is what keeps the viewer from fighting the user's own orbiting.
        let still = parse_json(&render_json_with_camera("cube(1);", &[], c)).unwrap();
        let cam = still.get("camera").unwrap().as_array().unwrap();
        let got: Vec<f64> = cam.iter().map(|n| n.as_f64().unwrap()).collect();
        assert_eq!(got, vec![70.0, 0.0, 15.0, 1.0, 2.0, 3.0, 250.0, 45.0]);
    }

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
