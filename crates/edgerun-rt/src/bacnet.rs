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

pub struct BacnetClient {
    connected: bool,
    properties: Vec<BacnetProperty>,
}

impl BacnetClient {
    pub fn new() -> Self {
        Self {
            connected: false,
            properties: Vec::new(),
        }
    }

    pub fn connect(&mut self, host: &[u8], port: u16) -> BacnetConnectFuture {
        let result = if !host.is_empty() && port != 0 {
            self.connected = true;
            Ok(())
        } else {
            Err(())
        };
        BacnetConnectFuture {
            result: Some(result),
        }
    }

    pub fn read(&self, req: &BacnetRequest) -> BacnetReadFuture {
        let result = if !self.connected {
            Err(())
        } else {
            let mut response = BacnetResponse::new();
            if let Some(property) = self.properties.iter().find(|p| p.id == req.property) {
                response.value = property.value.clone();
            }
            Ok(response)
        };
        BacnetReadFuture {
            result: Some(result),
        }
    }

    pub fn write(&mut self, req: &BacnetRequest, value: &[u8]) -> BacnetWriteFuture {
        let result = if !self.connected {
            Err(())
        } else {
            if let Some(property) = self.properties.iter_mut().find(|p| p.id == req.property) {
                property.value = value.to_vec();
            } else {
                self.properties.push(BacnetProperty {
                    id: req.property,
                    value: value.to_vec(),
                    datatype: 0,
                });
            }
            Ok(())
        };
        BacnetWriteFuture {
            result: Some(result),
        }
    }

    pub fn subscribe_cov(&self, _req: &BacnetRequest) -> BacnetCovFuture {
        BacnetCovFuture {
            result: Some(if self.connected { Ok(()) } else { Err(()) }),
        }
    }

    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

impl Default for BacnetClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BacnetConnectFuture {
    result: Option<Result<(), ()>>,
}

impl Future for BacnetConnectFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct BacnetReadFuture {
    result: Option<Result<BacnetResponse, ()>>,
}

impl Future for BacnetReadFuture {
    type Output = Result<BacnetResponse, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct BacnetWriteFuture {
    result: Option<Result<(), ()>>,
}

impl Future for BacnetWriteFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct BacnetCovFuture {
    result: Option<Result<(), ()>>,
}

impl Future for BacnetCovFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}
