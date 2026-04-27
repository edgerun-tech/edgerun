use crate::{
    discovery::DiscoveryPacket, router::MeshRouter, FrameType, LocalNode, MeshFrame,
    MeshFrameHeader, MeshPeer, MeshRoute, MeshRoutingTable,
};
use alloc::vec;
use alloc::vec::Vec;
use edgerun_hardware_signing::{NodeID, MESH_SIGNATURE_LENGTH};

// ---------------------------------------------------------------------------
// Discovery packet (serialized payload)
// ---------------------------------------------------------------------------
use super::*;

// -----------------------------------------------------------------------
// Test helpers
// -----------------------------------------------------------------------

fn node_id(v: u8) -> NodeID {
    let mut bytes = [0u8; 64];
    bytes[0] = v;
    NodeID(bytes)
}

fn make_router(id: u8) -> MeshRouter {
    MeshRouter::new(LocalNode::new(node_id(id)))
}

// =======================================================================
// 1. Discovery packet encoding/decoding
// =======================================================================

#[test]
fn discovery_encode_decode_roundtrip() {
    let packet = DiscoveryPacket {
        sequence: 42,
        routes: vec![
            MeshRoute {
                destination: node_id(0xAA),
                next_hop: None,
                cost: 1,
            },
            MeshRoute {
                destination: node_id(0xBB),
                next_hop: Some(node_id(0xAA)),
                cost: 2,
            },
        ],
    };
    let encoded = packet.encode();
    let decoded = DiscoveryPacket::decode(&encoded).expect("decode should succeed");
    assert_eq!(decoded.sequence, 42);
    assert_eq!(decoded.routes.len(), 2);
    assert_eq!(decoded.routes[0].destination, node_id(0xAA));
    assert_eq!(decoded.routes[0].cost, 1);
    assert_eq!(decoded.routes[1].destination, node_id(0xBB));
    assert_eq!(decoded.routes[1].cost, 2);
}

#[test]
fn discovery_decode_rejects_truncated() {
    assert!(DiscoveryPacket::decode(&[]).is_none());
    assert!(DiscoveryPacket::decode(&[0; 4]).is_none()); // no count byte
    assert!(DiscoveryPacket::decode(&[0; 5]).is_some()); // 0 routes is valid
    assert!(DiscoveryPacket::decode(&[1; 69]).is_none()); // count=1 but only 64 data bytes (need 65)
}

#[test]
fn discovery_encode_empty_routes() {
    let packet = DiscoveryPacket {
        sequence: 0,
        routes: vec![],
    };
    let encoded = packet.encode();
    assert_eq!(encoded.len(), 5); // 4 bytes sequence + 1 byte count
    assert_eq!(encoded[0..4], 0u32.to_le_bytes());
    assert_eq!(encoded[4], 0);
}

#[test]
fn discovery_encode_clips_max_routes() {
    // Build a packet with more routes than MAX_ROUTES
    let mut routes = Vec::new();
    for i in 0..=DiscoveryPacket::MAX_ROUTES {
        routes.push(MeshRoute {
            destination: node_id(i as u8),
            next_hop: None,
            cost: 1,
        });
    }
    let packet = DiscoveryPacket {
        sequence: 1,
        routes,
    };
    let encoded = packet.encode();
    // Should encode only MAX_ROUTES entries
    let count_byte = encoded[4];
    assert_eq!(count_byte as usize, DiscoveryPacket::MAX_ROUTES);
    // Decode should yield exactly MAX_ROUTES
    let decoded = DiscoveryPacket::decode(&encoded).unwrap();
    assert_eq!(decoded.routes.len(), DiscoveryPacket::MAX_ROUTES);
}

#[test]
fn discovery_decode_extra_bytes_are_ignored() {
    // Valid packet with extra trailing bytes
    let mut buf = vec![0u8; 5]; // 0 routes
    buf.extend_from_slice(&[0xFF; 10]); // trailing garbage
    let decoded = DiscoveryPacket::decode(&buf).unwrap();
    assert_eq!(decoded.routes.len(), 0);
}

#[test]
fn discovery_decode_partial_route_rejected() {
    // count=1 but only 64 bytes of route data (missing cost byte)
    // Need to set count byte to 1: [seq=0 (4 bytes), count=1, 64 bytes of route data]
    let mut buf = vec![0u8; 5];
    buf[4] = 1; // count = 1
    buf.extend_from_slice(&[0xAA; 64]); // only 64 bytes, need 65
    assert!(DiscoveryPacket::decode(&buf).is_none());
}

#[test]
fn discovery_encode_sequence_byte_order() {
    let packet = DiscoveryPacket {
        sequence: 0x01020304,
        routes: vec![],
    };
    let encoded = packet.encode();
    assert_eq!(&encoded[0..4], &[0x04, 0x03, 0x02, 0x01]); // little-endian
}

#[test]
fn discovery_from_local_increments_sequence() {
    let mut local = LocalNode::new(node_id(0x01));
    let table = MeshRoutingTable::default();
    let packet = DiscoveryPacket::from_local(&mut local, &table);
    assert_eq!(packet.sequence, 1);
    assert_eq!(local.discovery_sequence, 1);
}

#[test]
fn discovery_packet_clone_and_debug() {
    let packet = DiscoveryPacket {
        sequence: 1,
        routes: vec![],
    };
    let cloned = packet.clone();
    assert_eq!(packet, cloned);
    let debug = format!("{packet:?}");
    assert!(debug.contains("DiscoveryPacket"));
}

#[test]
fn discovery_max_routes_constant() {
    // 65 bytes per route * 50 = 3250 + 5 header = 3255
    let routes = (0..DiscoveryPacket::MAX_ROUTES)
        .map(|i| MeshRoute {
            destination: node_id(i as u8),
            next_hop: None,
            cost: 1,
        })
        .collect();
    let packet = DiscoveryPacket {
        sequence: 1,
        routes,
    };
    let encoded = packet.encode();
    let decoded = DiscoveryPacket::decode(&encoded).unwrap();
    assert_eq!(decoded.routes.len(), DiscoveryPacket::MAX_ROUTES);
}

// =======================================================================
// 2. Bellman-Ford routing algorithm
// =======================================================================

#[test]
fn bellman_ford_direct_peer_learned() {
    let mut router = make_router(0xAA);
    let peer_id = node_id(0xBB);
    let routes = vec![MeshRoute {
        destination: node_id(0xCC),
        next_hop: None,
        cost: 1,
    }];
    let packet = DiscoveryPacket {
        sequence: 1,
        routes,
    };
    let changed = router.process_discovery(peer_id, &packet, 1000);
    assert!(changed);

    // Direct route to peer
    let peer_route = router.routing_table().lookup(&peer_id).unwrap();
    assert_eq!(peer_route.cost, 1);
    assert!(peer_route.next_hop.is_none());
}

#[test]
fn bellman_ford_propagates_route_cost() {
    // A knows C at cost 1. B learns A's discovery -> B knows C at cost 2 via A.
    let mut a = make_router(0xAA);
    let mut b = make_router(0xBB);

    // Manually give A a route to C via a fake discovery
    let fake_packet = DiscoveryPacket {
        sequence: 1,
        routes: vec![MeshRoute {
            destination: node_id(0xCC),
            next_hop: None,
            cost: 1,
        }],
    };
    a.process_discovery(node_id(0xCC), &fake_packet, 1000);

    let disc_a = {
        let table_clone = a.routing_table().clone();
        DiscoveryPacket::from_local(&mut a.local, &table_clone)
    };
    let changed = b.process_discovery(node_id(0xAA), &disc_a, 1000);
    assert!(changed);

    let route = b.routing_table().lookup(&node_id(0xCC)).unwrap();
    assert_eq!(route.cost, 2);
    assert_eq!(route.next_hop, Some(node_id(0xAA)));
}

#[test]
fn bellman_ford_no_improvement_no_change() {
    let mut router = make_router(0xAA);
    // Pre-populate: learn BB as peer, and CC via BB at cost 2
    let fake = DiscoveryPacket {
        sequence: 1,
        routes: vec![MeshRoute {
            destination: node_id(0xCC),
            next_hop: None,
            cost: 1,
        }],
    };
    router.process_discovery(node_id(0xBB), &fake, 1000);

    // Now BB is already a known peer. Send same info again from BB.
    // The existing route to CC has cost 2. New cost via BB is also 2.
    // dominated: existing.cost (2) <= new_cost (2), so no route update.
    // And BB is already a peer so no peer route update either.
    let packet = DiscoveryPacket {
        sequence: 2,
        routes: vec![MeshRoute {
            destination: node_id(0xCC),
            next_hop: None,
            cost: 1,
        }],
    };
    let _ = router.process_discovery(node_id(0xBB), &packet, 2000);
    // The route to CC should not change (equal cost).
    // But changed may be true if the self-route is re-inserted (it always is).
    // What matters is the route to CC is unchanged.
    let route = router.routing_table().lookup(&node_id(0xCC)).unwrap();
    assert_eq!(route.cost, 2);
    assert_eq!(route.next_hop, Some(node_id(0xBB)));
}

#[test]
fn bellman_ford_improves_existing_route() {
    let mut router = make_router(0xAA);
    // Pre-populate with a suboptimal route via DD (cost 4 -> total 5)
    let fake = DiscoveryPacket {
        sequence: 1,
        routes: vec![MeshRoute {
            destination: node_id(0xCC),
            next_hop: None,
            cost: 4,
        }],
    };
    router.process_discovery(node_id(0xDD), &fake, 1000);

    // Peer advertises a better route (cost 1 -> total 2)
    let packet = DiscoveryPacket {
        sequence: 1,
        routes: vec![MeshRoute {
            destination: node_id(0xCC),
            next_hop: None,
            cost: 1,
        }],
    };
    let changed = router.process_discovery(node_id(0xBB), &packet, 1000);
    assert!(changed);

    let route = router.routing_table().lookup(&node_id(0xCC)).unwrap();
    assert_eq!(route.cost, 2);
    assert_eq!(route.next_hop, Some(node_id(0xBB)));
}

#[test]
fn bellman_ford_skips_self_destination() {
    let mut router = make_router(0xAA);
    let packet = DiscoveryPacket {
        sequence: 1,
        routes: vec![MeshRoute {
            destination: node_id(0xAA), // ourselves
            next_hop: None,
            cost: 3,
        }],
    };
    let _ = router.process_discovery(node_id(0xBB), &packet, 1000);
    // Should not create a route to ourselves via someone else
    let route = router.routing_table().lookup(&node_id(0xAA));
    // Only the self-insert at the end exists with cost 0
    assert!(route.map_or(true, |r| r.cost == 0));
}

#[test]
fn bellman_ford_skips_sender_echo() {
    let mut router = make_router(0xAA);
    let peer = node_id(0xBB);
    let packet = DiscoveryPacket {
        sequence: 1,
        routes: vec![MeshRoute {
            destination: peer, // sender advertising itself
            next_hop: None,
            cost: 0,
        }],
    };
    let _ = router.process_discovery(peer, &packet, 1000);
    // Should not learn a route to the sender from the sender's own advertisement
    // (the direct route is added separately with cost 1)
    let route = router.routing_table().lookup(&peer).unwrap();
    assert_eq!(route.cost, 1);
    assert!(route.next_hop.is_none());
}

