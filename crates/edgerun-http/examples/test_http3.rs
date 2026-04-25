use edgerun_http::{into_handler, HttpClient, HttpServer, HttpVersion, Response, StatusCode};
use edgerun_rt::{sleep, spawn, Runtime};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

static PORT: AtomicU32 = AtomicU32::new(16000);

fn next_port() -> u16 {
    PORT.fetch_add(1, Ordering::Relaxed) as u16
}

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

    rt.block_on(async {
        let port = next_port();

        let cert = edgerun_tls::generate_self_signed(&["127.0.0.1", "localhost"]).expect("cert");

        let handler = into_handler(|_req| Response::text(StatusCode::new(200).unwrap(), "OK"));

        let server = HttpServer::new(handler)
            .with_tls(cert.clone())
            .with_http3()
            .bind(format!("127.0.0.1:{}", port))
            .await
            .expect("bind");

        let shutdown = edgerun_rt::CancellationToken::new();
        let shutdown_clone = shutdown.clone();

        let server_task = spawn(async move { server.serve_with_shutdown(shutdown_clone).await });

        sleep(Duration::from_millis(200)).await;

        let client = HttpClient::new().version(HttpVersion::Http3);

        let resp = client
            .get(&format!("https://127.0.0.1:{}/", port))
            .await
            .expect("request");

        println!("Got response: {}", resp.status());

        if resp.status().as_u16() == 200 {
            println!("=== HTTP/3 TEST PASSED ===");
        } else {
            println!("=== HTTP/3 TEST FAILED ===");
        }

        shutdown.cancel();
        server_task.await.expect("server");
    });
}
