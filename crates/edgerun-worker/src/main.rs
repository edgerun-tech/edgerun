use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let demo_payload = edgerun_types::BundlePayload {
        v: 1,
        runtime_id: [0_u8; 32],
        wasm: vec![0x00, 0x61, 0x73, 0x6d],
        input: vec![1, 2, 3],
        limits: edgerun_types::Limits {
            max_memory_bytes: 1024,
            max_instructions: 2048,
        },
    };
    let demo_bytes = edgerun_types::encode_bundle_payload_canonical(&demo_payload)?;
    let (_bundle_hash, _decoded) = hash_then_decode_bundle(&demo_bytes)?;

    tracing::info!("edgerun-worker scaffold");
    tracing::info!("TODO: heartbeat, assignment polling, runtime execution, submit_result tx");
    Ok(())
}

fn hash_then_decode_bundle(
    downloaded_bundle_payload_bytes: &[u8],
) -> Result<([u8; 32], edgerun_types::BundlePayload), edgerun_types::BundleCodecError> {
    // Required invariant: hash exactly raw downloaded bytes before any decode.
    let bundle_hash = edgerun_crypto::compute_bundle_hash(downloaded_bundle_payload_bytes);
    let decoded = edgerun_types::decode_bundle_payload_canonical(downloaded_bundle_payload_bytes)?;
    Ok((bundle_hash, decoded))
}