#[test]
fn bellman_ford_saturating_add_overflow() {
    let mut router = make_router(0xAA);
    // Advertise a route with cost 255 -> 255 + 1 = 256 -> saturates to 255
    // But saturating_add(1) on 255 = 255, which is != 0 so passes that check
    // Actually: 255.saturating_add(1) = 255, which is != 0, so it IS processed.
    // Let's check with cost = 255
    let packet = DiscoveryPacket {
        sequence: 1,
        routes: vec![MeshRoute {
            destination: node_id(0xCC),
            next_hop: None,
            cost: 255,
        }],
    };
    let _ = router.process_discovery(node_id(0xBB), &packet, 1000);
    let route = router.routing_table().lookup(&node_id(0xCC)).unwrap();
    // 255.saturating_add(1) = 255
    assert_eq!(route.cost, 255);
}

#[test]
fn bellman_ford_zero_cost_advertised_becomes_cost_one() {
    let mut router = make_router(0xAA);
    // Advertised cost 0 -> computed cost = 0 + 1 = 1
    let packet = DiscoveryPacket {
        sequence: 1,
        routes: vec![MeshRoute {
            destination: node_id(0xCC),
            next_hop: None,
            cost: 0,
        }],
    };
    let changed = router.process_discovery(node_id(0xBB), &packet, 1000);
    assert!(changed);
    // Route exists with cost 1 (0 + 1 hop)
    let route = router.routing_table().lookup(&node_id(0xCC)).unwrap();
    assert_eq!(route.cost, 1);
}

#[test]
fn bellman_ford_self_route_always_inserted() {
    let mut router = make_router(0xAA);
    let packet = DiscoveryPacket {
        sequence: 1,
        routes: vec![],
    };
    let _ = router.process_discovery(node_id(0xBB), &packet, 1000);
    let self_route = router.routing_table().lookup(&node_id(0xAA)).unwrap();
    assert_eq!(self_route.cost, 0);
    assert!(self_route.next_hop.is_none());
}

#[test]
fn bellman_ford_equal_cost_no_update() {
    let mut router = make_router(0xAA);
    // Pre-populate: learn DD as peer, and CC via DD at cost 3 (advertised 2 + 1)
    let fake = DiscoveryPacket {
        sequence: 1,
        routes: vec![MeshRoute {
            destination: node_id(0xCC),
            next_hop: None,
            cost: 2,
        }],
    };
    router.process_discovery(node_id(0xDD), &fake, 1000);

    // BB advertises CC at cost 2 -> new_cost = 3, same as existing via DD.
    // But BB is a NEW peer, so the changed flag will be set for adding BB's direct route.
    // We verify the CC route is NOT changed (still via DD at cost 3).
    let old_cost = router.routing_table().lookup(&node_id(0xCC)).unwrap().cost;
    let old_next_hop = router
        .routing_table()
        .lookup(&node_id(0xCC))
        .unwrap()
        .next_hop;

    let packet = DiscoveryPacket {
        sequence: 1,
        routes: vec![MeshRoute {
            destination: node_id(0xCC),
            next_hop: None,
            cost: 2,
        }],
    };
    router.process_discovery(node_id(0xBB), &packet, 1000);

    // CC route should remain via DD at cost 3 (equal cost, first wins)
    let route = router.routing_table().lookup(&node_id(0xCC)).unwrap();
    assert_eq!(route.cost, old_cost);
    assert_eq!(route.next_hop, old_next_hop);
}

// =======================================================================
// 3. Peer liveness and heartbeat tracking
// =======================================================================

#[test]
fn peer_learned_from_discovery() {
    let mut router = make_router(0xAA);
    let mut peer_router = make_router(0xBB);
    let disc = peer_router.build_discovery_frame();
    let pkt = DiscoveryPacket::decode(&disc.payload).unwrap();
    router.process_discovery(node_id(0xBB), &pkt, 1000);

    assert_eq!(router.active_peers().len(), 1);
    assert_eq!(router.all_peers().len(), 1);
}

#[test]
fn heartbeat_increments_missed_count() {
    let mut router = make_router(0xAA);
    let peer_id = node_id(0xBB);
    router.process_discovery(
        peer_id,
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );

    {
        let peers = router.all_peers();
        let peer = peers.first().unwrap();
        assert_eq!(peer.missed_heartbeats, 0);
    }

    let dead = router.tick_heartbeat();
    assert!(dead.is_empty());

    {
        let peers = router.all_peers();
        let peer = peers.first().unwrap();
        assert_eq!(peer.missed_heartbeats, 1);
    }
}

#[test]
fn peer_dead_after_three_missed_heartbeats() {
    let mut router = make_router(0xAA);
    let peer_id = node_id(0xBB);
    router.process_discovery(
        peer_id,
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );

    router.tick_heartbeat(); // 1
    router.tick_heartbeat(); // 2
    let dead = router.tick_heartbeat(); // 3 -> dead
    assert_eq!(dead, vec![node_id(0xBB)]);

    assert_eq!(router.active_peers().len(), 0);
    assert_eq!(router.all_peers().len(), 0);
}

#[test]
fn peer_resurrected_by_new_discovery_before_death() {
    let mut router = make_router(0xAA);
    let peer_id = node_id(0xBB);
    router.process_discovery(
        peer_id,
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );

    router.tick_heartbeat(); // 1 miss
    router.tick_heartbeat(); // 2 misses

    // New discovery resets the heartbeat
    router.process_discovery(
        peer_id,
        &DiscoveryPacket {
            sequence: 2,
            routes: vec![],
        },
        2000,
    );

    {
        let peers = router.all_peers();
        let peer = peers.first().unwrap();
        assert_eq!(peer.missed_heartbeats, 0);
    }

    // Should not go dead now
    router.tick_heartbeat(); // 1
    router.tick_heartbeat(); // 2
    router.tick_heartbeat(); // 3 -> dead
    assert_eq!(router.active_peers().len(), 0);
}

