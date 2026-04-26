//! OPC-UA client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpcUAAttribute {
    NodeId,
    NodeClass,
    BrowseName,
    DisplayName,
    Description,
    Value,
    DataType,
    ValueRank,
    AccessLevel,
    UserAccessLevel,
    MinimumSamplingInterval,
    Historizing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpcUANodeClass {
    Object,
    Variable,
    Method,
    ObjectType,
    VariableType,
    DataType,
    ReferenceType,
    View,
}

pub struct OpcUANodeId {
    pub namespace: u16,
    pub identifier: u32,
}

impl OpcUANodeId {
    pub fn new(namespace: u16, identifier: u32) -> Self {
        Self { namespace, identifier }
    }
}

pub struct OpcUAReadRequest {
    pub node_id: OpcUANodeId,
    pub attribute: OpcUAAttribute,
}

pub struct OpcUAWriteRequest {
    pub node_id: OpcUANodeId,
    pub attribute: OpcUAAttribute,
    pub value: Vec<u8>,
}

pub struct OpcUASubscription {
    pub sub_id: u32,
    pub node_id: OpcUANodeId,
}

pub struct OpcUAClient;

impl OpcUAClient {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&self, _url: &[u8]) -> OpcUAConnectFuture {
        OpcUAConnectFuture { done: false }
    }

    pub fn browse(&self, _node_id: &OpcUANodeId) -> OpcUABrowseFuture {
        OpcUABrowseFuture { done: false }
    }

    pub fn read(&self, _req: &OpcUAReadRequest) -> OpcUAReadFuture {
        OpcUAReadFuture { done: false }
    }

    pub fn write(&self, _req: &OpcUAWriteRequest) -> OpcUAWriteFuture {
        OpcUAWriteFuture { done: false }
    }

    pub fn subscribe(&self, _nodes: &[OpcUANodeId], _handler: fn(&[u8])) -> OpcUASubscribeFuture {
        OpcUASubscribeFuture { done: false }
    }

    pub fn call(&self, _method: &OpcUANodeId, _args: &[u8]) -> OpcUACallFuture {
        OpcUACallFuture { done: false }
    }
}

impl Default for OpcUAClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OpcUAConnectFuture {
    done: bool,
}

impl Future for OpcUAConnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct OpcUABrowseFuture {
    done: bool,
}

impl Future for OpcUABrowseFuture {
    type Output = Result<Vec<OpcUANodeId>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct OpcUAReadFuture {
    done: bool,
}

impl Future for OpcUAReadFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct OpcUAWriteFuture {
    done: bool,
}

impl Future for OpcUAWriteFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct OpcUASubscribeFuture {
    done: bool,
}

impl Future for OpcUASubscribeFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct OpcUACallFuture {
    done: bool,
}

impl Future for OpcUACallFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}