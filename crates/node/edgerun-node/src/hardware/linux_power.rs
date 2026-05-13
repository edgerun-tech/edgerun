use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_devices::power::{
    BatteryStatus, LidState, PowerInventory, PowerSourceInfo, PowerSupplyKind, PowerSystemInfo,
    default_power_descriptor,
};
use edgerun_linux_sysfs::prelude::v1::*;
use edgerun_linux_sysfs::{parse_bool_flag, parse_u8, parse_u64, read_trimmed};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxPowerSupply {
    pub instance_id: String,
    pub sysfs_path: PathBuf,
    pub kind: PowerSupplyKind,
    pub manufacturer: Option<String>,
    pub model_name: Option<String>,
    pub online: Option<bool>,
    pub present: Option<bool>,
    pub battery_status: Option<BatteryStatus>,
    pub capacity_percent: Option<u8>,
    pub voltage_now_uv: Option<u64>,
    pub current_now_ua: Option<u64>,
    pub power_now_uw: Option<u64>,
    pub energy_now_uwh: Option<u64>,
    pub energy_full_uwh: Option<u64>,
    pub energy_full_design_uwh: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxPowerBackend {
    pub power_supply_root: PathBuf,
    pub proc_acpi_root: PathBuf,
}

fn power_kind_from_type(value: Option<String>) -> PowerSupplyKind {
    match value.as_deref() {
        Some("Battery") => PowerSupplyKind::Battery,
        Some("Mains") => PowerSupplyKind::Mains,
        Some("USB") => PowerSupplyKind::Usb,
        Some("USB_C") | Some("USB-C") | Some("USB_PD") => PowerSupplyKind::UsbC,
        Some("Wireless") => PowerSupplyKind::Wireless,
        _ => PowerSupplyKind::Unknown,
    }
}

fn battery_status_from_str(value: Option<String>) -> Option<BatteryStatus> {
    value.as_deref().map(|status| match status {
        "Charging" => BatteryStatus::Charging,
        "Discharging" => BatteryStatus::Discharging,
        "Full" => BatteryStatus::Full,
        "Not charging" => BatteryStatus::NotCharging,
        _ => BatteryStatus::Unknown,
    })
}

fn lid_state_from_proc(root: &Path) -> LidState {
    let Ok(entries) = fs::read_dir(root) else {
        return LidState::Unknown;
    };
    for entry in entries.flatten() {
        let state_path = entry.path().join("state");
        let Some(contents) = fs::read_to_string(state_path).ok() else {
            continue;
        };
        for line in contents.lines() {
            let lower = line.to_ascii_lowercase();
            if lower.contains("open") {
                return LidState::Open;
            }
            if lower.contains("closed") {
                return LidState::Closed;
            }
        }
    }
    LidState::Unknown
}

pub fn discover_power_supplies() -> Result<Vec<LinuxPowerSupply>, CapabilityError> {
    discover_power_supplies_in(Path::new("/sys/class/power_supply"))
}

pub fn discover_power_supplies_in(root: &Path) -> Result<Vec<LinuxPowerSupply>, CapabilityError> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(err) => {
            return Err(CapabilityError::Provider(format!(
                "failed to read power supply sysfs: {err}"
            )));
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let instance_id = entry.file_name().to_string_lossy().to_string();
        let kind = power_kind_from_type(read_trimmed(&path.join("type")));
        out.push(LinuxPowerSupply {
            instance_id,
            sysfs_path: path.clone(),
            kind,
            manufacturer: read_trimmed(&path.join("manufacturer")),
            model_name: read_trimmed(&path.join("model_name")),
            online: parse_bool_flag(read_trimmed(&path.join("online"))),
            present: parse_bool_flag(read_trimmed(&path.join("present"))),
            battery_status: battery_status_from_str(read_trimmed(&path.join("status"))),
            capacity_percent: parse_u8(read_trimmed(&path.join("capacity"))),
            voltage_now_uv: parse_u64(read_trimmed(&path.join("voltage_now"))),
            current_now_ua: parse_u64(read_trimmed(&path.join("current_now"))),
            power_now_uw: parse_u64(read_trimmed(&path.join("power_now"))),
            energy_now_uwh: parse_u64(read_trimmed(&path.join("energy_now")))
                .or_else(|| parse_u64(read_trimmed(&path.join("charge_now")))),
            energy_full_uwh: parse_u64(read_trimmed(&path.join("energy_full")))
                .or_else(|| parse_u64(read_trimmed(&path.join("charge_full")))),
            energy_full_design_uwh: parse_u64(read_trimmed(&path.join("energy_full_design")))
                .or_else(|| parse_u64(read_trimmed(&path.join("charge_full_design")))),
        });
    }
    out.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
    Ok(out)
}

pub fn discover_power_system() -> Result<PowerSystemInfo, CapabilityError> {
    discover_power_system_in(
        Path::new("/sys/class/power_supply"),
        Path::new("/proc/acpi/button/lid"),
    )
}

pub fn discover_power_system_in(
    power_supply_root: &Path,
    proc_acpi_lid_root: &Path,
) -> Result<PowerSystemInfo, CapabilityError> {
    let supplies = discover_power_supplies_in(power_supply_root)?;
    let sources: Vec<PowerSourceInfo> = supplies
        .iter()
        .map(|supply| PowerSourceInfo {
            provider: "linux-power".into(),
            instance_id: supply.instance_id.clone(),
            kind: supply.kind,
            manufacturer: supply.manufacturer.clone(),
            model_name: supply.model_name.clone(),
            online: supply.online,
            present: supply.present,
            battery_status: supply.battery_status,
            capacity_percent: supply.capacity_percent,
            voltage_now_uv: supply.voltage_now_uv,
            current_now_ua: supply.current_now_ua,
            power_now_uw: supply.power_now_uw,
            energy_now_uwh: supply.energy_now_uwh,
            energy_full_uwh: supply.energy_full_uwh,
            energy_full_design_uwh: supply.energy_full_design_uwh,
        })
        .collect();

    let on_ac_power = supplies
        .iter()
        .filter(|s| {
            matches!(
                s.kind,
                PowerSupplyKind::Mains | PowerSupplyKind::Usb | PowerSupplyKind::UsbC
            )
        })
        .find_map(|s| s.online);

    let battery_percent = supplies
        .iter()
        .filter(|s| s.kind == PowerSupplyKind::Battery)
        .find_map(|s| s.capacity_percent);

    Ok(PowerSystemInfo {
        sources,
        lid_state: lid_state_from_proc(proc_acpi_lid_root),
        on_ac_power,
        battery_percent,
    })
}

impl CapabilityProvider for LinuxPowerBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_power_descriptor("linux-power", "system")
    }
}

