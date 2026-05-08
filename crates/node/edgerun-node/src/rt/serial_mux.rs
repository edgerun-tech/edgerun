//! Runtime serial multiplexer I/O.
//!
//! The frame format is implemented by `edgerun-protocols`; this module only
//! connects that codec to node-owned serial devices.

use core::sync::atomic::{AtomicU16, Ordering};

pub use edgerun_protocols::serial_mux::{
    DecodeError, Frame, ReceivedFrame, Receiver, CHANNEL_CONTROL, CHANNEL_LOG, CHANNEL_NET,
    CHANNEL_WIFI, MAX_FRAME_PAYLOAD,
};

static TX_SEQ: AtomicU16 = AtomicU16::new(0);

#[inline]
pub fn write(channel: u8, payload: &[u8]) {
    for chunk in payload.chunks(MAX_FRAME_PAYLOAD) {
        let seq = TX_SEQ.fetch_add(1, Ordering::Relaxed);
        write_with_seq(channel, seq, chunk);
    }
}

pub fn write_with_seq(channel: u8, seq: u16, payload: &[u8]) {
    for chunk in payload.chunks(MAX_FRAME_PAYLOAD) {
        edgerun_protocols::serial_mux::encode_frame(channel, seq, chunk, write_raw);
    }
}

pub fn poll<const N: usize>(
    receiver: &mut Receiver<N>,
) -> Result<Option<ReceivedFrame<N>>, DecodeError> {
    while let Some(byte) = read_raw_byte() {
        if let Some(frame) = receiver.push(byte)? {
            return Ok(Some(frame));
        }
    }
    Ok(None)
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
