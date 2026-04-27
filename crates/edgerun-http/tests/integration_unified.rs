//! Integration tests for unified HTTP client and server.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use edgerun_bare_rt::{sleep, spawn, Runtime};
use edgerun_http::{Handler, HttpClient, HttpServer, HttpVersion, Request, Response, StatusCode};

static PORT: AtomicU32 = AtomicU32::new(13000);

fn next_port() -> u16 {
    PORT.fetch_add(1, Ordering::Relaxed) as u16
}

fn run<F, Fut>(name: &str, f: F)
where
    F: FnOnce(u16) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
        + Send
        + 'static,
{
    eprintln!("[test] {}", name);
    let port = next_port();
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move { f(port).await })
        .unwrap_or_else(|e| panic!("[{}] test failed: {}", name, e));
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

// ============================================================================
// Server tests - HTTP/1.1 plain
// ============================================================================

#[test]
fn server_http1_get() {
    run("server_http1_get", |port| async move {
        let server = HttpServer::new(EchoHandler)
            .keep_alive(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = spawn(async move { server.accept_one().await });
        sleep(Duration::from_millis(50)).await;

        let client = HttpClient::new();
        let resp = client.get(&format!("http://127.0.0.1:{}/", port)).await?;

        assert_eq!(resp.status().as_u16(), 200);
        server_task.await?;
        Ok(())
    });
}

#[test]
fn server_http1_post() {
    run("server_http1_post", |port| async move {
        let server = HttpServer::new(EchoHandler)
            .keep_alive(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = spawn(async move { server.accept_one().await });
        sleep(Duration::from_millis(50)).await;

        let client = HttpClient::new();
        let resp = client
            .post(&format!("http://127.0.0.1:{}/", port), b"hello")
            .await?;

        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.body_as_string().unwrap(), "hello");
        server_task.await?;
        Ok(())
    });
}

#[test]
fn server_http1_delete() {
    run("server_http1_delete", |port| async move {
        let server = HttpServer::new(EchoHandler)
            .keep_alive(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = spawn(async move { server.accept_one().await });
        sleep(Duration::from_millis(50)).await;

        let client = HttpClient::new();
        let resp = client
            .delete(&format!("http://127.0.0.1:{}/", port))
            .await?;

        assert_eq!(resp.status().as_u16(), 200);
        server_task.await?;
        Ok(())
    });
}

#[test]
fn server_http1_put() {
    run("server_http1_put", |port| async move {
        let server = HttpServer::new(EchoHandler)
            .keep_alive(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = spawn(async move { server.accept_one().await });
        sleep(Duration::from_millis(50)).await;

        let client = HttpClient::new();
        let resp = client
            .put(&format!("http://127.0.0.1:{}/", port), b"data")
            .await?;

        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.body_as_string().unwrap(), "data");
        server_task.await?;
        Ok(())
    });
}

#[test]
fn server_http1_head() {
    run("server_http1_head", |port| async move {
        let server = HttpServer::new(EchoHandler)
            .keep_alive(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = spawn(async move { server.accept_one().await });
        sleep(Duration::from_millis(50)).await;

        let client = HttpClient::new();
        let resp = client.head(&format!("http://127.0.0.1:{}/", port)).await?;

        assert_eq!(resp.status().as_u16(), 200);
        assert!(resp.body().is_empty());
        server_task.await?;
        Ok(())
    });
}

// ============================================================================
// Server with TLS (HTTPS)
// ============================================================================

#[test]
fn server_https_get() {
    use edgerun_tls::generate_self_signed as gen_cert;

    run("server_https_get", |port| async move {
        let cert = gen_cert(&["127.0.0.1", "localhost"]).expect("cert");
        let server = HttpServer::new(EchoHandler)
            .with_tls(cert)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = spawn(async move { server.accept_one().await });
        sleep(Duration::from_millis(100)).await;

        let client = HttpClient::new().version(HttpVersion::Http1);
        let resp = client.get(&format!("https://127.0.0.1:{}/", port)).await?;

        assert_eq!(resp.status().as_u16(), 200);
        server_task.await?;
        Ok(())
    });
}

// ============================================================================
// Client version tests
// ============================================================================

#[test]
fn client_http1_explicit() {
    run("client_http1_explicit", |port| async move {
        let server = HttpServer::new(EchoHandler)
            .keep_alive(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = spawn(async move { server.accept_one().await });
        sleep(Duration::from_millis(50)).await;

        let client = HttpClient::new().version(HttpVersion::Http1);
        let resp = client.get(&format!("http://127.0.0.1:{}/", port)).await?;

        assert_eq!(resp.status().as_u16(), 200);
        server_task.await?;
        Ok(())
    });
}

#[test]
fn client_http2_explicit() {
    use edgerun_tls::generate_self_signed as gen_cert;

    run("client_http2_explicit", |port| async move {
        let cert = gen_cert(&["127.0.0.1", "localhost"]).expect("cert");
        let server = HttpServer::new(EchoHandler)
            .with_tls(cert)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = spawn(async move { server.accept_one().await });
        sleep(Duration::from_millis(100)).await;

        let client = HttpClient::new().version(HttpVersion::Http2);
        let resp = client.get(&format!("https://127.0.0.1:{}/", port)).await?;

        assert_eq!(resp.status().as_u16(), 200);
        server_task.await?;
        Ok(())
    });
}

#[test]
fn client_http3_explicit() {
    use edgerun_tls::generate_self_signed as gen_cert;

    run("client_http3_explicit", |port| async move {
        let cert = gen_cert(&["127.0.0.1", "localhost"]).expect("cert");
        let server = HttpServer::new(EchoHandler)
            .with_tls(cert.clone())
            .with_http3()
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let shutdown = edgerun_bare_rt::CancellationToken::new();
        let shutdown_clone = shutdown.clone();
        let server_task = spawn(async move { server.serve_with_shutdown(shutdown_clone).await });
        sleep(Duration::from_millis(100)).await;

        eprintln!("[test] Starting client connection...");

        let client = HttpClient::new().version(HttpVersion::Http3);
        eprintln!("[test] Client created, making request...");

        let resp = client.get(&format!("https://127.0.0.1:{}/", port)).await?;
        eprintln!("[test] Got response: {}", resp.status());

        assert_eq!(resp.status().as_u16(), 200);
        shutdown.cancel();
        server_task.await?;
        Ok(())
    });
}

#[test]
fn client_best_negotiation() {
    use edgerun_tls::generate_self_signed as gen_cert;

    run("client_best_negotiation", |port| async move {
        let cert = gen_cert(&["127.0.0.1", "localhost"]).expect("cert");
        let server = HttpServer::new(EchoHandler)
            .with_tls(cert.clone())
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let shutdown = edgerun_bare_rt::CancellationToken::new();
        let shutdown_clone = shutdown.clone();
        let server_task = spawn(async move { server.serve_with_shutdown(shutdown_clone).await });
        sleep(Duration::from_millis(100)).await;

        let client = HttpClient::new();
        let resp = client.get(&format!("https://127.0.0.1:{}/", port)).await?;

        assert_eq!(resp.status().as_u16(), 200);
        shutdown.cancel();
        server_task.await?;
        Ok(())
    });
}