#[test]
fn tick_heartbeat_empty_peers() {
    let mut router = make_router(0xAA);
    let dead = router.tick_heartbeat();
    assert!(dead.is_empty());
}

#[test]
fn tick_heartbeat_multiple_peers_partial_death() {
    let mut router = make_router(0xAA);
    // Learn two peers
    router.process_discovery(
        node_id(0xBB),
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );
    router.process_discovery(
        node_id(0xCC),
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );

    // Tick 3 times — both should die
    router.tick_heartbeat();
    router.tick_heartbeat();
    let dead = router.tick_heartbeat();
    assert_eq!(dead.len(), 2);
    assert!(dead.contains(&node_id(0xBB)));
    assert!(dead.contains(&node_id(0xCC)));
    assert_eq!(router.active_peers().len(), 0);
}

#[test]
fn routes_via_dead_peer_removed() {
    let mut router = make_router(0xAA);
    let peer_id = node_id(0xBB);
    // Peer advertises a route to CC
    router.process_discovery(
        peer_id,
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![MeshRoute {
                destination: node_id(0xCC),
                next_hop: None,
                cost: 1,
            }],
        },
        1000,
    );

    assert!(router.has_route_to(&node_id(0xCC)));

    // Kill the peer
    router.tick_heartbeat();
    router.tick_heartbeat();
    router.tick_heartbeat();

    // Routes via BB should be gone
    assert!(!router.has_route_to(&node_id(0xBB)));
    assert!(!router.has_route_to(&node_id(0xCC)));
}

#[test]
fn missed_heartbeats_saturating() {
    let mut router = make_router(0xAA);
    let peer_id = node_id(0xBB);
    router.process_discovery(
        peer_id,
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );

    // Tick many times — should saturate, not overflow
    for _ in 0..200 {
        router.tick_heartbeat();
    }
    // After the 3rd tick the peer is removed, so further ticks are no-ops
    assert_eq!(router.all_peers().len(), 0);
}

// =======================================================================
// 4. Route table updates and best path selection
// =======================================================================

#[test]
fn routing_table_empty_initially() {
    let router = make_router(0xAA);
    assert!(router.routing_table().is_empty());
    assert_eq!(router.routing_table().len(), 0);
}

#[test]
fn routing_table_iter() {
    let mut router = make_router(0xAA);
    router.process_discovery(
        node_id(0xBB),
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );
    let routes: Vec<_> = router.routing_table().iter().collect();
    assert!(!routes.is_empty());
    // At minimum: self route + peer route
    assert!(routes.len() >= 2);
}

#[test]
fn routing_table_best_route_lowest_cost_wins() {
    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: node_id(0xCC),
        next_hop: Some(node_id(0xBB)),
        cost: 5,
    });
    table.update(MeshRoute {
        destination: node_id(0xCC),
        next_hop: Some(node_id(0xDD)),
        cost: 2,
    });

    let best = table.lookup(&node_id(0xCC)).unwrap();
    assert_eq!(best.cost, 2);
    assert_eq!(best.next_hop, Some(node_id(0xDD)));
}

#[test]
fn routing_table_update_preserves_better_cost() {
    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: node_id(0xCC),
        next_hop: Some(node_id(0xBB)),
        cost: 3,
    });
    // Worse cost should not replace
    table.update(MeshRoute {
        destination: node_id(0xCC),
        next_hop: Some(node_id(0xDD)),
        cost: 5,
    });
    let best = table.lookup(&node_id(0xCC)).unwrap();
    assert_eq!(best.cost, 3);
}

#[test]
fn routing_table_remove_via() {
    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: node_id(0xCC),
        next_hop: Some(node_id(0xBB)),
        cost: 2,
    });
    table.update(MeshRoute {
        destination: node_id(0xDD),
        next_hop: Some(node_id(0xBB)),
        cost: 3,
    });
    table.update(MeshRoute {
        destination: node_id(0xEE),
        next_hop: None,
        cost: 1,
    });

    table.remove_via(&node_id(0xBB));
    assert!(table.lookup(&node_id(0xCC)).is_none());
    assert!(table.lookup(&node_id(0xDD)).is_none());
    // Direct route should remain
    assert!(table.lookup(&node_id(0xEE)).is_some());
}

#[test]
fn routing_table_remove_destination() {
    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: node_id(0xCC),
        next_hop: Some(node_id(0xBB)),
        cost: 2,
    });
    table.remove_destination(&node_id(0xCC));
    assert!(table.lookup(&node_id(0xCC)).is_none());
}

#[test]
fn routing_table_len_and_is_empty() {
    let mut table = MeshRoutingTable::default();
    assert!(table.is_empty());
    assert_eq!(table.len(), 0);

    table.update(MeshRoute {
        destination: node_id(0xCC),
        next_hop: None,
        cost: 1,
    });
    assert!(!table.is_empty());
    assert_eq!(table.len(), 1);
}

#[test]
fn routing_table_clone() {
    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: node_id(0xCC),
        next_hop: None,
        cost: 1,
    });
    let cloned = table.clone();
    assert_eq!(table.len(), cloned.len());
    assert_eq!(
        table.lookup(&node_id(0xCC)).map(|r| r.cost),
        cloned.lookup(&node_id(0xCC)).map(|r| r.cost)
    );
}

#[test]
fn routing_table_debug_format() {
    let table = MeshRoutingTable::default();
    let debug = format!("{table:?}");
    assert!(debug.contains("MeshRoutingTable"));
}

// =======================================================================
// 5. Multi-hop route learning
// =======================================================================

