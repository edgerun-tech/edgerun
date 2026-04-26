//! Storage driver for block devices

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub const MAX_DEVICES: usize = 8;
pub const SECTOR_SIZE: usize = 512;

pub struct BlockDevice {
    pub id: usize,
    pub size_sectors: u64,
    pub readonly: bool,
}

impl BlockDevice {
    pub fn new(id: usize, size_sectors: u64, readonly: bool) -> Self {
        Self { id, size_sectors, readonly }
    }

    pub fn read_sectors(&self, _sector: u64, _count: usize) -> ReadSectorFuture {
        ReadSectorFuture { done: false }
    }

    pub fn write_sectors(&self, _sector: u64, _data: &[u8]) -> WriteSectorFuture {
        WriteSectorFuture { done: false }
    }
}

pub struct ReadSectorFuture {
    done: bool,
}

impl Future for ReadSectorFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct WriteSectorFuture {
    done: bool,
}

impl Future for WriteSectorFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct StorageDevice;

impl StorageDevice {
    pub fn new() -> Self {
        Self
    }

    pub fn devices(&self) -> usize {
        0
    }
}

impl Default for StorageDevice {
    fn default() -> Self {
        Self::new()
    }
}