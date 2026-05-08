use std::path::PathBuf;

use edgerun_devices::network_interface::NetworkLinkState;
use edgerun_linux_netif::discover_network_interfaces;
use edgerun_storage::fs::scan_event_logs;

pub fn cmd_status(path: &PathBuf) {
    let scanned = scan_event_logs(&path.join("events")).unwrap_or_else(|e| {
        eprintln!(
            "error: event log not found or invalid at {}: {}. Run `edged init` first.",
            path.display(),
            e
        );
        std::process::exit(1);
    });
    let Some(genesis) = scanned.iter().find(|event| event.event.seq == 0) else {
        eprintln!("error: event log has no genesis event");
        std::process::exit(1);
    };
    let node_id_hex =
        edgerun_protocols::core_protocol::util::bytes_to_hex(&genesis.event.stream_id);

    println!("edgerun Node Status");
    println!("  Data root:  {}", path.display());
    println!("  NodeID:     {}", node_id_hex);
    println!("  Events:     {}", scanned.len());
    println!(
        "  Head seq:   {}",
        scanned.iter().map(|e| e.event.seq).max().unwrap_or(0)
    );

    let interfaces = discover_network_interfaces().unwrap_or_default();
    let up_interfaces: Vec<_> = interfaces
        .iter()
        .filter(|i| i.link_state == NetworkLinkState::Up && i.name != "lo")
        .map(|i| &i.name)
        .collect();
    println!("  UP interfaces: {:?}", up_interfaces);

    // Hardware inventory
    println!();
    println!("Hardware Inventory:");
    let hw = edgerun_node::hardware::HardwareInventory::discover();
    println!("{}", hw.summary());
    let runtime_id = edgerun_node::runtime::sha256(&genesis.event.stream_id);
    let provider_apps = hw.capability_provider_apps(runtime_id);
    println!("  Provider apps:   {}", provider_apps.len());
}
