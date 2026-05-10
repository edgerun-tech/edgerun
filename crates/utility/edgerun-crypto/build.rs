fn main() {
    println!("cargo:rustc-check-cfg=cfg(curve25519_dalek_bits, values(\"32\", \"64\"))");
    println!("cargo:rustc-check-cfg=cfg(curve25519_dalek_backend, values(\"serial\", \"simd\"))");
    println!("cargo:rustc-check-cfg=cfg(nightly)");
    println!("cargo:rustc-check-cfg=cfg(allow_unused_unsafe)");
    println!("cargo:rustc-check-cfg=cfg(curve25519_dalek_upstream_tests)");
    println!("cargo:rustc-check-cfg=cfg(ed25519_dalek_upstream_tests)");
    println!("cargo:rustc-check-cfg=cfg(has_i128)");
    println!("cargo:rustc-check-cfg=cfg(assert_no_panic)");
    println!("cargo:rustc-check-cfg=cfg(arch_enabled)");
    println!("cargo:rustc-check-cfg=cfg(f16_enabled)");
    println!("cargo:rustc-check-cfg=cfg(f128_enabled)");
    println!("cargo:rustc-check-cfg=cfg(intrinsics_enabled)");
    println!("cargo:rustc-check-cfg=cfg(optimizations_enabled)");
    println!("cargo:rustc-check-cfg=cfg(x86_no_sse)");
    println!("cargo:rustc-check-cfg=cfg(num_bigint_upstream_tests)");

    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_pointer_width = std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap_or_default();
    let bits = if target_pointer_width == "64" {
        "64"
    } else {
        "32"
    };
    // Benchmarked in May 2026: the x86_64 SIMD Curve25519 backend made Ed25519
    // signing roughly 25x slower than the serial backend in this crate, while
    // P-256 performance was unchanged. Keep Curve25519 on serial by default.
    let backend = "serial";

    println!("cargo:rustc-cfg=curve25519_dalek_bits=\"{bits}\"");
    println!("cargo:rustc-cfg=curve25519_dalek_backend=\"{backend}\"");
    if target_os != "none" {
        println!("cargo:rustc-cfg=has_i128");
    }
    println!("cargo:rustc-cfg=arch_enabled");

    let target_features = std::env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    if target_arch == "x86" && !target_features.split(',').any(|feature| feature == "sse") {
        println!("cargo:rustc-cfg=x86_no_sse");
    }
}
