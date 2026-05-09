#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

pub use edgerun_machine_report::{
    ContainerRuntime, DeploymentMachineInventory, DeploymentMode as DeployMode,
    MachineCapabilityReport, ServiceManager, plan_deployment_modes,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeployError {
    Inventory(String),
}

impl core::fmt::Display for DeployError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Inventory(message) => f.write_str(message),
        }
    }
}

impl core::error::Error for DeployError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeployTarget {
    Local,
    RemoteSsh {
        user: String,
        host: String,
        port: u16,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetInventory {
    pub target: DeployTarget,
    pub machine: DeploymentMachineInventory,
}

impl TargetInventory {
    pub fn modes(&self) -> Vec<DeployMode> {
        plan_deployment_modes(&self.machine)
    }
}

pub fn parse_ssh_target(input: &str) -> Result<DeployTarget, DeployError> {
    let (user, rest) = input
        .split_once('@')
        .ok_or_else(|| DeployError::Inventory("ssh target must be user@host[:port]".to_string()))?;
    if user.is_empty() || rest.is_empty() {
        return Err(DeployError::Inventory(
            "ssh target must include both user and host".to_string(),
        ));
    }
    let (host, port) = if let Some((host, port)) = rest.rsplit_once(':') {
        if host.contains(']') || rest.matches(':').count() == 1 {
            let parsed = port
                .parse::<u16>()
                .map_err(|_| DeployError::Inventory("bad ssh target port".to_string()))?;
            (host.trim_matches(['[', ']']).to_string(), parsed)
        } else {
            (rest.to_string(), 22)
        }
    } else {
        (rest.to_string(), 22)
    };
    if host.is_empty() {
        return Err(DeployError::Inventory(
            "ssh target host is empty".to_string(),
        ));
    }
    Ok(DeployTarget::RemoteSsh {
        user: user.to_string(),
        host,
        port,
    })
}

pub fn inventory_report(inventory: &TargetInventory) -> String {
    let machine = &inventory.machine;
    let mut out = String::new();
    push_line(&mut out, "target", &target_label(&inventory.target));
    push_line(&mut out, "os", &machine.os);
    push_line(&mut out, "arch", &machine.arch);
    push_line(&mut out, "kernel", &machine.kernel);
    push_line(
        &mut out,
        "libc",
        machine.libc.as_deref().unwrap_or("unknown"),
    );
    push_line(
        &mut out,
        "pid1",
        machine.pid1.as_deref().unwrap_or("unknown"),
    );
    push_line(
        &mut out,
        "uid",
        &machine
            .uid
            .map(|uid| uid.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
    );
    push_line(&mut out, "root", bool_str(machine.is_root));
    push_line(&mut out, "sudo", bool_str(machine.has_sudo));
    push_line(&mut out, "doas", bool_str(machine.has_doas));
    push_line(
        &mut out,
        "low_port_bind",
        bool_str(machine.low_port_bind_allowed),
    );
    push_line(
        &mut out,
        "writable_paths",
        &join_strings(&machine.writable_paths),
    );
    let managers = machine
        .service_managers
        .iter()
        .map(|manager| manager.as_str().to_string())
        .collect::<Vec<_>>();
    push_line(&mut out, "service_managers", &join_strings(&managers));
    let containers = machine
        .container_runtimes
        .iter()
        .map(|runtime| runtime.as_str().to_string())
        .collect::<Vec<_>>();
    push_line(&mut out, "container_runtimes", &join_strings(&containers));
    let modes = inventory
        .modes()
        .iter()
        .map(|mode| mode.label())
        .collect::<Vec<_>>();
    push_line(&mut out, "available_modes", &join_strings(&modes));
    out
}

pub fn parse_deployment_machine_inventory(
    text: &str,
) -> Result<DeploymentMachineInventory, DeployError> {
    let value = |key: &str| -> Option<&str> {
        text.lines()
            .filter_map(|line| line.split_once('='))
            .find_map(|(candidate, value)| (candidate == key).then_some(value.trim()))
    };
    let os = required_value(value("os"), "os")?.to_string();
    let arch = required_value(value("arch"), "arch")?.to_string();
    let kernel = required_value(value("kernel"), "kernel")?.to_string();
    let libc = optional_value(value("libc"));
    let pid1 = optional_value(value("pid1"));
    let uid = optional_value(value("uid")).and_then(|uid| uid.parse::<u32>().ok());
    Ok(DeploymentMachineInventory {
        os,
        arch,
        kernel,
        libc,
        pid1,
        uid,
        is_root: parse_bool(value("root")),
        has_sudo: parse_bool(value("sudo")),
        has_doas: parse_bool(value("doas")),
        writable_paths: parse_string_list(value("writable_paths")),
        service_managers: parse_service_managers(value("service_managers")),
        container_runtimes: parse_container_runtimes(value("container_runtimes")),
        low_port_bind_allowed: parse_bool(value("low_port_bind")),
    })
}

fn required_value<'a>(value: Option<&'a str>, key: &str) -> Result<&'a str, DeployError> {
    value
        .filter(|value| !value.is_empty() && *value != "unknown")
        .ok_or_else(|| DeployError::Inventory(format!("remote inventory missing {key}")))
}

fn optional_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "unknown" && *value != "none")
        .map(str::to_string)
}

