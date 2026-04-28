//! Framed serial multiplexer for shared debug/control/network links.
//!
//! The wire format is an HDLC-like byte stream:
//! `0x7e channel len_lo len_hi seq_lo seq_hi payload crc_lo crc_hi 0x7e`,
//! with `0x7e` and `0x7d` escaped as `0x7d, byte ^ 0x20`.

use core::sync::atomic::{AtomicU16, Ordering};

pub const CHANNEL_CONTROL: u8 = 0;
pub const CHANNEL_LOG: u8 = 1;
pub const CHANNEL_NET: u8 = 2;
pub const CHANNEL_WIFI: u8 = 3;

const FLAG: u8 = 0x7e;
const ESC: u8 = 0x7d;
const ESC_XOR: u8 = 0x20;
const HEADER_LEN: usize = 5;
const CRC_LEN: usize = 2;
const MAX_FRAME_PAYLOAD: usize = 1024;

static TX_SEQ: AtomicU16 = AtomicU16::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    BufferTooSmall,
    Crc,
    Length,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Frame<'a> {
    pub channel: u8,
    pub seq: u16,
    pub payload: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReceivedFrame<const N: usize> {
    pub channel: u8,
    pub seq: u16,
    len: usize,
    payload: [u8; N],
}

impl<const N: usize> ReceivedFrame<N> {
    pub fn payload(&self) -> &[u8] {
        &self.payload[..self.len]
    }
}

pub struct Receiver<const N: usize> {
    buf: [u8; N],
    len: usize,
    in_frame: bool,
    escaped: bool,
}

impl<const N: usize> Receiver<N> {
    pub const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
            in_frame: false,
            escaped: false,
        }
    }

    pub fn poll(&mut self) -> Result<Option<ReceivedFrame<N>>, DecodeError> {
        while let Some(byte) = read_raw_byte() {
            if byte == FLAG {
                if self.in_frame && self.len > 0 {
                    let frame = decode_owned(&self.buf[..self.len])?;
                    self.len = 0;
                    self.escaped = false;
                    return Ok(Some(frame));
                }
                self.in_frame = true;
                self.len = 0;
                self.escaped = false;
                continue;
            }
            if !self.in_frame {
                continue;
            }
            let decoded = if self.escaped {
                self.escaped = false;
                byte ^ ESC_XOR
            } else if byte == ESC {
                self.escaped = true;
                continue;
            } else {
                byte
            };
            if self.len == self.buf.len() {
                self.len = 0;
                self.in_frame = false;
                self.escaped = false;
                return Err(DecodeError::BufferTooSmall);
            }
            self.buf[self.len] = decoded;
            self.len += 1;
        }
        Ok(None)
    }
}

impl<const N: usize> Default for Receiver<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
pub fn write(channel: u8, payload: &[u8]) {
    for chunk in payload.chunks(MAX_FRAME_PAYLOAD) {
        let seq = TX_SEQ.fetch_add(1, Ordering::Relaxed);
        write_frame(channel, seq, chunk);
    }
}

pub fn write_with_seq(channel: u8, seq: u16, payload: &[u8]) {
    for chunk in payload.chunks(MAX_FRAME_PAYLOAD) {
        write_frame(channel, seq, chunk);
    }
}

fn write_frame(channel: u8, seq: u16, payload: &[u8]) {
    let len = payload.len() as u16;
    let mut crc = 0xffff;
    let mut writer = EncodedWriter::new();
    writer.write_raw(FLAG);
    writer.write_escaped_crc(channel, &mut crc);
    writer.write_escaped_crc(len as u8, &mut crc);
    writer.write_escaped_crc((len >> 8) as u8, &mut crc);
    writer.write_escaped_crc(seq as u8, &mut crc);
    writer.write_escaped_crc((seq >> 8) as u8, &mut crc);
    for &byte in payload {
        writer.write_escaped_crc(byte, &mut crc);
    }
    let crc = !crc;
    writer.write_escaped(crc as u8);
    writer.write_escaped((crc >> 8) as u8);
    writer.write_raw(FLAG);
    writer.flush();
}

