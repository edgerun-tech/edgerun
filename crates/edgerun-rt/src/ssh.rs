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

pub struct SshClient;

impl SshClient {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&self, _host: &[u8], _port: u16) -> SshConnectFuture {
        SshConnectFuture { done: false }
    }

    pub fn authenticate_password(&self, _user: &[u8], _pass: &[u8]) -> SshAuthFuture {
        SshAuthFuture { done: false }
    }

    pub fn authenticate_key(&self, _user: &[u8], _key: &SshKey) -> SshAuthFuture {
        SshAuthFuture { done: false }
    }

    pub fn exec(&self, _cmd: &[u8]) -> SshExecFuture {
        SshExecFuture { done: false }
    }

    pub fn shell(&self) -> SshShellFuture {
        SshShellFuture { done: false }
    }

    pub fn open_session(&self) -> SshSessionFuture {
        SshSessionFuture { done: false }
    }

    pub fn disconnect(&self) -> SshDisconnectFuture {
        SshDisconnectFuture { done: false }
    }
}

impl Default for SshClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SshConnectFuture {
    done: bool,
}

impl Future for SshConnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SshAuthFuture {
    done: bool,
}

impl Future for SshAuthFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SshExecFuture {
    done: bool,
}

impl Future for SshExecFuture {
    type Output = Result<Vec<u8>, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SshShellFuture {
    done: bool,
}

impl Future for SshShellFuture {
    type Output = Result<SshSession, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SshSessionFuture {
    done: bool,
}

impl Future for SshSessionFuture {
    type Output = Result<SshSession, ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

pub struct SshDisconnectFuture {
    done: bool,
}

impl Future for SshDisconnectFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}