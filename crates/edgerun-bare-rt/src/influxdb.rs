//! InfluxDB client

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub struct InfluxPoint {
    pub measurement: Vec<u8>,
    pub tags: Vec<(Vec<u8>, Vec<u8>)>,
    pub fields: Vec<(Vec<u8>, Vec<u8>)>,
    pub timestamp: Option<i64>,
}

impl InfluxPoint {
    pub fn new(measurement: &[u8]) -> Self {
        Self { measurement: measurement.to_vec(), tags: Vec::new(), fields: Vec::new(), timestamp: None }
    }
    pub fn tag(&mut self, key: &[u8], value: &[u8]) { self.tags.push((key.to_vec(), value.to_vec())); }
    pub fn field(&mut self, key: &[u8], value: &[u8]) { self.fields.push((key.to_vec(), value.to_vec())); }
    pub fn timestamp(&mut self, ts: i64) { self.timestamp = Some(ts); }
}

pub struct InfluxClient;

impl InfluxClient {
    pub fn new() -> Self { Self }
    pub fn connect(&self, _url: &[u8]) -> InfluxConnectFuture { InfluxConnectFuture { done: false } }
    pub fn write(&self, _db: &[u8], _point: &InfluxPoint) -> InfluxWriteFuture { InfluxWriteFuture { done: false } }
    pub fn query(&self, _db: &[u8], _cql: &[u8]) -> InfluxQueryFuture { InfluxQueryFuture { done: false } }
    pub fn ping(&self) -> InfluxPingFuture { InfluxPingFuture { done: false } }
}
impl Default for InfluxClient { fn default() -> Self { Self::new() } }

pub struct InfluxConnectFuture { done: bool }
impl Future for InfluxConnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending }
}
pub struct InfluxWriteFuture { done: bool }
impl Future for InfluxWriteFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending }
}
pub struct InfluxQueryFuture { done: bool }
impl Future for InfluxQueryFuture {
    type Output = Result<Vec<InfluxPoint>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending }
}
pub struct InfluxPingFuture { done: bool }
impl Future for InfluxPingFuture {
    type Output = Result<(u16, String), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending }
}