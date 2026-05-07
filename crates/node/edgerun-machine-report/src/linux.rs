extern crate alloc;
use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Write as _;
use core::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use core::option::Option::{self, None, Some};
use core::result::Result::{Err, Ok};
use core::writeln;
use edgerun_encoding::kv::{format_hex_u16, format_hex_u32, format_hex_u8};
#[cfg(target_os = "none")]
use edgerun_linux_pci::path::PathBuf;
#[cfg(not(target_os = "none"))]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
#[cfg(not(target_os = "none"))]
use std::path::PathBuf;

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

use crate::{ContainerRuntime, DeploymentMachineInventory, ServiceManager};

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
    pub cpus: ReportSection<CpuInventoryReport>,
    pub memory: ReportSection<MemoryReport>,
    pub power_supplies: ReportSection<Vec<PowerSupplyReport>>,
    pub thermal_zones: ReportSection<Vec<ThermalZoneReport>>,
    pub storage_mounts: ReportSection<Vec<StorageMountReport>>,
    pub filesystem_permissions: ReportSection<Vec<FilesystemPermissionReport>>,
    pub block_devices: ReportSection<Vec<BlockDeviceReport>>,
    pub device_nodes: ReportSection<Vec<DeviceNodeReport>>,
    pub dns_records: ReportSection<Vec<DnsRecordReport>>,
    pub runtime: ReportSection<RuntimePressureReport>,
    pub kernel: ReportSection<KernelCapabilityReport>,
    pub network_routes: ReportSection<NetworkRouteReport>,
    pub security: ReportSection<SecurityAccessReport>,
    pub packages: ReportSection<PackageSurfaceReport>,
    pub edgerun_conflicts: ReportSection<EdgerunConflictReport>,
    pub processes: ReportSection<Vec<ProcessReport>>,
    pub open_ports: ReportSection<Vec<OpenPortReport>>,
    pub services: ReportSection<ServiceInventoryReport>,
    pub boot: ReportSection<BootConfigurationReport>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimePressureReport {
    pub uptime_seconds: Option<u64>,
    pub idle_seconds: Option<u64>,
    pub load_1m: Option<String>,
    pub load_5m: Option<String>,
    pub load_15m: Option<String>,
    pub runnable_tasks: Option<u64>,
    pub total_tasks: Option<u64>,
    pub last_pid: Option<u64>,
    pub disk_usage: Vec<DiskUsageReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskUsageReport {
    pub target: String,
    pub fs_type: Option<String>,
    pub source: Option<String>,
    pub total_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub capacity_percent: Option<u8>,
    pub total_inodes: Option<u64>,
    pub used_inodes: Option<u64>,
    pub available_inodes: Option<u64>,
    pub inode_capacity_percent: Option<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KernelCapabilityReport {
    pub loaded_modules: Vec<KernelModuleReport>,
    pub filesystems: Vec<KernelFilesystemReport>,
    pub namespaces: Vec<String>,
    pub cgroup_controllers: Vec<String>,
    pub lsm: Vec<String>,
    pub apparmor_enabled: Option<bool>,
    pub selinux_enabled: Option<bool>,
    pub seccomp_available: Option<bool>,
    pub bpf_jit_enabled: Option<bool>,
    pub kexec: KexecReport,
    pub selected_sysctls: Vec<SysctlReport>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KexecReport {
    pub syscall_present: Option<bool>,
    pub load_disabled: Option<bool>,
    pub lockdown: Option<String>,
    pub tool_present: bool,
    pub tool_version: Option<String>,
    pub proc_loaded_image_present: bool,
    pub sysfs_crash_loaded_present: bool,
    pub sysfs_crash_size_present: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelModuleReport {
    pub name: String,
    pub size_bytes: Option<u64>,
    pub ref_count: Option<u64>,
    pub dependencies: Vec<String>,
    pub state: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelFilesystemReport {
    pub name: String,
    pub nodev: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SysctlReport {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetworkRouteReport {
    pub hostname: Option<String>,
    pub fqdn: Option<String>,
    pub resolv_conf: Vec<ResolverConfigReport>,
    pub ipv4_routes: Vec<Ipv4RouteReport>,
    pub ipv6_routes: Vec<Ipv6RouteReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolverConfigReport {
    pub kind: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ipv4RouteReport {
    pub interface: String,
    pub destination: String,
    pub gateway: String,
    pub flags: String,
    pub metric: Option<u32>,
    pub mask: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ipv6RouteReport {
    pub destination: String,
    pub prefix_len: u8,
    pub next_hop: String,
    pub metric: Option<u32>,
    pub interface: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecurityAccessReport {
    pub users: Vec<UserAccountReport>,
    pub groups: Vec<GroupReport>,
    pub sudoers: Vec<PolicyFileReport>,
    pub doas_conf: Option<PolicyFileReport>,
    pub sshd_config: Option<PolicyFileReport>,
    pub authorized_keys: Vec<AuthorizedKeysReport>,
    pub firewall: FirewallReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserAccountReport {
    pub name: String,
    pub uid: u32,
    pub gid: u32,
    pub home: String,
    pub shell: String,
    pub system: bool,
    pub privileged: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupReport {
    pub name: String,
    pub gid: u32,
    pub members: Vec<String>,
    pub privileged: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyFileReport {
    pub path: String,
    pub exists: bool,
    pub readable: bool,
    pub mode_octal: Option<String>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub line_count: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizedKeysReport {
    pub user: String,
    pub path: String,
    pub exists: bool,
    pub readable: bool,
    pub key_count: usize,
    pub mode_octal: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FirewallReport {
    pub nft_present: bool,
    pub nft_ruleset_available: bool,
    pub nft_rule_count: Option<usize>,
    pub iptables_present: bool,
    pub iptables_rule_count: Option<usize>,
    pub ip6tables_present: bool,
    pub ip6tables_rule_count: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackageSurfaceReport {
    pub managers: Vec<PackageManagerReport>,
    pub core_binaries: Vec<BinaryVersionReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageManagerReport {
    pub name: String,
    pub present: bool,
    pub installed_count: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryVersionReport {
    pub name: String,
    pub path_present: bool,
    pub version: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EdgerunConflictReport {
    pub paths: Vec<PolicyFileReport>,
    pub services: Vec<String>,
    pub processes: Vec<ProcessReport>,
    pub open_ports: Vec<OpenPortReport>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CpuInventoryReport {
    pub architecture: String,
    pub logical_processors: usize,
    pub physical_packages: usize,
    pub physical_cores: usize,
    pub vendor_id: Option<String>,
    pub model_name: Option<String>,
    pub microcode: Option<String>,
    pub flags: Vec<String>,
    pub processors: Vec<CpuProcessorReport>,
    pub frequency_policies: Vec<CpuFrequencyPolicyReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuProcessorReport {
    pub processor: u32,
    pub vendor_id: Option<String>,
    pub model_name: Option<String>,
    pub physical_id: Option<u32>,
    pub core_id: Option<u32>,
    pub siblings: Option<u32>,
    pub cpu_cores: Option<u32>,
    pub cpu_mhz_khz: Option<u64>,
    pub cache_size_kb: Option<u64>,
    pub microcode: Option<String>,
    pub flags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuFrequencyPolicyReport {
    pub policy: String,
    pub affected_cpus: Option<String>,
    pub scaling_driver: Option<String>,
    pub scaling_governor: Option<String>,
    pub scaling_min_freq_khz: Option<u64>,
    pub scaling_max_freq_khz: Option<u64>,
    pub cpuinfo_min_freq_khz: Option<u64>,
    pub cpuinfo_max_freq_khz: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryReport {
    pub mem_total_bytes: Option<u64>,
    pub mem_free_bytes: Option<u64>,
    pub mem_available_bytes: Option<u64>,
    pub buffers_bytes: Option<u64>,
    pub cached_bytes: Option<u64>,
    pub swap_cached_bytes: Option<u64>,
    pub active_bytes: Option<u64>,
    pub inactive_bytes: Option<u64>,
    pub active_file_bytes: Option<u64>,
    pub inactive_file_bytes: Option<u64>,
    pub active_anon_bytes: Option<u64>,
    pub inactive_anon_bytes: Option<u64>,
    pub slab_bytes: Option<u64>,
    pub reclaimable_slab_bytes: Option<u64>,
    pub unreclaimable_slab_bytes: Option<u64>,
    pub kernel_stack_bytes: Option<u64>,
    pub page_tables_bytes: Option<u64>,
    pub percpu_bytes: Option<u64>,
    pub vmalloc_used_bytes: Option<u64>,
    pub dirty_bytes: Option<u64>,
    pub writeback_bytes: Option<u64>,
    pub shmem_bytes: Option<u64>,
    pub swap_total_bytes: Option<u64>,
    pub swap_free_bytes: Option<u64>,
    pub huge_pages_total: Option<u64>,
    pub huge_pages_free: Option<u64>,
    pub huge_page_size_bytes: Option<u64>,
    pub direct_map_4k_bytes: Option<u64>,
    pub direct_map_2m_bytes: Option<u64>,
    pub direct_map_1g_bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PowerSupplyReport {
    pub name: String,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub scope: Option<String>,
    pub online: Option<bool>,
    pub capacity_percent: Option<u32>,
    pub health: Option<String>,
    pub technology: Option<String>,
    pub manufacturer: Option<String>,
    pub model_name: Option<String>,
    pub serial_number: Option<String>,
    pub energy_now_uwatt_hours: Option<u64>,
    pub energy_full_uwatt_hours: Option<u64>,
    pub charge_now_uamp_hours: Option<u64>,
    pub charge_full_uamp_hours: Option<u64>,
    pub power_now_uwatts: Option<u64>,
    pub current_now_uamps: Option<u64>,
    pub voltage_now_uvolts: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalZoneReport {
    pub name: String,
    pub kind: Option<String>,
    pub temp_millidegree_celsius: Option<i64>,
    pub policy: Option<String>,
    pub mode: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceNodeReport {
    pub path: String,
    pub category: String,
    pub node_type: String,
    pub symlink_target: Option<String>,
    pub major: Option<u64>,
    pub minor: Option<u64>,
    pub mode_octal: String,
    pub uid: u32,
    pub gid: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServiceInventoryReport {
    pub openrc: Option<OpenRcServiceInventory>,
    pub systemd: Option<SystemdServiceInventory>,
    pub runit: Option<RunitServiceInventory>,
    pub s6: Option<S6ServiceInventory>,
    pub dinit: Option<DinitServiceInventory>,
    pub supervisor: Option<SupervisorServiceInventory>,
    pub cron: Option<CronServiceInventory>,
    pub rc_local: Option<RcLocalServiceInventory>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenRcServiceInventory {
    pub services: Vec<OpenRcServiceReport>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenRcServiceReport {
    pub name: String,
    pub runlevels: Vec<String>,
    pub status: Option<String>,
    pub path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemdServiceInventory {
    pub units: Vec<SystemdServiceReport>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemdServiceReport {
    pub name: String,
    pub load_state: Option<String>,
    pub active_state: Option<String>,
    pub sub_state: Option<String>,
    pub unit_file_state: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunitServiceInventory {
    pub services: Vec<RunitServiceReport>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunitServiceReport {
    pub name: String,
    pub path: String,
    pub status: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct S6ServiceInventory {
    pub services: Vec<S6ServiceReport>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct S6ServiceReport {
    pub name: String,
    pub path: String,
    pub status: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DinitServiceInventory {
    pub services: Vec<DinitServiceReport>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DinitServiceReport {
    pub name: String,
    pub state: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupervisorServiceInventory {
    pub programs: Vec<SupervisorProgramReport>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupervisorProgramReport {
    pub name: String,
    pub state: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CronServiceInventory {
    pub files: Vec<CronFileReport>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CronFileReport {
    pub path: String,
    pub user: Option<String>,
    pub readable: bool,
    pub entry_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RcLocalServiceInventory {
    pub path: String,
    pub exists: bool,
    pub executable: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BootConfigurationReport {
    pub firmware: BootFirmwareReport,
    pub system: BootSystemReport,
    pub kernel: BootKernelReport,
    pub boot_entries: Vec<BootEntryReport>,
    pub config_files: Vec<BootConfigFileReport>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BootFirmwareReport {
    pub mode: BootFirmwareMode,
    pub secure_boot: Option<bool>,
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub release_date: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BootFirmwareMode {
    Uefi,
    Bios,
    #[default]
    Unknown,
}

impl BootFirmwareMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Uefi => "uefi",
            Self::Bios => "bios",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BootSystemReport {
    pub manufacturer: Option<String>,
    pub product_name: Option<String>,
    pub product_version: Option<String>,
    pub board_name: Option<String>,
    pub board_vendor: Option<String>,
    pub chassis_type: Option<String>,
    pub virtualization: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BootKernelReport {
    pub cmdline: Option<String>,
    pub init: Option<String>,
    pub root: Option<String>,
    pub rootfstype: Option<String>,
    pub console: Vec<String>,
    pub initrd: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootEntryReport {
    pub source: String,
    pub id: String,
    pub title: Option<String>,
    pub version: Option<String>,
    pub linux: Option<String>,
    pub initrd: Vec<String>,
    pub options: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootConfigFileReport {
    pub path: String,
    pub kind: String,
    pub exists: bool,
    pub readable: bool,
    pub size_bytes: Option<u64>,
    pub modified_unix_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DnsRecordReport {
    pub query: String,
    pub record_type: String,
    pub value: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessReport {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub uid: Option<u32>,
    pub name: String,
    pub vm_size_bytes: Option<u64>,
    pub vm_rss_bytes: Option<u64>,
    pub rss_anon_bytes: Option<u64>,
    pub rss_file_bytes: Option<u64>,
    pub rss_shmem_bytes: Option<u64>,
    pub vm_swap_bytes: Option<u64>,
    pub pss_bytes: Option<u64>,
    pub shared_clean_bytes: Option<u64>,
    pub shared_dirty_bytes: Option<u64>,
    pub private_clean_bytes: Option<u64>,
    pub private_dirty_bytes: Option<u64>,
    pub exe: Option<String>,
    pub cmdline: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenPortReport {
    pub protocol: String,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: Option<String>,
    pub remote_port: Option<u16>,
    pub state: String,
    pub inode: Option<u64>,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageMountReport {
    pub source: String,
    pub target: String,
    pub fs_type: String,
    pub options: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemPermissionReport {
    pub path: String,
    pub purpose: String,
    pub exists: bool,
    pub kind: String,
    pub source: Option<String>,
    pub mount_target: Option<String>,
    pub fs_type: Option<String>,
    pub mount_options: Option<String>,
    pub read_only_mount: bool,
    pub noexec_mount: bool,
    pub nosuid_mount: bool,
    pub nodev_mount: bool,
    pub mode_octal: Option<String>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub current_user_read: bool,
    pub current_user_write: bool,
    pub current_user_execute: bool,
    pub current_user_can_create_file: bool,
    pub parent_path: Option<String>,
    pub parent_mode_octal: Option<String>,
    pub parent_uid: Option<u32>,
    pub parent_gid: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockDeviceReport {
    pub name: String,
    pub size_bytes: Option<u64>,
    pub removable: Option<bool>,
    pub read_only: Option<bool>,
    pub rotational: Option<bool>,
    pub model: Option<String>,
    pub vendor: Option<String>,
    pub wwid: Option<String>,
}

pub fn gather_machine_report() -> MachineReport {
    gather_machine_report_for_host(None)
}

pub fn gather_machine_report_for_host(host: Option<&str>) -> MachineReport {
    let generated_at_unix_ms = current_unix_ms();

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
                            cpus: gather_cpus(),
                            memory: gather_memory(),
                            power_supplies: gather_power_supplies(),
                            thermal_zones: gather_thermal_zones(),
                            storage_mounts: gather_storage_mounts(),
                            filesystem_permissions: gather_filesystem_permissions(),
                            block_devices: gather_block_devices(),
                            device_nodes: gather_device_nodes(),
                            dns_records: gather_dns_records(host),
                            runtime: gather_runtime_pressure(),
                            kernel: gather_kernel_capabilities(),
                            network_routes: gather_network_routes(),
                            security: gather_security_access(),
                            packages: gather_package_surface(),
                            edgerun_conflicts: gather_edgerun_conflicts(),
                            processes: gather_processes(),
                            open_ports: gather_open_ports(),
                            services: gather_service_inventory(),
                            boot: gather_boot_configuration(),
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
        cpus: gather_cpus(),
        memory: gather_memory(),
        power_supplies: gather_power_supplies(),
        thermal_zones: gather_thermal_zones(),
        storage_mounts: gather_storage_mounts(),
        filesystem_permissions: gather_filesystem_permissions(),
        block_devices: gather_block_devices(),
        device_nodes: gather_device_nodes(),
        dns_records: gather_dns_records(host),
        runtime: gather_runtime_pressure(),
        kernel: gather_kernel_capabilities(),
        network_routes: gather_network_routes(),
        security: gather_security_access(),
        packages: gather_package_surface(),
        edgerun_conflicts: gather_edgerun_conflicts(),
        processes: gather_processes(),
        open_ports: gather_open_ports(),
        services: gather_service_inventory(),
        boot: gather_boot_configuration(),
    }
}

pub fn gather_deployment_machine_inventory() -> DeploymentMachineInventory {
    DeploymentMachineInventory {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        kernel: command_trim("uname", &["-r"]).unwrap_or_else(|| "unknown".to_string()),
        libc: detect_libc(),
        pid1: detect_pid1(),
        uid: current_uid(),
        is_root: current_uid() == Some(0),
        has_sudo: executable_in_path("sudo"),
        has_doas: executable_in_path("doas"),
        writable_paths: detect_writable_paths(),
        service_managers: detect_service_managers(),
        container_runtimes: detect_container_runtimes(),
        low_port_bind_allowed: current_uid() == Some(0) || linux_unprivileged_low_ports(),
    }
}

fn detect_pid1() -> Option<String> {
    std::fs::read_to_string("/proc/1/comm")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| command_trim("ps", &["-p", "1", "-o", "comm="]))
}

fn current_uid() -> Option<u32> {
    command_trim("id", &["-u"]).and_then(|value| value.parse::<u32>().ok())
}

fn current_gid() -> Option<u32> {
    command_trim("id", &["-g"]).and_then(|value| value.parse::<u32>().ok())
}

fn current_group_ids() -> Vec<u32> {
    command_trim("id", &["-G"])
        .map(|output| {
            output
                .split_whitespace()
                .filter_map(|value| value.parse::<u32>().ok())
                .collect()
        })
        .unwrap_or_default()
}

fn detect_libc() -> Option<String> {
    if let Some(output) = command_trim("ldd", &["--version"]) {
        let lower = output.to_ascii_lowercase();
        if lower.contains("musl") {
            return Some("musl".to_string());
        }
        if lower.contains("glibc") || lower.contains("gnu libc") {
            return Some("glibc".to_string());
        }
    }
    if std::path::Path::new("/lib/ld-musl-x86_64.so.1").exists()
        || std::path::Path::new("/lib/ld-musl-aarch64.so.1").exists()
    {
        return Some("musl".to_string());
    }
    if std::path::Path::new("/lib/x86_64-linux-gnu/libc.so.6").exists()
        || std::path::Path::new("/lib/aarch64-linux-gnu/libc.so.6").exists()
        || std::path::Path::new("/usr/lib/libc.so.6").exists()
    {
        return Some("glibc".to_string());
    }
    None
}

fn detect_writable_paths() -> Vec<String> {
    ["/opt/edgerun", "/usr/local/bin", "/var/lib/edgerun", "/tmp"]
        .into_iter()
        .filter(|path| writable_or_parent_writable(path))
        .map(str::to_string)
        .collect()
}

fn writable_or_parent_writable(path: &str) -> bool {
    let path = std::path::Path::new(path);
    let dir = if path.exists() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    if !dir.is_dir() {
        return false;
    }
    let probe = dir.join(format!(".edgerun-write-probe-{}", std::process::id()));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

fn detect_service_managers() -> Vec<ServiceManager> {
    let mut managers = Vec::new();
    if std::path::Path::new("/run/systemd/system").exists() || executable_in_path("systemctl") {
        managers.push(ServiceManager::Systemd);
    }
    if executable_in_path("rc-service") || std::path::Path::new("/etc/init.d").exists() {
        managers.push(ServiceManager::OpenRc);
    }
    if std::path::Path::new("/etc/runit").exists() || std::path::Path::new("/service").exists() {
        managers.push(ServiceManager::Runit);
    }
    if std::path::Path::new("/etc/s6").exists() || executable_in_path("s6-svscan") {
        managers.push(ServiceManager::S6);
    }
    if executable_in_path("dinitctl") {
        managers.push(ServiceManager::Dinit);
    }
    if executable_in_path("supervisord") || executable_in_path("supervisorctl") {
        managers.push(ServiceManager::Supervisor);
    }
    if executable_in_path("crontab") {
        managers.push(ServiceManager::Cron);
    }
    if std::path::Path::new("/etc/rc.local").exists() {
        managers.push(ServiceManager::RcLocal);
    }
    managers
}

fn detect_container_runtimes() -> Vec<ContainerRuntime> {
    let mut runtimes = Vec::new();
    if executable_in_path("docker") {
        runtimes.push(ContainerRuntime::Docker);
    }
    if executable_in_path("podman") {
        runtimes.push(ContainerRuntime::Podman);
    }
    if executable_in_path("ctr") || executable_in_path("containerd") {
        runtimes.push(ContainerRuntime::Containerd);
    }
    runtimes
}

fn linux_unprivileged_low_ports() -> bool {
    std::fs::read_to_string("/proc/sys/net/ipv4/ip_unprivileged_port_start")
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
        .is_some_and(|start| start == 0)
}

fn executable_in_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(name).is_file())
}

fn command_trim(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(not(target_os = "none"))]
fn current_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(target_os = "none")]
fn current_unix_ms() -> i64 {
    0
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

fn gather_cpus() -> ReportSection<CpuInventoryReport> {
    let text = match std::fs::read_to_string("/proc/cpuinfo") {
        Ok(text) => text,
        Err(error) => {
            return ReportSection {
                items: CpuInventoryReport::default(),
                error: Some(error.to_string()),
            };
        }
    };
    let mut processors = parse_cpuinfo(&text);
    processors.sort_by_key(|cpu| cpu.processor);
    let mut packages = Vec::new();
    let mut cores = Vec::new();
    let mut flags = Vec::new();
    for processor in &processors {
        if let Some(package) = processor.physical_id {
            if !packages.contains(&package) {
                packages.push(package);
            }
        }
        if let (Some(package), Some(core)) = (processor.physical_id, processor.core_id) {
            if !cores.contains(&(package, core)) {
                cores.push((package, core));
            }
        }
        for flag in &processor.flags {
            if !flags.contains(flag) {
                flags.push(flag.clone());
            }
        }
    }
    flags.sort();
    ReportSection {
        items: CpuInventoryReport {
            architecture: std::env::consts::ARCH.to_string(),
            logical_processors: processors.len(),
            physical_packages: if packages.is_empty() {
                usize::from(!processors.is_empty())
            } else {
                packages.len()
            },
            physical_cores: if cores.is_empty() {
                processors
                    .iter()
                    .filter_map(|processor| processor.cpu_cores)
                    .max()
                    .map(|cores| cores as usize)
                    .unwrap_or(processors.len())
            } else {
                cores.len()
            },
            vendor_id: processors.iter().find_map(|cpu| cpu.vendor_id.clone()),
            model_name: processors.iter().find_map(|cpu| cpu.model_name.clone()),
            microcode: processors.iter().find_map(|cpu| cpu.microcode.clone()),
            flags,
            processors,
            frequency_policies: gather_cpu_frequency_policies(),
        },
        error: None,
    }
}

fn parse_cpuinfo(text: &str) -> Vec<CpuProcessorReport> {
    text.split("\n\n")
        .filter_map(|block| {
            let mut processor = CpuProcessorReport {
                processor: 0,
                vendor_id: None,
                model_name: None,
                physical_id: None,
                core_id: None,
                siblings: None,
                cpu_cores: None,
                cpu_mhz_khz: None,
                cache_size_kb: None,
                microcode: None,
                flags: Vec::new(),
            };
            let mut saw_processor = false;
            for line in block.lines() {
                let Some((key, value)) = line.split_once(':') else {
                    continue;
                };
                let key = key.trim();
                let value = value.trim();
                match key {
                    "processor" => {
                        if let Ok(id) = value.parse::<u32>() {
                            processor.processor = id;
                            saw_processor = true;
                        }
                    }
                    "vendor_id" => processor.vendor_id = non_empty_string(value),
                    "model name" | "Hardware" => processor.model_name = non_empty_string(value),
                    "physical id" => processor.physical_id = value.parse::<u32>().ok(),
                    "core id" => processor.core_id = value.parse::<u32>().ok(),
                    "siblings" => processor.siblings = value.parse::<u32>().ok(),
                    "cpu cores" => processor.cpu_cores = value.parse::<u32>().ok(),
                    "cpu MHz" => processor.cpu_mhz_khz = parse_decimal_mhz_to_khz(value),
                    "cache size" => processor.cache_size_kb = parse_kb_value(value),
                    "microcode" => processor.microcode = non_empty_string(value),
                    "flags" | "Features" => {
                        processor.flags = value.split_whitespace().map(str::to_string).collect();
                    }
                    _ => {}
                }
            }
            saw_processor.then_some(processor)
        })
        .collect()
}

fn parse_decimal_mhz_to_khz(value: &str) -> Option<u64> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    let whole = whole.trim().parse::<u64>().ok()?;
    let mut fraction_khz = 0u64;
    let mut scale = 100u64;
    for byte in fraction.bytes().take(3) {
        if !byte.is_ascii_digit() {
            break;
        }
        fraction_khz = fraction_khz.saturating_add(((byte - b'0') as u64).saturating_mul(scale));
        scale /= 10;
    }
    Some(whole.saturating_mul(1000).saturating_add(fraction_khz))
}

fn parse_kb_value(value: &str) -> Option<u64> {
    value
        .split_whitespace()
        .next()
        .and_then(|value| value.parse::<u64>().ok())
}

fn gather_cpu_frequency_policies() -> Vec<CpuFrequencyPolicyReport> {
    let root = std::path::Path::new("/sys/devices/system/cpu/cpufreq");
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut policies = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("policy") {
            continue;
        }
        let path = entry.path();
        policies.push(CpuFrequencyPolicyReport {
            policy: name,
            affected_cpus: read_trimmed(&path.join("affected_cpus")),
            scaling_driver: read_trimmed(&path.join("scaling_driver")),
            scaling_governor: read_trimmed(&path.join("scaling_governor")),
            scaling_min_freq_khz: read_trimmed_u64(&path.join("scaling_min_freq")),
            scaling_max_freq_khz: read_trimmed_u64(&path.join("scaling_max_freq")),
            cpuinfo_min_freq_khz: read_trimmed_u64(&path.join("cpuinfo_min_freq")),
            cpuinfo_max_freq_khz: read_trimmed_u64(&path.join("cpuinfo_max_freq")),
        });
    }
    policies.sort_by(|left, right| left.policy.cmp(&right.policy));
    policies
}

fn gather_memory() -> ReportSection<MemoryReport> {
    let text = match std::fs::read_to_string("/proc/meminfo") {
        Ok(text) => text,
        Err(error) => {
            return ReportSection {
                items: MemoryReport::default(),
                error: Some(error.to_string()),
            };
        }
    };
    let value = |key: &str| meminfo_bytes(&text, key);
    ReportSection {
        items: MemoryReport {
            mem_total_bytes: value("MemTotal"),
            mem_free_bytes: value("MemFree"),
            mem_available_bytes: value("MemAvailable"),
            buffers_bytes: value("Buffers"),
            cached_bytes: value("Cached"),
            swap_cached_bytes: value("SwapCached"),
            active_bytes: value("Active"),
            inactive_bytes: value("Inactive"),
            active_file_bytes: value("Active(file)"),
            inactive_file_bytes: value("Inactive(file)"),
            active_anon_bytes: value("Active(anon)"),
            inactive_anon_bytes: value("Inactive(anon)"),
            slab_bytes: value("Slab"),
            reclaimable_slab_bytes: value("SReclaimable"),
            unreclaimable_slab_bytes: value("SUnreclaim"),
            kernel_stack_bytes: value("KernelStack"),
            page_tables_bytes: value("PageTables"),
            percpu_bytes: value("Percpu"),
            vmalloc_used_bytes: value("VmallocUsed"),
            dirty_bytes: value("Dirty"),
            writeback_bytes: value("Writeback"),
            shmem_bytes: value("Shmem"),
            swap_total_bytes: value("SwapTotal"),
            swap_free_bytes: value("SwapFree"),
            huge_pages_total: meminfo_count(&text, "HugePages_Total"),
            huge_pages_free: meminfo_count(&text, "HugePages_Free"),
            huge_page_size_bytes: value("Hugepagesize"),
            direct_map_4k_bytes: value("DirectMap4k"),
            direct_map_2m_bytes: value("DirectMap2M"),
            direct_map_1g_bytes: value("DirectMap1G"),
        },
        error: None,
    }
}

fn meminfo_bytes(text: &str, key: &str) -> Option<u64> {
    meminfo_count(text, key).map(|kb| kb.saturating_mul(1024))
}

fn meminfo_count(text: &str, key: &str) -> Option<u64> {
    let prefix = format!("{key}:");
    text.lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok())
}

fn gather_power_supplies() -> ReportSection<Vec<PowerSupplyReport>> {
    let root = std::path::Path::new("/sys/class/power_supply");
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            return ReportSection {
                items: Vec::new(),
                error: Some(error.to_string()),
            };
        }
    };
    let mut items = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        items.push(PowerSupplyReport {
            name,
            kind: read_trimmed(&path.join("type")),
            status: read_trimmed(&path.join("status")),
            scope: read_trimmed(&path.join("scope")),
            online: read_bool_file(&path.join("online")),
            capacity_percent: read_trimmed_u64(&path.join("capacity")).map(|value| value as u32),
            health: read_trimmed(&path.join("health")),
            technology: read_trimmed(&path.join("technology")),
            manufacturer: read_trimmed(&path.join("manufacturer")),
            model_name: read_trimmed(&path.join("model_name")),
            serial_number: read_trimmed(&path.join("serial_number")),
            energy_now_uwatt_hours: read_trimmed_u64(&path.join("energy_now")),
            energy_full_uwatt_hours: read_trimmed_u64(&path.join("energy_full")),
            charge_now_uamp_hours: read_trimmed_u64(&path.join("charge_now")),
            charge_full_uamp_hours: read_trimmed_u64(&path.join("charge_full")),
            power_now_uwatts: read_trimmed_u64(&path.join("power_now")),
            current_now_uamps: read_trimmed_u64(&path.join("current_now")),
            voltage_now_uvolts: read_trimmed_u64(&path.join("voltage_now")),
        });
    }
    items.sort_by(|left, right| left.name.cmp(&right.name));
    ReportSection { items, error: None }
}

fn gather_thermal_zones() -> ReportSection<Vec<ThermalZoneReport>> {
    let root = std::path::Path::new("/sys/class/thermal");
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            return ReportSection {
                items: Vec::new(),
                error: Some(error.to_string()),
            };
        }
    };
    let mut items = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("thermal_zone") {
            continue;
        }
        let path = entry.path();
        items.push(ThermalZoneReport {
            name,
            kind: read_trimmed(&path.join("type")),
            temp_millidegree_celsius: read_trimmed(&path.join("temp"))
                .and_then(|value| value.parse::<i64>().ok()),
            policy: read_trimmed(&path.join("policy")),
            mode: read_trimmed(&path.join("mode")),
        });
    }
    items.sort_by(|left, right| left.name.cmp(&right.name));
    ReportSection { items, error: None }
}

fn gather_storage_mounts() -> ReportSection<Vec<StorageMountReport>> {
    match std::fs::read_to_string("/proc/mounts") {
        Ok(mounts) => ReportSection {
            items: mounts
                .lines()
                .filter_map(parse_mount_line)
                .collect::<Vec<_>>(),
            error: None,
        },
        Err(error) => ReportSection {
            items: Vec::new(),
            error: Some(error.to_string()),
        },
    }
}

fn parse_mount_line(line: &str) -> Option<StorageMountReport> {
    let mut fields = line.split_whitespace();
    Some(StorageMountReport {
        source: decode_mount_field(fields.next()?)?,
        target: decode_mount_field(fields.next()?)?,
        fs_type: fields.next()?.to_string(),
        options: fields.next().unwrap_or("").to_string(),
    })
}

fn gather_filesystem_permissions() -> ReportSection<Vec<FilesystemPermissionReport>> {
    let mounts = gather_storage_mounts().items;
    let mut samples = filesystem_permission_samples(&mounts);
    samples.sort_by(|left, right| left.0.cmp(&right.0));
    samples.dedup_by(|left, right| left.0 == right.0);
    let uid = current_uid().unwrap_or(u32::MAX);
    let gid = current_gid().unwrap_or(u32::MAX);
    let groups = current_group_ids();
    let mut items = samples
        .iter()
        .map(|(path, purpose)| {
            filesystem_permission_report(path, purpose, &mounts, uid, gid, &groups)
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.path.cmp(&right.path));
    ReportSection { items, error: None }
}

fn filesystem_permission_samples(mounts: &[StorageMountReport]) -> Vec<(String, String)> {
    let mut samples = Vec::new();
    for mount in mounts {
        samples.push((mount.target.clone(), "mount".to_string()));
    }
    for (path, purpose) in [
        ("/", "root"),
        ("/boot", "boot"),
        ("/etc", "system-config"),
        ("/etc/edgerun", "edgerun-config"),
        ("/opt", "optional-apps"),
        ("/opt/edgerun", "edgerun-install"),
        ("/usr/local/bin", "local-bin"),
        ("/var", "state-root"),
        ("/var/lib", "state-root"),
        ("/var/lib/edgerun", "edgerun-state"),
        ("/var/log", "logs"),
        ("/run", "runtime-state"),
        ("/tmp", "temporary"),
        ("/home", "home-root"),
    ] {
        samples.push((path.to_string(), purpose.to_string()));
    }
    if let Some(home) = std::env::var_os("HOME") {
        samples.push((
            home.to_string_lossy().into_owned(),
            "current-home".to_string(),
        ));
    }
    if let Ok(cwd) = std::env::current_dir() {
        samples.push((
            cwd.display().to_string(),
            "current-working-directory".to_string(),
        ));
    }
    samples
}

fn filesystem_permission_report(
    path: &str,
    purpose: &str,
    mounts: &[StorageMountReport],
    current_uid: u32,
    current_gid: u32,
    current_groups: &[u32],
) -> FilesystemPermissionReport {
    let path_ref = std::path::Path::new(path);
    let metadata = std::fs::symlink_metadata(path_ref).ok();
    let parent_path = path_ref.parent().map(|parent| parent.display().to_string());
    let parent_metadata = path_ref
        .parent()
        .and_then(|parent| std::fs::symlink_metadata(parent).ok());
    let permission_metadata = metadata.as_ref().or(parent_metadata.as_ref());
    let mount = find_mount_for_path(mounts, path);
    let mount_options = mount.map(|mount| mount.options.clone());
    let read_only_mount = mount_options
        .as_deref()
        .map(|options| mount_option_present(options, "ro"))
        .unwrap_or(false);
    let noexec_mount = mount_options
        .as_deref()
        .map(|options| mount_option_present(options, "noexec"))
        .unwrap_or(false);
    let nosuid_mount = mount_options
        .as_deref()
        .map(|options| mount_option_present(options, "nosuid"))
        .unwrap_or(false);
    let nodev_mount = mount_options
        .as_deref()
        .map(|options| mount_option_present(options, "nodev"))
        .unwrap_or(false);
    FilesystemPermissionReport {
        path: path.to_string(),
        purpose: purpose.to_string(),
        exists: metadata.is_some(),
        kind: metadata
            .as_ref()
            .map(metadata_kind)
            .unwrap_or_else(|| "missing".to_string()),
        source: mount.map(|mount| mount.source.clone()),
        mount_target: mount.map(|mount| mount.target.clone()),
        fs_type: mount.map(|mount| mount.fs_type.clone()),
        mount_options,
        read_only_mount,
        noexec_mount,
        nosuid_mount,
        nodev_mount,
        mode_octal: metadata.as_ref().map(metadata_mode_octal),
        uid: metadata.as_ref().map(|metadata| metadata.uid()),
        gid: metadata.as_ref().map(|metadata| metadata.gid()),
        current_user_read: permission_metadata
            .map(|metadata| {
                metadata_allows(metadata, current_uid, current_gid, current_groups, 0o444)
            })
            .unwrap_or(false),
        current_user_write: permission_metadata
            .map(|metadata| {
                !read_only_mount
                    && metadata_allows(metadata, current_uid, current_gid, current_groups, 0o222)
            })
            .unwrap_or(false),
        current_user_execute: permission_metadata
            .map(|metadata| {
                !noexec_mount
                    && metadata_allows(metadata, current_uid, current_gid, current_groups, 0o111)
            })
            .unwrap_or(false),
        current_user_can_create_file: can_create_file_in_path(path_ref, read_only_mount),
        parent_path,
        parent_mode_octal: parent_metadata.as_ref().map(metadata_mode_octal),
        parent_uid: parent_metadata.as_ref().map(|metadata| metadata.uid()),
        parent_gid: parent_metadata.as_ref().map(|metadata| metadata.gid()),
    }
}

fn find_mount_for_path<'a>(
    mounts: &'a [StorageMountReport],
    path: &str,
) -> Option<&'a StorageMountReport> {
    mounts
        .iter()
        .filter(|mount| path_is_under_mount(path, &mount.target))
        .max_by_key(|mount| mount.target.len())
}

fn path_is_under_mount(path: &str, mount: &str) -> bool {
    if mount == "/" {
        return path.starts_with('/');
    }
    path == mount
        || path
            .strip_prefix(mount)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn mount_option_present(options: &str, needle: &str) -> bool {
    options.split(',').any(|option| option == needle)
}

fn metadata_kind(metadata: &std::fs::Metadata) -> String {
    let file_type = metadata.file_type();
    if file_type.is_dir() {
        "dir"
    } else if file_type.is_file() {
        "file"
    } else if file_type.is_symlink() {
        "symlink"
    } else if file_type.is_char_device() {
        "char"
    } else if file_type.is_block_device() {
        "block"
    } else if file_type.is_socket() {
        "socket"
    } else if file_type.is_fifo() {
        "fifo"
    } else {
        "unknown"
    }
    .to_string()
}

fn metadata_mode_octal(metadata: &std::fs::Metadata) -> String {
    format!("{:04o}", metadata.permissions().mode() & 0o7777)
}

fn metadata_allows(
    metadata: &std::fs::Metadata,
    current_uid: u32,
    current_gid: u32,
    current_groups: &[u32],
    masks: u32,
) -> bool {
    if current_uid == 0 {
        return true;
    }
    let mode = metadata.permissions().mode();
    let user_bit = (masks & 0o700) >> 6;
    let group_bit = (masks & 0o070) >> 3;
    let other_bit = masks & 0o007;
    if metadata.uid() == current_uid {
        return mode & (user_bit << 6) != 0;
    }
    if metadata.gid() == current_gid || current_groups.contains(&metadata.gid()) {
        return mode & (group_bit << 3) != 0;
    }
    mode & other_bit != 0
}

fn can_create_file_in_path(path: &std::path::Path, read_only_mount: bool) -> bool {
    if read_only_mount {
        return false;
    }
    let dir = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    if !dir.is_dir() {
        return false;
    }
    let probe = dir.join(format!(".edgerun-permission-probe-{}", std::process::id()));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

fn decode_mount_field(value: &str) -> Option<String> {
    let mut out = String::new();
    let bytes = value.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b'\\' && idx + 3 < bytes.len() {
            let octal = core::str::from_utf8(&bytes[idx + 1..idx + 4]).ok()?;
            if let Ok(value) = u8::from_str_radix(octal, 8) {
                out.push(value as char);
                idx += 4;
                continue;
            }
        }
        out.push(bytes[idx] as char);
        idx += 1;
    }
    Some(out)
}

fn gather_block_devices() -> ReportSection<Vec<BlockDeviceReport>> {
    let root = std::path::Path::new("/sys/block");
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            return ReportSection {
                items: Vec::new(),
                error: Some(error.to_string()),
            };
        }
    };
    let mut items = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        items.push(BlockDeviceReport {
            name,
            size_bytes: read_trimmed(&path.join("size"))
                .and_then(|value| value.parse::<u64>().ok())
                .map(|sectors| sectors.saturating_mul(512)),
            removable: read_bool_file(&path.join("removable")),
            read_only: read_bool_file(&path.join("ro")),
            rotational: read_bool_file(&path.join("queue/rotational")),
            model: read_trimmed(&path.join("device/model")),
            vendor: read_trimmed(&path.join("device/vendor")),
            wwid: read_trimmed(&path.join("wwid")),
        });
    }
    items.sort_by(|left, right| left.name.cmp(&right.name));
    ReportSection { items, error: None }
}

fn gather_device_nodes() -> ReportSection<Vec<DeviceNodeReport>> {
    let roots = [
        ("/dev", 1usize),
        ("/dev/bus/usb", 3),
        ("/dev/dri", 1),
        ("/dev/input", 1),
        ("/dev/net", 1),
        ("/dev/snd", 1),
        ("/dev/v4l", 2),
        ("/dev/serial", 2),
        ("/dev/disk", 2),
        ("/dev/mapper", 1),
        ("/dev/vfio", 1),
    ];
    let mut items = Vec::new();
    let mut errors = Vec::new();
    for (root, max_depth) in roots {
        let path = std::path::Path::new(root);
        if !path.exists() {
            continue;
        }
        if !collect_device_tree(path, max_depth, &mut items) {
            errors.push(format!("{root}: read_dir failed"));
        }
    }
    items.sort_by(|left, right| left.path.cmp(&right.path));
    items.dedup_by(|left, right| left.path == right.path);
    ReportSection {
        items,
        error: (!errors.is_empty()).then(|| errors.join("; ")),
    }
}

fn collect_device_tree(
    path: &std::path::Path,
    remaining_depth: usize,
    out: &mut Vec<DeviceNodeReport>,
) -> bool {
    collect_device_node(path, out);
    if remaining_depth == 0 {
        return true;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    for entry in entries.flatten() {
        let child = entry.path();
        collect_device_node(&child, out);
        if child.is_dir() {
            collect_device_tree(&child, remaining_depth - 1, out);
        }
    }
    true
}

fn collect_device_node(path: &std::path::Path, out: &mut Vec<DeviceNodeReport>) {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return;
    };
    let file_type = metadata.file_type();
    let node_type = if file_type.is_char_device() {
        "char"
    } else if file_type.is_block_device() {
        "block"
    } else if file_type.is_symlink() {
        "symlink"
    } else if file_type.is_dir() {
        "dir"
    } else if file_type.is_file() {
        "file"
    } else if file_type.is_socket() {
        "socket"
    } else if file_type.is_fifo() {
        "fifo"
    } else {
        "unknown"
    };
    if node_type == "dir" && path != std::path::Path::new("/dev") {
        return;
    }
    let rdev = metadata.rdev();
    let (major, minor) = if file_type.is_char_device() || file_type.is_block_device() {
        (Some(linux_dev_major(rdev)), Some(linux_dev_minor(rdev)))
    } else {
        (None, None)
    };
    let path_text = path.display().to_string();
    out.push(DeviceNodeReport {
        category: device_node_category(&path_text).to_string(),
        symlink_target: std::fs::read_link(path)
            .ok()
            .map(|target| target.display().to_string()),
        path: path_text,
        node_type: node_type.to_string(),
        major,
        minor,
        mode_octal: format!("{:04o}", metadata.permissions().mode() & 0o7777),
        uid: metadata.uid(),
        gid: metadata.gid(),
    });
}

fn linux_dev_major(dev: u64) -> u64 {
    ((dev >> 8) & 0x0fff) | ((dev >> 32) & !0x0fff)
}

fn linux_dev_minor(dev: u64) -> u64 {
    (dev & 0x00ff) | ((dev >> 12) & !0x00ff)
}

fn device_node_category(path: &str) -> &'static str {
    let name = path.rsplit('/').next().unwrap_or(path);
    if path.starts_with("/dev/dri") {
        "gpu"
    } else if path.starts_with("/dev/input")
        || name.starts_with("input")
        || name.starts_with("event")
    {
        "input"
    } else if path.starts_with("/dev/snd") || name.starts_with("snd") {
        "audio"
    } else if path.starts_with("/dev/v4l") || name.starts_with("video") || name.starts_with("media")
    {
        "video"
    } else if path.starts_with("/dev/disk")
        || name.starts_with("sd")
        || name.starts_with("vd")
        || name.starts_with("nvme")
        || name.starts_with("loop")
        || name.starts_with("zram")
    {
        "block"
    } else if path.starts_with("/dev/serial")
        || name.starts_with("ttyS")
        || name.starts_with("ttyUSB")
        || name.starts_with("ttyACM")
    {
        "serial"
    } else if name == "kvm" {
        "virtualization"
    } else if name == "net" || name == "tun" || path == "/dev/net/tun" {
        "network"
    } else if name.starts_with("usb") || path.starts_with("/dev/bus/usb") {
        "usb"
    } else if path.starts_with("/dev/vfio") {
        "vfio"
    } else {
        "misc"
    }
}

fn read_bool_file(path: &std::path::Path) -> Option<bool> {
    read_trimmed(path).and_then(|value| match value.as_str() {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    })
}

fn read_trimmed_u64(path: &std::path::Path) -> Option<u64> {
    read_trimmed(path).and_then(|value| value.parse::<u64>().ok())
}

fn read_trimmed(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn non_empty_string(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn gather_runtime_pressure() -> ReportSection<RuntimePressureReport> {
    ReportSection {
        items: RuntimePressureReport {
            uptime_seconds: read_uptime_field(0),
            idle_seconds: read_uptime_field(1),
            load_1m: read_load_field(0),
            load_5m: read_load_field(1),
            load_15m: read_load_field(2),
            runnable_tasks: read_load_tasks().map(|(runnable, _total)| runnable),
            total_tasks: read_load_tasks().map(|(_runnable, total)| total),
            last_pid: read_load_field(4).and_then(|value| value.parse::<u64>().ok()),
            disk_usage: gather_disk_usage(),
        },
        error: None,
    }
}

fn read_uptime_field(index: usize) -> Option<u64> {
    let text = std::fs::read_to_string("/proc/uptime").ok()?;
    let value = text.split_whitespace().nth(index)?;
    parse_decimal_seconds_to_u64(value)
}

fn parse_decimal_seconds_to_u64(value: &str) -> Option<u64> {
    value.split('.').next()?.parse::<u64>().ok()
}

fn read_load_field(index: usize) -> Option<String> {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|text| text.split_whitespace().nth(index).map(str::to_string))
}

fn read_load_tasks() -> Option<(u64, u64)> {
    let text = std::fs::read_to_string("/proc/loadavg").ok()?;
    let tasks = text.split_whitespace().nth(3)?;
    let (runnable, total) = tasks.split_once('/')?;
    Some((runnable.parse().ok()?, total.parse().ok()?))
}

fn gather_disk_usage() -> Vec<DiskUsageReport> {
    let mounts = gather_storage_mounts().items;
    let mut reports = parse_df_bytes(&mounts);
    merge_df_inodes(&mut reports);
    reports.sort_by(|left, right| left.target.cmp(&right.target));
    reports
}

fn parse_df_bytes(mounts: &[StorageMountReport]) -> Vec<DiskUsageReport> {
    let Some(output) = command_stdout("df", &["-P", "-B1"]) else {
        return Vec::new();
    };
    output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() < 6 {
                return None;
            }
            let target = fields[5].to_string();
            let mount = find_mount_for_path(mounts, &target);
            Some(DiskUsageReport {
                target,
                fs_type: mount.map(|mount| mount.fs_type.clone()),
                source: mount
                    .map(|mount| mount.source.clone())
                    .or_else(|| Some(fields[0].to_string())),
                total_bytes: fields[1].parse::<u64>().ok(),
                used_bytes: fields[2].parse::<u64>().ok(),
                available_bytes: fields[3].parse::<u64>().ok(),
                capacity_percent: parse_percent_u8(fields[4]),
                total_inodes: None,
                used_inodes: None,
                available_inodes: None,
                inode_capacity_percent: None,
            })
        })
        .collect()
}

fn merge_df_inodes(reports: &mut [DiskUsageReport]) {
    let Some(output) = command_stdout("df", &["-P", "-i"]) else {
        return;
    };
    for line in output.lines().skip(1) {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 6 {
            continue;
        }
        if let Some(report) = reports.iter_mut().find(|report| report.target == fields[5]) {
            report.total_inodes = fields[1].parse::<u64>().ok();
            report.used_inodes = fields[2].parse::<u64>().ok();
            report.available_inodes = fields[3].parse::<u64>().ok();
            report.inode_capacity_percent = parse_percent_u8(fields[4]);
        }
    }
}

fn parse_percent_u8(value: &str) -> Option<u8> {
    value
        .trim_end_matches('%')
        .parse::<u16>()
        .ok()
        .and_then(|value| (value <= u8::MAX as u16).then_some(value as u8))
}

fn gather_kernel_capabilities() -> ReportSection<KernelCapabilityReport> {
    ReportSection {
        items: KernelCapabilityReport {
            loaded_modules: gather_kernel_modules(),
            filesystems: gather_kernel_filesystems(),
            namespaces: gather_namespaces(),
            cgroup_controllers: gather_cgroup_controllers(),
            lsm: read_trimmed(std::path::Path::new("/sys/kernel/security/lsm"))
                .map(|value| value.split(',').map(str::to_string).collect())
                .unwrap_or_default(),
            apparmor_enabled: read_trimmed(std::path::Path::new(
                "/sys/module/apparmor/parameters/enabled",
            ))
            .map(|value| matches!(value.as_str(), "Y" | "y" | "1")),
            selinux_enabled: std::path::Path::new("/sys/fs/selinux")
                .exists()
                .then_some(true),
            seccomp_available: read_trimmed(std::path::Path::new(
                "/proc/sys/kernel/seccomp/actions_avail",
            ))
            .map(|value| !value.is_empty()),
            bpf_jit_enabled: read_bool_file(std::path::Path::new(
                "/proc/sys/net/core/bpf_jit_enable",
            )),
            kexec: gather_kexec_report(),
            selected_sysctls: gather_selected_sysctls(),
        },
        error: None,
    }
}

fn gather_kernel_modules() -> Vec<KernelModuleReport> {
    std::fs::read_to_string("/proc/modules")
        .ok()
        .map(|text| {
            text.lines()
                .filter_map(|line| {
                    let fields = line.split_whitespace().collect::<Vec<_>>();
                    Some(KernelModuleReport {
                        name: fields.first()?.to_string(),
                        size_bytes: fields.get(1).and_then(|value| value.parse().ok()),
                        ref_count: fields.get(2).and_then(|value| value.parse().ok()),
                        dependencies: fields
                            .get(3)
                            .filter(|value| **value != "-")
                            .map(|value| {
                                value
                                    .split(',')
                                    .filter(|value| !value.is_empty())
                                    .map(str::to_string)
                                    .collect()
                            })
                            .unwrap_or_default(),
                        state: fields.get(4).map(|value| (*value).to_string()),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn gather_kernel_filesystems() -> Vec<KernelFilesystemReport> {
    std::fs::read_to_string("/proc/filesystems")
        .ok()
        .map(|text| {
            text.lines()
                .filter_map(|line| {
                    let fields = line.split_whitespace().collect::<Vec<_>>();
                    match fields.as_slice() {
                        ["nodev", name] => Some(KernelFilesystemReport {
                            name: (*name).to_string(),
                            nodev: true,
                        }),
                        [name] => Some(KernelFilesystemReport {
                            name: (*name).to_string(),
                            nodev: false,
                        }),
                        _ => None,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn gather_namespaces() -> Vec<String> {
    std::fs::read_dir("/proc/self/ns")
        .ok()
        .map(|entries| {
            let mut names = entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            names.sort();
            names
        })
        .unwrap_or_default()
}

fn gather_cgroup_controllers() -> Vec<String> {
    if let Some(value) = read_trimmed(std::path::Path::new("/sys/fs/cgroup/cgroup.controllers")) {
        return value.split_whitespace().map(str::to_string).collect();
    }
    std::fs::read_to_string("/proc/cgroups")
        .ok()
        .map(|text| {
            text.lines()
                .filter(|line| !line.starts_with('#'))
                .filter_map(|line| line.split_whitespace().next().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn gather_kexec_report() -> KexecReport {
    let tool_present = executable_in_path("kexec");
    KexecReport {
        syscall_present: detect_kexec_syscall(),
        load_disabled: read_bool_file(std::path::Path::new("/proc/sys/kernel/kexec_load_disabled")),
        lockdown: read_trimmed(std::path::Path::new("/sys/kernel/security/lockdown")),
        tool_present,
        tool_version: tool_present
            .then(|| command_output_text("kexec", &["--version"]))
            .flatten()
            .map(first_output_line),
        proc_loaded_image_present: std::path::Path::new("/proc/kexec_loaded").exists(),
        sysfs_crash_loaded_present: std::path::Path::new("/sys/kernel/kexec_crash_loaded").exists(),
        sysfs_crash_size_present: std::path::Path::new("/sys/kernel/kexec_crash_size").exists(),
    }
}

#[cfg(target_arch = "x86_64")]
fn detect_kexec_syscall() -> Option<bool> {
    let text = std::fs::read_to_string("/proc/kallsyms").ok()?;
    if text.contains(" sys_kexec_load\n")
        || text.contains(" __x64_sys_kexec_load\n")
        || text.contains(" __ia32_sys_kexec_load\n")
    {
        Some(true)
    } else {
        None
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn detect_kexec_syscall() -> Option<bool> {
    None
}

fn first_output_line(text: String) -> String {
    text.lines().next().unwrap_or("").trim().to_string()
}

fn gather_selected_sysctls() -> Vec<SysctlReport> {
    [
        ("net.ipv4.ip_forward", "/proc/sys/net/ipv4/ip_forward"),
        (
            "net.ipv4.ip_unprivileged_port_start",
            "/proc/sys/net/ipv4/ip_unprivileged_port_start",
        ),
        ("net.core.somaxconn", "/proc/sys/net/core/somaxconn"),
        (
            "kernel.unprivileged_bpf_disabled",
            "/proc/sys/kernel/unprivileged_bpf_disabled",
        ),
        ("kernel.kptr_restrict", "/proc/sys/kernel/kptr_restrict"),
        ("kernel.dmesg_restrict", "/proc/sys/kernel/dmesg_restrict"),
        (
            "kernel.kexec_load_disabled",
            "/proc/sys/kernel/kexec_load_disabled",
        ),
        ("fs.file-max", "/proc/sys/fs/file-max"),
    ]
    .into_iter()
    .map(|(name, path)| SysctlReport {
        name: name.to_string(),
        value: read_trimmed(std::path::Path::new(path)),
    })
    .collect()
}

fn gather_network_routes() -> ReportSection<NetworkRouteReport> {
    ReportSection {
        items: NetworkRouteReport {
            hostname: command_trim("hostname", &[]),
            fqdn: command_trim("hostname", &["-f"]),
            resolv_conf: gather_resolver_config(),
            ipv4_routes: gather_ipv4_routes(),
            ipv6_routes: gather_ipv6_routes(),
        },
        error: None,
    }
}

fn gather_resolver_config() -> Vec<ResolverConfigReport> {
    std::fs::read_to_string("/etc/resolv.conf")
        .ok()
        .map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .filter_map(|line| {
                    let mut fields = line.split_whitespace();
                    Some(ResolverConfigReport {
                        kind: fields.next()?.to_string(),
                        value: fields.collect::<Vec<_>>().join(" "),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn gather_ipv4_routes() -> Vec<Ipv4RouteReport> {
    std::fs::read_to_string("/proc/net/route")
        .ok()
        .map(|text| {
            text.lines()
                .skip(1)
                .filter_map(|line| {
                    let fields = line.split_whitespace().collect::<Vec<_>>();
                    Some(Ipv4RouteReport {
                        interface: fields.first()?.to_string(),
                        destination: parse_ipv4_hex(fields.get(1)?)?.to_string(),
                        gateway: parse_ipv4_hex(fields.get(2)?)?.to_string(),
                        flags: fields.get(3)?.to_string(),
                        metric: fields.get(6).and_then(|value| value.parse().ok()),
                        mask: parse_ipv4_hex(fields.get(7)?)?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn gather_ipv6_routes() -> Vec<Ipv6RouteReport> {
    std::fs::read_to_string("/proc/net/ipv6_route")
        .ok()
        .map(|text| {
            text.lines()
                .filter_map(|line| {
                    let fields = line.split_whitespace().collect::<Vec<_>>();
                    if fields.len() < 10 {
                        return None;
                    }
                    Some(Ipv6RouteReport {
                        destination: parse_ipv6_hex(fields[0])?.to_string(),
                        prefix_len: u8::from_str_radix(fields[1], 16).ok()?,
                        next_hop: parse_ipv6_hex(fields[4])?.to_string(),
                        metric: u32::from_str_radix(fields[5], 16).ok(),
                        interface: fields[9].to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn gather_dns_records(host: Option<&str>) -> ReportSection<Vec<DnsRecordReport>> {
    let mut items = Vec::new();
    let Some(host) = host.map(str::trim).filter(|host| !host.is_empty()) else {
        return ReportSection { items, error: None };
    };
    if let Ok(addrs) = std::net::ToSocketAddrs::to_socket_addrs(&(host, 0)) {
        for addr in addrs {
            let ip = addr.ip();
            let record_type = if ip.is_ipv4() { "A" } else { "AAAA" };
            push_dns_record(
                &mut items,
                host,
                record_type,
                &ip.to_string(),
                "system-resolver",
            );
            for ptr in reverse_dns_names(&ip) {
                push_dns_record(&mut items, &ip.to_string(), "PTR", &ptr, "system-resolver");
            }
        }
    }
    for record in getent_hosts(host) {
        push_dns_record(&mut items, host, &record.0, &record.1, "getent-hosts");
    }
    items.sort_by(|left, right| {
        (&left.query, &left.record_type, &left.value).cmp(&(
            &right.query,
            &right.record_type,
            &right.value,
        ))
    });
    items.dedup_by(|left, right| {
        left.query == right.query
            && left.record_type == right.record_type
            && left.value == right.value
    });
    ReportSection { items, error: None }
}

fn push_dns_record(
    items: &mut Vec<DnsRecordReport>,
    query: &str,
    record_type: &str,
    value: &str,
    source: &str,
) {
    items.push(DnsRecordReport {
        query: query.to_string(),
        record_type: record_type.to_string(),
        value: value.to_string(),
        source: source.to_string(),
    });
}

fn getent_hosts(host: &str) -> Vec<(String, String)> {
    let Some(output) = command_stdout("getent", &["hosts", host]) else {
        return Vec::new();
    };
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let value = fields.next()?;
            let record_type = value
                .parse::<IpAddr>()
                .ok()
                .map(|ip| if ip.is_ipv4() { "A" } else { "AAAA" })
                .unwrap_or("HOST");
            Some((record_type.to_string(), value.to_string()))
        })
        .collect()
}

fn reverse_dns_names(ip: &IpAddr) -> Vec<String> {
    let ip_text = ip.to_string();
    command_stdout("getent", &["hosts", &ip_text])
        .map(|output| {
            output
                .lines()
                .flat_map(|line| line.split_whitespace().skip(1))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn gather_security_access() -> ReportSection<SecurityAccessReport> {
    ReportSection {
        items: SecurityAccessReport {
            users: gather_users(),
            groups: gather_groups(),
            sudoers: gather_sudoers_files(),
            doas_conf: Some(policy_file_report("/etc/doas.conf")),
            sshd_config: Some(policy_file_report("/etc/ssh/sshd_config")),
            authorized_keys: gather_authorized_keys(),
            firewall: gather_firewall(),
        },
        error: None,
    }
}

fn gather_users() -> Vec<UserAccountReport> {
    let mut users = std::fs::read_to_string("/etc/passwd")
        .ok()
        .map(|text| {
            text.lines()
                .filter_map(|line| {
                    let fields = line.split(':').collect::<Vec<_>>();
                    let uid = fields.get(2)?.parse::<u32>().ok()?;
                    let gid = fields.get(3)?.parse::<u32>().ok()?;
                    let name = fields.first()?.to_string();
                    Some(UserAccountReport {
                        privileged: uid == 0 || name == "root",
                        system: uid < 1000 && uid != 0,
                        name,
                        uid,
                        gid,
                        home: fields.get(5).unwrap_or(&"").to_string(),
                        shell: fields.get(6).unwrap_or(&"").to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    users.sort_by(|left, right| left.uid.cmp(&right.uid).then(left.name.cmp(&right.name)));
    users
}

fn gather_groups() -> Vec<GroupReport> {
    let privileged = ["root", "wheel", "sudo", "doas", "adm", "docker", "podman"];
    let mut groups = std::fs::read_to_string("/etc/group")
        .ok()
        .map(|text| {
            text.lines()
                .filter_map(|line| {
                    let fields = line.split(':').collect::<Vec<_>>();
                    let name = fields.first()?.to_string();
                    let gid = fields.get(2)?.parse::<u32>().ok()?;
                    Some(GroupReport {
                        privileged: privileged.contains(&name.as_str()) || gid == 0,
                        members: fields
                            .get(3)
                            .unwrap_or(&"")
                            .split(',')
                            .filter(|value| !value.is_empty())
                            .map(str::to_string)
                            .collect(),
                        name,
                        gid,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    groups.sort_by(|left, right| left.gid.cmp(&right.gid).then(left.name.cmp(&right.name)));
    groups
}

fn gather_sudoers_files() -> Vec<PolicyFileReport> {
    let mut files = vec![policy_file_report("/etc/sudoers")];
    if let Ok(entries) = std::fs::read_dir("/etc/sudoers.d") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(policy_file_report(&path.display().to_string()));
            }
        }
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    files
}

fn policy_file_report(path: &str) -> PolicyFileReport {
    let metadata = std::fs::symlink_metadata(path).ok();
    let text = std::fs::read_to_string(path).ok();
    PolicyFileReport {
        path: path.to_string(),
        exists: metadata.is_some(),
        readable: text.is_some(),
        mode_octal: metadata.as_ref().map(metadata_mode_octal),
        uid: metadata.as_ref().map(|metadata| metadata.uid()),
        gid: metadata.as_ref().map(|metadata| metadata.gid()),
        line_count: text.map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .count()
        }),
    }
}

fn gather_authorized_keys() -> Vec<AuthorizedKeysReport> {
    let mut reports = Vec::new();
    for user in gather_users() {
        if user.home.is_empty() || user.home == "/" || user.shell.contains("nologin") {
            continue;
        }
        let path = format!("{}/.ssh/authorized_keys", user.home.trim_end_matches('/'));
        let metadata = std::fs::symlink_metadata(&path).ok();
        let text = std::fs::read_to_string(&path).ok();
        reports.push(AuthorizedKeysReport {
            user: user.name,
            path,
            exists: metadata.is_some(),
            readable: text.is_some(),
            key_count: text
                .map(|text| {
                    text.lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty() && !line.starts_with('#'))
                        .count()
                })
                .unwrap_or(0),
            mode_octal: metadata.as_ref().map(metadata_mode_octal),
        });
    }
    reports.sort_by(|left, right| left.user.cmp(&right.user));
    reports
}

fn gather_firewall() -> FirewallReport {
    let nft_present = executable_in_path("nft");
    let nft_output = nft_present
        .then(|| command_stdout("nft", &["list", "ruleset"]))
        .flatten();
    let iptables_present = executable_in_path("iptables");
    let iptables_output = iptables_present
        .then(|| command_stdout("iptables", &["-S"]))
        .flatten();
    let ip6tables_present = executable_in_path("ip6tables");
    let ip6tables_output = ip6tables_present
        .then(|| command_stdout("ip6tables", &["-S"]))
        .flatten();
    FirewallReport {
        nft_present,
        nft_ruleset_available: nft_output.is_some(),
        nft_rule_count: nft_output.as_deref().map(non_comment_line_count),
        iptables_present,
        iptables_rule_count: iptables_output.as_deref().map(non_comment_line_count),
        ip6tables_present,
        ip6tables_rule_count: ip6tables_output.as_deref().map(non_comment_line_count),
    }
}

fn non_comment_line_count(text: &str) -> usize {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .count()
}

fn gather_package_surface() -> ReportSection<PackageSurfaceReport> {
    ReportSection {
        items: PackageSurfaceReport {
            managers: gather_package_managers(),
            core_binaries: gather_core_binaries(),
        },
        error: None,
    }
}

fn gather_package_managers() -> Vec<PackageManagerReport> {
    [
        ("apk", "apk", &["info"][..]),
        ("pacman", "pacman", &["-Qq"][..]),
        (
            "dpkg",
            "dpkg-query",
            &["-f", "${binary:Package}\n", "-W"][..],
        ),
        ("rpm", "rpm", &["-qa"][..]),
    ]
    .into_iter()
    .map(|(name, command, args)| {
        let present = executable_in_path(command);
        let installed_count = present
            .then(|| command_stdout(command, args))
            .flatten()
            .as_deref()
            .map(non_comment_line_count);
        PackageManagerReport {
            name: name.to_string(),
            present,
            installed_count,
        }
    })
    .collect()
}

fn gather_core_binaries() -> Vec<BinaryVersionReport> {
    [
        ("sh", &["--version"][..]),
        ("ssh", &["-V"][..]),
        ("sshd", &["-V"][..]),
        ("nft", &["--version"][..]),
        ("iptables", &["--version"][..]),
        ("docker", &["--version"][..]),
        ("podman", &["--version"][..]),
        ("containerd", &["--version"][..]),
        ("openssl", &["version"][..]),
    ]
    .into_iter()
    .map(|(name, args)| BinaryVersionReport {
        name: name.to_string(),
        path_present: executable_in_path(name),
        version: executable_in_path(name)
            .then(|| command_output_text(name, args))
            .flatten(),
    })
    .collect()
}

fn gather_edgerun_conflicts() -> ReportSection<EdgerunConflictReport> {
    let processes = gather_processes().items;
    let open_ports = gather_open_ports().items;
    let current_pid = std::process::id();
    ReportSection {
        items: EdgerunConflictReport {
            paths: [
                "/etc/edgerun",
                "/opt/edgerun",
                "/var/lib/edgerun",
                "/usr/local/bin/edgerun-server",
                "/etc/init.d/edgerun-server",
                "/etc/systemd/system/edgerun-server.service",
                "/etc/systemd/system/edgerun.service",
            ]
            .into_iter()
            .map(policy_file_report)
            .collect(),
            services: gather_edgerun_services(),
            processes: processes
                .into_iter()
                .filter(|process| process.pid != current_pid && process_is_edgerun(process))
                .collect(),
            open_ports: open_ports
                .into_iter()
                .filter(|port| {
                    port.process_name
                        .as_deref()
                        .map(|name| name.contains("edgerun"))
                        .unwrap_or(false)
                })
                .collect(),
        },
        error: None,
    }
}

fn process_is_edgerun(process: &ProcessReport) -> bool {
    process.name.starts_with("edgerun")
        || process
            .exe
            .as_deref()
            .is_some_and(path_basename_starts_with_edgerun)
        || process
            .cmdline
            .first()
            .is_some_and(|arg| path_basename_starts_with_edgerun(arg))
}

fn path_basename_starts_with_edgerun(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|name| name.starts_with("edgerun"))
}

fn gather_edgerun_services() -> Vec<String> {
    let mut services = Vec::new();
    if let Some(output) = command_stdout("systemctl", &["list-unit-files", "--no-legend"]) {
        services.extend(
            output
                .lines()
                .filter_map(|line| line.split_whitespace().next())
                .filter(|name| name.contains("edgerun"))
                .map(str::to_string),
        );
    }
    if let Ok(entries) = std::fs::read_dir("/etc/init.d") {
        services.extend(
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.contains("edgerun"))
                .map(|name| format!("openrc:{name}")),
        );
    }
    services.sort();
    services.dedup();
    services
}

fn gather_processes() -> ReportSection<Vec<ProcessReport>> {
    let entries = match std::fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(error) => {
            return ReportSection {
                items: Vec::new(),
                error: Some(error.to_string()),
            };
        }
    };
    let mut items = Vec::new();
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(pid) = file_name
            .to_str()
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };
        let path = entry.path();
        let status = std::fs::read_to_string(path.join("status")).unwrap_or_default();
        let name = status_field(&status, "Name")
            .or_else(|| read_trimmed(&path.join("comm")))
            .unwrap_or_else(|| pid.to_string());
        let ppid = status_field(&status, "PPid").and_then(|value| value.parse::<u32>().ok());
        let uid = status_field(&status, "Uid")
            .and_then(|value| value.split_whitespace().next().map(str::to_string))
            .and_then(|value| value.parse::<u32>().ok());
        let exe = std::fs::read_link(path.join("exe"))
            .ok()
            .map(|value| value.display().to_string());
        let cmdline = std::fs::read(path.join("cmdline"))
            .ok()
            .map(parse_cmdline)
            .unwrap_or_default();
        items.push(ProcessReport {
            pid,
            ppid,
            uid,
            name,
            vm_size_bytes: status_kb_field(&status, "VmSize"),
            vm_rss_bytes: status_kb_field(&status, "VmRSS"),
            rss_anon_bytes: status_kb_field(&status, "RssAnon"),
            rss_file_bytes: status_kb_field(&status, "RssFile"),
            rss_shmem_bytes: status_kb_field(&status, "RssShmem"),
            vm_swap_bytes: status_kb_field(&status, "VmSwap"),
            pss_bytes: smaps_rollup_kb_field(&path, "Pss"),
            shared_clean_bytes: smaps_rollup_kb_field(&path, "Shared_Clean"),
            shared_dirty_bytes: smaps_rollup_kb_field(&path, "Shared_Dirty"),
            private_clean_bytes: smaps_rollup_kb_field(&path, "Private_Clean"),
            private_dirty_bytes: smaps_rollup_kb_field(&path, "Private_Dirty"),
            exe,
            cmdline,
        });
    }
    items.sort_by_key(|process| process.pid);
    ReportSection { items, error: None }
}

fn status_field(status: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    status
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn status_kb_field(status: &str, key: &str) -> Option<u64> {
    status_field(status, key)
        .and_then(|value| value.split_whitespace().next().map(str::to_string))
        .and_then(|value| value.parse::<u64>().ok())
        .map(|kb| kb.saturating_mul(1024))
}

fn smaps_rollup_kb_field(proc_path: &std::path::Path, key: &str) -> Option<u64> {
    let text = std::fs::read_to_string(proc_path.join("smaps_rollup")).ok()?;
    status_kb_field(&text, key)
}

fn parse_cmdline(bytes: Vec<u8>) -> Vec<String> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .filter_map(|part| String::from_utf8(part.to_vec()).ok())
        .collect()
}

fn gather_open_ports() -> ReportSection<Vec<OpenPortReport>> {
    let processes = gather_processes().items;
    let inode_map = process_socket_inode_map(&processes);
    let mut items = Vec::new();
    for (path, protocol, ipv6) in [
        ("/proc/net/tcp", "tcp", false),
        ("/proc/net/tcp6", "tcp6", true),
        ("/proc/net/udp", "udp", false),
        ("/proc/net/udp6", "udp6", true),
    ] {
        if let Ok(text) = std::fs::read_to_string(path) {
            for line in text.lines().skip(1) {
                if let Some(mut socket) = parse_proc_net_socket_line(line, protocol, ipv6) {
                    if protocol.starts_with("tcp") && socket.state != "LISTEN" {
                        continue;
                    }
                    if protocol.starts_with("udp")
                        && socket.local_address != "0.0.0.0"
                        && socket.local_address != "::"
                        && socket.local_port == 0
                    {
                        continue;
                    }
                    if let Some(inode) = socket.inode {
                        if let Some((pid, name)) =
                            inode_map.iter().find_map(|(candidate, pid, name)| {
                                (*candidate == inode).then(|| (*pid, name.clone()))
                            })
                        {
                            socket.pid = Some(pid);
                            socket.process_name = Some(name);
                        }
                    }
                    items.push(socket);
                }
            }
        }
    }
    items.sort_by(|left, right| {
        (
            &left.protocol,
            left.local_port,
            &left.local_address,
            left.pid,
        )
            .cmp(&(
                &right.protocol,
                right.local_port,
                &right.local_address,
                right.pid,
            ))
    });
    ReportSection { items, error: None }
}

fn process_socket_inode_map(processes: &[ProcessReport]) -> Vec<(u64, u32, String)> {
    let mut out = Vec::new();
    for process in processes {
        let fd_dir = format!("/proc/{}/fd", process.pid);
        let Ok(entries) = std::fs::read_dir(fd_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(target) = std::fs::read_link(entry.path()) else {
                continue;
            };
            let text = target.to_string_lossy();
            if let Some(inode) = text
                .strip_prefix("socket:[")
                .and_then(|value| value.strip_suffix(']'))
                .and_then(|value| value.parse::<u64>().ok())
            {
                out.push((inode, process.pid, process.name.clone()));
            }
        }
    }
    out
}

fn parse_proc_net_socket_line(line: &str, protocol: &str, ipv6: bool) -> Option<OpenPortReport> {
    let fields = line.split_whitespace().collect::<Vec<_>>();
    let local = *fields.get(1)?;
    let remote = *fields.get(2)?;
    let state = *fields.get(3)?;
    let inode = fields.get(9).and_then(|value| value.parse::<u64>().ok());
    let (local_address, local_port) = parse_proc_net_addr(local, ipv6)?;
    let (remote_address, remote_port) = parse_proc_net_addr(remote, ipv6)?;
    Some(OpenPortReport {
        protocol: protocol.to_string(),
        local_address,
        local_port,
        remote_address: (remote_port != 0).then_some(remote_address),
        remote_port: (remote_port != 0).then_some(remote_port),
        state: tcp_state_name(state).to_string(),
        inode,
        pid: None,
        process_name: None,
    })
}

fn parse_proc_net_addr(value: &str, ipv6: bool) -> Option<(String, u16)> {
    let (addr_hex, port_hex) = value.split_once(':')?;
    let port = u16::from_str_radix(port_hex, 16).ok()?;
    let address = if ipv6 {
        parse_ipv6_hex(addr_hex)?.to_string()
    } else {
        parse_ipv4_hex(addr_hex)?.to_string()
    };
    Some((address, port))
}

fn parse_ipv4_hex(value: &str) -> Option<Ipv4Addr> {
    if value.len() != 8 {
        return None;
    }
    let raw = u32::from_str_radix(value, 16).ok()?;
    Some(Ipv4Addr::from(raw.to_le_bytes()))
}

fn parse_ipv6_hex(value: &str) -> Option<Ipv6Addr> {
    if value.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for idx in 0..4 {
        let word = u32::from_str_radix(&value[idx * 8..idx * 8 + 8], 16).ok()?;
        bytes[idx * 4..idx * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    Some(Ipv6Addr::from(bytes))
}

fn tcp_state_name(hex: &str) -> &'static str {
    match hex {
        "01" => "ESTABLISHED",
        "02" => "SYN_SENT",
        "03" => "SYN_RECV",
        "04" => "FIN_WAIT1",
        "05" => "FIN_WAIT2",
        "06" => "TIME_WAIT",
        "07" => "CLOSE",
        "08" => "CLOSE_WAIT",
        "09" => "LAST_ACK",
        "0A" => "LISTEN",
        "0B" => "CLOSING",
        _ => "UNKNOWN",
    }
}

fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn command_output_text(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        if !text.ends_with('\n') && !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    let text = text.trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn gather_service_inventory() -> ReportSection<ServiceInventoryReport> {
    let managers = detect_service_managers();
    ReportSection {
        items: ServiceInventoryReport {
            openrc: managers
                .contains(&ServiceManager::OpenRc)
                .then(gather_openrc_services),
            systemd: managers
                .contains(&ServiceManager::Systemd)
                .then(gather_systemd_services),
            runit: managers
                .contains(&ServiceManager::Runit)
                .then(gather_runit_services),
            s6: managers
                .contains(&ServiceManager::S6)
                .then(gather_s6_services),
            dinit: managers
                .contains(&ServiceManager::Dinit)
                .then(gather_dinit_services),
            supervisor: managers
                .contains(&ServiceManager::Supervisor)
                .then(gather_supervisor_services),
            cron: managers
                .contains(&ServiceManager::Cron)
                .then(gather_cron_services),
            rc_local: Some(gather_rc_local_service()),
        },
        error: None,
    }
}

fn gather_openrc_services() -> OpenRcServiceInventory {
    let mut services = Vec::new();
    let mut error = None;
    let init_dir = std::path::Path::new("/etc/init.d");
    match std::fs::read_dir(init_dir) {
        Ok(entries) => {
            let runlevels = gather_openrc_runlevels();
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let is_executable = entry
                    .metadata()
                    .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false);
                if !is_executable {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') || name.ends_with(".sh") {
                    continue;
                }
                services.push(OpenRcServiceReport {
                    runlevels: runlevels
                        .iter()
                        .filter_map(|(service, runlevel)| {
                            (service == &name).then(|| runlevel.clone())
                        })
                        .collect(),
                    status: command_output_text("rc-service", &[&name, "status"]),
                    path: Some(path.display().to_string()),
                    name,
                });
            }
        }
        Err(err) => error = Some(err.to_string()),
    }
    services.sort_by(|left, right| left.name.cmp(&right.name));
    OpenRcServiceInventory { services, error }
}

fn gather_openrc_runlevels() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let root = std::path::Path::new("/etc/runlevels");
    let Ok(runlevels) = std::fs::read_dir(root) else {
        return out;
    };
    for runlevel in runlevels.flatten() {
        let Ok(file_type) = runlevel.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let runlevel_name = runlevel.file_name().to_string_lossy().into_owned();
        let Ok(services) = std::fs::read_dir(runlevel.path()) else {
            continue;
        };
        for service in services.flatten() {
            out.push((
                service.file_name().to_string_lossy().into_owned(),
                runlevel_name.clone(),
            ));
        }
    }
    out.sort();
    out.dedup();
    out
}

fn gather_systemd_services() -> SystemdServiceInventory {
    let mut units = Vec::new();
    let mut error = None;
    if let Some(output) = command_stdout(
        "systemctl",
        &[
            "list-units",
            "--type=service",
            "--all",
            "--no-legend",
            "--no-pager",
        ],
    ) {
        for line in output.lines() {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() < 4 {
                continue;
            }
            units.push(SystemdServiceReport {
                name: fields[0].to_string(),
                load_state: fields.get(1).map(|value| (*value).to_string()),
                active_state: fields.get(2).map(|value| (*value).to_string()),
                sub_state: fields.get(3).map(|value| (*value).to_string()),
                unit_file_state: None,
                description: (fields.len() > 4).then(|| fields[4..].join(" ")),
            });
        }
    } else {
        error = Some("systemctl list-units failed".to_string());
    }
    if let Some(output) = command_stdout(
        "systemctl",
        &[
            "list-unit-files",
            "--type=service",
            "--no-legend",
            "--no-pager",
        ],
    ) {
        for line in output.lines() {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() < 2 {
                continue;
            }
            if let Some(unit) = units.iter_mut().find(|unit| unit.name == fields[0]) {
                unit.unit_file_state = Some(fields[1].to_string());
            } else {
                units.push(SystemdServiceReport {
                    name: fields[0].to_string(),
                    load_state: None,
                    active_state: None,
                    sub_state: None,
                    unit_file_state: Some(fields[1].to_string()),
                    description: None,
                });
            }
        }
    }
    units.sort_by(|left, right| left.name.cmp(&right.name));
    SystemdServiceInventory { units, error }
}

fn gather_runit_services() -> RunitServiceInventory {
    let mut services = Vec::new();
    for root in ["/etc/service", "/service", "/var/service"] {
        collect_supervised_services(root, "sv", &mut services, |name, path, status| {
            RunitServiceReport { name, path, status }
        });
    }
    services.sort_by(|left, right| left.path.cmp(&right.path));
    services.dedup_by(|left, right| left.path == right.path);
    RunitServiceInventory {
        services,
        error: None,
    }
}

fn gather_s6_services() -> S6ServiceInventory {
    let mut services = Vec::new();
    for root in ["/run/service", "/service", "/etc/s6"] {
        collect_supervised_services(root, "s6-svstat", &mut services, |name, path, status| {
            S6ServiceReport { name, path, status }
        });
    }
    services.sort_by(|left, right| left.path.cmp(&right.path));
    services.dedup_by(|left, right| left.path == right.path);
    S6ServiceInventory {
        services,
        error: None,
    }
}

fn collect_supervised_services<T>(
    root: &str,
    status_program: &str,
    out: &mut Vec<T>,
    mut build: impl FnMut(String, String, Option<String>) -> T,
) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let path_text = path.display().to_string();
        let status = if status_program == "sv" {
            command_output_text(status_program, &["status", &path_text])
        } else {
            command_output_text(status_program, &[&path_text])
        };
        out.push(build(name, path_text, status));
    }
}

fn gather_dinit_services() -> DinitServiceInventory {
    let mut services = Vec::new();
    let mut error = None;
    if let Some(output) = command_stdout("dinitctl", &["list"]) {
        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut fields = trimmed.split_whitespace();
            let Some(name) = fields.next() else {
                continue;
            };
            services.push(DinitServiceReport {
                name: name.to_string(),
                state: Some(fields.collect::<Vec<_>>().join(" ")).filter(|value| !value.is_empty()),
            });
        }
    } else {
        error = Some("dinitctl list failed".to_string());
    }
    DinitServiceInventory { services, error }
}

fn gather_supervisor_services() -> SupervisorServiceInventory {
    let mut programs = Vec::new();
    let mut error = None;
    if let Some(output) = command_stdout("supervisorctl", &["status"]) {
        for line in output.lines() {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.is_empty() {
                continue;
            }
            programs.push(SupervisorProgramReport {
                name: fields[0].to_string(),
                state: fields.get(1).map(|value| (*value).to_string()),
                description: (fields.len() > 2).then(|| fields[2..].join(" ")),
            });
        }
    } else {
        error = Some("supervisorctl status failed".to_string());
    }
    SupervisorServiceInventory { programs, error }
}

fn gather_cron_services() -> CronServiceInventory {
    let mut files = Vec::new();
    collect_cron_file("/etc/crontab", None, &mut files);
    collect_cron_dir("/etc/cron.d", None, &mut files);
    collect_cron_dir("/etc/crontabs", Some("filename"), &mut files);
    collect_cron_dir("/var/spool/cron/crontabs", Some("filename"), &mut files);
    collect_cron_dir("/var/spool/cron", Some("filename"), &mut files);
    files.sort_by(|left, right| left.path.cmp(&right.path));
    files.dedup_by(|left, right| left.path == right.path);
    CronServiceInventory { files, error: None }
}

fn collect_cron_dir(path: &str, user_source: Option<&str>, out: &mut Vec<CronFileReport>) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let user = (user_source == Some("filename")).then(|| {
            entry
                .file_name()
                .to_string_lossy()
                .trim_start_matches('.')
                .to_string()
        });
        collect_cron_file(&path.display().to_string(), user, out);
    }
}

fn collect_cron_file(path: &str, user: Option<String>, out: &mut Vec<CronFileReport>) {
    let text = std::fs::read_to_string(path);
    let (readable, entry_count) = match text {
        Ok(text) => (
            true,
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .count(),
        ),
        Err(_) => (false, 0),
    };
    if readable || std::path::Path::new(path).exists() {
        out.push(CronFileReport {
            path: path.to_string(),
            user,
            readable,
            entry_count,
        });
    }
}

fn gather_rc_local_service() -> RcLocalServiceInventory {
    let path = "/etc/rc.local";
    let metadata = std::fs::metadata(path).ok();
    RcLocalServiceInventory {
        path: path.to_string(),
        exists: metadata.is_some(),
        executable: metadata
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false),
    }
}

fn gather_boot_configuration() -> ReportSection<BootConfigurationReport> {
    ReportSection {
        items: BootConfigurationReport {
            firmware: gather_boot_firmware(),
            system: gather_boot_system(),
            kernel: gather_boot_kernel(),
            boot_entries: gather_boot_entries(),
            config_files: gather_boot_config_files(),
        },
        error: None,
    }
}

fn gather_boot_firmware() -> BootFirmwareReport {
    let mode = if std::path::Path::new("/sys/firmware/efi").exists() {
        BootFirmwareMode::Uefi
    } else if read_trimmed(std::path::Path::new("/sys/class/dmi/id/bios_vendor")).is_some() {
        BootFirmwareMode::Bios
    } else {
        BootFirmwareMode::Unknown
    };
    BootFirmwareReport {
        mode,
        secure_boot: read_secure_boot_state(),
        vendor: read_trimmed(std::path::Path::new("/sys/class/dmi/id/bios_vendor"))
            .or_else(|| dmidecode_string("bios-vendor")),
        version: read_trimmed(std::path::Path::new("/sys/class/dmi/id/bios_version"))
            .or_else(|| dmidecode_string("bios-version")),
        release_date: read_trimmed(std::path::Path::new("/sys/class/dmi/id/bios_date"))
            .or_else(|| dmidecode_string("bios-release-date")),
    }
}

fn read_secure_boot_state() -> Option<bool> {
    let vars = std::fs::read_dir("/sys/firmware/efi/efivars").ok()?;
    for entry in vars.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("SecureBoot-") {
            continue;
        }
        let bytes = std::fs::read(entry.path()).ok()?;
        return bytes.get(4).map(|value| *value == 1);
    }
    None
}

fn gather_boot_system() -> BootSystemReport {
    BootSystemReport {
        manufacturer: read_trimmed(std::path::Path::new("/sys/class/dmi/id/sys_vendor"))
            .or_else(|| dmidecode_string("system-manufacturer")),
        product_name: read_trimmed(std::path::Path::new("/sys/class/dmi/id/product_name"))
            .or_else(|| dmidecode_string("system-product-name")),
        product_version: read_trimmed(std::path::Path::new("/sys/class/dmi/id/product_version"))
            .or_else(|| dmidecode_string("system-version")),
        board_name: read_trimmed(std::path::Path::new("/sys/class/dmi/id/board_name"))
            .or_else(|| dmidecode_string("baseboard-product-name")),
        board_vendor: read_trimmed(std::path::Path::new("/sys/class/dmi/id/board_vendor"))
            .or_else(|| dmidecode_string("baseboard-manufacturer")),
        chassis_type: read_trimmed(std::path::Path::new("/sys/class/dmi/id/chassis_type")),
        virtualization: command_trim("systemd-detect-virt", &[]).or_else(detect_virtualization),
    }
}

fn dmidecode_string(kind: &str) -> Option<String> {
    if !executable_in_path("dmidecode") {
        return None;
    }
    command_trim("dmidecode", &["-s", kind])
}

fn detect_virtualization() -> Option<String> {
    let product =
        read_trimmed(std::path::Path::new("/sys/class/dmi/id/product_name"))?.to_ascii_lowercase();
    if product.contains("kvm") {
        Some("kvm".to_string())
    } else if product.contains("qemu") {
        Some("qemu".to_string())
    } else if product.contains("vmware") {
        Some("vmware".to_string())
    } else if product.contains("virtualbox") {
        Some("virtualbox".to_string())
    } else if product.contains("xen") {
        Some("xen".to_string())
    } else {
        None
    }
}

fn gather_boot_kernel() -> BootKernelReport {
    let cmdline = std::fs::read_to_string("/proc/cmdline")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let mut kernel = BootKernelReport {
        cmdline: cmdline.clone(),
        ..BootKernelReport::default()
    };
    if let Some(cmdline) = cmdline {
        for token in cmdline.split_whitespace() {
            if let Some((key, value)) = token.split_once('=') {
                match key {
                    "init" => kernel.init = Some(value.to_string()),
                    "root" => kernel.root = Some(value.to_string()),
                    "rootfstype" => kernel.rootfstype = Some(value.to_string()),
                    "console" => kernel.console.push(value.to_string()),
                    "initrd" => kernel.initrd.push(value.to_string()),
                    _ => {}
                }
            }
        }
    }
    kernel
}

fn gather_boot_entries() -> Vec<BootEntryReport> {
    let mut entries = Vec::new();
    collect_loader_entries("/boot/loader/entries", &mut entries);
    collect_loader_entries("/efi/loader/entries", &mut entries);
    collect_loader_entries("/boot/efi/loader/entries", &mut entries);
    entries.sort_by(|left, right| (&left.source, &left.id).cmp(&(&right.source, &right.id)));
    entries.dedup_by(|left, right| left.source == right.source && left.id == right.id);
    entries
}

fn collect_loader_entries(path: &str, out: &mut Vec<BootEntryReport>) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("conf") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut report = BootEntryReport {
            source: path.display().to_string(),
            id: entry.file_name().to_string_lossy().into_owned(),
            title: None,
            version: None,
            linux: None,
            initrd: Vec::new(),
            options: None,
        };
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.splitn(2, char::is_whitespace);
            let Some(key) = fields.next() else {
                continue;
            };
            let value = fields.next().unwrap_or("").trim();
            match key {
                "title" => report.title = Some(value.to_string()),
                "version" => report.version = Some(value.to_string()),
                "linux" | "efi" => report.linux = Some(value.to_string()),
                "initrd" => report.initrd.push(value.to_string()),
                "options" => report.options = Some(value.to_string()),
                _ => {}
            }
        }
        out.push(report);
    }
}

fn gather_boot_config_files() -> Vec<BootConfigFileReport> {
    let paths = [
        ("/proc/cmdline", "kernel-cmdline"),
        ("/etc/fstab", "fstab"),
        ("/etc/inittab", "init"),
        ("/boot/grub/grub.cfg", "grub"),
        ("/boot/grub2/grub.cfg", "grub"),
        ("/boot/extlinux.conf", "extlinux"),
        ("/boot/syslinux/syslinux.cfg", "syslinux"),
        ("/boot/loader/loader.conf", "systemd-boot"),
        ("/efi/loader/loader.conf", "systemd-boot"),
        ("/boot/efi/loader/loader.conf", "systemd-boot"),
        ("/etc/default/grub", "grub-defaults"),
        ("/etc/mkinitcpio.conf", "initramfs"),
        ("/etc/dracut.conf", "initramfs"),
        ("/etc/update-extlinux.conf", "extlinux-defaults"),
    ];
    let mut files = paths
        .iter()
        .map(|(path, kind)| boot_config_file(path, kind))
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    files
}

fn boot_config_file(path: &str, kind: &str) -> BootConfigFileReport {
    let metadata = std::fs::metadata(path).ok();
    BootConfigFileReport {
        path: path.to_string(),
        kind: kind.to_string(),
        exists: metadata.is_some(),
        readable: std::fs::File::open(path).is_ok(),
        size_bytes: metadata.as_ref().map(std::fs::Metadata::len),
        modified_unix_ms: metadata
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64),
    }
}

pub fn render_machine_report(report: &MachineReport) -> String {
    render_text(report)
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
        "DNS records",
        report.dns_records.items.len(),
        report.dns_records.error.as_deref(),
    );
    for record in &report.dns_records.items {
        writeln!(
            &mut out,
            "- query={} type={} value={} source={}",
            record.query, record.record_type, record.value, record.source
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

    render_cpu_inventory(&mut out, &report.cpus);
    writeln!(&mut out).unwrap();

    render_memory(&mut out, &report.memory);
    writeln!(&mut out).unwrap();

    render_section_header(
        &mut out,
        "Power supplies",
        report.power_supplies.items.len(),
        report.power_supplies.error.as_deref(),
    );
    for supply in &report.power_supplies.items {
        writeln!(
            &mut out,
            "- {} type={} status={} online={} capacity_percent={} health={} technology={} manufacturer={} model={} serial={} energy_now_uwh={} energy_full_uwh={} charge_now_uah={} charge_full_uah={} power_now_uw={} current_now_ua={} voltage_now_uv={}",
            supply.name,
            supply.kind.as_deref().unwrap_or(""),
            supply.status.as_deref().unwrap_or(""),
            option_bool_text(supply.online),
            supply
                .capacity_percent
                .map(|value| value.to_string())
                .unwrap_or_default(),
            supply.health.as_deref().unwrap_or(""),
            supply.technology.as_deref().unwrap_or(""),
            supply.manufacturer.as_deref().unwrap_or(""),
            supply.model_name.as_deref().unwrap_or(""),
            supply.serial_number.as_deref().unwrap_or(""),
            supply
                .energy_now_uwatt_hours
                .map(|value| value.to_string())
                .unwrap_or_default(),
            supply
                .energy_full_uwatt_hours
                .map(|value| value.to_string())
                .unwrap_or_default(),
            supply
                .charge_now_uamp_hours
                .map(|value| value.to_string())
                .unwrap_or_default(),
            supply
                .charge_full_uamp_hours
                .map(|value| value.to_string())
                .unwrap_or_default(),
            supply
                .power_now_uwatts
                .map(|value| value.to_string())
                .unwrap_or_default(),
            supply
                .current_now_uamps
                .map(|value| value.to_string())
                .unwrap_or_default(),
            supply
                .voltage_now_uvolts
                .map(|value| value.to_string())
                .unwrap_or_default(),
        )
        .unwrap();
    }
    writeln!(&mut out).unwrap();

    render_section_header(
        &mut out,
        "Thermal zones",
        report.thermal_zones.items.len(),
        report.thermal_zones.error.as_deref(),
    );
    for zone in &report.thermal_zones.items {
        writeln!(
            &mut out,
            "- {} type={} temp_millic={} policy={} mode={}",
            zone.name,
            zone.kind.as_deref().unwrap_or(""),
            zone.temp_millidegree_celsius
                .map(|value| value.to_string())
                .unwrap_or_default(),
            zone.policy.as_deref().unwrap_or(""),
            zone.mode.as_deref().unwrap_or("")
        )
        .unwrap();
    }
    writeln!(&mut out).unwrap();

    render_section_header(
        &mut out,
        "Storage mounts",
        report.storage_mounts.items.len(),
        report.storage_mounts.error.as_deref(),
    );
    for mount in &report.storage_mounts.items {
        writeln!(
            &mut out,
            "- source={} target={} fs={} options={}",
            mount.source, mount.target, mount.fs_type, mount.options
        )
        .unwrap();
    }
    writeln!(&mut out).unwrap();

    render_section_header(
        &mut out,
        "Filesystem permissions",
        report.filesystem_permissions.items.len(),
        report.filesystem_permissions.error.as_deref(),
    );
    for permission in &report.filesystem_permissions.items {
        writeln!(
            &mut out,
            "- path={} purpose={} exists={} kind={} fs={} source={} mount={} ro={} noexec={} nosuid={} nodev={} mode={} uid={} gid={} read={} write={} execute={} create={} parent={} parent_mode={} parent_uid={} parent_gid={}",
            permission.path,
            permission.purpose,
            permission.exists,
            permission.kind,
            permission.fs_type.as_deref().unwrap_or(""),
            permission.source.as_deref().unwrap_or(""),
            permission.mount_target.as_deref().unwrap_or(""),
            permission.read_only_mount,
            permission.noexec_mount,
            permission.nosuid_mount,
            permission.nodev_mount,
            permission.mode_octal.as_deref().unwrap_or(""),
            permission.uid.map(|value| value.to_string()).unwrap_or_default(),
            permission.gid.map(|value| value.to_string()).unwrap_or_default(),
            permission.current_user_read,
            permission.current_user_write,
            permission.current_user_execute,
            permission.current_user_can_create_file,
            permission.parent_path.as_deref().unwrap_or(""),
            permission.parent_mode_octal.as_deref().unwrap_or(""),
            permission.parent_uid.map(|value| value.to_string()).unwrap_or_default(),
            permission.parent_gid.map(|value| value.to_string()).unwrap_or_default(),
        )
        .unwrap();
    }
    writeln!(&mut out).unwrap();

    render_section_header(
        &mut out,
        "Block devices",
        report.block_devices.items.len(),
        report.block_devices.error.as_deref(),
    );
    for device in &report.block_devices.items {
        writeln!(
            &mut out,
            "- {} size_bytes={} removable={} read_only={} rotational={} vendor={} model={} wwid={}",
            device.name,
            device
                .size_bytes
                .map(|value| value.to_string())
                .unwrap_or_default(),
            option_bool_text(device.removable),
            option_bool_text(device.read_only),
            option_bool_text(device.rotational),
            device.vendor.as_deref().unwrap_or(""),
            device.model.as_deref().unwrap_or(""),
            device.wwid.as_deref().unwrap_or(""),
        )
        .unwrap();
    }
    writeln!(&mut out).unwrap();

    render_section_header(
        &mut out,
        "Device nodes",
        report.device_nodes.items.len(),
        report.device_nodes.error.as_deref(),
    );
    for node in &report.device_nodes.items {
        writeln!(
            &mut out,
            "- path={} category={} type={} target={} major={} minor={} mode={} uid={} gid={}",
            node.path,
            node.category,
            node.node_type,
            node.symlink_target.as_deref().unwrap_or(""),
            node.major
                .map(|value| value.to_string())
                .unwrap_or_default(),
            node.minor
                .map(|value| value.to_string())
                .unwrap_or_default(),
            node.mode_octal,
            node.uid,
            node.gid
        )
        .unwrap();
    }
    writeln!(&mut out).unwrap();

    render_boot_configuration(&mut out, &report.boot);
    writeln!(&mut out).unwrap();

    render_runtime_pressure(&mut out, &report.runtime);
    writeln!(&mut out).unwrap();

    render_kernel_capabilities(&mut out, &report.kernel);
    writeln!(&mut out).unwrap();

    render_network_routes(&mut out, &report.network_routes);
    writeln!(&mut out).unwrap();

    render_security_access(&mut out, &report.security);
    writeln!(&mut out).unwrap();

    render_package_surface(&mut out, &report.packages);
    writeln!(&mut out).unwrap();

    render_edgerun_conflicts(&mut out, &report.edgerun_conflicts);
    writeln!(&mut out).unwrap();

    render_section_header(
        &mut out,
        "Open ports",
        report.open_ports.items.len(),
        report.open_ports.error.as_deref(),
    );
    for port in &report.open_ports.items {
        writeln!(
            &mut out,
            "- {} {}:{} state={} pid={} process={} remote={}:{} inode={}",
            port.protocol,
            port.local_address,
            port.local_port,
            port.state,
            port.pid.map(|value| value.to_string()).unwrap_or_default(),
            port.process_name.as_deref().unwrap_or(""),
            port.remote_address.as_deref().unwrap_or(""),
            port.remote_port
                .map(|value| value.to_string())
                .unwrap_or_default(),
            port.inode
                .map(|value| value.to_string())
                .unwrap_or_default(),
        )
        .unwrap();
    }
    writeln!(&mut out).unwrap();

    render_service_inventory(&mut out, &report.services);
    writeln!(&mut out).unwrap();

    render_section_header(
        &mut out,
        "Processes",
        report.processes.items.len(),
        report.processes.error.as_deref(),
    );
    for process in &report.processes.items {
        writeln!(
            &mut out,
            "- pid={} ppid={} uid={} name={} vm_size={} vm_rss={} rss_anon={} rss_file={} rss_shmem={} vm_swap={} pss={} shared_clean={} shared_dirty={} private_clean={} private_dirty={} exe={} cmdline={}",
            process.pid,
            process
                .ppid
                .map(|value| value.to_string())
                .unwrap_or_default(),
            process
                .uid
                .map(|value| value.to_string())
                .unwrap_or_default(),
            process.name,
            option_u64_text(process.vm_size_bytes),
            option_u64_text(process.vm_rss_bytes),
            option_u64_text(process.rss_anon_bytes),
            option_u64_text(process.rss_file_bytes),
            option_u64_text(process.rss_shmem_bytes),
            option_u64_text(process.vm_swap_bytes),
            option_u64_text(process.pss_bytes),
            option_u64_text(process.shared_clean_bytes),
            option_u64_text(process.shared_dirty_bytes),
            option_u64_text(process.private_clean_bytes),
            option_u64_text(process.private_dirty_bytes),
            process.exe.as_deref().unwrap_or(""),
            process.cmdline.join(" ").replace(['\r', '\n', '\t'], " "),
        )
        .unwrap();
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

fn render_cpu_inventory(out: &mut String, section: &ReportSection<CpuInventoryReport>) {
    render_section_header(
        out,
        "CPUs",
        section.items.processors.len(),
        section.error.as_deref(),
    );
    writeln!(
        out,
        "- summary arch={} logical={} packages={} cores={} vendor={} model={} microcode={} flags={}",
        section.items.architecture,
        section.items.logical_processors,
        section.items.physical_packages,
        section.items.physical_cores,
        section.items.vendor_id.as_deref().unwrap_or(""),
        section.items.model_name.as_deref().unwrap_or(""),
        section.items.microcode.as_deref().unwrap_or(""),
        section.items.flags.join(",")
    )
    .unwrap();
    for processor in &section.items.processors {
        writeln!(
            out,
            "- cpu{} vendor={} model={} package={} core={} siblings={} cpu_cores={} mhz_khz={} cache_kb={} microcode={} flags={}",
            processor.processor,
            processor.vendor_id.as_deref().unwrap_or(""),
            processor.model_name.as_deref().unwrap_or(""),
            processor
                .physical_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
            processor
                .core_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
            processor
                .siblings
                .map(|value| value.to_string())
                .unwrap_or_default(),
            processor
                .cpu_cores
                .map(|value| value.to_string())
                .unwrap_or_default(),
            processor
                .cpu_mhz_khz
                .map(|value| value.to_string())
                .unwrap_or_default(),
            processor
                .cache_size_kb
                .map(|value| value.to_string())
                .unwrap_or_default(),
            processor.microcode.as_deref().unwrap_or(""),
            processor.flags.join(",")
        )
        .unwrap();
    }
    for policy in &section.items.frequency_policies {
        writeln!(
            out,
            "- cpufreq policy={} affected_cpus={} driver={} governor={} scaling_min_khz={} scaling_max_khz={} cpuinfo_min_khz={} cpuinfo_max_khz={}",
            policy.policy,
            policy.affected_cpus.as_deref().unwrap_or(""),
            policy.scaling_driver.as_deref().unwrap_or(""),
            policy.scaling_governor.as_deref().unwrap_or(""),
            policy
                .scaling_min_freq_khz
                .map(|value| value.to_string())
                .unwrap_or_default(),
            policy
                .scaling_max_freq_khz
                .map(|value| value.to_string())
                .unwrap_or_default(),
            policy
                .cpuinfo_min_freq_khz
                .map(|value| value.to_string())
                .unwrap_or_default(),
            policy
                .cpuinfo_max_freq_khz
                .map(|value| value.to_string())
                .unwrap_or_default()
        )
        .unwrap();
    }
}

fn render_memory(out: &mut String, section: &ReportSection<MemoryReport>) {
    render_section_header(out, "Memory", 1, section.error.as_deref());
    let memory = &section.items;
    writeln!(
        out,
        "- mem_total={} mem_free={} mem_available={} buffers={} cached={} swap_cached={} active={} inactive={} active_file={} inactive_file={} active_anon={} inactive_anon={} slab={} sreclaimable={} sunreclaim={} kernel_stack={} page_tables={} percpu={} vmalloc_used={} dirty={} writeback={} shmem={} swap_total={} swap_free={} huge_pages_total={} huge_pages_free={} huge_page_size={} direct_map_4k={} direct_map_2m={} direct_map_1g={}",
        option_u64_text(memory.mem_total_bytes),
        option_u64_text(memory.mem_free_bytes),
        option_u64_text(memory.mem_available_bytes),
        option_u64_text(memory.buffers_bytes),
        option_u64_text(memory.cached_bytes),
        option_u64_text(memory.swap_cached_bytes),
        option_u64_text(memory.active_bytes),
        option_u64_text(memory.inactive_bytes),
        option_u64_text(memory.active_file_bytes),
        option_u64_text(memory.inactive_file_bytes),
        option_u64_text(memory.active_anon_bytes),
        option_u64_text(memory.inactive_anon_bytes),
        option_u64_text(memory.slab_bytes),
        option_u64_text(memory.reclaimable_slab_bytes),
        option_u64_text(memory.unreclaimable_slab_bytes),
        option_u64_text(memory.kernel_stack_bytes),
        option_u64_text(memory.page_tables_bytes),
        option_u64_text(memory.percpu_bytes),
        option_u64_text(memory.vmalloc_used_bytes),
        option_u64_text(memory.dirty_bytes),
        option_u64_text(memory.writeback_bytes),
        option_u64_text(memory.shmem_bytes),
        option_u64_text(memory.swap_total_bytes),
        option_u64_text(memory.swap_free_bytes),
        option_u64_text(memory.huge_pages_total),
        option_u64_text(memory.huge_pages_free),
        option_u64_text(memory.huge_page_size_bytes),
        option_u64_text(memory.direct_map_4k_bytes),
        option_u64_text(memory.direct_map_2m_bytes),
        option_u64_text(memory.direct_map_1g_bytes)
    )
    .unwrap();
}

fn render_runtime_pressure(out: &mut String, section: &ReportSection<RuntimePressureReport>) {
    render_section_header(
        out,
        "Runtime pressure",
        1 + section.items.disk_usage.len(),
        section.error.as_deref(),
    );
    let runtime = &section.items;
    writeln!(
        out,
        "- uptime_seconds={} idle_seconds={} load_1m={} load_5m={} load_15m={} runnable_tasks={} total_tasks={} last_pid={}",
        option_u64_text(runtime.uptime_seconds),
        option_u64_text(runtime.idle_seconds),
        runtime.load_1m.as_deref().unwrap_or(""),
        runtime.load_5m.as_deref().unwrap_or(""),
        runtime.load_15m.as_deref().unwrap_or(""),
        option_u64_text(runtime.runnable_tasks),
        option_u64_text(runtime.total_tasks),
        option_u64_text(runtime.last_pid)
    )
    .unwrap();
    for disk in &runtime.disk_usage {
        writeln!(
            out,
            "- disk target={} fs={} source={} total={} used={} available={} capacity_percent={} inodes_total={} inodes_used={} inodes_available={} inode_capacity_percent={}",
            disk.target,
            disk.fs_type.as_deref().unwrap_or(""),
            disk.source.as_deref().unwrap_or(""),
            option_u64_text(disk.total_bytes),
            option_u64_text(disk.used_bytes),
            option_u64_text(disk.available_bytes),
            disk.capacity_percent.map(|value| value.to_string()).unwrap_or_default(),
            option_u64_text(disk.total_inodes),
            option_u64_text(disk.used_inodes),
            option_u64_text(disk.available_inodes),
            disk.inode_capacity_percent.map(|value| value.to_string()).unwrap_or_default(),
        )
        .unwrap();
    }
}

fn render_kernel_capabilities(out: &mut String, section: &ReportSection<KernelCapabilityReport>) {
    let kernel = &section.items;
    render_section_header(
        out,
        "Kernel capabilities",
        kernel.loaded_modules.len() + kernel.filesystems.len() + kernel.selected_sysctls.len() + 1,
        section.error.as_deref(),
    );
    writeln!(
        out,
        "- namespaces={} cgroups={} lsm={} apparmor={} selinux={} seccomp={} bpf_jit={}",
        kernel.namespaces.join(","),
        kernel.cgroup_controllers.join(","),
        kernel.lsm.join(","),
        option_bool_text(kernel.apparmor_enabled),
        option_bool_text(kernel.selinux_enabled),
        option_bool_text(kernel.seccomp_available),
        option_bool_text(kernel.bpf_jit_enabled),
    )
    .unwrap();
    writeln!(
        out,
        "- kexec syscall={} load_disabled={} lockdown={} tool={} version={} proc_loaded_image={} crash_loaded={} crash_size={}",
        option_bool_text(kernel.kexec.syscall_present),
        option_bool_text(kernel.kexec.load_disabled),
        kernel.kexec.lockdown.as_deref().unwrap_or(""),
        kernel.kexec.tool_present,
        kernel.kexec.tool_version.as_deref().unwrap_or(""),
        kernel.kexec.proc_loaded_image_present,
        kernel.kexec.sysfs_crash_loaded_present,
        kernel.kexec.sysfs_crash_size_present,
    )
    .unwrap();
    for sysctl in &kernel.selected_sysctls {
        writeln!(
            out,
            "- sysctl {}={}",
            sysctl.name,
            sysctl.value.as_deref().unwrap_or("")
        )
        .unwrap();
    }
    for filesystem in &kernel.filesystems {
        writeln!(
            out,
            "- filesystem name={} nodev={}",
            filesystem.name, filesystem.nodev
        )
        .unwrap();
    }
    for module in &kernel.loaded_modules {
        writeln!(
            out,
            "- module name={} size={} refs={} deps={} state={}",
            module.name,
            option_u64_text(module.size_bytes),
            option_u64_text(module.ref_count),
            module.dependencies.join(","),
            module.state.as_deref().unwrap_or("")
        )
        .unwrap();
    }
}

fn render_network_routes(out: &mut String, section: &ReportSection<NetworkRouteReport>) {
    let routes = &section.items;
    render_section_header(
        out,
        "Network routes",
        routes.resolv_conf.len() + routes.ipv4_routes.len() + routes.ipv6_routes.len(),
        section.error.as_deref(),
    );
    writeln!(
        out,
        "- host hostname={} fqdn={}",
        routes.hostname.as_deref().unwrap_or(""),
        routes.fqdn.as_deref().unwrap_or("")
    )
    .unwrap();
    for resolver in &routes.resolv_conf {
        writeln!(out, "- resolver {}={}", resolver.kind, resolver.value).unwrap();
    }
    for route in &routes.ipv4_routes {
        writeln!(
            out,
            "- ipv4 iface={} dest={} gateway={} mask={} flags={} metric={}",
            route.interface,
            route.destination,
            route.gateway,
            route.mask,
            route.flags,
            route
                .metric
                .map(|value| value.to_string())
                .unwrap_or_default()
        )
        .unwrap();
    }
    for route in &routes.ipv6_routes {
        writeln!(
            out,
            "- ipv6 iface={} dest={}/{} next_hop={} metric={}",
            route.interface,
            route.destination,
            route.prefix_len,
            route.next_hop,
            route
                .metric
                .map(|value| value.to_string())
                .unwrap_or_default()
        )
        .unwrap();
    }
}

fn render_security_access(out: &mut String, section: &ReportSection<SecurityAccessReport>) {
    let security = &section.items;
    render_section_header(
        out,
        "Security access",
        security.users.len()
            + security.groups.len()
            + security.sudoers.len()
            + security.authorized_keys.len(),
        section.error.as_deref(),
    );
    writeln!(
        out,
        "- firewall nft_present={} nft_ruleset={} nft_rules={} iptables_present={} iptables_rules={} ip6tables_present={} ip6tables_rules={}",
        security.firewall.nft_present,
        security.firewall.nft_ruleset_available,
        security.firewall.nft_rule_count.map(|value| value.to_string()).unwrap_or_default(),
        security.firewall.iptables_present,
        security.firewall.iptables_rule_count.map(|value| value.to_string()).unwrap_or_default(),
        security.firewall.ip6tables_present,
        security.firewall.ip6tables_rule_count.map(|value| value.to_string()).unwrap_or_default(),
    )
    .unwrap();
    for file in security
        .sudoers
        .iter()
        .chain(security.doas_conf.iter())
        .chain(security.sshd_config.iter())
    {
        writeln!(
            out,
            "- policy path={} exists={} readable={} mode={} uid={} gid={} lines={}",
            file.path,
            file.exists,
            file.readable,
            file.mode_octal.as_deref().unwrap_or(""),
            file.uid.map(|value| value.to_string()).unwrap_or_default(),
            file.gid.map(|value| value.to_string()).unwrap_or_default(),
            file.line_count
                .map(|value| value.to_string())
                .unwrap_or_default(),
        )
        .unwrap();
    }
    for user in &security.users {
        writeln!(
            out,
            "- user name={} uid={} gid={} home={} shell={} system={} privileged={}",
            user.name, user.uid, user.gid, user.home, user.shell, user.system, user.privileged
        )
        .unwrap();
    }
    for group in &security.groups {
        writeln!(
            out,
            "- group name={} gid={} members={} privileged={}",
            group.name,
            group.gid,
            group.members.join(","),
            group.privileged
        )
        .unwrap();
    }
    for keys in &security.authorized_keys {
        writeln!(
            out,
            "- authorized_keys user={} path={} exists={} readable={} keys={} mode={}",
            keys.user,
            keys.path,
            keys.exists,
            keys.readable,
            keys.key_count,
            keys.mode_octal.as_deref().unwrap_or("")
        )
        .unwrap();
    }
}

fn render_package_surface(out: &mut String, section: &ReportSection<PackageSurfaceReport>) {
    let packages = &section.items;
    render_section_header(
        out,
        "Package surface",
        packages.managers.len() + packages.core_binaries.len(),
        section.error.as_deref(),
    );
    for manager in &packages.managers {
        writeln!(
            out,
            "- manager name={} present={} installed_count={}",
            manager.name,
            manager.present,
            manager
                .installed_count
                .map(|value| value.to_string())
                .unwrap_or_default()
        )
        .unwrap();
    }
    for binary in &packages.core_binaries {
        writeln!(
            out,
            "- binary name={} present={} version={}",
            binary.name,
            binary.path_present,
            binary
                .version
                .as_deref()
                .unwrap_or("")
                .replace(['\r', '\n', '\t'], " ")
        )
        .unwrap();
    }
}

fn render_edgerun_conflicts(out: &mut String, section: &ReportSection<EdgerunConflictReport>) {
    let conflicts = &section.items;
    render_section_header(
        out,
        "Edgerun conflicts",
        conflicts.paths.len()
            + conflicts.services.len()
            + conflicts.processes.len()
            + conflicts.open_ports.len(),
        section.error.as_deref(),
    );
    for path in &conflicts.paths {
        writeln!(
            out,
            "- path path={} exists={} readable={} mode={} uid={} gid={} lines={}",
            path.path,
            path.exists,
            path.readable,
            path.mode_octal.as_deref().unwrap_or(""),
            path.uid.map(|value| value.to_string()).unwrap_or_default(),
            path.gid.map(|value| value.to_string()).unwrap_or_default(),
            path.line_count
                .map(|value| value.to_string())
                .unwrap_or_default()
        )
        .unwrap();
    }
    for service in &conflicts.services {
        writeln!(out, "- service {service}").unwrap();
    }
    for process in &conflicts.processes {
        writeln!(
            out,
            "- process pid={} name={} vm_rss={} pss={} exe={} cmdline={}",
            process.pid,
            process.name,
            option_u64_text(process.vm_rss_bytes),
            option_u64_text(process.pss_bytes),
            process.exe.as_deref().unwrap_or(""),
            process.cmdline.join(" ").replace(['\r', '\n', '\t'], " ")
        )
        .unwrap();
    }
    for port in &conflicts.open_ports {
        writeln!(
            out,
            "- port {} {}:{} process={} pid={}",
            port.protocol,
            port.local_address,
            port.local_port,
            port.process_name.as_deref().unwrap_or(""),
            port.pid.map(|value| value.to_string()).unwrap_or_default()
        )
        .unwrap();
    }
}

fn render_boot_configuration(out: &mut String, section: &ReportSection<BootConfigurationReport>) {
    render_section_header(
        out,
        "Boot configuration",
        boot_configuration_count(&section.items),
        section.error.as_deref(),
    );
    let boot = &section.items;
    writeln!(
        out,
        "- firmware mode={} secure_boot={} vendor={} version={} date={}",
        boot.firmware.mode.as_str(),
        option_bool_text(boot.firmware.secure_boot),
        boot.firmware.vendor.as_deref().unwrap_or(""),
        boot.firmware.version.as_deref().unwrap_or(""),
        boot.firmware.release_date.as_deref().unwrap_or("")
    )
    .unwrap();
    writeln!(
        out,
        "- system manufacturer={} product={} version={} board_vendor={} board={} chassis={} virt={}",
        boot.system.manufacturer.as_deref().unwrap_or(""),
        boot.system.product_name.as_deref().unwrap_or(""),
        boot.system.product_version.as_deref().unwrap_or(""),
        boot.system.board_vendor.as_deref().unwrap_or(""),
        boot.system.board_name.as_deref().unwrap_or(""),
        boot.system.chassis_type.as_deref().unwrap_or(""),
        boot.system.virtualization.as_deref().unwrap_or("")
    )
    .unwrap();
    writeln!(
        out,
        "- kernel root={} rootfstype={} init={} console={} initrd={} cmdline={}",
        boot.kernel.root.as_deref().unwrap_or(""),
        boot.kernel.rootfstype.as_deref().unwrap_or(""),
        boot.kernel.init.as_deref().unwrap_or(""),
        boot.kernel.console.join(","),
        boot.kernel.initrd.join(","),
        boot.kernel.cmdline.as_deref().unwrap_or("")
    )
    .unwrap();
    for entry in &boot.boot_entries {
        writeln!(
            out,
            "- boot_entry source={} id={} title={} version={} linux={} initrd={} options={}",
            entry.source,
            entry.id,
            entry.title.as_deref().unwrap_or(""),
            entry.version.as_deref().unwrap_or(""),
            entry.linux.as_deref().unwrap_or(""),
            entry.initrd.join(","),
            entry.options.as_deref().unwrap_or("")
        )
        .unwrap();
    }
    for file in &boot.config_files {
        if !file.exists {
            continue;
        }
        writeln!(
            out,
            "- boot_config path={} kind={} readable={} size_bytes={} modified_unix_ms={}",
            file.path,
            file.kind,
            file.readable,
            file.size_bytes
                .map(|value| value.to_string())
                .unwrap_or_default(),
            file.modified_unix_ms
                .map(|value| value.to_string())
                .unwrap_or_default()
        )
        .unwrap();
    }
}

fn boot_configuration_count(report: &BootConfigurationReport) -> usize {
    3 + report.boot_entries.len()
        + report
            .config_files
            .iter()
            .filter(|file| file.exists)
            .count()
}

fn render_service_inventory(out: &mut String, section: &ReportSection<ServiceInventoryReport>) {
    let count = service_inventory_count(&section.items);
    render_section_header(out, "Services", count, section.error.as_deref());
    if let Some(openrc) = &section.items.openrc {
        writeln!(
            out,
            "  openrc: {} services{}",
            openrc.services.len(),
            service_error_suffix(openrc.error.as_deref())
        )
        .unwrap();
        for service in &openrc.services {
            writeln!(
                out,
                "  - openrc name={} status={} runlevels={} path={}",
                service.name,
                service.status.as_deref().unwrap_or(""),
                service.runlevels.join(","),
                service.path.as_deref().unwrap_or("")
            )
            .unwrap();
        }
    }
    if let Some(systemd) = &section.items.systemd {
        writeln!(
            out,
            "  systemd: {} units{}",
            systemd.units.len(),
            service_error_suffix(systemd.error.as_deref())
        )
        .unwrap();
        for unit in &systemd.units {
            writeln!(
                out,
                "  - systemd name={} load={} active={} sub={} file_state={} description={}",
                unit.name,
                unit.load_state.as_deref().unwrap_or(""),
                unit.active_state.as_deref().unwrap_or(""),
                unit.sub_state.as_deref().unwrap_or(""),
                unit.unit_file_state.as_deref().unwrap_or(""),
                unit.description.as_deref().unwrap_or("")
            )
            .unwrap();
        }
    }
    if let Some(runit) = &section.items.runit {
        writeln!(
            out,
            "  runit: {} services{}",
            runit.services.len(),
            service_error_suffix(runit.error.as_deref())
        )
        .unwrap();
        for service in &runit.services {
            writeln!(
                out,
                "  - runit name={} status={} path={}",
                service.name,
                service.status.as_deref().unwrap_or(""),
                service.path
            )
            .unwrap();
        }
    }
    if let Some(s6) = &section.items.s6 {
        writeln!(
            out,
            "  s6: {} services{}",
            s6.services.len(),
            service_error_suffix(s6.error.as_deref())
        )
        .unwrap();
        for service in &s6.services {
            writeln!(
                out,
                "  - s6 name={} status={} path={}",
                service.name,
                service.status.as_deref().unwrap_or(""),
                service.path
            )
            .unwrap();
        }
    }
    if let Some(dinit) = &section.items.dinit {
        writeln!(
            out,
            "  dinit: {} services{}",
            dinit.services.len(),
            service_error_suffix(dinit.error.as_deref())
        )
        .unwrap();
        for service in &dinit.services {
            writeln!(
                out,
                "  - dinit name={} state={}",
                service.name,
                service.state.as_deref().unwrap_or("")
            )
            .unwrap();
        }
    }
    if let Some(supervisor) = &section.items.supervisor {
        writeln!(
            out,
            "  supervisor: {} programs{}",
            supervisor.programs.len(),
            service_error_suffix(supervisor.error.as_deref())
        )
        .unwrap();
        for program in &supervisor.programs {
            writeln!(
                out,
                "  - supervisor name={} state={} description={}",
                program.name,
                program.state.as_deref().unwrap_or(""),
                program.description.as_deref().unwrap_or("")
            )
            .unwrap();
        }
    }
    if let Some(cron) = &section.items.cron {
        writeln!(
            out,
            "  cron: {} files{}",
            cron.files.len(),
            service_error_suffix(cron.error.as_deref())
        )
        .unwrap();
        for file in &cron.files {
            writeln!(
                out,
                "  - cron path={} user={} readable={} entries={}",
                file.path,
                file.user.as_deref().unwrap_or(""),
                file.readable,
                file.entry_count
            )
            .unwrap();
        }
    }
    if let Some(rc_local) = &section.items.rc_local {
        writeln!(
            out,
            "  rc.local: path={} exists={} executable={}",
            rc_local.path, rc_local.exists, rc_local.executable
        )
        .unwrap();
    }
}

fn service_error_suffix(error: Option<&str>) -> String {
    error
        .map(|error| format!(" (error: {error})"))
        .unwrap_or_default()
}

fn service_inventory_count(report: &ServiceInventoryReport) -> usize {
    report
        .openrc
        .as_ref()
        .map(|value| value.services.len())
        .unwrap_or(0)
        + report
            .systemd
            .as_ref()
            .map(|value| value.units.len())
            .unwrap_or(0)
        + report
            .runit
            .as_ref()
            .map(|value| value.services.len())
            .unwrap_or(0)
        + report
            .s6
            .as_ref()
            .map(|value| value.services.len())
            .unwrap_or(0)
        + report
            .dinit
            .as_ref()
            .map(|value| value.services.len())
            .unwrap_or(0)
        + report
            .supervisor
            .as_ref()
            .map(|value| value.programs.len())
            .unwrap_or(0)
        + report
            .cron
            .as_ref()
            .map(|value| value.files.len())
            .unwrap_or(0)
        + usize::from(report.rc_local.as_ref().is_some_and(|value| value.exists))
}

fn option_bool_text(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "",
    }
}

fn option_u64_text(value: Option<u64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

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

    fn empty_services() -> ReportSection<ServiceInventoryReport> {
        ReportSection {
            items: ServiceInventoryReport::default(),
            error: None,
        }
    }

    fn empty_boot() -> ReportSection<BootConfigurationReport> {
        ReportSection {
            items: BootConfigurationReport::default(),
            error: None,
        }
    }

    fn empty_cpus() -> ReportSection<CpuInventoryReport> {
        ReportSection {
            items: CpuInventoryReport::default(),
            error: None,
        }
    }

    fn empty_memory() -> ReportSection<MemoryReport> {
        ReportSection {
            items: MemoryReport::default(),
            error: None,
        }
    }

    fn empty_power_supplies() -> ReportSection<Vec<PowerSupplyReport>> {
        ReportSection {
            items: Vec::new(),
            error: None,
        }
    }

    fn empty_thermal_zones() -> ReportSection<Vec<ThermalZoneReport>> {
        ReportSection {
            items: Vec::new(),
            error: None,
        }
    }

    fn empty_device_nodes() -> ReportSection<Vec<DeviceNodeReport>> {
        ReportSection {
            items: Vec::new(),
            error: None,
        }
    }

    fn empty_filesystem_permissions() -> ReportSection<Vec<FilesystemPermissionReport>> {
        ReportSection {
            items: Vec::new(),
            error: None,
        }
    }

    fn empty_runtime() -> ReportSection<RuntimePressureReport> {
        ReportSection {
            items: RuntimePressureReport::default(),
            error: None,
        }
    }

    fn empty_kernel() -> ReportSection<KernelCapabilityReport> {
        ReportSection {
            items: KernelCapabilityReport::default(),
            error: None,
        }
    }

    fn empty_network_routes() -> ReportSection<NetworkRouteReport> {
        ReportSection {
            items: NetworkRouteReport::default(),
            error: None,
        }
    }

    fn empty_security() -> ReportSection<SecurityAccessReport> {
        ReportSection {
            items: SecurityAccessReport::default(),
            error: None,
        }
    }

    fn empty_packages() -> ReportSection<PackageSurfaceReport> {
        ReportSection {
            items: PackageSurfaceReport::default(),
            error: None,
        }
    }

    fn empty_edgerun_conflicts() -> ReportSection<EdgerunConflictReport> {
        ReportSection {
            items: EdgerunConflictReport::default(),
            error: None,
        }
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
            cpus: empty_cpus(),
            memory: empty_memory(),
            power_supplies: empty_power_supplies(),
            thermal_zones: empty_thermal_zones(),
            storage_mounts: ReportSection {
                items: Vec::new(),
                error: None,
            },
            filesystem_permissions: empty_filesystem_permissions(),
            block_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            device_nodes: empty_device_nodes(),
            dns_records: ReportSection {
                items: Vec::new(),
                error: None,
            },
            runtime: empty_runtime(),
            kernel: empty_kernel(),
            network_routes: empty_network_routes(),
            security: empty_security(),
            packages: empty_packages(),
            edgerun_conflicts: empty_edgerun_conflicts(),
            processes: ReportSection {
                items: Vec::new(),
                error: None,
            },
            open_ports: ReportSection {
                items: Vec::new(),
                error: None,
            },
            services: empty_services(),
            boot: empty_boot(),
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
            cpus: empty_cpus(),
            memory: empty_memory(),
            power_supplies: empty_power_supplies(),
            thermal_zones: empty_thermal_zones(),
            storage_mounts: ReportSection {
                items: Vec::new(),
                error: None,
            },
            filesystem_permissions: empty_filesystem_permissions(),
            block_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            device_nodes: empty_device_nodes(),
            dns_records: ReportSection {
                items: Vec::new(),
                error: None,
            },
            runtime: empty_runtime(),
            kernel: empty_kernel(),
            network_routes: empty_network_routes(),
            security: empty_security(),
            packages: empty_packages(),
            edgerun_conflicts: empty_edgerun_conflicts(),
            processes: ReportSection {
                items: Vec::new(),
                error: None,
            },
            open_ports: ReportSection {
                items: Vec::new(),
                error: None,
            },
            services: empty_services(),
            boot: empty_boot(),
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
            cpus: empty_cpus(),
            memory: empty_memory(),
            power_supplies: empty_power_supplies(),
            thermal_zones: empty_thermal_zones(),
            storage_mounts: ReportSection {
                items: Vec::new(),
                error: None,
            },
            filesystem_permissions: empty_filesystem_permissions(),
            block_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            device_nodes: empty_device_nodes(),
            dns_records: ReportSection {
                items: Vec::new(),
                error: None,
            },
            runtime: empty_runtime(),
            kernel: empty_kernel(),
            network_routes: empty_network_routes(),
            security: empty_security(),
            packages: empty_packages(),
            edgerun_conflicts: empty_edgerun_conflicts(),
            processes: ReportSection {
                items: Vec::new(),
                error: None,
            },
            open_ports: ReportSection {
                items: Vec::new(),
                error: None,
            },
            services: empty_services(),
            boot: empty_boot(),
        };
        let text = render_machine_report(&report);
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
            cpus: empty_cpus(),
            memory: empty_memory(),
            power_supplies: empty_power_supplies(),
            thermal_zones: empty_thermal_zones(),
            storage_mounts: ReportSection {
                items: Vec::new(),
                error: None,
            },
            filesystem_permissions: empty_filesystem_permissions(),
            block_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            device_nodes: empty_device_nodes(),
            dns_records: ReportSection {
                items: Vec::new(),
                error: None,
            },
            runtime: empty_runtime(),
            kernel: empty_kernel(),
            network_routes: empty_network_routes(),
            security: empty_security(),
            packages: empty_packages(),
            edgerun_conflicts: empty_edgerun_conflicts(),
            processes: ReportSection {
                items: Vec::new(),
                error: None,
            },
            open_ports: ReportSection {
                items: Vec::new(),
                error: None,
            },
            services: empty_services(),
            boot: empty_boot(),
        };
        let text = render_machine_report(&report);
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
            cpus: empty_cpus(),
            memory: empty_memory(),
            power_supplies: empty_power_supplies(),
            thermal_zones: empty_thermal_zones(),
            storage_mounts: ReportSection {
                items: Vec::new(),
                error: None,
            },
            filesystem_permissions: empty_filesystem_permissions(),
            block_devices: ReportSection {
                items: Vec::new(),
                error: None,
            },
            device_nodes: empty_device_nodes(),
            dns_records: ReportSection {
                items: Vec::new(),
                error: None,
            },
            runtime: empty_runtime(),
            kernel: empty_kernel(),
            network_routes: empty_network_routes(),
            security: empty_security(),
            packages: empty_packages(),
            edgerun_conflicts: empty_edgerun_conflicts(),
            processes: ReportSection {
                items: Vec::new(),
                error: None,
            },
            open_ports: ReportSection {
                items: Vec::new(),
                error: None,
            },
            services: empty_services(),
            boot: empty_boot(),
        };
        let text = render_machine_report(&report);
        assert!(text.contains("generated_at_unix_ms: 1234567890"));
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
    fn gather_machine_report_returns_report() {
        // This will read real sysfs data. Just verify it returns a valid report.
        let report = gather_machine_report();
        // generated_at_unix_ms should be recent
        assert!(report.generated_at_unix_ms > 1_700_000_000_000);
    }

    #[test]
    fn gather_machine_report_to_text() {
        let report = gather_machine_report();
        let text = render_machine_report(&report);
        assert!(text.contains("edgerun machine report"));
        assert!(text.contains("generated_at_unix_ms:"));
    }
}
