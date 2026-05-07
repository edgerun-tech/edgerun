//! HDMI-CEC message construction.
//!
//! This module owns CEC message bytes. Linux CEC ioctls, adapter discovery,
//! and display capability mapping stay in adapter/capability crates.

use crate::prelude::*;
use alloc::vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CecLogicalAddress {
    Tv = 0,
    Playback1 = 4,
    AudioSystem = 5,
    Playback2 = 8,
    Playback3 = 11,
    Unregistered = 15,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CecMessage {
    pub bytes: Vec<u8>,
}

pub fn cec_header(initiator: CecLogicalAddress, destination: CecLogicalAddress) -> u8 {
    ((initiator as u8) << 4) | destination as u8
}

pub fn image_view_on(initiator: CecLogicalAddress, destination: CecLogicalAddress) -> CecMessage {
    CecMessage {
        bytes: vec![cec_header(initiator, destination), 0x04],
    }
}

pub fn text_view_on(initiator: CecLogicalAddress, destination: CecLogicalAddress) -> CecMessage {
    CecMessage {
        bytes: vec![cec_header(initiator, destination), 0x0d],
    }
}

pub fn standby(initiator: CecLogicalAddress, destination: CecLogicalAddress) -> CecMessage {
    CecMessage {
        bytes: vec![cec_header(initiator, destination), 0x36],
    }
}

pub fn active_source(initiator: CecLogicalAddress, physical_address: u16) -> CecMessage {
    CecMessage {
        bytes: vec![
            cec_header(initiator, CecLogicalAddress::Unregistered),
            0x82,
            (physical_address >> 8) as u8,
            physical_address as u8,
        ],
    }
}

pub fn set_stream_path(initiator: CecLogicalAddress, physical_address: u16) -> CecMessage {
    CecMessage {
        bytes: vec![
            cec_header(initiator, CecLogicalAddress::Unregistered),
            0x86,
            (physical_address >> 8) as u8,
            physical_address as u8,
        ],
    }
}

pub fn wake_sequence(
    initiator: CecLogicalAddress,
    physical_address: Option<u16>,
) -> Vec<CecMessage> {
    let mut messages = vec![image_view_on(initiator, CecLogicalAddress::Tv)];
    if let Some(physical_address) = physical_address {
        messages.push(active_source(initiator, physical_address));
        messages.push(set_stream_path(initiator, physical_address));
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_encodes_initiator_and_destination() {
        assert_eq!(
            cec_header(CecLogicalAddress::Tv, CecLogicalAddress::Tv),
            0x00
        );
        assert_eq!(
            cec_header(CecLogicalAddress::Playback1, CecLogicalAddress::Tv),
            0x40
        );
        assert_eq!(
            cec_header(
                CecLogicalAddress::Unregistered,
                CecLogicalAddress::Unregistered
            ),
            0xff
        );
    }

    #[test]
    fn command_builders_emit_expected_bytes() {
        assert_eq!(
            image_view_on(CecLogicalAddress::Playback1, CecLogicalAddress::Tv).bytes,
            vec![0x40, 0x04]
        );
        assert_eq!(
            standby(CecLogicalAddress::Playback1, CecLogicalAddress::Tv).bytes,
            vec![0x40, 0x36]
        );
        assert_eq!(
            active_source(CecLogicalAddress::Playback1, 0x2100).bytes,
            vec![0x4f, 0x82, 0x21, 0x00]
        );
    }
}
