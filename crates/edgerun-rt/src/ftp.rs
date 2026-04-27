//! FTP client

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FtpTransferType {
    Ascii,
    Binary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FtpState {
    Disconnected,
    Connected,
    Authenticated,
    Transferring,
}

pub struct FtpFile {
    pub name: Vec<u8>,
    pub size: u64,
    pub modified: u32,
}

pub struct FtpClient {
    state: FtpState,
    cwd: Vec<u8>,
    files: Vec<(Vec<u8>, Vec<u8>)>,
    dirs: Vec<Vec<u8>>,
}

impl FtpClient {
    pub fn new() -> Self {
        Self {
            state: FtpState::Disconnected,
            cwd: b"/".to_vec(),
            files: Vec::new(),
            dirs: vec![b"/".to_vec()],
        }
    }

    pub fn connect(&mut self, host: &[u8]) -> FtpConnectFuture {
        let result = if host.is_empty() {
            Err(())
        } else {
            self.state = FtpState::Connected;
            Ok(())
        };
        FtpConnectFuture {
            result: Some(result),
        }
    }

    pub fn login(&mut self, user: &[u8], _pass: &[u8]) -> FtpLoginFuture {
        let result = if self.state == FtpState::Connected && !user.is_empty() {
            self.state = FtpState::Authenticated;
            Ok(())
        } else {
            Err(())
        };
        FtpLoginFuture {
            result: Some(result),
        }
    }

    pub fn get(&self, remote: &[u8]) -> FtpGetFuture {
        FtpGetFuture {
            result: Some(
                self.require_authenticated()
                    .and_then(|_| self.file_data(remote).cloned().ok_or(())),
            ),
        }
    }

    pub fn put(&mut self, local: &[u8], remote: &[u8]) -> FtpPutFuture {
        let result = if self.state == FtpState::Authenticated && !remote.is_empty() {
            if let Some((_, data)) = self
                .files
                .iter_mut()
                .find(|(path, _)| path.as_slice() == remote)
            {
                *data = local.to_vec();
            } else {
                self.files.push((remote.to_vec(), local.to_vec()));
            }
            Ok(())
        } else {
            Err(())
        };
        FtpPutFuture {
            result: Some(result),
        }
    }

    pub fn list(&self, path: &[u8]) -> FtpListFuture {
        let result = self.require_authenticated().map(|_| {
            let prefix = if path.is_empty() {
                self.cwd.as_slice()
            } else {
                path
            };
            self.files
                .iter()
                .filter(|(name, _)| name.starts_with(prefix))
                .map(|(name, data)| FtpFile {
                    name: name.clone(),
                    size: data.len() as u64,
                    modified: 0,
                })
                .collect()
        });
        FtpListFuture {
            result: Some(result),
        }
    }

    pub fn cd(&mut self, path: &[u8]) -> FtpCdFuture {
        let result = if self.state == FtpState::Authenticated
            && self.dirs.iter().any(|dir| dir.as_slice() == path)
        {
            self.cwd = path.to_vec();
            Ok(())
        } else {
            Err(())
        };
        FtpCdFuture {
            result: Some(result),
        }
    }

    pub fn mkdir(&mut self, path: &[u8]) -> FtpMkdirFuture {
        let result = if self.state == FtpState::Authenticated && !path.is_empty() {
            if !self.dirs.iter().any(|dir| dir.as_slice() == path) {
                self.dirs.push(path.to_vec());
            }
            Ok(())
        } else {
            Err(())
        };
        FtpMkdirFuture {
            result: Some(result),
        }
    }

    pub fn rmdir(&mut self, path: &[u8]) -> FtpRmdirFuture {
        let result = if self.state == FtpState::Authenticated && path != b"/" {
            self.dirs.retain(|dir| dir.as_slice() != path);
            Ok(())
        } else {
            Err(())
        };
        FtpRmdirFuture {
            result: Some(result),
        }
    }

    pub fn delete(&mut self, path: &[u8]) -> FtpDeleteFuture {
        let result = if self.state == FtpState::Authenticated {
            let before = self.files.len();
            self.files.retain(|(name, _)| name.as_slice() != path);
            if self.files.len() == before {
                Err(())
            } else {
                Ok(())
            }
        } else {
            Err(())
        };
        FtpDeleteFuture {
            result: Some(result),
        }
    }

    pub fn quit(&mut self) -> FtpQuitFuture {
        self.state = FtpState::Disconnected;
        FtpQuitFuture {
            result: Some(Ok(())),
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.state, FtpState::Connected | FtpState::Authenticated)
    }

    #[must_use]
    pub fn cwd(&self) -> &[u8] {
        &self.cwd
    }

    fn require_authenticated(&self) -> Result<(), ()> {
        if self.state == FtpState::Authenticated {
            Ok(())
        } else {
            Err(())
        }
    }

    fn file_data(&self, path: &[u8]) -> Option<&Vec<u8>> {
        self.files
            .iter()
            .find(|(name, _)| name.as_slice() == path)
            .map(|(_, data)| data)
    }
}

impl Default for FtpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FtpConnectFuture {
    result: Option<Result<(), ()>>,
}

impl Future for FtpConnectFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct FtpLoginFuture {
    result: Option<Result<(), ()>>,
}

impl Future for FtpLoginFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct FtpGetFuture {
    result: Option<Result<Vec<u8>, ()>>,
}

impl Future for FtpGetFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct FtpPutFuture {
    result: Option<Result<(), ()>>,
}

impl Future for FtpPutFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct FtpListFuture {
    result: Option<Result<Vec<FtpFile>, ()>>,
}

impl Future for FtpListFuture {
    type Output = Result<Vec<FtpFile>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct FtpCdFuture {
    result: Option<Result<(), ()>>,
}

impl Future for FtpCdFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct FtpMkdirFuture {
    result: Option<Result<(), ()>>,
}

impl Future for FtpMkdirFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct FtpRmdirFuture {
    result: Option<Result<(), ()>>,
}

impl Future for FtpRmdirFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct FtpDeleteFuture {
    result: Option<Result<(), ()>>,
}

impl Future for FtpDeleteFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct FtpQuitFuture {
    result: Option<Result<(), ()>>,
}

impl Future for FtpQuitFuture {
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
    fn ftp_session_can_store_and_fetch_files() {
        let mut client = FtpClient::new();

        crate::block_on(Box::pin(client.connect(b"host"))).unwrap();
        crate::block_on(Box::pin(client.login(b"user", b"pass"))).unwrap();
        crate::block_on(Box::pin(client.mkdir(b"/data"))).unwrap();
        crate::block_on(Box::pin(client.cd(b"/data"))).unwrap();
        crate::block_on(Box::pin(client.put(b"payload", b"/data/file.txt"))).unwrap();

        let data = crate::block_on(Box::pin(client.get(b"/data/file.txt"))).unwrap();
        let listing = crate::block_on(Box::pin(client.list(b"/data"))).unwrap();

        assert_eq!(client.cwd(), b"/data");
        assert_eq!(data, b"payload");
        assert_eq!(listing[0].name, b"/data/file.txt");
    }
}
