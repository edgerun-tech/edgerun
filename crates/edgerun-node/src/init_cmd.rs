use std::fs;
use std::path::PathBuf;

use edgerun_crypto::rand_core::RngCore;
use edgerun_hardware_signing::NodeID;
use edgerun_yubikey::YubiKeySigningKey;

pub fn cmd_init(path: &PathBuf, name: Option<String>, software: bool) {
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
        eprintln!("  edgerund init --config {} --software", path.display());
        std::process::exit(1);
    }

    let (node_id, signer_block) = if software {
        eprintln!("WARNING: --software generates an INSECURE key stored in the config file.");
        eprintln!("This is for development/testing only. NEVER use in production.");
        eprintln!();
        let mut key_bytes = [0u8; 32];
        edgerun_crypto::getrandom::fill(&mut key_bytes).expect("getrandom failed");
        let signing_key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&key_bytes.into()).unwrap_or_else(|e| {
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
        let persistent_handle = find_available_tpm_handle(0x8100_0001, 0x8100_00FF)
            .unwrap_or_else(|e| {
                eprintln!("error: failed to scan TPM handles: {}", e);
                std::process::exit(1);
            });

        // Create and persist the TPM key using native TPM commands
        let provisioned_key = {
            let mut tpm = edgerun_tpm::TpmDevice::new(edgerun_tpm::LinuxTpmDevice::new("/dev/tpmrm0"));
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

        eprintln!("  Persistent handle: 0x{:08X}", provisioned_key.persistent_handle);
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

        let yubikey = edgerun_yubikey::LinuxPcscYubiKey::new(device, edgerun_yubikey::YubiKeyPivSlot::Authentication);
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
            eprintln!("error: YubiKey public key is not 64 bytes (got {})", yubi_key_info.public_key.len());
            std::process::exit(1);
        }

        let mut pub_bytes = [0u8; 64];
        pub_bytes.copy_from_slice(&yubi_key_info.public_key);
        let node_id = NodeID(pub_bytes);

        eprintln!("  USB Device: {}", device_display);
        eprintln!("  Slot: 9a");
        eprintln!("  Public key: {}", node_id.to_hex());

        let signer_block = format!(
            r#"signer:
  type: "yubikey"
  handle: "9a"
"#,
        );
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
    println!("  edgerund run --config {} --listen 0.0.0.0:8080", path.display());
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
