use edgerun_devices::gpu::{GpuConnectorInfo, GpuDisplayMode, GpuInventory};
use edgerun_linux_gpu::{discover_gpus, LinuxGpuBackend};
use std::path::PathBuf;

fn usage() {
    eprintln!("usage: linux-gpu-tool [list|inventory]");
}

fn format_mode(mode: &GpuDisplayMode) -> String {
    format!(
        "{}x{}@{}",
        mode.width,
        mode.height,
        mode.refresh_millihz / 1000
    )
}

fn format_connector(connector: &GpuConnectorInfo) -> String {
    let current = connector
        .current_mode
        .as_ref()
        .map(format_mode)
        .unwrap_or_else(|| "n/a".into());
    format!(
        "{}#{}:{}:{}:current={}:modes={}:cec={}",
        connector.name,
        connector
            .connector_id
            .map(|v| v.to_string())
            .unwrap_or_else(|| "n/a".into()),
        if connector.connected {
            "connected"
        } else {
            "disconnected"
        },
        if connector.enabled {
            "enabled"
        } else {
            "disabled"
        },
        current,
        connector.modes.len(),
        connector.cec_adapter.as_deref().unwrap_or("n/a")
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let backend = LinuxGpuBackend {
        pci_root: PathBuf::from("/sys/bus/pci/devices"),
        drm_root: PathBuf::from("/sys/class/drm"),
    };

    match args.get(1).map(|value| value.as_str()) {
        None | Some("list") => match discover_gpus() {
            Ok(devices) => {
                for device in devices {
                    let info = device.into_info();
                    println!(
                        concat!(
                            "instance={} vendor={:?} pci={} vid={:#06x?} did={:#06x?} ",
                            "driver={} module={} boot_vga={} cards={} render={} connectors={} ",
                            "card_nodes={} render_nodes={} display={} render_cap={} compute={} virtual={} modalias={}"
                        ),
                        info.instance_id,
                        info.vendor,
                        info.pci_address.as_deref().unwrap_or("n/a"),
                        info.vendor_id,
                        info.device_id,
                        info.driver.as_deref().unwrap_or("n/a"),
                        info.driver_module.as_deref().unwrap_or("n/a"),
                        info.boot_vga,
                        info.drm_cards.join(","),
                        info.drm_render_nodes.join(","),
                        info.connectors
                            .iter()
                            .map(format_connector)
                            .collect::<Vec<_>>()
                            .join(","),
                        info.drm_card_device_nodes.join(","),
                        info.drm_render_device_nodes.join(","),
                        info.supports_display,
                        info.supports_render,
                        info.supports_compute,
                        info.is_virtual,
                        info.modalias.as_deref().unwrap_or("n/a"),
                    );
                }
            }
            Err(err) => {
                eprintln!("list error: {err}");
                std::process::exit(1);
            }
        },
        Some("inventory") => match backend.list_gpus() {
            Ok(list) => println!("gpus={}", list.len()),
            Err(err) => {
                eprintln!("inventory error: {err}");
                std::process::exit(1);
            }
        },
        _ => {
            usage();
            std::process::exit(2);
        }
    }
}
