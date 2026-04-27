use crate::error::{GattError, GattResult};
use crate::prelude::v1::*;
use std::io;
use std::mem::size_of;
use std::os::fd::RawFd;

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

pub struct AsyncL2capSocket {
    fd: RawFd,
    local_addr_type: u8,
    peer_addr: Option<[u8; 6]>,
    state: L2capChannelState,
    mtu: u16,
    remote_mtu: u16,
}

impl AsyncL2capSocket {
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

    pub async fn connect_to_device(&mut self, device_addr: &str, addr_type: u8) -> GattResult<()> {
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

    pub async fn connect_with_le_psm(
        &mut self,
        device_addr: &str,
        addr_type: u8,
    ) -> GattResult<()> {
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

    async fn send_all_async(&self, data: &[u8]) -> GattResult<()> {
        let mut written = 0;
        while written < data.len() {
            let rc = unsafe {
                send(
                    self.fd,
                    data[written..].as_ptr().cast(),
                    data.len() - written,
                    0,
                )
            };
            if rc < 0 {
                return Err(GattError::SendFailed("send failed".to_string()));
            }
            written += rc as usize;
        }
        Ok(())
    }

    async fn recv_some_async(&self, buf: &mut [u8]) -> GattResult<usize> {
        let read = unsafe { recv(self.fd, buf.as_mut_ptr().cast(), buf.len(), 0) };
        if read < 0 {
            return Err(GattError::RecvFailed("recv failed".to_string()));
        }
        Ok(read as usize)
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
}

impl Drop for AsyncL2capSocket {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

unsafe impl Send for AsyncL2capSocket {}
unsafe impl Sync for AsyncL2capSocket {}

fn parse_bdaddr_string(addr: &str) -> Option<[u8; 6]> {
    edgerun_encoding::hex::parse_mac(addr)
}

fn format_bdaddr_hex(bytes: &[u8]) -> String {
    edgerun_encoding::hex::format_mac_bytes(bytes).unwrap_or_default()
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
    fn close(fd: i32) -> i32;
}

pub struct AsyncAttProtocol {
    socket: AsyncL2capSocket,
    mtu: u16,
    transaction_id: u8,
}

impl AsyncAttProtocol {
    pub fn new(socket: AsyncL2capSocket) -> Self {
        Self {
            socket,
            mtu: DEFAULT_MTU,
            transaction_id: 0,
        }
    }

    pub fn with_mtu(socket: AsyncL2capSocket, mtu: u16) -> Self {
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

    async fn send_recv_async(&mut self, req: &[u8]) -> GattResult<Vec<u8>> {
        self.socket.send_all_async(req).await?;
        let mut buf = vec![0u8; self.mtu as usize];
        let n = self.socket.recv_some_async(&mut buf).await?;
        buf.truncate(n);
        Ok(buf)
    }

    pub async fn exchange_mtu(&mut self, client_mtu: u16) -> GattResult<u16> {
        let mut req = vec![0x02];
        req.extend_from_slice(&client_mtu.to_le_bytes());
        let resp = self.send_recv_async(&req).await?;

        if resp.len() < 3 || resp[0] != 0x03 {
            return Err(GattError::MtuExchangeFailed);
        }

        let server_mtu = u16::from_le_bytes([resp[1], resp[2]]);
        self.mtu = std::cmp::min(client_mtu, server_mtu);
        Ok(self.mtu)
    }

    pub async fn read_by_group_type(
        &mut self,
        start: u16,
        end: u16,
        group_type: &[u8],
    ) -> GattResult<Vec<u8>> {
        let mut req = vec![0x10];
        req.extend_from_slice(&start.to_le_bytes());
        req.extend_from_slice(&end.to_le_bytes());
        req.extend_from_slice(group_type);
        self.send_recv_async(&req).await
    }

    pub async fn find_information(&mut self, start: u16, end: u16) -> GattResult<Vec<u8>> {
        let mut req = vec![0x04];
        req.extend_from_slice(&start.to_le_bytes());
        req.extend_from_slice(&end.to_le_bytes());
        self.send_recv_async(&req).await
    }

    pub async fn read_by_type(&mut self, start: u16, end: u16, uuid: &[u8]) -> GattResult<Vec<u8>> {
        let mut req = vec![0x08];
        req.extend_from_slice(&start.to_le_bytes());
        req.extend_from_slice(&end.to_le_bytes());
        req.extend_from_slice(uuid);
        self.send_recv_async(&req).await
    }

    pub async fn read_value(&mut self, handle: u16) -> GattResult<Vec<u8>> {
        let mut req = vec![0x0a];
        req.extend_from_slice(&handle.to_le_bytes());
        self.send_recv_async(&req).await
    }

    pub async fn read_blob(&mut self, handle: u16, offset: u16) -> GattResult<Vec<u8>> {
        let mut req = vec![0x0c];
        req.extend_from_slice(&handle.to_le_bytes());
        req.extend_from_slice(&offset.to_le_bytes());
        self.send_recv_async(&req).await
    }

    pub async fn write_value(
        &mut self,
        handle: u16,
        data: &[u8],
        with_response: bool,
    ) -> GattResult<()> {
        let opcode = if with_response { 0x12 } else { 0x52 };
        let mut req = vec![opcode];
        req.extend_from_slice(&handle.to_le_bytes());
        req.extend_from_slice(data);
        self.socket.send_all_async(&req).await?;

        if with_response {
            let mut buf = vec![0u8; self.mtu as usize];
            let n = self.socket.recv_some_async(&mut buf).await?;
            if n > 0 && buf[0] == 0x13 {
                return Ok(());
            } else if n > 1 {
                return Err(GattError::att_error_code(buf[1]));
            }
        }
        Ok(())
    }

    pub async fn write_cmd(&mut self, handle: u16, data: &[u8]) -> GattResult<()> {
        let mut req = vec![0x52];
        req.extend_from_slice(&handle.to_le_bytes());
        req.extend_from_slice(data);
        self.socket.send_all_async(&req).await
    }

    pub async fn prepare_write_value(
        &mut self,
        handle: u16,
        offset: u16,
        data: &[u8],
    ) -> GattResult<()> {
        let mut req = vec![0x16];
        req.extend_from_slice(&handle.to_le_bytes());
        req.extend_from_slice(&offset.to_le_bytes());
        req.extend_from_slice(data);
        self.socket.send_all_async(&req).await
    }

    pub async fn execute_write(&mut self, commit: bool) -> GattResult<()> {
        let flag = if commit { 0x01 } else { 0x00 };
        let req = vec![0x18, flag];
        self.socket.send_all_async(&req).await
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
        let entry_size = 2 + uuid_size;
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
    fn format_bdaddr_test() {
        let bytes = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let formatted = format_bdaddr_hex(&bytes);
        assert_eq!(formatted, "aa:bb:cc:dd:ee:ff");
    }

    #[test]
    fn async_l2cap_socket_state() {
        let socket = AsyncL2capSocket::new().unwrap();
        assert_eq!(socket.state(), L2capChannelState::Closed);
        assert!(!socket.is_connected());
    }

    #[test]
    fn async_att_protocol_mtu() {
        let socket = AsyncL2capSocket::new().unwrap();
        let proto = AsyncAttProtocol::new(socket);
        assert_eq!(proto.mtu(), 23);
    }

    #[test]
    fn parse_error_response() {
        let socket = AsyncL2capSocket::new().unwrap();
        let proto = AsyncAttProtocol::new(socket);
        let data = vec![0x01, 0x0A, 0x00, 0x0A];
        let result = proto.parse_error_response(&data);
        assert_eq!(result, Some((0x000A, 0x0A)));
    }

    #[test]
    fn parse_find_info_response() {
        let socket = AsyncL2capSocket::new().unwrap();
        let proto = AsyncAttProtocol::new(socket);
        let data = vec![0x05, 0x01, 0x03, 0x00, 0x03, 0x28];
        let results = proto.parse_find_information_response(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 0x0003);
        assert_eq!(results[0].1, vec![0x03, 0x28]);
    }

    #[test]
    fn parse_read_by_group_response() {
        let socket = AsyncL2capSocket::new().unwrap();
        let mut proto = AsyncAttProtocol::new(socket);
        proto.set_mtu(50);
        let data = vec![0x11, 0x06, 0x01, 0x00, 0x08, 0x00, 0x00, 0x28];
        let results = proto.parse_read_by_group_response(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 0x0001);
        assert_eq!(results[0].1, 0x0008);
    }
}
