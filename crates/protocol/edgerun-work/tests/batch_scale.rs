use edgerun_work::*;

fn synthetic_receipt_hash(index: u64) -> Hash {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"edgerun:synthetic-receipt-hash:v1");
    bytes.extend_from_slice(&index.to_be_bytes());
    blake3_hash(&bytes)
}

fn synthetic_receipt_hashes(count: usize) -> Vec<Hash> {
    (0..count)
        .map(|index| synthetic_receipt_hash(index as u64))
        .collect()
}

#[test]
fn synthetic_batch_root_handles_100k_receipt_hashes() {
    let count = 100_000usize;
    let admission_hash = blake3_hash(b"synthetic-100k-admission");
    let receipt_hashes = synthetic_receipt_hashes(count);
    let total_claim = count as u64;

    let root = receipt_batch_root(admission_hash, &receipt_hashes, total_claim);
    let root_again = receipt_batch_root(admission_hash, &receipt_hashes, total_claim);

    assert_eq!(root, root_again);
    assert_ne!(root, [0u8; 32]);
    assert_eq!(receipt_hashes.len(), count);

    let changed_total = receipt_batch_root(admission_hash, &receipt_hashes, total_claim + 1);
    assert_ne!(root, changed_total);

    let mut changed_hashes = receipt_hashes.clone();
    changed_hashes[count / 2] = synthetic_receipt_hash(999_999_999);
    let changed_root = receipt_batch_root(admission_hash, &changed_hashes, total_claim);
    assert_ne!(root, changed_root);
}

#[test]
fn synthetic_merkle_root_is_order_sensitive_at_100k() {
    let count = 100_000usize;
    let mut hashes = synthetic_receipt_hashes(count);
    let root = merkle_root(hashes.clone());

    hashes.swap(10, count - 10);
    let reordered_root = merkle_root(hashes);

    assert_ne!(root, [0u8; 32]);
    assert_ne!(root, reordered_root);
}

#[test]
#[ignore = "heavy stress test: run with --ignored --test batch_scale"]
fn synthetic_batch_root_handles_1m_receipt_hashes() {
    let count = 1_000_000usize;
    let admission_hash = blake3_hash(b"synthetic-1m-admission");
    let receipt_hashes = synthetic_receipt_hashes(count);
    let total_claim = count as u64;

    let root = receipt_batch_root(admission_hash, &receipt_hashes, total_claim);
    let root_again = receipt_batch_root(admission_hash, &receipt_hashes, total_claim);

    assert_eq!(root, root_again);
    assert_ne!(root, [0u8; 32]);
    assert_eq!(receipt_hashes.len(), count);
}
