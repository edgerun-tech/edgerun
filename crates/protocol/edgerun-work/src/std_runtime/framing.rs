use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::codec::{packet_bytes, packet_from_bytes};
use crate::protocol::{MAX_WORK_FRAME_LEN, WorkAck, WorkPacket};

pub fn read_work_packet(stream: &mut TcpStream) -> io::Result<WorkPacket> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len)?;
    let len = u32::from_be_bytes(len) as usize;
    if len == 0 || len > MAX_WORK_FRAME_LEN {
        return Err(io::Error::new(ErrorKind::InvalidData, "invalid work frame length"));
    }
    let mut bytes = vec![0u8; len];
    stream.read_exact(&mut bytes)?;
    packet_from_bytes(&bytes)
        .map_err(|error| io::Error::new(ErrorKind::InvalidData, format!("invalid work packet: {error:?}")))
}

pub fn write_work_packet(stream: &mut TcpStream, packet: &WorkPacket) -> io::Result<()> {
    let bytes = packet_bytes(packet)
        .map_err(|error| io::Error::new(ErrorKind::InvalidData, format!("invalid work packet: {error:?}")))?;
    write_encoded_work_packet(stream, &bytes)
}

pub fn write_encoded_work_packet(stream: &mut TcpStream, bytes: &[u8]) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_WORK_FRAME_LEN {
        return Err(io::Error::new(ErrorKind::InvalidData, "invalid work frame length"));
    }
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(bytes)?;
    stream.flush()
}

pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn ack(ok: bool, code: u16, text: &str) -> WorkPacket {
    WorkPacket::Ack(WorkAck {
        ok,
        code,
        text: text.to_owned(),
    })
}
