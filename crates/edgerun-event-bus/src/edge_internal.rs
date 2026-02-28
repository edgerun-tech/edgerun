// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_stream::stream;
use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;
use tokio::sync::broadcast;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::{Request, Response, Status};
use tower::service_fn;

use crate::proto::edge_internal_event_bus_client::EdgeInternalEventBusClient as GrpcClient;
use crate::proto::edge_internal_event_bus_server::{
    EdgeInternalEventBus as GrpcEdgeInternalEventBus, EdgeInternalEventBusServer,
};
use crate::proto::{
    EventEnvelope as ProtoEventEnvelope, EventTopic as ProtoEventTopic, Overlay as ProtoOverlay,
    PublishRequest, PublishResponse, SubscribeRequest,
};
use crate::{EventBusError, EventEnvelope, EventTopic, OverlayNetwork, RuntimeEventBus};

#[derive(Debug, Clone)]
pub struct EdgeInternalBrokerHandle {
    pub socket_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct EdgeInternalClient {
    client: Arc<tokio::sync::Mutex<GrpcClient<Channel>>>,
    subscriptions: Arc<Mutex<HashMap<EventTopic, broadcast::Sender<EventEnvelope>>>>,
}

#[derive(Clone)]
struct EdgeInternalGrpcService {
    bus: RuntimeEventBus,
}

type SubscribeStream =
    std::pin::Pin<Box<dyn futures_core::Stream<Item = Result<ProtoEventEnvelope, Status>> + Send>>;

#[tonic::async_trait]
impl GrpcEdgeInternalEventBus for EdgeInternalGrpcService {
    async fn publish(
        &self,
        request: Request<PublishRequest>,
    ) -> Result<Response<PublishResponse>, Status> {
        let req = request.into_inner();
        let topic = req
            .topic
            .ok_or_else(|| Status::invalid_argument("missing topic"))?;
        let event_topic = proto_topic_to_runtime(&topic)
            .map_err(|err| Status::invalid_argument(err.to_string()))?;
        let envelope = self
            .bus
            .publish(&event_topic, req.publisher, req.payload_type, req.payload)
            .map_err(|err| Status::internal(err.to_string()))?;
        Ok(Response::new(PublishResponse {
            accepted: true,
            event_id: envelope.event_id,
        }))
    }

    type SubscribeStream = SubscribeStream;

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let req = request.into_inner();
        let topic = req
            .topic
            .ok_or_else(|| Status::invalid_argument("missing topic"))?;
        let event_topic = proto_topic_to_runtime(&topic)
            .map_err(|err| Status::invalid_argument(err.to_string()))?;
        let mut rx = self
            .bus
            .subscribe(&event_topic)
            .map_err(|err| Status::not_found(err.to_string()))?;
        let out = stream! {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        yield Ok(runtime_event_to_proto(event));
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        };
        Ok(Response::new(Box::pin(out) as Self::SubscribeStream))
    }
}

pub async fn spawn_edge_internal_broker(
    socket_path: &Path,
    bus: RuntimeEventBus,
) -> Result<EdgeInternalBrokerHandle, EventBusError> {
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| EventBusError::PublishFailed(format!("create socket dir: {e}")))?;
    }
    if socket_path.exists() {
        tokio::fs::remove_file(socket_path)
            .await
            .map_err(|e| EventBusError::PublishFailed(format!("remove stale socket: {e}")))?;
    }
    let uds = tokio::net::UnixListener::bind(socket_path)
        .map_err(|e| EventBusError::PublishFailed(format!("bind edge-internal socket: {e}")))?;
    let incoming = UnixListenerStream::new(uds);
    let service = EdgeInternalGrpcService { bus };
    tokio::spawn(async move {
        let _ = Server::builder()
            .add_service(EdgeInternalEventBusServer::new(service))
            .serve_with_incoming(incoming)
            .await;
    });
    Ok(EdgeInternalBrokerHandle {
        socket_path: socket_path.to_path_buf(),
    })
}

