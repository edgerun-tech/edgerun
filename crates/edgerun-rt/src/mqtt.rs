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
}

impl MqttClient {
    pub fn new(client_id: &[u8]) -> Self {
        Self {
            state: MqttState::Disconnected,
            client_id: client_id.to_vec(),
            keepalive: 60,
        }
    }

    pub fn connect(&mut self) -> MqttConnectFuture {
        MqttConnectFuture { client: self }
    }

    pub fn publish(&mut self, _topic: &[u8], _payload: &[u8], _qos: MqttQos) -> MqttPublishFuture {
        MqttPublishFuture { done: false }
    }

    pub fn subscribe(&mut self, _topic: &[u8], _qos: MqttQos) -> MqttSubscribeFuture {
        MqttSubscribeFuture { done: false }
    }

    pub fn disconnect(&mut self) {
        self.state = MqttState::Disconnecting;
    }

    pub fn is_connected(&self) -> bool {
        self.state == MqttState::Connected
    }
}

pub struct MqttConnectFuture<'a> {
    client: &'a mut MqttClient,
}

impl Future for MqttConnectFuture<'_> {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if self.client.is_connected() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}

pub struct MqttPublishFuture {
    done: bool,
}

impl Future for MqttPublishFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct MqttSubscribeFuture {
    done: bool,
}

impl Future for MqttSubscribeFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct MqttMessage {
    pub topic: Vec<u8>,
    pub payload: Vec<u8>,
    pub qos: MqttQos,
}