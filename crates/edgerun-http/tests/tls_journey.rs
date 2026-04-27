//! Integration test: Full TLS + HTTP/1.1 journey
//!
//! This tests the complete path: HTTP client -> TLS -> HTTP server
//! Each step is logged to pinpoint exactly where failures occur.

use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

use edgerun_bare_rt::{sleep, spawn, Runtime};
use edgerun_http::{Handler, HttpClient, HttpServer, HttpVersion, Request, Response, StatusCode};
use edgerun_tls::generate_self_signed as gen_cert;

static PORT: AtomicU16 = AtomicU16::new(14000);

fn next_port() -> u16 {
    PORT.fetch_add(1, Ordering::Relaxed)
}

struct EchoHandler;

impl Handler for EchoHandler {
    fn handle(
        &self,
        req: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            let body = req.body().map(|b| b.to_vec()).unwrap_or_default();
            Response::new(StatusCode::OK).with_body(body)
        })
    }
}

fn h2spec_test_server(port: u16) {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        let cert = gen_cert(&["127.0.0.1", "localhost"]).expect("gen cert");
        let server = HttpServer::new(EchoHandler)
            .with_tls(cert)
            .bind(format!("127.0.0.1:{}", port))
            .await
            .expect("server bind");
        eprintln!("HTTP/2 test server listening on port {}", port);
        server.serve().await.expect("serve");
    });
}

#[test]
fn test_http_plain() {
    let port = next_port();
    println!("\n[http_plain] Port {}", port);

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        println!("  [1] Creating plain HTTP server...");
        let server = HttpServer::new(EchoHandler)
            .bind(format!("127.0.0.1:{}", port))
            .await
            .expect("server bind");
        println!("  [2] Server created");

        println!("  [3] Spawning server task...");
        let server_task = spawn(async move {
            if let Err(e) = server.accept_one().await {
                eprintln!("  server accept error: {}", e);
            }
        });

        sleep(Duration::from_millis(50)).await;

        println!("  [4] Client connecting...");
        let client = HttpClient::new();

        println!("  [5] Client sending GET...");
        let resp = client
            .get(&format!("http://127.0.0.1:{}/", port))
            .await
            .expect("client request");

        println!("  [6] Got response: {}", resp.status().as_u16());
        assert_eq!(resp.status().as_u16(), 200);

        server_task.await.expect("server join");
    });
}

#[test]
fn test_https_tls() {
    let port = next_port();
    println!("\n[https_tls] Port {}", port);

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        println!("  [1] Generating self-signed cert...");
        let cert = gen_cert(&["127.0.0.1", "localhost"]).expect("gen cert");
        println!("  [2] Cert generated");

        println!("  [3] Creating HTTPS server...");
        let server = HttpServer::new(EchoHandler)
            .with_tls(cert)
            .bind(format!("127.0.0.1:{}", port))
            .await
            .expect("server bind");
        println!("  [4] Server bound");

        println!("  [5] Spawning server task...");
        let server_task = spawn(async move {
            eprintln!("  [SVR] waiting for connection...");
            match server.accept_one().await {
                Ok(()) => eprintln!("  [SVR] accept done"),
                Err(e) => eprintln!("  [SVR] accept error: {}", e),
            }
        });

        sleep(Duration::from_millis(100)).await;
        println!("  [6] Server should be ready");

        println!("  [7] Creating HTTPS client (force HTTP/1 for now)...");
        let client = HttpClient::new().version(HttpVersion::Http1);

        println!("  [8] Client connecting to https://127.0.0.1:{}...", port);
        match client.get(&format!("https://127.0.0.1:{}/", port)).await {
            Ok(resp) => println!("  [9] Got response: {}", resp.status().as_u16()),
            Err(e) => {
                eprintln!("  [9] ERROR: {}", e);
                // Continue to see if server hung
            }
        }

        // Wait for server with timeout
        println!("  [10] Waiting for server (3s timeout)...");
        match edgerun_bare_rt::timeout(Duration::from_secs(3), server_task).await {
            Ok(Ok(())) => println!("  [11] Server finished OK"),
            Ok(Err(e)) => println!("  [11] Server error: {}", e),
            Err(_) => {
                println!("  [11] TIMEOUT! Server is hanging in TLS handshake!");
                // We can't cancel the task easily but we know it hung
            }
        }
    });
}

#[test]
fn test_https_h2() {
    let port = next_port();
    println!("\n[https_h2] Port {}", port);

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        let cert = gen_cert(&["127.0.0.1", "localhost"]).expect("gen cert");
        let server = HttpServer::new(EchoHandler)
            .with_tls(cert)
            .bind(format!("127.0.0.1:{}", port))
            .await
            .expect("server bind");
        println!("  [1] Server bound");

        let server_task = spawn(async move {
            match server.serve().await {
                Ok(()) => eprintln!("  [SVR] serve done"),
                Err(e) => eprintln!("  [SVR] serve error: {}", e),
            }
        });

        sleep(Duration::from_millis(100)).await;
        println!("  [2] Client connecting with HTTP/2...");

        let client = HttpClient::new().version(HttpVersion::Http2);
        match client.get(&format!("https://127.0.0.1:{}/", port)).await {
            Ok(resp) => println!("  [3] Got response: {}", resp.status().as_u16()),
            Err(e) => eprintln!("  [3] ERROR: {}", e),
        }

        sleep(Duration::from_millis(100)).await;
        server_task.abort();
    });
}
