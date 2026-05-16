use crate::error::{GattError, GattResult};
use crate::l2cap::*;
use crate::prelude::v1::*;
use edgerun_encoding::byteorder::read_u16_le;
use edgerun_protocols::bluetooth_gatt::{format_bdaddr_hex, parse_bdaddr_string, reverse_bdaddr};
use std::io;
use std::mem::size_of;
use std::os::fd::{AsRawFd, RawFd};
use std::time::Duration;

mod libc {
    pub const EINPROGRESS: i32 = 115;
    pub const F_GETFL: i32 = 3;
    pub const F_SETFL: i32 = 4;
    pub const O_NONBLOCK: i32 = 0x800;
    pub const SOL_SOCKET: i32 = 1;
    pub const SO_ERROR: i32 = 4;
    #[allow(non_camel_case_types)]
    pub type socklen_t = u32;

    unsafe extern "C" {
        pub fn fcntl(fd: i32, cmd: i32, arg: i32) -> i32;
        pub fn getsockopt(
            fd: i32,
            level: i32,
            optname: i32,
            optval: *mut core::ffi::c_void,
            optlen: *mut socklen_t,
        ) -> i32;
    }
}

pub struct L2capSocket {
    fd: RawFd,
    local_addr_type: u8,
    peer_addr: Option<[u8; 6]>,
    state: L2capChannelState,
    mtu: u16,
    remote_mtu: u16,
}

impl Clone for L2capSocket {
    fn clone(&self) -> Self {
        let fd = unsafe { dup(self.fd) };
        Self {
            fd,
            local_addr_type: self.local_addr_type,
            peer_addr: self.peer_addr,
            state: self.state,
            mtu: self.mtu,
            remote_mtu: self.remote_mtu,
        }
    }
}

impl L2capSocket {
    pub fn new() -> GattResult<Self> {
        let fd = unsafe { socket(AF_BLUETOOTH, SOCK_SEQPACKET, BTPROTO_L2CAP) };
        if fd < 0 {
            return Err(GattError::SocketFailed(
                "failed to create L2CAP socket".to_string(),
            ));
        }
        Ok(Self {
            fd,
            local_addr_type: 0,
            peer_addr: None,
            state: L2capChannelState::Closed,
            mtu: DEFAULT_MTU,
            remote_mtu: DEFAULT_MTU,
        })
    }

    pub fn with_mtu(mtu: u16) -> GattResult<Self> {
        let mut sock = Self::new()?;
        sock.mtu = mtu.min(MAX_MTU);
        Ok(sock)
    }

    pub fn bind_public(&self) -> GattResult<()> {
        self.bind_with_addr_type(0x01)
    }

    pub fn bind_random(&self) -> GattResult<()> {
        self.bind_with_addr_type(0x02)
    }

    pub fn bind_with_addr_type(&self, addr_type: u8) -> GattResult<()> {
        let l2cap_addr = SockAddrL2 {
            l2_bdaddr: [0u8; 6],
            l2_bdaddr_type: addr_type,
            ..Default::default()
        };

        let rc = unsafe {
            bind(
                self.fd,
                (&l2cap_addr as *const SockAddrL2).cast(),
                size_of::<SockAddrL2>() as u32,
            )
        };

        if rc < 0 {
            return Err(GattError::from(io::Error::last_os_error()));
        }
        Ok(())
    }

    pub fn bind_local(&self, addr_type: u8) -> GattResult<()> {
        self.bind_with_addr_type(addr_type)
    }

    pub fn bind_att_any(&self) -> GattResult<()> {
        let l2cap_addr = SockAddrL2 {
            l2_cid: ATT_CID,
            ..Default::default()
        };

        let rc = unsafe {
            bind(
                self.fd,
                (&l2cap_addr as *const SockAddrL2).cast(),
                size_of::<SockAddrL2>() as u32,
            )
        };

        if rc < 0 {
            return Err(GattError::from(io::Error::last_os_error()));
        }
        Ok(())
    }

    pub fn set_local_addr_type(&mut self, addr_type: u8) {
        self.local_addr_type = addr_type;
    }

