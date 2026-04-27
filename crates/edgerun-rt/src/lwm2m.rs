//! LwM2M client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub const LWM2M_MAX_OBJECTS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LwM2MOperation {
    Read,
    Write,
    Execute,
    Observe,
    Discover,
}

pub struct LwM2MObject {
    pub id: u16,
    pub instances: usize,
}

pub struct LwM2MResource {
    pub id: u16,
    pub data: Vec<u8>,
    pub type_id: u8,
}

pub struct LwM2MClient {
    endpoint: Vec<u8>,
    server: Vec<u8>,
    objects: Vec<LwM2MObject>,
    resources: Vec<(u16, u16, Vec<u8>)>,
    registered: bool,
}

impl LwM2MClient {
    pub fn new(endpoint: &[u8], server: &[u8]) -> Self {
        Self {
            endpoint: endpoint.to_vec(),
            server: server.to_vec(),
            objects: Vec::new(),
            resources: Vec::new(),
            registered: false,
        }
    }

    pub fn register(&mut self) -> LwM2MRegisterFuture {
        let result = if self.endpoint.is_empty() || self.server.is_empty() {
            Err(())
        } else {
            self.registered = true;
            Ok(())
        };
        LwM2MRegisterFuture {
            result: Some(result),
        }
    }

    pub fn read(&self, obj_id: u16, res_id: u16) -> LwM2MReadFuture {
        let result = if !self.registered {
            Err(())
        } else {
            self.resources
                .iter()
                .find(|(obj, res, _)| *obj == obj_id && *res == res_id)
                .map(|(_, _, data)| data.clone())
                .ok_or(())
        };
        LwM2MReadFuture {
            result: Some(result),
        }
    }

    pub fn write(&mut self, obj_id: u16, res_id: u16, data: &[u8]) -> LwM2MWriteFuture {
        let result = if !self.registered {
            Err(())
        } else {
            if let Some((_, _, stored)) = self
                .resources
                .iter_mut()
                .find(|(obj, res, _)| *obj == obj_id && *res == res_id)
            {
                *stored = data.to_vec();
            } else {
                self.resources.push((obj_id, res_id, data.to_vec()));
            }
            if !self.objects.iter().any(|object| object.id == obj_id) {
                self.objects.push(LwM2MObject {
                    id: obj_id,
                    instances: 1,
                });
            }
            Ok(())
        };
        LwM2MWriteFuture {
            result: Some(result),
        }
    }

    pub fn observe(&self, obj_id: u16, res_id: u16) -> LwM2MObserveFuture {
        self.read(obj_id, res_id).into_observe()
    }

    pub fn deregister(&mut self) -> LwM2MDeregisterFuture {
        self.registered = false;
        LwM2MDeregisterFuture {
            result: Some(Ok(())),
        }
    }

    #[must_use]
    pub fn is_registered(&self) -> bool {
        self.registered
    }

    #[must_use]
    pub fn objects(&self) -> &[LwM2MObject] {
        &self.objects
    }
}

pub struct LwM2MRegisterFuture {
    result: Option<Result<(), ()>>,
}

impl Future for LwM2MRegisterFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct LwM2MReadFuture {
    result: Option<Result<Vec<u8>, ()>>,
}

impl LwM2MReadFuture {
    fn into_observe(self) -> LwM2MObserveFuture {
        LwM2MObserveFuture {
            result: self.result,
        }
    }
}

impl Future for LwM2MReadFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct LwM2MWriteFuture {
    result: Option<Result<(), ()>>,
}

impl Future for LwM2MWriteFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct LwM2MObserveFuture {
    result: Option<Result<Vec<u8>, ()>>,
}

impl Future for LwM2MObserveFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct LwM2MDeregisterFuture {
    result: Option<Result<(), ()>>,
}

impl Future for LwM2MDeregisterFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}
