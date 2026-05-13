//! Local Tuya device discovery and control via UDP broadcast.
//!
//! Tuya devices listen on UDP port 6667 for discovery broadcasts.
//! After discovery, control is via TCP port 6668 with a simple protocol.
//!
//! This provides LOCAL-ONLY control - no cloud account, no internet required.

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::str;
use core::time::Duration;
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_node::rt::{timeout, AsyncUdpSocket, Elapsed};

pub use edgerun_protocols::tuya::{
    control_request_bytes, decode_json_payload, decrypt_6699_payload, derive_v35_session_key,
    discovery_request_bytes, pack_55aa, pack_6699_with_iv, parse_55aa_body_len,
    parse_55aa_wire_message, parse_6699_body_len, parse_6699_wire_message, parse_response_bytes,
    strip_retcode, TuyaCommand, TuyaDevice, TuyaDeviceState, TuyaProtocolError, TuyaResponse,
    TuyaWireMessage, DP_QUERY_NEW, PREFIX_55AA, PREFIX_6699, PROTOCOL_VERSION, SESS_KEY_NEG_FINISH,
    SESS_KEY_NEG_RESP, SESS_KEY_NEG_START, SUFFIX_55AA, SUFFIX_6699, TUYA_BROADCAST_ADDR,
    TUYA_CONTROL_PORT, TUYA_DISCOVERY_PORT,
};

#[cfg(target_os = "none")]
use core::net::SocketAddr;
#[cfg(target_os = "none")]
use edgerun_node::rt::io::IoError;

#[cfg(not(target_os = "none"))]
type IoError = std::io::Error;
#[cfg(not(target_os = "none"))]
use std::net::SocketAddr;

pub struct TuyaDiscovery;

impl Default for TuyaDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl TuyaDiscovery {
    pub fn new() -> Self {
        Self
    }

    pub async fn broadcast_discovery(
        &self,
        bind_addr: SocketAddr,
    ) -> Result<Vec<TuyaDevice>, IoError> {
        let broadcast_addr: SocketAddr = format!("{}:{}", TUYA_BROADCAST_ADDR, TUYA_DISCOVERY_PORT)
            .parse()
            .unwrap();

        let socket = AsyncUdpSocket::bind(bind_addr).map_err(rt_error)?;
        socket.set_broadcast(true).map_err(rt_error)?;

        let request = discovery_request_bytes().map_err(json_error)?;

        socket
            .send_to(&request, broadcast_addr)
            .await
            .map_err(rt_error)?;

        let mut devices = Vec::new();
        let mut last_addr: Option<SocketAddr> = None;
        let mut buf = [0u8; 4096];

        for _ in 0..10 {
            match timeout(Duration::from_millis(500), socket.recv_from(&mut buf)).await {
                Ok(Ok((size, addr))) => {
                    if last_addr == Some(addr) {
                        continue;
                    }
                    last_addr = Some(addr);

                    if let Ok(TuyaResponse::Discovery {
                        msg_id: _,
                        devId,
                        product_type,
                        version,
                        ..
                    }) = parse_response_bytes(&buf[..size])
                    {
                        let ip = addr.ip().to_string();
                        devices.push(TuyaDevice {
                            id: devId,
                            key: None,
                            ip,
                            name: None,
                            product_type: Some(product_type),
                            version: Some(version),
                            state: TuyaDeviceState::default(),
                        });
                    }
                }
                Ok(Err(e)) => {
                    return Err(rt_error(e));
                }
                Err(Elapsed) => {
                    break;
                }
            }
        }

        Ok(devices)
    }
}

pub struct TuyaController {
    device: TuyaDevice,
    local_key: [u8; 16],
}

impl TuyaController {
    pub fn new(device: TuyaDevice, local_key: &str) -> Self {
        let key_bytes = local_key.as_bytes();
        let mut key = [0u8; 16];
        key[..key_bytes.len().min(16)].copy_from_slice(&key_bytes[..key_bytes.len().min(16)]);

        Self {
            device,
            local_key: key,
        }
    }

    pub fn device(&self) -> &TuyaDevice {
        &self.device
    }

    pub async fn send_command(
        &mut self,
        dps: edgerun_json::Value,
    ) -> Result<edgerun_json::Value, IoError> {
        let ip = format!("{}:{}", self.device.ip, TUYA_CONTROL_PORT);
        let addr: SocketAddr = ip.parse().unwrap();

        let socket = AsyncUdpSocket::bind("0.0.0.0:0").map_err(rt_error)?;
        socket.set_broadcast(true).map_err(rt_error)?;

        let request = control_request_bytes(&self.device.id, dps).map_err(json_error)?;

        socket.send_to(&request, addr).await.map_err(rt_error)?;

        let mut buf = [0u8; 4096];
        let (size, _) = socket.recv_from(&mut buf).await.map_err(rt_error)?;

        let response = str::from_utf8(&buf[..size]).unwrap();
        let tape = edgerun_json::parse_json_tape(response).unwrap();
        let parsed = tape
            .root(response)
            .and_then(|value| value.to_json_value())
            .unwrap_or(edgerun_json::Value::Null);

        Ok(parsed)
    }

    pub async fn set_power(&mut self, on: bool) -> Result<(), IoError> {
        let dps = edgerun_json::json!({ "1": on });
        self.send_command(dps).await?;
        self.device.state.on = Some(on);
        Ok(())
    }

    pub async fn set_temperature(&mut self, temp: i32) -> Result<(), IoError> {
        let dps = edgerun_json::json!({ "2": temp });
        self.send_command(dps).await?;
        self.device.state.temperature = Some(temp);
        Ok(())
    }

    pub async fn set_mode(&mut self, mode: &str) -> Result<(), IoError> {
        let dps = edgerun_json::json!({ "4": mode });
        self.send_command(dps).await?;
        self.device.state.mode = Some(mode.to_string());
        Ok(())
    }

    pub async fn set_fan_speed(&mut self, speed: &str) -> Result<(), IoError> {
        let dps = edgerun_json::json!({ "5": speed });
        self.send_command(dps).await?;
        self.device.state.fan_speed = Some(speed.to_string());
        Ok(())
    }
}

#[cfg(not(target_os = "none"))]
fn json_error(error: edgerun_json::JsonError) -> IoError {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
}

#[cfg(not(target_os = "none"))]
fn rt_error(error: edgerun_node::rt::IoError) -> IoError {
    match error {
        edgerun_node::rt::IoError::UnexpectedEof => {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, error)
        }
        edgerun_node::rt::IoError::WriteZero => {
            std::io::Error::new(std::io::ErrorKind::WriteZero, error)
        }
        edgerun_node::rt::IoError::Other(_) => {
            std::io::Error::new(std::io::ErrorKind::Other, error)
        }
    }
}

#[cfg(target_os = "none")]
fn json_error(_error: edgerun_json::JsonError) -> IoError {
    IoError::Other("invalid json")
}

#[cfg(target_os = "none")]
fn rt_error(error: edgerun_node::rt::IoError) -> IoError {
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuya_device_default_state() {
        let state = TuyaDeviceState::default();
        assert_eq!(state.on, None);
    }
}
