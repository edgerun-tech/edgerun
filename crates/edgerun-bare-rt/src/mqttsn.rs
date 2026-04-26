//! MQTT-SN client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MqttsnQos {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

pub struct MqttsnClient;

impl MqttsnClient {
    pub fn new() -> Self { Self }
    pub fn connect(&self, _gateway: &[u8], _port: u16, _client_id: &[u8]) -> MqttsnConnectFuture { MqttsnConnectFuture { done: false } }
    pub fn register(&self, _topic_id: u16) -> MqttsnRegisterFuture { MqttsnRegisterFuture { done: false } }
    pub fn subscribe(&self, _topic: &[u8], _qos: MqttsnQos) -> MqttsnSubscribeFuture { MqttsnSubscribeFuture { done: false } }
    pub fn publish(&self, _topic_id: u16, _data: &[u8], _qos: MqttsnQos) -> MqttsnPublishFuture { MqttsnPublishFuture { done: false } }
    pub fn unsubscribe(&self, _topic: &[u8]) -> MqttsnUnsubscribeFuture { MqttsnUnsubscribeFuture { done: false } }
    pub fn disconnect(&self) -> MqttsnDisconnectFuture { MqttsnDisconnectFuture { done: false } }
}
impl Default for MqttsnClient { fn default() -> Self { Self::new() } }

pub struct MqttsnConnectFuture { done: bool }
impl Future for MqttsnConnectFuture { type Output = Result<u16, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct MqttsnRegisterFuture { done: bool }
impl Future for MqttsnRegisterFuture { type Output = Result<u16, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct MqttsnSubscribeFuture { done: bool }
impl Future for MqttsnSubscribeFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct MqttsnPublishFuture { done: bool }
impl Future for MqttsnPublishFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct MqttsnUnsubscribeFuture { done: bool }
impl Future for MqttsnUnsubscribeFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct MqttsnDisconnectFuture { done: bool }
impl Future for MqttsnDisconnectFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }