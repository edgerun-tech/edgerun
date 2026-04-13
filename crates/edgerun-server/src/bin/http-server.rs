//! Standalone HTTP server binary supporting HTTP/1.1, HTTP/2, and HTTP/3.
//!
//! Usage:
//!   http-server --port 8080                       # HTTP/1.1 + HTTP/2
//!   http-server --port 8443 --tls                 # HTTP/1.1 + HTTP/2 + TLS
//!   http-server --port 8443 --tls --http3         # HTTP/1.1 + HTTP/2 + HTTP/3 + TLS
//!
//! Then test with:
//!   curl http://127.0.0.1:8080/test
//!   curl -k https://127.0.0.1:8443/test
//!   h2spec -h 127.0.0.1 -p 8443 -k

use std::process;

use edgerun_http::{Handler, Request, Response, StatusCode};
use edgerun_tls::certificate_gen::generate_self_signed;
use std::future::Future;
use std::pin::Pin;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut port = 8080u16;
    let mut tls = false;
    let mut http3 = false;

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
            "--http3" | "--h3" => { http3 = true; }
            _ => {}
        }
        i += 1;
    }

    let mut server = edgerun_server::Server::new()
        .with_http(EchoHandler, format!("127.0.0.1:{port}"));

    if tls {
        let cert = generate_self_signed(&["127.0.0.1", "localhost"])
            .expect("self-signed cert generation should not fail");
        server = server.with_tls(cert);
    }

    if http3 {
        if !tls {
            eprintln!("HTTP/3 requires TLS (--tls). Enabling TLS automatically.");
            let cert = generate_self_signed(&["127.0.0.1", "localhost"])
                .expect("self-signed cert generation should not fail");
            server = server.with_tls(cert);
        }
        server = server.with_http3();
    }

    let protocols = if http3 { "HTTP/1.1 + HTTP/2 + HTTP/3" } else if tls { "HTTP/1.1 + HTTP/2 + TLS" } else { "HTTP/1.1 + HTTP/2" };
    println!("HTTP server listening on 127.0.0.1:{port} ({protocols})");

    let rt = edgerun_rt::Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        let shutdown = edgerun_rt::CancellationToken::new();
        match server.build().await {
            Ok(mut bound) => {
                if let Err(e) = bound.run(shutdown).await {
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
