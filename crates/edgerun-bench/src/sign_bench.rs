//! ECDSA P-256 signing benchmark: Software vs TPM vs YubiKey vs CPU Hash
//!
//! Compares three signing backends on the same input:
//! - **Software**: Pure Rust P-256 (edgerun-p256 crate)
//! - **TPM 2.0**: Hardware signing via `/dev/tpmrm0`
//! - **CPU Hash**: SHA-256 hash only (the hash that feeds ECDSA)
//!
//! Usage: `cargo run --bin edgerun-bench -- --sign-only`

use std::time::{Duration, Instant};

// ===========================================================================
// Software ECDSA P-256 signing
// ===========================================================================

fn benchmark_software_sign(ops_target: u64) -> (u64, Duration) {
    use edgerun_crypto::p256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};

    // Generate a fresh key
    let mut key_bytes = [0u8; 32];
    edgerun_crypto::fill_random(&mut key_bytes).expect("random generation failed");
    let signing_key = SigningKey::from_bytes((&key_bytes).into()).expect("valid key");

    let message = [0xDEu8; 32];
    let start = Instant::now();
    let mut count: u64 = 0;

    while count < ops_target {
        let sig: Signature = signing_key.sign_prehash(&message).expect("sign");
        std::hint::black_box(sig.to_bytes());
        count += 1;
    }

    (count, start.elapsed())
}

// ===========================================================================
// TPM 2.0 ECDSA P-256 signing
// ===========================================================================

fn benchmark_tpm_sign(ops_target: u64) -> Option<(u64, Duration)> {
    use edgerun_hardware_signing::{HardwareSigningKey, TpmHardwareKeyAdapter};
    use edgerun_tpm::{LinuxTpmSigningKey, TpmHandle};

    // First, try to find an existing persistent key handle
    let handle = find_existing_tpm_key().or_else(provision_tpm_key);
    let handle = handle?;

    let tpm_key = LinuxTpmSigningKey::new("/dev/tpmrm0", TpmHandle(handle));
    let adapter = TpmHardwareKeyAdapter::new(tpm_key);

    let message = [0xDEu8; 32];
    let start = Instant::now();
    let mut count: u64 = 0;

    while count < ops_target {
        match adapter.sign_message(&message) {
            Ok(sig) => {
                std::hint::black_box(&sig);
                count += 1;
            }
            Err(_) => break,
        }
    }

    Some((count, start.elapsed()))
}

fn find_existing_tpm_key() -> Option<u32> {
    let mut device = edgerun_tpm::TpmDevice::new(edgerun_tpm::LinuxTpmDevice::new("/dev/tpmrm0"));
    // Scan persistent handles 0x8100_0001..0x8100_00FF
    (0x8100_0001..=0x8100_00FF)
        .step_by(1)
        .find(|&h| device.read_public(edgerun_tpm::TpmHandle(h)).is_ok())
}

fn provision_tpm_key() -> Option<u32> {
    let mut device = edgerun_tpm::TpmDevice::new(edgerun_tpm::LinuxTpmDevice::new("/dev/tpmrm0"));
    // Find a free handle
    let mut free_handle = None;
    for h in (0x8100_0001..=0x8100_00FF).step_by(1) {
        if device.read_public(edgerun_tpm::TpmHandle(h)).is_err() {
            free_handle = Some(h);
            break;
        }
    }
    let handle = free_handle?;

    // Create the key
    let provisioned = device.create_ecdsa_p256_signing_key(handle).ok()?;
    eprintln!(
        "  [TPM] Provisioned ECDSA P-256 key at handle 0x{:08X}",
        provisioned.persistent_handle
    );
    Some(provisioned.persistent_handle)
}

// ===========================================================================
// SHA-256 only (the hash that feeds ECDSA)
// ===========================================================================

fn benchmark_sha256_only(ops_target: u64) -> (u64, Duration) {
    use edgerun_protocols::core_protocol::crypto::sha256;

    let message = [0xDEu8; 256];
    let start = Instant::now();
    let mut count: u64 = 0;

    while count < ops_target {
        let hash = sha256(&message);
        std::hint::black_box(hash);
        count += 1;
    }

    (count, start.elapsed())
}

// ===========================================================================
// Full comparison runner
// ===========================================================================

const OPS_TARGET: u64 = 100; // TPM is slow, don't do thousands

pub fn run_sign_comparison() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║       ECDSA P-256 Signing Backend Comparison             ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Running {OPS_TARGET} sign operations per backend...");
    println!();

    // Software P-256
    println!("  Software (P-256 Rust) ...");
    let (sw_count, sw_dur) = benchmark_software_sign(OPS_TARGET);
    let sw_ops = if sw_dur.as_micros() > 0 {
        sw_count.saturating_mul(1_000_000) / sw_dur.as_micros() as u64
    } else {
        0
    };
    let sw_per_op = if sw_count > 0 {
        sw_dur.as_nanos() as f64 / sw_count as f64
    } else {
        0.0
    };
    println!(
        "    → {} ops/s ({:.2} µs per sign)",
        sw_ops,
        sw_per_op / 1000.0
    );

    // SHA-256 only
    println!("  SHA-256 only (CPU) ...");
    let (hash_count, hash_dur) = benchmark_sha256_only(OPS_TARGET * 1000);
    let hash_ops = if hash_dur.as_micros() > 0 {
        hash_count.saturating_mul(1_000_000) / hash_dur.as_micros() as u64
    } else {
        0
    };
    let hash_per_op = if hash_count > 0 {
        hash_dur.as_nanos() as f64 / hash_count as f64
    } else {
        0.0
    };
    println!("    → {} ops/s ({:.2} ns per hash)", hash_ops, hash_per_op);

    // TPM
    println!("  TPM 2.0 (/dev/tpmrm0) ...");
    match benchmark_tpm_sign(OPS_TARGET) {
        Some((tpm_count, tpm_dur)) => {
            let tpm_ops = if tpm_dur.as_micros() > 0 {
                tpm_count.saturating_mul(1_000_000) / tpm_dur.as_micros() as u64
            } else {
                0
            };
            let tpm_per_op = if tpm_count > 0 {
                tpm_dur.as_nanos() as f64 / tpm_count as f64
            } else {
                0.0
            };
            println!(
                "    → {} ops/s ({:.2} µs per sign)",
                tpm_ops,
                tpm_per_op / 1000.0
            );

            // Comparison table
            println!();
            println!("  ┌──────────────────┬───────────┬─────────────┬─────────────┐");
            println!("  │ Backend          │ ops/s     │ µs/sign     │ vs Software │");
            println!("  ├──────────────────┼───────────┼─────────────┼─────────────┤");
            println!(
                "  │ Software P-256   │ {:>9} │ {:>9.2} │      1.00x  │",
                sw_ops,
                sw_per_op / 1000.0
            );
            println!(
                "  │ SHA-256 (CPU)    │ {:>9} │ {:>5.2} ns  │    {:.3}x  │",
                hash_ops,
                hash_per_op,
                hash_per_op / (sw_per_op)
            );
            println!(
                "  │ TPM 2.0          │ {:>9} │ {:>9.2} │   {:>6.2}x  │",
                tpm_ops,
                tpm_per_op / 1000.0,
                tpm_per_op / sw_per_op
            );
            println!("  └──────────────────┴───────────┴─────────────┴─────────────┘");
        }
        None => {
            println!("    → SKIPPED (no TPM access or key provisioning failed)");
        }
    }
}
