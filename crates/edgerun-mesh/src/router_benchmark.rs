/// Router lookup benchmark for the edgerun mesh.
///
/// Measures next-hop routing table lookups with a realistic peer table.
use std::time::{Duration, Instant};

use crate::{router::MeshRouter, LocalNode, MeshRoute};
use edgerun_hardware_signing::NodeID;

// ===========================================================================
// Router Lookup Benchmark
// ===========================================================================

pub fn benchmark_router_lookup() -> u64 {
    let local_id = node_id_with_byte(0x11);
    let mut router = MeshRouter::new(LocalNode::new(local_id));

    // Populate with 50 peers via discovery
    let mut peer_ids = Vec::new();
    for i in 0..50u8 {
        let peer_id = node_id_with_pattern(i);
        peer_ids.push(peer_id);

        // Create a discovery packet with this peer's route
        use crate::DiscoveryPacket;
        let route = MeshRoute {
            destination: peer_id,
            next_hop: None,
            cost: 1,
        };
        let packet = DiscoveryPacket {
            sequence: 1,
            routes: vec![route],
        };
        router.process_discovery(peer_id, &packet, 1_000_000);
    }

    let start = Instant::now();
    let target = Duration::from_millis(50);
    let mut ops: u64 = 0;
    let mut idx = 0usize;

    while start.elapsed() < target {
        let target_id = &peer_ids[idx % peer_ids.len()];
        if router.next_hop_for(target_id).is_some() {
            ops += 1;
        }
        idx += 1;
    }

    std::hint::black_box(ops);

    let elapsed_us = start.elapsed().as_micros() as u64;
    if elapsed_us > 0 {
        ops.saturating_mul(1_000_000) / elapsed_us
    } else {
        0
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

fn node_id_with_byte(b: u8) -> NodeID {
    let mut bytes = [0u8; 64];
    bytes[0] = b;
    NodeID(bytes)
}

fn node_id_with_pattern(v: u8) -> NodeID {
    let mut bytes = [0u8; 64];
    for b in bytes.iter_mut() {
        *b = v;
    }
    NodeID(bytes)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_lookup_fast() {
        let ops = benchmark_router_lookup();
        assert!(ops > 0, "router lookup was 0");
    }
}
