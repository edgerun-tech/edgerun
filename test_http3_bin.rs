use edgerun_http::{HttpServer, HttpClient, HttpVersion, Handler, Request, Response};
use edgerun_rt::{Runtime, spawn, sleep};
use std::time::Duration;
use std::sync::atomic::{AtomicU32, Ordering};

static PORT: AtomicU32 = AtomicU32::new(14000);

fn next_port() -> u16 {
    PORT.fetch_add(1, Ordering::Relaxed) as u16
}

#[derive(Clone)]
struct EchoHandler;

impl Handler for EchoHandler {
    fn handle(&self, _req: Request) -> Response {
        Response::ok()
    }
}

fn main() {
    println!("=== Starting standalone HTTP/3 test ===");
    
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    
    rt.block_on(async {
        let port = next_port();
        println!("Using port {}", port);
        
        let cert = edgerun_tls::generate_self_signed(&["127.0.0.1", "localhost"]).expect("cert");
        println!("Generated cert");
        
        let server = HttpServer::new(EchoHandler)
            .with_tls(cert.clone())
            .with_http3()
            .bind(format!("127.0.0.1:{}", port))
            .await
            .expect("bind");
        
        println!("Server bound, starting...");
        
        let shutdown = edgerun_rt::CancellationToken::new();
        let shutdown_clone = shutdown.clone();
        
        let server_task = spawn(async move { 
            println!("Server serve starting");
            server.serve_with_shutdown(shutdown_clone).await 
        });
        
        sleep(Duration::from_millis(200)).await;
        println!("Sleep done, creating client...");
        
        let client = HttpClient::new().version(HttpVersion::Http3);
        println!("Client created, making request...");
        
        let resp = client.get(&format!("https://127.0.0.1:{}/", port)).await.expect("request");
        println!("Got response: {}", resp.status());
        
        shutdown.cancel();
        server_task.await.expect("server");
        
        println!("=== Test PASSED ===");
    });
}