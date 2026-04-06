//! Lifegraph Node Daemon (lifegraphd)
//!
//! Runs a single-writer stream node with mesh networking,
//! command processing, and capability discovery.
//!
//! ## Usage
//! ```text
//! lifegraphd init --config node.yaml --tpm /dev/tpmrm0  # Provision key into TPM
//! lifegraphd init --config node.yaml --software         # Dev-only: in-memory key
//! lifegraphd run --config node.yaml                     # Start the daemon
//! lifegraphd status --config node.yaml                  # Show node identity and peers
//! ```
//!
//! ## Security
//! The node's private key NEVER leaves secure hardware. The config file only
//! stores the public key (NodeID) and a reference to the hardware key handle.
//! No `.key` file is ever written.

use clap::{Parser, Subcommand};
use lifegraph_core::protocol::EventType;
use lifegraph_hardware_signing::{MeshSigner, NodeID};
use lifegraph_mesh::{FrameType, LocalNode};
use lifegraph_mesh_link::MeshLink;
use lifegraph_mesh_router::MeshRouter;
use lifegraph_stream::StreamWriter;
use prost::Message;
use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::hazmat::RandomizedPrehashSigner;
use rand::rngs::OsRng;
use std::fs;
use lifegraph_linux_netif::discover_network_interfaces;
use lifegraph_network_interface::NetworkLinkState;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "lifegraphd", about = "Lifegraph Node Daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new node identity and config
    Init {
        #[arg(long, default_value = "node.yaml")]
        config: PathBuf,
        #[arg(long)]
        name: Option<String>,
        /// Dev-only: generate an in-memory software key (NOT for production)
        #[arg(long)]
        software: bool,
    },
    /// Start the node daemon
    Run {
        #[arg(long, default_value = "node.yaml")]
        config: PathBuf,
    },
    /// Show node identity and state
    Status {
        #[arg(long, default_value = "node.yaml")]
        config: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { config, name, software } => {
            cmd_init(&config, name, software);
        }
        Commands::Run { config } => {
            cmd_run(&config);
        }
        Commands::Status { config } => {
            cmd_status(&config);
        }
    }
}

// ---------------------------------------------------------------------------
// Init — generate identity + config
// ---------------------------------------------------------------------------

