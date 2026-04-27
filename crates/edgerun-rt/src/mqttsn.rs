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

pub struct MqttsnClient {
    connected: bool,
    gateway_id: u16,
    next_topic_id: u16,
    topics: Vec<(u16, Vec<u8>)>,
    outbound: Vec<(u16, Vec<u8>, MqttsnQos)>,
}

impl MqttsnClient {
    pub fn new() -> Self {
        Self {
            connected: false,
            gateway_id: 0,
            next_topic_id: 1,
            topics: Vec::new(),
            outbound: Vec::new(),
        }
    }

    pub fn connect(&mut self, gateway: &[u8], port: u16, client_id: &[u8]) -> MqttsnConnectFuture {
        let result = if gateway.is_empty() || port == 0 || client_id.is_empty() {
            Err(())
        } else {
            self.connected = true;
            self.gateway_id = 1;
            Ok(self.gateway_id)
        };
        MqttsnConnectFuture {
            result: Some(result),
        }
    }

    pub fn register(&mut self, topic_id: u16) -> MqttsnRegisterFuture {
        let result = if self.connected && topic_id != 0 {
            if !self.topics.iter().any(|(id, _)| *id == topic_id) {
                self.topics.push((topic_id, Vec::new()));
            }
            Ok(topic_id)
        } else if self.connected {
            let id = self.next_topic_id;
            self.next_topic_id = self.next_topic_id.saturating_add(1);
            self.topics.push((id, Vec::new()));
            Ok(id)
        } else {
            Err(())
        };
        MqttsnRegisterFuture {
            result: Some(result),
        }
    }

    pub fn subscribe(&mut self, topic: &[u8], _qos: MqttsnQos) -> MqttsnSubscribeFuture {
        let result = if self.connected && !topic.is_empty() {
            let id = self.next_topic_id;
            self.next_topic_id = self.next_topic_id.saturating_add(1);
            self.topics.push((id, topic.to_vec()));
            Ok(())
        } else {
            Err(())
        };
        MqttsnSubscribeFuture {
            result: Some(result),
        }
    }

    pub fn publish(&mut self, topic_id: u16, data: &[u8], qos: MqttsnQos) -> MqttsnPublishFuture {
        let result = if self.connected && topic_id != 0 {
            self.outbound.push((topic_id, data.to_vec(), qos));
            Ok(())
        } else {
            Err(())
        };
        MqttsnPublishFuture {
            result: Some(result),
        }
    }

    pub fn unsubscribe(&mut self, topic: &[u8]) -> MqttsnUnsubscribeFuture {
        let result = if self.connected {
            self.topics.retain(|(_, name)| name.as_slice() != topic);
            Ok(())
        } else {
            Err(())
        };
        MqttsnUnsubscribeFuture {
            result: Some(result),
        }
    }

    pub fn disconnect(&mut self) -> MqttsnDisconnectFuture {
        self.connected = false;
        MqttsnDisconnectFuture {
            result: Some(Ok(())),
        }
    }

    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    #[must_use]
    pub fn outbound(&self) -> &[(u16, Vec<u8>, MqttsnQos)] {
        &self.outbound
    }
}

impl Default for MqttsnClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MqttsnConnectFuture {
    result: Option<Result<u16, ()>>,
}
impl Future for MqttsnConnectFuture {
    type Output = Result<u16, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct MqttsnRegisterFuture {
    result: Option<Result<u16, ()>>,
}
impl Future for MqttsnRegisterFuture {
    type Output = Result<u16, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct MqttsnSubscribeFuture {
    result: Option<Result<(), ()>>,
}
impl Future for MqttsnSubscribeFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct MqttsnPublishFuture {
    result: Option<Result<(), ()>>,
}
impl Future for MqttsnPublishFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct MqttsnUnsubscribeFuture {
    result: Option<Result<(), ()>>,
}
impl Future for MqttsnUnsubscribeFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct MqttsnDisconnectFuture {
    result: Option<Result<(), ()>>,
}
impl Future for MqttsnDisconnectFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    #[test]
    fn mqttsn_connect_subscribe_publish_complete() {
        let mut client = MqttsnClient::new();

        let gateway = crate::block_on(Box::pin(client.connect(b"gw", 1884, b"node"))).unwrap();
        crate::block_on(Box::pin(
            client.subscribe(b"sensor/temp", MqttsnQos::AtLeastOnce),
        ))
        .unwrap();
        crate::block_on(Box::pin(client.publish(
            gateway,
            b"21",
            MqttsnQos::AtMostOnce,
        )))
        .unwrap();

        assert!(client.is_connected());
        assert_eq!(client.outbound()[0].1, b"21");
    }
}