    pub fn connect_to_device(&mut self, device_addr: &str, addr_type: u8) -> GattResult<()> {
        let bdaddr = reverse_bdaddr(device_addr)
            .ok_or_else(|| GattError::InvalidAddress(device_addr.to_string()))?;

        self.bind_att_any()?;
        self.peer_addr = Some(bdaddr);
        self.state = L2capChannelState::Opening;

        let l2cap_addr = SockAddrL2::for_att_device(&bdaddr, addr_type);

        let rc = unsafe {
            connect(
                self.fd,
                (&l2cap_addr as *const SockAddrL2).cast(),
                size_of::<SockAddrL2>() as u32,
            )
        };

        if rc < 0 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EINPROGRESS) {
                self.wait_for_connect(10_000)?;
                self.state = L2capChannelState::Open;
                return Ok(());
            }
            self.state = L2capChannelState::Closed;
            return Err(GattError::from(err));
        }

        self.state = L2capChannelState::Open;
        Ok(())
    }

    pub fn connect_with_le_psm(&mut self, device_addr: &str, addr_type: u8) -> GattResult<()> {
        let bdaddr = reverse_bdaddr(device_addr)
            .ok_or_else(|| GattError::InvalidAddress(device_addr.to_string()))?;

        self.peer_addr = Some(bdaddr);
        self.state = L2capChannelState::Opening;

        let l2cap_addr = SockAddrL2::for_device(&bdaddr, addr_type, LE_PSM);

        let rc = unsafe {
            connect(
                self.fd,
                (&l2cap_addr as *const SockAddrL2).cast(),
                size_of::<SockAddrL2>() as u32,
            )
        };

        if rc < 0 {
            let err = io::Error::last_os_error();
            self.state = L2capChannelState::Closed;
            return Err(GattError::from(err));
        }

        self.state = L2capChannelState::Open;
        Ok(())
    }

    pub fn connect_device(&self, device_addr: &str, addr_type: u8) -> GattResult<()> {
        let bdaddr = reverse_bdaddr(device_addr)
            .ok_or_else(|| GattError::InvalidAddress(device_addr.to_string()))?;

        self.bind_att_any()?;
        let l2cap_addr = SockAddrL2::for_att_device(&bdaddr, addr_type);

        let rc = unsafe {
            connect(
                self.fd,
                (&l2cap_addr as *const SockAddrL2).cast(),
                size_of::<SockAddrL2>() as u32,
            )
        };

        if rc < 0 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EINPROGRESS) {
                return self.wait_for_connect(10_000);
            }
            match err.kind() {
                io::ErrorKind::ConnectionRefused => Err(GattError::ConnectionRefused(
                    "device may not be in range or may reject connection".to_string(),
                )),
                io::ErrorKind::TimedOut => Err(GattError::Timeout(0)),
                io::ErrorKind::HostUnreachable => Err(GattError::HostUnreachable),
                _ => Err(GattError::from(err)),
            }
        } else {
            Ok(())
        }
    }

    pub fn connect_with_transport(
        &self,
        device_addr: &str,
        addr_type: u8,
        psm: u16,
    ) -> GattResult<()> {
        let bdaddr = reverse_bdaddr(device_addr)
            .ok_or_else(|| GattError::InvalidAddress(device_addr.to_string()))?;

        let l2cap_addr = SockAddrL2::for_device(&bdaddr, addr_type, psm);

        let rc = unsafe {
            connect(
                self.fd,
                (&l2cap_addr as *const SockAddrL2).cast(),
                size_of::<SockAddrL2>() as u32,
            )
        };

        if rc < 0 {
            Err(GattError::from(io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }

    pub fn send_data(&self, data: &[u8], timeout_ms: i32) -> GattResult<()> {
        let mut pollfd = PollFd {
            fd: self.fd,
            events: 0x0004,
            revents: 0,
        };

        let poll_rc = unsafe { poll(&mut pollfd, 1, timeout_ms) };
        if poll_rc < 0 {
            return Err(GattError::PollFailed("poll failed".to_string()));
        }
        if poll_rc == 0 {
            return Err(GattError::Timeout(timeout_ms as u32));
        }

        let rc = unsafe { send(self.fd, data.as_ptr().cast(), data.len(), 0) };
        if rc < 0 {
            return Err(GattError::SendFailed("send failed".to_string()));
        }

        Ok(())
    }

    pub fn recv_data(&self, buf: &mut [u8], timeout_ms: i32) -> GattResult<usize> {
        let mut pollfd = PollFd {
            fd: self.fd,
            events: 0x0001,
            revents: 0,
        };

        let poll_rc = unsafe { poll(&mut pollfd, 1, timeout_ms) };
        if poll_rc < 0 {
            return Err(GattError::PollFailed("poll failed".to_string()));
        }
        if poll_rc == 0 {
            return Err(GattError::Timeout(timeout_ms as u32));
        }

        let read = unsafe { recv(self.fd, buf.as_mut_ptr().cast(), buf.len(), 0) };
        if read < 0 {
            return Err(GattError::RecvFailed("recv failed".to_string()));
        }

        Ok(read as usize)
    }

    pub fn recv_timeout(&self, buf: &mut [u8], timeout: Duration) -> GattResult<Option<usize>> {
        match self.recv_data(buf, timeout.as_millis() as i32) {
            Ok(n) => Ok(Some(n)),
            Err(GattError::Timeout(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn fd(&self) -> RawFd {
        self.fd
    }

    pub fn state(&self) -> L2capChannelState {
        self.state
    }

    pub fn is_connected(&self) -> bool {
        matches!(
            self.state,
            L2capChannelState::Open
                | L2capChannelState::ConfigRequest
                | L2capChannelState::ConfigResponse
                | L2capChannelState::Connected
        )
    }

    pub fn mtu(&self) -> u16 {
        self.mtu
    }

    pub fn remote_mtu(&self) -> u16 {
        self.remote_mtu
    }

    pub fn set_mtu(&mut self, mtu: u16) {
        self.mtu = mtu.min(MAX_MTU);
    }

    pub fn peer_address(&self) -> Option<String> {
        self.peer_addr.map(|addr| format_bdaddr_hex(&addr))
    }

    pub fn set_options(&self, _option_name: i32, _value: &[u8]) -> GattResult<()> {
        Ok(())
    }

    pub fn set_nonblocking(&self, nonblock: bool) -> GattResult<()> {
        #[cfg(not(unix))]
        {
            let _ = nonblock;
            return Ok(());
        }
        #[cfg(unix)]
        {
            let flags = unsafe { libc::fcntl(self.fd, libc::F_GETFL, 0) };
            if flags < 0 {
                return Err(GattError::SocketFailed("fcntl F_GETFL failed".to_string()));
            }
            let new_flags = if nonblock {
                flags | libc::O_NONBLOCK
            } else {
                flags & !libc::O_NONBLOCK
            };
            let rc = unsafe { libc::fcntl(self.fd, libc::F_SETFL, new_flags) };
            if rc < 0 {
                Err(GattError::SocketFailed("fcntl F_SETFL failed".to_string()))
            } else {
                Ok(())
            }
        }
    }

    fn wait_for_connect(&self, timeout_ms: i32) -> GattResult<()> {
        #[cfg(not(unix))]
        {
            let _ = timeout_ms;
            return Err(GattError::SocketFailed(
                "L2CAP connect polling requires Unix sockets".to_string(),
            ));
        }
        #[cfg(unix)]
        {
            let mut pollfd = PollFd {
                fd: self.fd,
                events: 0x0004,
                revents: 0,
            };
            let poll_rc = unsafe { poll(&mut pollfd, 1, timeout_ms) };
            if poll_rc < 0 {
                return Err(GattError::PollFailed("connect poll failed".to_string()));
            }
            if poll_rc == 0 {
                return Err(GattError::Timeout(timeout_ms as u32));
            }

            let mut so_error: i32 = 0;
            let mut len = size_of::<i32>() as libc::socklen_t;
            let rc = unsafe {
                libc::getsockopt(
                    self.fd,
                    libc::SOL_SOCKET,
                    libc::SO_ERROR,
                    (&mut so_error as *mut i32).cast(),
                    &mut len,
                )
            };
            if rc < 0 {
                return Err(GattError::from(io::Error::last_os_error()));
            }
            if so_error != 0 {
                return Err(GattError::from(io::Error::from_raw_os_error(so_error)));
            }
            Ok(())
        }
    }
}

impl Drop for L2capSocket {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

unsafe impl Send for L2capSocket {}
unsafe impl Sync for L2capSocket {}

pub fn format_bdaddr(bytes: &[u8]) -> String {
    format_bdaddr_hex(bytes)
}

pub struct AttProtocol {
    socket: L2capSocket,
    mtu: u16,
    transaction_id: u8,
}

impl AttProtocol {
    pub fn new(socket: L2capSocket) -> Self {
        Self {
            socket,
            mtu: DEFAULT_MTU,
            transaction_id: 0,
        }
    }

    pub fn with_mtu(socket: L2capSocket, mtu: u16) -> Self {
        Self {
            socket,
            mtu: mtu.min(MAX_MTU),
            transaction_id: 0,
        }
    }

    pub fn set_mtu(&mut self, mtu: u16) {
        self.mtu = mtu.min(MAX_MTU);
    }

    pub fn mtu(&self) -> u16 {
        self.mtu
    }

    fn next_transaction_id(&mut self) -> u8 {
        self.transaction_id = self.transaction_id.wrapping_add(1);
        self.transaction_id
    }

    fn send_recv(&mut self, req: &[u8]) -> GattResult<Vec<u8>> {
        self.socket.send_data(req, 1000)?;
        let mut buf = vec![0u8; self.mtu as usize];
        let n = self.socket.recv_data(&mut buf, 2000)?;
        buf.truncate(n);
        Ok(buf)
    }

    pub fn exchange_mtu(&mut self, client_mtu: u16) -> GattResult<u16> {
        let mut req = vec![0x02];
        req.extend_from_slice(&client_mtu.to_le_bytes());
        let resp = self.send_recv(&req)?;

        if resp.len() < 3 || resp[0] != 0x03 {
            return Err(GattError::MtuExchangeFailed);
        }

        let server_mtu = read_u16_le(&resp, 1);
        self.mtu = std::cmp::min(client_mtu, server_mtu);
        Ok(self.mtu)
    }

    pub fn read_by_group_type(
        &mut self,
        start: u16,
        end: u16,
        group_type: &[u8],
    ) -> GattResult<Vec<u8>> {
        let mut req = vec![0x10];
        req.extend_from_slice(&start.to_le_bytes());
        req.extend_from_slice(&end.to_le_bytes());
        req.extend_from_slice(group_type);
        self.send_recv(&req)
    }

    pub fn find_information(&mut self, start: u16, end: u16) -> GattResult<Vec<u8>> {
        let mut req = vec![0x04];
        req.extend_from_slice(&start.to_le_bytes());
        req.extend_from_slice(&end.to_le_bytes());
        self.send_recv(&req)
    }

    pub fn read_by_type(&mut self, start: u16, end: u16, uuid: &[u8]) -> GattResult<Vec<u8>> {
        let mut req = vec![0x08];
        req.extend_from_slice(&start.to_le_bytes());
        req.extend_from_slice(&end.to_le_bytes());
        req.extend_from_slice(uuid);
        self.send_recv(&req)
    }

    pub fn read_value(&mut self, handle: u16) -> GattResult<Vec<u8>> {
        let mut req = vec![0x0a];
        req.extend_from_slice(&handle.to_le_bytes());
        self.send_recv(&req)
    }

    pub fn read_blob(&mut self, handle: u16, offset: u16) -> GattResult<Vec<u8>> {
        let mut req = vec![0x0c];
        req.extend_from_slice(&handle.to_le_bytes());
        req.extend_from_slice(&offset.to_le_bytes());
        self.send_recv(&req)
    }

    pub fn write_value(&mut self, handle: u16, data: &[u8], with_response: bool) -> GattResult<()> {
        let opcode = if with_response { 0x12 } else { 0x52 };
        let mut req = vec![opcode];
        req.extend_from_slice(&handle.to_le_bytes());
        req.extend_from_slice(data);
        self.socket.send_data(&req, 1000)?;

        if with_response {
            let mut buf = vec![0u8; self.mtu as usize];
            let n = self.socket.recv_data(&mut buf, 2000)?;
            if n > 0 && buf[0] == 0x13 {
                return Ok(());
            } else if n > 1 {
                return Err(GattError::att_error_code(buf[1]));
            }
        }
        Ok(())
    }

    pub fn write_cmd(&mut self, handle: u16, data: &[u8]) -> GattResult<()> {
        let mut req = vec![0x52];
        req.extend_from_slice(&handle.to_le_bytes());
        req.extend_from_slice(data);
        self.socket.send_data(&req, 500)
    }

    pub fn prepare_write_value(&mut self, handle: u16, offset: u16, data: &[u8]) -> GattResult<()> {
        let mut req = vec![0x16];
        req.extend_from_slice(&handle.to_le_bytes());
        req.extend_from_slice(&offset.to_le_bytes());
        req.extend_from_slice(data);
        self.socket.send_data(&req, 1000)?;

        let mut buf = vec![0u8; self.mtu as usize];
        let n = self.socket.recv_data(&mut buf, 2000)?;
        if n < 5 || buf[0] != 0x17 {
            return if n > 1 {
                Err(GattError::att_error_code(buf[1]))
            } else {
                Err(GattError::ParseError(
                    "invalid prepare write response".into(),
                ))
            };
        }
        let response_handle = read_u16_le(&buf, 1);
        let response_offset = read_u16_le(&buf, 3);
        if response_handle != handle || response_offset != offset {
            return Err(GattError::ParseError(
                "prepare write response handle/offset mismatch".into(),
            ));
        }
        Ok(())
    }

    pub fn execute_write(&mut self, commit: bool) -> GattResult<()> {
        let flag = if commit { 0x01 } else { 0x00 };
        let req = vec![0x18, flag];
        self.socket.send_data(&req, 1000)?;

        let mut buf = vec![0u8; self.mtu as usize];
        let n = self.socket.recv_data(&mut buf, 2000)?;
        if n > 0 && buf[0] == 0x19 {
            Ok(())
        } else if n > 1 {
            Err(GattError::att_error_code(buf[1]))
        } else {
            Err(GattError::ParseError(
                "invalid execute write response".into(),
            ))
        }
    }

    pub fn handle_notification(&self, data: &[u8]) -> Option<(u16, Vec<u8>)> {
        edgerun_protocols::bluetooth_gatt::handle_notification(data)
    }

    pub fn handle_indication(&self, data: &[u8]) -> Option<(u16, Vec<u8>)> {
        edgerun_protocols::bluetooth_gatt::handle_indication(data)
    }

    pub fn handle_execute_write_response(&self, data: &[u8]) -> bool {
        edgerun_protocols::bluetooth_gatt::handle_execute_write_response(data)
    }

    pub fn parse_read_by_group_response(&self, data: &[u8]) -> Vec<(u16, u16, Vec<u8>)> {
        edgerun_protocols::bluetooth_gatt::parse_read_by_group_response(data)
    }

    pub fn parse_read_by_type_response(&self, data: &[u8]) -> Vec<(u16, Vec<u8>)> {
        edgerun_protocols::bluetooth_gatt::parse_read_by_type_response(data)
    }

    pub fn parse_find_information_response(&self, data: &[u8]) -> Vec<(u16, Vec<u8>)> {
        edgerun_protocols::bluetooth_gatt::parse_find_information_response(data)
    }

    pub fn parse_error_response(&self, data: &[u8]) -> Option<(u16, u8)> {
        edgerun_protocols::bluetooth_gatt::parse_error_response(data)
    }

    pub fn parse_mtu_response(&self, data: &[u8]) -> Option<u16> {
        edgerun_protocols::bluetooth_gatt::parse_mtu_response(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bdaddr() {
        let addr = "AA:BB:CC:DD:EE:FF";
        let result = parse_bdaddr_string(addr).unwrap();
        assert_eq!(result, [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    }

    #[test]
    fn reverse_bdaddr_test() {
        let addr = "AA:BB:CC:DD:EE:FF";
        let reversed = reverse_bdaddr(addr).unwrap();
        assert_eq!(reversed, [0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA]);
    }

    #[test]
    fn format_bdaddr_test() {
        let bytes = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let formatted = format_bdaddr_hex(&bytes);
        assert_eq!(formatted, "aa:bb:cc:dd:ee:ff");
    }

    #[test]
    fn l2cap_socket_state() {
        let socket = L2capSocket::new().unwrap();
        assert_eq!(socket.state(), L2capChannelState::Closed);
        assert!(!socket.is_connected());
    }

    #[test]
    fn att_protocol_mtu() {
        let socket = L2capSocket::new().unwrap();
        let proto = AttProtocol::new(socket);
        assert_eq!(proto.mtu(), 23);
    }

    #[test]
    fn parse_error_response() {
        let socket = L2capSocket::new().unwrap();
        let proto = AttProtocol::new(socket);
        let data = vec![0x01, 0x0A, 0x00, 0x0A];
        let result = proto.parse_error_response(&data);
        assert_eq!(result, Some((0x000A, 0x0A)));
    }

    #[test]
    fn parse_find_info_response() {
        let socket = L2capSocket::new().unwrap();
        let proto = AttProtocol::new(socket);
        let data = vec![0x05, 0x01, 0x03, 0x00, 0x03, 0x28];
        let results = proto.parse_find_information_response(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 0x0003);
        assert_eq!(results[0].1, vec![0x03, 0x28]);
    }

    #[test]
    fn parse_read_by_group_response() {
        let socket = L2capSocket::new().unwrap();
        let mut proto = AttProtocol::new(socket);
        proto.set_mtu(50);
        let data = vec![0x11, 0x06, 0x01, 0x00, 0x08, 0x00, 0x00, 0x28];
        let results = proto.parse_read_by_group_response(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 0x0001);
        assert_eq!(results[0].1, 0x0008);
    }
}
