#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PowerState {
    Off = 0,
    On = 1,
    Sleep = 2,
    Busy = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerSpec {
    pub voltage_min: u32,
    pub voltage_typ: u32,
    pub voltage_max: u32,
    pub current_peak: u32,
    pub current_avg: u32,
}

impl PowerSpec {
    pub const fn new(
        voltage_min: u32,
        voltage_typ: u32,
        voltage_max: u32,
        current_peak: u32,
        current_avg: u32,
    ) -> Self {
        Self {
            voltage_min,
            voltage_typ,
            voltage_max,
            current_peak,
            current_avg,
        }
    }
}

pub struct PowerSupply {
    pub name: &'static str,
    pub spec: PowerSpec,
}

pub const VBAT_BB: PowerSupply = PowerSupply {
    name: "VBAT_BB",
    spec: PowerSpec::new(3300, 3700, 4300, 800, 300),
};

pub const VBAT_RF: PowerSupply = PowerSupply {
    name: "VBAT_RF",
    spec: PowerSpec::new(3300, 3700, 4300, 1800, 500),
};

pub const VDD_EXT: PowerSupply = PowerSupply {
    name: "VDD_EXT",
    spec: PowerSpec::new(1710, 1800, 1890, 100, 50),
};

pub const POWER_SUPPLIES: &[&PowerSupply] = &[&VBAT_BB, &VBAT_RF, &VDD_EXT];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SleepMode {
    #[default]
    Disabled = 0,
    LightSleep = 1,
    DeepSleep = 2,
}

pub struct PowerConfig {
    pub enable_pwrkey: bool,
    pub enable_auto_pwr: bool,
    pub sleep_mode: SleepMode,
}

impl PowerConfig {
    pub const fn new() -> Self {
        Self {
            enable_pwrkey: true,
            enable_auto_pwr: false,
            sleep_mode: SleepMode::Disabled,
        }
    }

    pub const fn with_pwrkey(mut self, enable: bool) -> Self {
        self.enable_pwrkey = enable;
        self
    }

    pub const fn with_auto_power(mut self, enable: bool) -> Self {
        self.enable_auto_pwr = enable;
        self
    }

    pub const fn with_sleep_mode(mut self, mode: SleepMode) -> Self {
        self.sleep_mode = mode;
        self
    }
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self::new()
    }
}
use crate::prelude::v1::*;
