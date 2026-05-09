#![no_std]

extern crate alloc;
#[cfg(target_os = "linux")]
extern crate std;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceManager {
    Systemd,
    OpenRc,
    Runit,
    S6,
    Dinit,
    Supervisor,
    Cron,
    RcLocal,
}

impl ServiceManager {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Systemd => "systemd",
            Self::OpenRc => "openrc",
            Self::Runit => "runit",
            Self::S6 => "s6",
            Self::Dinit => "dinit",
            Self::Supervisor => "supervisor",
            Self::Cron => "cron",
            Self::RcLocal => "rc.local",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerRuntime {
    Docker,
    Podman,
    Containerd,
}

impl ContainerRuntime {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
            Self::Containerd => "containerd",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentMachineInventory {
    pub os: String,
    pub arch: String,
    pub kernel: String,
    pub libc: Option<String>,
    pub pid1: Option<String>,
    pub uid: Option<u32>,
    pub is_root: bool,
    pub has_sudo: bool,
    pub has_doas: bool,
    pub writable_paths: Vec<String>,
    pub service_managers: Vec<ServiceManager>,
    pub container_runtimes: Vec<ContainerRuntime>,
    pub low_port_bind_allowed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeploymentMode {
    Foreground,
    UserService(ServiceManager),
    SystemService(ServiceManager),
    InitReplacement,
    Container(ContainerRuntime),
    BareMetalImage,
}

impl DeploymentMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Foreground => "foreground",
            Self::UserService(manager) | Self::SystemService(manager) => manager.as_str(),
            Self::InitReplacement => "init",
            Self::Container(runtime) => runtime.as_str(),
            Self::BareMetalImage => "bare-metal",
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Foreground => "foreground".to_string(),
            Self::UserService(manager) => {
                let mut out = String::from("user:");
                out.push_str(manager.as_str());
                out
            }
            Self::SystemService(manager) => {
                let mut out = String::from("system:");
                out.push_str(manager.as_str());
                out
            }
            Self::InitReplacement => "init".to_string(),
            Self::Container(runtime) => {
                let mut out = String::from("container:");
                out.push_str(runtime.as_str());
                out
            }
            Self::BareMetalImage => "bare-metal".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineCapabilityReport {
    pub deployment: DeploymentMachineInventory,
    #[cfg(target_os = "linux")]
    pub machine: MachineReport,
    pub available_deploy_modes: Vec<DeploymentMode>,
}

#[cfg(target_os = "linux")]
pub fn gather_machine_capability_report() -> MachineCapabilityReport {
    gather_machine_capability_report_for_host(None)
}

#[cfg(target_os = "linux")]
pub fn gather_machine_capability_report_for_host(host: Option<&str>) -> MachineCapabilityReport {
    let deployment = gather_deployment_machine_inventory();
    let available_deploy_modes = plan_deployment_modes(&deployment);
    MachineCapabilityReport {
        deployment,
        machine: gather_machine_report_for_host(host),
        available_deploy_modes,
    }
}

pub fn plan_deployment_modes(inventory: &DeploymentMachineInventory) -> Vec<DeploymentMode> {
    let mut modes = Vec::new();
    modes.push(DeploymentMode::Foreground);
    for manager in &inventory.service_managers {
        match manager {
            ServiceManager::Systemd => {
                modes.push(DeploymentMode::UserService(*manager));
                if inventory.is_root || inventory.has_sudo || inventory.has_doas {
                    modes.push(DeploymentMode::SystemService(*manager));
                }
            }
            ServiceManager::Supervisor | ServiceManager::Cron => {
                modes.push(DeploymentMode::UserService(*manager));
            }
            ServiceManager::OpenRc
            | ServiceManager::Runit
            | ServiceManager::S6
            | ServiceManager::Dinit
            | ServiceManager::RcLocal => {
                if inventory.is_root || inventory.has_sudo || inventory.has_doas {
                    modes.push(DeploymentMode::SystemService(*manager));
                }
            }
        }
    }
    for runtime in &inventory.container_runtimes {
        modes.push(DeploymentMode::Container(*runtime));
    }
    if inventory.is_root {
        modes.push(DeploymentMode::InitReplacement);
        modes.push(DeploymentMode::BareMetalImage);
    }
    dedup_deployment_modes(modes)
}

fn dedup_deployment_modes(modes: Vec<DeploymentMode>) -> Vec<DeploymentMode> {
    let mut out = Vec::new();
    for mode in modes {
        if !out.contains(&mode) {
            out.push(mode);
        }
    }
    out
}

#[cfg(target_os = "linux")]
pub fn render_machine_capability_report(report: &MachineCapabilityReport) -> String {
    let mut out = String::new();
    out.push_str("edgerun capability report\n\n");
    out.push_str(&render_deployment_machine_inventory(&report.deployment));
    let modes = report
        .available_deploy_modes
        .iter()
        .map(|mode| mode.label())
        .collect::<Vec<_>>();
    push_inventory_line(&mut out, "available_modes", &join_inventory_strings(&modes));
    out.push('\n');
    out.push_str(&render_machine_report(&report.machine));
    out
}

pub fn render_deployment_machine_inventory(inventory: &DeploymentMachineInventory) -> String {
    let mut out = String::new();
    push_inventory_line(&mut out, "os", &inventory.os);
    push_inventory_line(&mut out, "arch", &inventory.arch);
    push_inventory_line(&mut out, "kernel", &inventory.kernel);
    push_inventory_line(
        &mut out,
        "libc",
        inventory.libc.as_deref().unwrap_or("unknown"),
    );
    push_inventory_line(
        &mut out,
        "pid1",
        inventory.pid1.as_deref().unwrap_or("unknown"),
    );
    push_inventory_line(
        &mut out,
        "uid",
        &inventory
            .uid
            .map(|uid| uid.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
    );
    push_inventory_line(&mut out, "root", yes_no(inventory.is_root));
    push_inventory_line(&mut out, "sudo", yes_no(inventory.has_sudo));
    push_inventory_line(&mut out, "doas", yes_no(inventory.has_doas));
    push_inventory_line(
        &mut out,
        "low_port_bind",
        yes_no(inventory.low_port_bind_allowed),
    );
    push_inventory_line(
        &mut out,
        "writable_paths",
        &join_inventory_strings(&inventory.writable_paths),
    );
    let service_managers = inventory
        .service_managers
        .iter()
        .map(|manager| manager.as_str().to_string())
        .collect::<Vec<_>>();
    push_inventory_line(
        &mut out,
        "service_managers",
        &join_inventory_strings(&service_managers),
    );
    let container_runtimes = inventory
        .container_runtimes
        .iter()
        .map(|runtime| runtime.as_str().to_string())
        .collect::<Vec<_>>();
    push_inventory_line(
        &mut out,
        "container_runtimes",
        &join_inventory_strings(&container_runtimes),
    );
    out
}

fn push_inventory_line(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push('=');
    out.push_str(value);
    out.push('\n');
}

fn join_inventory_strings(values: &[String]) -> String {
    if values.is_empty() {
        return "none".to_string();
    }
    let mut out = String::new();
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(value);
    }
    out
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
