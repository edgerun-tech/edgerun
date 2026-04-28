//! HTTP/3 integration tests.
//!
//! Tests the full HTTP/3 client↔server roundtrip over real UDP sockets.
//! These tests exercise the complete stack: QUIC handshake + TLS 1.3 + HTTP/3 frames + QPACK.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use edgerun_http::{Handler, HttpClient, HttpServer, HttpVersion, Request, Response, StatusCode};
use edgerun_rt::{sleep, spawn, Runtime};
use edgerun_tls::generate_self_signed as gen_cert;

static PORT: AtomicU32 = AtomicU32::new(15000);

fn next_port() -> u16 {
    PORT.fetch_add(1, Ordering::Relaxed) as u16
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

fn with_server<F, Fut>(name: &'static str, f: F)
where
    F: FnOnce(u16) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
        + Send
        + 'static,
{
    let port = next_port();
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        let cert = gen_cert(&["127.0.0.1", "localhost"]).expect("gen cert");
        let server = HttpServer::new(EchoHandler)
            .with_tls(cert.clone())
            .with_http3()
            .bind(format!("127.0.0.1:{}", port))
            .await
            .expect("server bind");

        let shutdown = edgerun_rt::CancellationToken::new();
        let shutdown_clone = shutdown.clone();
        let server_task = spawn(async move {
            if let Err(e) = server.serve_with_shutdown(shutdown_clone).await {
                eprintln!("[{}] server error: {}", name, e);
            }
        });

        sleep(Duration::from_millis(200)).await;

        f(port)
            .await
            .unwrap_or_else(|e| panic!("[{}] test failed: {}", name, e));

        shutdown.cancel();
        let _ = server_task.await;
    });
}

#[test]
fn test_http3_basic_get() {
    with_server("http3_basic_get", |port| async move {
        let client = HttpClient::new().version(HttpVersion::Http3);
        let resp = client.get(&format!("https://127.0.0.1:{}/", port)).await?;
        assert_eq!(resp.status().as_u16(), 200);
        Ok(())
    });
}

#[test]
fn test_http3_echo_post_body() {
    with_server("http3_echo_post", |port| async move {
        let client = HttpClient::new().version(HttpVersion::Http3);
        let resp = client
            .post(&format!("https://127.0.0.1:{}/", port), b"hello http3")
            .await?;
        assert_eq!(resp.status().as_u16(), 200);
        let body = resp.body_as_string().ok_or_else(|| "no body")?;
        assert_eq!(body, "hello http3");
        Ok(())
    });
}

#[test]
fn test_http3_multiple_requests() {
    with_server("http3_multi", |port| async move {
        let client = HttpClient::new().version(HttpVersion::Http3);

        let resp1 = client
            .get(&format!("https://127.0.0.1:{}/path1", port))
            .await?;
        assert_eq!(resp1.status().as_u16(), 200);

        let resp2 = client
            .get(&format!("https://127.0.0.1:{}/path2", port))
            .await?;
        assert_eq!(resp2.status().as_u16(), 200);

        let resp3 = client
            .post(&format!("https://127.0.0.1:{}/path3", port), b"data")
            .await?;
        assert_eq!(resp3.status().as_u16(), 200);

        Ok(())
    });
}

#[test]
fn test_http3_large_body() {
    with_server("http3_large", |port| async move {
        let body = vec![0xAB; 64 * 1024];
        let client = HttpClient::new().version(HttpVersion::Http3);
        let resp = client
            .post(&format!("https://127.0.0.1:{}/", port), body.clone())
            .await?;
        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.body().len(), body.len());
        Ok(())
    });
}
