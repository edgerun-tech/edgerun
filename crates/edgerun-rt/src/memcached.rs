//! Memcached client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub struct MemcachedClient;

impl MemcachedClient {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&self, _host: &[u8], _port: u16) -> McConnectFuture {
        McConnectFuture { done: false }
    }

    pub fn get(&self, _key: &[u8]) -> McGetFuture {
        McGetFuture { done: false }
    }

    pub fn set(&self, _key: &[u8], _value: &[u8], _flags: u32, _ttl: u32) -> McSetFuture {
        McSetFuture { done: false }
    }

    pub fn add(&self, _key: &[u8], _value: &[u8], _flags: u32, _ttl: u32) -> McSetFuture {
        McSetFuture { done: false }
    }

    pub fn replace(&self, _key: &[u8], _value: &[u8], _flags: u32, _ttl: u32) -> McSetFuture {
        McSetFuture { done: false }
    }

    pub fn append(&self, _key: &[u8], _value: &[u8]) -> McAppendFuture {
        McAppendFuture { done: false }
    }

    pub fn prepend(&self, _key: &[u8], _value: &[u8]) -> McAppendFuture {
        McAppendFuture { done: false }
    }

    pub fn delete(&self, _key: &[u8]) -> McDeleteFuture {
        McDeleteFuture { done: false }
    }

    pub fn incr(&self, _key: &[u8], _value: u64) -> McIncrFuture {
        McIncrFuture { done: false }
    }

    pub fn decr(&self, _key: &[u8], _value: u64) -> McIncrFuture {
        McIncrFuture { done: false }
    }

    pub fn gets(&self, _keys: &[&[u8]]) -> McGetsFuture {
        McGetsFuture { done: false }
    }

    pub fn cas(&self, _key: &[u8], _value: &[u8], _flags: u32, _ttl: u32, _cas: u64) -> McCasFuture {
        McCasFuture { done: false }
    }

    pub fn stats(&self) -> McStatsFuture {
        McStatsFuture { done: false }
    }

    pub fn flush(&self, _delay: u32) -> McFlushFuture {
        McFlushFuture { done: false }
    }

    pub fn version(&self) -> McVersionFuture {
        McVersionFuture { done: false }
    }

    pub fn quit(&self) -> McQuitFuture {
        McQuitFuture { done: false }
    }
}

impl Default for MemcachedClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct McConnectFuture { done: bool }
impl Future for McConnectFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct McGetFuture { done: bool }
impl Future for McGetFuture { type Output = Result<Option<(Vec<u8>, u32)>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct McSetFuture { done: bool }
impl Future for McSetFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct McAppendFuture { done: bool }
impl Future for McAppendFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct McDeleteFuture { done: bool }
impl Future for McDeleteFuture { type Output = Result<bool, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct McIncrFuture { done: bool }
impl Future for McIncrFuture { type Output = Result<u64, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct McGetsFuture { done: bool }
impl Future for McGetsFuture { type Output = Result<Vec<(Vec<u8>, Vec<u8>, u32)>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct McCasFuture { done: bool }
impl Future for McCasFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct McStatsFuture { done: bool }
impl Future for McStatsFuture { type Output = Result<Vec<(Vec<u8>, Vec<u8>)>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct McFlushFuture { done: bool }
impl Future for McFlushFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct McVersionFuture { done: bool }
impl Future for McVersionFuture { type Output = Result<Vec<u8>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct McQuitFuture { done: bool }
impl Future for McQuitFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }