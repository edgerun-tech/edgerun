//! CLI tool for managing the Lifegraph mesh daemon.
//!
//! Usage:
//!   mesh-tool run                    — Start the mesh daemon
//!   mesh-tool status                 — Show peers, routes, interfaces
//!   mesh-tool discover               — Broadcast one discovery packet
//!   mesh-tool add-tunnel <id> <ip> <port> — Add an IP tunnel to a peer
//!   mesh-tool interfaces             — List UP network interfaces

use lifegraph_hardware_signing::NodeID;
use lifegraph_mesh_daemon::{MeshDaemon, MeshDaemonConfig};
use lifegraph_remote_capability::{RemoteCapabilityProvider, RemoteInvocationResult};
use lifegraph_capabilities::CapabilityError;
use lifegraph_proto::lifegraph::v0::capability::{
    CapabilityDescriptor, CapabilityGrant, CapabilityInvocation,
    CapabilityRequest, CapabilityRevocation,
};
use lifegraph_proto::lifegraph::v0::capability_runtime::{
    CapabilitySessionAccept, CapabilitySessionClose, CapabilitySessionOpen,
};

/// No-op capability provider for the CLI tool (doesn't serve capabilities).
struct NoopProvider;

impl RemoteCapabilityProvider for NoopProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor::default()
    }
    fn open_session(&mut self, _open: &CapabilitySessionOpen) -> Result<CapabilitySessionAccept, CapabilityError> {
        Err(CapabilityError::Unsupported("cli tool does not serve capabilities"))
    }
    fn invoke(&mut self, _session_id: &[u8], _invocation: &CapabilityInvocation, _inline_params: Option<&[u8]>) -> Result<RemoteInvocationResult, CapabilityError> {
        Err(CapabilityError::Unsupported("cli tool does not serve capabilities"))
    }
    fn close_session(&mut self, _close: &CapabilitySessionClose) -> Result<(), CapabilityError> { Ok(()) }
    fn handle_request(&mut self, _request: &CapabilityRequest) -> Result<Option<CapabilityGrant>, CapabilityError> { Ok(None) }
    fn handle_grant(&mut self, _grant: &CapabilityGrant) -> Result<(), CapabilityError> { Ok(()) }
    fn handle_revocation(&mut self, _revocation: &CapabilityRevocation) -> Result<(), CapabilityError> { Ok(()) }
}

type CliDaemon = MeshDaemon<NoopProvider>;

fn usage() {
    eprintln!("usage: mesh-tool <command> [args]");
    eprintln!();
    eprintln!("commands:");
    eprintln!("  run                              Start the mesh daemon");
    eprintln!("  status                           Show peers, routes, interfaces");
    eprintln!("  discover                         Broadcast one discovery packet");
    eprintln!("  add-tunnel <id-hex> <ip> <port>  Add an IP tunnel to a peer");
    eprintln!("  interfaces                       List UP network interfaces");
    eprintln!("  help                             Show this help");
}

fn parse_node_id(hex: &str) -> Result<NodeID, String> {
    let bytes = hex::decode(hex).map_err(|e| format!("invalid hex: {e}"))?;
    if bytes.len() != 64 {
        return Err(format!(
            "NodeID must be 64 bytes (128 hex chars), got {} bytes ({} hex chars)",
            bytes.len(),
            hex.len()
        ));
    }
    let mut id = [0u8; 64];
    id.copy_from_slice(&bytes);
    Ok(NodeID(id))
}

fn cmd_run() -> Result<(), Box<dyn std::error::Error>> {
    // Generate a random NodeID for this node (from /dev/urandom)
    let node_id = generate_node_id();
    println!("mesh-daemon: starting with NodeID {}", node_id.short());

    let config = MeshDaemonConfig::default();
    let mut daemon = CliDaemon::new(node_id, config, NoopProvider);

    // Discover and open interfaces
    let interfaces = daemon.discover_and_open_interfaces()?;
    if interfaces.is_empty() {
        eprintln!("mesh-daemon: no UP interfaces found (excluding lo)");
    } else {
        println!("mesh-daemon: opened raw sockets on: {}", interfaces.join(", "));
    }

    // Install signal handler for clean shutdown
    setup_signal_handler(&mut daemon);

    println!("mesh-daemon: running (press Ctrl+C to stop)");
    daemon.run()?;
    println!("mesh-daemon: stopped");
    Ok(())
}

