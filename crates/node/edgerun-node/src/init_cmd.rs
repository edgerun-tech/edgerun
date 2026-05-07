use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use edgerun_crypto::rand_core::RngCore;
use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_protocols::keygen::{generate_node_signing_key, node_id_from_signing_key};
use edgerun_protocols::seal::{generate_seal_key, seal_node_signing_key};
use edgerun_protocols::sign::ProtocolSigner;
use edgerun_protocols::sign_p256::P256ProtocolSigner;
use edgerun_storage::fs::FsEventLog;
use edgerun_storage::DurableStreamWriter;
use edgerun_yubikey::YubiKeySigningKey;

use crate::protocol_signer::MeshProtocolSigner;
use crate::signer::{parse_signing_key_hex, SyncSoftwareSigner};

fn create_stream_or_exit<S>(root: &PathBuf, node_id: NodeID, signer: S)
where
    S: ProtocolSigner,
{
    let events_dir = root.join("events");
    let log = FsEventLog::new(events_dir);
    DurableStreamWriter::new(
        node_id.0,
        signer,
        edgerun_protocols::core_protocol::util::now_unix_millis_i64(),
        log,
    )
    .unwrap_or_else(|e| {
        eprintln!(
            "error: failed to create signed event log at {}: {}",
            root.display(),
            e
        );
        std::process::exit(1);
    });
}

