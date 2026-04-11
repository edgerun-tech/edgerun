//! Integration tests for HTTP/1.1 client and server.
//!
//! These tests spin up a real server on a random port and make actual
//! HTTP requests from the client, exercising the full TCP stack,
//! request parsing, response serialization, and keep-alive.

use std::future::poll_fn;
use std::io::{self, Read};
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use edgerun_http::http1::{Server, Client, into_handler, into_handler_async, Request, Response, HttpVersion, ConnectionState, determine_connection};
use edgerun_http::{Method, StatusCode};
use edgerun_rt::{Runtime, AsyncRead, AsyncReadExt, AsyncWriteExt, ConnectFuture};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

static PORT: AtomicU32 = AtomicU32::new(12600);

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

/// Read all available data from an async reader until EOF or buffer full.
async fn read_all<R: AsyncRead + Unpin>(reader: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    let mut total = 0;
    loop {
        let n = poll_fn(|cx| Pin::new(&mut *reader).poll_read(cx, &mut buf[total..])).await?;
        if n == 0 { break; }
        total += n;
        if total >= buf.len() { break; }
    }
    Ok(total)
}

/// Connect with timeout, retrying on connection refused.
async fn connect_with_retry(addr: &str, max_retries: u32) -> io::Result<Arc<edgerun_rt::AsyncTcpStream>> {
    for attempt in 0..max_retries {
        let fut = ConnectFuture::new(addr);
        match edgerun_rt::timeout(Duration::from_secs(3), fut).await {
            Ok(Ok(stream)) => return Ok(stream),
            Ok(Err(e)) => {
                if attempt + 1 < max_retries {
                    edgerun_rt::sleep(Duration::from_millis(50)).await;
                    continue;
                }
                return Err(e);
            }
            Err(_) => {
                if attempt + 1 < max_retries {
                    edgerun_rt::sleep(Duration::from_millis(50)).await;
                    continue;
                }
                return Err(io::Error::new(io::ErrorKind::TimedOut, "connect timed out"));
            }
        }
    }
    Err(io::Error::new(io::ErrorKind::ConnectionRefused, "all retries failed"))
}

// ---------------------------------------------------------------------------
// Basic GET
// ---------------------------------------------------------------------------

