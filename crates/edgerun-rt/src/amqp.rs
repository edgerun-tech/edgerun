//! AMQP client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmqpExchange {
    Direct,
    Fanout,
    Topic,
    Headers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmqpQos {
    AtMostOnce,
    AtLeastOnce,
}

pub struct AmqpConnection {
    host: Vec<u8>,
    port: u16,
    vhost: Vec<u8>,
}

pub struct AmqpChannel {
    id: u16,
    exchange: AmqpExchange,
}

impl AmqpChannel {
    pub fn declare_exchange(&mut self, _name: &[u8], _kind: AmqpExchange) -> AmqpExchangeFuture {
        AmqpExchangeFuture { done: false }
    }

    pub fn queue_declare(&mut self, _name: &[u8]) -> AmqpQueueFuture {
        AmqpQueueFuture { done: false }
    }

    pub fn publish(&mut self, _exchange: &[u8], _routing_key: &[u8], _data: &[u8]) -> AmqpPublishFuture {
        AmqpPublishFuture { done: false }
    }

    pub fn consume(&mut self, _queue: &[u8]) -> AmqpConsumeFuture {
        AmqpConsumeFuture { done: false }
    }
}

pub struct AmqpClient;

impl AmqpClient {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&self, _host: &[u8], _port: u16) -> AmqpConnectFuture {
        AmqpConnectFuture { done: false }
    }

    pub fn channel(&self) -> AmqpChannel {
        AmqpChannel { id: 0, exchange: AmqpExchange::Direct }
    }
}

impl Default for AmqpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AmqpConnectFuture {
    done: bool,
}

impl Future for AmqpConnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct AmqpExchangeFuture {
    done: bool,
}

impl Future for AmqpExchangeFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct AmqpQueueFuture {
    done: bool,
}

impl Future for AmqpQueueFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct AmqpPublishFuture {
    done: bool,
}

impl Future for AmqpPublishFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct AmqpConsumeFuture {
    done: bool,
}

impl Future for AmqpConsumeFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}