pub fn cmd_init(path: &PathBuf, name: Option<String>, software: bool) {
    let has_tpm = PathBuf::from("/dev/tpmrm0").exists();
    let has_yubikey = check_yubikey_available();

    if !software && !has_tpm && !has_yubikey {
        eprintln!("error: no secure hardware found.");
        eprintln!();
        eprintln!("Available hardware backends:");
        eprintln!(
            "  TPM 2.0:    {} (device: /dev/tpmrm0)",
            if has_tpm { "FOUND" } else { "not found" }
        );
        eprintln!(
            "  YubiKey:    {}",
            if has_yubikey { "FOUND" } else { "not found" }
        );
        eprintln!();
        eprintln!("For development only, you can generate a software key with --software:");
        eprintln!("  edged init --config {} --software", path.display());
        std::process::exit(1);
    }

    let (node_id, signer_kind) = if software {
        eprintln!("WARNING: --software generates an INSECURE local key file.");
        eprintln!("This is for development/testing only. NEVER use in production.");
        eprintln!();
        let (signing_key, identity) = generate_node_signing_key();
        let node_id = NodeID(identity.node_id);
        let key_hex = edgerun_protocols::core_protocol::util::bytes_to_hex(&signing_key.to_bytes());
        fs::create_dir_all(path).unwrap_or_else(|e| {
            eprintln!(
                "error: failed to create data root {}: {}",
                path.display(),
                e
            );
            std::process::exit(1);
        });
        let key_path = path.join("identity.key");
        fs::write(&key_path, key_hex.as_bytes()).unwrap_or_else(|e| {
            eprintln!(
                "error: failed to write local key {}: {}",
                key_path.display(),
                e
            );
            std::process::exit(1);
        });
        create_stream_or_exit(path, node_id, P256ProtocolSigner::new(signing_key));
        (node_id, "software")
    } else if has_tpm {
        eprintln!("Provisioning ECDSA P-256 signing key in TPM 2.0...");
        eprintln!("  TPM device: /dev/tpmrm0");

        // Find an available persistent handle
        let persistent_handle =
            find_available_tpm_handle(0x8100_0001, 0x8100_00FF).unwrap_or_else(|e| {
                eprintln!("error: failed to scan TPM handles: {}", e);
                std::process::exit(1);
            });

        // Create and persist the TPM key using native TPM commands
        let provisioned_key = {
            let mut tpm =
                edgerun_tpm::TpmDevice::new(edgerun_tpm::LinuxTpmDevice::new("/dev/tpmrm0"));
            tpm.create_ecdsa_p256_signing_key(persistent_handle)
                .unwrap_or_else(|e| {
                    eprintln!("error: failed to create TPM signing key: {}", e);
                    std::process::exit(1);
                })
        };

        let x_bytes = &provisioned_key.public_key_bytes[..32];
        let y_bytes = &provisioned_key.public_key_bytes[32..];

        let mut pub_bytes = [0u8; 64];
        pub_bytes[..32].copy_from_slice(x_bytes);
        pub_bytes[32..].copy_from_slice(y_bytes);
        let node_id = NodeID(pub_bytes);

        eprintln!(
            "  Persistent handle: 0x{:08X}",
            provisioned_key.persistent_handle
        );
        eprintln!("  Public key: {}", node_id.to_hex());

        let tpm_key = edgerun_tpm::LinuxTpmSigningKey::new(
            "/dev/tpmrm0",
            edgerun_tpm::TpmHandle(provisioned_key.persistent_handle),
        );
        let adapter = edgerun_hardware_signing::TpmHardwareKeyAdapter::new(tpm_key);
        let mesh_signer = edgerun_hardware_signing::HardwareMeshSigner::new(adapter)
            .unwrap_or_else(|e| {
                eprintln!("error: failed to initialize TPM signer: {}", e);
                std::process::exit(1);
            });
        create_stream_or_exit(
            path,
            node_id,
            MeshProtocolSigner::new(Arc::new(mesh_signer)),
        );
        (node_id, "tpm")
    } else {
        // YubiKey: scan for an existing ECDSA P-256 key in slot 9a (authentication)
        eprintln!("Scanning YubiKey for ECDSA P-256 key in slot 9a...");

        let device = detect_yubikey_device().unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            std::process::exit(1);
        });
        let device_display = format!("{:03}:{:03}", device.bus, device.device);

        let yubikey = edgerun_yubikey::LinuxPcscYubiKey::new(
            device,
            edgerun_yubikey::YubiKeyPivSlot::Authentication,
        );
        let yubi_key_info = yubikey.key_info().unwrap_or_else(|e| {
            eprintln!("error: failed to read YubiKey key info: {}", e);
            eprintln!();
            eprintln!("Make sure a YubiKey with PIV applet is inserted.");
            eprintln!("Use yubico-piv-tool to generate an ECDSA P-256 key in slot 9a:");
            eprintln!("  yubico-piv-tool -a generate -s 9a -A ECCP256");
            eprintln!("  yubico-piv-tool -a verify -a selfsign-certificate -s 9a");
            eprintln!("  yubico-piv-tool -a import-certificate -s 9a");
            std::process::exit(1);
        });

        if yubi_key_info.algorithm != edgerun_yubikey::YubiKeySignatureAlgorithm::EcdsaP256Sha256 {
            eprintln!("error: YubiKey slot 9a does not contain an ECDSA P-256 key.");
            eprintln!("Found algorithm: {:?}", yubi_key_info.algorithm);
            std::process::exit(1);
        }

        if yubi_key_info.public_key.len() != 64 {
            eprintln!(
                "error: YubiKey public key is not 64 bytes (got {})",
                yubi_key_info.public_key.len()
            );
            std::process::exit(1);
        }

        let mut pub_bytes = [0u8; 64];
        pub_bytes.copy_from_slice(&yubi_key_info.public_key);
        let node_id = NodeID(pub_bytes);

        eprintln!("  USB Device: {}", device_display);
        eprintln!("  Slot: 9a");
        eprintln!("  Public key: {}", node_id.to_hex());

        let adapter = edgerun_hardware_signing::YubiKeyHardwareKeyAdapter::new(yubikey);
        let mesh_signer = edgerun_hardware_signing::HardwareMeshSigner::new(adapter)
            .unwrap_or_else(|e| {
                eprintln!("error: failed to initialize YubiKey signer: {}", e);
                std::process::exit(1);
            });
        create_stream_or_exit(
            path,
            node_id,
            MeshProtocolSigner::new(Arc::new(mesh_signer)),
        );
        (node_id, "yubikey")
    };

    let node_name = name.unwrap_or_else(|| format!("edgerun-{}", node_id.short()));
    let stream_id = format!("stream-{}", node_id.short());

    println!();
    println!("Node identity generated:");
    println!("  NodeID:     {}", node_id.to_hex());
    println!("  Short ID:   {}", node_id.short());
    println!("  Stream ID:  {}", stream_id);
    println!("  Name:       {}", node_name);
    if software {
        println!("  Signer:     software (INSECURE — development only)");
        println!();
        println!(
            "WARNING: This is a SOFTWARE KEY. The private key is stored as local key material."
        );
        println!("Do NOT use this key in production.");
    } else if has_tpm {
        println!("  Signer:     TPM 2.0 (ECDSA P-256)");
    } else {
        println!("  Signer:     YubiKey PIV (ECDSA P-256, slot 9a)");
    }
    println!("  Signer:     {}", signer_kind);
    println!("  Event log:  {}", path.join("events").display());
    println!();
    println!("Inspect the node with:");
    println!("  edged status --config {}", path.display());
    println!();

    // Run benchmarks and cache performance certificate
    // TODO: benchmark module not yet implemented in edgerun-core
    // println!("Running performance benchmarks...");
    // let cert = edgerun_protocols::core_protocol::benchmark::run_full_benchmark(node_id.0);
    // ...
}

