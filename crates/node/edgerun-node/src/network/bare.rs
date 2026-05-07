use alloc::string::ToString;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::rt;

use super::provider::NodeDatagram;
use super::types::{TransportAddress, TransportCarrier, TransportError};

#[derive(Clone, Copy, Debug, Default)]
pub struct BareFrameTransport;

impl BareFrameTransport {
    pub fn bind_datagram_now(
        &self,
        addr: &TransportAddress,
    ) -> Result<rt::UdpSocket, TransportError> {
        let local = parse_rt_datagram_addr(addr)?;
        let mut socket = rt::UdpSocket::new();
        socket
            .bind(local)
            .map_err(|_| TransportError::Io("bare datagram bind failed".into()))?;
        Ok(socket)
    }
}

impl NodeDatagram for rt::UdpSocket {
    fn local_transport_addr(&self) -> Result<TransportAddress, TransportError> {
        self.local_addr()
            .map(rt_datagram_addr)
            .ok_or(TransportError::AddressUnavailable)
    }

    fn peer_transport_addr(&self) -> Result<Option<TransportAddress>, TransportError> {
        Ok(self.remote_addr().map(rt_datagram_addr))
    }

    fn poll_recv_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        payload: &mut [u8],
    ) -> Poll<Result<(usize, TransportAddress), TransportError>> {
        match self.recv_from(payload) {
            Ok((len, source)) => Poll::Ready(Ok((len, rt_datagram_addr(source)))),
            Err(_) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    fn poll_send_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        payload: &[u8],
        destination: &TransportAddress,
    ) -> Poll<Result<usize, TransportError>> {
        let destination = match parse_rt_datagram_addr(destination) {
            Ok(addr) => addr,
            Err(error) => return Poll::Ready(Err(error)),
        };
        Poll::Ready(
            self.send_to(payload, destination)
                .map_err(|_| TransportError::Io("bare datagram send failed".into())),
        )
    }
}

fn rt_datagram_addr(addr: rt::SocketAddr) -> TransportAddress {
    let ip = addr.ip_bytes();
    TransportAddress::bare_datagram(
        alloc::format!("{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], addr.port()).into_bytes(),
    )
}

fn parse_rt_datagram_addr(addr: &TransportAddress) -> Result<rt::SocketAddr, TransportError> {
    if addr.carrier != TransportCarrier::BareFrame {
        return Err(TransportError::UnsupportedCarrier);
    }
    let text =
        core::str::from_utf8(&addr.endpoint).map_err(|_| TransportError::AddressUnavailable)?;
    let Some((ip_text, port_text)) = text.rsplit_once(':') else {
        return Err(TransportError::AddressUnavailable);
    };
    let port = port_text
        .parse::<u16>()
        .map_err(|_| TransportError::AddressUnavailable)?;
    let mut octets = [0u8; 4];
    let mut count = 0usize;
    for (idx, part) in ip_text.split('.').enumerate() {
        if idx >= 4 {
            return Err(TransportError::AddressUnavailable);
        }
        octets[idx] = part
            .parse::<u8>()
            .map_err(|_| TransportError::AddressUnavailable)?;
        count += 1;
    }
    if count != 4 {
        return Err(TransportError::AddressUnavailable);
    }
    Ok(rt::SocketAddr::from_bytes4(octets, port))
}
