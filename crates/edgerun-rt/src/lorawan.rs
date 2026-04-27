//! LoRaWAN client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoraRegion {
    EU868,
    US915,
    AU915,
    AS923,
    KR920,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoraClass {
    A,
    B,
    C,
}

pub struct LoraKeys {
    pub dev_eui: [u8; 8],
    pub app_eui: [u8; 8],
    pub app_key: [u8; 16],
    pub nwk_key: [u8; 16],
}

impl LoraKeys {
    pub fn new() -> Self {
        Self {
            dev_eui: [0; 8],
            app_eui: [0; 8],
            app_key: [0; 16],
            nwk_key: [0; 16],
        }
    }
}

impl Default for LoraKeys {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LoraClient {
    region: Option<LoraRegion>,
    class: LoraClass,
    joined: bool,
    sleeping: bool,
    uplink: Vec<(u8, Vec<u8>)>,
    downlink: Vec<(u8, Vec<u8>)>,
}

impl LoraClient {
    pub fn new() -> Self {
        Self {
            region: None,
            class: LoraClass::A,
            joined: false,
            sleeping: false,
            uplink: Vec::new(),
            downlink: Vec::new(),
        }
    }

    pub fn init(&mut self, region: LoraRegion) -> LoraInitFuture {
        self.region = Some(region);
        self.sleeping = false;
        LoraInitFuture {
            result: Some(Ok(())),
        }
    }

    pub fn join(&mut self, _keys: &LoraKeys, class: LoraClass) -> LoraJoinFuture {
        let result = if self.region.is_some() {
            self.class = class;
            self.joined = true;
            Ok(())
        } else {
            Err(())
        };
        LoraJoinFuture {
            result: Some(result),
        }
    }

    pub fn send(&mut self, port: u8, data: &[u8]) -> LoraSendFuture {
        let result = if self.joined && !self.sleeping && port != 0 {
            self.uplink.push((port, data.to_vec()));
            self.downlink.push((port, data.to_vec()));
            Ok(())
        } else {
            Err(())
        };
        LoraSendFuture {
            result: Some(result),
        }
    }

    pub fn recv(&mut self) -> LoraRecvFuture {
        LoraRecvFuture {
            result: Some(if self.joined {
                self.downlink.pop().ok_or(())
            } else {
                Err(())
            }),
        }
    }

    pub fn sleep(&mut self, class: LoraClass) -> LoraSleepFuture {
        let result = if self.joined {
            self.class = class;
            self.sleeping = true;
            Ok(())
        } else {
            Err(())
        };
        LoraSleepFuture {
            result: Some(result),
        }
    }

    pub fn wake(&mut self) -> LoraWakeFuture {
        self.sleeping = false;
        LoraWakeFuture {
            result: Some(Ok(())),
        }
    }

    #[must_use]
    pub fn is_joined(&self) -> bool {
        self.joined
    }

    #[must_use]
    pub fn uplink(&self) -> &[(u8, Vec<u8>)] {
        &self.uplink
    }
}

impl Default for LoraClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LoraInitFuture {
    result: Option<Result<(), ()>>,
}
impl Future for LoraInitFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct LoraJoinFuture {
    result: Option<Result<(), ()>>,
}
impl Future for LoraJoinFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct LoraSendFuture {
    result: Option<Result<(), ()>>,
}
impl Future for LoraSendFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct LoraRecvFuture {
    result: Option<Result<(u8, Vec<u8>), ()>>,
}
impl Future for LoraRecvFuture {
    type Output = Result<(u8, Vec<u8>), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct LoraSleepFuture {
    result: Option<Result<(), ()>>,
}
impl Future for LoraSleepFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct LoraWakeFuture {
    result: Option<Result<(), ()>>,
}
impl Future for LoraWakeFuture {
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
    fn lorawan_init_join_send_recv_complete() {
        let mut client = LoraClient::new();

        crate::block_on(Box::pin(client.init(LoraRegion::EU868))).unwrap();
        crate::block_on(Box::pin(client.join(&LoraKeys::new(), LoraClass::A))).unwrap();
        crate::block_on(Box::pin(client.send(1, b"ping"))).unwrap();
        let downlink = crate::block_on(Box::pin(client.recv())).unwrap();

        assert!(client.is_joined());
        assert_eq!(client.uplink()[0], (1, b"ping".to_vec()));
        assert_eq!(downlink, (1, b"ping".to_vec()));
    }
}
