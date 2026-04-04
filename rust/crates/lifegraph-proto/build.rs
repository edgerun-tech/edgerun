use std::path::PathBuf;

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .ancestors()
        .nth(3)
        .expect("workspace repo root");
    let proto_root = repo_root.join("proto");

    let files = [
        "lifegraph/v0/common.proto",
        "lifegraph/v0/identity.proto",
        "lifegraph/v0/trust.proto",
        "lifegraph/v0/stream.proto",
        "lifegraph/v0/object.proto",
        "lifegraph/v0/access.proto",
        "lifegraph/v0/capability.proto",
        "lifegraph/v0/capability_runtime.proto",
        "lifegraph/v0/network.proto",
    ];

    for file in &files {
        println!("cargo:rerun-if-changed={}", proto_root.join(file).display());
    }

    let protos: Vec<PathBuf> = files.iter().map(|f| proto_root.join(f)).collect();
    prost_build::Config::new()
        .compile_protos(&protos, &[proto_root])
        .expect("compile protos");
}
