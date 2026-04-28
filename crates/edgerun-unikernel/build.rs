use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let linker_script = match target_arch.as_str() {
        "xtensa" => manifest_dir.join("linker-xtensa-esp32s3.ld"),
        _ => manifest_dir.join("linker.ld"),
    };

    println!("cargo:rerun-if-changed={}", linker_script.display());
    println!(
        "cargo:rustc-link-arg-bin=edgerun-unikernel=-T{}",
        linker_script.display()
    );
    if target_arch == "xtensa" {
        println!("cargo:rustc-link-arg-bin=edgerun-unikernel=-nostartfiles");
    }
}
