use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::net::SocketAddr;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::time::Duration;

use edgerun_protocols::proxy::{
    parse_http_proxy_request, parse_socks5_request, socks5_reply, socks5_select_no_auth,
    split_host_port, HttpProxyRequest, Socks5Request, SOCKS5_REP_ADDR_NOT_SUPPORTED,
    SOCKS5_REP_GENERAL_FAILURE, SOCKS5_REP_SUCCESS,
};
use edgerun_rt::{
    copy_bidirectional, spawn, timeout, AsyncReadExt, AsyncTcpListener, AsyncTcpStream,
    AsyncWriteExt, CancellationToken, ConnectFuture,
};

#[derive(Clone)]
pub struct ProxyRuntimeConfig {
    pub bind_addr: String,
    pub socks5_bind_addr: Option<String>,
    pub upstream_proxy: Option<String>,
    pub connect_timeout: Duration,
    pub tunnel_buffer_size: usize,
    pub tunnel_read_timeout: Duration,
    pub tunnel_write_timeout: Duration,
    pub max_connections: Option<usize>,
}

pub struct ProxyRuntime {
    config: ProxyRuntimeConfig,
    active_connections: Arc<AtomicUsize>,
    graceful_shutdown: Arc<AtomicBool>,
}

impl ProxyRuntime {
    pub fn new(config: ProxyRuntimeConfig) -> Self {
        Self {
            config,
            active_connections: Arc::new(AtomicUsize::new(0)),
            graceful_shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn run(&self, shutdown: CancellationToken) -> edgerun_rt::io::Result<()> {
        let http_listener = AsyncTcpListener::bind(&self.config.bind_addr)?;
        let socks5_listener = if let Some(ref addr) = self.config.socks5_bind_addr {
            Some(AsyncTcpListener::bind(addr)?)
        } else {
            None
        };

        let http_handle = spawn({
            let config = self.config.clone();
            let active = Arc::clone(&self.active_connections);
            let graceful = Arc::clone(&self.graceful_shutdown);
            async move {
                accept_loop(http_listener, config, active, graceful, handle_http_proxy).await;
                Ok::<(), edgerun_rt::io::IoError>(())
            }
        });

        let socks5_handle = socks5_listener.map(|listener| {
            let config = self.config.clone();
            let active = Arc::clone(&self.active_connections);
            let graceful = Arc::clone(&self.graceful_shutdown);
            spawn(async move {
                accept_loop(listener, config, active, graceful, handle_socks5).await;
                Ok::<(), edgerun_rt::io::IoError>(())
            })
        });

        shutdown.cancelled().await;
        self.graceful_shutdown.store(true, Ordering::Relaxed);
        while self.active_connections.load(Ordering::Relaxed) > 0 {
            edgerun_rt::sleep(Duration::from_millis(100)).await;
        }

        let _ = http_handle.await;
        if let Some(handle) = socks5_handle {
            let _ = handle.await;
        }
        Ok(())
    }
}

async fn accept_loop<F, Fut>(
    listener: AsyncTcpListener,
    config: ProxyRuntimeConfig,
    active: Arc<AtomicUsize>,
    graceful: Arc<AtomicBool>,
    handler: F,
) where
    F: Fn(Arc<AsyncTcpStream>, ProxyRuntimeConfig) -> Fut + Copy + Send + 'static,
    Fut: core::future::Future<Output = edgerun_rt::io::Result<()>> + Send + 'static,
{
    loop {
        if graceful.load(Ordering::Relaxed) {
            break;
        }
        match listener.accept().await {
            Ok((socket, _addr)) => {
                if let Some(limit) = config.max_connections {
                    if active.load(Ordering::Relaxed) >= limit {
                        continue;
                    }
                }
                active.fetch_add(1, Ordering::Relaxed);
                let config = config.clone();
                let active = Arc::clone(&active);
                spawn(async move {
                    let _ = handler(socket, config).await;
                    active.fetch_sub(1, Ordering::Relaxed);
                    Ok::<(), edgerun_rt::io::IoError>(())
                });
            }
            Err(_) => {}
        }
    }
}

async fn handle_http_proxy(
    mut socket: Arc<AsyncTcpStream>,
    config: ProxyRuntimeConfig,
) -> edgerun_rt::io::Result<()> {
    let mut buffer = vec![0; config.tunnel_buffer_size];
    let n = socket.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }

    match parse_http_proxy_request(&buffer[..n]) {
        Ok(HttpProxyRequest::Connect { target }) => {
            let (host, port) = split_host_port(&target, 443);
            let upstream = connect_numeric(host, port, config.connect_timeout).await?;
            socket
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .await?;
            tunnel(socket, upstream).await
        }
        Ok(HttpProxyRequest::Forward { .. }) if config.upstream_proxy.is_some() => {
            write_response(&mut socket, 501, "Forward proxy chaining not implemented").await
        }
        Ok(HttpProxyRequest::Forward { .. }) => {
            write_response(&mut socket, 400, "Use CONNECT for tunneling").await
        }
        Err(_) => write_response(&mut socket, 400, "Bad Request").await,
    }
}

async fn handle_socks5(
    mut socket: Arc<AsyncTcpStream>,
    config: ProxyRuntimeConfig,
) -> edgerun_rt::io::Result<()> {
    let mut buffer = vec![0; 1024];
    let n = socket.read(&mut buffer).await?;
    let selection = match socks5_select_no_auth(&buffer[..n]) {
        Ok(selection) => selection,
        Err(_) => {
            socket
                .write_all(&[edgerun_protocols::proxy::SOCKS5_VERSION, 0xff])
                .await?;
            return Ok(());
        }
    };
    socket.write_all(&selection).await?;

    let n = socket.read(&mut buffer).await?;
    let Socks5Request::Connect { host, port } = match parse_socks5_request(&buffer[..n]) {
        Ok(request) => request,
        Err(_) => {
            socket
                .write_all(&socks5_reply(SOCKS5_REP_ADDR_NOT_SUPPORTED))
                .await?;
            return Ok(());
        }
    };

    match connect_numeric(&host, port, config.connect_timeout).await {
        Ok(upstream) => {
            socket.write_all(&socks5_reply(SOCKS5_REP_SUCCESS)).await?;
            tunnel(upstream, socket).await
        }
        Err(_) => {
            socket
                .write_all(&socks5_reply(SOCKS5_REP_GENERAL_FAILURE))
                .await?;
            Ok(())
        }
    }
}

async fn connect_numeric(
    host: &str,
    port: u16,
    timeout_after: Duration,
) -> edgerun_rt::io::Result<Arc<AsyncTcpStream>> {
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|_| edgerun_rt::io::IoError::Other("proxy target must be numeric socket addr"))?;
    match timeout(timeout_after, ConnectFuture::new(addr)).await {
        Ok(result) => result,
        Err(_) => Err(edgerun_rt::io::IoError::Other("proxy connect timeout")),
    }
}

async fn tunnel(a: Arc<AsyncTcpStream>, b: Arc<AsyncTcpStream>) -> edgerun_rt::io::Result<()> {
    let mut a = a;
    let mut b = b;
    let _ = copy_bidirectional(&mut a, &mut b).await?;
    Ok(())
}

async fn write_response(
    socket: &mut Arc<AsyncTcpStream>,
    status: u16,
    message: &str,
) -> edgerun_rt::io::Result<()> {
    let status_text = match status {
        400 => "Bad Request",
        501 => "Not Implemented",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        message.len(),
        message
    );
    socket.write_all(response.as_bytes()).await
}
