//! TcpSocket — builder-pattern for TCP connections (no_std stub).


extern crate alloc;

use alloc::sync::Arc;
use alloc::boxed::Box;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::io_traits::Error;
use crate::tcp::{AsyncTcpListener, AsyncTcpStream};

pub struct TcpSocket {
    domain: isize,
    reuse_addr: bool,
    reuse_port: bool,
    ttl: Option<u32>,
    nodelay: bool,
    send_buffer_size: Option<usize>,
    recv_buffer_size: Option<usize>,
}

impl TcpSocket {
    pub fn new_v4() -> Result<Self, Error> {
        Ok(Self {
            domain: 2,
            reuse_addr: true,
            reuse_port: false,
            ttl: None,
            nodelay: false,
            send_buffer_size: None,
            recv_buffer_size: None,
        })
    }

    pub fn new_v6() -> Result<Self, Error> {
        Ok(Self {
            domain: 10,
            reuse_addr: true,
            reuse_port: false,
            ttl: None,
            nodelay: false,
            send_buffer_size: None,
            recv_buffer_size: None,
        })
    }

    pub fn set_reuseaddr(&mut self, value: bool) -> &mut Self {
        self.reuse_addr = value;
        self
    }

    pub fn set_reuseport(&mut self, value: bool) -> &mut Self {
        self.reuse_port = value;
        self
    }

    pub fn set_ttl(&mut self, ttl: u32) -> &mut Self {
        self.ttl = Some(ttl);
        self
    }

    pub fn set_nodelay(&mut self, nodelay: bool) -> &mut Self {
        self.nodelay = nodelay;
        self
    }

    pub fn set_send_buffer_size(&mut self, size: usize) -> &mut Self {
        self.send_buffer_size = Some(size);
        self
    }

    pub fn set_recv_buffer_size(&mut self, size: usize) -> &mut Self {
        self.recv_buffer_size = Some(size);
        self
    }

    pub fn bind(self, _addr: crate::tcp::SocketAddr) -> Result<AsyncTcpListener, Error> {
        Err(Error::new(crate::io_traits::ErrorKind::Other))
    }

    pub async fn connect(self, _addr: crate::tcp::SocketAddr) -> Result<Arc<AsyncTcpStream>, Error> {
        Err(Error::new(crate::io_traits::ErrorKind::Other))
    }
}