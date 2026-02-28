// SPDX-License-Identifier: Apache-2.0
fn main() {
    println!("cargo:rerun-if-changed=proto/local/v1/node_local_bridge.proto");
    println!("cargo:rerun-if-changed=proto/profile/v1/profile.proto");
    println!("cargo:rerun-if-changed=proto/profile/v1/oidc_scopes.proto");
    println!("cargo:rerun-if-changed=proto/tunnel/v1/tunnel_control.proto");

    prost_build::compile_protos(
        &[
            "proto/local/v1/node_local_bridge.proto",
            "proto/profile/v1/profile.proto",
            "proto/profile/v1/oidc_scopes.proto",
            "proto/tunnel/v1/tunnel_control.proto",
        ],
        &["proto"],
    )
    .expect("failed to compile runtime profile protobuf schema");
}
