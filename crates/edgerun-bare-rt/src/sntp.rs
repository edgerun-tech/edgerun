//! SNTP client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NtpMode {
    Client,
    Server,
    Symmetric,
    Broadcast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NtpStratum {
    Unspecified,
    Primary,
    Secondary,
}

pub struct NtpTimestamp {
    pub seconds: u32,
    pub fraction: u32,
}

impl NtpTimestamp {
    pub fn now() -> Self {
        Self { seconds: 0, fraction: 0 }
    }

    pub fn to_secs(&self) -> f64 {
        (self.seconds as f64) + (self.fraction as f64 / u32::MAX as f64)
    }
}

pub struct NtpPacket {
    pub version: u8,
    pub mode: NtpMode,
    pub stratum: NtpStratum,
    pub poll: i8,
    pub precision: i8,
    pub ref_id: [u8; 4],
    pub ref_timestamp: NtpTimestamp,
    pub orig_timestamp: NtpTimestamp,
    pub recv_timestamp: NtpTimestamp,
    pub trans_timestamp: NtpTimestamp,
}

impl NtpPacket {
    pub fn new() -> Self {
        Self {
            version: 4,
            mode: NtpMode::Client,
            stratum: NtpStratum::Unspecified,
            poll: 6,
            precision: -6,
            ref_id: [0; 4],
            ref_timestamp: NtpTimestamp::now(),
            orig_timestamp: NtpTimestamp::now(),
            recv_timestamp: NtpTimestamp::now(),
            trans_timestamp: NtpTimestamp::now(),
        }
    }
}

impl Default for NtpPacket {
    fn default() -> Self {
        Self::new()
    }
}

pub struct NtpClient;

impl NtpClient {
    pub fn new() -> Self {
        Self
    }

    pub fn query(&self, _server: &[u8]) -> NtpQueryFuture {
        NtpQueryFuture { done: false }
    }
}

impl Default for NtpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct NtpQueryFuture {
    done: bool,
}

impl Future for NtpQueryFuture {
    type Output = Result<NtpTimestamp, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}