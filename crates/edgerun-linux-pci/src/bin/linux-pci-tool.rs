use edgerun_linux_pci::{discover_pci_devices, LinuxPciBackend};
use edgerun_pci::PciInventory;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let backend = LinuxPciBackend {
        root_path: PathBuf::from("/sys/bus/pci/devices"),
    };
    match args.get(1).map(|v| v.as_str()) {
        None | Some("list") => match discover_pci_devices() {
            Ok(list) => {
                for dev in list {
                    println!(
                        "{} vid={:04x?} did={:04x?} class={:06x?} rev={:02x?} driver={} parent={} children={}",
                        dev.address,
                        dev.vendor_id,
                        dev.device_id,
                        dev.class_code,
                        dev.revision,
                        dev.driver.unwrap_or_default(),
                        dev.parent_address.unwrap_or_default(),
                        dev.child_addresses.join(",")
                    );
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        Some("inventory") => match backend.list_devices() {
            Ok(list) => println!("devices={}", list.len()),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        _ => {
            eprintln!("usage: linux-pci-tool [list|inventory]");
            std::process::exit(2);
        }
    }
}
