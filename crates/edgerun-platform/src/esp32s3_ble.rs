//! Minimal `no_std` BLE HCI scaffold for ESP32-S3 bare-metal builds.
//!
//! This module does not depend on `esp-idf` and keeps a tiny, auditable API surface.
//! It currently provides:
//! - HCI packet encoding helpers (command + ACL)
//! - a parser for common HCI events and a small ATT-over-ACL parser path
//! - a transport trait and a stateful BLE peripheral wrapper
//! - a single place to evolve toward a full ESP32-S3 controller bring-up

#![allow(unsafe_op_in_unsafe_fn)]

use core::fmt;

pub const BLE_ADDR_LEN: usize = 6;
pub const HCI_MAX_PARAM_LEN: usize = 255;
pub const HCI_MAX_PACKET_LEN: usize = HCI_MAX_PARAM_LEN + 4;
pub const HCI_ACL_PAYLOAD_LEN: usize = 512;
pub const ATT_PAYLOAD_MAX: usize = 247;

pub const HCI_PACKET_EVENT: u8 = 0x04;
pub const HCI_PACKET_COMMAND: u8 = 0x01;
pub const HCI_PACKET_ACL: u8 = 0x02;

pub const HCI_CMD_COMPLETE: u8 = 0x0E;
pub const HCI_CMD_STATUS: u8 = 0x0F;
pub const HCI_LE_META_EVENT: u8 = 0x3E;
pub const HCI_LE_META_CONN_COMPLETE: u8 = 0x01;
pub const HCI_LE_META_DISCONNECT_COMPLETE: u8 = 0x05;
pub const HCI_LE_META_ADVERTISING_REPORT: u8 = 0x02;

pub const HCI_OPCODE_RESET: u16 = 0x0C03;
pub const HCI_OPCODE_SET_RANDOM_ADDR: u16 = 0x2005;
pub const HCI_OPCODE_LE_SET_ADV_PARAMS: u16 = 0x2006;
pub const HCI_OPCODE_LE_SET_ADV_DATA: u16 = 0x2008;
pub const HCI_OPCODE_LE_SET_ADV_ENABLE: u16 = 0x200A;
pub const HCI_OPCODE_LE_SET_SCAN_ENABLE: u16 = 0x200C;

pub const L2CAP_ATT_CID: u16 = 0x0004;
pub const ATT_OP_WRITE_REQ: u8 = 0x12;
pub const ATT_OP_WRITE_CMD: u8 = 0x52;
pub const ATT_OP_HANDLE_VALUE_NTF: u8 = 0x1B;
pub const ATT_OP_HANDLE_VALUE_IND: u8 = 0x1D;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BleAddress(pub [u8; BLE_ADDR_LEN]);

impl BleAddress {
    pub const fn new(raw: [u8; BLE_ADDR_LEN]) -> Self {
        Self(raw)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BleError {
    TransportUnavailable,
    PacketTooLarge,
    MalformedPacket,
    NotConnected,
    BufferOverflow,
}

pub type BleResult<T> = Result<T, BleError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeripheralState {
    ResetNeeded,
    Advertising,
    Connected,
    Idle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub conn_handle: u16,
    pub role: u8,
    pub peer: BleAddress,
}

pub trait BleLinkTransport {
    fn tx(&mut self, packet: &[u8]) -> bool;
    fn rx(&mut self, out: &mut [u8]) -> Option<usize>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HciEvent {
    CommandComplete {
        opcode: u16,
        status: u8,
    },
    CommandStatus {
        opcode: u16,
        status: u8,
    },
    LeConnectionComplete {
        status: u8,
        handle: u16,
        role: u8,
        peer_address: BleAddress,
    },
    LeDisconnectionComplete {
        status: u8,
        reason: u8,
        handle: u16,
    },
    LeAdvertisingReport {
        addr: BleAddress,
        rssi: i8,
    },
    Unknown {
        event_code: u8,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AttWriteEvent {
    pub conn_handle: u16,
    pub attr_handle: u16,
    pub value_len: usize,
    pub value: [u8; ATT_PAYLOAD_MAX],
}

impl fmt::Debug for AttWriteEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = &self.value[..self.value_len];
        f.debug_struct("AttWriteEvent")
            .field("conn_handle", &self.conn_handle)
            .field("attr_handle", &self.attr_handle)
            .field("value", &value)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollEvent {
    Hci(HciEvent),
    AttWrite(AttWriteEvent),
    RawAcl {
        conn_handle: u16,
        cid: u16,
        data_len: usize,
    },
}

pub struct Esp32s3BlePeripheral<R: BleLinkTransport> {
    transport: R,
    state: PeripheralState,
    conn: Option<ConnectionInfo>,
    mtu: u16,
}

impl<R: BleLinkTransport> Esp32s3BlePeripheral<R> {
    pub const fn new(transport: R) -> Self {
        Self {
            transport,
            state: PeripheralState::ResetNeeded,
            conn: None,
            mtu: 247,
        }
    }

    pub fn init(&mut self) -> BleResult<()> {
        self.send_hci_cmd(HCI_OPCODE_RESET, &[])?;
        self.state = PeripheralState::Idle;
        Ok(())
    }

    pub fn state(&self) -> PeripheralState {
        self.state
    }

    pub fn mtu(&self) -> u16 {
        self.mtu
    }

    pub fn connection(&self) -> Option<ConnectionInfo> {
        self.conn
    }

    pub fn set_mtu(&mut self, mtu: u16) {
        self.mtu = mtu;
    }

    pub fn start_advertising(&mut self) -> BleResult<()> {
        self.send_hci_cmd(HCI_OPCODE_LE_SET_ADV_ENABLE, &[0x01])?;
        self.state = PeripheralState::Advertising;
        Ok(())
    }

    pub fn stop_advertising(&mut self) -> BleResult<()> {
        self.send_hci_cmd(HCI_OPCODE_LE_SET_ADV_ENABLE, &[0x00])?;
        if self.state == PeripheralState::Advertising {
            self.state = PeripheralState::Idle;
        }
        Ok(())
    }

    pub fn start_scan_passthrough(&mut self) -> BleResult<()> {
        self.send_hci_cmd(HCI_OPCODE_LE_SET_SCAN_ENABLE, &[0x01, 0x00])?;
        Ok(())
    }

    pub fn set_adv_data(&mut self, payload: &[u8]) -> BleResult<()> {
        if payload.len() > 31 {
            return Err(BleError::PacketTooLarge);
        }
        let mut data = [0u8; 32];
        data[0] = payload.len() as u8 + 2;
        data[1] = 0x01;
        data[2] = 0x06;
        data[3..(payload.len() + 3)].copy_from_slice(payload);
        let params = &data[..payload.len() + 3];
        self.send_hci_cmd(HCI_OPCODE_LE_SET_ADV_DATA, params)?;
        Ok(())
    }

    pub fn poll(&mut self) -> Option<PollEvent> {
        let mut raw = [0u8; HCI_ACL_PAYLOAD_LEN];
        let len = self.transport.rx(&mut raw)?;
        let Some(raw_len) = parse_raw_packet(&raw[..len]) else {
            return None;
        };

        match raw_len {
            ParsedRawPacket::Event(event_code, payload) => {
                let event = parse_hci_event(event_code, payload)?;
                self.handle_event(event);
                Some(PollEvent::Hci(event))
            }
            ParsedRawPacket::Acl {
                conn_handle,
                cid,
                pdu,
                pdu_len,
            } => {
                if cid == L2CAP_ATT_CID {
                    match parse_att_write(conn_handle, pdu, pdu_len) {
                        Some(ev) => Some(PollEvent::AttWrite(ev)),
                        None => Some(PollEvent::RawAcl {
                            conn_handle,
                            cid,
                            data_len: pdu_len,
                        }),
                    }
                } else {
                    Some(PollEvent::RawAcl {
                        conn_handle,
                        cid,
                        data_len: pdu_len,
                    })
                }
            }
        }
    }

    pub fn send_notification(
        &mut self,
        conn_handle: u16,
        attr_handle: u16,
        value: &[u8],
    ) -> BleResult<()> {
        let mut att = [0u8; ATT_PAYLOAD_MAX + 3];
        if value.len() + 3 > ATT_PAYLOAD_MAX {
            return Err(BleError::PacketTooLarge);
        }
        att[0] = ATT_OP_HANDLE_VALUE_NTF;
        att[1] = (attr_handle & 0x00ff) as u8;
        att[2] = (attr_handle >> 8) as u8;
        att[3..(value.len() + 3)].copy_from_slice(value);

        self.send_att(conn_handle, L2CAP_ATT_CID, &att[..value.len() + 3])
    }

    pub fn send_indication(
        &mut self,
        conn_handle: u16,
        attr_handle: u16,
        value: &[u8],
    ) -> BleResult<()> {
        let mut att = [0u8; ATT_PAYLOAD_MAX + 3];
        if value.len() + 3 > ATT_PAYLOAD_MAX {
            return Err(BleError::PacketTooLarge);
        }
        att[0] = ATT_OP_HANDLE_VALUE_IND;
        att[1] = (attr_handle & 0x00ff) as u8;
        att[2] = (attr_handle >> 8) as u8;
        att[3..(value.len() + 3)].copy_from_slice(value);
        self.send_att(conn_handle, L2CAP_ATT_CID, &att[..value.len() + 3])
    }

    pub fn send_gatt_write_response(
        &mut self,
        conn_handle: u16,
        response_opcode: u8,
    ) -> BleResult<()> {
        let response = [response_opcode, 0x00];
        self.send_att(conn_handle, L2CAP_ATT_CID, &response)
    }

    pub fn raw_transport_mut(&mut self) -> &mut R {
        &mut self.transport
    }

    fn handle_event(&mut self, event: HciEvent) {
        match event {
            HciEvent::LeConnectionComplete {
                status: 0,
                handle,
                role,
                peer_address,
            } => {
                self.conn = Some(ConnectionInfo {
                    conn_handle: handle,
                    role,
                    peer: peer_address,
                });
                self.state = PeripheralState::Connected;
            }
            HciEvent::LeDisconnectionComplete { status: 0, .. } => {
                self.conn = None;
                self.state = PeripheralState::Idle;
            }
            _ => {}
        }
    }

    fn send_hci_cmd(&mut self, opcode: u16, params: &[u8]) -> BleResult<()> {
        if params.len() > HCI_MAX_PARAM_LEN {
            return Err(BleError::PacketTooLarge);
        }
        let mut packet = [0u8; HCI_MAX_PACKET_LEN];
        packet[0] = HCI_PACKET_COMMAND;
        packet[1] = (opcode & 0x00ff) as u8;
        packet[2] = (opcode >> 8) as u8;
        packet[3] = params.len() as u8;
        packet[4..(params.len() + 4)].copy_from_slice(params);
        let full_len = params.len() + 4;
        if !self.transport.tx(&packet[..full_len]) {
            return Err(BleError::TransportUnavailable);
        }
        Ok(())
    }

    fn send_acl(&mut self, conn_handle: u16, payload: &[u8]) -> BleResult<()> {
        let data_len = payload.len();
        if data_len > HCI_MAX_PARAM_LEN {
            return Err(BleError::PacketTooLarge);
        }
        let mut packet = [0u8; HCI_MAX_PACKET_LEN];
        packet[0] = HCI_PACKET_ACL;
        let handle = conn_handle & 0x0fff;
        packet[1] = handle as u8;
        packet[2] = (handle >> 8) as u8;
        packet[3] = (data_len & 0x00ff) as u8;
        packet[4] = (data_len >> 8) as u8;
        packet[5..(data_len + 5)].copy_from_slice(&payload[..data_len]);
        if !self.transport.tx(&packet[..data_len + 5]) {
            return Err(BleError::TransportUnavailable);
        }
        Ok(())
    }

    fn send_att(&mut self, conn_handle: u16, cid: u16, payload: &[u8]) -> BleResult<()> {
        let mut l2cap = [0u8; ATT_PAYLOAD_MAX + 4];
        let att_len = payload.len();
        let l2cap_len = att_len + 2;
        if att_len > ATT_PAYLOAD_MAX || l2cap_len > u16::MAX as usize {
            return Err(BleError::PacketTooLarge);
        }
        l2cap[0] = (l2cap_len as u8) & 0xff;
        l2cap[1] = (l2cap_len >> 8) as u8;
        l2cap[2] = (cid & 0x00ff) as u8;
        l2cap[3] = (cid >> 8) as u8;
        l2cap[4..(att_len + 4)].copy_from_slice(payload);
        self.send_acl(conn_handle, &l2cap[..(att_len + 4)])
    }
}

enum ParsedRawPacket<'a> {
    Event(u8, &'a [u8]),
    Acl {
        conn_handle: u16,
        cid: u16,
        pdu: &'a [u8],
        pdu_len: usize,
    },
}

fn parse_att_write(conn_handle: u16, pdu: &[u8], pdu_len: usize) -> Option<AttWriteEvent> {
    if pdu_len < 3 {
        return None;
    }
    let op = pdu[0];
    if op != ATT_OP_WRITE_REQ && op != ATT_OP_WRITE_CMD {
        return None;
    }
    let attr_handle = u16::from_le_bytes([pdu[1], pdu[2]]);
    let value_len = pdu_len.saturating_sub(3);
    if value_len > ATT_PAYLOAD_MAX {
        return None;
    }

    let mut value = [0u8; ATT_PAYLOAD_MAX];
    value[..value_len].copy_from_slice(&pdu[3..pdu_len]);
    Some(AttWriteEvent {
        conn_handle,
        attr_handle,
        value_len,
        value,
    })
}

fn parse_raw_packet(raw: &[u8]) -> Option<ParsedRawPacket<'_>> {
    if raw.is_empty() {
        return None;
    }
    match raw[0] {
        HCI_PACKET_EVENT if raw.len() >= 3 => {
            let event_code = raw[1];
            let len = raw[2] as usize;
            if raw.len() < len + 3 {
                return None;
            }
            Some(ParsedRawPacket::Event(event_code, &raw[3..3 + len]))
        }
        HCI_PACKET_ACL if raw.len() >= 9 => {
            let handle_flags = u16::from_le_bytes([raw[1], raw[2]]) & 0x0fff;
            let plen = u16::from_le_bytes([raw[3], raw[4]]) as usize;
            if raw.len() < 5 + plen || plen < 4 {
                return None;
            }
            let l2cap_len = u16::from_le_bytes([raw[5], raw[6]]) as usize;
            let cid = u16::from_le_bytes([raw[7], raw[8]]);
            if 4 + l2cap_len > plen || raw.len() < 9 + l2cap_len {
                return None;
            }
            let pdu_len = l2cap_len;
            let pdu = &raw[9..(9 + pdu_len)];
            Some(ParsedRawPacket::Acl {
                conn_handle: handle_flags,
                cid,
                pdu,
                pdu_len,
            })
        }
        _ => None,
    }
}

fn parse_hci_event(code: u8, payload: &[u8]) -> Option<HciEvent> {
    match code {
        HCI_CMD_COMPLETE => {
            if payload.len() < 4 {
                return None;
            }
            let opcode = u16::from_le_bytes([payload[1], payload[2]]);
            let status = payload[3];
            Some(HciEvent::CommandComplete { opcode, status })
        }
        HCI_CMD_STATUS => {
            if payload.len() < 4 {
                return None;
            }
            let opcode = u16::from_le_bytes([payload[1], payload[2]]);
            let status = payload[3];
            Some(HciEvent::CommandStatus { opcode, status })
        }
        HCI_LE_META_EVENT => {
            if payload.is_empty() {
                return None;
            }
            match payload[0] {
                HCI_LE_META_CONN_COMPLETE => {
                    if payload.len() < 15 {
                        return None;
                    }
                    let status = payload[1];
                    let role = payload[2];
                    let handle = u16::from_le_bytes([payload[3], payload[4]]);
                    let mut addr = [0u8; BLE_ADDR_LEN];
                    addr.copy_from_slice(&payload[7..13]);
                    Some(HciEvent::LeConnectionComplete {
                        status,
                        handle,
                        role,
                        peer_address: BleAddress(addr),
                    })
                }
                HCI_LE_META_DISCONNECT_COMPLETE => {
                    if payload.len() < 6 {
                        return None;
                    }
                    let status = payload[1];
                    let handle = u16::from_le_bytes([payload[2], payload[3]]);
                    let reason = payload[4];
                    Some(HciEvent::LeDisconnectionComplete {
                        status,
                        reason,
                        handle,
                    })
                }
                HCI_LE_META_ADVERTISING_REPORT => {
                    if payload.len() < 12 {
                        return None;
                    }
                    let mut idx = 2;
                    if payload.len() <= idx + 1 + 1 + BLE_ADDR_LEN + 1 {
                        return None;
                    }
                    let _event_type = payload[idx];
                    idx += 1;
                    let _addr_type = payload[idx];
                    idx += 1;
                    let mut addr = [0u8; BLE_ADDR_LEN];
                    addr.copy_from_slice(&payload[idx..idx + BLE_ADDR_LEN]);
                    idx += BLE_ADDR_LEN;
                    if payload.len() <= idx {
                        return None;
                    }
                    let data_len = payload[idx] as usize;
                    idx += 1;
                    if payload.len() < idx + data_len + 1 {
                        return None;
                    }
                    let rssi_index = idx + data_len;
                    Some(HciEvent::LeAdvertisingReport {
                        addr: BleAddress(addr),
                        rssi: payload[rssi_index] as i8,
                    })
                }
                _ => Some(HciEvent::Unknown {
                    event_code: payload[0],
                }),
            }
        }
        _ => Some(HciEvent::Unknown { event_code: code }),
    }
}
