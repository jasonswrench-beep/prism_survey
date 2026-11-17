//! A local development bridge: `prismsurvey serve` answers the interface's commands over HTTP so the screens can be
//! tried in a browser against the real engine. It listens on 127.0.0.1 only, refuses requests from other web pages,
//! and exists only when built with `--features dev-server`. The desktop app does not use it: it calls the same command
//! layer directly.

use prismsurvey_api::Api;
use serde_json::{json, Value};
use std::io::Read;
use tiny_http::{Header, Method, Request, Response, Server};

fn json_response(status: u16, body: &Value) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut r = Response::from_string(body.to_string()).with_status_code(status);
    r.add_header(
        Header::from_bytes("Content-Type", "application/json; charset=utf-8").expect("header"),
    );
    r.add_header(Header::from_bytes("Cache-Control", "no-store").expect("header"));
    r
}

/// Only pages served from this computer may call the bridge.
fn origin_allowed(request: &Request) -> bool {
    let header = |name: &str| {
        request
            .headers()
            .iter()
            .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
            .map(|h| h.value.as_str().to_string())
    };
    let local = |value: &str| {
        let rest = value.strip_prefix("http://").unwrap_or(value);
        let host = rest
            .split('/')
            .next()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("");
        host == "127.0.0.1" || host == "localhost"
    };
    match header("Origin") {
        Some(origin) => local(&origin),
        None => header("Host").map(|h| local(&h)).unwrap_or(false),
    }
}

pub fn run(port: u16) -> Result<(), String> {
    let server = Server::http(("127.0.0.1", port))
        .map_err(|e| format!("cannot listen on port {port}: {e}"))?;
    let mut api = Api::from_env();
    println!("Prism Survey development bridge listening on http://127.0.0.1:{port}");
    for mut request in server.incoming_requests() {
        let response = if !origin_allowed(&request) {
            json_response(
                403,
                &json!({ "ok": false, "error": { "code": "forbidden", "message": "this bridge only answers pages served from this computer" } }),
            )
        } else if *request.method() == Method::Get && request.url() == "/health" {
            json_response(200, &json!({ "ok": true }))
        } else if *request.method() == Method::Post && request.url() == "/rpc" {
            let mut body = String::new();
            let _ = request
                .as_reader()
                .take(64 * 1024 * 1024)
                .read_to_string(&mut body);
            match serde_json::from_str::<Value>(&body) {
                Ok(v) => {
                    let cmd = v
                        .get("cmd")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let args = v.get("args").cloned().unwrap_or(Value::Null);
                    match api.call(&cmd, args) {
                        Ok(result) => json_response(200, &json!({ "ok": true, "result": result })),
                        Err(e) => json_response(
                            200,
                            &json!({ "ok": false, "error": { "code": e.code, "message": e.message } }),
                        ),
                    }
                }
                Err(e) => json_response(
                    400,
                    &json!({ "ok": false, "error": { "code": "bad_request", "message": e.to_string() } }),
                ),
            }
        } else {
            json_response(
                404,
                &json!({ "ok": false, "error": { "code": "not_found", "message": "unknown address" } }),
            )
        };
        let _ = request.respond(response);
    }
    Ok(())
}