#[test]
fn multi_hop_route_learned_via_bellman_ford() {
    let mut a = make_router(0xAA);
    let mut b = make_router(0xBB);
    let mut c = make_router(0xCC);

    // A discovers B directly
    let disc_a = a.build_discovery_frame();
    let pkt_a = DiscoveryPacket::decode(&disc_a.payload).unwrap();
    b.process_discovery(node_id(0xAA), &pkt_a, 1000);

    // B discovers C directly
    let disc_b = b.build_discovery_frame();
    let pkt_b = DiscoveryPacket::decode(&disc_b.payload).unwrap();
    c.process_discovery(node_id(0xBB), &pkt_b, 1000);

    // C should now have:
    //   -> B cost 1 (direct)
    //   -> A cost 2 (via B, learned from B's discovery)
    assert!(c.has_route_to(&node_id(0xAA)));
    let route_to_a = c.routing_table().lookup(&node_id(0xAA)).unwrap();
    assert_eq!(route_to_a.cost, 2);
    assert_eq!(route_to_a.next_hop, Some(node_id(0xBB)));

    assert!(c.has_route_to(&node_id(0xBB)));
    let route_to_b = c.routing_table().lookup(&node_id(0xBB)).unwrap();
    assert_eq!(route_to_b.cost, 1);
}

#[test]
fn four_node_chain_a_b_c_d() {
    let mut a = make_router(0xAA);
    let mut b = make_router(0xBB);
    let mut c = make_router(0xCC);
    let mut d = make_router(0xDD);

    // A -> B
    let pkt_a = DiscoveryPacket::decode(&a.build_discovery_frame().payload).unwrap();
    b.process_discovery(node_id(0xAA), &pkt_a, 1000);

    // B -> C
    let pkt_b = DiscoveryPacket::decode(&b.build_discovery_frame().payload).unwrap();
    c.process_discovery(node_id(0xBB), &pkt_b, 1000);

    // C -> D
    let pkt_c = DiscoveryPacket::decode(&c.build_discovery_frame().payload).unwrap();
    d.process_discovery(node_id(0xCC), &pkt_c, 1000);

    // D should reach A at cost 3, B at cost 2, C at cost 1
    let route_to_a = d.routing_table().lookup(&node_id(0xAA)).unwrap();
    assert_eq!(route_to_a.cost, 3);
    assert_eq!(route_to_a.next_hop, Some(node_id(0xCC)));

    let route_to_b = d.routing_table().lookup(&node_id(0xBB)).unwrap();
    assert_eq!(route_to_b.cost, 2);
    assert_eq!(route_to_b.next_hop, Some(node_id(0xCC)));

    let route_to_c = d.routing_table().lookup(&node_id(0xCC)).unwrap();
    assert_eq!(route_to_c.cost, 1);
    assert!(route_to_c.next_hop.is_none()); // direct
}

#[test]
fn diamond_topology_two_paths() {
    //   A
    //  / \
    // B   C
    //  \ /
    //   D
    let mut a = make_router(0xAA);
    let mut b = make_router(0xBB);
    let mut c = make_router(0xCC);
    let mut d = make_router(0xDD);

    // A->B, A->C
    let pkt_a1 = DiscoveryPacket::decode(&a.build_discovery_frame().payload).unwrap();
    b.process_discovery(node_id(0xAA), &pkt_a1, 1000);
    let pkt_a2 = DiscoveryPacket::decode(&a.build_discovery_frame().payload).unwrap();
    c.process_discovery(node_id(0xAA), &pkt_a2, 1000);

    // B->D (D learns A via B at cost 2)
    let pkt_b = DiscoveryPacket::decode(&b.build_discovery_frame().payload).unwrap();
    d.process_discovery(node_id(0xBB), &pkt_b, 1000);

    // C->D (D learns A via C also at cost 2, but no improvement so kept)
    let pkt_c = DiscoveryPacket::decode(&c.build_discovery_frame().payload).unwrap();
    d.process_discovery(node_id(0xCC), &pkt_c, 1000);

    let route_to_a = d.routing_table().lookup(&node_id(0xAA)).unwrap();
    assert_eq!(route_to_a.cost, 2);
    // First learned path wins on equal cost
    assert_eq!(route_to_a.next_hop, Some(node_id(0xBB)));
}

#[test]
fn multi_hop_route_updates_next_hop_to_intermediate() {
    // A-B-C-D chain: D's next_hop to A should be C
    let mut a = make_router(0xAA);
    let mut b = make_router(0xBB);
    let mut c = make_router(0xCC);
    let mut d = make_router(0xDD);

    let pkt_a = DiscoveryPacket::decode(&a.build_discovery_frame().payload).unwrap();
    b.process_discovery(node_id(0xAA), &pkt_a, 1000);

    let pkt_b = DiscoveryPacket::decode(&b.build_discovery_frame().payload).unwrap();
    c.process_discovery(node_id(0xBB), &pkt_b, 1000);

    let pkt_c = DiscoveryPacket::decode(&c.build_discovery_frame().payload).unwrap();
    d.process_discovery(node_id(0xCC), &pkt_c, 1000);

    // D's next_hop to A is C (the node that sent the discovery)
    let route = d.routing_table().lookup(&node_id(0xAA)).unwrap();
    assert_eq!(route.next_hop, Some(node_id(0xCC)));
}

// =======================================================================
// 6. TTL and forwarding behavior
// =======================================================================

#[test]
fn default_ttl_is_sixteen() {
    let router = make_router(0xAA);
    assert_eq!(router.default_ttl(), 16);
}

#[test]
fn set_default_ttl() {
    let mut router = make_router(0xAA);
    router.set_default_ttl(32);
    assert_eq!(router.default_ttl(), 32);
}

