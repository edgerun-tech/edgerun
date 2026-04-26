//! DHCP client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub const DHCP_MAX_LEASES: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DhcpState {
    Selecting,
    Requesting,
    Bound,
    Renewing,
    Rebinding,
    Releasing,
}

pub struct DhcpLease {
    pub address: [u8; 4],
    pub router: [u8; 4],
    pub subnet: [u8; 4],
    pub dns: [[u8; 4]; 2],
    pub lease_time: u32,
    pub renew_time: u32,
    pub rebind_time: u32,
}

impl DhcpLease {
    pub fn new() -> Self {
        Self {
            address: [0; 4],
            router: [0; 4],
            subnet: [0; 4],
            dns: [[0; 4]; 2],
            lease_time: 0,
            renew_time: 0,
            rebind_time: 0,
        }
    }
}

pub struct DhcpMessage {
    pub op: u8,
    pub htype: u8,
    pub hlen: u8,
    pub hops: u8,
    pub xid: u32,
    pub ciaddr: [u8; 4],
    pub yiaddr: [u8; 4],
    pub siaddr: [u8; 4],
    pub giaddr: [u8; 4],
    pub chaddr: [u8; 16],
    pub options: Vec<u8>,
}

pub struct DhcpClient;

impl DhcpClient {
    pub fn new() -> Self {
        Self
    }

    pub fn discover(&self) -> DhcpDiscoverFuture {
        DhcpDiscoverFuture { done: false }
    }

    pub fn request(&self, _lease: &DhcpLease) -> DhcpRequestFuture {
        DhcpRequestFuture { done: false }
    }

    pub fn renew(&self, _lease: &mut DhcpLease) -> DhcpRenewFuture {
        DhcpRenewFuture { done: false }
    }

    pub fn release(&self, _lease: &DhcpLease) -> DhcpReleaseFuture {
        DhcpReleaseFuture { done: false }
    }
}

impl Default for DhcpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DhcpDiscoverFuture {
    done: bool,
}

impl Future for DhcpDiscoverFuture {
    type Output = Result<DhcpLease, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct DhcpRequestFuture {
    done: bool,
}

impl Future for DhcpRequestFuture {
    type Output = Result<DhcpLease, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct DhcpRenewFuture {
    done: bool,
}

impl Future for DhcpRenewFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct DhcpReleaseFuture {
    done: bool,
}

impl Future for DhcpReleaseFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}