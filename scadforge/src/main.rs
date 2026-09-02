use std::process::exit;

fn main() {
    let mut port: u16 = 4571;
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                port = match args.get(i + 1).and_then(|v| v.parse().ok()) {
                    Some(p) => p,
                    None => {
                        eprintln!("--port requires a number");
                        exit(2);
                    }
                };
                i += 2;
            }
            other => {
                eprintln!("unknown argument '{}'; usage: scadforge [--port N]", other);
                exit(2);
            }
        }
    }
    if let Err(e) = scadforge::http::serve(port) {
        eprintln!("failed to start: {}", e);
        exit(1);
    }
}
