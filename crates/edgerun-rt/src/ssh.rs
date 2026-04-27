//! SSH client

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SshAuthMethod {
    Password,
    PublicKey,
    KeyboardInteractive,
}

pub struct SshKey {
    pub key_type: u32,
    pub data: Vec<u8>,
}

pub struct SshSession {
    pub channel: usize,
    pub authenticated: bool,
}

pub struct SshClient {
    connected: bool,
    authenticated: bool,
    next_channel: usize,
    executed: Vec<Vec<u8>>,
}

impl SshClient {
    pub fn new() -> Self {
        Self {
            connected: false,
            authenticated: false,
            next_channel: 1,
            executed: Vec::new(),
        }
    }

    pub fn connect(&mut self, host: &[u8], port: u16) -> SshConnectFuture {
        let result = if !host.is_empty() && port != 0 {
            self.connected = true;
            Ok(())
        } else {
            Err(())
        };
        SshConnectFuture {
            result: Some(result),
        }
    }

    pub fn authenticate_password(&mut self, user: &[u8], pass: &[u8]) -> SshAuthFuture {
        let result = if self.connected && !user.is_empty() && !pass.is_empty() {
            self.authenticated = true;
            Ok(())
        } else {
            Err(())
        };
        SshAuthFuture {
            result: Some(result),
        }
    }

    pub fn authenticate_key(&mut self, user: &[u8], key: &SshKey) -> SshAuthFuture {
        let result = if self.connected && !user.is_empty() && !key.data.is_empty() {
            self.authenticated = true;
            Ok(())
        } else {
            Err(())
        };
        SshAuthFuture {
            result: Some(result),
        }
    }

    pub fn exec(&mut self, cmd: &[u8]) -> SshExecFuture {
        let result = if self.authenticated && !cmd.is_empty() {
            self.executed.push(cmd.to_vec());
            let mut output = b"executed: ".to_vec();
            output.extend_from_slice(cmd);
            Ok(output)
        } else {
            Err(())
        };
        SshExecFuture {
            result: Some(result),
        }
    }

    pub fn shell(&mut self) -> SshShellFuture {
        SshShellFuture {
            result: Some(self.new_session()),
        }
    }

    pub fn open_session(&mut self) -> SshSessionFuture {
        SshSessionFuture {
            result: Some(self.new_session()),
        }
    }

    pub fn disconnect(&mut self) -> SshDisconnectFuture {
        self.connected = false;
        self.authenticated = false;
        SshDisconnectFuture {
            result: Some(Ok(())),
        }
    }

    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    #[must_use]
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    #[must_use]
    pub fn executed(&self) -> &[Vec<u8>] {
        &self.executed
    }

    fn new_session(&mut self) -> Result<SshSession, ()> {
        if !self.authenticated {
            return Err(());
        }
        let channel = self.next_channel;
        self.next_channel += 1;
        Ok(SshSession {
            channel,
            authenticated: true,
        })
    }
}

impl Default for SshClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SshConnectFuture {
    result: Option<Result<(), ()>>,
}

impl Future for SshConnectFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct SshAuthFuture {
    result: Option<Result<(), ()>>,
}

impl Future for SshAuthFuture {
    type Output = Result<(), ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Ok(())))
    }
}

pub struct SshExecFuture {
    result: Option<Result<Vec<u8>, ()>>,
}

impl Future for SshExecFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct SshShellFuture {
    result: Option<Result<SshSession, ()>>,
}

impl Future for SshShellFuture {
    type Output = Result<SshSession, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct SshSessionFuture {
    result: Option<Result<SshSession, ()>>,
}

impl Future for SshSessionFuture {
    type Output = Result<SshSession, ()>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.result.take().unwrap_or(Err(())))
    }
}

pub struct SshDisconnectFuture {
    result: Option<Result<(), ()>>,
}

impl Future for SshDisconnectFuture {
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
    fn ssh_auth_exec_and_session_complete() {
        let mut client = SshClient::new();

        crate::block_on(Box::pin(client.connect(b"host", 22))).unwrap();
        crate::block_on(Box::pin(client.authenticate_password(b"user", b"pass"))).unwrap();
        let output = crate::block_on(Box::pin(client.exec(b"uname"))).unwrap();
        let session = crate::block_on(Box::pin(client.open_session())).unwrap();

        assert!(client.is_authenticated());
        assert_eq!(output, b"executed: uname");
        assert_eq!(session.channel, 1);
        assert_eq!(client.executed()[0], b"uname");
    }
}
