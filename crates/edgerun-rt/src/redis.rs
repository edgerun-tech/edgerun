//! Redis client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedisType {
    String,
    List,
    Set,
    Zset,
    Hash,
    Stream,
}

pub struct RedisValue {
    pub rtype: RedisType,
    pub data: Vec<u8>,
}

impl RedisValue {
    pub fn string(data: Vec<u8>) -> Self {
        Self { rtype: RedisType::String, data }
    }
    pub fn is_string(&self) -> bool {
        matches!(self.rtype, RedisType::String)
    }
}

pub struct RedisClient;

impl RedisClient {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&self, _host: &[u8], _port: u16) -> RedisConnectFuture {
        RedisConnectFuture { done: false }
    }

    pub fn auth(&self, _pass: &[u8]) -> RedisAuthFuture {
        RedisAuthFuture { done: false }
    }

    pub fn get(&self, _key: &[u8]) -> RedisGetFuture {
        RedisGetFuture { done: false }
    }

    pub fn set(&self, _key: &[u8], _value: &[u8]) -> RedisSetFuture {
        RedisSetFuture { done: false }
    }

    pub fn set_ex(&self, _key: &[u8], _value: &[u8], _ttl: u64) -> RedisSetFuture {
        RedisSetFuture { done: false }
    }

    pub fn del(&self, _keys: &[&[u8]]) -> RedisDelFuture {
        RedisDelFuture { done: false }
    }

    pub fn exists(&self, _key: &[u8]) -> RedisExistsFuture {
        RedisExistsFuture { done: false }
    }

    pub fn expire(&self, _key: &[u8], _ttl: u64) -> RedisExpireFuture {
        RedisExpireFuture { done: false }
    }

    pub fn ttl(&self, _key: &[u8]) -> RedisTtlFuture {
        RedisTtlFuture { done: false }
    }

    pub fn incr(&self, _key: &[u8]) -> RedisIncrFuture {
        RedisIncrFuture { done: false }
    }

    pub fn decr(&self, _key: &[u8]) -> RedisIncrFuture {
        RedisIncrFuture { done: false }
    }

    pub fn lpush(&self, _key: &[u8], _value: &[u8]) -> RedisLpushFuture {
        RedisLpushFuture { done: false }
    }

    pub fn rpush(&self, _key: &[u8], _value: &[u8]) -> RedisLpushFuture {
        RedisLpushFuture { done: false }
    }

    pub fn lpop(&self, _key: &[u8]) -> RedisLpopFuture {
        RedisLpopFuture { done: false }
    }

    pub fn lrange(&self, _key: &[u8], _start: i64, _stop: i64) -> RedisLrangeFuture {
        RedisLrangeFuture { done: false }
    }

    pub fn sadd(&self, _key: &[u8], _member: &[u8]) -> RedisSaddFuture {
        RedisSaddFuture { done: false }
    }

    pub fn smembers(&self, _key: &[u8]) -> RedisSmembersFuture {
        RedisSmembersFuture { done: false }
    }

    pub fn hset(&self, _key: &[u8], _field: &[u8], _value: &[u8]) -> RedisHsetFuture {
        RedisHsetFuture { done: false }
    }

    pub fn hget(&self, _key: &[u8], _field: &[u8]) -> RedisHgetFuture {
        RedisHgetFuture { done: false }
    }

    pub fn hgetall(&self, _key: &[u8]) -> RedisHgetallFuture {
        RedisHgetallFuture { done: false }
    }

    pub fn ping(&self) -> RedisPingFuture {
        RedisPingFuture { done: false }
    }

    pub fn quit(&self) -> RedisQuitFuture {
        RedisQuitFuture { done: false }
    }
}

impl Default for RedisClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RedisConnectFuture { done: bool }
impl Future for RedisConnectFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisAuthFuture { done: bool }
impl Future for RedisAuthFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisGetFuture { done: bool }
impl Future for RedisGetFuture { type Output = Result<RedisValue, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisSetFuture { done: bool }
impl Future for RedisSetFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisDelFuture { done: bool }
impl Future for RedisDelFuture { type Output = Result<u64, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisExistsFuture { done: bool }
impl Future for RedisExistsFuture { type Output = Result<bool, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisExpireFuture { done: bool }
impl Future for RedisExpireFuture { type Output = Result<bool, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisTtlFuture { done: bool }
impl Future for RedisTtlFuture { type Output = Result<i64, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisIncrFuture { done: bool }
impl Future for RedisIncrFuture { type Output = Result<i64, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisLpushFuture { done: bool }
impl Future for RedisLpushFuture { type Output = Result<u64, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisLpopFuture { done: bool }
impl Future for RedisLpopFuture { type Output = Result<RedisValue, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisLrangeFuture { done: bool }
impl Future for RedisLrangeFuture { type Output = Result<Vec<RedisValue>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisSaddFuture { done: bool }
impl Future for RedisSaddFuture { type Output = Result<u64, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisSmembersFuture { done: bool }
impl Future for RedisSmembersFuture { type Output = Result<Vec<RedisValue>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisHsetFuture { done: bool }
impl Future for RedisHsetFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisHgetFuture { done: bool }
impl Future for RedisHgetFuture { type Output = Result<RedisValue, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisHgetallFuture { done: bool }
impl Future for RedisHgetallFuture { type Output = Result<Vec<RedisValue>, ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisPingFuture { done: bool }
impl Future for RedisPingFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }
pub struct RedisQuitFuture { done: bool }
impl Future for RedisQuitFuture { type Output = Result<(), ()>; fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> { Poll::Pending } }