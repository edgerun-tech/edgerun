//! DNS client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub const DNS_MAX_NAME: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DnsType {
    A,
    AAAA,
    CNAME,
    MX,
    TXT,
    NS,
    SOA,
    PTR,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DnsResultCode {
    NoError,
    FormError,
    ServFail,
    NXDomain,
    NotImp,
    Refused,
}

pub struct DnsQuery {
    pub name: Vec<u8>,
    pub qtype: DnsType,
}

impl DnsQuery {
    pub fn new(name: &[u8], qtype: DnsType) -> Self {
        Self {
            name: name.to_vec(),
            qtype,
        }
    }
}

pub struct DnsResponse {
    pub result_code: DnsResultCode,
    pub answers: Vec<DnsRecord>,
}

pub struct DnsRecord {
    pub name: Vec<u8>,
    pub rtype: DnsType,
    pub ttl: u32,
    pub data: Vec<u8>,
}

pub struct DnsClient;

impl DnsClient {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve(&self, _name: &[u8], _qtype: DnsType) -> DnsResolveFuture {
        DnsResolveFuture { done: false }
    }
}

impl Default for DnsClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DnsResolveFuture {
    done: bool,
}

impl Future for DnsResolveFuture {
    type Output = Result<DnsResponse, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}
