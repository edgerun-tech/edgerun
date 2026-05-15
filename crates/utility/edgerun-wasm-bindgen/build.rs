use std::env;

macro_rules! deprecated_crate_feature {
    ($name:literal) => {
        #[cfg(feature = $name)]
        {
            println!("cargo:warning=The `{}` feature is deprecated and will be removed in the next major version.", $name);
        }
    };
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    deprecated_crate_feature!("msrv");
    deprecated_crate_feature!("xxx_debug_only_print_generated_code");

    println!("cargo:rustc-check-cfg=cfg(wbg_diagnostic)");

    println!("cargo:rustc-cfg=wbg_diagnostic");

    let target_arch = env::var_os("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_os = env::var_os("CARGO_CFG_TARGET_OS").unwrap();

    let target_features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    let target_features: Vec<_> = target_features.split(',').map(str::trim).collect();

    println!("cargo:rustc-check-cfg=cfg(wbg_reference_types)");

    if target_features.contains(&"reference-types") {
        println!("cargo:rustc-cfg=wbg_reference_types");
    }

    if target_arch == "wasm32" && target_os == "emscripten" {
        // Emscripten uses emcc to handle the linking and it will deadcode elimainte __instance_terminated
        // which causes the test on Emscripten to fail to build.
        println!("cargo:rustc-link-arg=-Wl,--export=__instance_terminated");
    }
}
