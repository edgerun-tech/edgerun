use std::fs;
use std::path::PathBuf;

use edgerun_crypto::rand_core::RngCore;
use edgerun_hardware_signing::NodeID;
use edgerun_yubikey::YubiKeySigningKey;

use crate::config::{parse_config, NodeConfig};

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
        eprintln!("  edgerund init --config {} --software", path.display());
        std::process::exit(1);
    }

    let (node_id, signer_block) = if software {
        eprintln!("WARNING: --software generates an INSECURE key stored in the config file.");
        eprintln!("This is for development/testing only. NEVER use in production.");
        eprintln!();
        let mut key_bytes = [0u8; 32];
        edgerun_crypto::fill_random(&mut key_bytes).expect("random generation failed");
        let signing_key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&key_bytes.into())
            .unwrap_or_else(|e| {
                eprintln!("error: failed to create signing key: {}", e);
                std::process::exit(1);
            });
        let verifying_key = signing_key.verifying_key();
        let encoded = verifying_key.to_encoded_point(false);
        let mut node_id_bytes = [0u8; 64];
        node_id_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
        let node_id = NodeID(node_id_bytes);
        let key_hex = edgerun_core::util::bytes_to_hex(&signing_key.to_bytes());
        let signer_block = format!(
            r#"signer:
  type: "software"
  public_key_hex: "{node_id_hex}"
  private_key_hex: "{key_hex}"
"#,
            node_id_hex = node_id.to_hex(),
        );
        (node_id, signer_block)
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

        let signer_block = format!(
            r#"signer:
  type: "tpm"
  handle: "0x{handle:08X}"
"#,
            handle = provisioned_key.persistent_handle,
        );
        (node_id, signer_block)
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

        let signer_block = r#"signer:
  type: "yubikey"
  handle: "9a"
