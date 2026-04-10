use std::fs;
use std::path::PathBuf;

use crate::config::{NodeConfig, parse_config};
use crate::signer::load_signer_from_config;
use edgerun_linux_netif::discover_network_interfaces;
use edgerun_network_interface::NetworkLinkState;

pub fn cmd_status(path: &PathBuf) {
    let yaml = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("error: config not found at {}: {}. Run `edgerund init` first.", path.display(), e);
            std::process::exit(1);
        }
    };
    let config: NodeConfig = parse_config(&yaml).unwrap_or_else(|e| {
        eprintln!("error: invalid config: {}", e);
        std::process::exit(1);
    });

    let signer = load_signer_from_config(&config);
    let node_id = signer.node_id();

    println!("edgerun Node Status");
    println!("  Name:       {}", config.name.as_deref().unwrap_or("(unnamed)"));
    println!("  Stream ID:  {}", config.stream_id);
    println!("  NodeID:     {}", node_id.to_hex());
    println!("  Short ID:   {}", node_id.short());
    println!("  Signer:     {}", config.signer.as_ref().map(|s| &s.signer_type).unwrap_or(&"unconfigured".to_string()));
    println!("  Controllers: {:?}", config.controllers);
    println!("  Trust nodes: {:?}", config.trust_nodes);

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
    let hw = crate::hardware::HardwareInventory::discover();
    println!("{}", hw.summary());
}
