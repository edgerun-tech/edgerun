//! Cassandra client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub struct CassandraClient;

impl CassandraClient {
    pub fn new() -> Self { Self }
    pub fn connect(&self, _hosts: &[&[u8]]) -> CqlConnectFuture { CqlConnectFuture { done: false } }
    pub fn execute(&self, _cql: &[u8], _params: &[&[u8]]) -> CqlExecuteFuture { CqlExecuteFuture { done: false } }
    pub fn prepare(&self, _cql: &[u8]) -> CqlPrepareFuture { CqlPrepareFuture { done: false } }
    pub fn query(&self, _cql: &[u8]) -> CqlQueryFuture { CqlQueryFuture { done: false } }
    pub fn close(&self) -> CqlCloseFuture { CqlCloseFuture { done: false } }
}
impl Default for CassandraClient { fn default() -> Self { Self::new() } }

pub struct CqlConnectFuture { done: bool }
impl Future for CqlConnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending }
}
pub struct CqlExecuteFuture { done: bool }
impl Future for CqlExecuteFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending }
}
pub struct CqlPrepareFuture { done: bool }
impl Future for CqlPrepareFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending }
}
pub struct CqlQueryFuture { done: bool }
impl Future for CqlQueryFuture {
    type Output = Result<Vec<Vec<u8>>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending }
}
pub struct CqlCloseFuture { done: bool }
impl Future for CqlCloseFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending }
}