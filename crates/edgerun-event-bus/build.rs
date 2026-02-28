// SPDX-License-Identifier: Apache-2.0
fn main() {
    println!("cargo:rerun-if-changed=proto/edge_internal/v1/event_bus.proto");
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_protos(&["proto/edge_internal/v1/event_bus.proto"], &["proto"])
        .expect("failed to compile edge-internal gRPC schemas");
}
