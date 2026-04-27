//! MQTT client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub const MQTT_MAX_PAYLOAD: usize = 268435455;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MqttQos {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MqttState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

pub struct MqttClient {
    state: MqttState,
    client_id: Vec<u8>,
    keepalive: u16,
    subscriptions: Vec<Vec<u8>>,
    outbound: Vec<MqttMessage>,
}

impl MqttClient {
    pub fn new(client_id: &[u8]) -> Self {
        Self {
            state: MqttState::Disconnected,
            client_id: client_id.to_vec(),
            keepalive: 60,
            subscriptions: Vec::new(),
            outbound: Vec::new(),
        }
    }

    pub fn connect(&mut self) -> MqttConnectFuture<'_> {
        MqttConnectFuture { client: self }
    }

    pub fn publish(&mut self, topic: &[u8], payload: &[u8], qos: MqttQos) -> MqttPublishFuture {
        let result =
            if self.is_connected() && !topic.is_empty() && payload.len() <= MQTT_MAX_PAYLOAD {
                self.outbound.push(MqttMessage {
                    topic: topic.to_vec(),
                    payload: payload.to_vec(),
                    qos,
                });
                Ok(())
            } else {
                Err(())
            };
        MqttPublishFuture {
            result: Some(result),
        }
    }

    pub fn subscribe(&mut self, topic: &[u8], _qos: MqttQos) -> MqttSubscribeFuture {
        let result = if self.is_connected() && !topic.is_empty() {
            self.subscriptions.push(topic.to_vec());
            Ok(())
        } else {
            Err(())
        };
        MqttSubscribeFuture {
            result: Some(result),
        }
    }

    pub fn disconnect(&mut self) {
        self.state = MqttState::Disconnecting;
    }

    pub fn is_connected(&self) -> bool {
        self.state == MqttState::Connected
    }

    #[must_use]
    pub fn client_id(&self) -> &[u8] {
        &self.client_id
    }

    #[must_use]
    pub fn keepalive(&self) -> u16 {
        self.keepalive
    }

    #[must_use]
    pub fn subscriptions(&self) -> &[Vec<u8>] {
        &self.subscriptions
    }

    #[must_use]
    pub fn outbound(&self) -> &[MqttMessage] {
        &self.outbound
    }
}

pub struct MqttConnectFuture<'a> {
    client: &'a mut MqttClient,
}

impl Future for MqttConnectFuture<'_> {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        self.client.state = MqttState::Connected;
        Poll::Ready(Ok(()))
    }
}

pub struct MqttPublishFuture {
    result: Option<Result<(), ()>>,
}

impl Future for MqttPublishFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct MqttSubscribeFuture {
    result: Option<Result<(), ()>>,
}

impl Future for MqttSubscribeFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct MqttMessage {
    pub topic: Vec<u8>,
    pub payload: Vec<u8>,
    pub qos: MqttQos,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    #[test]
    fn mqtt_connect_publish_and_subscribe_complete() {
        let mut client = MqttClient::new(b"node-1");

        crate::block_on(Box::pin(client.connect())).unwrap();
        crate::block_on(Box::pin(
            client.subscribe(b"sensors/#", MqttQos::AtLeastOnce),
        ))
        .unwrap();
        crate::block_on(Box::pin(client.publish(
            b"sensors/temp",
            b"21",
            MqttQos::AtMostOnce,
        )))
        .unwrap();

        assert!(client.is_connected());
        assert_eq!(client.subscriptions()[0], b"sensors/#");
        assert_eq!(client.outbound()[0].payload, b"21");
    }
}
