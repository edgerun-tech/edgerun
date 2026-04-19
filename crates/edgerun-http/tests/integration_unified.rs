//! Integration tests for unified HTTP client with automatic protocol negotiation.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use edgerun_http::{HttpServer, HttpClient, Handler, Request, Response, StatusCode};
use edgerun_rt::{Runtime, spawn, sleep};

static PORT: AtomicU32 = AtomicU32::new(12900);

fn next_port() -> u16 {
    PORT.fetch_add(1, Ordering::Relaxed) as u16
}

fn run<F, Fut>(name: &str, f: F)
where
    F: FnOnce(u16) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
{
    eprintln!("[integration] {}", name);
    let port = next_port();
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move { f(port).await })
        .unwrap_or_else(|e| panic!("[{}] test failed: {}", name, e));
}

struct EchoHandler;

impl Handler for EchoHandler {
    fn handle(&self, req: Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            let body = req.body().map(|b| b.to_vec()).unwrap_or_default();
            Response::new(StatusCode::OK).with_body(body)
        })
    }
}

#[test]
fn test_http1_get() {
    run("http1_get", |port| async move {
        let server = HttpServer::new(EchoHandler)
            .keep_alive(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = spawn(async move {
            server.accept_one().await
        });

        sleep(Duration::from_millis(50)).await;

        let client = HttpClient::new();
        let resp = client.get(&format!("http://127.0.0.1:{}/", port)).await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        assert_eq!(resp.status().as_u16(), 200);

        server_task.await.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        Ok(())
    });
}

#[test]
fn test_http1_post() {
    run("http1_post", |port| async move {
        let server = HttpServer::new(EchoHandler)
            .keep_alive(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = spawn(async move {
            server.accept_one().await
        });

        sleep(Duration::from_millis(50)).await;

        let client = HttpClient::new();
        let resp = client.post(&format!("http://127.0.0.1:{}/", port), b"hello").await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.body_as_string().unwrap(), "hello");

        server_task.await.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        Ok(())
    });
}

#[test]
fn test_http1_delete() {
    run("http1_delete", |port| async move {
        let server = HttpServer::new(EchoHandler)
            .keep_alive(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = spawn(async move {
            server.accept_one().await
        });

        sleep(Duration::from_millis(50)).await;

        let client = HttpClient::new();
        let resp = client.delete(&format!("http://127.0.0.1:{}/", port)).await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        assert_eq!(resp.status().as_u16(), 200);

        server_task.await.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        Ok(())
    });
}