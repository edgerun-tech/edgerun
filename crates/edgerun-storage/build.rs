// SPDX-License-Identifier: GPL-2.0-only
fn main() {
    println!("cargo:rerun-if-changed=proto/event_bus/v1/event_bus.proto");
    println!("cargo:rerun-if-changed=proto/scheduler/v1/scheduler.proto");
    println!("cargo:rerun-if-changed=proto/chain/v1/chain.proto");
    println!("cargo:rerun-if-changed=proto/storage/v1/storage.proto");
    prost_build::compile_protos(
        &[
            "proto/event_bus/v1/event_bus.proto",
            "proto/scheduler/v1/scheduler.proto",
            "proto/chain/v1/chain.proto",
            "proto/storage/v1/storage.proto",
        ],
        &["proto"],
    )
    .expect("failed to compile event bus protobuf schemas");
}
