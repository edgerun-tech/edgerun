use alloc::boxed::Box;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use edgerun_rt::AsyncTcpStream;

pub struct ConnectionInterceptorAdapter {
    handler: Arc<dyn edgerun_http::connection_middleware::ConnectionHandler>,
}

impl ConnectionInterceptorAdapter {
    pub fn new(handler: Arc<dyn edgerun_http::connection_middleware::ConnectionHandler>) -> Self {
        Self { handler }
    }
}

impl edgerun_email::server::ConnectionInterceptor for ConnectionInterceptorAdapter {
    fn intercept(
        &self,
        peer: SocketAddr,
        stream: Arc<AsyncTcpStream>,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        let handler = Arc::clone(&self.handler);
        Box::pin(async move { handler.handle(peer, stream).await })
    }
}
