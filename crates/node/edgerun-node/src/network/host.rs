use alloc::sync::Arc;
use core::future::Future;
use core::net::SocketAddr;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::rt;

use super::provider::{NodeDatagram, NodeStream, NodeStreamListener, Ready, RuntimeTransport};
use super::types::{TransportAddress, TransportCarrier, TransportError, TransportProtocol};

#[derive(Clone, Copy, Debug, Default)]
pub struct HostSocketTransport;

pub struct HostConnectFuture {
    inner: rt::ConnectFuture,
}

impl Future for HostConnectFuture {
    type Output = Result<Arc<rt::AsyncTcpStream>, TransportError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match unsafe { Pin::new_unchecked(&mut self.inner) }.poll(cx) {
            Poll::Ready(Ok(stream)) => Poll::Ready(Ok(stream)),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error.into())),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl RuntimeTransport for HostSocketTransport {
    type Stream = Arc<rt::AsyncTcpStream>;
    type Listener = rt::AsyncTcpListener;
    type Datagram = rt::AsyncUdpSocket;
    type ConnectFuture<'a> = HostConnectFuture;
    type BindStreamFuture<'a> = Ready<Result<Self::Listener, TransportError>>;
    type BindDatagramFuture<'a> = Ready<Result<Self::Datagram, TransportError>>;

    fn connect_stream<'a>(&'a self, addr: &'a TransportAddress) -> Self::ConnectFuture<'a> {
        let target = core::str::from_utf8(&addr.endpoint).unwrap_or_default();
        HostConnectFuture {
            inner: rt::ConnectFuture::new(target),
        }
    }

    fn bind_stream<'a>(&'a self, addr: &'a TransportAddress) -> Self::BindStreamFuture<'a> {
        Ready::new(
            parse_host_socket_addr(addr)
                .and_then(|addr| rt::AsyncTcpListener::bind(addr).map_err(TransportError::from)),
        )
    }

    fn bind_datagram<'a>(&'a self, addr: &'a TransportAddress) -> Self::BindDatagramFuture<'a> {
        Ready::new(
            parse_host_socket_addr(addr)
                .and_then(|addr| rt::AsyncUdpSocket::bind(addr).map_err(TransportError::from)),
        )
    }
}

impl HostSocketTransport {
    pub fn bind_stream_now(
        &self,
        addr: &TransportAddress,
    ) -> Result<rt::AsyncTcpListener, TransportError> {
        parse_host_socket_addr(addr)
            .and_then(|addr| rt::AsyncTcpListener::bind(addr).map_err(TransportError::from))
    }

    pub fn bind_datagram_now(
        &self,
        addr: &TransportAddress,
    ) -> Result<rt::AsyncUdpSocket, TransportError> {
        parse_host_socket_addr(addr)
            .and_then(|addr| rt::AsyncUdpSocket::bind(addr).map_err(TransportError::from))
    }
}

impl NodeStream for rt::AsyncTcpStream {
    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        Ok(self.local_addr()?.into())
    }

    fn peer_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        Ok(self.peer_addr()?.into())
    }
}

impl NodeStream for Arc<rt::AsyncTcpStream> {
    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        Ok(self.local_addr()?.into())
    }

    fn peer_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        Ok(self.peer_addr()?.into())
    }
}

impl NodeStreamListener for rt::AsyncTcpListener {
    type Stream = Arc<rt::AsyncTcpStream>;

    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        Ok(self.local_addr()?.into())
    }

    fn poll_accept_stream(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(Self::Stream, TransportAddress), TransportError>> {
        let listener = self.get_mut();
        let mut accept = listener.accept();
        match unsafe { Pin::new_unchecked(&mut accept) }.poll(cx) {
            Poll::Ready(Ok((stream, peer))) => Poll::Ready(Ok((stream, peer.into()))),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error.into())),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl NodeDatagram for rt::AsyncUdpSocket {
    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        let mut addr: TransportAddress = self.local_addr()?.into();
        addr.protocol = TransportProtocol::Datagram;
        Ok(addr)
    }

    fn peer_transport_addr(&self) -> Result<Option<TransportAddress>, TransportError> {
        match self.peer_addr() {
            Ok(peer) => {
                let mut addr: TransportAddress = peer.into();
                addr.protocol = TransportProtocol::Datagram;
                Ok(Some(addr))
            }
            Err(_) => Ok(None),
        }
    }

    fn poll_recv_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        payload: &mut [u8],
    ) -> Poll<Result<(usize, TransportAddress), TransportError>> {
        let socket = self.get_mut();
        let mut recv = socket.recv_from(payload);
        match unsafe { Pin::new_unchecked(&mut recv) }.poll(cx) {
            Poll::Ready(Ok((len, source))) => {
                let mut addr: TransportAddress = source.into();
                addr.protocol = TransportProtocol::Datagram;
                Poll::Ready(Ok((len, addr)))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error.into())),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_send_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        payload: &[u8],
        destination: &TransportAddress,
    ) -> Poll<Result<usize, TransportError>> {
        let destination = parse_host_socket_addr(destination)?;
        let socket = self.get_mut();
        let mut send = socket.send_to(payload, destination);
        match unsafe { Pin::new_unchecked(&mut send) }.poll(cx) {
            Poll::Ready(Ok(len)) => Poll::Ready(Ok(len)),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error.into())),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn parse_host_socket_addr(addr: &TransportAddress) -> Result<SocketAddr, TransportError> {
    if addr.carrier != TransportCarrier::HostSocket {
        return Err(TransportError::UnsupportedCarrier);
    }
    let text =
        core::str::from_utf8(&addr.endpoint).map_err(|_| TransportError::AddressUnavailable)?;
    text.parse().map_err(|_| TransportError::AddressUnavailable)
}
