use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::rt::{AsyncRead, AsyncWrite};

use super::types::{TransportAddress, TransportError};

pub trait NodeStream: AsyncRead + AsyncWrite {
    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError>;
    fn peer_transport_addr(&self) -> Result<TransportAddress, TransportError>;
}

pub trait NodeStreamListener {
    type Stream: NodeStream;

    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError>;
    fn poll_accept_stream(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(Self::Stream, TransportAddress), TransportError>>;
}

pub trait NodeDatagram {
    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError>;
    fn peer_transport_addr(&self) -> Result<Option<TransportAddress>, TransportError>;
    fn poll_recv_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        payload: &mut [u8],
    ) -> Poll<Result<(usize, TransportAddress), TransportError>>;
    fn poll_send_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        payload: &[u8],
        destination: &TransportAddress,
    ) -> Poll<Result<usize, TransportError>>;
}

pub trait RuntimeTransport {
    type Stream: NodeStream;
    type Listener: NodeStreamListener<Stream = Self::Stream>;
    type Datagram: NodeDatagram;
    type ConnectFuture<'a>: Future<Output = Result<Self::Stream, TransportError>> + 'a
    where
        Self: 'a;
    type BindStreamFuture<'a>: Future<Output = Result<Self::Listener, TransportError>> + 'a
    where
        Self: 'a;
    type BindDatagramFuture<'a>: Future<Output = Result<Self::Datagram, TransportError>> + 'a
    where
        Self: 'a;

    fn connect_stream<'a>(&'a self, addr: &'a TransportAddress) -> Self::ConnectFuture<'a>;
    fn bind_stream<'a>(&'a self, addr: &'a TransportAddress) -> Self::BindStreamFuture<'a>;
    fn bind_datagram<'a>(&'a self, addr: &'a TransportAddress) -> Self::BindDatagramFuture<'a>;
}

pub type BoxedNodeStream = Box<dyn NodeStream>;

pub struct Ready<T> {
    value: Option<T>,
}

impl<T> Ready<T> {
    pub const fn new(value: T) -> Self {
        Self { value: Some(value) }
    }
}

impl<T: Unpin> Future for Ready<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(
            self.value
                .take()
                .expect("ready future polled after completion"),
        )
    }
}