"#
        .to_string();
        (node_id, signer_block)
    };

    let node_name = name.unwrap_or_else(|| format!("edgerun-{}", node_id.short()));
    let stream_id = format!("stream-{}", node_id.short());

    let config_yaml = format!(
        r#"# edgerun Node Configuration
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

    println!();
    println!("Node identity generated:");
    println!("  NodeID:     {}", node_id.to_hex());
    println!("  Short ID:   {}", node_id.short());
    println!("  Stream ID:  {}", stream_id);
    println!("  Name:       {}", node_name);
    if software {
        println!("  Signer:     software (INSECURE — development only)");
        println!();
        println!("WARNING: This is a SOFTWARE KEY. The private key is stored in the config file.");
        println!("Do NOT use this key in production.");
    } else if has_tpm {
        println!("  Signer:     TPM 2.0 (ECDSA P-256)");
    } else {
        println!("  Signer:     YubiKey PIV (ECDSA P-256, slot 9a)");
    }
    println!("  Config:     {}", path.display());
    println!();
    println!("Start the node with:");
    println!(
        "  edgerund run --config {} --listen 0.0.0.0:8080",
        path.display()
    );
    println!();

    // Run benchmarks and cache performance certificate
    println!("Running performance benchmarks...");
    let cert = edgerun_core::benchmark::run_full_benchmark(node_id.0);
    let data_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let cert_path = data_dir.join("perf_cert.bin");
    if let Err(e) = std::fs::write(&cert_path, cert.to_bytes()) {
        eprintln!("warning: failed to cache perf cert: {}", e);
    } else {
        let mult = cert.cpu_core_multiplier().to_raw() as f64 / 65536.0;
        println!("  CPU:      {:.2}x reference", mult);
        println!("  Mem BW:   {} MB/s", cert.mem_bandwidth_mbps);
        println!("  Mem Lat:  {} ns", cert.mem_latency_ns);
        println!("  Stor IOPS: {}", cert.storage_random_iops);
        println!("  Cert:     {}", cert_path.display());
    }
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

pub fn cmd_init_encrypted(
    path: &PathBuf,
    key_path: &PathBuf,
    name: Option<String>,
    passphrase: Option<String>,
) {
    let passphrase = passphrase.unwrap_or_else(|| {
        eprintln!("error: --passphrase is required for encrypted key generation");
        eprintln!("Usage: edgerund init-encrypted --config <path> --key-file <path> --passphrase <passphrase>");
        std::process::exit(1);
    });

    if passphrase.len() < 8 {
        eprintln!("error: passphrase must be at least 8 characters");
        std::process::exit(1);
    }

    eprintln!("Generating ECDSA P-256 signing key...");
    let mut key_bytes = [0u8; 32];
    edgerun_crypto::fill_random(&mut key_bytes).expect("random generation failed");
    let signing_key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&key_bytes.into())
        .unwrap_or_else(|e| {
            eprintln!("error: failed to create signing key: {}", e);
            std::process::exit(1);
        });

    let verifying_key = signing_key.verifying_key();
    let encoded = verifying_key.to_encoded_point(false);
    let mut node_id_bytes = [0u8; 64];
    node_id_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
    let node_id = NodeID(node_id_bytes);

    eprintln!("Encrypting key with AES-256-GCM (PBKDF2 100k iterations)...");
    let encrypted_data = edgerun_crypto::encrypt_signing_key(&signing_key, &passphrase);

    eprintln!("Writing encrypted key to: {}", key_path.display());
    std::fs::write(key_path, &encrypted_data).unwrap_or_else(|e| {
        eprintln!("error: failed to write encrypted key file: {}", e);
        std::process::exit(1);
    });

    let signer_block = format!(
        r#"signer:
  type: "encrypted"
  public_key_hex: "{node_id_hex}"
  encrypted_key_path: "{key_path_str}"
  passphrase_env: "EDGERUN_KEY_PASSPHRASE"
"#,
        node_id_hex = node_id.to_hex(),
        key_path_str = key_path.display(),
    );

    let node_name = name.unwrap_or_else(|| format!("edgerun-{}", node_id.short()));
    let stream_id = format!("stream-{}", node_id.short());

    let config_yaml = format!(
        r#"# edgerun Node Configuration
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

    println!();
    println!("Node identity generated:");
    println!("  NodeID:     {}", node_id.to_hex());
    println!("  Short ID:   {}", node_id.short());
    println!("  Stream ID:  {}", stream_id);
    println!("  Name:       {}", node_name);
    println!("  Signer:     encrypted (AES-256-GCM + PBKDF2)");
    println!("  Key file:   {}", key_path.display());
    println!("  Config:     {}", path.display());
    println!();
    println!("To start the node, set the passphrase and run:");
    println!("  export EDGERUN_KEY_PASSPHRASE='{}'", passphrase);
    println!(
        "  edgerund run --config {} --listen 0.0.0.0:8080",
        path.display()
    );
    println!();
    println!(
        "IMPORTANT: Keep the key file safe. Without the passphrase, the key cannot be recovered."
    );
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
    let mut key_bytes = [0u8; 32];
    edgerun_crypto::fill_random(&mut key_bytes).expect("random generation failed");
    let signing_key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&key_bytes.into())
        .unwrap_or_else(|e| {
            eprintln!("error: failed to create signing key: {}", e);
            std::process::exit(1);
        });
    let verifying_key = signing_key.verifying_key();
    let encoded = verifying_key.to_encoded_point(false);
    let mut node_id_bytes = [0u8; 64];
    node_id_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
    let node_id = NodeID(node_id_bytes);

    let pairing_pin = generate_pairing_pin();
    let stream_id = format!("stream-{}", node_id.short());
    let node_name = name.unwrap_or_else(|| format!("edgerun-{}", node_id.short()));
    let controller_list = controller.map(|c| vec![c]).unwrap_or_default();
    let controller_str = if controller_list.is_empty() {
        String::new()
    } else {
        format!(
            "controllers: [{}]",
            controller_list
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let signer_block = format!(
        r#"signer:
  type: "provisioned"
  public_key_hex: "{node_id_hex}"
  state: "provisioning"
  pairing_pin: "{pin}"
  passphrase_env: "EDGERUN_KEY_PASSPHRASE"
"#,
        node_id_hex = node_id.to_hex(),
        pin = pairing_pin,
    );

    let config_yaml = format!(
        r#"# edgerun Node Configuration (provisioning mode)
# Run `edgerund run --config <config>` to start in provisioning mode
stream_id: "{stream_id}"
name: "{node_name}"
{controller_config}
# TODO: add more configuration here
# trust_nodes: []
# allowed_peers: []
# bootstrap_peers: []
{signer_block}
metadata:
  environment: "production"
"#,
        stream_id = stream_id,
        node_name = node_name,
        controller_config = controller_str,
        signer_block = signer_block,
    );

    fs::write(path, &config_yaml).unwrap_or_else(|e| {
        eprintln!("error: failed to write config to {}: {}", path.display(), e);
        std::process::exit(1);
    });

    eprintln!();
    eprintln!("Node PUBLIC KEY:");
    eprintln!("  {}", node_id.to_hex());
    eprintln!();
    eprintln!("PAIRING PIN: {}", pairing_pin);
    eprintln!();
    eprintln!("Configuration saved to: {}", path.display());
    eprintln!();
    eprintln!("FROM YOUR LAPTOP, run:");
    eprintln!(
        "  edgerund provision --config {} --pin {}",
        path.display(),
        pairing_pin
    );
    eprintln!();
    eprintln!("The node is now advertising in provisioning mode.");
    eprintln!("Once provisioned, the password will be required on boot.");
}

pub fn cmd_provision(
    config_path: &PathBuf,
    pin: &str,
    password: Option<String>,
    target_addr: Option<String>,
) {
    let yaml = fs::read_to_string(config_path).unwrap_or_else(|e| {
        eprintln!("error: failed to read config: {}", e);
        std::process::exit(1);
    });
    let config = parse_config(&yaml).unwrap_or_else(|e| {
        eprintln!("error: invalid config: {}", e);
        std::process::exit(1);
    });

    let Some(signer_config) = &config.signer else {
        eprintln!("error: no signer in config");
        std::process::exit(1);
    };

    if signer_config.signer_type != "provisioned" {
        eprintln!("error: signer type must be 'provisioned'");
        std::process::exit(1);
    }

    let Some(config_pin) = &signer_config.pairing_pin else {
        eprintln!("error: no pairing pin in config");
        std::process::exit(1);
    };

    if pin != config_pin {
        eprintln!("error: PIN mismatch");
        std::process::exit(1);
    }

    let passphrase = password.unwrap_or_else(|| {
        eprintln!("Enter password: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("read failed");
        input.trim().to_string()
    });

    if passphrase.len() < 8 {
        eprintln!("error: password must be at least 8 characters");
        std::process::exit(1);
    }

    let target = target_addr.unwrap_or_else(|| "127.0.0.1:35630".to_string());
    eprintln!("Connecting to {}...", target);

    if let Err(e) = provision_sync(&target, pin, &passphrase, &signer_config.public_key_hex) {
        eprintln!("error: provisioning failed: {}", e);
        std::process::exit(1);
    }

    eprintln!("\nPassword will be required on every boot.");
}

fn provision_sync(target: &str, pin: &str, passphrase: &str, node_id: &str) -> Result<(), String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(target).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    let payload = format!(
        "{{\"type\":\"provision\",\"pin\":\"{}\",\"password\":\"{}\",\"node_id\":\"{}\"}}",
        pin, passphrase, node_id
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

pub fn cmd_unlock(config_path: &PathBuf, password: Option<String>, target_addr: Option<String>) {
    let yaml = fs::read_to_string(config_path).unwrap_or_else(|e| {
        eprintln!("error: failed to read config: {}", e);
        std::process::exit(1);
    });
    let config = parse_config(&yaml).unwrap_or_else(|e| {
        eprintln!("error: invalid config: {}", e);
        std::process::exit(1);
    });

    let Some(signer_config) = &config.signer else {
        eprintln!("error: no signer in config");
        std::process::exit(1);
    };

    let passphrase = password.unwrap_or_else(|| {
        eprintln!("Enter password: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("read failed");
        input.trim().to_string()
    });

    if passphrase.len() < 8 {
        eprintln!("error: password must be at least 8 characters");
        std::process::exit(1);
    }

    let target = target_addr.unwrap_or_else(|| "127.0.0.1:35630".to_string());
    eprintln!("Sending unlock to {}...", target);

    if let Err(e) = unlock_sync(&target, &passphrase, &signer_config.public_key_hex) {
        eprintln!("error: unlock failed: {}", e);
        std::process::exit(1);
    }

    eprintln!("Unlock request sent.");
}

fn unlock_sync(target: &str, passphrase: &str, node_id: &str) -> Result<(), String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(target).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    let payload = format!(
        "{{\"type\":\"unlock\",\"password\":\"{}\",\"node_id\":\"{}\"}}",
        passphrase, node_id
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
