use lifegraph_linux_wifi::{discover_wifi_interfaces, LinuxWifiBackend};
use lifegraph_wifi::{
    WifiAccessPointConfig, WifiAccessPointController, WifiController, WifiPowerState, WifiScanner,
};

fn usage() {
    eprintln!(
        "usage: linux-wifi-tool list | state <ifname> | scan <ifname> | ap-state <ifname> | ap-start <ifname> <ssid> [freq_mhz] | ap-stop <ifname> | enable <ifname> | disable <ifname> | block <ifname>"
    );
}

fn find_backend(name: &str) -> Result<LinuxWifiBackend, String> {
    let interfaces = discover_wifi_interfaces().map_err(|e| e.to_string())?;
    let interface = interfaces
        .into_iter()
        .find(|v| v.name == name)
        .ok_or_else(|| format!("wifi interface not found: {name}"))?;
    Ok(LinuxWifiBackend { interface })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
        std::process::exit(2);
    }
    match args[1].as_str() {
        "list" => match discover_wifi_interfaces() {
            Ok(list) => {
                for iface in list {
                    println!(
                        "{} mac={} phy={} operstate={} power={:?}",
                        iface.name,
                        iface.mac_address.unwrap_or_default(),
                        iface.phy_name.unwrap_or_default(),
                        iface.operstate.unwrap_or_default(),
                        iface.power_state
                    );
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        "state" if args.len() == 3 => {
            match find_backend(&args[2]).and_then(|b| b.power_state().map_err(|e| e.to_string())) {
                Ok(s) => println!("{:?}", s),
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
        }
        "scan" if args.len() == 3 => match find_backend(&args[2]) {
            Ok(b) => match b.scan_nearby() {
                Ok(scan) => {
                    if scan.observations.is_empty() {
                        println!("no current network observation");
                    }
                    for obs in scan.observations {
                        println!(
                            "if={} ssid={} bssid={} signal_dbm={} freq_mhz={} secure={:?}",
                            obs.interface_name,
                            obs.ssid.unwrap_or_default(),
                            obs.bssid.unwrap_or_default(),
                            obs.signal_dbm.map(|v| v.to_string()).unwrap_or_default(),
                            obs.frequency_mhz.map(|v| v.to_string()).unwrap_or_default(),
                            obs.secure,
                        );
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        "ap-state" if args.len() == 3 => match find_backend(&args[2]) {
            Ok(b) => match b.access_point_state() {
                Ok(state) => println!(
                    "if={} active={} ssid={} freq_mhz={} hidden={} secure={:?}",
                    state.interface_name,
                    state.active,
                    state.ssid.unwrap_or_default(),
                    state
                        .frequency_mhz
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    state.hidden,
                    state.secure
                ),
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        "ap-start" if args.len() == 4 || args.len() == 5 => match find_backend(&args[2]) {
            Ok(b) => {
                let frequency_mhz = if args.len() == 5 {
                    match args[4].parse::<u32>() {
                        Ok(v) => Some(v),
                        Err(e) => {
                            eprintln!("error: invalid frequency: {e}");
                            std::process::exit(2);
                        }
                    }
                } else {
                    None
                };
                match b.start_access_point(&WifiAccessPointConfig {
                    ssid: args[3].clone(),
                    frequency_mhz,
                    hidden: false,
                    secure: false,
                }) {
                    Ok(state) => println!(
                        "if={} active={} ssid={} freq_mhz={}",
                        state.interface_name,
                        state.active,
                        state.ssid.unwrap_or_default(),
                        state
                            .frequency_mhz
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                    ),
                    Err(e) => {
                        eprintln!("error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        "ap-stop" if args.len() == 3 => match find_backend(&args[2]) {
            Ok(b) => match b.stop_access_point() {
                Ok(state) => println!(
                    "if={} active={} ssid={}",
                    state.interface_name,
                    state.active,
                    state.ssid.unwrap_or_default(),
                ),
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        "enable" if args.len() == 3 => match find_backend(&args[2]).and_then(|b| {
            b.set_power_state(WifiPowerState::Enabled)
                .map_err(|e| e.to_string())
        }) {
            Ok(s) => println!("{:?}", s),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        "disable" if args.len() == 3 => match find_backend(&args[2]).and_then(|b| {
            b.set_power_state(WifiPowerState::Disabled)
                .map_err(|e| e.to_string())
        }) {
            Ok(s) => println!("{:?}", s),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        "block" if args.len() == 3 => match find_backend(&args[2]).and_then(|b| {
            b.set_power_state(WifiPowerState::Blocked)
                .map_err(|e| e.to_string())
        }) {
            Ok(s) => println!("{:?}", s),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        _ => {
            usage();
            std::process::exit(2);
        }
    }
}
