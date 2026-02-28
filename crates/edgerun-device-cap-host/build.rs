// SPDX-License-Identifier: Apache-2.0

fn main() {
    println!("cargo:rerun-if-changed=proto/device_capability/v1/device_capability.proto");
    prost_build::compile_protos(
        &["proto/device_capability/v1/device_capability.proto"],
        &["proto"],
    )
    .expect("failed to compile device capability protobuf schemas");
}