impl EdgeInternalClient {
    pub async fn connect(socket_path: &Path) -> Result<Self, EventBusError> {
        let path = socket_path.to_path_buf();
        let channel = Endpoint::try_from("http://[::]:50051")
            .map_err(|e| EventBusError::PublishFailed(format!("endpoint init: {e}")))?
            .connect_with_connector(service_fn(move |_| {
                let path = path.clone();
                async move {
                    let stream = UnixStream::connect(path).await?;
                    Ok::<_, io::Error>(TokioIo::new(stream))
                }
            }))
            .await
            .map_err(|e| EventBusError::PublishFailed(format!("connect edge-internal: {e}")))?;

        Ok(Self {
            client: Arc::new(tokio::sync::Mutex::new(GrpcClient::new(channel))),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn subscribe(
        &self,
        topic: &EventTopic,
    ) -> Result<broadcast::Receiver<EventEnvelope>, EventBusError> {
        let tx = {
            let mut guard = self.subscriptions.lock().expect("lock poisoned");
            guard
                .entry(topic.clone())
                .or_insert_with(|| {
                    let (tx, _) = broadcast::channel(1024);
                    tx
                })
                .clone()
        };

        let req = SubscribeRequest {
            topic: Some(runtime_topic_to_proto(topic)),
        };
        let mut stream = {
            let mut client = self.client.lock().await;
            client
                .subscribe(req)
                .await
                .map_err(|e| EventBusError::PublishFailed(format!("subscribe rpc: {e}")))?
                .into_inner()
        };
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            while let Some(item) = stream.message().await.transpose() {
                match item {
                    Ok(proto_event) => {
                        if let Ok(event) = proto_event_to_runtime(proto_event) {
                            let _ = tx_clone.send(event);
                        }
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "edge-internal subscribe stream error");
                        break;
                    }
                }
            }
        });
        Ok(tx.subscribe())
    }

    pub async fn publish(
        &self,
        topic: &EventTopic,
        publisher: impl Into<String>,
        payload_type: impl Into<String>,
        payload: Vec<u8>,
    ) -> Result<(), EventBusError> {
        let req = PublishRequest {
            topic: Some(runtime_topic_to_proto(topic)),
            publisher: publisher.into(),
            payload_type: payload_type.into(),
            payload,
        };
        let mut client = self.client.lock().await;
        client
            .publish(req)
            .await
            .map_err(|e| EventBusError::PublishFailed(format!("publish rpc: {e}")))?;
        Ok(())
    }
}

fn runtime_topic_to_proto(topic: &EventTopic) -> ProtoEventTopic {
    ProtoEventTopic {
        overlay: runtime_overlay_to_proto(topic.overlay) as i32,
        name: topic.name.clone(),
    }
}

fn proto_topic_to_runtime(topic: &ProtoEventTopic) -> Result<EventTopic, EventBusError> {
    let overlay = proto_overlay_to_runtime(topic.overlay)?;
    EventTopic::new(overlay, topic.name.clone())
}

fn runtime_event_to_proto(event: EventEnvelope) -> ProtoEventEnvelope {
    ProtoEventEnvelope {
        event_id: event.event_id,
        topic: Some(runtime_topic_to_proto(&event.topic)),
        publisher: event.publisher,
        payload_type: event.payload_type,
        payload: event.payload,
        ts_unix_ms: event.ts_unix_ms,
    }
}

fn proto_event_to_runtime(event: ProtoEventEnvelope) -> Result<EventEnvelope, EventBusError> {
    let topic = event
        .topic
        .as_ref()
        .ok_or_else(|| EventBusError::InvalidEnvelope("missing topic".to_string()))
        .and_then(proto_topic_to_runtime)?;
    Ok(EventEnvelope {
        event_id: event.event_id,
        topic,
        publisher: event.publisher,
        payload_type: event.payload_type,
        payload: event.payload,
        ts_unix_ms: event.ts_unix_ms,
    })
}

fn runtime_overlay_to_proto(overlay: OverlayNetwork) -> ProtoOverlay {
    match overlay {
        OverlayNetwork::EdgeInternal => ProtoOverlay::EdgeInternal,
        OverlayNetwork::EdgeCluster => ProtoOverlay::EdgeCluster,
    }
}

fn proto_overlay_to_runtime(overlay: i32) -> Result<OverlayNetwork, EventBusError> {
    if overlay == ProtoOverlay::EdgeInternal as i32 {
        Ok(OverlayNetwork::EdgeInternal)
    } else if overlay == ProtoOverlay::EdgeCluster as i32 {
        Ok(OverlayNetwork::EdgeCluster)
    } else {
        Err(EventBusError::InvalidTopic(
            "overlay must be specified".to_string(),
        ))
    }
}
