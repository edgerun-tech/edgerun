use edgerun_devices::power::PowerInventory;
use edgerun_linux_power::{discover_power_supplies, discover_power_system, LinuxPowerBackend};
use std::path::PathBuf;

fn usage() {
    eprintln!("usage: linux-power-tool [list|system|inventory]");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let backend = LinuxPowerBackend {
        power_supply_root: PathBuf::from("/sys/class/power_supply"),
        proc_acpi_root: PathBuf::from("/proc/acpi/button/lid"),
    };
    match args.get(1).map(|value| value.as_str()) {
        None | Some("list") => match discover_power_supplies() {
            Ok(supplies) => {
                for supply in supplies {
                    println!(
                        "{} kind={:?} online={:?} present={:?} status={:?} capacity={:?}",
                        supply.instance_id,
                        supply.kind,
                        supply.online,
                        supply.present,
                        supply.battery_status,
                        supply.capacity_percent,
                    );
                }
            }
            Err(err) => {
                eprintln!("list error: {err}");
                std::process::exit(1);
            }
        },
        Some("system") => match discover_power_system() {
            Ok(system) => {
                println!("sources={}", system.sources.len());
                println!("lid_state={:?}", system.lid_state);
                println!("on_ac_power={:?}", system.on_ac_power);
                println!("battery_percent={:?}", system.battery_percent);
            }
            Err(err) => {
                eprintln!("system error: {err}");
                std::process::exit(1);
            }
        },
        Some("inventory") => match backend.power_info() {
            Ok(system) => println!("sources={}", system.sources.len()),
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
