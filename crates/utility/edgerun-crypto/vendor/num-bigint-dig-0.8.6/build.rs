fn main() {
    println!("cargo:rustc-check-cfg=cfg(has_i128)");
    println!("cargo:rustc-cfg=has_i128");
    println!("cargo:rustc-check-cfg=cfg(assert_no_panic)");
    println!("cargo:rustc-check-cfg=cfg(arch_enabled)");
    println!("cargo:rustc-check-cfg=cfg(f16_enabled)");
    println!("cargo:rustc-check-cfg=cfg(f128_enabled)");
    println!("cargo:rustc-check-cfg=cfg(intrinsics_enabled)");
    println!("cargo:rustc-check-cfg=cfg(optimizations_enabled)");
    println!("cargo:rustc-check-cfg=cfg(x86_no_sse)");

    if std::env::var_os("CARGO_FEATURE_LIBM").is_some() {
        println!("cargo:rustc-cfg=arch_enabled");
    }

    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_features = std::env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    if target_arch == "x86" && !target_features.split(',').any(|feature| feature == "sse") {
        println!("cargo:rustc-cfg=x86_no_sse");
    }
}
