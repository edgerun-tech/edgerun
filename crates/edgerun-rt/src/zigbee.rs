//! Zigbee client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZbDeviceType {
    OnOffSwitch,
    Dimmer,
    TemperatureSensor,
    humiditySensor,
    DoorLock,
    Thermostat,
    ColorLight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZbDeviceState {
    Added,
    Removed,
    Online,
    Offline,
}

pub struct ZbDevice {
    pub short_id: u16,
    pub ieee_addr: [u8; 8],
    pub device_type: ZbDeviceType,
    pub state: ZbDeviceState,
    pub endpoint: u8,
}

pub struct ZbCluster {
    pub id: u16,
    pub attr_id: u16,
    pub value: Vec<u8>,
}

pub struct ZbClient;

impl ZbClient {
    pub fn new() -> Self { Self }
    pub fn init(&self, _channel: u8) -> ZbInitFuture { ZbInitFuture { done: false } }
    pub fn permit_join(&self, _duration: u8) -> ZbPermitJoinFuture { ZbPermitJoinFuture { done: false } }
    pub fn devices(&self) -> ZbDevicesFuture { ZbDevicesFuture { done: false } }
    pub fn remove(&self, _ieee: &[u8]) -> ZbRemoveFuture { ZbRemoveFuture { done: false } }
    pub fn read_attr(&self, _ieee: &[u8], _ep: u8, _cluster: u16, _attr: u16) -> ZbReadAttrFuture { ZbReadAttrFuture { done: false } }
    pub fn write_attr(&self, _ieee: &[u8], _ep: u8, _cluster: u16, _attr: u16, _value: &[u8]) -> ZbWriteAttrFuture { ZbWriteAttrFuture { done: false } }
    pub fn send_onoff(&self, _ieee: &[u8], _ep: u8, _on: bool) -> ZbOnOffFuture { ZbOnOffFuture { done: false } }
    pub fn level(&self, _ieee: &[u8], _ep: u8, _level: u8) -> ZbLevelFuture { ZbLevelFuture { done: false } }
    pub fn color(&self, _ieee: &[u8], _ep: u8, _hue: u8, _sat: u8) -> ZbColorFuture { ZbColorFuture { done: false } }
}
impl Default for ZbClient { fn default() -> Self { Self::new() } }

pub struct ZbInitFuture { done: bool }
impl Future for ZbInitFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct ZbPermitJoinFuture { done: bool }
impl Future for ZbPermitJoinFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct ZbDevicesFuture { done: bool }
impl Future for ZbDevicesFuture { type Output = Result<Vec<ZbDevice>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct ZbRemoveFuture { done: bool }
impl Future for ZbRemoveFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct ZbReadAttrFuture { done: bool }
impl Future for ZbReadAttrFuture { type Output = Result<Vec<u8>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct ZbWriteAttrFuture { done: bool }
impl Future for ZbWriteAttrFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct ZbOnOffFuture { done: bool }
impl Future for ZbOnOffFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct ZbLevelFuture { done: bool }
impl Future for ZbLevelFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct ZbColorFuture { done: bool }
impl Future for ZbColorFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }