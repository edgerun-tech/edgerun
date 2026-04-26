//! PostgreSQL client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub struct PgRow {
    pub values: Vec<Option<Vec<u8>>>,
}

pub struct PgResult {
    pub rows: Vec<PgRow>,
    pub affected: u64,
}

pub struct PgClient;

impl PgClient {
    pub fn new() -> Self { Self }
    pub fn connect(&self, _host: &[u8], _port: u16, _user: &[u8], _pass: &[u8], _db: &[u8]) -> PgConnectFuture { PgConnectFuture { done: false } }
    pub fn execute(&self, _sql: &[u8]) -> PgExecuteFuture { PgExecuteFuture { done: false } }
    pub fn query(&self, _sql: &[u8]) -> PgQueryFuture { PgQueryFuture { done: false } }
    pub fn prepare(&self, _sql: &[u8]) -> PgPrepareFuture { PgPrepareFuture { done: false } }
    pub fn close(&self) -> PgCloseFuture { PgCloseFuture { done: false } }
}
impl Default for PgClient { fn default() -> Self { Self::new() } }

pub struct PgConnectFuture { done: bool }
impl Future for PgConnectFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct PgExecuteFuture { done: bool }
impl Future for PgExecuteFuture { type Output = Result<PgResult, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct PgQueryFuture { done: bool }
impl Future for PgQueryFuture { type Output = Result<PgResult, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct PgPrepareFuture { done: bool }
impl Future for PgPrepareFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct PgCloseFuture { done: bool }
impl Future for PgCloseFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }