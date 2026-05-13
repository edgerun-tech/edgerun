use crate::event::queue::EventQueue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EventType {
    Network = 1,
    Disk = 2,
    Timer = 3,
}

impl EventType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(EventType::Network),
            2 => Some(EventType::Disk),
            3 => Some(EventType::Timer),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NetworkSubtype {
    Connected = 1,
    Disconnected = 2,
    Received = 3,
    Error = 4,
}

impl NetworkSubtype {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(NetworkSubtype::Connected),
            2 => Some(NetworkSubtype::Disconnected),
            3 => Some(NetworkSubtype::Received),
            4 => Some(NetworkSubtype::Error),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DiskSubtype {
    ReadDone = 1,
    WriteDone = 2,
    Error = 3,
}

impl DiskSubtype {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(DiskSubtype::ReadDone),
            2 => Some(DiskSubtype::WriteDone),
            3 => Some(DiskSubtype::Error),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TimerSubtype {
    Fired = 1,
}

impl TimerSubtype {
    pub fn from_u8(_v: u8) -> Option<Self> {
        Some(TimerSubtype::Fired)
    }

    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub data: [u8; Self::MAX_DATA_LEN],
    pub data_len: usize,
}

impl Event {
    pub const MAX_DATA_LEN: usize = 4096;

    pub fn from_u8(v: u8) -> Option<EventType> {
        EventType::from_u8(v)
    }

    pub fn new(event_type: EventType, data: &[u8]) -> Option<Self> {
        if data.len() > Self::MAX_DATA_LEN {
            return None;
        }
        let mut buf = [0u8; Self::MAX_DATA_LEN];
        buf[..data.len()].copy_from_slice(data);
        Some(Self {
            event_type,
            data: buf,
            data_len: data.len(),
        })
    }

    pub fn network_connected(sock_id: u32) -> Self {
        let mut d = [0u8; Self::MAX_DATA_LEN];
        d[0..4].copy_from_slice(&sock_id.to_le_bytes());
        d[4] = NetworkSubtype::Connected.to_u8();
        Self {
            event_type: EventType::Network,
            data: d,
            data_len: 5,
        }
    }

    pub fn network_disconnected(sock_id: u32) -> Self {
        let mut d = [0u8; Self::MAX_DATA_LEN];
        d[0..4].copy_from_slice(&sock_id.to_le_bytes());
        d[4] = NetworkSubtype::Disconnected.to_u8();
        Self {
            event_type: EventType::Network,
            data: d,
            data_len: 5,
        }
    }

    pub fn network_received(sock_id: u32, payload: &[u8]) -> Option<Self> {
        if payload.len() > Self::MAX_DATA_LEN - 9 {
            return None;
        }
        let mut d = [0u8; Self::MAX_DATA_LEN];
        d[0..4].copy_from_slice(&sock_id.to_le_bytes());
        d[4] = NetworkSubtype::Received.to_u8();
        d[5..9].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        d[9..9 + payload.len()].copy_from_slice(payload);
        Some(Self {
            event_type: EventType::Network,
            data: d,
            data_len: 9 + payload.len(),
        })
    }

    pub fn network_error(sock_id: u32) -> Self {
        let mut d = [0u8; Self::MAX_DATA_LEN];
        d[0..4].copy_from_slice(&sock_id.to_le_bytes());
        d[4] = NetworkSubtype::Error.to_u8();
        Self {
            event_type: EventType::Network,
            data: d,
            data_len: 5,
        }
    }

    pub fn disk_read_done(op_id: u32, data: &[u8]) -> Option<Self> {
        if data.len() > Self::MAX_DATA_LEN - 5 {
            return None;
        }
        let mut d = [0u8; Self::MAX_DATA_LEN];
        d[0..4].copy_from_slice(&op_id.to_le_bytes());
        d[4] = DiskSubtype::ReadDone.to_u8();
        d[5..5 + data.len()].copy_from_slice(data);
        Some(Self {
            event_type: EventType::Disk,
            data: d,
            data_len: 5 + data.len(),
        })
    }

    pub fn disk_write_done(op_id: u32) -> Self {
        let mut d = [0u8; Self::MAX_DATA_LEN];
        d[0..4].copy_from_slice(&op_id.to_le_bytes());
        d[4] = DiskSubtype::WriteDone.to_u8();
        Self {
            event_type: EventType::Disk,
            data: d,
            data_len: 5,
        }
    }

    pub fn disk_error(op_id: u32) -> Self {
        let mut d = [0u8; Self::MAX_DATA_LEN];
        d[0..4].copy_from_slice(&op_id.to_le_bytes());
        d[4] = DiskSubtype::Error.to_u8();
        Self {
            event_type: EventType::Disk,
            data: d,
            data_len: 5,
        }
    }

    pub fn timer_fired(timer_id: u64) -> Self {
        let mut d = [0u8; Self::MAX_DATA_LEN];
        d[0] = TimerSubtype::Fired.to_u8();
        d[1..9].copy_from_slice(&timer_id.to_le_bytes());
        Self {
            event_type: EventType::Timer,
            data: d,
            data_len: 9,
        }
    }

    pub fn to_bytes(&self) -> [u8; Self::MAX_DATA_LEN + 1] {
        let mut out = [0u8; Self::MAX_DATA_LEN + 1];
        out[0] = self.event_type.to_u8();
        out[1..1 + self.data_len].copy_from_slice(&self.data[..self.data_len]);
        out
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.data_len]
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }
        let event_type = EventType::from_u8(bytes[0])?;
        let data_len = bytes.len() - 1;
        if data_len > Self::MAX_DATA_LEN {
            return None;
        }
        let mut data = [0u8; Self::MAX_DATA_LEN];
        data[..data_len].copy_from_slice(&bytes[1..]);
        Some(Self {
            event_type,
            data,
            data_len,
        })
    }

    pub fn as_raw_bytes(&self) -> [u8; Self::MAX_DATA_LEN + 1] {
        let mut out = [0u8; Self::MAX_DATA_LEN + 1];
        out[0] = self.event_type.to_u8();
        out[1..1 + self.data_len].copy_from_slice(&self.data[..self.data_len]);
        out
    }

    pub fn total_len(&self) -> usize {
        1 + self.data_len
    }

    pub fn sock_id(&self) -> Option<u32> {
        if self.event_type != EventType::Network || self.data_len < 4 {
            return None;
        }
        Some(u32::from_le_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
        ]))
    }

    pub fn network_subtype(&self) -> Option<NetworkSubtype> {
        if self.event_type != EventType::Network || self.data_len < 5 {
            return None;
        }
        NetworkSubtype::from_u8(self.data[4])
    }

    pub fn network_payload(&self) -> Option<&[u8]> {
        if self.event_type != EventType::Network
            || self.data_len < 9
            || self.network_subtype()? != NetworkSubtype::Received
        {
            return None;
        }
        let payload_len =
            u32::from_le_bytes([self.data[5], self.data[6], self.data[7], self.data[8]]) as usize;
        if self.data_len < 9 + payload_len {
            return None;
        }
        Some(&self.data[9..9 + payload_len])
    }

    pub fn disk_op_id(&self) -> Option<u32> {
        if self.event_type != EventType::Disk || self.data_len < 4 {
            return None;
        }
        Some(u32::from_le_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
        ]))
    }

    pub fn disk_subtype(&self) -> Option<DiskSubtype> {
        if self.event_type != EventType::Disk || self.data_len < 5 {
            return None;
        }
        DiskSubtype::from_u8(self.data[4])
    }

    pub fn disk_data(&self) -> Option<&[u8]> {
        if self.event_type != EventType::Disk
            || self.data_len < 5
            || self.disk_subtype()? != DiskSubtype::ReadDone
        {
            return None;
        }
        Some(&self.data[5..self.data_len])
    }

    pub fn timer_id(&self) -> Option<u64> {
        if self.event_type != EventType::Timer || self.data_len < 9 {
            return None;
        }
        Some(u64::from_le_bytes([
            self.data[1],
            self.data[2],
            self.data[3],
            self.data[4],
            self.data[5],
            self.data[6],
            self.data[7],
            self.data[8],
        ]))
    }
}

pub fn push_event_raw<const N: usize>(
    queue: &mut EventQueue<N>,
    event_type: u8,
    data: &[u8],
) -> bool {
    if data.len() > Event::MAX_DATA_LEN {
        return false;
    }
    queue.push(event_type, data)
}

pub fn push_network_connected<const N: usize>(queue: &mut EventQueue<N>, sock_id: u32) -> bool {
    let event = Event::network_connected(sock_id);
    queue.push(event.event_type.to_u8(), event.as_slice())
}

pub fn push_network_disconnected<const N: usize>(queue: &mut EventQueue<N>, sock_id: u32) -> bool {
    let event = Event::network_disconnected(sock_id);
    queue.push(event.event_type.to_u8(), event.as_slice())
}

pub fn push_network_received<const N: usize>(
    queue: &mut EventQueue<N>,
    sock_id: u32,
    payload: &[u8],
) -> bool {
    match Event::network_received(sock_id, payload) {
        Some(event) => queue.push(event.event_type.to_u8(), event.as_slice()),
        None => false,
    }
}

pub fn push_network_error<const N: usize>(queue: &mut EventQueue<N>, sock_id: u32) -> bool {
    let event = Event::network_error(sock_id);
    queue.push(event.event_type.to_u8(), event.as_slice())
}

pub fn push_disk_read_done<const N: usize>(
    queue: &mut EventQueue<N>,
    op_id: u32,
    data: &[u8],
) -> bool {
    match Event::disk_read_done(op_id, data) {
        Some(event) => queue.push(event.event_type.to_u8(), event.as_slice()),
        None => false,
    }
}

pub fn push_disk_write_done<const N: usize>(queue: &mut EventQueue<N>, op_id: u32) -> bool {
    let event = Event::disk_write_done(op_id);
    queue.push(event.event_type.to_u8(), event.as_slice())
}

pub fn push_disk_error<const N: usize>(queue: &mut EventQueue<N>, op_id: u32) -> bool {
    let event = Event::disk_error(op_id);
    queue.push(event.event_type.to_u8(), event.as_slice())
}

pub fn push_timer_fired<const N: usize>(queue: &mut EventQueue<N>, timer_id: u64) -> bool {
    let event = Event::timer_fired(timer_id);
    queue.push(event.event_type.to_u8(), event.as_slice())
}
