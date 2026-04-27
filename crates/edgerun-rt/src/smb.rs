//! SMB client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmbFileMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

pub struct SmbFile {
    pub name: Vec<u8>,
    pub size: u64,
    pub created: u32,
    pub modified: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmbShare {
    pub name: Vec<u8>,
    pub comment: Vec<u8>,
}

pub struct SmbClient {
    connected: bool,
    authenticated: bool,
    shares: Vec<SmbShare>,
    files: Vec<(usize, Vec<u8>, Vec<u8>)>,
    next_handle: usize,
}

impl SmbClient {
    pub fn new() -> Self {
        Self {
            connected: false,
            authenticated: false,
            shares: alloc::vec![SmbShare {
                name: b"edgerun".to_vec(),
                comment: b"default share".to_vec(),
            }],
            files: Vec::new(),
            next_handle: 1,
        }
    }

    pub fn connect(&mut self, host: &[u8]) -> SmbConnectFuture {
        let result = if host.is_empty() {
            Err(())
        } else {
            self.connected = true;
            Ok(())
        };
        SmbConnectFuture {
            result: Some(result),
        }
    }

    pub fn login(&mut self, user: &[u8], _pass: &[u8], _domain: &[u8]) -> SmbLoginFuture {
        let result = if self.connected && !user.is_empty() {
            self.authenticated = true;
            Ok(())
        } else {
            Err(())
        };
        SmbLoginFuture {
            result: Some(result),
        }
    }

    pub fn shares(&self) -> SmbSharesFuture {
        SmbSharesFuture {
            result: Some(if self.authenticated {
                Ok(self.shares.clone())
            } else {
                Err(())
            }),
        }
    }

    pub fn open(&mut self, share: &[u8], path: &[u8], _mode: SmbFileMode) -> SmbOpenFuture {
        let result = if self.authenticated
            && !path.is_empty()
            && self.shares.iter().any(|s| s.name.as_slice() == share)
        {
            if let Some((handle, _, _)) = self.files.iter().find(|(_, p, _)| p.as_slice() == path) {
                Ok(*handle)
            } else {
                let handle = self.next_handle;
                self.next_handle += 1;
                self.files.push((handle, path.to_vec(), Vec::new()));
                Ok(handle)
            }
        } else {
            Err(())
        };
        SmbOpenFuture {
            result: Some(result),
        }
    }

    pub fn read(&self, fh: usize, offset: u64, len: usize) -> SmbReadFuture {
        let result = if !self.authenticated {
            Err(())
        } else {
            self.files
                .iter()
                .find(|(handle, _, _)| *handle == fh)
                .map(|(_, _, data)| {
                    let start = core::cmp::min(offset as usize, data.len());
                    let end = core::cmp::min(start.saturating_add(len), data.len());
                    data[start..end].to_vec()
                })
                .ok_or(())
        };
        SmbReadFuture {
            result: Some(result),
        }
    }

    pub fn write(&mut self, fh: usize, offset: u64, data: &[u8]) -> SmbWriteFuture {
        let result = if !self.authenticated {
            Err(())
        } else if let Some((_, _, stored)) =
            self.files.iter_mut().find(|(handle, _, _)| *handle == fh)
        {
            let start = offset as usize;
            if stored.len() < start {
                stored.resize(start, 0);
            }
            if stored.len() < start + data.len() {
                stored.resize(start + data.len(), 0);
            }
            stored[start..start + data.len()].copy_from_slice(data);
            Ok(data.len())
        } else {
            Err(())
        };
        SmbWriteFuture {
            result: Some(result),
        }
    }

    pub fn mkdir(&self, _share: &[u8], _path: &[u8]) -> SmbMkdirFuture {
        SmbMkdirFuture {
            result: Some(if self.authenticated { Ok(()) } else { Err(()) }),
        }
    }

    pub fn rmdir(&self, _share: &[u8], _path: &[u8]) -> SmbRmdirFuture {
        SmbRmdirFuture {
            result: Some(if self.authenticated { Ok(()) } else { Err(()) }),
        }
    }

    pub fn unlink(&mut self, _share: &[u8], path: &[u8]) -> SmbUnlinkFuture {
        let result = if self.authenticated {
            let before = self.files.len();
            self.files.retain(|(_, p, _)| p.as_slice() != path);
            if before == self.files.len() {
                Err(())
            } else {
                Ok(())
            }
        } else {
            Err(())
        };
        SmbUnlinkFuture {
            result: Some(result),
        }
    }

    pub fn close(&self, _fh: usize) -> SmbCloseFuture {
        SmbCloseFuture {
            result: Some(if self.authenticated { Ok(()) } else { Err(()) }),
        }
    }

    pub fn disconnect(&mut self) -> SmbDisconnectFuture {
        self.connected = false;
        self.authenticated = false;
        SmbDisconnectFuture {
            result: Some(Ok(())),
        }
    }

    #[must_use]
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }
}

impl Default for SmbClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SmbConnectFuture {
    result: Option<Result<(), ()>>,
}

impl Future for SmbConnectFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct SmbLoginFuture {
    result: Option<Result<(), ()>>,
}

impl Future for SmbLoginFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct SmbSharesFuture {
    result: Option<Result<Vec<SmbShare>, ()>>,
}

impl Future for SmbSharesFuture {
    type Output = Result<Vec<SmbShare>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct SmbOpenFuture {
    result: Option<Result<usize, ()>>,
}

impl Future for SmbOpenFuture {
    type Output = Result<usize, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct SmbReadFuture {
    result: Option<Result<Vec<u8>, ()>>,
}

impl Future for SmbReadFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct SmbWriteFuture {
    result: Option<Result<usize, ()>>,
}

impl Future for SmbWriteFuture {
    type Output = Result<usize, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct SmbMkdirFuture {
    result: Option<Result<(), ()>>,
}

impl Future for SmbMkdirFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct SmbRmdirFuture {
    result: Option<Result<(), ()>>,
}

impl Future for SmbRmdirFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct SmbUnlinkFuture {
    result: Option<Result<(), ()>>,
}

impl Future for SmbUnlinkFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct SmbCloseFuture {
    result: Option<Result<(), ()>>,
}

impl Future for SmbCloseFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct SmbDisconnectFuture {
    result: Option<Result<(), ()>>,
}

impl Future for SmbDisconnectFuture {
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
    fn smb_open_write_read_unlink_complete() {
        let mut client = SmbClient::new();

        crate::block_on(Box::pin(client.connect(b"host"))).unwrap();
        crate::block_on(Box::pin(client.login(b"user", b"pass", b"domain"))).unwrap();
        let share = crate::block_on(Box::pin(client.shares())).unwrap();
        let handle = crate::block_on(Box::pin(client.open(
            &share[0].name,
            b"/file.txt",
            SmbFileMode::ReadWrite,
        )))
        .unwrap();
        crate::block_on(Box::pin(client.write(handle, 0, b"payload"))).unwrap();
        let data = crate::block_on(Box::pin(client.read(handle, 0, 64))).unwrap();
        crate::block_on(Box::pin(client.unlink(&share[0].name, b"/file.txt"))).unwrap();

        assert!(client.is_authenticated());
        assert_eq!(data, b"payload");
    }
}