pub fn decode<'a>(encoded: &[u8], scratch: &'a mut [u8]) -> Result<Frame<'a>, DecodeError> {
    let mut out_len = 0;
    let mut in_frame = false;
    let mut escaped = false;

    for &byte in encoded {
        if byte == FLAG {
            if in_frame && out_len > 0 {
                break;
            }
            in_frame = true;
            out_len = 0;
            escaped = false;
            continue;
        }
        if !in_frame {
            continue;
        }
        let decoded = if escaped {
            escaped = false;
            byte ^ ESC_XOR
        } else if byte == ESC {
            escaped = true;
            continue;
        } else {
            byte
        };
        if out_len == scratch.len() {
            return Err(DecodeError::BufferTooSmall);
        }
        scratch[out_len] = decoded;
        out_len += 1;
    }

    decode_inner(&scratch[..out_len])
}

fn decode_inner(raw: &[u8]) -> Result<Frame<'_>, DecodeError> {
    if raw.len() < HEADER_LEN + CRC_LEN {
        return Err(DecodeError::Length);
    }
    let frame_without_crc = raw.len() - CRC_LEN;
    let expected = u16::from_le_bytes([raw[frame_without_crc], raw[frame_without_crc + 1]]);
    if crc16(&raw[..frame_without_crc]) != expected {
        return Err(DecodeError::Crc);
    }
    let len = u16::from_le_bytes([raw[1], raw[2]]) as usize;
    if frame_without_crc != HEADER_LEN + len {
        return Err(DecodeError::Length);
    }
    let seq = u16::from_le_bytes([raw[3], raw[4]]);
    Ok(Frame {
        channel: raw[0],
        seq,
        payload: &raw[HEADER_LEN..HEADER_LEN + len],
    })
}

fn decode_owned<const N: usize>(raw: &[u8]) -> Result<ReceivedFrame<N>, DecodeError> {
    let frame = decode_inner(raw)?;
    if frame.payload.len() > N {
        return Err(DecodeError::BufferTooSmall);
    }
    let mut payload = [0; N];
    payload[..frame.payload.len()].copy_from_slice(frame.payload);
    Ok(ReceivedFrame {
        channel: frame.channel,
        seq: frame.seq,
        len: frame.payload.len(),
        payload,
    })
}

struct EncodedWriter {
    buf: [u8; 64],
    len: usize,
}

impl EncodedWriter {
    const fn new() -> Self {
        Self {
            buf: [0; 64],
            len: 0,
        }
    }

    #[inline]
    fn write_escaped_crc(&mut self, byte: u8, crc: &mut u16) {
        *crc = crc16_update(*crc, byte);
        self.write_escaped(byte);
    }

    #[inline]
    fn write_escaped(&mut self, byte: u8) {
        if byte == FLAG || byte == ESC {
            self.write_raw(ESC);
            self.write_raw(byte ^ ESC_XOR);
        } else {
            self.write_raw(byte);
        }
    }

    #[inline]
    fn write_raw(&mut self, byte: u8) {
        if self.len == self.buf.len() {
            self.flush();
        }
        self.buf[self.len] = byte;
        self.len += 1;
    }

    #[inline]
    fn flush(&mut self) {
        if self.len == 0 {
            return;
        }
        write_raw(&self.buf[..self.len]);
        self.len = 0;
    }
}

fn crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0xffff;
    for &byte in bytes {
        crc = crc16_update(crc, byte);
    }
    !crc
}

fn crc16_update(mut crc: u16, byte: u8) -> u16 {
    crc ^= byte as u16;
    let mut bit = 0;
    while bit < 8 {
        if crc & 1 != 0 {
            crc = (crc >> 1) ^ 0x8408;
        } else {
            crc >>= 1;
        }
        bit += 1;
    }
    crc
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
#[inline]
fn write_raw(bytes: &[u8]) {
    unsafe {
        edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(bytes);
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
#[inline]
fn write_raw(bytes: &[u8]) {
    for &byte in bytes {
        unsafe {
            core::arch::asm!("out dx, al", in("al") byte, in("dx") 0x3f8u16);
        }
    }
}

#[cfg(not(any(
    all(target_arch = "xtensa", target_os = "none"),
    all(target_arch = "x86_64", target_os = "none")
)))]
#[inline]
fn write_raw(_bytes: &[u8]) {}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
#[inline]
fn read_raw_byte() -> Option<u8> {
    unsafe { edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_read_byte() }
}

