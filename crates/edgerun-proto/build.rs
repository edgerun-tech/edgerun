use std::env;
use std::path::PathBuf;

fn main() {
    let proto_dir = PathBuf::from("../../proto");
    let proto_files = [
        "edgerun/v0/common.proto",
        "edgerun/v0/identity.proto",
        "edgerun/v0/trust.proto",
        "edgerun/v0/capability.proto",
        "edgerun/v0/capability_runtime.proto",
        "edgerun/v0/stream.proto",
        "edgerun/v0/object.proto",
        "edgerun/v0/access.proto",
        "edgerun/v0/network.proto",
        "edgerun/v0/ui.proto",
        "edgerun/v0/app.proto",
        "edgerun/v0/appabi.proto",
        "edgerun/wallet/v0/exchange.proto",
    ];

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_else(|_| "src/gen".to_string()));
    let gen_dir = PathBuf::from("src/gen");

    // Create gen directory structure
    std::fs::create_dir_all(&gen_dir).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/v0")).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/v0/stream")).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/v0/app")).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/v0/capability")).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/v0/capability_runtime")).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/v0/common")).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/v0/trust")).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/v0/access")).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/v0/network")).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/v0/object")).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/v0/ui")).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/v0/appabi")).ok();
    std::fs::create_dir_all(gen_dir.join("edgerun/wallet/v0")).ok();

    // Build prost config
    let mut config = prost_build::Config::new();

    // Compile protos
    config
        .compile_protos(
            &proto_files.map(|f| proto_dir.join(f)),
            &[proto_dir.clone()],
        )
        .expect("Failed to compile protos");

    // Move generated files to proper locations
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    for entry in std::fs::read_dir(&out_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            let target_path = if file_name.contains("stream") {
                gen_dir.join("edgerun/v0/stream").join(file_name)
            } else if file_name.contains("appabi") {
                gen_dir.join("edgerun/v0/appabi").join(file_name)
            } else if file_name.contains("app.") {
                gen_dir.join("edgerun/v0/app").join(file_name)
            } else if file_name.contains("capability_runtime") {
                gen_dir.join("edgerun/v0/capability_runtime").join(file_name)
            } else if file_name.contains("capability") && !file_name.contains("capability_runtime") {
                gen_dir.join("edgerun/v0/capability").join(file_name)
            } else if file_name.contains("common") {
                gen_dir.join("edgerun/v0/common").join(file_name)
            } else if file_name.contains("trust") {
                gen_dir.join("edgerun/v0/trust").join(file_name)
            } else if file_name.contains("access") {
                gen_dir.join("edgerun/v0/access").join(file_name)
            } else if file_name.contains("network") {
                gen_dir.join("edgerun/v0/network").join(file_name)
            } else if file_name.contains("object") {
                gen_dir.join("edgerun/v0/object").join(file_name)
            } else if file_name.contains("ui") {
                gen_dir.join("edgerun/v0/ui").join(file_name)
            } else if file_name.contains("identity") {
                gen_dir.join("edgerun/v0").join(file_name)
            } else if file_name.contains("exchange") || file_name.contains("wallet") {
                gen_dir.join("edgerun/wallet/v0").join(file_name)
            } else {
                gen_dir.join(file_name)
            };
            std::fs::copy(&path, &target_path).expect("Failed to copy generated file");
        }
    }

    println!("cargo:rerun-if-changed=../../proto");
}
