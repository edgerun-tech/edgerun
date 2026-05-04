use prost_build::Config;
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
    ];

    let gen_dir = PathBuf::from("src/gen");
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

    let mut config = Config::new();
    config.bytes(["edgerun.v0.common.Digest", "edgerun.v0.common.Signature"]);
    
    config.compile_protos(
        &proto_files.iter().map(|f| proto_dir.join(f)).collect::<Vec<_>>(),
        &[proto_dir],
    ).expect("Failed to compile protos");
    
    println!("Proto files generated successfully!");
}