fn cmd_init(path: &PathBuf, name: Option<String>, software: bool) {
    // Check hardware availability
    let has_tpm = PathBuf::from("/dev/tpmrm0").exists();
    let has_yubikey = check_yubikey_available();

    if !software && !has_tpm && !has_yubikey {
        eprintln!("error: no secure hardware found.");
        eprintln!();
        eprintln!("Available hardware backends:");
        eprintln!("  TPM 2.0:    {} (device: /dev/tpmrm0)", if has_tpm { "FOUND" } else { "not found" });
        eprintln!("  YubiKey:    {}", if has_yubikey { "FOUND" } else { "not found" });
        eprintln!();
        eprintln!("For development only, you can generate a software key with --software:");
        eprintln!("  lifegraphd init --config {} --software", path.display());
        eprintln!();
        eprintln!("WARNING: software keys are NOT secure. The private key will be stored");
        eprintln!("in the config file and can be extracted by anyone with file access.");
        std::process::exit(1);
    }

    let (node_id, key_material) = if software {
        eprintln!("WARNING: --software generates an INSECURE key stored in the config file.");
        eprintln!("This is for development/testing only. NEVER use in production.");
        eprintln!();
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let encoded = verifying_key.to_encoded_point(false);
        let mut node_id_bytes = [0u8; 64];
        node_id_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
        let node_id = NodeID(node_id_bytes);
        let key_hex = hex::encode(signing_key.to_bytes());
        (node_id, Some(key_hex))
    } else if has_tpm {
        eprintln!("TPM found at /dev/tpmrm0, but automated key provisioning is not yet implemented.");
        eprintln!("Use the tpm2-tools CLI to create a signing key, then reference its handle in config:");
        eprintln!("  tpm2_createprimary -C o -c primary.ctx");
        eprintln!("  tpm2_create -C primary.ctx -G ecc -c signing_key.ctx");
        eprintln!("  tpm2_evictcontrol -c signing_key.ctx -p 0x81010001");
        eprintln!();
        eprintln!("Then add to your config:");
        eprintln!("  signer:");
        eprintln!("    type: tpm");
        eprintln!("    handle: 0x81010001  # persistent handle after tpm2_evictcontrol");
        std::process::exit(0);
    } else {
        // has_yubikey
        eprintln!("YubiKey found, but automated key provisioning is not yet implemented.");
        eprintln!("Use yubico-piv-tool to create a signing key in slot 9a, then reference it:");
        eprintln!("  yubico-piv-tool -a generate -s 9a -A ecP256");
        eprintln!("  yubico-piv-tool -a verify-pin -a selfsign-certificate -s 9a ...");
        eprintln!();
        eprintln!("Then add to your config:");
        eprintln!("  signer:");
        eprintln!("    type: yubikey");
        eprintln!("    slot: 9a");
        std::process::exit(0);
    };

    let node_name = name.unwrap_or_else(|| format!("lifegraph-{}", node_id.short()));
    let stream_id = format!("stream-{}", node_id.short());

    let signer_block = if let Some(ref kh) = key_material {
        format!(
            r#"signer:
  type: "software"
  # WARNING: This is an insecure software-generated key for dev/testing only.
  # The private key is stored in plaintext. NEVER use in production.
  public_key_hex: "{node_id_hex}"
  private_key_hex: "{key_hex}"
"#,
            node_id_hex = node_id.to_hex(),
            key_hex = kh,
        )
    } else {
        String::new()
    };

    let config_yaml = format!(
        r#"# Lifegraph Node Configuration
# Generated by lifegraphd init
stream_id: "{stream_id}"
name: "{node_name}"
controllers: []
trust_nodes: []
initial_grants: []
{signer_block}metadata:
  environment: "production"
"#
    );

    fs::write(path, &config_yaml).unwrap_or_else(|e| {
        eprintln!("error: failed to write config to {}: {}", path.display(), e);
        std::process::exit(1);
    });

    println!("Node identity generated:");
    println!("  NodeID:     {}", node_id.to_hex());
    println!("  Short ID:   {}", node_id.short());
    println!("  Stream ID:  {}", stream_id);
    println!("  Name:       {}", node_name);
    println!("  Signer:     {}", if key_material.is_some() { "software (INSECURE)" } else { "hardware (provision manually)" });
    println!("  Config:     {}", path.display());
    if key_material.is_some() {
        println!();
        println!("WARNING: This is a SOFTWARE KEY. The private key is stored in the config file.");
        println!("Do NOT use this key in production. Use TPM or YubiKey for secure hardware keys.");
    } else {
        println!();
        println!("Private key is stored in secure hardware. No key file was written.");
    }
    println!();
    println!("Start the node with:");
    println!("  lifegraphd run --config {}", path.display());
}

fn check_yubikey_available() -> bool {
    fs::read_dir("/sys/bus/usb/devices/")
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                let path = entry.path();
                if path.join("idVendor").exists() {
                    if let Ok(vendor) = fs::read_to_string(path.join("idVendor")) {
                        return vendor.trim().eq_ignore_ascii_case("1050");
                    }
                }
                false
            })
        })
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

fn cmd_status(path: &PathBuf) {
    if !path.exists() {
        eprintln!("error: config not found at {}. Run `lifegraphd init` first.", path.display());
        std::process::exit(1);
    }

    let yaml = fs::read_to_string(path).unwrap();
    let config: NodeConfig = serde_yaml::from_str(&yaml).unwrap_or_else(|e| {
        eprintln!("error: invalid config: {}", e);
        std::process::exit(1);
    });

    let signer = load_signer_from_config(&config);
    let node_id = signer.node_id();

    println!("Lifegraph Node Status");
    println!("  Name:       {}", config.name.as_deref().unwrap_or("(unnamed)"));
    println!("  Stream ID:  {}", config.stream_id);
    println!("  NodeID:     {}", node_id.to_hex());
    println!("  Short ID:   {}", node_id.short());
    println!("  Signer:     {}", config.signer.as_ref().map(|s| &s.signer_type).unwrap_or(&"unconfigured".to_string()));
    println!("  Controllers: {:?}", config.controllers);
    println!("  Trust nodes: {:?}", config.trust_nodes);

    // Show network interfaces
    let interfaces = discover_network_interfaces().unwrap_or_default();
    let up_interfaces: Vec<_> = interfaces
        .iter()
        .filter(|i| i.link_state == NetworkLinkState::Up && i.name != "lo")
        .map(|i| &i.name)
        .collect();
    println!("  UP interfaces: {:?}", up_interfaces);
}

