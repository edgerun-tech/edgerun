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
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::str;
use core::time::Duration;
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_json::{FromJson, JsonValue, JsonValueError, Map, ToJson};
use edgerun_rt::{timeout, AsyncUdpSocket, Elapsed};

#[cfg(target_os = "none")]
use core::net::SocketAddr;
#[cfg(target_os = "none")]
use edgerun_rt::io::IoError;

#[cfg(not(target_os = "none"))]
type IoError = std::io::Error;
#[cfg(not(target_os = "none"))]
use std::net::SocketAddr;

const TUYA_BROADCAST_ADDR: &str = "255.255.255.255";
const TUYA_DISCOVERY_PORT: u16 = 6667;
const TUYA_CONTROL_PORT: u16 = 6668;

const PROTOCOL_VERSION: &str = "3.3";

#[derive(Clone, Debug, PartialEq)]
pub struct TuyaDevice {
    pub id: String,
    pub key: Option<String>,
    pub ip: String,
    pub name: Option<String>,
    pub product_type: Option<String>,
    pub version: Option<String>,
    pub state: TuyaDeviceState,
}

impl ToJson for TuyaDevice {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        object.push_field("id", &self.id);
        object.push_field("ip", &self.ip);
        object.push_opt_field("name", self.name.as_ref());
        object.push_opt_field("product_type", self.product_type.as_ref());
        object.push_opt_field("version", self.version.as_ref());
        object.push_field("state", self.state.to_json());
        object.into()
    }
}

impl FromJson for TuyaDevice {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = match value {
            JsonValue::Object(object) => object,
            _ => {
                return Err(JsonValueError::WrongType(format!(
                    "expected tuya device object"
                )))
            }
        };
        Ok(Self {
            id: FromJson::from_json(
                object
                    .remove("id")
                    .ok_or_else(|| JsonValueError::WrongType(format!("missing field `id`")))?,
            )?,
            key: None,
            ip: FromJson::from_json(
                object
                    .remove("ip")
                    .ok_or_else(|| JsonValueError::WrongType(format!("missing field `ip`")))?,
            )?,
            name: object.remove("name").map(FromJson::from_json).transpose()?,
            product_type: object
                .remove("product_type")
                .map(FromJson::from_json)
                .transpose()?,
            version: object
                .remove("version")
                .map(FromJson::from_json)
                .transpose()?,
            state: FromJson::from_json(
                object
                    .remove("state")
                    .ok_or_else(|| JsonValueError::WrongType(format!("missing field `state`")))?,
            )?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TuyaDeviceState {
    pub on: Option<bool>,
    pub temperature: Option<i32>,
    pub mode: Option<String>,
    pub fan_speed: Option<String>,
    pub swing: Option<String>,
}

edgerun_json::impl_json_struct! {
    TuyaDeviceState {
        required {}
        optional {
            on: "on" => bool,
            temperature: "temperature" => i32,
            mode: "mode" => String,
            fan_speed: "fan_speed" => String,
            swing: "swing" => String,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TuyaCommand {
    Discovery {
        protocol_version: String,
    },
    Control {
        devId: String,
        dps: edgerun_json::Value,
    },
}

impl ToJson for TuyaCommand {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        match self {
            Self::Discovery { protocol_version } => {
                object.push_field("action", "discovery");
                object.push_field("protocol_version", protocol_version);
            }
            Self::Control { devId, dps } => {
                object.push_field("action", "control");
                object.push_field("devId", devId);
                object.push_field("dps", dps.clone());
            }
        }
        object.into()
    }
}

#[derive(Clone, Debug)]
pub enum TuyaResponse {
    Discovery {
        msg_id: String,
        devId: String,
        product_type: String,
        version: String,
        ability: Option<edgerun_json::Value>,
    },
    Control {
        devId: String,
        dps: edgerun_json::Value,
    },
}

impl FromJson for TuyaResponse {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = match value {
            JsonValue::Object(object) => object,
            _ => {
                return Err(JsonValueError::WrongType(format!(
                    "expected tuya response object"
                )))
            }
        };
        let action = String::from_json(
            object
                .remove("action")
                .ok_or_else(|| JsonValueError::WrongType(format!("missing field `action`")))?,
        )?;
        match action.as_str() {
            "discovery" => {
                Ok(Self::Discovery {
                    msg_id: FromJson::from_json(object.remove("msg_id").ok_or_else(|| {
                        JsonValueError::WrongType(format!("missing field `msg_id`"))
                    })?)?,
                    devId: FromJson::from_json(object.remove("devId").ok_or_else(|| {
                        JsonValueError::WrongType(format!("missing field `devId`"))
                    })?)?,
                    product_type: FromJson::from_json(object.remove("product_type").ok_or_else(
                        || JsonValueError::WrongType(format!("missing field `product_type`")),
                    )?)?,
                    version: FromJson::from_json(object.remove("version").ok_or_else(|| {
                        JsonValueError::WrongType(format!("missing field `version`"))
                    })?)?,
                    ability: object
                        .remove("ability")
                        .map(FromJson::from_json)
                        .transpose()?,
                })
            }
            "control" => {
                Ok(Self::Control {
                    devId: FromJson::from_json(object.remove("devId").ok_or_else(|| {
                        JsonValueError::WrongType(format!("missing field `devId`"))
                    })?)?,
                    dps: FromJson::from_json(object.remove("dps").ok_or_else(|| {
                        JsonValueError::WrongType(format!("missing field `dps`"))
                    })?)?,
                })
            }
            other => Err(JsonValueError::WrongType(format!(
                "unknown tuya response action `{other}`"
            ))),
        }
    }
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

    pub async fn broadcast_discovery(
        &self,
        bind_addr: SocketAddr,
    ) -> Result<Vec<TuyaDevice>, IoError> {
        let broadcast_addr: SocketAddr = format!("{}:{}", TUYA_BROADCAST_ADDR, TUYA_DISCOVERY_PORT)
            .parse()
            .unwrap();

        let socket = AsyncUdpSocket::bind(bind_addr)?;
        socket.set_broadcast(true)?;

        let request = TuyaCommand::Discovery {
            protocol_version: PROTOCOL_VERSION.to_string(),
        }
        .to_json()
        .to_json_string()
        .map_err(json_error)?;

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

                    if let Ok(response) = str::from_utf8(&buf[..size]) {
                        if let Ok(TuyaResponse::Discovery {
                            msg_id: _,
                            devId,
                            product_type,
                            version,
                            ..
                        }) = edgerun_json::from_json_str(response)
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

        let socket = AsyncUdpSocket::bind("0.0.0.0:0")?;
        socket.set_broadcast(true)?;

        let request = TuyaCommand::Control {
            devId: self.device.id.clone(),
            dps,
        }
        .to_json()
        .to_json_string()
        .map_err(json_error)?;

        socket.send_to(request.as_bytes(), addr).await?;

        let mut buf = [0u8; 4096];
        let (size, _) = socket.recv_from(&mut buf).await?;

        let response = str::from_utf8(&buf[..size]).unwrap();
        let parsed = edgerun_json::parse_json(response).unwrap();

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

#[cfg(target_os = "none")]
fn json_error(_error: edgerun_json::JsonError) -> IoError {
    IoError::Other("invalid json")
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