fn cmd_status() -> Result<(), Box<dyn std::error::Error>> {
    let node_id = generate_node_id();
    let config = MeshDaemonConfig::default();
    let daemon = CliDaemon::new(node_id, config, NoopProvider);

    // Show interfaces
    let interfaces = lifegraph_linux_netif::discover_network_interfaces()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    println!("Interfaces:");
    for iface in &interfaces {
        if iface.name == "lo" {
            continue;
        }
        println!(
            "  {}  state={:?}  mac={}",
            iface.name,
            iface.link_state,
            iface.mac_address.as_deref().unwrap_or("n/a")
        );
    }

    // Show routing table
    println!("\nRouting table (empty — no discovery yet):");
    for route in daemon.router().routing_table().iter() {
        println!(
            "  → {}  via={}  cost={}",
            route.destination.short(),
            route
                .next_hop
                .as_ref()
                .map(|n| n.short())
                .unwrap_or_else(|| "direct".to_string()),
            route.cost
        );
    }
    if daemon.router().routing_table().is_empty() {
        println!("  (no routes)");
    }

    // Show peers
    println!("\nPeers:");
    if daemon.router().all_peers().is_empty() {
        println!("  (no peers discovered)");
    }
    for peer in daemon.router().all_peers() {
        let status = if peer.is_dead() { "DEAD" } else { "alive" };
        println!(
            "  {}  last_seen={}  missed={}  status={}",
            peer.node_id.short(),
            peer.last_seen_unix,
            peer.missed_heartbeats,
            status
        );
    }

    Ok(())
}

fn cmd_discover() -> Result<(), Box<dyn std::error::Error>> {
    let node_id = generate_node_id();
    let config = MeshDaemonConfig::default();
    let mut daemon = CliDaemon::new(node_id, config, NoopProvider);

    let interfaces = daemon.discover_and_open_interfaces()?;
    if interfaces.is_empty() {
        eprintln!("mesh-tool: no UP interfaces found");
        return Ok(());
    }

    daemon.open_multicast_sockets()?;
    daemon.broadcast_discovery()?;
    println!("mesh-tool: discovery broadcast sent on {} interfaces", interfaces.len());
    Ok(())
}

fn cmd_add_tunnel(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 {
        eprintln!("usage: mesh-tool add-tunnel <node-id-hex> <ip> <port>");
        return Ok(());
    }
    let node_id = parse_node_id(&args[0])?;
    let ip: [u8; 4] = args[1]
        .split('.')
        .map(|s| s.parse::<u8>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("invalid IPv4: {e}"))?
        .try_into()
        .map_err(|_| "IPv4 must have exactly 4 octets".to_string())?;
    let port: u16 = args[2].parse().map_err(|e| format!("invalid port: {e}"))?;

    let config = MeshDaemonConfig::default();
    let mut daemon = CliDaemon::new(generate_node_id(), config, NoopProvider);
    daemon.add_tunnel(node_id, ip, port)?;
    println!("mesh-tool: tunnel added to {} at {}:{}", node_id.short(), args[1], port);
    Ok(())
}

fn cmd_interfaces() -> Result<(), Box<dyn std::error::Error>> {
    let interfaces = lifegraph_linux_netif::discover_network_interfaces()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    println!("Network interfaces:");
    for iface in &interfaces {
        // Check admin state via sysfs operstate
        let operstate = std::fs::read_to_string(iface.sysfs_path.join("operstate"))
            .unwrap_or_default();
        let up = operstate.trim() == "up";
        let marker = if up { "  UP" } else { "  DOWN" };
        println!(
            "  {}{}  kind={:?}  mac={}  mtu={}",
            iface.name,
            marker,
            iface.kind,
            iface.mac_address.as_deref().unwrap_or("n/a"),
            iface.mtu.map(|v| v.to_string()).unwrap_or_else(|| "n/a".to_string())
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generates a random 64-byte NodeID from /dev/urandom.
/// In a real system, this would come from the hardware secure enclave.
fn generate_node_id() -> NodeID {
    use std::io::Read;
    let mut bytes = [0u8; 64];
    let mut f = std::fs::File::open("/dev/urandom").expect("failed to open /dev/urandom");
    f.read_exact(&mut bytes).expect("failed to read from /dev/urandom");
    NodeID(bytes)
}

/// Installs signal handlers for clean shutdown.
fn setup_signal_handler(_daemon: &mut CliDaemon) {
    extern "C" fn handle_signal(_sig: i32) {
        std::process::exit(0);
    }
    unsafe {
        libc::signal(libc::SIGINT, handle_signal as *const () as usize);
        libc::signal(libc::SIGTERM, handle_signal as *const () as usize);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(|s| s.as_str()) {
        Some("run") => cmd_run(),
        Some("status") => cmd_status(),
        Some("discover") => cmd_discover(),
        Some("add-tunnel") => cmd_add_tunnel(&args[1..]),
        Some("interfaces") => cmd_interfaces(),
        Some("help") | None => {
            usage();
            Ok(())
        }
        Some(cmd) => {
            eprintln!("unknown command: {cmd}");
            usage();
            std::process::exit(1);
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
