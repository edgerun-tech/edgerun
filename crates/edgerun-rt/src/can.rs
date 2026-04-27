//! CAN bus client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanState {
    ErrorActive,
    ErrorPassive,
    BusOff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanFrame {
    pub id: u32,
    pub dlc: u8,
    pub data: [u8; 8],
    pub extended: bool,
}

impl CanFrame {
    pub fn new(id: u32, data: &[u8]) -> Self {
        let len = core::cmp::min(data.len(), 8);
        let mut frame = Self {
            id,
            dlc: len as u8,
            data: [0; 8],
            extended: false,
        };
        frame.data[..len].copy_from_slice(&data[..len]);
        frame
    }
}

pub struct CanClient {
    initialized: bool,
    state: CanState,
    filters: Vec<(u32, u32)>,
    tx: Vec<CanFrame>,
    rx: Vec<CanFrame>,
}

impl CanClient {
    pub fn new() -> Self {
        Self {
            initialized: false,
            state: CanState::BusOff,
            filters: Vec::new(),
            tx: Vec::new(),
            rx: Vec::new(),
        }
    }

    pub fn init(&mut self, _channel: u8) -> CanInitFuture {
        self.initialized = true;
        self.state = CanState::ErrorActive;
        CanInitFuture {
            result: Some(Ok(())),
        }
    }

    pub fn send(&mut self, frame: &CanFrame) -> CanSendFuture {
        let result = if self.initialized && self.state != CanState::BusOff {
            self.tx.push(*frame);
            self.rx.push(*frame);
            Ok(())
        } else {
            Err(())
        };
        CanSendFuture {
            result: Some(result),
        }
    }

    pub fn recv(&mut self) -> CanRecvFuture {
        let result = if self.initialized {
            self.rx.pop().ok_or(())
        } else {
            Err(())
        };
        CanRecvFuture {
            result: Some(result),
        }
    }

    pub fn filter(&mut self, id: u32, mask: u32) -> CanFilterFuture {
        let result = if self.initialized {
            self.filters.push((id, mask));
            Ok(())
        } else {
            Err(())
        };
        CanFilterFuture {
            result: Some(result),
        }
    }

    pub fn state(&self) -> CanStateFuture {
        CanStateFuture {
            result: Some(if self.initialized {
                Ok(self.state)
            } else {
                Err(())
            }),
        }
    }

    #[must_use]
    pub fn transmitted(&self) -> &[CanFrame] {
        &self.tx
    }
}

impl Default for CanClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CanInitFuture {
    result: Option<Result<(), ()>>,
}
impl Future for CanInitFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct CanSendFuture {
    result: Option<Result<(), ()>>,
}
impl Future for CanSendFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct CanRecvFuture {
    result: Option<Result<CanFrame, ()>>,
}
impl Future for CanRecvFuture {
    type Output = Result<CanFrame, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct CanFilterFuture {
    result: Option<Result<(), ()>>,
}
impl Future for CanFilterFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct CanStateFuture {
    result: Option<Result<CanState, ()>>,
}
impl Future for CanStateFuture {
    type Output = Result<CanState, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    #[test]
    fn can_init_send_recv_complete() {
        let mut client = CanClient::new();
        let frame = CanFrame::new(0x123, b"abc");

        crate::block_on(Box::pin(client.init(0))).unwrap();
        crate::block_on(Box::pin(client.filter(0x100, 0x700))).unwrap();
        crate::block_on(Box::pin(client.send(&frame))).unwrap();
        let received = crate::block_on(Box::pin(client.recv())).unwrap();

        assert_eq!(received, frame);
        assert_eq!(client.transmitted()[0], frame);
    }
}
