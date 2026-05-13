use alloc::vec::Vec;

use crate::erasure_storage::{ErasureManifest, ErasureShard};
use crate::preimage::HashBuilder;
use crate::protocol::{Hash, NodeId};

pub const COST_UNIT_BYTES: u64 = 1024;
const STORAGE_COST_ESTIMATE_DOMAIN: &[u8] = b"edgerun:v1:work:storage-cost-estimate";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitPriceTable {
    pub store_per_kib_epoch: u64,
    pub retrieve_per_kib: u64,
    pub relay_per_kib: u64,
    pub receipt_base: u64,
}

impl UnitPriceTable {
    pub const fn cheap_test() -> Self {
        Self {
            store_per_kib_epoch: 1,
            retrieve_per_kib: 1,
            relay_per_kib: 1,
            receipt_base: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeCostShare {
    pub node_id: NodeId,
    pub bytes: u64,
    pub amount: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageCostEstimate {
    pub original_bytes: u64,
    pub stored_bytes: u64,
    pub erasure_overhead_bps: u64,
    pub storage_epochs: u64,
    pub storage_total: u64,
    pub retrieval_total: u64,
    pub relay_total: u64,
    pub receipt_base_total: u64,
    pub total: u64,
    pub storage_shares: Vec<NodeCostShare>,
    pub retrieval_shares: Vec<NodeCostShare>,
    pub relay_shares: Vec<NodeCostShare>,
    pub estimate_hash: Hash,
}

pub fn estimate_erasure_storage_cost(
    manifest: &ErasureManifest,
    shards: &[ErasureShard],
    relay_nodes: &[NodeId],
    epochs: u64,
    prices: UnitPriceTable,
) -> StorageCostEstimate {
    let original_bytes = manifest.original_len;
    let stored_bytes = shards
        .iter()
        .map(|shard| shard.bytes.len() as u64)
        .sum::<u64>();
    let erasure_overhead_bps = if original_bytes == 0 {
        0
    } else {
        stored_bytes.saturating_mul(10_000) / original_bytes
    };

    let mut storage_total = 0u64;
    let mut storage_shares = Vec::with_capacity(shards.len());
    for shard in shards {
        let amount = cost_for_bytes(shard.bytes.len() as u64, prices.store_per_kib_epoch)
            .saturating_mul(epochs)
            .saturating_add(prices.receipt_base);
        storage_total = storage_total.saturating_add(amount);
        storage_shares.push(NodeCostShare {
            node_id: shard.assigned_node,
            bytes: shard.bytes.len() as u64,
            amount,
        });
    }

    let mut retrieval_total = 0u64;
    let mut retrieval_shares = Vec::with_capacity(shards.len());
    for shard in shards {
        let amount = cost_for_bytes(shard.bytes.len() as u64, prices.retrieve_per_kib)
            .saturating_add(prices.receipt_base);
        retrieval_total = retrieval_total.saturating_add(amount);
        retrieval_shares.push(NodeCostShare {
            node_id: shard.assigned_node,
            bytes: shard.bytes.len() as u64,
            amount,
        });
    }

    let mut relay_total = 0u64;
    let mut relay_shares = Vec::new();
    for (index, relay_node) in relay_nodes.iter().enumerate() {
        let bytes = if shards.is_empty() {
            0
        } else {
            shards[index % shards.len()].bytes.len() as u64
        };
        let amount =
            cost_for_bytes(bytes, prices.relay_per_kib).saturating_add(prices.receipt_base);
        relay_total = relay_total.saturating_add(amount);
        relay_shares.push(NodeCostShare {
            node_id: *relay_node,
            bytes,
            amount,
        });
    }

    let receipt_base_total = prices.receipt_base.saturating_mul(
        (storage_shares.len() + retrieval_shares.len() + relay_shares.len()) as u64,
    );
    let total = storage_total
        .saturating_add(retrieval_total)
        .saturating_add(relay_total);
    let mut estimate = StorageCostEstimate {
        original_bytes,
        stored_bytes,
        erasure_overhead_bps,
        storage_epochs: epochs,
        storage_total,
        retrieval_total,
        relay_total,
        receipt_base_total,
        total,
        storage_shares,
        retrieval_shares,
        relay_shares,
        estimate_hash: [0u8; 32],
    };
    estimate.estimate_hash = storage_cost_estimate_hash(&estimate);
    estimate
}

pub fn cost_for_bytes(bytes: u64, price_per_kib: u64) -> u64 {
    let units = bytes.saturating_add(COST_UNIT_BYTES - 1) / COST_UNIT_BYTES;
    units.saturating_mul(price_per_kib)
}

pub fn storage_cost_estimate_hash(estimate: &StorageCostEstimate) -> Hash {
    let mut builder = HashBuilder::domain(STORAGE_COST_ESTIMATE_DOMAIN)
        .u64(estimate.original_bytes)
        .u64(estimate.stored_bytes)
        .u64(estimate.erasure_overhead_bps)
        .u64(estimate.storage_epochs)
        .u64(estimate.storage_total)
        .u64(estimate.retrieval_total)
        .u64(estimate.relay_total)
        .u64(estimate.receipt_base_total)
        .u64(estimate.total);
    builder = encode_shares(builder, &estimate.storage_shares);
    builder = encode_shares(builder, &estimate.retrieval_shares);
    builder = encode_shares(builder, &estimate.relay_shares);
    builder.finish()
}

fn encode_shares(mut builder: HashBuilder, shares: &[NodeCostShare]) -> HashBuilder {
    builder = builder.u64(shares.len() as u64);
    for share in shares {
        builder = builder
            .node_id(&share.node_id)
            .u64(share.bytes)
            .u64(share.amount);
    }
    builder
}
