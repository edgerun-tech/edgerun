//! KNX client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub const KNX_MAX_GA: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnxDpt {
    Dpt1_001,  // Switch
    Dpt2_001,  // Enable
    Dpt3_007,  // Blinds
    Dpt4_001,  // Char
    Dpt5_001,  // Percentage
    Dpt5_003,  // Angle
    Dpt6_001,  // Signed
    Dpt9_001,  // 2-byte float
    Dpt10_001, // Time
    Dpt11_001, // Date
    Dpt13_001, // 4-byte signed
}

pub struct KnxAddress {
    pub area: u8,
    pub line: u8,
    pub member: u8,
}

impl KnxAddress {
    pub fn new(area: u8, line: u8, member: u8) -> Self {
        Self { area, line, member }
    }

    pub fn to_bytes(&self) -> [u8; 2] {
        let hi = (self.area << 3) | (self.line >> 3);
        let lo = (self.line << 5) | self.member;
        [hi, lo]
    }
}

pub struct KnxGroupAddress {
    pub main: u8,
    pub middle: u8,
    pub sub: u8,
}

impl KnxGroupAddress {
    pub fn new(main: u8, middle: u8, sub: u8) -> Self {
        Self { main, middle, sub }
    }

    pub fn to_u16(&self) -> u16 {
        ((self.main as u16) << 8) | ((self.middle as u16) << 4) | (self.sub as u16)
    }
}

pub struct KnxTelegram {
    pub source: KnxAddress,
    pub dest: KnxGroupAddress,
    pub dpt: KnxDpt,
    pub data: Vec<u8>,
}

pub struct KnxClient;

impl KnxClient {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&self, _host: &[u8], _port: u16) -> KnxConnectFuture {
        KnxConnectFuture { done: false }
    }

    pub fn read(&self, _ga: KnxGroupAddress) -> KnxReadFuture {
        KnxReadFuture { done: false }
    }

    pub fn write(&self, _ga: KnxGroupAddress, _dpt: KnxDpt, _data: &[u8]) -> KnxWriteFuture {
        KnxWriteFuture { done: false }
    }

    pub fn callback(&self, _ga: KnxGroupAddress, _handler: fn(&[u8])) {
    }
}

impl Default for KnxClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct KnxConnectFuture {
    done: bool,
}

impl Future for KnxConnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct KnxReadFuture {
    done: bool,
}

impl Future for KnxReadFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct KnxWriteFuture {
    done: bool,
}

impl Future for KnxWriteFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}