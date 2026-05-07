//! edgerun Benchmark Runner
//!
//! Runs the full CPU, memory, storage, and network benchmark suite
//! and prints a unified performance report.
//!
//! Usage:
//!   edgerun-bench              # Full suite + signing comparison
//!   edgerun-bench --sign-only  # Only ECDSA signing comparison

mod sign_bench;

fn main() {
    // Check for --sign-only flag
    let sign_only = std::env::args().any(|a| a == "--sign-only");

    if sign_only {
        sign_bench::run_sign_comparison();
        return;
    }

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║           edgerun Performance Benchmark Suite            ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Generate a fake node ID (we don't sign in bench mode)
    let node_id = [0x00u8; 64];

    println!("Running core benchmarks (CPU, memory, storage)...");
    println!();

    let _cert = edgerun_protocols::core_protocol::benchmark::run_full_benchmark(node_id);

    // Run network benchmarks
    println!("Running network benchmarks...");
    println!();
    let (enc_dec, sign_vrfy, udp) = edgerun_mesh::benchmark::run_mesh_benchmarks();
    let router = edgerun_mesh::router_benchmark::benchmark_router_lookup();

    // Build full certificate
    let full_cert = edgerun_protocols::core_protocol::benchmark::run_full_benchmark_with_network(
        node_id, enc_dec, sign_vrfy, udp, router,
    );

    // Print results
    println!();
    edgerun_protocols::core_protocol::benchmark::print_benchmark_results(&full_cert);

    // Print reference scores and multipliers
    println!();
    println!("=== Multipliers vs Reference ===");
    let cpu_m = full_cert.cpu_core_multiplier();
    let mem_m = full_cert.memory_multiplier();
    let sto_m = full_cert.storage_multiplier();
    println!("  CPU:          {:.4}x", to_f64(cpu_m));
    println!("  Memory:       {:.4}x", to_f64(mem_m));
    println!("  Storage IOPS: {:.4}x", to_f64(sto_m));

    // Reference table
    println!();
    println!("=== Reference Baselines ===");
    println!(
        "  CPU Int:          {} ops/s",
        edgerun_protocols::core_protocol::accounting::REFERENCE_CPU_INT_SCORE
    );
    println!(
        "  CPU Crypto:       {} hashes/s",
        edgerun_protocols::core_protocol::accounting::REFERENCE_CPU_CRYPTO_SCORE
    );
    println!(
        "  Memory BW:        {} MB/s",
        edgerun_protocols::core_protocol::accounting::REFERENCE_MEM_BW_MBPS
    );
    println!(
        "  Memory Lat:       {} ns",
        edgerun_protocols::core_protocol::accounting::REFERENCE_MEM_LATENCY_NS
    );
    println!(
        "  Storage IOPS:     {} ops/s",
        edgerun_protocols::core_protocol::accounting::REFERENCE_STORAGE_IOPS
    );
    println!(
        "  Storage Seq:      {} MB/s",
        edgerun_protocols::core_protocol::accounting::REFERENCE_STORAGE_SEQ_MBPS
    );
    println!(
        "  Storage Events:   {} ops/s",
        edgerun_protocols::core_protocol::accounting::REFERENCE_STORAGE_EVENT_IOPS
    );
    println!(
        "  Storage Blob:     {} ops/s",
        edgerun_protocols::core_protocol::accounting::REFERENCE_STORAGE_BLOB_OPS
    );
    println!(
        "  Storage Object:   {} ops/s",
        edgerun_protocols::core_protocol::accounting::REFERENCE_STORAGE_OBJECT_OPS
    );
    println!(
        "  Net Enc/Dec:      {} ops/s",
        edgerun_protocols::core_protocol::accounting::REFERENCE_NET_FRAME_ENCODE_DECODE_OPS
    );
    println!(
        "  Net Sign/Vrfy:    {} ops/s",
        edgerun_protocols::core_protocol::accounting::REFERENCE_NET_FRAME_SIGN_VERIFY_OPS
    );
    println!(
        "  Net UDP:          {} ops/s",
        edgerun_protocols::core_protocol::accounting::REFERENCE_NET_UDP_THROUGHPUT_OPS
    );
    println!(
        "  Net Router:       {} ops/s",
        edgerun_protocols::core_protocol::accounting::REFERENCE_NET_ROUTER_LOOKUP_OPS
    );

    // Summary
    let duration_ms = (full_cert.benchmark_completed_us - full_cert.benchmark_started_us) / 1000;
    println!();
    println!("Total suite runtime: {} ms", duration_ms);
    println!("Certificate digest: {}", hex_slice(&full_cert.digest));

    // Now run the signing comparison
    println!();
    println!();
    sign_bench::run_sign_comparison();
}

/// Convert FixedPoint16 to f64 for display.
fn to_f64(fp: edgerun_protocols::core_protocol::fixed_point::FixedPoint16) -> f64 {
    fp.to_raw() as f64 / 65536.0
}

fn hex_slice(b: &[u8]) -> String {
    edgerun_encoding::hex::bytes_to_hex(b)
}