#[test]
fn discovery_frame_has_ttl_one() {
    let mut router = make_router(0xAA);
    let frame = router.build_discovery_frame();
    assert_eq!(frame.header.ttl, 1);
}

#[test]
fn discovery_frame_is_broadcast() {
    let mut router = make_router(0xAA);
    let frame = router.build_discovery_frame();
    assert_eq!(frame.header.dest, NodeID([0u8; 64]));
}

#[test]
fn discovery_frame_type_is_discovery() {
    let mut router = make_router(0xAA);
    let frame = router.build_discovery_frame();
    assert_eq!(frame.header.frame_type, FrameType::Discovery);
}

#[test]
fn discovery_frame_source_is_local_node() {
    let mut router = make_router(0xAA);
    let frame = router.build_discovery_frame();
    assert_eq!(frame.header.src, node_id(0xAA));
}

#[test]
fn should_forward_destined_for_us_returns_none() {
    let router = make_router(0xAA);
    let hdr = MeshFrameHeader {
        dest: node_id(0xAA),
        src: node_id(0xBB),
        ttl: 10,
        frame_type: FrameType::Data,
    };
    assert!(router.should_forward(&hdr).is_none());
}

#[test]
fn should_forward_zero_ttl_returns_none() {
    let mut router = make_router(0xAA);
    // Learn a route to BB
    router.process_discovery(
        node_id(0xBB),
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );
    let hdr = MeshFrameHeader {
        dest: node_id(0xBB),
        src: node_id(0xCC),
        ttl: 0,
        frame_type: FrameType::Data,
    };
    assert!(router.should_forward(&hdr).is_none());
}

#[test]
fn should_forward_no_route_returns_none() {
    let router = make_router(0xAA);
    let hdr = MeshFrameHeader {
        dest: node_id(0xFF),
        src: node_id(0xBB),
        ttl: 10,
        frame_type: FrameType::Data,
    };
    assert!(router.should_forward(&hdr).is_none());
}

#[test]
fn should_forward_valid_returns_decremented_ttl() {
    let mut router = make_router(0xAA);
    router.process_discovery(
        node_id(0xBB),
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );
    let hdr = MeshFrameHeader {
        dest: node_id(0xBB),
        src: node_id(0xCC),
        ttl: 5,
        frame_type: FrameType::Data,
    };
    assert_eq!(router.should_forward(&hdr), Some(4));
}

#[test]
fn should_forward_ttl_one() {
    let mut router = make_router(0xAA);
    router.process_discovery(
        node_id(0xBB),
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );
    let hdr = MeshFrameHeader {
        dest: node_id(0xBB),
        src: node_id(0xCC),
        ttl: 1,
        frame_type: FrameType::Data,
    };
    assert_eq!(router.should_forward(&hdr), Some(0));
}

#[test]
fn should_forward_ttl_255() {
    let mut router = make_router(0xAA);
    router.process_discovery(
        node_id(0xBB),
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );
    let hdr = MeshFrameHeader {
        dest: node_id(0xBB),
        src: node_id(0xCC),
        ttl: 255,
        frame_type: FrameType::Data,
    };
    assert_eq!(router.should_forward(&hdr), Some(254));
}

// =======================================================================
// 7. next_hop_for and should_forward functions
// =======================================================================

#[test]
fn next_hop_for_self_returns_none() {
    let router = make_router(0xAA);
    assert!(router.next_hop_for(&node_id(0xAA)).is_none());
}

#[test]
fn next_hop_for_unknown_destination_returns_none() {
    let router = make_router(0xAA);
    assert!(router.next_hop_for(&node_id(0xFF)).is_none());
}

#[test]
fn next_hop_for_direct_peer() {
    let mut router = make_router(0xAA);
    router.process_discovery(
        node_id(0xBB),
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );
    // Direct route: next_hop is None in table, so next_hop_for returns the destination itself
    assert_eq!(router.next_hop_for(&node_id(0xBB)), Some(node_id(0xBB)));
}

#[test]
fn next_hop_for_multi_hop() {
    let mut a = make_router(0xAA);
    let mut b = make_router(0xBB);
    let mut c = make_router(0xCC);

    let pkt_a = DiscoveryPacket::decode(&a.build_discovery_frame().payload).unwrap();
    b.process_discovery(node_id(0xAA), &pkt_a, 1000);

    let pkt_b = DiscoveryPacket::decode(&b.build_discovery_frame().payload).unwrap();
    c.process_discovery(node_id(0xBB), &pkt_b, 1000);

    // C's next_hop to A should be B
    assert_eq!(c.next_hop_for(&node_id(0xAA)), Some(node_id(0xBB)));
}

#[test]
fn has_route_to_self_after_discovery() {
    let mut router = make_router(0xAA);
    // Initially no route to self
    assert!(!router.has_route_to(&node_id(0xAA)));

    // After processing any discovery, self-route is added
    router.process_discovery(
        node_id(0xBB),
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );
    assert!(router.has_route_to(&node_id(0xAA)));
}

#[test]
fn has_route_to_unknown_returns_false() {
    let router = make_router(0xAA);
    assert!(!router.has_route_to(&node_id(0xFF)));
}

// =======================================================================
// 8. Accessors and setters
// =======================================================================

#[test]
fn accessor_node_id() {
    let router = make_router(0xAA);
    assert_eq!(router.node_id(), node_id(0xAA));
}

#[test]
fn accessor_local_ref() {
    let router = make_router(0xAA);
    assert_eq!(router.local().node_id, node_id(0xAA));
}

#[test]
fn accessor_local_mut() {
    let mut router = make_router(0xAA);
    router.local_mut().discovery_sequence = 999;
    assert_eq!(router.local().discovery_sequence, 999);
}

