use std::process::exit;

/// scadforge runs in two modes:
///   * server (default):  `scadforge [--port N]`  — serves the web app.
///   * headless render:   `scadforge -o OUT[.stl|.off|.amf|.svg|.dxf] \
///                           [-D name=value ...] INPUT.scad`
/// The headless mode is the reference CLI's `-o` export with `-D` Customizer
/// overrides, sharing eval::render_export with the web `/export` route.
fn main() {
    // args_os, not args: std::env::args PANICS mid-iteration on an argument
    // that is not valid Unicode, and a Linux filename is an arbitrary byte
    // string — a Latin-1 .scad name aborted the process with a backtrace
    // before any of the diagnostics below could run.
    let args: Vec<String> = match std::env::args_os().map(|a| a.into_string()).collect() {
        Ok(v) => v,
        Err(bad) => {
            eprintln!("argument is not valid UTF-8: {}", bad.to_string_lossy());
            exit(2);
        }
    };
    let mut port: u16 = 4571;
    let mut output: Option<String> = None;
    let mut input: Option<String> = None;
    let mut defines: Vec<(String, String)> = Vec::new();
    let mut preset_file: Option<String> = None;
    let mut preset_name: Option<String> = None;
    let mut camera = scadforge::eval::Camera::DEFAULT;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                port = match args.get(i + 1).and_then(|v| v.parse().ok()) {
                    Some(p) => p,
                    None => {
                        eprintln!("--port requires a number");
                        exit(2);
                    }
                };
                i += 2;
            }
            "-o" | "--output" => {
                match args.get(i + 1) {
                    Some(v) => output = Some(v.clone()),
                    None => {
                        eprintln!("-o requires an output path");
                        exit(2);
                    }
                }
                i += 2;
            }
            "-D" | "--define" => {
                // `-D name=value`: a Customizer override, honored only if `name`
                // is a real parameter and `value` is a literal of its kind.
                match args.get(i + 1).and_then(|v| v.split_once('=')) {
                    Some((n, val)) => defines.push((n.trim().to_string(), val.trim().to_string())),
                    None => {
                        eprintln!("-D requires 'name=value'");
                        exit(2);
                    }
                }
                i += 2;
            }
            "-p" => {
                // `-p FILE.json`: a Customizer preset file (parameter sets).
                match args.get(i + 1) {
                    Some(v) => preset_file = Some(v.clone()),
                    None => {
                        eprintln!("-p requires a preset JSON path");
                        exit(2);
                    }
                }
                i += 2;
            }
            // --camera=tx,ty,tz,rx,ry,rz,dist populates $vpt/$vpr/$vpd, per
            // the reference — which spells it with the `=`. Matching only the
            // bare "--camera" sent the documented form to the
            // unknown-argument arm, so the comment below promised something
            // the code refused. Both spellings now work; the eye/center form
            // is not supported and says so rather than guessing.
            a if a == "--camera" || a.starts_with("--camera=") => {
                let (spec, step) = match a.strip_prefix("--camera=") {
                    Some(v) => (v.to_string(), 1),
                    None => match args.get(i + 1) {
                        Some(v) => (v.clone(), 2),
                        None => {
                            eprintln!("--camera requires tx,ty,tz,rx,ry,rz,dist");
                            exit(2);
                        }
                    },
                };
                match parse_camera(&spec) {
                    Some(c) => camera = c,
                    None => {
                        eprintln!(
                            "--camera expects seven comma-separated numbers \
                             (tx,ty,tz,rx,ry,rz,dist); the eye/center form is \
                             not supported"
                        );
                        exit(2);
                    }
                }
                i += step;
            }
            "-P" => {
                // `-P NAME`: the preset set to select from the `-p` file.
                match args.get(i + 1) {
                    Some(v) => preset_name = Some(v.clone()),
                    None => {
                        eprintln!("-P requires a preset set name");
                        exit(2);
                    }
                }
                i += 2;
            }
            other if !other.starts_with('-') => {
                if input.is_some() {
                    eprintln!("only one input file may be given");
                    exit(2);
                }
                input = Some(other.to_string());
                i += 1;
            }
            other => {
                eprintln!(
                    "unknown argument '{}'; usage: scadforge [--port N] | \
                     scadforge -o OUT [-D name=value ...] \
                     [--camera=tx,ty,tz,rx,ry,rz,dist] INPUT.scad",
                    other
                );
                exit(2);
            }
        }
    }

    // Headless render when an input file (or -o) is present; otherwise serve.
    if input.is_some() || output.is_some() {
        exit(render_headless(input, output, &defines, preset_file, preset_name, camera));
    }

    if let Err(e) = scadforge::http::serve(port) {
        eprintln!("failed to start: {}", e);
        exit(1);
    }
}

