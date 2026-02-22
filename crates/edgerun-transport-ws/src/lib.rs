// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use bytes::Bytes;
use edgerun_transport_core::{
    MuxedTransportSession, TransportCapabilities, TransportConnector, TransportEndpoint,
    TransportError, TransportKind, TransportStream,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message;
use url::Url;

#[derive(Debug, Default, Clone)]
pub struct WebSocketConnector;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsMuxFrame {
    Open { stream_id: u64 },
    Data { stream_id: u64, chunk_b64: String },
    Finish { stream_id: u64 },
}

struct WsMuxSession {
    writer_tx: mpsc::UnboundedSender<WsMuxFrame>,
    inbound_streams: Arc<Mutex<HashMap<u64, mpsc::UnboundedSender<Option<Bytes>>>>>,
    pending_chunks: Arc<Mutex<HashMap<u64, Vec<Bytes>>>>,
    accept_rx: Mutex<mpsc::UnboundedReceiver<u64>>,
    next_stream_id: AtomicU64,
    closed: AtomicBool,
}

struct WsMuxStream {
    id: u64,
    writer_tx: mpsc::UnboundedSender<WsMuxFrame>,
    recv_rx: mpsc::UnboundedReceiver<Option<Bytes>>,
    finished: bool,
}

#[async_trait]
impl TransportStream for WsMuxStream {
    fn id(&self) -> u64 {
        self.id
    }

    async fn send(&mut self, chunk: Bytes) -> Result<(), TransportError> {
        let frame = WsMuxFrame::Data {
            stream_id: self.id,
            chunk_b64: base64::engine::general_purpose::STANDARD.encode(&chunk),
        };
        self.writer_tx
            .send(frame)
            .map_err(|e| TransportError::Protocol(e.to_string()))
    }

    async fn recv(&mut self) -> Result<Option<Bytes>, TransportError> {
        match self.recv_rx.recv().await {
            Some(Some(chunk)) => Ok(Some(chunk)),
            Some(None) | None => Ok(None),
        }
    }

    async fn finish(&mut self) -> Result<(), TransportError> {
        if self.finished {
            return Ok(());
        }
        self.finished = true;
        self.writer_tx
            .send(WsMuxFrame::Finish { stream_id: self.id })
            .map_err(|e| TransportError::Protocol(e.to_string()))
    }
}

#[async_trait]
impl MuxedTransportSession for WsMuxSession {
    fn kind(&self) -> TransportKind {
        TransportKind::WebSocket
    }

    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            multiplexed_streams: true,
            reliable_ordered_delivery: true,
            encrypted_channel: true,
        }
    }

    async fn open_stream(&self) -> Result<Box<dyn TransportStream>, TransportError> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Protocol("session closed".to_string()));
        }
        let stream_id = self.next_stream_id.fetch_add(2, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        self.inbound_streams.lock().await.insert(stream_id, tx);
        self.writer_tx
            .send(WsMuxFrame::Open { stream_id })
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        Ok(Box::new(WsMuxStream {
            id: stream_id,
            writer_tx: self.writer_tx.clone(),
            recv_rx: rx,
            finished: false,
        }))
    }

    async fn accept_stream(&self) -> Result<Box<dyn TransportStream>, TransportError> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransportError::Protocol("session closed".to_string()));
        }
        let mut accept_rx = self.accept_rx.lock().await;
        let stream_id = accept_rx
            .recv()
            .await
            .ok_or_else(|| TransportError::Protocol("session accept loop closed".to_string()))?;
        drop(accept_rx);
        let (tx, rx) = mpsc::unbounded_channel();
        self.inbound_streams
            .lock()
            .await
            .insert(stream_id, tx.clone());
        if let Some(chunks) = self.pending_chunks.lock().await.remove(&stream_id) {
            for chunk in chunks {
                let _ = tx.send(Some(chunk));
            }
        }
        Ok(Box::new(WsMuxStream {
            id: stream_id,
            writer_tx: self.writer_tx.clone(),
            recv_rx: rx,
            finished: false,
        }))
    }

    async fn close(&self) -> Result<(), TransportError> {
        self.closed.store(true, Ordering::Relaxed);
        Ok(())
    }
}

#[async_trait]
impl TransportConnector for WebSocketConnector {
    fn supports_kind(&self, kind: TransportKind) -> bool {
        kind == TransportKind::WebSocket
    }