#[test]
fn accessor_routing_table_ref() {
    let router = make_router(0xAA);
    assert!(router.routing_table().is_empty());
}

#[test]
fn heartbeat_interval_default() {
    let router = make_router(0xAA);
    assert_eq!(router.heartbeat_interval_secs(), 5);
}

#[test]
fn heartbeat_interval_setter() {
    let mut router = make_router(0xAA);
    router.set_heartbeat_interval_secs(30);
    assert_eq!(router.heartbeat_interval_secs(), 30);
}

// =======================================================================
// 9. build_discovery_frame and process_discovery integration
// =======================================================================

#[test]
fn build_discovery_frame_payload_encodes_routing_table() {
    let mut router = make_router(0xAA);
    // Add a route via discovery
    router.process_discovery(
        node_id(0xBB),
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![],
        },
        1000,
    );
    let frame = router.build_discovery_frame();
    let packet = DiscoveryPacket::decode(&frame.payload).unwrap();
    assert!(!packet.routes.is_empty());
    assert!(packet.routes.iter().any(|r| r.destination == node_id(0xBB)));
}

#[test]
fn process_discovery_returns_changed_flag() {
    let mut router = make_router(0xAA);
    // First discovery: new routes -> changed
    let packet = DiscoveryPacket {
        sequence: 1,
        routes: vec![MeshRoute {
            destination: node_id(0xCC),
            next_hop: None,
            cost: 1,
        }],
    };
    assert!(router.process_discovery(node_id(0xBB), &packet, 1000));

    // Second discovery with same info: no improvement -> not changed
    let packet2 = DiscoveryPacket {
        sequence: 2,
        routes: vec![MeshRoute {
            destination: node_id(0xCC),
            next_hop: None,
            cost: 1,
        }],
    };
    assert!(!router.process_discovery(node_id(0xBB), &packet2, 2000));
}

#[test]
fn discovery_frame_sequence_increments() {
    let mut router = make_router(0xAA);
    let frame1 = router.build_discovery_frame();
    let pkt1 = DiscoveryPacket::decode(&frame1.payload).unwrap();
    assert_eq!(pkt1.sequence, 1);

    let frame2 = router.build_discovery_frame();
    let pkt2 = DiscoveryPacket::decode(&frame2.payload).unwrap();
    assert_eq!(pkt2.sequence, 2);
}

// =======================================================================
// 10. Edge cases: empty tables, single node, disconnected nodes
// =======================================================================

#[test]
fn single_node_no_peers() {
    let mut router = make_router(0xAA);
    // No peers, build discovery
    let frame = router.build_discovery_frame();
    let packet = DiscoveryPacket::decode(&frame.payload).unwrap();
    assert_eq!(packet.routes.len(), 0);
    assert_eq!(packet.sequence, 1);

    assert_eq!(router.active_peers().len(), 0);
    assert!(!router.has_route_to(&node_id(0xBB)));
}

#[test]
fn empty_discovery_packet_no_routes() {
    let packet = DiscoveryPacket {
        sequence: 100,
        routes: vec![],
    };
    let encoded = packet.encode();
    let decoded = DiscoveryPacket::decode(&encoded).unwrap();
    assert_eq!(decoded.sequence, 100);
    assert!(decoded.routes.is_empty());
}

#[test]
fn disconnected_nodes_no_routes() {
    let a = make_router(0xAA);
    let b = make_router(0xBB);
    // Neither knows about the other
    assert!(!a.has_route_to(&node_id(0xBB)));
    assert!(!b.has_route_to(&node_id(0xAA)));
    assert!(a.next_hop_for(&node_id(0xBB)).is_none());
    assert!(b.next_hop_for(&node_id(0xAA)).is_none());
}

#[test]
fn remove_nonexistent_peer_no_panic() {
    let mut router = make_router(0xAA);
    router.remove_peer(&node_id(0xFF));
    // Should be a no-op
    assert_eq!(router.all_peers().len(), 0);
}

#[test]
fn remove_peer_also_removes_routes() {
    let mut router = make_router(0xAA);
    router.process_discovery(
        node_id(0xBB),
        &DiscoveryPacket {
            sequence: 1,
            routes: vec![MeshRoute {
                destination: node_id(0xCC),
                next_hop: None,
                cost: 1,
            }],
        },
        1000,
    );

    assert!(router.has_route_to(&node_id(0xBB)));
    assert!(router.has_route_to(&node_id(0xCC)));

    router.remove_peer(&node_id(0xBB));

    assert!(!router.has_route_to(&node_id(0xBB)));
    assert!(!router.has_route_to(&node_id(0xCC)));
}

#[test]
fn routing_table_best_route_on_empty() {
    let table = MeshRoutingTable::default();
    assert!(table.best_route().is_none());
}

#[test]
fn routing_table_lookup_nonexistent_destination() {
    let table = MeshRoutingTable::default();
    assert!(table.lookup(&node_id(0xFF)).is_none());
}

#[test]
fn mesh_route_clone_debug_eq() {
    let route = MeshRoute {
        destination: node_id(0xAA),
        next_hop: Some(node_id(0xBB)),
        cost: 3,
    };
    let cloned = route.clone();
    assert_eq!(route, cloned);
    let debug = format!("{route:?}");
    assert!(debug.contains("MeshRoute"));
}

#[test]
fn mesh_peer_is_dead_threshold() {
    let peer = MeshPeer {
        node_id: node_id(0xAA),
        advertised_routes: vec![],
        last_seen_unix: 1000,
        missed_heartbeats: 0,
    };
    assert!(!peer.is_dead());

    let peer_dead = MeshPeer {
        missed_heartbeats: 3,
        ..peer.clone()
    };
    assert!(peer_dead.is_dead());

    let peer_dead_2 = MeshPeer {
        missed_heartbeats: 2,
        ..peer
    };
    assert!(!peer_dead_2.is_dead());
}

