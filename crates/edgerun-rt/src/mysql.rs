//! MySQL client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub struct MySqlRow {
    pub values: Vec<Option<Vec<u8>>>,
}

pub struct MySqlResult {
    pub rows: Vec<MySqlRow>,
    pub affected: u64,
    pub insert_id: u64,
}

pub struct MySqlClient;

impl MySqlClient {
    pub fn new() -> Self { Self }
    pub fn connect(&self, _host: &[u8], _port: u16, _user: &[u8], _pass: &[u8], _db: &[u8]) -> MyConnectFuture { MyConnectFuture { done: false } }
    pub fn execute(&self, _sql: &[u8]) -> MyExecuteFuture { MyExecuteFuture { done: false } }
    pub fn query(&self, _sql: &[u8]) -> MyQueryFuture { MyQueryFuture { done: false } }
    pub fn prepare(&self, _sql: &[u8]) -> MyPrepareFuture { MyPrepareFuture { done: false } }
    pub fn close(&self) -> MyCloseFuture { MyCloseFuture { done: false } }
}
impl Default for MySqlClient { fn default() -> Self { Self::new() } }

pub struct MyConnectFuture { done: bool }
impl Future for MyConnectFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct MyExecuteFuture { done: bool }
impl Future for MyExecuteFuture { type Output = Result<MySqlResult, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct MyQueryFuture { done: bool }
impl Future for MyQueryFuture { type Output = Result<MySqlResult, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct MyPrepareFuture { done: bool }
impl Future for MyPrepareFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct MyCloseFuture { done: bool }
impl Future for MyCloseFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }