use edgerun_devices::usb::UsbInventory;
use edgerun_linux_usb::{discover_usb_devices, LinuxUsbBackend};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let backend = LinuxUsbBackend {
        root_path: PathBuf::from("/sys/bus/usb/devices"),
    };
    match args.get(1).map(|v| v.as_str()) {
        None | Some("list") => match discover_usb_devices() {
            Ok(list) => {
                for dev in list {
                    println!(
                        "{} bus={} dev={} vid={:04x?} pid={:04x?} usb={} cfg={} ifaces={} speed={:?} mfg={} product={} driver={} parent={} children={}",
                        dev.instance_id,
                        dev.bus_num.unwrap_or_default(),
                        dev.dev_num.unwrap_or_default(),
                        dev.vendor_id,
                        dev.product_id,
                        dev.usb_version.unwrap_or_default(),
                        dev.configuration_value.map(|v| v.to_string()).unwrap_or_default(),
                        dev.interfaces.len(),
                        dev.speed,
                        dev.manufacturer.unwrap_or_default(),
                        dev.product_name.unwrap_or_default(),
                        dev.driver.unwrap_or_default(),
                        dev.parent_instance_id.unwrap_or_default(),
                        dev.child_instance_ids.join(",")
                    );
                    for iface in dev.interfaces {
                        println!(
                            "  iface={} num={} alt={} class={:02x?} subclass={:02x?} proto={:02x?} name={} driver={}",
                            iface.instance_id,
                            iface.interface_number.map(|v| v.to_string()).unwrap_or_default(),
                            iface.alternate_setting.map(|v| v.to_string()).unwrap_or_default(),
                            iface.interface_class,
                            iface.interface_subclass,
                            iface.interface_protocol,
                            iface.interface_name.unwrap_or_default(),
                            iface.driver.unwrap_or_default(),
                        );
                    }
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
            eprintln!("usage: linux-usb-tool [list|inventory]");
            std::process::exit(2);
        }
    }
}
