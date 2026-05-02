//! Build script — mirrors `buf.gen.yaml` prost output into `src/gen/`.
//!
//! Both `buf generate` and `cargo build -p edgerun-proto` produce the same
//! flat file layout so there is exactly one canonical generated type per proto.

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

    let gen_dir = PathBuf::from("src/gen");
    std::fs::create_dir_all(&gen_dir).ok();

    let mut config = prost_build::Config::new();
    config.out_dir(&gen_dir);

    config
        .compile_protos(
            &proto_files.map(|f| proto_dir.join(f)),
            &[proto_dir.clone()],
        )
        .expect("Failed to compile protos");

    println!("cargo:rerun-if-changed=../../proto");
}