pub fn check_yubikey_available() -> bool {
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

/// Finds the first YubiKey USB device info.
pub fn detect_yubikey_device() -> Result<edgerun_yubikey::LinuxUsbYubiKeyInfo, String> {
    let devices = edgerun_yubikey::LinuxUsbYubiKey::discover()
        .map_err(|e| format!("YubiKey USB discovery failed: {}", e))?;
    devices
        .into_iter()
        .next()
        .ok_or_else(|| "no YubiKey devices found on USB bus".into())
}

pub fn cmd_init_encrypted(path: &PathBuf, key_path: &PathBuf, name: Option<String>) {
    eprintln!("Generating ECDSA P-256 signing key...");
    let (signing_key, identity) = generate_node_signing_key();
    let node_id = NodeID(identity.node_id);
    let seal_key = generate_seal_key().unwrap_or_else(|e| {
        eprintln!("error: failed to generate seal key: {:?}", e);
        std::process::exit(1);
    });
    let seal_key_hex =
        edgerun_protocols::core_protocol::util::bytes_to_hex(seal_key.expose_secret());

    eprintln!("Sealing key with AES-256-GCM...");
    let encrypted_data = seal_node_signing_key(&signing_key, &seal_key).unwrap_or_else(|e| {
        eprintln!("error: failed to seal signing key: {:?}", e);
        std::process::exit(1);
    });

    eprintln!("Writing encrypted key to: {}", key_path.display());
    std::fs::write(key_path, &encrypted_data).unwrap_or_else(|e| {
        eprintln!("error: failed to write encrypted key file: {}", e);
        std::process::exit(1);
    });

    let node_name = name.unwrap_or_else(|| format!("edgerun-{}", node_id.short()));
    let stream_id = format!("stream-{}", node_id.short());
    create_stream_or_exit(path, node_id, P256ProtocolSigner::new(signing_key));

    println!();
    println!("Node identity generated:");
    println!("  NodeID:     {}", node_id.to_hex());
    println!("  Short ID:   {}", node_id.short());
    println!("  Stream ID:  {}", stream_id);
    println!("  Name:       {}", node_name);
    println!("  Signer:     encrypted (AES-256-GCM with generated seal key)");
    println!("  Key file:   {}", key_path.display());
    println!("  Event log:  {}", path.join("events").display());
    println!();
    println!("To inspect the node, set the generated seal key and run:");
    println!("  export EDGERUN_SEAL_KEY_HEX='{}'", seal_key_hex);
    println!("  edged status --config {}", path.display());
    println!();
    println!("IMPORTANT: Keep the seal key safe. Without it, the node key cannot be recovered.");
}

const PIN_CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

pub fn generate_pairing_pin() -> String {
    let mut pin = String::with_capacity(8);
    for _ in 0..8 {
        let mut byte = [0u8; 1];
        edgerun_crypto::fill_random(&mut byte).expect("random generation failed");
        let idx = (byte[0] as usize) % PIN_CHARSET.len();
        pin.push(PIN_CHARSET[idx] as char);
    }
    pin
}

pub fn cmd_init_provisioned(path: &PathBuf, name: Option<String>, controller: Option<String>) {
    let (signing_key, _) = generate_node_signing_key();
    let node_id = NodeID(node_id_from_signing_key(&signing_key));
    let key_hex = edgerun_protocols::core_protocol::util::bytes_to_hex(&signing_key.to_bytes());

    let pairing_pin = generate_pairing_pin();
    let stream_id = format!("stream-{}", node_id.short());
    let node_name = name.unwrap_or_else(|| format!("edgerun-{}", node_id.short()));
    if let Some(controller) = controller {
        eprintln!("Controller bootstrap input must be recorded by a signed bootstrap contract: {controller}");
    }
    fs::create_dir_all(path).unwrap_or_else(|e| {
        eprintln!(
            "error: failed to create data root {}: {}",
            path.display(),
            e
        );
        std::process::exit(1);
    });
    let key_path = path.join("identity.key");
    fs::write(&key_path, key_hex.as_bytes()).unwrap_or_else(|e| {
        eprintln!(
            "error: failed to write local key {}: {}",
            key_path.display(),
            e
        );
        std::process::exit(1);
    });
    create_stream_or_exit(path, node_id, P256ProtocolSigner::new(signing_key));

    eprintln!();
    eprintln!("Node PUBLIC KEY:");
    eprintln!("  {}", node_id.to_hex());
    eprintln!();
    eprintln!("PAIRING PIN: {}", pairing_pin);
    eprintln!();
    eprintln!("Event log created at: {}", path.join("events").display());
    eprintln!();
    eprintln!("FROM YOUR LAPTOP, run:");
    eprintln!(
        "  edged provision --config {} --pin {}",
        path.display(),
        pairing_pin
    );
    eprintln!();
    eprintln!("The node is now advertising in provisioning mode.");
    eprintln!("Control is bound to the generated node private key.");
    eprintln!("Protect and back up local key material; there is no account recovery path.");
}

pub fn cmd_provision(config_path: &PathBuf, pin: &str, target_addr: Option<String>) {
    let key_hex = fs::read_to_string(config_path.join("identity.key")).unwrap_or_else(|e| {
        eprintln!("error: failed to read local identity key: {}", e);
        std::process::exit(1);
    });
    let signing_key = parse_signing_key_hex(&key_hex);
    let signer = SyncSoftwareSigner::new(signing_key);
    let node_id_hex = signer.node_id().to_hex();

    let target = target_addr.unwrap_or_else(|| "127.0.0.1:35630".to_string());
    eprintln!("Connecting to {}...", target);

    if let Err(e) = provision_sync(&target, pin, &node_id_hex) {
        eprintln!("error: provisioning failed: {}", e);
        std::process::exit(1);
    }

    eprintln!("\nProvisioning accepted. The node is controlled by its generated private key.");
}

fn provision_sync(target: &str, pin: &str, node_id: &str) -> Result<(), String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(target).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    let payload = format!(
        "{{\"type\":\"provision\",\"pin\":\"{}\",\"node_id\":\"{}\"}}",
        pin, node_id
    );

    stream
        .write_all(payload.as_bytes())
        .map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;

    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).map_err(|e| e.to_string())?;

    if n > 0 {
        let response = String::from_utf8_lossy(&buf[..n]);
        eprintln!("Response: {}", response);
    }

    eprintln!("Provisioning request sent.");
    Ok(())
}

/// Scans TPM persistent handles to find the first unused one.
pub fn find_available_tpm_handle(start: u32, end: u32) -> Result<u32, String> {
    let mut tpm = edgerun_tpm::TpmDevice::new(edgerun_tpm::LinuxTpmDevice::new("/dev/tpmrm0"));
    for h in (start..=end).step_by(1) {
        match tpm.read_public(edgerun_tpm::TpmHandle(h)) {
            Ok(_) => continue,
            Err(_) => return Ok(h),
        }
    }
    Err("no free TPM persistent handles in range".into())
}