    async fn connect(
        &self,
        endpoint: &TransportEndpoint,
    ) -> Result<Box<dyn MuxedTransportSession>, TransportError> {
        if endpoint.kind != TransportKind::WebSocket {
            return Err(TransportError::UnsupportedKind(endpoint.kind));
        }
        let parsed = Url::parse(&endpoint.uri)
            .map_err(|e| TransportError::InvalidEndpoint(e.to_string()))?;
        if parsed.scheme() != "ws" && parsed.scheme() != "wss" {
            return Err(TransportError::InvalidEndpoint(format!(
                "expected ws:// or wss:// URI, got {}",
                endpoint.uri
            )));
        }
        let (stream, _resp) = tokio_tungstenite::connect_async(endpoint.uri.as_str())
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        Ok(Box::new(build_session(stream, true)))
    }
}

fn build_session<S>(stream: S, client_role: bool) -> WsMuxSession
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error>
        + Unpin
        + Send
        + 'static,
{
    let (mut sink, mut source) = stream.split();
    let (writer_tx, mut writer_rx) = mpsc::unbounded_channel::<WsMuxFrame>();
    let inbound_streams = Arc::new(Mutex::new(HashMap::<
        u64,
        mpsc::UnboundedSender<Option<Bytes>>,
    >::new()));
    let pending_chunks = Arc::new(Mutex::new(HashMap::<u64, Vec<Bytes>>::new()));
    let inbound_streams_for_reader = inbound_streams.clone();
    let pending_chunks_for_reader = pending_chunks.clone();
    let (accept_tx, accept_rx) = mpsc::unbounded_channel::<u64>();

    tokio::spawn(async move {
        while let Some(frame) = writer_rx.recv().await {
            let encoded = match serde_json::to_string(&frame) {
                Ok(v) => v,
                Err(_) => break,
            };
            if sink.send(Message::Text(encoded.into())).await.is_err() {
                break;
            }
        }
    });

    tokio::spawn(async move {
        while let Some(next) = source.next().await {
            let msg = match next {
                Ok(v) => v,
                Err(_) => break,
            };
            if !msg.is_text() {
                continue;
            }
            let text = match msg.to_text() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let frame = match serde_json::from_str::<WsMuxFrame>(text) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match frame {
                WsMuxFrame::Open { stream_id } => {
                    let _ = accept_tx.send(stream_id);
                }
                WsMuxFrame::Data {
                    stream_id,
                    chunk_b64,
                } => {
                    let chunk = match base64::engine::general_purpose::STANDARD.decode(chunk_b64) {
                        Ok(v) => Bytes::from(v),
                        Err(_) => continue,
                    };
                    let mut guard = inbound_streams_for_reader.lock().await;
                    if let Some(tx) = guard.get_mut(&stream_id) {
                        let _ = tx.send(Some(chunk));
                    } else {
                        drop(guard);
                        pending_chunks_for_reader
                            .lock()
                            .await
                            .entry(stream_id)
                            .or_default()
                            .push(chunk);
                    }
                }
                WsMuxFrame::Finish { stream_id } => {
                    let mut guard = inbound_streams_for_reader.lock().await;
                    if let Some(tx) = guard.remove(&stream_id) {
                        let _ = tx.send(None);
                    }
                }
            }
        }
    });

    WsMuxSession {
        writer_tx,
        inbound_streams,
        pending_chunks,
        accept_rx: Mutex::new(accept_rx),
        next_stream_id: AtomicU64::new(if client_role { 1 } else { 2 }),
        closed: AtomicBool::new(false),
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::*;
    use tokio::net::TcpListener;

    #[test]
    fn connector_supports_ws() {
        let c = WebSocketConnector;
        assert!(c.supports_kind(TransportKind::WebSocket));
        assert!(!c.supports_kind(TransportKind::Quic));
    }

    #[tokio::test]
    async fn ws_connector_dials_and_exchanges_muxed_stream() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");

        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.expect("accept");
            let ws = tokio_tungstenite::accept_async(tcp)
                .await
                .expect("handshake");
            let session = build_session(ws, false);
            let mut incoming = session.accept_stream().await.expect("accept stream");
            let payload = incoming.recv().await.expect("recv").expect("payload");
            assert_eq!(&payload[..], b"hello");
            incoming
                .send(Bytes::from_static(b"world"))
                .await
                .expect("send");
        });

        let endpoint = TransportEndpoint::new(TransportKind::WebSocket, ws_uri(addr));
        let connector = WebSocketConnector;
        let session = connector.connect(&endpoint).await.expect("connect");
        let mut stream = session.open_stream().await.expect("open");
        stream
            .send(Bytes::from_static(b"hello"))
            .await
            .expect("send");
        let reply = stream.recv().await.expect("recv").expect("reply");
        assert_eq!(&reply[..], b"world");
        session.close().await.expect("close");
    }

    fn ws_uri(addr: SocketAddr) -> String {
        format!("ws://{addr}")
    }
}
