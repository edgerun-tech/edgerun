use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let linker_script = match target_arch.as_str() {
        "xtensa" => manifest_dir.join("linker-xtensa-esp32s3.ld"),
        _ => manifest_dir.join("linker.ld"),
    };

    if target_os == "none" {
        println!("cargo:rerun-if-changed={}", linker_script.display());
        println!(
            "cargo:rustc-link-arg-bin=edgerun-unikernel=-T{}",
            linker_script.display()
        );
    }

    if target_arch == "xtensa" && target_os == "none" {
        println!("cargo:rustc-link-arg-bin=edgerun-unikernel=-nostartfiles");
        if std::env::var_os("CARGO_FEATURE_ESP32S3_WIFI_BLOB").is_some() {
            let root = manifest_dir
                .ancestors()
                .nth(2)
                .expect("workspace root")
                .to_path_buf();
            let idf_components = root.join(
                "devices/edgerun-tcl-usb-ap-bridge/.embuild/espressif/esp-idf/v5.5.3/components",
            );
            let esp32s3_rom_ld = idf_components.join("esp_rom/esp32s3/ld/esp32s3.rom.ld");
            let esp32s3_rom_api_ld = idf_components.join("esp_rom/esp32s3/ld/esp32s3.rom.api.ld");
            println!("cargo:rerun-if-changed={}", esp32s3_rom_ld.display());
            println!("cargo:rerun-if-changed={}", esp32s3_rom_api_ld.display());
            println!(
                "cargo:rustc-link-arg-bin=edgerun-unikernel=-T{}",
                esp32s3_rom_ld.display()
            );
            println!(
                "cargo:rustc-link-arg-bin=edgerun-unikernel=-T{}",
                esp32s3_rom_api_ld.display()
            );
            for lib_dir in [
                "esp_wifi/lib/esp32s3",
                "esp_phy/lib/esp32s3",
                "esp_coex/lib/esp32s3",
            ] {
                println!(
                    "cargo:rustc-link-search=native={}",
                    idf_components.join(lib_dir).display()
                );
            }
            for lib in ["core", "net80211", "pp", "phy", "coexist"] {
                println!("cargo:rustc-link-lib=static={lib}");
            }
        }
    }
}
