use edgerun_http::{Handler, HttpServer, Request, Response, StatusCode};
use edgerun_rt::Runtime;
use edgerun_tls::generate_self_signed as gen_cert;
use std::env;

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

fn main() {
    let port = env::args().nth(1).unwrap_or_else(|| "14443".to_string());
    let port: u16 = port.parse().unwrap_or(14443);

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        let cert = gen_cert(&["127.0.0.1", "localhost"]).expect("gen cert");
        let server = HttpServer::new(EchoHandler)
            .with_tls(cert)
            .bind(format!("127.0.0.1:{}", port))
            .await
            .expect("server bind");
        println!("HTTP/2 TLS server listening on {}", port);
        server.serve().await.expect("serve");
    });
}
