use crate::prelude::v1::*;
use crate::tlv::TlvWriter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcMode {
    Off = 0,
    Cool = 1,
    Heat = 2,
    Auto = 3,
    Dry = 4,
    FanOnly = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanSpeed {
    Auto = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Turbo = 4,
    Silent = 5,
}

#[derive(Debug, Clone)]
pub struct AcState {
    pub power: bool,
    pub mode: AcMode,
    pub target_temperature: f32,
    pub fan_speed: FanSpeed,
    pub vertical_swing: bool,
    pub horizontal_swing: bool,
}

impl Default for AcState {
    fn default() -> Self {
        Self {
            power: false,
            mode: AcMode::Cool,
            target_temperature: 24.0,
            fan_speed: FanSpeed::Auto,
            vertical_swing: false,
            horizontal_swing: false,
        }
    }
}

pub struct AcMatterClient {
    state: AcState,
}

impl AcMatterClient {
    pub fn new() -> Self {
        Self {
            state: AcState::default(),
        }
    }

    pub fn with_state(state: AcState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &AcState {
        &self.state
    }

    pub fn set_power(&mut self, on: bool) -> MatterRequest {
        self.state.power = on;
        let mut writer = TlvWriter::new();
        writer.write_bool(on);
        MatterRequest {
            endpoint: 1,
            cluster: CLUSTER_THERMOSTAT,
            attribute: ATTRIBUTE_ON_OFF,
            data: writer.into_bytes(),
        }
    }

    pub fn set_temperature(&mut self, temp: f32) -> MatterRequest {
        self.state.target_temperature = temp;
        let value = (temp * 100.0) as i32;
        let mut writer = TlvWriter::new();
        writer.write_i32(value);
        MatterRequest {
            endpoint: 1,
            cluster: CLUSTER_THERMOSTAT,
            attribute: ATTRIBUTE_TARGET_TEMP,
            data: writer.into_bytes(),
        }
    }

    pub fn set_mode(&mut self, mode: AcMode) -> MatterRequest {
        self.state.mode = mode;
        let mut writer = TlvWriter::new();
        writer.write_i32(mode as i32);
        MatterRequest {
            endpoint: 1,
            cluster: CLUSTER_FAN,
            attribute: ATTRIBUTE_FAN_MODE,
            data: writer.into_bytes(),
        }
    }

    pub fn set_fan_speed(&mut self, speed: FanSpeed) -> MatterRequest {
        self.state.fan_speed = speed;
        let mut writer = TlvWriter::new();
        writer.write_i32(speed as i32);
        MatterRequest {
            endpoint: 1,
            cluster: CLUSTER_FAN,
            attribute: ATTRIBUTE_FAN_MODE,
            data: writer.into_bytes(),
        }
    }

    pub fn set_vertical_swing(&mut self, on: bool) -> MatterRequest {
        self.state.vertical_swing = on;
        let mut writer = TlvWriter::new();
        writer.write_bool(on);
        MatterRequest {
            endpoint: 1,
            cluster: CLUSTER_ROCK,
            attribute: ATTRIBUTE_VERTICAL_SWING,
            data: writer.into_bytes(),
        }
    }

    pub fn set_horizontal_swing(&mut self, on: bool) -> MatterRequest {
        self.state.horizontal_swing = on;
        let mut writer = TlvWriter::new();
        writer.write_bool(on);
        MatterRequest {
            endpoint: 1,
            cluster: CLUSTER_ROCK,
            attribute: ATTRIBUTE_HORIZONTAL_SWING,
            data: writer.into_bytes(),
        }
    }

    pub fn swing_on(&mut self) -> MatterRequest {
        self.set_vertical_swing(true)
    }

    pub fn swing_off(&mut self) -> MatterRequest {
        self.set_vertical_swing(false)
    }

    pub fn build_read_request(&self, attributes: &[u32]) -> Vec<MatterRequest> {
        attributes
            .iter()
            .map(|&attr| MatterRequest {
                endpoint: 1,
                cluster: CLUSTER_THERMOSTAT,
                attribute: attr,
                data: vec![],
            })
            .collect()
    }
}

impl Default for AcMatterClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct MatterRequest {
    pub endpoint: u32,
    pub cluster: u32,
    pub attribute: u32,
    pub data: Vec<u8>,
}

pub const CLUSTER_THERMOSTAT: u32 = 0x0201;
pub const CLUSTER_FAN: u32 = 0x0202;
pub const CLUSTER_ROCK: u32 = 0x0402;

pub const ATTRIBUTE_ON_OFF: u32 = 0x0000;
pub const ATTRIBUTE_TARGET_TEMP: u32 = 0x0011;
pub const ATTRIBUTE_TARGET_TEMP_FRAC: u32 = 0x0012;
pub const ATTRIBUTE_FAN_MODE: u32 = 0x0000;
pub const ATTRIBUTE_VERTICAL_SWING: u32 = 0x0000;
pub const ATTRIBUTE_HORIZONTAL_SWING: u32 = 0x0001;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ac_client() {
        let mut client = AcMatterClient::new();
        let req = client.set_power(true);
        assert_eq!(req.endpoint, 1);
        assert_eq!(req.cluster, CLUSTER_THERMOSTAT);
    }

    #[test]
    fn test_set_temperature() {
        let mut client = AcMatterClient::new();
        let req = client.set_temperature(24.0);
        assert!(!req.data.is_empty());
    }
}
