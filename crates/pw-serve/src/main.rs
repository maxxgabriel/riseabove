//! Development server: exposes the same API the desktop shell uses over
//! local HTTP so the interface can be built and tested in a browser.
//!
//! pathway-serve [--port 8787] [--data DIR] [--static DIR]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use pw_view::Api;
use tiny_http::{Header, Method, Request, Response, Server};

fn arg(name: &str) -> Option<String> {
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        if a == name {
            return it.next();
        }
    }
    None
}

fn json_header() -> Header {
    Header::from_bytes("Content-Type", "application/json; charset=utf-8").expect("static header")
}

fn cors() -> Header {
    Header::from_bytes("Access-Control-Allow-Origin", "*").expect("static header")
}

fn content_type(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript",
        Some("css") => "text/css",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("woff2") => "font/woff2",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

fn handle(api: &Api, statics: Option<&Path>, mut req: Request) {
    let url = req.url().split('?').next().unwrap_or("/").to_string();
    if req.method() == &Method::Options {
        let r = Response::empty(204).with_header(cors()).with_header(Header::from_bytes("Access-Control-Allow-Headers", "content-type").expect("static header"));
        let _ = req.respond(r);
        return;
    }
    if let Some(method) = url.strip_prefix("/api/") {
        let mut body = String::new();
        let _ = req.as_reader().read_to_string(&mut body);
        let args: serde_json::Value = if body.trim().is_empty() { serde_json::json!({}) } else { serde_json::from_str(&body).unwrap_or(serde_json::json!({})) };
        let (status, payload) = match api.call(method, args) {
            Ok(v) => (200, v),
            Err(e) => {
                let status = match e.code() {
                    "not_found" => 404,
                    "bad_request" => 400,
                    _ => 409,
                };
                (status, serde_json::json!({"error": {"code": e.code(), "message": e.to_string()}}))
            }
        };
        let r = Response::from_string(payload.to_string()).with_status_code(status).with_header(json_header()).with_header(cors());
        let _ = req.respond(r);
        return;
    }
    if let Some(root) = statics {
        let rel = url.trim_start_matches('/');
        let mut path = root.join(if rel.is_empty() { "index.html" } else { rel });
        if rel.contains("..") || !path.is_file() {
            path = root.join("index.html");
        }
        if let Ok(bytes) = std::fs::read(&path) {
            let r = Response::from_data(bytes).with_header(Header::from_bytes("Content-Type", content_type(&path)).expect("static header"));
            let _ = req.respond(r);
            return;
        }
    }
    let _ = req.respond(Response::from_string("not found").with_status_code(404));
}

fn main() {
    let port: u16 = arg("--port").and_then(|p| p.parse().ok()).unwrap_or(8787);
    let data = PathBuf::from(arg("--data").unwrap_or_else(|| ".pathway-data".into()));
    let statics = arg("--static").map(PathBuf::from);
    let api = Api::new(&data);
    let server = Server::http(("127.0.0.1", port)).unwrap_or_else(|e| {
        eprintln!("cannot listen on port {port}: {e}");
        std::process::exit(1);
    });
    println!("pathway-serve listening on http://127.0.0.1:{port} (data: {})", data.display());
    let server = Arc::new(server);
    loop {
        match server.recv() {
            Ok(req) => {
                let api = api.clone();
                let statics = statics.clone();
                std::thread::spawn(move || handle(&api, statics.as_deref(), req));
            }
            Err(e) => {
                eprintln!("server error: {e}");
                break;
            }
        }
    }
}