/// Run the headless render pipeline; returns a process exit code.
/// `tx,ty,tz,rx,ry,rz,dist` — the rotational `--camera` form.
fn parse_camera(spec: &str) -> Option<scadforge::eval::Camera> {
    let n: Vec<f64> = spec
        .split(',')
        .map(|t| t.trim().parse::<f64>().ok())
        .collect::<Option<Vec<f64>>>()?;
    if n.len() != 7 || !n.iter().all(|v| v.is_finite()) {
        return None;
    }
    Some(scadforge::eval::Camera {
        trans: [n[0], n[1], n[2]],
        rot: [n[3], n[4], n[5]],
        dist: n[6],
        ..scadforge::eval::Camera::DEFAULT
    })
}

fn render_headless(
    input: Option<String>,
    output: Option<String>,
    defines: &[(String, String)],
    preset_file: Option<String>,
    preset_name: Option<String>,
    camera: scadforge::eval::Camera,
) -> i32 {
    let (input, output) = match (input, output) {
        (Some(i), Some(o)) => (i, o),
        _ => {
            eprintln!("headless render needs both an INPUT file and -o OUTPUT");
            return 2;
        }
    };
    // The export format is the output file's extension (the tags
    // eval::export_string understands).
    let ext = output.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let format = match ext.as_str() {
        "stl" | "off" | "amf" | "svg" | "dxf" | "pdf" | "3mf" | "echo" | "csg" => ext,
        // Known 2021.01 debug formats we deliberately do not produce. The
        // reference recommends refusing the CGAL dumps outright rather than
        // emulating kernel internals; `.ast`/`.term` are re-serializations
        // of stages this pipeline does not keep. Name them, so a user does
        // not read "unknown extension" and assume a typo.
        "nef3" | "nefdbg" => {
            eprintln!(
                "'{}' is a CGAL Nef polyhedron dump — kernel internals this \
                 implementation has no equivalent for, and does not emulate. \
                 Use .csg for the evaluated tree.",
                ext
            );
            return 2;
        }
        "ast" | "term" => {
            eprintln!(
                "'{}' export is not implemented. Use .csg for the fully \
                 evaluated instantiation tree.",
                ext
            );
            return 2;
        }
        _ => {
            eprintln!(
                "cannot infer export format from '{}' \
                 (use .stl/.off/.amf/.svg/.dxf/.pdf/.3mf/.echo/.csg)",
                output
            );
            return 2;
        }
    };
    let source = match std::fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read '{}': {}", input, e);
            return 1;
        }
    };
    let base = std::path::Path::new(&input)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| ".".into());

    // Layer the overrides: a `-p`/`-P` preset set first, then `-D` on top (so
    // an explicit `-D` wins under last-write-wins — the reference precedence).
    let mut overrides: Vec<(String, String)> = Vec::new();
    if let Some(pf) = &preset_file {
        let pj = match std::fs::read_to_string(pf) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("cannot read preset file '{}': {}", pf, e);
                return 1;
            }
        };
        let sets = scadforge::customizer::parse_presets(&pj);
        let set = match &preset_name {
            Some(name) => match sets.iter().find(|(n, _)| n == name) {
                Some((_, params)) => params.clone(),
                None => {
                    eprintln!("preset set '{}' not found in '{}'", name, pf);
                    return 1;
                }
            },
            None => {
                eprintln!("-p given without -P: name a preset set to apply");
                return 2;
            }
        };
        overrides.extend(scadforge::customizer::preset_to_overrides(&source, &set));
    }
    overrides.extend_from_slice(defines);

    // The console stream comes back alongside the bytes and goes to stderr:
    // every ECHO, WARNING and DEPRECATED the evaluation produced used to be
    // dropped on the floor for any format but `.echo`, so a headless render
    // reported "wrote part.stl" and nothing about what went wrong in it.
    let (result, console) =
        scadforge::eval::render_export_bytes_reporting(&source, &base, &overrides, &format, camera);
    if !console.is_empty() {
        eprint!("{}", console);
    }
    match result {
        Ok(body) => match std::fs::write(&output, &body) {
            Ok(()) => {
                eprintln!("wrote {}", output);
                0
            }
            Err(e) => {
                eprintln!("cannot write '{}': {}", output, e);
                1
            }
        },
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}
