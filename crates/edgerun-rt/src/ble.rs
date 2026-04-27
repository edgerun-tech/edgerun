//! BLE client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BleState {
    PoweredOff,
    PoweredOn,
    Advertising,
    Connected,
    Disconnected,
}

pub struct BleService {
    pub uuid: [u8; 16],
    pub handle: u16,
}

pub struct BleCharacteristic {
    pub uuid: [u8; 16],
    pub handle: u16,
    pub properties: u8,
}

pub struct BleDevice {
    pub addr: [u8; 6],
    pub rssi: i8,
    pub name: Vec<u8>,
}

pub struct BleClient;

impl BleClient {
    pub fn new() -> Self { Self }
    pub fn init(&self) -> BleInitFuture { BleInitFuture { done: false } }
    pub fn scan(&self, _duration: u16) -> BleScanFuture { BleScanFuture { done: false } }
    pub fn connect(&self, _addr: &[u8]) -> BleConnectFuture { BleConnectFuture { done: false } }
    pub fn disconnect(&self, _handle: u16) -> BleDisconnectFuture { BleDisconnectFuture { done: false } }
    pub fn discover_services(&self, _handle: u16) -> BleDiscoverSvcsFuture { BleDiscoverSvcsFuture { done: false } }
    pub fn discover_chars(&self, _handle: u16, _svc: u16) -> BleDiscoverCharsFuture { BleDiscoverCharsFuture { done: false } }
    pub fn read(&self, _handle: u16) -> BleReadFuture { BleReadFuture { done: false } }
    pub fn write(&self, _handle: u16, _data: &[u8]) -> BleWriteFuture { BleWriteFuture { done: false } }
    pub fn notify(&self, _handle: u16, _enable: bool) -> BleNotifyFuture { BleNotifyFuture { done: false } }
}
impl Default for BleClient { fn default() -> Self { Self::new() } }

pub struct BleInitFuture { done: bool }
impl Future for BleInitFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct BleScanFuture { done: bool }
impl Future for BleScanFuture { type Output = Result<Vec<BleDevice>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct BleConnectFuture { done: bool }
impl Future for BleConnectFuture { type Output = Result<u16, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct BleDisconnectFuture { done: bool }
impl Future for BleDisconnectFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct BleDiscoverSvcsFuture { done: bool }
impl Future for BleDiscoverSvcsFuture { type Output = Result<Vec<BleService>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct BleDiscoverCharsFuture { done: bool }
impl Future for BleDiscoverCharsFuture { type Output = Result<Vec<BleCharacteristic>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct BleReadFuture { done: bool }
impl Future for BleReadFuture { type Output = Result<Vec<u8>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct BleWriteFuture { done: bool }
impl Future for BleWriteFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct BleNotifyFuture { done: bool }
impl Future for BleNotifyFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }