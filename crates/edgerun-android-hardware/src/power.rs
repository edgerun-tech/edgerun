//! Android Power/Battery capability via sysfs.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityError, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

#[cfg(feature = "android-real")]
mod real {
    use super::*;

    fn read_sysfs_prop(path: &str) -> Option<String> {
        std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
    }

    #[derive(Clone, Debug, Default)]
    pub struct BatteryInfo {
        pub level_percent: i32,
        pub is_charging: bool,
        pub health: String,
        pub temperature_celsius: f32,
        pub voltage_mv: i32,
    }

    pub struct AndroidPowerProvider {
        last_info: Option<BatteryInfo>,
    }

    impl AndroidPowerProvider {
        pub fn new() -> Self {
            Self { last_info: None }
        }

        pub fn get_battery_info(&mut self) -> Result<BatteryInfo, CapabilityError> {
            let base = if std::path::Path::new("/sys/class/power_supply/battery").exists() {
                "/sys/class/power_supply/battery"
            } else if std::path::Path::new("/sys/class/power_supply/main").exists() {
                "/sys/class/power_supply/main"
            } else {
                return Err(CapabilityError::Provider(
                    "No power supply found in sysfs".into(),
                ));
            };

            let level_percent = read_sysfs_prop(&format!("{base}/capacity"))
                .and_then(|s| s.parse().ok())
                .unwrap_or(-1);
            let status = read_sysfs_prop(&format!("{base}/status")).unwrap_or_default();
            let is_charging = status == "Charging" || status == "Full";
            let health = read_sysfs_prop(&format!("{base}/health")).unwrap_or_default();
            let temperature_celsius = read_sysfs_prop(&format!("{base}/temp"))
                .and_then(|s| s.parse::<i32>().ok())
                .map(|t| t as f32 / 10.0)
                .unwrap_or(0.0);
            let voltage_mv = read_sysfs_prop(&format!("{base}/voltage_now"))
                .and_then(|s| s.parse().ok())
                .or_else(|| {
                    read_sysfs_prop(&format!("{base}/voltage_avg")).and_then(|s| s.parse().ok())
                })
                .unwrap_or(0) as i32;

            let info = BatteryInfo {
                level_percent,
                is_charging,
                health,
                temperature_celsius,
                voltage_mv,
            };
            self.last_info = Some(info.clone());
            Ok(info)
        }

        pub fn is_on_ac_power(&self) -> bool {
            read_sysfs_prop("/sys/class/power_supply/ac/online").as_deref() == Some("1")
                || read_sysfs_prop("/sys/class/power_supply/usb/online").as_deref() == Some("1")
        }
    }

    impl CapabilityProvider for AndroidPowerProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-power",
                "android",
                CapabilityRole::Input,
                &[CapabilityModality::Other],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query],
                Vec::new(),
            )
        }
    }
}

#[cfg(not(feature = "android-real"))]
mod real {
    use super::*;
    pub struct BatteryInfo;
    pub struct AndroidPowerProvider;
    impl Default for AndroidPowerProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AndroidPowerProvider {
        pub fn new() -> Self {
            Self
        }
        pub fn get_battery_info(&self) -> Result<BatteryInfo, CapabilityError> {
            Ok(BatteryInfo)
        }
        pub fn is_on_ac_power(&self) -> bool {
            false
        }
    }
    impl CapabilityProvider for AndroidPowerProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-power-stub",
                "stub",
                CapabilityRole::Input,
                &[CapabilityModality::Other],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query],
                Vec::new(),
            )
        }
    }
}

pub use real::{AndroidPowerProvider, BatteryInfo};
