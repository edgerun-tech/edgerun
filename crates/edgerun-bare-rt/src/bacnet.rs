//! BACnet client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BacnetService {
    ReadProperty,
    WriteProperty,
    ReadMultiple,
    WriteMultiple,
    SubscribeCOV,
    AtomicWriteFile,
    AtomicReadFile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BacnetObjectType {
    AnalogInput,
    AnalogOutput,
    AnalogValue,
    BinaryInput,
    BinaryOutput,
    BinaryValue,
    Device,
    MultistateInput,
    MultistateOutput,
}

pub struct BacnetObject {
    pub obj_type: BacnetObjectType,
    pub instance: u32,
}

pub struct BacnetProperty {
    pub id: u32,
    pub value: Vec<u8>,
    pub datatype: u8,
}

pub struct BacnetRequest {
    pub device: u32,
    pub object: BacnetObject,
    pub property: u32,
}

impl BacnetRequest {
    pub fn new(device: u32, obj_type: BacnetObjectType, instance: u32, property: u32) -> Self {
        Self {
            device,
            object: BacnetObject { obj_type, instance },
            property,
        }
    }
}

pub struct BacnetResponse {
    pub value: Vec<u8>,
}

impl BacnetResponse {
    pub fn new() -> Self {
        Self { value: Vec::new() }
    }
}

impl Default for BacnetResponse {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BacnetClient;

impl BacnetClient {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&self, _host: &[u8], _port: u16) -> BacnetConnectFuture {
        BacnetConnectFuture { done: false }
    }

    pub fn read(&self, _req: &BacnetRequest) -> BacnetReadFuture {
        BacnetReadFuture { done: false }
    }

    pub fn write(&self, _req: &BacnetRequest, _value: &[u8]) -> BacnetWriteFuture {
        BacnetWriteFuture { done: false }
    }

    pub fn subscribe_cov(&self, _req: &BacnetRequest) -> BacnetCovFuture {
        BacnetCovFuture { done: false }
    }
}

impl Default for BacnetClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BacnetConnectFuture {
    done: bool,
}

impl Future for BacnetConnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct BacnetReadFuture {
    done: bool,
}

impl Future for BacnetReadFuture {
    type Output = Result<BacnetResponse, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct BacnetWriteFuture {
    done: bool,
}

impl Future for BacnetWriteFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct BacnetCovFuture {
    done: bool,
}

impl Future for BacnetCovFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}