#[cfg(not(all(target_arch = "xtensa", target_os = "none")))]
#[inline]
fn read_raw_byte() -> Option<u8> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_for_test(channel: u8, seq: u16, payload: &[u8], out: &mut [u8]) -> usize {
        let len = payload.len() as u16;
        let mut raw = [0u8; 64];
        raw[0] = channel;
        raw[1..3].copy_from_slice(&len.to_le_bytes());
        raw[3..5].copy_from_slice(&seq.to_le_bytes());
        raw[5..5 + payload.len()].copy_from_slice(payload);
        let crc = crc16(&raw[..5 + payload.len()]);
        raw[5 + payload.len()..7 + payload.len()].copy_from_slice(&crc.to_le_bytes());

        let mut written = 0;
        out[written] = FLAG;
        written += 1;
        for &byte in &raw[..7 + payload.len()] {
            if byte == FLAG || byte == ESC {
                out[written] = ESC;
                out[written + 1] = byte ^ ESC_XOR;
                written += 2;
            } else {
                out[written] = byte;
                written += 1;
            }
        }
        out[written] = FLAG;
        written + 1
    }

    #[test]
    fn decodes_escaped_frame() {
        let payload = [0x7e, 0x7d, 0x00, 0xff];
        let mut encoded = [0u8; 64];
        let len = encode_for_test(CHANNEL_LOG, 42, &payload, &mut encoded);
        let mut scratch = [0u8; 64];

        let frame = decode(&encoded[..len], &mut scratch).unwrap();

        assert_eq!(frame.channel, CHANNEL_LOG);
        assert_eq!(frame.seq, 42);
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn rejects_bad_crc() {
        let mut encoded = [0u8; 64];
        let len = encode_for_test(CHANNEL_NET, 7, b"abc", &mut encoded);
        encoded[len - 3] ^= 1;
        let mut scratch = [0u8; 64];

        assert_eq!(decode(&encoded[..len], &mut scratch), Err(DecodeError::Crc));
    }

    #[test]
    fn receiver_accepts_streamed_frame() {
        let payload = b"hello";
        let mut encoded = [0u8; 64];
        let len = encode_for_test(CHANNEL_CONTROL, 9, payload, &mut encoded);
        let mut rx = Receiver::<64>::new();

        for &byte in &encoded[..len - 1] {
            assert!(rx.push_for_test(byte).unwrap().is_none());
        }
        let frame = rx.push_for_test(encoded[len - 1]).unwrap().unwrap();

        assert_eq!(frame.channel, CHANNEL_CONTROL);
        assert_eq!(frame.seq, 9);
        assert_eq!(frame.payload(), payload);
    }

    impl<const N: usize> Receiver<N> {
        fn push_for_test(&mut self, byte: u8) -> Result<Option<ReceivedFrame<N>>, DecodeError> {
            if byte == FLAG {
                if self.in_frame && self.len > 0 {
                    let frame = decode_owned(&self.buf[..self.len])?;
                    self.len = 0;
                    self.escaped = false;
                    return Ok(Some(frame));
                }
                self.in_frame = true;
                self.len = 0;
                self.escaped = false;
                return Ok(None);
            }
            if !self.in_frame {
                return Ok(None);
            }
            let decoded = if self.escaped {
                self.escaped = false;
                byte ^ ESC_XOR
            } else if byte == ESC {
                self.escaped = true;
                return Ok(None);
            } else {
                byte
            };
            if self.len == self.buf.len() {
                return Err(DecodeError::BufferTooSmall);
            }
            self.buf[self.len] = decoded;
            self.len += 1;
            Ok(None)
        }
    }
}
