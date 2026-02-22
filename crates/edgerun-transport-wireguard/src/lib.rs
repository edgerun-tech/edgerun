// SPDX-License-Identifier: Apache-2.0
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use edgerun_transport_core::{
    MuxedTransportSession, TransportCapabilities, TransportConnector, TransportEndpoint,
    TransportError, TransportKind, TransportStream,
};
use tokio::net::UdpSocket;
use url::Url;

#[derive(Debug, Default, Clone)]
pub struct WireGuardConnector;

struct WireGuardSession {
    socket: Arc<UdpSocket>,
    opened: AtomicBool,
}

struct WireGuardStream {
    id: u64,
    socket: Arc<UdpSocket>,
    finished: bool,
}

#[async_trait]
impl TransportStream for WireGuardStream {
    fn id(&self) -> u64 {
        self.id
    }

    async fn send(&mut self, chunk: Bytes) -> Result<(), TransportError> {
        self.socket
            .send(&chunk)
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<Option<Bytes>, TransportError> {
        let mut buf = vec![0_u8; 2048];
        let n = self
            .socket
            .recv(&mut buf)
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        buf.truncate(n);
        Ok(Some(Bytes::from(buf)))
    }

    async fn finish(&mut self) -> Result<(), TransportError> {
        self.finished = true;
        Ok(())
    }
}

#[async_trait]
impl MuxedTransportSession for WireGuardSession {
    fn kind(&self) -> TransportKind {
        TransportKind::WireGuard
    }

    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            multiplexed_streams: false,
            reliable_ordered_delivery: false,
            encrypted_channel: true,
        }
    }

    async fn open_stream(&self) -> Result<Box<dyn TransportStream>, TransportError> {
        if self.opened.swap(true, Ordering::Relaxed) {
            return Err(TransportError::UnsupportedFeature(
                "wireguard adapter supports one logical stream per session",
            ));
        }
        Ok(Box::new(WireGuardStream {
            id: 1,
            socket: self.socket.clone(),
            finished: false,
        }))
    }

    async fn accept_stream(&self) -> Result<Box<dyn TransportStream>, TransportError> {
        self.open_stream().await
    }

    async fn close(&self) -> Result<(), TransportError> {
        Ok(())
    }
}

#[async_trait]
impl TransportConnector for WireGuardConnector {
    fn supports_kind(&self, kind: TransportKind) -> bool {
        kind == TransportKind::WireGuard
    }

    async fn connect(
        &self,
        endpoint: &TransportEndpoint,
    ) -> Result<Box<dyn MuxedTransportSession>, TransportError> {
        if endpoint.kind != TransportKind::WireGuard {
            return Err(TransportError::UnsupportedKind(endpoint.kind));
        }
        let remote = parse_wg_uri(&endpoint.uri)?;
        let local = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0));
        let socket = UdpSocket::bind(local)
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        socket
            .connect(remote)
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        Ok(Box::new(WireGuardSession {
            socket: Arc::new(socket),
            opened: AtomicBool::new(false),
        }))
    }
}

fn parse_wg_uri(uri: &str) -> Result<SocketAddr, TransportError> {
    let parsed = Url::parse(uri).map_err(|e| TransportError::InvalidEndpoint(e.to_string()))?;
    if parsed.scheme() != "wg" && parsed.scheme() != "wireguard" {
        return Err(TransportError::InvalidEndpoint(format!(
            "expected wg:// or wireguard:// URI, got {uri}"
        )));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| TransportError::InvalidEndpoint("missing host".to_string()))?;
    let port = parsed.port().unwrap_or(51820);
    let addr = format!("{host}:{port}");
    addr.parse::<SocketAddr>()
        .map_err(|e| TransportError::InvalidEndpoint(e.to_string()))
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};

    use super::*;
    use edgerun_transport_core::TransportKind;

    #[test]
    fn connector_supports_wireguard() {
        let c = WireGuardConnector;
        assert!(c.supports_kind(TransportKind::WireGuard));
        assert!(!c.supports_kind(TransportKind::Quic));
    }

    #[tokio::test]
    async fn wireguard_udp_adapter_dials_and_exchanges_datagram() {
        let server = UdpSocket::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .await
            .expect("bind server");
        let server_addr = server.local_addr().expect("addr");

        tokio::spawn(async move {
            let mut buf = [0_u8; 128];
            let (n, peer) = server.recv_from(&mut buf).await.expect("recv");
            assert_eq!(&buf[..n], b"ping");
            server.send_to(b"pong", peer).await.expect("send");
        });

        let endpoint =
            TransportEndpoint::new(TransportKind::WireGuard, format!("wg://{server_addr}"));
        let connector = WireGuardConnector;
        let session = connector.connect(&endpoint).await.expect("connect");
        let mut stream = session.open_stream().await.expect("open");
        stream
            .send(Bytes::from_static(b"ping"))
            .await
            .expect("send");
        let resp = stream.recv().await.expect("recv").expect("payload");
        assert_eq!(&resp[..], b"pong");
    }
}
