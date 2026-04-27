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
        Self { dev_eui: [0; 8], app_eui: [0; 8], app_key: [0; 16], nwk_key: [0; 16] }
    }
}

pub struct LoraClient;

impl LoraClient {
    pub fn new() -> Self { Self }
    pub fn init(&self, _region: LoraRegion) -> LoraInitFuture { LoraInitFuture { done: false } }
    pub fn join(&self, _keys: &LoraKeys, _class: LoraClass) -> LoraJoinFuture { LoraJoinFuture { done: false } }
    pub fn send(&self, _port: u8, _data: &[u8]) -> LoraSendFuture { LoraSendFuture { done: false } }
    pub fn recv(&self) -> LoraRecvFuture { LoraRecvFuture { done: false } }
    pub fn sleep(&self, _class: LoraClass) -> LoraSleepFuture { LoraSleepFuture { done: false } }
    pub fn wake(&self) -> LoraWakeFuture { LoraWakeFuture { done: false } }
}
impl Default for LoraClient { fn default() -> Self { Self::new() } }

pub struct LoraInitFuture { done: bool }
impl Future for LoraInitFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct LoraJoinFuture { done: bool }
impl Future for LoraJoinFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct LoraSendFuture { done: bool }
impl Future for LoraSendFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct LoraRecvFuture { done: bool }
impl Future for LoraRecvFuture { type Output = Result<(u8, Vec<u8>), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct LoraSleepFuture { done: bool }
impl Future for LoraSleepFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct LoraWakeFuture { done: bool }
impl Future for LoraWakeFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }