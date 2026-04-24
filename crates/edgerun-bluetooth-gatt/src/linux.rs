use crate::error::{GattError, GattResult};
use std::io;
use std::mem::size_of;
use std::os::fd::{AsRawFd, RawFd};
use std::time::Duration;

const AF_BLUETOOTH: i32 = 31;
const SOCK_SEQPACKET: i32 = 5;
const BTPROTO_L2CAP: i32 = 2;

const ATT_PSM: u16 = 0x001F;
const DEFAULT_MTU: u16 = 23;
const MAX_MTU: u16 = 512;
const LE_PSM: u16 = 0x002F;

const OCF_L2CAP_CONN_REQ: u16 = 0x0401;
const OCF_L2CAP_CONFIG_REQ: u16 = 0x0411;
const OCF_L2CAP_DISCONN_REQ: u16 = 0x0403;
const OCF_L2CAP_INFO_REQ: u16 = 0x040A;

const HCI_EV_LE_META_EVENT: u8 = 0x3E;
const HCI_EV_CONN_COMPLETE: u8 = 0x13;
const HCI_EV_DISCONN_COMPLETE: u8 = 0x05;

#[repr(C)]
#[derive(Clone, Copy)]
struct SockAddrL2 {
    l2_family: u16,
    l2_cid: u16,
    l2_bdaddr_type: u8,
    l2_bdaddr: [u8; 6],
    l2_psm: u16,
}

impl Default for SockAddrL2 {
    fn default() -> Self {
        Self {
            l2_family: AF_BLUETOOTH as u16,
            l2_cid: 0,
            l2_bdaddr_type: 0,
            l2_bdaddr: [0; 6],
            l2_psm: 0,
        }
    }
}

impl SockAddrL2 {
    fn for_device(addr: &[u8; 6], addr_type: u8, psm: u16) -> Self {
        Self {
            l2_family: AF_BLUETOOTH as u16,
            l2_cid: 0,
            l2_bdaddr_type: addr_type,
            l2_bdaddr: *addr,
            l2_psm: psm,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum L2capChannelState {
    Closed,
    Opening,
    Open,
    ConfigRequest,
    ConfigResponse,
    Connected,
    Disconnecting,
}

#[derive(Clone)]
pub struct L2capSocket {
    fd: RawFd,
    local_addr_type: u8,
    peer_addr: Option<[u8; 6]>,
    state: L2capChannelState,
    mtu: u16,
    remote_mtu: u16,
}

impl L2capSocket {
    pub fn new() -> GattResult<Self> {
        let fd = unsafe { socket(AF_BLUETOOTH, SOCK_SEQPACKET, BTPROTO_L2CAP) };
        if fd < 0 {
            return Err(GattError::SocketFailed("failed to create L2CAP socket".to_string()));
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

    pub fn set_local_addr_type(&mut self, addr_type: u8) {
        self.local_addr_type = addr_type;
    }

    pub fn connect_to_device(
        &mut self,
        device_addr: &str,
        addr_type: u8,
    ) -> GattResult<()> {
        let bdaddr = parse_bdaddr_string(device_addr)
            .ok_or_else(|| GattError::InvalidAddress(device_addr.to_string()))?;

        self.peer_addr = Some(bdaddr);
        self.state = L2capChannelState::Opening;

        let l2cap_addr = SockAddrL2::for_device(&bdaddr, addr_type, ATT_PSM);

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

    pub fn connect_with_le_psm(&mut self, device_addr: &str, addr_type: u8) -> GattResult<()> {
        let bdaddr = parse_bdaddr_string(device_addr)
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
        let bdaddr = parse_bdaddr_string(device_addr)
            .ok_or_else(|| GattError::InvalidAddress(device_addr.to_string()))?;

        let l2cap_addr = SockAddrL2::for_device(&bdaddr, addr_type, ATT_PSM);

        let rc = unsafe {
            connect(
                self.fd,
                (&l2cap_addr as *const SockAddrL2).cast(),
                size_of::<SockAddrL2>() as u32,
            )
        };

        if rc < 0 {
            let err = io::Error::last_os_error();
            match err.kind() {
                io::ErrorKind::ConnectionRefused => {
                    Err(GattError::ConnectionRefused("device may not be in range or may reject connection".to_string()))
                }
                io::ErrorKind::TimedOut => {
                    Err(GattError::Timeout(0))
                }
                io::ErrorKind::HostUnreachable => {
                    Err(GattError::HostUnreachable)
                }
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
        let bdaddr = parse_bdaddr_string(device_addr)
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

impl Drop for L2capSocket {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

unsafe impl Send for L2capSocket {}
unsafe impl Sync for L2capSocket {}

fn parse_bdaddr_string(addr: &str) -> Option<[u8; 6]> {
    let parts: Vec<u8> = addr
        .split(':')
        .map(|p| u8::from_str_radix(p, 16).ok())
        .collect::<Option<_>>()?;
    if parts.len() != 6 {
        return None;
    }
    let mut result = [0u8; 6];
    result.copy_from_slice(&parts);
    Some(result)
}

fn format_bdaddr_hex(bytes: &[u8]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]
    )
}

fn reverse_bdaddr(addr: &str) -> Option<[u8; 6]> {
    parse_bdaddr_string(addr).map(|mut a| {
        a.reverse();
        a
    })
}

pub fn format_bdaddr(bytes: &[u8]) -> String {
    format_bdaddr_hex(bytes)
}

unsafe extern "C" {
    fn socket(domain: i32, ty: i32, protocol: i32) -> i32;
    fn bind(fd: i32, addr: *const core::ffi::c_void, len: u32) -> i32;
    fn connect(fd: i32, addr: *const core::ffi::c_void, len: u32) -> i32;
    fn send(fd: i32, buf: *const core::ffi::c_void, len: usize, flags: i32) -> isize;
    fn recv(fd: i32, buf: *mut core::ffi::c_void, len: usize, flags: i32) -> isize;
    fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
    fn close(fd: i32) -> i32;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
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

        let server_mtu = u16::from_le_bytes([resp[1], resp[2]]);
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

    pub fn read_by_type(
        &mut self,
        start: u16,
        end: u16,
        uuid: &[u8],
    ) -> GattResult<Vec<u8>> {
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

    pub fn write_value(
        &mut self,
        handle: u16,
        data: &[u8],
        with_response: bool,
    ) -> GattResult<()> {
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

    pub fn prepare_write_value(
        &mut self,
        handle: u16,
        offset: u16,
        data: &[u8],
    ) -> GattResult<()> {
        let mut req = vec![0x16];
        req.extend_from_slice(&handle.to_le_bytes());
        req.extend_from_slice(&offset.to_le_bytes());
        req.extend_from_slice(data);
        self.socket.send_data(&req, 1000)
    }

    pub fn execute_write(&mut self, commit: bool) -> GattResult<()> {
        let flag = if commit { 0x01 } else { 0x00 };
        let req = vec![0x18, flag];
        self.socket.send_data(&req, 1000)
    }

    pub fn handle_notification(&self, data: &[u8]) -> Option<(u16, Vec<u8>)> {
        if data.len() < 3 || data[0] != 0x1b {
            return None;
        }
        let handle = u16::from_le_bytes([data[1], data[2]]);
        Some((handle, data[3..].to_vec()))
    }

    pub fn handle_indication(&self, data: &[u8]) -> Option<(u16, Vec<u8>)> {
        if data.len() < 3 || data[0] != 0x1d {
            return None;
        }
        let handle = u16::from_le_bytes([data[1], data[2]]);
        Some((handle, data[3..].to_vec()))
    }

    pub fn handle_execute_write_response(&self, data: &[u8]) -> bool {
        data.first() == Some(&0x19)
    }

    pub fn parse_read_by_group_response(&self, data: &[u8]) -> Vec<(u16, u16, Vec<u8>)> {
        let mut results = Vec::new();
        if data.len() < 2 || data[0] != 0x11 {
            return results;
        }
        let format = data[1] as usize;
        if format != 6 && format != 20 {
            return results;
        }
        let entry_size = format;
        let mut offset = 2;
        while offset + entry_size <= data.len() {
            let start = u16::from_le_bytes([data[offset], data[offset + 1]]);
            let end = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
            let uuid = data[offset + 4..offset + entry_size].to_vec();
            results.push((start, end, uuid));
            offset += entry_size;
        }
        results
    }

    pub fn parse_read_by_type_response(&self, data: &[u8]) -> Vec<(u16, Vec<u8>)> {
        let mut results = Vec::new();
        if data.len() < 2 || data[0] != 0x09 {
            return results;
        }
        let format = data[1] as usize;
        if format < 7 {
            return results;
        }
        let mut offset = 2;
        while offset + format <= data.len() {
            let handle = u16::from_le_bytes([data[offset], data[offset + 1]]);
            let value = data[offset + 2..offset + format].to_vec();
            results.push((handle, value));
            offset += format;
        }
        results
    }

    pub fn parse_find_information_response(&self, data: &[u8]) -> Vec<(u16, Vec<u8>)> {
        let mut results = Vec::new();
        if data.len() < 2 || data[0] != 0x05 {
            return results;
        }
        let format = data[1];
        let uuid_size = match format {
            0x01 => 2,
            0x02 => 16,
            _ => return results,
        };
        let entry_size = 2 + uuid_size; // 2 bytes handle + uuid_size bytes
        let mut offset = 2;
        while offset + entry_size <= data.len() {
            let handle = u16::from_le_bytes([data[offset], data[offset + 1]]);
            let uuid = data[offset + 2..offset + entry_size].to_vec();
            results.push((handle, uuid));
            offset += entry_size;
        }
        results
    }

    pub fn parse_error_response(&self, data: &[u8]) -> Option<(u16, u8)> {
        if data.len() < 4 || data[0] != 0x01 {
            return None;
        }
        let handle = u16::from_le_bytes([data[1], data[2]]);
        let error_code = data[3];
        Some((handle, error_code))
    }

    pub fn parse_mtu_response(&self, data: &[u8]) -> Option<u16> {
        if data.len() < 3 || data[0] != 0x03 {
            return None;
        }
        Some(u16::from_le_bytes([data[1], data[2]]))
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
        let data = vec![
            0x11, 0x06, 0x01, 0x00, 0x08, 0x00, 0x00, 0x28,
        ];
        let results = proto.parse_read_by_group_response(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 0x0001);
        assert_eq!(results[0].1, 0x0008);
    }
}