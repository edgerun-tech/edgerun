use crate::prelude::v1::*;
use crate::{GattError, GattResult};
use edgerun_protocols::bluetooth_gatt::{
    parse_bdaddr_string, parse_connection_complete, reverse_bdaddr,
};
use std::collections::HashMap;
use std::io;
use std::mem::size_of;
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const AF_BLUETOOTH: i32 = 31;
const SOCK_RAW: i32 = 3;
const BTPROTO_HCI: i32 = 1;
const HCI_CHANNEL_CONTROL: u16 = 3;
const HCI_DEV_NONE: u16 = 0xffff;

const HCI_OP_LE_CREATE_CONN: u16 = 0x200d;
const HCI_OP_LE_CONN_UPDATE: u16 = 0x2013;
const HCI_OP_DISCONNECT: u16 = 0x0406;
const HCI_OP_AUTH_PAYLOAD_TIMEOUT: u16 = 0x201B;

const HCI_EV_LE_META_EVENT: u8 = 0x3E;
const HCI_EV_CONN_COMPLETE: u8 = 0x13;
const HCI_EV_DISCONN_COMPLETE: u8 = 0x05;
const HCI_EV_LE_CONN_COMPLETE: u8 = 0x01;

const LE_CONN_COMPLETE_STATUS_SUCCESS: u8 = 0x00;

#[repr(C)]
#[derive(Clone, Copy)]
struct SockAddrHci {
    hci_family: u16,
    hci_dev: u16,
    hci_channel: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LeConnParams {
    pub scan_interval: u16,
    pub scan_window: u16,
    pub peer_addr_type: u8,
    pub conn_interval_min: u16,
    pub conn_interval_max: u16,
    pub conn_latency: u16,
    pub supervision_timeout: u16,
}

impl LeConnParams {
    pub const fn default_fast() -> Self {
        Self {
            scan_interval: 0x0060,
            scan_window: 0x0030,
            peer_addr_type: 0x01,
            conn_interval_min: 0x0018,
            conn_interval_max: 0x0028,
            conn_latency: 0x0000,
            supervision_timeout: 0x01C0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HciConnReq {
    scan_interval: u16,
    scan_window: u16,
    initiator_filter_policy: u8,
    peer_addr_type: u8,
    peer_addr: [u8; 6],
    own_addr_type: u8,
    conn_interval_min: u16,
    conn_interval_max: u16,
    conn_latency: u16,
    supervision_timeout: u16,
    min_ce_length: u16,
    max_ce_length: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HciDisconnReq {
    handle: u16,
    reason: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
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

pub struct HciConnection {
    fd: RawFd,
    controller_index: u16,
    handle: u16,
    connected: AtomicBool,
    peer_addr: Mutex<Option<String>>,
}

impl HciConnection {
    pub fn new(controller_index: u16) -> GattResult<Self> {
        let fd = unsafe { socket(AF_BLUETOOTH, SOCK_RAW, BTPROTO_HCI) };
        if fd < 0 {
            return Err(GattError::SocketFailed(
                "failed to create HCI socket".to_string(),
            ));
        }

        let addr = SockAddrHci {
            hci_family: AF_BLUETOOTH as u16,
            hci_dev: controller_index,
            hci_channel: HCI_CHANNEL_CONTROL,
        };

        let rc = unsafe {
            bind(
                fd,
                (&addr as *const SockAddrHci).cast(),
                size_of::<SockAddrHci>() as u32,
            )
        };

        if rc < 0 {
            unsafe { close(fd) };
            return Err(GattError::SocketFailed(
                "failed to bind HCI socket".to_string(),
            ));
        }

        Ok(Self {
            fd,
            controller_index,
            handle: 0,
            connected: AtomicBool::new(false),
            peer_addr: Mutex::new(None),
        })
    }

    pub fn connect_le(&mut self, peer_addr: &str, params: LeConnParams) -> GattResult<u16> {
        let bdaddr = reverse_bdaddr(peer_addr)
            .ok_or_else(|| GattError::InvalidAddress(peer_addr.to_string()))?;

        let mut req = Vec::new();
        req.extend_from_slice(&HCI_OP_LE_CREATE_CONN.to_le_bytes());
        req.push((size_of::<HciConnReq>() as u8).wrapping_sub(3));
        req.push(self.controller_index as u8);
        req.push((self.controller_index >> 8) as u8);

        let conn_req = HciConnReq {
            scan_interval: params.scan_interval,
            scan_window: params.scan_window,
            initiator_filter_policy: 0x00,
            peer_addr_type: params.peer_addr_type,
            peer_addr: bdaddr,
            own_addr_type: 0x00,
            conn_interval_min: params.conn_interval_min,
            conn_interval_max: params.conn_interval_max,
            conn_latency: params.conn_latency,
            supervision_timeout: params.supervision_timeout,
            min_ce_length: 0x0000,
            max_ce_length: 0x0000,
        };

        let req_bytes = unsafe {
            std::slice::from_raw_parts(
                (&conn_req as *const HciConnReq).cast::<u8>(),
                size_of::<HciConnReq>(),
            )
        };
        req.extend_from_slice(req_bytes);

        let rc = unsafe { send(self.fd, req.as_ptr().cast(), req.len(), 0) };
        if rc < 0 {
            return Err(GattError::SendFailed(
                "failed to send HCI command".to_string(),
            ));
        }

        self.peer_addr = Mutex::new(Some(peer_addr.to_string()));

        let handle = self.wait_for_connection_complete(10_000)?;
        self.handle = handle;
        self.connected.store(true, Ordering::SeqCst);
        Ok(handle)
    }

    fn wait_for_connection_complete(&self, timeout_ms: u32) -> GattResult<u16> {
        let start = std::time::Instant::now();
        let mut remaining = timeout_ms as i32;

        while remaining > 0 {
            let poll_ms = remaining.min(100);
            let mut pollfd = PollFd {
                fd: self.fd,
                events: 0x0001,
                revents: 0,
            };

            let poll_rc = unsafe { poll(&mut pollfd, 1, poll_ms) };
            if poll_rc < 0 {
                return Err(GattError::PollFailed(
                    "poll failed on HCI socket".to_string(),
                ));
            }

            if poll_rc > 0 {
                let mut buf = vec![0u8; 256];
                let read = unsafe { recv(self.fd, buf.as_mut_ptr().cast(), buf.len(), 0) };
                if read > 0 {
                    let buf = &buf[..read as usize];
                    if let Some((status, handle)) = self.parse_conn_complete(buf) {
                        if status == LE_CONN_COMPLETE_STATUS_SUCCESS {
                            return Ok(handle);
                        } else {
                            return Err(GattError::ConnectionFailed(format!(
                                "LE connection failed with status: 0x{:02x}",
                                status
                            )));
                        }
                    }
                }
            }

            remaining = (timeout_ms as i32) - (start.elapsed().as_millis() as i32);
        }

        Err(GattError::Timeout(timeout_ms))
    }

    fn parse_conn_complete(&self, data: &[u8]) -> Option<(u8, u16)> {
        parse_connection_complete(data)
    }

    pub fn disconnect(&mut self) -> GattResult<()> {
        if !self.connected.load(Ordering::SeqCst) {
            return Ok(());
        }

        let mut req = Vec::new();
        req.extend_from_slice(&HCI_OP_DISCONNECT.to_le_bytes());
        req.push(3);
        req.push(self.controller_index as u8);
        req.push((self.controller_index >> 8) as u8);

        let disconn_req = HciDisconnReq {
            handle: self.handle,
            reason: 0x13,
        };

        let req_bytes = unsafe {
            std::slice::from_raw_parts(
                (&disconn_req as *const HciDisconnReq).cast::<u8>(),
                size_of::<HciDisconnReq>(),
            )
        };
        req.extend_from_slice(req_bytes);

        let rc = unsafe { send(self.fd, req.as_ptr().cast(), req.len(), 0) };
        if rc < 0 {
            return Err(GattError::SendFailed(
                "failed to send disconnect command".to_string(),
            ));
        }

        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    pub fn handle(&self) -> u16 {
        self.handle
    }

    pub fn peer_address(&self) -> Option<String> {
        self.peer_addr.lock().unwrap().clone()
    }
}

impl Drop for HciConnection {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

unsafe impl Send for HciConnection {}
unsafe impl Sync for HciConnection {}

pub struct HciConnectionPool {
    controller_index: u16,
    connections: Mutex<HashMap<String, Arc<HciConnection>>>,
}

impl HciConnectionPool {
    pub fn new(controller_index: u16) -> GattResult<Self> {
        Ok(Self {
            controller_index,
            connections: Mutex::new(HashMap::new()),
        })
    }

    pub fn connect_le(&self, peer_addr: &str, addr_type: u8) -> GattResult<Arc<HciConnection>> {
        {
            let conns = self.connections.lock().unwrap();
            if let Some(existing) = conns.get(peer_addr) {
                if existing.is_connected() {
                    return Ok(existing.clone());
                }
            }
        }

        let mut hci = HciConnection::new(self.controller_index)?;
        let mut params = LeConnParams::default_fast();
        params.peer_addr_type = addr_type;
        let handle = hci.connect_le(peer_addr, params)?;

        let conn = Arc::new(hci);
        let mut conns = self.connections.lock().unwrap();
        conns.insert(peer_addr.to_string(), conn.clone());
        Ok(conn)
    }

    pub fn disconnect(&self, peer_addr: &str) -> GattResult<()> {
        let mut conns = self.connections.lock().unwrap();
        if let Some(conn) = conns.remove(peer_addr) {
            let mut hci = HciConnection::new(self.controller_index)?;
            hci.disconnect()?;
        }
        Ok(())
    }

    pub fn list_connections(&self) -> Vec<String> {
        let conns = self.connections.lock().unwrap();
        conns.keys().cloned().collect()
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
}
