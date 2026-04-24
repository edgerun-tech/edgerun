//! Local Tuya device discovery and control via UDP broadcast.
//!
//! Tuya devices listen on UDP port 6667 for discovery broadcasts.
//! After discovery, control is via TCP port 6668 with a simple protocol.
//!
//! This provides LOCAL-ONLY control - no cloud account, no internet required.

use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_rt::{AsyncUdpSocket, timeout, Elapsed};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

const TUYA_BROADCAST_ADDR: &str = "255.255.255.255";
const TUYA_DISCOVERY_PORT: u16 = 6667;
const TUYA_CONTROL_PORT: u16 = 6668;

const PROTOCOL_VERSION: &str = "3.3";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TuyaDevice {
    pub id: String,
    #[serde(skip)]
    pub key: Option<String>,
    pub ip: String,
    pub name: Option<String>,
    pub product_type: Option<String>,
    pub version: Option<String>,
    pub state: TuyaDeviceState,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TuyaDeviceState {
    pub on: Option<bool>,
    pub temperature: Option<i32>,
    pub mode: Option<String>,
    pub fan_speed: Option<String>,
    pub swing: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum TuyaCommand {
    #[serde(rename = "discovery")]
    Discovery { protocol_version: String },
    #[serde(rename = "control")]
    Control { devId: String, dps: edgerun_json::Value },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum TuyaResponse {
    #[serde(rename = "discovery")]
    Discovery {
        msg_id: String,
        devId: String,
        product_type: String,
        version: String,
        ability: Option<edgerun_json::Value>,
    },
    #[serde(rename = "control")]
    Control { devId: String, dps: edgerun_json::Value },
}

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

    pub async fn broadcast_discovery(&self, bind_addr: SocketAddr) -> Result<Vec<TuyaDevice>, std::io::Error> {
        let broadcast_addr: SocketAddr = format!("{}:{}", TUYA_BROADCAST_ADDR, TUYA_DISCOVERY_PORT)
            .parse()
            .unwrap();

        let socket = AsyncUdpSocket::bind(bind_addr)?;
        socket.set_broadcast(true)?;

        let request = edgerun_json::to_string(&TuyaCommand::Discovery {
            protocol_version: PROTOCOL_VERSION.to_string(),
        })?;

        socket.send_to(request.as_bytes(), broadcast_addr).await?;

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

                    if let Ok(response) = std::str::from_utf8(&buf[..size]) {
                        if let Ok(TuyaResponse::Discovery {
                            msg_id: _,
                            devId,
                            product_type,
                            version,
                            ..
                        }) = edgerun_json::from_str(response)
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
                }
                Ok(Err(e)) => {
                    return Err(e);
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

        Self { device, local_key: key }
    }

    pub fn device(&self) -> &TuyaDevice {
        &self.device
    }

    pub async fn send_command(&mut self, dps: edgerun_json::Value) -> Result<edgerun_json::Value, std::io::Error> {
        let ip = format!("{}:{}", self.device.ip, TUYA_CONTROL_PORT);
        let addr: SocketAddr = ip.parse().unwrap();

        let socket = AsyncUdpSocket::bind("0.0.0.0:0")?;
        socket.set_broadcast(true)?;

        let request = edgerun_json::to_string(&TuyaCommand::Control {
            devId: self.device.id.clone(),
            dps,
        })?;

        socket.send_to(request.as_bytes(), addr).await?;

        let mut buf = [0u8; 4096];
        let (size, _) = socket.recv_from(&mut buf).await?;

        let response = std::str::from_utf8(&buf[..size]).unwrap();
        let parsed: edgerun_json::Value = edgerun_json::from_str(response).unwrap();

        Ok(parsed)
    }

    pub async fn set_power(&mut self, on: bool) -> Result<(), std::io::Error> {
        let dps = edgerun_json::json!({ "1": on });
        self.send_command(dps).await?;
        self.device.state.on = Some(on);
        Ok(())
    }

    pub async fn set_temperature(&mut self, temp: i32) -> Result<(), std::io::Error> {
        let dps = edgerun_json::json!({ "2": temp });
        self.send_command(dps).await?;
        self.device.state.temperature = Some(temp);
        Ok(())
    }

    pub async fn set_mode(&mut self, mode: &str) -> Result<(), std::io::Error> {
        let dps = edgerun_json::json!({ "4": mode });
        self.send_command(dps).await?;
        self.device.state.mode = Some(mode.to_string());
        Ok(())
    }

    pub async fn set_fan_speed(&mut self, speed: &str) -> Result<(), std::io::Error> {
        let dps = edgerun_json::json!({ "5": speed });
        self.send_command(dps).await?;
        self.device.state.fan_speed = Some(speed.to_string());
        Ok(())
    }
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