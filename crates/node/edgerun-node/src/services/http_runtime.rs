use alloc::string::ToString;
use alloc::sync::Arc;

use crate::rt::{
    self, AsyncReadExt, AsyncTcpListener, AsyncTcpStream, AsyncWriteExt, CancellationToken,
};
use crate::transport::{HostSocketTransport, TransportAddress};

#[cfg(target_os = "none")]
use crate::rt::io;
#[cfg(not(target_os = "none"))]
use std::io;

pub struct HttpNodeBinding {
    listener: Arc<AsyncTcpListener>,
    target_app_id: [u8; 32],
}

impl HttpNodeBinding {
    pub fn bind(addr: &str, target_app_id: [u8; 32]) -> io::Result<Self> {
        let listener = HostSocketTransport
            .bind_stream_now(&TransportAddress::host_stream(addr.as_bytes().to_vec()))
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        Ok(Self {
            listener: Arc::new(listener),
            target_app_id,
        })
    }

    pub async fn run(self, shutdown: CancellationToken) -> io::Result<()> {
        while !shutdown.is_cancelled() {
            match rt::timeout(
                core::time::Duration::from_millis(100),
                self.listener.accept(),
            )
            .await
            {
                Ok(Ok((stream, _peer))) => {
                    let target_app_id = self.target_app_id;
                    rt::spawn(async move {
                        if let Err(error) = dispatch_to_app(stream, target_app_id).await {
                            crate::node_warn!("http app dispatch failed: {}", error);
                        }
                    });
                }
                Ok(Err(error)) => return Err(io_error(error)),
                Err(_) => {}
            }
        }
        Ok(())
    }
}

async fn dispatch_to_app(
    mut stream: Arc<AsyncTcpStream>,
    target_app_id: [u8; 32],
) -> io::Result<()> {
    let mut buffer = [0u8; 8192];
    let _ = stream.read(&mut buffer).await.map_err(io_error)?;
    crate::node_info!(
        "http request accepted for app {:02x}{:02x}{:02x}{:02x}",
        target_app_id[0],
        target_app_id[1],
        target_app_id[2],
        target_app_id[3]
    );
    stream
        .write_all(
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 37\r\nConnection: close\r\n\r\napp ipc http dispatch is not wired yet\n",
        )
        .await
        .map_err(io_error)
}

fn io_error(error: rt::IoError) -> io::Error {
    match error {
        rt::IoError::UnexpectedEof => io::Error::new(io::ErrorKind::UnexpectedEof, error),
        rt::IoError::WriteZero => io::Error::new(io::ErrorKind::WriteZero, error),
        rt::IoError::Other(_) => io::Error::new(io::ErrorKind::Other, error),
    }
}
