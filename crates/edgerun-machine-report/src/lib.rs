use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use edgerun_json::JsonValue;
use edgerun_linux_netif::{discover_network_interfaces, LinuxNetifBackend};
use edgerun_linux_pci::LinuxPciBackend;
use edgerun_linux_usb::LinuxUsbBackend;
use edgerun_linux_wifi::{discover_wifi_interfaces, LinuxWifiBackend};
use edgerun_network_interface::{NetworkInterfaceController, NetworkInterfaceInfo};
use edgerun_pci::{PciDeviceInfo, PciInventory};
use edgerun_usb::{UsbDeviceInfo, UsbInventory};
use edgerun_wifi::{
    WifiAccessPointController, WifiAccessPointState, WifiController, WifiInterfaceInfo,
    WifiNetworkObservation, WifiScanner,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportSection<T> {
    pub items: T,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiInterfaceReport {
    pub info: Option<WifiInterfaceInfo>,
    pub info_error: Option<String>,
    pub observations: Vec<WifiNetworkObservation>,
    pub observation_error: Option<String>,
    pub access_point_state: Option<WifiAccessPointState>,
    pub access_point_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineReport {
    pub generated_at_unix_ms: i64,
    pub pci_devices: ReportSection<Vec<PciDeviceInfo>>,
    pub usb_devices: ReportSection<Vec<UsbDeviceInfo>>,
    pub network_interfaces: ReportSection<Vec<NetworkInterfaceInfo>>,
    pub wifi_interfaces: ReportSection<Vec<WifiInterfaceReport>>,
}

pub fn gather_machine_report() -> MachineReport {
    let generated_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0);

    let pci_devices = {
        let backend = LinuxPciBackend {
            root_path: PathBuf::from("/sys/bus/pci/devices"),
        };
        match backend.list_devices() {
            Ok(items) => ReportSection { items, error: None },
            Err(error) => ReportSection {
                items: Vec::new(),
                error: Some(error.to_string()),
            },
        }
    };

    let usb_devices = {
        let backend = LinuxUsbBackend {
            root_path: PathBuf::from("/sys/bus/usb/devices"),
        };
        match backend.list_devices() {
            Ok(items) => ReportSection { items, error: None },
            Err(error) => ReportSection {
                items: Vec::new(),
                error: Some(error.to_string()),
            },
        }
    };

    let network_interfaces = match discover_network_interfaces() {
        Ok(interfaces) => {
            let mut items = Vec::with_capacity(interfaces.len());
            for interface in interfaces {
                let backend = LinuxNetifBackend { interface };
                match backend.interface_info() {
                    Ok(info) => items.push(info),
                    Err(error) => {
                        return MachineReport {
                            generated_at_unix_ms,
                            pci_devices,
                            usb_devices,
                            network_interfaces: ReportSection {
                                items,
                                error: Some(error.to_string()),
                            },
                            wifi_interfaces: gather_wifi_section(),
                        };
                    }
                }
            }
            ReportSection { items, error: None }
        }
        Err(error) => ReportSection {
            items: Vec::new(),
            error: Some(error.to_string()),
        },
    };

    let wifi_interfaces = gather_wifi_section();

    MachineReport {
        generated_at_unix_ms,
        pci_devices,
        usb_devices,
        network_interfaces,
        wifi_interfaces,
    }
}

fn gather_wifi_section() -> ReportSection<Vec<WifiInterfaceReport>> {
    match discover_wifi_interfaces() {
        Ok(interfaces) => {
            let mut items = Vec::with_capacity(interfaces.len());
            for interface in interfaces {
                let backend = LinuxWifiBackend { interface };
                let info = backend.interface_info().map_err(|error| error.to_string());
                let observations = backend.scan_nearby().map_err(|error| error.to_string());
                let access_point_state = backend
                    .access_point_state()
                    .map_err(|error| error.to_string());

                items.push(WifiInterfaceReport {
                    info: info.clone().ok(),
                    info_error: info.err(),
                    observations: observations
                        .as_ref()
                        .map(|scan| scan.observations.clone())
                        .unwrap_or_default(),
                    observation_error: observations.err(),
                    access_point_state: access_point_state.clone().ok(),
                    access_point_error: access_point_state.err(),
                });
            }
            ReportSection { items, error: None }
        }
        Err(error) => ReportSection {
            items: Vec::new(),
            error: Some(error.to_string()),
        },
    }
}

pub fn render_machine_report(
    report: &MachineReport,
    format: OutputFormat,
) -> Result<String, String> {
    match format {
        OutputFormat::Text => Ok(render_text(report)),
        OutputFormat::Json => report
            .to_json()
            .to_json_string()
            .map_err(|error| error.to_string()),
    }
}

fn render_text(report: &MachineReport) -> String {
    let mut out = String::new();
    writeln!(&mut out, "edgerun machine report").unwrap();
    writeln!(
        &mut out,
        "generated_at_unix_ms: {}",
        report.generated_at_unix_ms
    )
    .unwrap();
    writeln!(&mut out).unwrap();

    render_section_header(
        &mut out,
        "Network interfaces",
        report.network_interfaces.items.len(),
        report.network_interfaces.error.as_deref(),
    );
    for interface in &report.network_interfaces.items {
        writeln!(
            &mut out,
            "- {} kind={:?} admin={:?} link={:?} mac={} mtu={}",
            interface.interface_name,
            interface.kind,
            interface.admin_state,
            interface.link_state,
            interface.mac_address.as_deref().unwrap_or(""),
            interface
                .mtu
                .map(|value| value.to_string())
                .unwrap_or_default()
        )
        .unwrap();
    }
    writeln!(&mut out).unwrap();

    render_section_header(
        &mut out,
        "Wi-Fi interfaces",
        report.wifi_interfaces.items.len(),
        report.wifi_interfaces.error.as_deref(),
    );
    for interface in &report.wifi_interfaces.items {
        if let Some(info) = &interface.info {
            writeln!(
                &mut out,
                "- {} mode={:?} power={:?} operstate={} mac={} phy={}",
                info.interface_name,
                info.mode,
                info.power_state,
                info.operstate.as_deref().unwrap_or(""),
                info.mac_address.as_deref().unwrap_or(""),
                info.phy_name.as_deref().unwrap_or(""),
            )
            .unwrap();
        } else {
            writeln!(&mut out, "- <unknown wifi interface>").unwrap();
        }
        if let Some(error) = &interface.info_error {
            writeln!(&mut out, "  info_error: {error}").unwrap();
        }
        if let Some(ap_state) = &interface.access_point_state {
            writeln!(
                &mut out,
                "  access_point: active={} ssid={} freq_mhz={} hidden={} secure={}",
                ap_state.active,
                ap_state.ssid.as_deref().unwrap_or(""),
                ap_state
                    .frequency_mhz
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                ap_state.hidden,
                ap_state
                    .secure
                    .map(|value| if value { "true" } else { "false" })
                    .unwrap_or(""),
            )
            .unwrap();
        }
        if let Some(error) = &interface.access_point_error {
            writeln!(&mut out, "  access_point_error: {error}").unwrap();
        }
        if interface.observations.is_empty() {
            writeln!(&mut out, "  current_network: none").unwrap();
        } else {
            for observation in &interface.observations {
                writeln!(
                    &mut out,
                    "  current_network: ssid={} bssid={} signal_dbm={} freq_mhz={} secure={} observed_at_unix_ms={}",
                    observation.ssid.as_deref().unwrap_or(""),
                    observation.bssid.as_deref().unwrap_or(""),
                    observation
                        .signal_dbm
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                    observation
                        .frequency_mhz
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                    observation
                        .secure
                        .map(|value| if value { "true" } else { "false" })
                        .unwrap_or(""),
                    observation.observed_at_unix_ms,
                )
                .unwrap();
            }
        }
        if let Some(error) = &interface.observation_error {
            writeln!(&mut out, "  current_network_error: {error}").unwrap();
        }
    }
    writeln!(&mut out).unwrap();

    render_section_header(
        &mut out,
        "USB devices",
        report.usb_devices.items.len(),
        report.usb_devices.error.as_deref(),
    );
    for device in &report.usb_devices.items {
        writeln!(
            &mut out,
            "- {} bus={} dev={} vid={} pid={} speed={:?} mfg={} product={} driver={}",
            device.instance_id,
            device
                .bus_num
                .map(|value| value.to_string())
                .unwrap_or_default(),
            device
                .dev_num
                .map(|value| value.to_string())
                .unwrap_or_default(),
            format_hex_u16(device.vendor_id),
            format_hex_u16(device.product_id),
            device.speed,
            device.manufacturer.as_deref().unwrap_or(""),
            device.product_name.as_deref().unwrap_or(""),
            device.driver.as_deref().unwrap_or(""),
        )
        .unwrap();
        for interface in &device.interfaces {
            writeln!(
                &mut out,
                "  iface={} num={} alt={} class={} subclass={} proto={} name={} driver={}",
                interface.instance_id,
                interface
                    .interface_number
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                interface
                    .alternate_setting
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                format_hex_u8(interface.interface_class),
                format_hex_u8(interface.interface_subclass),
                format_hex_u8(interface.interface_protocol),
                interface.interface_name.as_deref().unwrap_or(""),
                interface.driver.as_deref().unwrap_or(""),
            )
            .unwrap();
        }
    }
    writeln!(&mut out).unwrap();

    render_section_header(
        &mut out,
        "PCI devices",
        report.pci_devices.items.len(),
        report.pci_devices.error.as_deref(),
    );
    for device in &report.pci_devices.items {
        writeln!(
            &mut out,
            "- {} vid={} did={} class={} rev={} driver={} parent={} children={}",
            device.address,
            format_hex_u16(device.vendor_id),
            format_hex_u16(device.device_id),
            format_hex_u32(device.class_code),
            format_hex_u8(device.revision),
            device.driver.as_deref().unwrap_or(""),
            device.parent_address.as_deref().unwrap_or(""),
            device.child_addresses.join(","),
        )
        .unwrap();
    }

    out
}

fn render_section_header(out: &mut String, name: &str, count: usize, error: Option<&str>) {
    match error {
        Some(error) => {
            writeln!(out, "{name}: {count} items (error: {error})").unwrap();
        }
        None => {
            writeln!(out, "{name}: {count} items").unwrap();
        }
    }
}

fn format_hex_u8(value: Option<u8>) -> String {
    edgerun_encoding::kv::format_hex_u8(value)
}

fn format_hex_u16(value: Option<u16>) -> String {
    edgerun_encoding::kv::format_hex_u16(value)
}

fn format_hex_u32(value: Option<u32>) -> String {
    edgerun_encoding::kv::format_hex_u32(value)
}

impl MachineReport {
    pub fn to_json(&self) -> JsonValue {
        JsonValue::object(vec![
            ("generated_at_unix_ms", self.generated_at_unix_ms.into()),
            (
                "network_interfaces",
                section_to_json(
                    &self.network_interfaces.items,
                    self.network_interfaces.error.as_deref(),
                    network_interface_to_json,
                ),
            ),
            (
                "wifi_interfaces",
                section_to_json(
                    &self.wifi_interfaces.items,
                    self.wifi_interfaces.error.as_deref(),
                    wifi_interface_to_json,
                ),
            ),
            (
                "usb_devices",
                section_to_json(
                    &self.usb_devices.items,
                    self.usb_devices.error.as_deref(),
                    usb_device_to_json,
                ),
            ),
            (
                "pci_devices",
                section_to_json(
                    &self.pci_devices.items,
                    self.pci_devices.error.as_deref(),
                    pci_device_to_json,
                ),
            ),
        ])
    }
}

fn section_to_json<T>(
    items: &[T],
    error: Option<&str>,
    mut item_to_json: impl FnMut(&T) -> JsonValue,
) -> JsonValue {
    JsonValue::object(vec![
        ("count", items.len().into()),
        ("error", error.map(str::to_owned).into()),
        (
            "items",
            JsonValue::array(items.iter().map(&mut item_to_json).collect()),
        ),
    ])
}

fn network_interface_to_json(info: &NetworkInterfaceInfo) -> JsonValue {
    JsonValue::object(vec![
        ("provider", info.provider.clone().into()),
        ("interface_name", info.interface_name.clone().into()),
        ("kind", format!("{:?}", info.kind).into()),
        ("mac_address", info.mac_address.clone().into()),
        ("mtu", info.mtu.into()),
        ("admin_state", format!("{:?}", info.admin_state).into()),
        ("link_state", format!("{:?}", info.link_state).into()),
    ])
}

fn wifi_interface_to_json(report: &WifiInterfaceReport) -> JsonValue {
    JsonValue::object(vec![
        (
            "info",
            report
                .info
                .as_ref()
                .map(wifi_info_to_json)
                .unwrap_or(JsonValue::Null),
        ),
        ("info_error", report.info_error.clone().into()),
        (
            "observations",
            JsonValue::array(
                report
                    .observations
                    .iter()
                    .map(wifi_observation_to_json)
                    .collect(),
            ),
        ),
        ("observation_error", report.observation_error.clone().into()),
        (
            "access_point_state",
            report
                .access_point_state
                .as_ref()
                .map(wifi_access_point_to_json)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "access_point_error",
            report.access_point_error.clone().into(),
        ),
    ])
}

fn wifi_info_to_json(info: &WifiInterfaceInfo) -> JsonValue {
    JsonValue::object(vec![
        ("provider", info.provider.clone().into()),
        ("interface_name", info.interface_name.clone().into()),
        ("mac_address", info.mac_address.clone().into()),
        ("phy_name", info.phy_name.clone().into()),
        ("operstate", info.operstate.clone().into()),
        ("power_state", format!("{:?}", info.power_state).into()),
        ("mode", format!("{:?}", info.mode).into()),
    ])
}

fn wifi_observation_to_json(observation: &WifiNetworkObservation) -> JsonValue {
    JsonValue::object(vec![
        ("interface_name", observation.interface_name.clone().into()),
        ("ssid", observation.ssid.clone().into()),
        ("bssid", observation.bssid.clone().into()),
        ("signal_dbm", observation.signal_dbm.into()),
        ("frequency_mhz", observation.frequency_mhz.into()),
        ("secure", observation.secure.into()),
        (
            "observed_at_unix_ms",
            observation.observed_at_unix_ms.into(),
        ),
    ])
}

fn wifi_access_point_to_json(state: &WifiAccessPointState) -> JsonValue {
    JsonValue::object(vec![
        ("interface_name", state.interface_name.clone().into()),
        ("active", state.active.into()),
        ("ssid", state.ssid.clone().into()),
        ("frequency_mhz", state.frequency_mhz.into()),
        ("hidden", state.hidden.into()),
        ("secure", state.secure.into()),
    ])
}

fn usb_device_to_json(device: &UsbDeviceInfo) -> JsonValue {
    JsonValue::object(vec![
        ("provider", device.provider.clone().into()),
        ("instance_id", device.instance_id.clone().into()),
        ("bus_num", device.bus_num.into()),
        ("dev_num", device.dev_num.into()),
        ("vendor_id", device.vendor_id.into()),
        ("product_id", device.product_id.into()),
        ("manufacturer", device.manufacturer.clone().into()),
        ("product_name", device.product_name.clone().into()),
        ("serial_number", device.serial_number.clone().into()),
        ("usb_version", device.usb_version.clone().into()),
        ("configuration_value", device.configuration_value.into()),
        (
            "configuration_name",
            device.configuration_name.clone().into(),
        ),
        ("port_path", device.port_path.clone().into()),
        ("max_children", device.max_children.into()),
        ("speed", format!("{:?}", device.speed).into()),
        ("driver", device.driver.clone().into()),
        (
            "parent_instance_id",
            device.parent_instance_id.clone().into(),
        ),
        (
            "child_instance_ids",
            JsonValue::array(
                device
                    .child_instance_ids
                    .iter()
                    .cloned()
                    .map(JsonValue::from)
                    .collect(),
            ),
        ),
        (
            "interfaces",
            JsonValue::array(
                device
                    .interfaces
                    .iter()
                    .map(usb_interface_to_json)
                    .collect(),
            ),
        ),
    ])
}

fn usb_interface_to_json(interface: &edgerun_usb::UsbInterfaceInfo) -> JsonValue {
    JsonValue::object(vec![
        ("instance_id", interface.instance_id.clone().into()),
        ("interface_number", interface.interface_number.into()),
        ("alternate_setting", interface.alternate_setting.into()),
        ("interface_class", interface.interface_class.into()),
        ("interface_subclass", interface.interface_subclass.into()),
        ("interface_protocol", interface.interface_protocol.into()),
        ("interface_name", interface.interface_name.clone().into()),
        ("driver", interface.driver.clone().into()),
    ])
}

fn pci_device_to_json(device: &PciDeviceInfo) -> JsonValue {
    JsonValue::object(vec![
        ("provider", device.provider.clone().into()),
        ("address", device.address.clone().into()),
        ("vendor_id", device.vendor_id.into()),
        ("device_id", device.device_id.into()),
        ("subsystem_vendor_id", device.subsystem_vendor_id.into()),
        ("subsystem_device_id", device.subsystem_device_id.into()),
        ("class_code", device.class_code.into()),
        ("revision", device.revision.into()),
        ("driver", device.driver.clone().into()),
        ("numa_node", device.numa_node.into()),
        (
            "current_link_speed",
            device.current_link_speed.clone().into(),
        ),
        ("current_link_width", device.current_link_width.into()),
        ("max_link_speed", device.max_link_speed.clone().into()),
        ("max_link_width", device.max_link_width.into()),
        ("parent_address", device.parent_address.clone().into()),
        (
            "child_addresses",
            JsonValue::array(
                device
                    .child_addresses
                    .iter()
                    .cloned()
                    .map(JsonValue::from)
                    .collect(),
            ),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_variants_are_copy() {
        let fmt = OutputFormat::Text;
        let _copied = fmt;
        assert_eq!(fmt, OutputFormat::Text);
    }

    #[test]
    fn output_format_equality() {
        assert_eq!(OutputFormat::Text, OutputFormat::Text);
        assert_eq!(OutputFormat::Json, OutputFormat::Json);
        assert_ne!(OutputFormat::Text, OutputFormat::Json);
    }

    #[test]
    fn output_format_debug() {
        assert_eq!(format!("{:?}", OutputFormat::Text), "Text");
        assert_eq!(format!("{:?}", OutputFormat::Json), "Json");
    }

    #[test]
    fn report_section_construction() {
        let section: ReportSection<Vec<i32>> = ReportSection {
            items: vec![1, 2, 3],
            error: None,
        };
        assert_eq!(section.items, vec![1, 2, 3]);
        assert_eq!(section.error, None);
    }

    #[test]
    fn report_section_with_error() {
        let section: ReportSection<Vec<String>> = ReportSection {
            items: vec!["a".into()],
            error: Some("test error".into()),
        };
        assert_eq!(section.error.as_deref(), Some("test error"));
    }

    #[test]
    fn report_section_clone() {
        let section: ReportSection<Vec<i32>> = ReportSection {
            items: vec![1],
            error: Some("err".into()),
        };
        assert_eq!(section.clone(), section);
    }

    #[test]
    fn wifi_interface_report_construction() {
        let report = WifiInterfaceReport {
            info: None,
            info_error: Some("failed to get info".into()),
            observations: Vec::new(),
            observation_error: None,
            access_point_state: None,
            access_point_error: None,
        };
        assert_eq!(report.info, None);
        assert_eq!(report.info_error.as_deref(), Some("failed to get info"));
        assert!(report.observations.is_empty());
    }

    #[test]
    fn wifi_interface_report_clone() {
        let report = WifiInterfaceReport {
            info: None,
            info_error: None,
            observations: Vec::new(),
            observation_error: None,
            access_point_state: None,
            access_point_error: None,
        };
        assert_eq!(report.clone(), report);
    }

    #[test]
    fn machine_report_construction() {
        let report = MachineReport {
            generated_at_unix_ms: 1_700_000_000_000,
            pci_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            usb_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            network_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
            wifi_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
        };
        assert_eq!(report.generated_at_unix_ms, 1_700_000_000_000);
    }

    #[test]
    fn machine_report_clone() {
        let report = MachineReport {
            generated_at_unix_ms: 42,
            pci_devices: ReportSection {
                items: Vec::new(),
                error: Some("pci err".into()),
            },
            usb_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            network_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
            wifi_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
        };
        assert_eq!(report.clone(), report);
    }

    #[test]
    fn text_report_renders_section_names() {
        let report = MachineReport {
            generated_at_unix_ms: 1,
            pci_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            usb_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            network_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
            wifi_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
        };
        let text = render_machine_report(&report, OutputFormat::Text).unwrap();
        assert!(text.contains("edgerun machine report"));
        assert!(text.contains("Network interfaces: 0 items"));
        assert!(text.contains("PCI devices: 0 items"));
    }

    #[test]
    fn text_report_renders_error_in_section() {
        let report = MachineReport {
            generated_at_unix_ms: 1,
            pci_devices: ReportSection {
                items: Vec::new(),
                error: Some("pci failed".into()),
            },
            usb_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            network_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
            wifi_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
        };
        let text = render_machine_report(&report, OutputFormat::Text).unwrap();
        assert!(text.contains("PCI devices: 0 items (error: pci failed)"));
    }

    #[test]
    fn text_report_renders_timestamp() {
        let report = MachineReport {
            generated_at_unix_ms: 1234567890,
            pci_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            usb_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            network_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
            wifi_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
        };
        let text = render_machine_report(&report, OutputFormat::Text).unwrap();
        assert!(text.contains("generated_at_unix_ms: 1234567890"));
    }

    #[test]
    fn json_report_contains_top_level_sections() {
        let report = MachineReport {
            generated_at_unix_ms: 7,
            pci_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            usb_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            network_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
            wifi_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
        };
        let json = render_machine_report(&report, OutputFormat::Json).unwrap();
        assert!(json.contains("\"generated_at_unix_ms\":7"));
        assert!(json.contains("\"network_interfaces\""));
        assert!(json.contains("\"wifi_interfaces\""));
        assert!(json.contains("\"usb_devices\""));
        assert!(json.contains("\"pci_devices\""));
    }

    #[test]
    fn json_report_includes_count_and_error_fields() {
        let report = MachineReport {
            generated_at_unix_ms: 1,
            pci_devices: ReportSection {
                items: Vec::new(),
                error: Some("pci err".into()),
            },
            usb_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            network_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
            wifi_interfaces: ReportSection {
                items: Vec::new(),
                error: None,
            },
        };
        let json = render_machine_report(&report, OutputFormat::Json).unwrap();
        assert!(json.contains("\"count\":0"));
        assert!(json.contains("\"error\":\"pci err\""));
    }

    #[test]
    fn format_hex_u8_valid() {
        assert_eq!(format_hex_u8(Some(0xff)), "ff");
        assert_eq!(format_hex_u8(Some(0x00)), "00");
        assert_eq!(format_hex_u8(Some(0x0a)), "0a");
    }

    #[test]
    fn format_hex_u8_none() {
        assert_eq!(format_hex_u8(None), "");
    }

    #[test]
    fn format_hex_u16_valid() {
        assert_eq!(format_hex_u16(Some(0x10de)), "10de");
        assert_eq!(format_hex_u16(Some(0x0000)), "0000");
    }

    #[test]
    fn format_hex_u16_none() {
        assert_eq!(format_hex_u16(None), "");
    }

    #[test]
    fn format_hex_u32_valid() {
        assert_eq!(format_hex_u32(Some(0x030000)), "030000");
        assert_eq!(format_hex_u32(Some(0x000001)), "000001");
    }

    #[test]
    fn format_hex_u32_none() {
        assert_eq!(format_hex_u32(None), "");
    }

    #[test]
    fn render_section_header_without_error() {
        let mut out = String::new();
        render_section_header(&mut out, "Test", 5, None);
        assert_eq!(out.trim(), "Test: 5 items");
    }

    #[test]
    fn render_section_header_with_error() {
        let mut out = String::new();
        render_section_header(&mut out, "Test", 3, Some("oops"));
        assert_eq!(out.trim(), "Test: 3 items (error: oops)");
    }

    #[test]
    fn section_to_json_empty_items() {
        let json = section_to_json::<String>(&[], None, |_item| JsonValue::from("x"));
        assert_eq!(
            json.to_json_string().unwrap(),
            "{\"count\":0,\"error\":null,\"items\":[]}"
        );
    }

    #[test]
    fn section_to_json_with_items_and_error() {
        let json = section_to_json(&[1, 2], Some("err"), |item| (*item).into());
        assert_eq!(
            json.to_json_string().unwrap(),
            "{\"count\":2,\"error\":\"err\",\"items\":[1,2]}"
        );
    }

    #[test]
    fn gather_machine_report_returns_report() {
        // This will read real sysfs data. Just verify it returns a valid report.
        let report = gather_machine_report();
        // generated_at_unix_ms should be recent
        assert!(report.generated_at_unix_ms > 1_700_000_000_000);
    }

    #[test]
    fn gather_machine_report_to_json() {
        let report = gather_machine_report();
        let json = report.to_json();
        // Should have the top-level keys
        let json_str = json.to_json_string().unwrap();
        assert!(json_str.contains("generated_at_unix_ms"));
        assert!(json_str.contains("pci_devices"));
        assert!(json_str.contains("usb_devices"));
        assert!(json_str.contains("network_interfaces"));
        assert!(json_str.contains("wifi_interfaces"));
    }

    #[test]
    fn gather_machine_report_to_text() {
        let report = gather_machine_report();
        let text = render_machine_report(&report, OutputFormat::Text).unwrap();
        assert!(text.contains("edgerun machine report"));
        assert!(text.contains("generated_at_unix_ms:"));
    }
}