impl PowerInventory for LinuxPowerBackend {
    fn power_info(&self) -> Result<PowerSystemInfo, CapabilityError> {
        discover_power_system_in(&self.power_supply_root, &self.proc_acpi_root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_linux_sysfs::temp_root;

    #[test]
    fn discover_battery_and_ac_from_sysfs() {
        let root = temp_root("edgerun-power-sysfs");
        let bat = root.join("BAT0");
        fs::create_dir_all(&bat).unwrap();
        fs::write(bat.join("type"), "Battery\n").unwrap();
        fs::write(bat.join("status"), "Discharging\n").unwrap();
        fs::write(bat.join("capacity"), "73\n").unwrap();
        fs::write(bat.join("energy_now"), "42000000\n").unwrap();
        fs::write(bat.join("energy_full"), "58000000\n").unwrap();

        let ac = root.join("AC");
        fs::create_dir_all(&ac).unwrap();
        fs::write(ac.join("type"), "Mains\n").unwrap();
        fs::write(ac.join("online"), "1\n").unwrap();

        let lid_root = temp_root("edgerun-power-lid");
        let lid = lid_root.join("LID0");
        fs::create_dir_all(&lid).unwrap();
        fs::write(lid.join("state"), "state:      open\n").unwrap();

        let system = discover_power_system_in(&root, &lid_root).unwrap();
        assert_eq!(system.sources.len(), 2);
        assert_eq!(system.battery_percent, Some(73));
        assert_eq!(system.on_ac_power, Some(true));
        assert_eq!(system.lid_state, LidState::Open);

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(lid_root).unwrap();
    }

    #[test]
    fn discover_power_supplies_returns_empty_for_missing_dir() {
        let result = discover_power_supplies_in(Path::new("/nonexistent/power_supply"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn discover_power_supplies_empty_directory() {
        let root = temp_root("edgerun-power-empty");
        let supplies = discover_power_supplies_in(&root).unwrap();
        assert!(supplies.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn power_kind_from_type_all_variants() {
        assert_eq!(
            power_kind_from_type(Some("Battery".into())),
            PowerSupplyKind::Battery
        );
        assert_eq!(
            power_kind_from_type(Some("Mains".into())),
            PowerSupplyKind::Mains
        );
        assert_eq!(
            power_kind_from_type(Some("USB".into())),
            PowerSupplyKind::Usb
        );
        assert_eq!(
            power_kind_from_type(Some("USB_C".into())),
            PowerSupplyKind::UsbC
        );
        assert_eq!(
            power_kind_from_type(Some("USB-C".into())),
            PowerSupplyKind::UsbC
        );
        assert_eq!(
            power_kind_from_type(Some("USB_PD".into())),
            PowerSupplyKind::UsbC
        );
        assert_eq!(
            power_kind_from_type(Some("Wireless".into())),
            PowerSupplyKind::Wireless
        );
    }

    #[test]
    fn power_kind_from_type_unknown() {
        assert_eq!(
            power_kind_from_type(Some("Unknown".into())),
            PowerSupplyKind::Unknown
        );
        assert_eq!(
            power_kind_from_type(Some("".into())),
            PowerSupplyKind::Unknown
        );
        assert_eq!(power_kind_from_type(None), PowerSupplyKind::Unknown);
    }

    #[test]
    fn battery_status_from_str_all_variants() {
        assert_eq!(
            battery_status_from_str(Some("Charging".into())),
            Some(BatteryStatus::Charging)
        );
        assert_eq!(
            battery_status_from_str(Some("Discharging".into())),
            Some(BatteryStatus::Discharging)
        );
        assert_eq!(
            battery_status_from_str(Some("Full".into())),
            Some(BatteryStatus::Full)
        );
        assert_eq!(
            battery_status_from_str(Some("Not charging".into())),
            Some(BatteryStatus::NotCharging)
        );
    }

    #[test]
    fn battery_status_from_str_unknown() {
        assert_eq!(
            battery_status_from_str(Some("Unknown".into())),
            Some(BatteryStatus::Unknown)
        );
        assert_eq!(
            battery_status_from_str(Some("".into())),
            Some(BatteryStatus::Unknown)
        );
        assert_eq!(battery_status_from_str(None), None);
    }

    #[test]
    fn lid_state_from_proc_open() {
        let root = temp_root("edgerun-lid-open");
        let lid = root.join("LID0");
        fs::create_dir_all(&lid).unwrap();
        fs::write(lid.join("state"), "state:      open\n").unwrap();

        assert_eq!(lid_state_from_proc(&root), LidState::Open);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn lid_state_from_proc_closed() {
        let root = temp_root("edgerun-lid-closed");
        let lid = root.join("LID0");
        fs::create_dir_all(&lid).unwrap();
        fs::write(lid.join("state"), "state:      closed\n").unwrap();

        assert_eq!(lid_state_from_proc(&root), LidState::Closed);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn lid_state_from_proc_missing_directory() {
        let result = lid_state_from_proc(Path::new("/nonexistent/lid/path"));
        assert_eq!(result, LidState::Unknown);
    }

    #[test]
    fn lid_state_from_proc_no_state_file() {
        let root = temp_root("edgerun-lid-nostate");
        let lid = root.join("LID0");
        fs::create_dir_all(&lid).unwrap();

        assert_eq!(lid_state_from_proc(&root), LidState::Unknown);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn linux_power_supply_clone() {
        let supply = LinuxPowerSupply {
            instance_id: "BAT0".into(),
            sysfs_path: PathBuf::from("/sys/class/power_supply/BAT0"),
            kind: PowerSupplyKind::Battery,
            manufacturer: Some("TestMfg".into()),
            model_name: Some("TestModel".into()),
            online: None,
            present: Some(true),
            battery_status: Some(BatteryStatus::Charging),
            capacity_percent: Some(80),
            voltage_now_uv: Some(12_000_000),
            current_now_ua: Some(1_500_000),
            power_now_uw: None,
            energy_now_uwh: Some(42_000_000),
            energy_full_uwh: Some(58_000_000),
            energy_full_design_uwh: Some(60_000_000),
        };
        assert_eq!(supply.clone(), supply);
    }

    #[test]
    fn linux_power_backend_descriptor() {
        let backend = LinuxPowerBackend {
            power_supply_root: PathBuf::from("/sys/class/power_supply"),
            proc_acpi_root: PathBuf::from("/proc/acpi/button/lid"),
        };
        let descriptor = backend.descriptor();
        assert_eq!(descriptor.provider_name, "linux-power");
        assert_eq!(descriptor.provider_instance_id, "system");
    }

    #[test]
    fn linux_power_backend_power_info() {
        let root = temp_root("edgerun-power-backend-info");
        let lid_root = temp_root("edgerun-power-backend-lid");

        let backend = LinuxPowerBackend {
            power_supply_root: root.clone(),
            proc_acpi_root: lid_root.clone(),
        };
        // Should work with empty directories
        let info = backend.power_info().unwrap();
        assert!(info.sources.is_empty());
        assert_eq!(info.lid_state, LidState::Unknown);
        assert_eq!(info.on_ac_power, None);
        assert_eq!(info.battery_percent, None);

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&lid_root);
    }

    #[test]
    fn discover_power_system_on_ac_power_from_usb() {
        let root = temp_root("edgerun-power-usb-ac");
        let usb = root.join("USB0");
        fs::create_dir_all(&usb).unwrap();
        fs::write(usb.join("type"), "USB\n").unwrap();
        fs::write(usb.join("online"), "1\n").unwrap();

        let lid_root = temp_root("edgerun-power-usb-lid");
        let system = discover_power_system_in(&root, &lid_root).unwrap();
        assert_eq!(system.on_ac_power, Some(true));

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&lid_root);
    }

    #[test]
    fn discover_power_system_on_ac_power_from_usbc() {
        let root = temp_root("edgerun-power-usbc-ac");
        let usbc = root.join("UCSI");
        fs::create_dir_all(&usbc).unwrap();
        fs::write(usbc.join("type"), "USB_C\n").unwrap();
        fs::write(usbc.join("online"), "1\n").unwrap();

        let lid_root = temp_root("edgerun-power-usbc-lid");
        let system = discover_power_system_in(&root, &lid_root).unwrap();
        assert_eq!(system.on_ac_power, Some(true));

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&lid_root);
    }

    #[test]
    fn discover_power_system_ac_not_online() {
        let root = temp_root("edgerun-power-ac-offline");
        let ac = root.join("AC0");
        fs::create_dir_all(&ac).unwrap();
        fs::write(ac.join("type"), "Mains\n").unwrap();
        fs::write(ac.join("online"), "0\n").unwrap();

        let lid_root = temp_root("edgerun-power-ac-offline-lid");
        let system = discover_power_system_in(&root, &lid_root).unwrap();
        assert_eq!(system.on_ac_power, Some(false));

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&lid_root);
    }

    #[test]
    fn discover_power_system_battery_percent_from_energy_fields() {
        let root = temp_root("edgerun-power-bat-energy");
        let bat = root.join("BAT1");
        fs::create_dir_all(&bat).unwrap();
        fs::write(bat.join("type"), "Battery\n").unwrap();
        fs::write(bat.join("capacity"), "55\n").unwrap();

        let lid_root = temp_root("edgerun-power-bat-energy-lid");
        let system = discover_power_system_in(&root, &lid_root).unwrap();
        assert_eq!(system.battery_percent, Some(55));

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&lid_root);
    }

    #[test]
    fn linux_power_supply_debug() {
        let supply = LinuxPowerSupply {
            instance_id: "AC0".into(),
            sysfs_path: PathBuf::from("/sys/class/power_supply/AC0"),
            kind: PowerSupplyKind::Mains,
            manufacturer: None,
            model_name: None,
            online: Some(true),
            present: None,
            battery_status: None,
            capacity_percent: None,
            voltage_now_uv: None,
            current_now_ua: None,
            power_now_uw: None,
            energy_now_uwh: None,
            energy_full_uwh: None,
            energy_full_design_uwh: None,
        };
        let debug = format!("{:?}", supply);
        assert!(debug.contains("AC0"));
        assert!(debug.contains("Mains"));
    }

    #[test]
    fn linux_power_backend_clone() {
        let backend = LinuxPowerBackend {
            power_supply_root: PathBuf::from("/sys/class/power_supply"),
            proc_acpi_root: PathBuf::from("/proc/acpi/button/lid"),
        };
        assert_eq!(backend.clone(), backend);
    }

    #[test]
    fn discover_power_supplies_sorted_by_instance_id() {
        let root = temp_root("edgerun-power-sorted");
        let bat0 = root.join("BAT1");
        fs::create_dir_all(&bat0).unwrap();
        fs::write(bat0.join("type"), "Battery\n").unwrap();
        let ac = root.join("AC");
        fs::create_dir_all(&ac).unwrap();
        fs::write(ac.join("type"), "Mains\n").unwrap();
        let bat1 = root.join("BAT0");
        fs::create_dir_all(&bat1).unwrap();
        fs::write(bat1.join("type"), "Battery\n").unwrap();

        let supplies = discover_power_supplies_in(&root).unwrap();
        assert_eq!(supplies.len(), 3);
        assert_eq!(supplies[0].instance_id, "AC");
        assert_eq!(supplies[1].instance_id, "BAT0");
        assert_eq!(supplies[2].instance_id, "BAT1");

        let _ = fs::remove_dir_all(&root);
    }
}