#[test]
fn test_basic_get() {
    run("basic_get", |port| async move {
        let handler = into_handler(|_req: Request| {
            let mut resp = Response::new(StatusCode::OK);
            let _ = resp.headers_mut().insert("Content-Type", "text/plain");
            resp.set_body(b"hello world".to_vec());
            resp
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let client = Client::new();
        let resp = client.get(&format!("http://127.0.0.1:{}/", port)).await?;

        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.body_as_string().unwrap(), "hello world");
        assert!(resp.headers().get("content-type").is_some());

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// POST with body echo
// ---------------------------------------------------------------------------

#[test]
fn test_post_echo_body() {
    run("post_echo_body", |port| async move {
        let handler = into_handler(|req: Request| {
            assert_eq!(req.method(), &Method::POST);
            let body = req.body().unwrap_or(&[]);
            let mut resp = Response::new(StatusCode::OK);
            resp.set_body(body.to_vec());
            resp
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let client = Client::new();
        let resp = client.post(&format!("http://127.0.0.1:{}/", port), b"echo me").await?;

        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.body_as_string().unwrap(), "echo me");

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// HEAD request — body must be suppressed
// ---------------------------------------------------------------------------

#[test]
fn test_head_no_body() {
    run("head_no_body", |port| async move {
        let handler = into_handler(|_req: Request| {
            let mut resp = Response::new(StatusCode::OK);
            resp.set_body(b"this body must not arrive".to_vec());
            resp
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let client = Client::new();
        let request = Request::builder()
            .method(Method::HEAD)
            .uri(&format!("http://127.0.0.1:{}/", port))
            .build()?;
        let resp = client.execute(&request).await?;

        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.body(), b"");

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// Custom status codes with proper reason phrases
// ---------------------------------------------------------------------------

#[test]
fn test_status_code_reason_phrases() {
    run("status_code_reason_phrases", |port| async move {
        for code in [404u16, 500, 301, 201, 418, 503] {
            let handler = into_handler(move |_req: Request| {
                Response::new(StatusCode::new(code).unwrap())
            });

            let server = Server::new(handler)
                .keep_alive_timeout(None)
                .bind(format!("127.0.0.1:{}", port))
                .await?;

            let server_task = edgerun_rt::spawn(async move {
                server.accept_one().await
            });

            let client = Client::new();
            let resp = client.get(&format!("http://127.0.0.1:{}/", port)).await?;
            assert_eq!(resp.status().as_u16(), code);

            server_task.await.expect("server task failed")?;

            // Bump port for next iteration
            let _ = PORT.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// Keep-alive: multiple requests on same connection
// ---------------------------------------------------------------------------

#[test]
fn test_keep_alive_multiple_requests() {
    run("keep_alive_multiple_requests", |port| async move {
        let counter = Arc::new(AtomicU32::new(0));

        let handler = {
            let counter = Arc::clone(&counter);
            into_handler(move |_req: Request| {
                let n = counter.fetch_add(1, Ordering::Relaxed) + 1;
                let mut resp = Response::new(StatusCode::OK);
                resp.set_body(format!("request #{}", n).into_bytes());
                resp
            })
        };

        let server = Server::new(handler)
            .keep_alive_timeout(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        // Give server time to start
        edgerun_rt::sleep(Duration::from_millis(50)).await;

        // Connect raw TCP and send multiple requests
        let stream = connect_with_retry(&format!("127.0.0.1:{}", port), 5).await?;
        let (mut read, mut write) = stream.split();

        // Send 3 requests back-to-back
        write.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").await?;
        write.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").await?;
        write.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").await?;

        // Read all responses
        let mut buf = vec![0u8; 4096];
        let mut total_read = 0;
        loop {
            let n = poll_fn(|cx| Pin::new(&mut read).poll_read(cx, &mut buf[total_read..])).await?;
            if n == 0 { break; }
            total_read += n;
            let data = String::from_utf8_lossy(&buf[..total_read]);
            if data.matches("HTTP/1.1 200 OK").count() >= 3 {
                break;
            }
            if total_read >= buf.len() - 100 {
                panic!("buffer overflow, only got {} responses",
                    data.matches("HTTP/1.1 200 OK").count());
            }
        }

        let data = String::from_utf8_lossy(&buf[..total_read]);
        assert!(data.contains("request #1"), "missing request #1 in: {}", data);
        assert!(data.contains("request #2"), "missing request #2 in: {}", data);
        assert!(data.contains("request #3"), "missing request #3 in: {}", data);

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// Malformed request → 400 Bad Request
// ---------------------------------------------------------------------------

#[test]
fn test_malformed_request() {
    run("malformed_request", |port| async move {
        let handler = into_handler(|_req: Request| {
            Response::new(StatusCode::OK)
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let stream = connect_with_retry(&format!("127.0.0.1:{}", port), 5).await?;
        let (mut read, mut write) = stream.split();

        write.write_all(b"GIBBERISH\r\n\r\n").await?;
        drop(write);

        let mut buf = vec![0u8; 1024];
        let n = read_all(&mut read, &mut buf).await?;
        let response = String::from_utf8_lossy(&buf[..n]);
        assert!(response.starts_with("HTTP/1.1 400"),
            "expected 400, got: {}", response);

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// Response with custom headers
// ---------------------------------------------------------------------------

#[test]
fn test_response_headers() {
    run("response_headers", |port| async move {
        let handler = into_handler(|_req: Request| {
            let mut resp = Response::new(StatusCode::OK);
            resp.set_body(b"ok".to_vec());
            let _ = resp.headers_mut().insert("X-Custom-Header", "custom-value");
            let _ = resp.headers_mut().insert("X-Another", "another-value");
            resp
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let client = Client::new();
        let resp = client.get(&format!("http://127.0.0.1:{}/", port)).await?;

        assert_eq!(resp.status().as_u16(), 200);
        let custom = resp.headers().get("x-custom-header")
            .expect("missing X-Custom-Header");
        assert_eq!(custom.as_str(), "custom-value");

        let another = resp.headers().get("x-another")
            .expect("missing X-Another");
        assert_eq!(another.as_str(), "another-value");

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// Large response body
// ---------------------------------------------------------------------------

#[test]
fn test_large_body() {
    run("large_body", |port| async move {
        let big_body = "x".repeat(100_000);
        let handler = into_handler(move |_req: Request| {
            let mut resp = Response::new(StatusCode::OK);
            resp.set_body(big_body.clone().into_bytes());
            resp
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let client = Client::new();
        let resp = client.get(&format!("http://127.0.0.1:{}/", port)).await?;

        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.body().len(), 100_000);
        assert!(resp.body().iter().all(|&b| b == b'x'));

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// Async handler
// ---------------------------------------------------------------------------

#[test]
fn test_async_handler() {
    run("async_handler", |port| async move {
        let handler = into_handler_async(|_req: Request| async {
            edgerun_rt::sleep(Duration::from_millis(10)).await;
            let mut resp = Response::new(StatusCode::OK);
            resp.set_body(b"async response".to_vec());
            resp
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let client = Client::new();
        let resp = client.get(&format!("http://127.0.0.1:{}/", port)).await?;

        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.body_as_string().unwrap(), "async response");

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// Connection: keep-alive header in responses
// ---------------------------------------------------------------------------

#[test]
fn test_keep_alive_header() {
    run("keep_alive_header", |port| async move {
        let handler = into_handler(|_req: Request| {
            Response::new(StatusCode::OK)
        });

        let server = Server::new(handler)
            .keep_alive_timeout(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let stream = connect_with_retry(&format!("127.0.0.1:{}", port), 5).await?;
        let (mut read, mut write) = stream.split();

        write.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").await?;

        let mut buf = vec![0u8; 1024];
        let n = read_all(&mut read, &mut buf).await?;
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.contains("Connection: keep-alive"),
            "expected keep-alive header, got: {}", response);

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// UTF-8 rejection in request line (BufReader fix)
// ---------------------------------------------------------------------------

#[test]
fn test_invalid_utf8_in_request_line() {
    run("invalid_utf8_request_line", |port| async move {
        let handler = into_handler(|_req: Request| {
            Response::new(StatusCode::OK)
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let stream = connect_with_retry(&format!("127.0.0.1:{}", port), 5).await?;
        let (mut read, mut write) = stream.split();

        // Invalid UTF-8: 0x80 is a continuation byte without a start byte
        write.write_all(b"GET /path\x80 HTTP/1.1\r\nHost: localhost\r\n\r\n").await?;
        drop(write);

        let mut buf = vec![0u8; 1024];
        let n = read_all(&mut read, &mut buf).await?;
        let response = String::from_utf8_lossy(&buf[..n]);

        // Should get 400 (Bad Request) because the server rejects invalid UTF-8
        assert!(response.starts_with("HTTP/1.1 400"),
            "expected 400 for invalid UTF-8, got: {}", response);

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// Chunked transfer encoding response
// ---------------------------------------------------------------------------

#[test]
fn test_chunked_response() {
    run("chunked_response", |port| async move {
        // Serve a raw chunked response via a sync TCP listener in a thread
        let chunked_response = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n";

        let listener = std::net::TcpListener::bind(format!("127.0.0.1:{}", port))?;
        let server_task = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).unwrap();
            use std::io::Write;
            let _ = stream.write_all(chunked_response).unwrap();
            let _ = stream.flush().unwrap();
        });

        std::thread::sleep(Duration::from_millis(50));

        let stream = connect_with_retry(&format!("127.0.0.1:{}", port), 5).await?;
        let (mut read, mut write) = stream.split();

        write.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").await?;

        let mut buf = vec![0u8; 1024];
        let n = read_all(&mut read, &mut buf).await?;
        let data = String::from_utf8_lossy(&buf[..n]);

        assert!(data.contains("200 OK"), "expected 200, got: {}", data);
        assert!(data.contains("hello world"), "expected dechunked body, got: {}", data);

        server_task.join().unwrap();
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// POST with JSON body and Content-Type
// ---------------------------------------------------------------------------

#[test]
fn test_post_json() {
    run("post_json", |port| async move {
        let handler = into_handler(|req: Request| {
            assert_eq!(req.method(), &Method::POST);
            let ct = req.headers().get("content-type")
                .expect("missing Content-Type");
            assert!(ct.as_str().contains("application/json"),
                "expected application/json, got: {}", ct.as_str());
            let body = req.body().unwrap_or(&[]);
            assert_eq!(body, b"{\"key\":\"value\"}");
            let mut resp = Response::new(StatusCode::OK);
            resp.set_body(b"{\"ok\":true}".to_vec());
            resp
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let client = Client::new();
        let resp = client.post_json(
            &format!("http://127.0.0.1:{}/", port),
            "{\"key\":\"value\"}",
        ).await?;

        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.body_as_string().unwrap(), "{\"ok\":true}");

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// Content-Length header accuracy
// ---------------------------------------------------------------------------

#[test]
fn test_content_length_accuracy() {
    run("content_length_accuracy", |port| async move {
        let handler = into_handler(|_req: Request| {
            let mut resp = Response::new(StatusCode::OK);
            resp.set_body(b"exact".to_vec());
            resp
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let stream = connect_with_retry(&format!("127.0.0.1:{}", port), 5).await?;
        let (mut read, mut write) = stream.split();

        write.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").await?;

        let mut buf = vec![0u8; 1024];
        let n = read_all(&mut read, &mut buf).await?;
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.contains("Content-Length: 5"),
            "expected Content-Length: 5, got: {}", response);
        assert!(response.ends_with("exact"),
            "expected body 'exact' at end, got: {}", response);

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// PUT method
// ---------------------------------------------------------------------------

#[test]
fn test_put_method() {
    run("put_method", |port| async move {
        let handler = into_handler(|req: Request| {
            assert_eq!(req.method(), &Method::PUT);
            let body = req.body().unwrap_or(&[]);
            let mut resp = Response::new(StatusCode::CREATED);
            resp.set_body(body.to_vec());
            resp
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let client = Client::new();
        let resp = client.put(&format!("http://127.0.0.1:{}/resource", port), b"new data").await?;

        assert_eq!(resp.status().as_u16(), 201);
        assert_eq!(resp.body_as_string().unwrap(), "new data");

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// DELETE method
// ---------------------------------------------------------------------------

#[test]
fn test_delete_method() {
    run("delete_method", |port| async move {
        let handler = into_handler(|req: Request| {
            assert_eq!(req.method(), &Method::DELETE);
            Response::new(StatusCode::NO_CONTENT)
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let client = Client::new();
        let resp = client.delete(&format!("http://127.0.0.1:{}/resource", port)).await?;

        assert_eq!(resp.status().as_u16(), 204);
        assert_eq!(resp.body(), b"");

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// HTTP/1.0 client — server responds with HTTP/1.0 and Connection: close
// ---------------------------------------------------------------------------

#[test]
fn test_http10_server_response() {
    run("http10_server_response", |port| async move {
        let handler = into_handler(|req: Request| {
            assert!(req.version().is_http10());
            let mut resp = Response::new(StatusCode::OK);
            resp.set_body(b"hello from 1.0".to_vec());
            resp
        });

        let server = Server::new(handler)
            .keep_alive_timeout(None)
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let stream = connect_with_retry(&format!("127.0.0.1:{}", port), 5).await?;
        let (mut read, mut write) = stream.split();

        write.write_all(b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n").await?;

        let mut buf = vec![0u8; 1024];
        let n = read_all(&mut read, &mut buf).await?;
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.starts_with("HTTP/1.0 200 OK"), "expected HTTP/1.0, got: {}", response);
        assert!(response.contains("hello from 1.0"), "expected body, got: {}", response);
        assert!(response.contains("Connection: close"), "expected Connection: close, got: {}", response);

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// HTTP/1.0 with explicit Connection: keep-alive
// ---------------------------------------------------------------------------

#[test]
fn test_http10_keep_alive() {
    run("http10_keep_alive", |port| async move {
        let counter = Arc::new(AtomicU32::new(0));

        let handler = {
            let counter = Arc::clone(&counter);
            into_handler(move |req: Request| {
                assert!(req.version().is_http10());
                let n = counter.fetch_add(1, Ordering::Relaxed) + 1;
                let mut resp = Response::new(StatusCode::OK);
                resp.set_body(format!("req #{}", n).into_bytes());
                resp
            })
        };

        let server = Server::new(handler)
            .keep_alive_timeout(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        edgerun_rt::sleep(Duration::from_millis(50)).await;

        let stream = connect_with_retry(&format!("127.0.0.1:{}", port), 5).await?;
        let (mut read, mut write) = stream.split();

        write.write_all(b"GET / HTTP/1.0\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n").await?;
        write.write_all(b"GET / HTTP/1.0\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n").await?;

        let mut buf = vec![0u8; 4096];
        let mut total_read = 0;
        loop {
            let n = poll_fn(|cx| Pin::new(&mut read).poll_read(cx, &mut buf[total_read..])).await?;
            if n == 0 { break; }
            total_read += n;
            let data = String::from_utf8_lossy(&buf[..total_read]);
            if data.matches("HTTP/1.0 200 OK").count() >= 2 {
                break;
            }
            if total_read >= buf.len() - 100 {
                panic!("buffer overflow");
            }
        }

        let data = String::from_utf8_lossy(&buf[..total_read]);
        assert!(data.contains("req #1"), "missing req #1 in: {}", data);
        assert!(data.contains("req #2"), "missing req #2 in: {}", data);

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}

// ---------------------------------------------------------------------------
// Connection: close closes after response
// ---------------------------------------------------------------------------

#[test]
fn test_connection_close() {
    run("connection_close", |port| async move {
        let handler = into_handler(|_req: Request| {
            Response::new(StatusCode::OK)
        });

        let server = Server::new(handler)
            .keep_alive_timeout(Some(Duration::from_secs(5)))
            .bind(format!("127.0.0.1:{}", port))
            .await?;

        let server_task = edgerun_rt::spawn(async move {
            server.accept_one().await
        });

        let stream = connect_with_retry(&format!("127.0.0.1:{}", port), 5).await?;
        let (mut read, mut write) = stream.split();

        write.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").await?;

        let mut buf = vec![0u8; 1024];
        let n = read_all(&mut read, &mut buf).await?;
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Connection: close"));

        drop(write);
        let n = poll_fn(|cx| Pin::new(&mut read).poll_read(cx, &mut [0u8; 1])).await?;
        assert_eq!(n, 0, "expected EOF after Connection: close");

        server_task.await.expect("server task failed")?;
        Ok(())
    });
}
