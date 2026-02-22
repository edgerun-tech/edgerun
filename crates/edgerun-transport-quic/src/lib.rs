use std::collections::VecDeque;
use std::io::Cursor;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Once;

use async_trait::async_trait;
use bytes::Bytes;
use edgerun_transport_core::{
    MuxedTransportSession, TransportCapabilities, TransportConnector, TransportEndpoint,
    TransportError, TransportKind, TransportStream,
};
use quinn::Connection;
use rustls::RootCertStore;
use url::Url;

#[derive(Debug, Default, Clone)]
pub struct QuicConnector;

struct QuicSession {
    _endpoint: quinn::Endpoint,
    conn: Connection,
    next_stream_id: AtomicU64,
}

struct QuicStream {
    id: u64,
    send: quinn::SendStream,
    recv: quinn::RecvStream,
}

#[async_trait]
impl TransportStream for QuicStream {
    fn id(&self) -> u64 {
        self.id
    }

    async fn send(&mut self, chunk: Bytes) -> Result<(), TransportError> {
        self.send
            .write_all(&chunk)
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))
    }

    async fn recv(&mut self) -> Result<Option<Bytes>, TransportError> {
        let maybe_chunk = self
            .recv
            .read_chunk(64 * 1024, true)
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        Ok(maybe_chunk.map(|c| Bytes::copy_from_slice(&c.bytes)))
    }

    async fn finish(&mut self) -> Result<(), TransportError> {
        self.send
            .finish()
            .map_err(|e| TransportError::Protocol(e.to_string()))
    }
}

#[async_trait]
impl MuxedTransportSession for QuicSession {
    fn kind(&self) -> TransportKind {
        TransportKind::Quic
    }

    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            multiplexed_streams: true,
            reliable_ordered_delivery: true,
            encrypted_channel: true,
        }
    }

    async fn open_stream(&self) -> Result<Box<dyn TransportStream>, TransportError> {
        let id = self.next_stream_id.fetch_add(1, Ordering::Relaxed) + 1;
        let (send, recv) = self
            .conn
            .open_bi()
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        Ok(Box::new(QuicStream { id, send, recv }))
    }

    async fn accept_stream(&self) -> Result<Box<dyn TransportStream>, TransportError> {
        let id = self.next_stream_id.fetch_add(1, Ordering::Relaxed) + 1;
        let (send, recv) = self
            .conn
            .accept_bi()
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        Ok(Box::new(QuicStream { id, send, recv }))
    }

    async fn close(&self) -> Result<(), TransportError> {
        self.conn.close(0_u32.into(), b"closing");
        Ok(())
    }
}

#[async_trait]
impl TransportConnector for QuicConnector {
    fn supports_kind(&self, kind: TransportKind) -> bool {
        kind == TransportKind::Quic
    }

    async fn connect(
        &self,
        endpoint: &TransportEndpoint,
    ) -> Result<Box<dyn MuxedTransportSession>, TransportError> {
        ensure_crypto_provider();
        if endpoint.kind != TransportKind::Quic {
            return Err(TransportError::UnsupportedKind(endpoint.kind));
        }
        let parsed = parse_quic_url(&endpoint.uri)?;
        let remote_addr = resolve_remote_addr(&parsed)?;
        let server_name = parsed
            .host_str()
            .ok_or_else(|| TransportError::InvalidEndpoint("missing host".to_string()))?
            .to_string();

        let local = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0));
        let mut qendpoint =
            quinn::Endpoint::client(local).map_err(|e| TransportError::Protocol(e.to_string()))?;

        let roots = roots_from_endpoint(endpoint)?;
        let crypto = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let qcrypto = quinn::crypto::rustls::QuicClientConfig::try_from(crypto)
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        let client_config = quinn::ClientConfig::new(Arc::new(qcrypto));
        qendpoint.set_default_client_config(client_config);

        let connecting = qendpoint
            .connect(remote_addr, &server_name)
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        let connected = connecting
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;

        Ok(Box::new(QuicSession {
            _endpoint: qendpoint,
            conn: connected,
            next_stream_id: AtomicU64::new(0),
        }))
    }
}