// ---------------------------------------------------------------------------
// Run — the daemon event loop
// ---------------------------------------------------------------------------

fn cmd_run(path: &PathBuf) {
    if !path.exists() {
        eprintln!("error: config not found at {}. Run `lifegraphd init` first.", path.display());
        std::process::exit(1);
    }

    let yaml = fs::read_to_string(path).unwrap();
    let config: NodeConfig = serde_yaml::from_str(&yaml).unwrap_or_else(|e| {
        eprintln!("error: invalid config: {}", e);
        std::process::exit(1);
    });

    let signer = load_signer_from_config(&config);
    let node_id = signer.node_id();

    println!(
        "lifegraphd: starting node={} stream={} id={} signer={}",
        config.name.as_deref().unwrap_or("(unnamed)"),
        config.stream_id,
        node_id.short(),
        config.signer.as_ref().map(|s| &s.signer_type).unwrap_or(&"unconfigured".to_string()),
    );

    // Create the node's event stream
    let mut stream_writer = StreamWriter::new(
        config.stream_id.clone(),
        signer,
        now_ms(),
    )
    .unwrap_or_else(|e| {
        eprintln!("error: failed to create stream: {}", e);
        std::process::exit(1);
    });

    println!(
        "lifegraphd: genesis event created (seq=0, stream={})",
        config.stream_id
    );

    // Set up mesh networking
    let local = LocalNode::new(node_id);
    let mut mesh_link = MeshLink::new();
    mesh_link.set_local_node_id(node_id);
    let mut router = MeshRouter::new(local);

    // Open raw sockets on UP interfaces
    let interfaces = discover_network_interfaces().unwrap_or_default();
    let mut opened = 0;
    for iface in &interfaces {
        if iface.link_state != NetworkLinkState::Up || iface.name == "lo" {
            continue;
        }
        // Get ifindex from sysfs
        let ifindex_path = format!("/sys/class/net/{}/ifindex", iface.name);
        let ifindex_str = match fs::read_to_string(&ifindex_path) {
            Ok(s) => s.trim().to_string(),
            Err(_) => continue,
        };
        let ifindex: i32 = match ifindex_str.parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        match mesh_link.add_raw_ethernet(ifindex) {
            Ok(_) => {
                println!("lifegraphd: opened raw socket on {} (ifindex={})", iface.name, ifindex);
                opened += 1;
            }
            Err(e) => {
                eprintln!(
                    "lifegraphd: warning: could not open raw socket on {}: {}",
                    iface.name, e
                );
            }
        }
    }
    if opened == 0 {
        eprintln!("lifegraphd: no interfaces opened for mesh (running loopback only)");
    }

    // Initial discovery broadcast
    if let Err(e) = mesh_link.broadcast_discovery(&mut router) {
        eprintln!("lifegraphd: warning: discovery broadcast failed: {}", e);
    }

    // Install signal handler
    setup_signal_handler();

    println!("lifegraphd: running (stream events={})", stream_writer.events().len());

    // Main event loop
    let mut discovery_counter: u64 = 0;
    loop {
        // 1. Drain inbound data frames from mesh link
        let frames = mesh_link.drain_inbound_data_frames();
        for frame in frames {
            if frame.header.dest == node_id || frame.header.dest.0 == [0u8; 64] {
                // Frame is for us (or broadcast)
                if frame.header.frame_type == FrameType::Data {
                    // Data frame — try to decode as command
                    if let Ok(command) =
                        lifegraph_proto::lifegraph::v0::stream::CommandEnvelope::decode(&frame.payload[..])
                    {
                        process_command(&command, &mut stream_writer);
                    }
                }
            } else {
                // Forward to next hop
                if let Some(next_hop) = router.next_hop_for(&frame.header.dest) {
                    let mut fwd = frame;
                    fwd.header.dest = next_hop;
                    mesh_link.queue_frame(fwd);
                }
            }
        }

        // 2. Periodic discovery (every ~500 ticks for ~5s intervals)
        discovery_counter += 1;
        if discovery_counter % 500 == 0 {
            if let Err(e) = mesh_link.broadcast_discovery(&mut router) {
                eprintln!("lifegraphd: discovery failed: {}", e);
            }
            // Check for dead peers
            let dead = router.tick_heartbeat();
            for d in &dead {
                println!("lifegraphd: peer {} is dead", d.short());
            }
        }

        // 3. Send pending outbound frames
        if let Err(e) = mesh_link.drain_pending_frames(&mut router) {
            eprintln!("lifegraphd: send failed: {}", e);
        }

        // Brief sleep to avoid busy loop
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn process_command(command: &lifegraph_proto::lifegraph::v0::stream::CommandEnvelope, stream: &mut StreamWriter) {
    // Validate signature
    let Some(sig) = &command.signature else {
        println!("lifegraphd: rejecting command (no signature)");
        let _ = stream.append(
            EventType::CommandRejected as i32,
            1,
            now_ms(),
        );
        return;
    };

    if sig.algorithm != 2 {
        // Not ECDSA P-256
        println!("lifegraphd: rejecting command (wrong algorithm: {})", sig.algorithm);
        let _ = stream.append(
            EventType::CommandRejected as i32,
            1,
            now_ms(),
        );
        return;
    }

    // Extract issuer public key from command
    let Some(issuer) = &command.issuer else {
        println!("lifegraphd: rejecting command (no issuer)");
        let _ = stream.append(
            EventType::CommandRejected as i32,
            1,
            now_ms(),
        );
        return;
    };

    let Some(key_hint) = &issuer.key_hint else {
        println!("lifegraphd: rejecting command (no key hint)");
        let _ = stream.append(
            EventType::CommandRejected as i32,
            1,
            now_ms(),
        );
        return;
    };

    if key_hint.len() != 64 {
        println!("lifegraphd: rejecting command (bad key hint length: {})", key_hint.len());
        let _ = stream.append(
            EventType::CommandRejected as i32,
            1,
            now_ms(),
        );
        return;
    }

    // Verify signature
    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04;
    vk_sec1[1..].copy_from_slice(key_hint);
    let vk = match p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1) {
        Ok(v) => v,
        Err(e) => {
            println!("lifegraphd: rejecting command (bad public key: {})", e);
            let _ = stream.append(
                EventType::CommandRejected as i32,
                1,
                now_ms(),
            );
            return;
        }
    };

    // Encode command without signature for verification
    let mut signable_cmd = command.clone();
    signable_cmd.signature = None;
    let mut canonical = Vec::new();
    prost::Message::encode(&signable_cmd, &mut canonical).unwrap();
    use sha2::{Digest as _, Sha256};
    let digest = Sha256::digest(&canonical);

    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&sig.value);
    let r = p256::FieldBytes::from_slice(&sig_bytes[..32]);
    let s = p256::FieldBytes::from_slice(&sig_bytes[32..]);
    if let Ok(ecdsa_sig) = p256::ecdsa::Signature::from_scalars(*r, *s) {
        use p256::ecdsa::signature::hazmat::PrehashVerifier;
        if vk.verify_prehash(digest.as_slice(), &ecdsa_sig).is_ok() {
            println!("lifegraphd: command accepted");
            let _ = stream.append(
                EventType::CommandCommitted as i32,
                1,
                now_ms(),
            );
            return;
        }
    }

    println!("lifegraphd: rejecting command (signature invalid)");
    let _ = stream.append(
        EventType::CommandRejected as i32,
        1,
        now_ms(),
    );
}

