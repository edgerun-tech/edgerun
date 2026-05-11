use edgerun_work::*;

#[test]
fn xor_erasure_job_stores_retrieves_and_reconstructs_all_shards() {
    let storage0 = SimNode::from_seed(61, NODE_ROLE_STORAGE);
    let storage1 = SimNode::from_seed(62, NODE_ROLE_STORAGE);
    let storage2 = SimNode::from_seed(63, NODE_ROLE_STORAGE);
    let nodes = [
        storage0.identity.node_id,
        storage1.identity.node_id,
        storage2.identity.node_id,
    ];
    let file = b"edge-run erasure coded storage test payload".to_vec();

    let (manifest, shards) = encode_xor_2_1(&file, nodes).expect("encode");
    verify_manifest(&manifest, &shards).expect("manifest verifies");

    let mut store = MemoryShardStore::new();
    for shard in shards {
        store.put(shard);
    }
    assert_eq!(store.len(), 3);

    let retrieved = retrieve_shards(&store, &manifest, &[0, 1, 2]);
    let reconstructed = reconstruct_xor_2_1(&manifest, &retrieved).expect("reconstruct");
    assert_eq!(reconstructed, file);
}

#[test]
fn xor_erasure_job_recovers_from_one_missing_data_shard() {
    let storage0 = SimNode::from_seed(71, NODE_ROLE_STORAGE);
    let storage1 = SimNode::from_seed(72, NODE_ROLE_STORAGE);
    let storage2 = SimNode::from_seed(73, NODE_ROLE_STORAGE);
    let nodes = [
        storage0.identity.node_id,
        storage1.identity.node_id,
        storage2.identity.node_id,
    ];
    let file = b"recover me even if one data shard disappears".to_vec();

    let (manifest, shards) = encode_xor_2_1(&file, nodes).expect("encode");
    let missing_data_hash = manifest.shard_hashes[1];

    let mut store = MemoryShardStore::new();
    for shard in shards {
        store.put(shard);
    }
    store.remove(&missing_data_hash);
    assert_eq!(store.len(), 2);

    let retrieved = retrieve_shards(&store, &manifest, &[0, 2]);
    let reconstructed = reconstruct_xor_2_1(&manifest, &retrieved).expect("recover from parity");
    assert_eq!(reconstructed, file);
}

#[test]
fn xor_erasure_job_rejects_corrupt_shard() {
    let storage0 = SimNode::from_seed(81, NODE_ROLE_STORAGE);
    let storage1 = SimNode::from_seed(82, NODE_ROLE_STORAGE);
    let storage2 = SimNode::from_seed(83, NODE_ROLE_STORAGE);
    let nodes = [
        storage0.identity.node_id,
        storage1.identity.node_id,
        storage2.identity.node_id,
    ];
    let file = b"corruption should be caught before reconstruction".to_vec();

    let (manifest, mut shards) = encode_xor_2_1(&file, nodes).expect("encode");
    shards[0].bytes[0] ^= 0xFF;
    assert!(matches!(
        verify_manifest(&manifest, &shards),
        Err(ErasureStorageError::HashMismatch)
    ));
}