fn ensure_crypto_provider() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn parse_quic_url(uri: &str) -> Result<Url, TransportError> {
    let parsed = Url::parse(uri).map_err(|e| TransportError::InvalidEndpoint(e.to_string()))?;
    if parsed.scheme() != "quic" {
        return Err(TransportError::InvalidEndpoint(format!(
            "expected quic:// URI, got {uri}"
        )));
    }
    Ok(parsed)
}

fn resolve_remote_addr(url: &Url) -> Result<SocketAddr, TransportError> {
    let host = url
        .host_str()
        .ok_or_else(|| TransportError::InvalidEndpoint("missing host".to_string()))?;
    let port = url.port().unwrap_or(4433);
    let addr = format!("{host}:{port}");
    addr.parse::<SocketAddr>()
        .map_err(|e| TransportError::InvalidEndpoint(e.to_string()))
}

fn roots_from_endpoint(endpoint: &TransportEndpoint) -> Result<RootCertStore, TransportError> {
    let mut roots = RootCertStore::empty();
    if let Some(pem_blob) = endpoint.metadata.get("ca_pem") {
        let mut certs = VecDeque::new();
        let mut reader = Cursor::new(pem_blob.as_bytes());
        for item in rustls_pemfile::certs(&mut reader) {
            let cert = item.map_err(|e| TransportError::InvalidEndpoint(e.to_string()))?;
            certs.push_back(cert);
        }
        while let Some(cert) = certs.pop_front() {
            roots
                .add(cert)
                .map_err(|e| TransportError::InvalidEndpoint(e.to_string()))?;
        }
        return Ok(roots);
    }
    if endpoint
        .metadata
        .get("insecure_no_verify")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        return Ok(roots);
    }
    Err(TransportError::InvalidEndpoint(
        "missing ca_pem metadata for QUIC endpoint".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};

    use super::*;
    use base64::Engine;
    use edgerun_transport_core::TransportKind;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

    #[test]
    fn connector_supports_quic() {
        let c = QuicConnector;
        assert!(c.supports_kind(TransportKind::Quic));
        assert!(!c.supports_kind(TransportKind::WebSocket));
    }

    #[tokio::test]
    async fn connect_and_exchange_quic_stream() {
        ensure_crypto_provider();
        let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()])
            .expect("self-signed cert");
        let cert_der_vec = cert.cert.der().to_vec();
        let key_der_vec = cert.key_pair.serialize_der();

        let server_config = quinn::ServerConfig::with_single_cert(
            vec![CertificateDer::from(cert_der_vec.clone())],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der_vec)),
        )
        .expect("server config");
        let bind_addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        let server = quinn::Endpoint::server(server_config, bind_addr).expect("bind quic server");
        let server_addr = server.local_addr().expect("local addr");

        tokio::spawn(async move {
            let incoming = server.accept().await.expect("incoming conn");
            let conn = incoming.await.expect("conn ready");
            let (mut send, mut recv) = conn.accept_bi().await.expect("accept stream");
            let got = recv
                .read_chunk(1024, true)
                .await
                .expect("read chunk")
                .expect("payload");
            assert_eq!(&got.bytes[..], b"ping");
            send.write_all(b"pong").await.expect("write");
            send.finish().expect("finish");
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        });

        let mut endpoint =
            TransportEndpoint::new(TransportKind::Quic, format!("quic://{server_addr}"));
        endpoint
            .metadata
            .insert("ca_pem".to_string(), pem_encode_cert(&cert_der_vec));

        let connector = QuicConnector;
        let session = connector.connect(&endpoint).await.expect("connect");
        let mut stream = session.open_stream().await.expect("open stream");
        stream
            .send(Bytes::from_static(b"ping"))
            .await
            .expect("send");
        stream.finish().await.expect("finish");
        let reply = stream.recv().await.expect("recv").expect("reply");
        assert_eq!(&reply[..], b"pong");
        session.close().await.expect("close");
    }

    fn pem_encode_cert(cert_der: &[u8]) -> String {
        let b64 = base64::engine::general_purpose::STANDARD.encode(cert_der);
        format!(
            "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
            b64
        )
    }
}
