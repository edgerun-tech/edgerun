use lifegraph_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use lifegraph_linux_sysfs::{parse_bool_flag, parse_u64, parse_u8, read_trimmed};
use lifegraph_power::{
    default_power_descriptor, BatteryStatus, LidState, PowerInventory, PowerSourceInfo,
    PowerSupplyKind, PowerSystemInfo,
};
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
            )))
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
    use lifegraph_linux_sysfs::temp_root;

    #[test]
    fn discover_battery_and_ac_from_sysfs() {
        let root = temp_root("lifegraph-power-sysfs");
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

        let lid_root = temp_root("lifegraph-power-lid");
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
}
