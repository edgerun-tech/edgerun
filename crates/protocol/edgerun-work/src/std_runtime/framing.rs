use std::io::{self, ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::codec::{
    archived_packet_frame_from_bytes, packet_bytes, packet_from_bytes, ArchivedWorkPacketFrame,
};
use crate::protocol::{WorkAck, WorkPacket, MAX_WORK_FRAME_LEN};
use crate::roles::{WorkServiceResponse, ROLE_STATUS_ACCEPTED};

const SERVICE_RESPONSE_KIND_NONE: u8 = 0;
const SERVICE_RESPONSE_KIND_PACKET: u8 = 1;
const SERVICE_RESPONSE_KIND_BYTES: u8 = 2;

pub fn read_work_packet(stream: &mut TcpStream) -> io::Result<WorkPacket> {
    read_work_packet_frame(stream)?
        .into_packet()
        .map_err(invalid_packet_error)
}

pub fn read_work_packet_frame(stream: &mut TcpStream) -> io::Result<ArchivedWorkPacketFrame> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len)?;
    let len = u32::from_be_bytes(len) as usize;
    if len == 0 || len > MAX_WORK_FRAME_LEN {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "invalid work frame length",
        ));
    }
    let mut bytes = vec![0u8; len];
    stream.read_exact(&mut bytes)?;
    archived_packet_frame_from_bytes(&bytes).map_err(invalid_packet_error)
}

pub fn write_work_packet(stream: &mut TcpStream, packet: &WorkPacket) -> io::Result<()> {
    let bytes = packet_bytes(packet).map_err(invalid_packet_error)?;
    write_encoded_work_packet(stream, &bytes)
}

pub fn write_encoded_work_packet(stream: &mut TcpStream, bytes: &[u8]) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_WORK_FRAME_LEN {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "invalid work frame length",
        ));
    }
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(bytes)?;
    stream.flush()
}

pub fn send_work_service_request<A: ToSocketAddrs>(
    addr: A,
    packet: &WorkPacket,
) -> io::Result<WorkServiceResponse> {
    let mut stream = TcpStream::connect(addr)?;
    write_work_packet(&mut stream, packet)?;
    read_work_service_response(&mut stream)
}

pub fn read_work_service_response(stream: &mut TcpStream) -> io::Result<WorkServiceResponse> {
    let mut header = [0u8; 7];
    stream.read_exact(&mut header)?;
    let status = u16::from_be_bytes([header[0], header[1]]);
    let kind = header[2];
    let len = u32::from_be_bytes([header[3], header[4], header[5], header[6]]) as usize;
    if len > MAX_WORK_FRAME_LEN {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "invalid service response length",
        ));
    }
    let mut body = vec![0u8; len];
    if len > 0 {
        stream.read_exact(&mut body)?;
    }
    match kind {
        SERVICE_RESPONSE_KIND_NONE => Ok(WorkServiceResponse {
            status,
            packet: None,
            bytes: body,
        }),
        SERVICE_RESPONSE_KIND_PACKET => {
            let packet = packet_from_bytes(&body).map_err(invalid_packet_error)?;
            Ok(WorkServiceResponse {
                status,
                packet: Some(packet),
                bytes: Vec::new(),
            })
        }
        SERVICE_RESPONSE_KIND_BYTES => Ok(WorkServiceResponse {
            status,
            packet: None,
            bytes: body,
        }),
        _ => Err(io::Error::new(
            ErrorKind::InvalidData,
            "invalid service response kind",
        )),
    }
}

pub fn write_work_service_response(
    stream: &mut TcpStream,
    output: &WorkServiceResponse,
) -> io::Result<()> {
    let (kind, body) = if let Some(packet) = &output.packet {
        let bytes = packet_bytes(packet).map_err(invalid_packet_error)?;
        (SERVICE_RESPONSE_KIND_PACKET, bytes)
    } else if !output.bytes.is_empty() {
        (SERVICE_RESPONSE_KIND_BYTES, output.bytes.clone())
    } else {
        (SERVICE_RESPONSE_KIND_NONE, Vec::new())
    };
    if output.status != ROLE_STATUS_ACCEPTED && kind == SERVICE_RESPONSE_KIND_NONE {
        return write_raw_service_response(stream, output.status, SERVICE_RESPONSE_KIND_BYTES, &[]);
    }
    write_raw_service_response(stream, output.status, kind, &body)
}

fn write_raw_service_response(
    stream: &mut TcpStream,
    status: u16,
    kind: u8,
    body: &[u8],
) -> io::Result<()> {
    if body.len() > MAX_WORK_FRAME_LEN {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "invalid service response length",
        ));
    }
    stream.write_all(&status.to_be_bytes())?;
    stream.write_all(&[kind])?;
    if kind == SERVICE_RESPONSE_KIND_PACKET {
        write_encoded_work_packet(stream, body)
    } else {
        stream.write_all(&(body.len() as u32).to_be_bytes())?;
        stream.write_all(body)?;
        stream.flush()
    }
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

fn invalid_packet_error(error: impl core::fmt::Debug) -> io::Error {
    io::Error::new(
        ErrorKind::InvalidData,
        format!("invalid work packet: {error:?}"),
    )
}