fn parse_bool(value: Option<&str>) -> bool {
    matches!(value, Some("yes" | "true" | "1"))
}

fn parse_string_list(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "none")
        .map(str::to_string)
        .collect()
}

fn parse_service_managers(value: Option<&str>) -> Vec<ServiceManager> {
    value
        .unwrap_or("")
        .split(',')
        .filter_map(|value| match value.trim() {
            "systemd" => Some(ServiceManager::Systemd),
            "openrc" => Some(ServiceManager::OpenRc),
            "runit" => Some(ServiceManager::Runit),
            "s6" => Some(ServiceManager::S6),
            "dinit" => Some(ServiceManager::Dinit),
            "supervisor" => Some(ServiceManager::Supervisor),
            "cron" => Some(ServiceManager::Cron),
            "rc.local" => Some(ServiceManager::RcLocal),
            _ => None,
        })
        .collect()
}

fn parse_container_runtimes(value: Option<&str>) -> Vec<ContainerRuntime> {
    value
        .unwrap_or("")
        .split(',')
        .filter_map(|value| match value.trim() {
            "docker" => Some(ContainerRuntime::Docker),
            "podman" => Some(ContainerRuntime::Podman),
            "containerd" => Some(ContainerRuntime::Containerd),
            _ => None,
        })
        .collect()
}

fn target_label(target: &DeployTarget) -> String {
    match target {
        DeployTarget::Local => "local".to_string(),
        DeployTarget::RemoteSsh { user, host, port } => format!("{user}@{host}:{port}"),
    }
}

fn push_line(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push('=');
    out.push_str(value);
    out.push('\n');
}

fn join_strings(values: &[String]) -> String {
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

fn bool_str(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(feature = "std")]
pub mod host {
    use super::*;

    pub fn inventory_local() -> TargetInventory {
        TargetInventory {
            target: DeployTarget::Local,
            machine: edgerun_machine_report::gather_deployment_machine_inventory(),
        }
    }

    pub fn capability_report_local() -> MachineCapabilityReport {
        edgerun_machine_report::gather_machine_capability_report()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_user_host_port() {
        assert_eq!(
            parse_ssh_target("root@example.com:2222").unwrap(),
            DeployTarget::RemoteSsh {
                user: "root".to_string(),
                host: "example.com".to_string(),
                port: 2222
            }
        );
    }

    #[test]
    fn root_inventory_gets_init_modes() {
        let inventory = DeploymentMachineInventory {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            kernel: "test".to_string(),
            libc: None,
            pid1: Some("init".to_string()),
            uid: Some(0),
            is_root: true,
            has_sudo: false,
            has_doas: false,
            writable_paths: Vec::new(),
            service_managers: vec![ServiceManager::OpenRc],
            container_runtimes: vec![ContainerRuntime::Podman],
            low_port_bind_allowed: true,
        };
        let modes = plan_deployment_modes(&inventory);
        assert!(modes.contains(&DeployMode::Foreground));
        assert!(modes.contains(&DeployMode::SystemService(ServiceManager::OpenRc)));
        assert!(modes.contains(&DeployMode::Container(ContainerRuntime::Podman)));
        assert!(modes.contains(&DeployMode::InitReplacement));
    }
}