#[test]
fn mesh_peer_clone_debug() {
    let peer = MeshPeer {
        node_id: node_id(0xAA),
        advertised_routes: vec![],
        last_seen_unix: 1000,
        missed_heartbeats: 0,
    };
    let cloned = peer.clone();
    assert_eq!(peer.node_id, cloned.node_id);
    let debug = format!("{peer:?}");
    assert!(debug.contains("MeshPeer"));
}

#[test]
fn local_node_next_sequence_starts_at_one() {
    let mut local = LocalNode::new(node_id(0xAA));
    assert_eq!(local.discovery_sequence, 0);
    assert_eq!(local.next_sequence(), 1);
    assert_eq!(local.next_sequence(), 2);
    assert_eq!(local.next_sequence(), 3);
}

#[test]
fn local_node_clone() {
    let mut local = LocalNode::new(node_id(0xAA));
    local.next_sequence();
    let cloned = local.clone();
    assert_eq!(cloned.discovery_sequence, 1);
}

#[test]
fn mesh_router_clone() {
    let router = make_router(0xAA);
    let cloned = router.clone();
    assert_eq!(router.node_id(), cloned.node_id());
    assert_eq!(router.default_ttl(), cloned.default_ttl());
    assert_eq!(
        router.heartbeat_interval_secs(),
        cloned.heartbeat_interval_secs()
    );
}

#[test]
fn frame_type_unknown_variant() {
    let ft = FrameType::Unknown(99);
    let debug = format!("{ft:?}");
    assert!(debug.contains("Unknown"));
}

#[test]
fn node_id_equality() {
    let a = node_id(0xAA);
    let b = node_id(0xAA);
    let c = node_id(0xBB);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn node_id_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(node_id(0xAA));
    set.insert(node_id(0xBB));
    assert_eq!(set.len(), 2);
    assert!(set.contains(&node_id(0xAA)));
    assert!(!set.contains(&node_id(0xCC)));
}

#[test]
fn multiple_discoveries_converge_full_topology() {
    // Star topology: center node learns all leaves, leaves learn center
    let mut center = make_router(0x01);
    let mut leaf1 = make_router(0x02);
    let mut leaf2 = make_router(0x03);
    let mut leaf3 = make_router(0x04);

    // All leaves discover center
    for leaf in [&mut leaf1, &mut leaf2, &mut leaf3] {
        let pkt = DiscoveryPacket::decode(&center.build_discovery_frame().payload).unwrap();
        leaf.process_discovery(node_id(0x01), &pkt, 1000);
    }

    // Center discovers all leaves
    for leaf_id in [0x02, 0x03, 0x04] {
        let pkt =
            DiscoveryPacket::decode(&make_router(leaf_id).build_discovery_frame().payload).unwrap();
        center.process_discovery(node_id(leaf_id), &pkt, 1000);
    }

    // Center should have routes to all leaves
    assert!(center.has_route_to(&node_id(0x02)));
    assert!(center.has_route_to(&node_id(0x03)));
    assert!(center.has_route_to(&node_id(0x04)));
    assert_eq!(center.active_peers().len(), 3);

    // Each leaf should have a route to center
    for leaf in [&leaf1, &leaf2, &leaf3] {
        assert!(leaf.has_route_to(&node_id(0x01)));
    }
}

#[test]
fn route_cost_accumulation_across_many_hops() {
    // Linear chain of 6 nodes: A-B-C-D-E-F
    // F should reach A at cost 5
    let mut routers: Vec<MeshRouter> = (0..6).map(|i| make_router(0x10 + i)).collect();

    // Each pair exchanges discovery (simulating propagation)
    for i in 0..5 {
        let pkt = DiscoveryPacket::decode(&routers[i].build_discovery_frame().payload).unwrap();
        let sender_id = routers[i].node_id();
        routers[i + 1].process_discovery(sender_id, &pkt, 1000);
    }

    // Last node (index 5, ID 0x15) should reach first node (0x10) at cost 5
    let last = &routers[5];
    let route = last.routing_table().lookup(&node_id(0x10)).unwrap();
    assert_eq!(route.cost, 5);
}

#[test]
fn bellman_ford_converges_to_best_path() {
    // A-B-D  cost 2
    // A-C-D  cost 2
    // Both paths should be discovered, first one wins
    let mut a = make_router(0xAA);
    let mut b = make_router(0xBB);
    let mut c = make_router(0xCC);
    let mut d = make_router(0xDD);

    // A->B
    let pkt_a = DiscoveryPacket::decode(&a.build_discovery_frame().payload).unwrap();
    b.process_discovery(node_id(0xAA), &pkt_a, 1000);

    // A->C
    let pkt_a2 = DiscoveryPacket::decode(&a.build_discovery_frame().payload).unwrap();
    c.process_discovery(node_id(0xAA), &pkt_a2, 1000);

    // B->D (B advertises A at cost 1)
    let pkt_b = DiscoveryPacket::decode(&b.build_discovery_frame().payload).unwrap();
    d.process_discovery(node_id(0xBB), &pkt_b, 1000);

    // C->D (C advertises A at cost 1)
    let pkt_c = DiscoveryPacket::decode(&c.build_discovery_frame().payload).unwrap();
    d.process_discovery(node_id(0xCC), &pkt_c, 1000);

    // D should reach A at cost 2 (either via B or C, first learned wins)
    let route = d.routing_table().lookup(&node_id(0xAA)).unwrap();
    assert_eq!(route.cost, 2);
    // Could be via BB or CC depending on processing order
    assert!(route.next_hop == Some(node_id(0xBB)) || route.next_hop == Some(node_id(0xCC)));
}
