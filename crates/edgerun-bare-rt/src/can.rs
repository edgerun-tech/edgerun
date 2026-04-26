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

pub struct CanFrame {
    pub id: u32,
    pub dlc: u8,
    pub data: [u8; 8],
    pub extended: bool,
}

impl CanFrame {
    pub fn new(id: u32, data: &[u8]) -> Self {
        let mut frame = Self { id, dlc: data.len() as u8, data: [0; 8], extended: false };
        frame.data[..data.len()].copy_from_slice(data);
        frame
    }
}

pub struct CanClient;

impl CanClient {
    pub fn new() -> Self { Self }
    pub fn init(&self, _channel: u8) -> CanInitFuture { CanInitFuture { done: false } }
    pub fn send(&self, _frame: &CanFrame) -> CanSendFuture { CanSendFuture { done: false } }
    pub fn recv(&self) -> CanRecvFuture { CanRecvFuture { done: false } }
    pub fn filter(&self, _id: u32, _mask: u32) -> CanFilterFuture { CanFilterFuture { done: false } }
    pub fn state(&self) -> CanStateFuture { CanStateFuture { done: false } }
}
impl Default for CanClient { fn default() -> Self { Self::new() } }

pub struct CanInitFuture { done: bool }
impl Future for CanInitFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct CanSendFuture { done: bool }
impl Future for CanSendFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct CanRecvFuture { done: bool }
impl Future for CanRecvFuture { type Output = Result<CanFrame, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct CanFilterFuture { done: bool }
impl Future for CanFilterFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct CanStateFuture { done: bool }
impl Future for CanStateFuture { type Output = Result<CanState, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }