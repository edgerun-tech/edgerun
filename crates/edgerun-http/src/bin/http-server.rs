//! Standalone HTTP server binary with optional TLS.
//!
//! Usage:
//!   http-server --port 8080                    # plain HTTP
//!   http-server --port 8443 --tls              # TLS with self-signed cert
//!
//! Then test with:
//!   curl http://127.0.0.1:8080/test
//!   curl -k https://127.0.0.1:8443/test
//!   h2spec -h 127.0.0.1 -p 8443 -k

use std::process;

use edgerun_http::{Handler, HttpServer, Request, Response, StatusCode};
use edgerun_tls::certificate_gen::generate_self_signed;
use std::future::Future;
use std::pin::Pin;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut port = 8080u16;
    let mut tls = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                if i + 1 < args.len() {
                    if let Ok(p) = args[i + 1].parse() { port = p; }
                    i += 1;
                }
            }
            "--tls" => { tls = true; }
            _ => {}
        }
        i += 1;
    }

    let server = HttpServer::new(EchoHandler);
    let server = if tls {
        let cert = generate_self_signed(&["127.0.0.1", "localhost"]);
        server.with_tls(cert)
    } else {
        server
    };

    println!("HTTP server listening on 127.0.0.1:{port}{}", if tls { " (TLS)" } else { "" });

    let rt = edgerun_rt::Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        match server.bind(format!("127.0.0.1:{port}")).await {
            Ok(bound) => {
                if let Err(e) = bound.serve().await {
                    eprintln!("Server error: {e}");
                    process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("Bind error: {e}");
                process::exit(1);
            }
        }
    });
}

struct EchoHandler;

impl Handler for EchoHandler {
    fn handle(&self, req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            Response::text(
                StatusCode::new(200).unwrap(),
                &format!("{} {}", req.method().as_str(), req.uri().request_target()),
            )
        })
    }
}
