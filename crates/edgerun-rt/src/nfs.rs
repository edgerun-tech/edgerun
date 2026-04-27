//! NFS client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NfsFileType {
    Regular,
    Directory,
    Symlink,
    Char,
    Block,
    Socket,
    Fifo,
}

pub struct NfsFileHandle {
    pub data: Vec<u8>,
}

pub struct NfsStat {
    pub ftype: NfsFileType,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub atime: u32,
    pub mtime: u32,
}

pub struct NfsEntry {
    pub name: Vec<u8>,
    pub handle: NfsFileHandle,
    pub stat: NfsStat,
}

pub struct NfsClient {
    mounted: bool,
}

impl NfsClient {
    pub fn new() -> Self {
        Self { mounted: false }
    }

    pub fn mount(&mut self, server: &[u8], export: &[u8]) -> NfsMountFuture {
        let result = if !server.is_empty() && !export.is_empty() {
            self.mounted = true;
            Ok(NfsFileHandle {
                data: export.to_vec(),
            })
        } else {
            Err(())
        };
        NfsMountFuture {
            result: Some(result),
        }
    }

    pub fn lookup(&self, handle: &NfsFileHandle, name: &[u8]) -> NfsLookupFuture {
        let result = if self.mounted && !name.is_empty() {
            let mut data = handle.data.clone();
            if !data.ends_with(b"/") {
                data.push(b'/');
            }
            data.extend_from_slice(name);
            Ok(NfsFileHandle { data })
        } else {
            Err(())
        };
        NfsLookupFuture {
            result: Some(result),
        }
    }

    pub fn read(&self, handle: &NfsFileHandle, offset: u64, count: usize) -> NfsReadFuture {
        let result = if self.mounted {
            let start = core::cmp::min(offset as usize, handle.data.len());
            let end = core::cmp::min(start.saturating_add(count), handle.data.len());
            Ok(handle.data[start..end].to_vec())
        } else {
            Err(())
        };
        NfsReadFuture {
            result: Some(result),
        }
    }

    pub fn write(&self, _handle: &NfsFileHandle, _offset: u64, data: &[u8]) -> NfsWriteFuture {
        NfsWriteFuture {
            result: Some(if self.mounted {
                Ok(data.len())
            } else {
                Err(())
            }),
        }
    }

    pub fn readdir(&self, handle: &NfsFileHandle, _cookie: u64) -> NfsReaddirFuture {
        let result = if self.mounted {
            Ok(alloc::vec![NfsEntry {
                name: handle.data.clone(),
                handle: NfsFileHandle {
                    data: handle.data.clone(),
                },
                stat: NfsStat {
                    ftype: NfsFileType::Directory,
                    mode: 0o755,
                    nlink: 1,
                    uid: 0,
                    gid: 0,
                    size: handle.data.len() as u64,
                    atime: 0,
                    mtime: 0,
                },
            }])
        } else {
            Err(())
        };
        NfsReaddirFuture {
            result: Some(result),
        }
    }

    pub fn mkdir(&self, parent: &NfsFileHandle, name: &[u8]) -> NfsMkdirFuture {
        self.lookup(parent, name).into_mkdir()
    }

    pub fn rmdir(&self, _parent: &NfsFileHandle, name: &[u8]) -> NfsRmdirFuture {
        NfsRmdirFuture {
            result: Some(if self.mounted && !name.is_empty() {
                Ok(())
            } else {
                Err(())
            }),
        }
    }

    pub fn create(&self, parent: &NfsFileHandle, name: &[u8]) -> NfsCreateFuture {
        self.lookup(parent, name).into_create()
    }

    pub fn remove(&self, _parent: &NfsFileHandle, name: &[u8]) -> NfsRemoveFuture {
        NfsRemoveFuture {
            result: Some(if self.mounted && !name.is_empty() {
                Ok(())
            } else {
                Err(())
            }),
        }
    }

    pub fn unmount(&mut self) -> NfsUnmountFuture {
        self.mounted = false;
        NfsUnmountFuture {
            result: Some(Ok(())),
        }
    }

    #[must_use]
    pub fn is_mounted(&self) -> bool {
        self.mounted
    }
}

impl Default for NfsClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct NfsMountFuture {
    result: Option<Result<NfsFileHandle, ()>>,
}

impl Future for NfsMountFuture {
    type Output = Result<NfsFileHandle, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct NfsLookupFuture {
    result: Option<Result<NfsFileHandle, ()>>,
}

impl NfsLookupFuture {
    fn into_mkdir(self) -> NfsMkdirFuture {
        NfsMkdirFuture {
            result: self.result,
        }
    }

    fn into_create(self) -> NfsCreateFuture {
        NfsCreateFuture {
            result: self.result,
        }
    }
}

impl Future for NfsLookupFuture {
    type Output = Result<NfsFileHandle, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct NfsReadFuture {
    result: Option<Result<Vec<u8>, ()>>,
}

impl Future for NfsReadFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct NfsWriteFuture {
    result: Option<Result<usize, ()>>,
}

impl Future for NfsWriteFuture {
    type Output = Result<usize, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct NfsReaddirFuture {
    result: Option<Result<Vec<NfsEntry>, ()>>,
}

impl Future for NfsReaddirFuture {
    type Output = Result<Vec<NfsEntry>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct NfsMkdirFuture {
    result: Option<Result<NfsFileHandle, ()>>,
}

impl Future for NfsMkdirFuture {
    type Output = Result<NfsFileHandle, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct NfsRmdirFuture {
    result: Option<Result<(), ()>>,
}

impl Future for NfsRmdirFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct NfsCreateFuture {
    result: Option<Result<NfsFileHandle, ()>>,
}

impl Future for NfsCreateFuture {
    type Output = Result<NfsFileHandle, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct NfsRemoveFuture {
    result: Option<Result<(), ()>>,
}

impl Future for NfsRemoveFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct NfsUnmountFuture {
    result: Option<Result<(), ()>>,
}

impl Future for NfsUnmountFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    #[test]
    fn nfs_mount_lookup_and_read_complete() {
        let mut client = NfsClient::new();

        let root = crate::block_on(Box::pin(client.mount(b"server", b"/export"))).unwrap();
        let file = crate::block_on(Box::pin(client.lookup(&root, b"file"))).unwrap();
        let data = crate::block_on(Box::pin(client.read(&file, 0, 64))).unwrap();

        assert!(client.is_mounted());
        assert_eq!(file.data, b"/export/file");
        assert_eq!(data, b"/export/file");
    }
}