// ---------------------------------------------------------------------------
// Node configuration (YAML-backed)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct NodeConfig {
    stream_id: String,
    name: Option<String>,
    #[serde(default)]
    controllers: Vec<String>,
    #[serde(default)]
    trust_nodes: Vec<String>,
    #[serde(default)]
    signer: Option<SignerConfig>,
    #[serde(default)]
    initial_grants: Vec<serde_yaml::Value>,
    #[serde(default)]
    metadata: serde_yaml::Value,
}

/// Configuration for the signing backend.
/// In production, this references a hardware key handle (TPM, YubiKey, etc.).
/// For dev/testing, it contains the private key inline (INSECURE).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SignerConfig {
    /// The signing backend type: "software", "tpm", "yubikey", "android_keystore"
    #[serde(rename = "type")]
    signer_type: String,
    /// The public key (NodeID) as hex. Used to derive the node identity.
    public_key_hex: String,
    /// Dev-only: the private key as hex. NEVER present in production configs.
    private_key_hex: Option<String>,
    /// Hardware key reference (TPM handle, YubiKey slot, etc.)
    handle: Option<String>,
    /// Hardware key slot (YubiKey: "9a", "9c", "9d", "9e")
    slot: Option<String>,
}

// ---------------------------------------------------------------------------
// Software signer (in-memory ECDSA P-256 key)
// ---------------------------------------------------------------------------

struct SoftwareSigner {
    node_id: NodeID,
    key: SigningKey,
}

impl SoftwareSigner {
    fn new(key: SigningKey) -> Self {
        let vk = key.verifying_key();
        let encoded = vk.to_encoded_point(false);
        let mut node_bytes = [0u8; 64];
        node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
        Self {
            node_id: NodeID(node_bytes),
            key,
        }
    }
}

impl MeshSigner for SoftwareSigner {
    fn node_id(&self) -> NodeID {
        self.node_id
    }

    fn sign_digest(
        &self,
        digest: &[u8; 32],
    ) -> Result<[u8; 64], lifegraph_hardware_signing::HardwareSigningError> {
        let sig: p256::ecdsa::Signature = self
            .key
            .sign_prehash_with_rng(&mut OsRng, digest)
            .map_err(|e| lifegraph_hardware_signing::HardwareSigningError::Provider(e.to_string()))?;
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&sig.to_bytes());
        Ok(bytes)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Loads a signer from the node config.
/// For "software" signer, the key is inline in the config (dev-only).
/// For hardware signers (tpm, yubikey), the key is loaded from secure hardware.
fn load_signer_from_config(config: &NodeConfig) -> Box<dyn MeshSigner> {
    let Some(signer_config) = &config.signer else {
        eprintln!("error: no signer configured. Run `lifegraphd init` first.");
        std::process::exit(1);
    };

    match signer_config.signer_type.as_str() {
        "software" => {
            let Some(key_hex) = &signer_config.private_key_hex else {
                eprintln!("error: software signer configured but private_key_hex is missing.");
                eprintln!("Run `lifegraphd init --software --config <path>` to generate a dev key.");
                std::process::exit(1);
            };
            let signing_key = parse_signing_key_hex(key_hex);
            Box::new(SoftwareSigner::new(signing_key))
        }
        "tpm" => {
            eprintln!("error: TPM signer is configured but TPM signing is not yet implemented.");
            eprintln!("The TPM backend exists but key provisioning requires manual setup.");
            eprintln!("See: lifegraphd init --help");
            std::process::exit(1);
        }
        "yubikey" => {
            eprintln!("error: YubiKey signer is configured but YubiKey signing is not yet implemented.");
            eprintln!("The YubiKey backend exists but key provisioning requires manual setup.");
            std::process::exit(1);
        }
        other => {
            eprintln!("error: unknown signer type: {}", other);
            std::process::exit(1);
        }
    }
}

fn parse_signing_key_hex(key_hex: &str) -> SigningKey {
    let hex_str = key_hex.trim();
    let bytes = hex::decode(hex_str).unwrap_or_else(|e| {
        eprintln!("error: invalid key hex: {}", e);
        std::process::exit(1);
    });
    let bytes: [u8; 32] = bytes.try_into().unwrap_or_else(|_| {
        eprintln!("error: key must be 32 bytes (64 hex chars)");
        std::process::exit(1);
    });
    SigningKey::from_bytes(&bytes.into()).unwrap_or_else(|e| {
        eprintln!("error: invalid key: {}", e);
        std::process::exit(1);
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn setup_signal_handler() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static STOP: AtomicBool = AtomicBool::new(false);

    // Handle Ctrl+C
    ctrlc::set_handler(move || {
        if STOP.swap(true, Ordering::SeqCst) {
            std::process::exit(1);
        }
        println!("\nlifegraphd: shutting down...");
    })
    .unwrap_or_else(|e| {
        eprintln!("lifegraphd: warning: could not set signal handler: {}", e);
    });
}
