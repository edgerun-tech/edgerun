use edgerun_linux_netif::{discover_network_interfaces, LinuxNetifBackend};
use edgerun_network_interface::{NetworkAdminState, NetworkInterfaceController};

fn usage() {
    eprintln!("usage: linux-netif-tool list | state <ifname> | up <ifname> | down <ifname>");
}

fn find_backend(name: &str) -> Result<LinuxNetifBackend, String> {
    let interfaces = discover_network_interfaces().map_err(|e| e.to_string())?;
    let interface = interfaces
        .into_iter()
        .find(|v| v.name == name)
        .ok_or_else(|| format!("network interface not found: {name}"))?;
    Ok(LinuxNetifBackend { interface })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
        std::process::exit(2);
    }
    match args[1].as_str() {
        "list" => match discover_network_interfaces() {
            Ok(list) => {
                for iface in list {
                    println!(
                        "{} kind={:?} mac={} mtu={} link={:?}",
                        iface.name,
                        iface.kind,
                        iface.mac_address.unwrap_or_default(),
                        iface.mtu.map(|v| v.to_string()).unwrap_or_default(),
                        iface.link_state,
                    );
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        "state" if args.len() == 3 => match find_backend(&args[2])
            .and_then(|b| b.interface_info().map_err(|e| e.to_string()))
        {
            Ok(info) => println!(
                "if={} kind={:?} admin={:?} link={:?} mac={} mtu={}",
                info.interface_name,
                info.kind,
                info.admin_state,
                info.link_state,
                info.mac_address.unwrap_or_default(),
                info.mtu.map(|v| v.to_string()).unwrap_or_default(),
            ),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        "up" if args.len() == 3 => match find_backend(&args[2]).and_then(|b| {
            b.set_admin_state(NetworkAdminState::Up)
                .map_err(|e| e.to_string())
        }) {
            Ok(state) => println!("{:?}", state),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        "down" if args.len() == 3 => match find_backend(&args[2]).and_then(|b| {
            b.set_admin_state(NetworkAdminState::Down)
                .map_err(|e| e.to_string())
        }) {
            Ok(state) => println!("{:?}", state),
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
