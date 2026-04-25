use super::*;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
use edgerun_crypto::p256::ecdsa::Signature;
use edgerun_crypto::p256::ecdsa::VerifyingKey;
use edgerun_hardware_signing::{NodeID, MESH_PUBLIC_KEY_LENGTH, MESH_SIGNATURE_LENGTH};

use super::*;
use edgerun_crypto::p256::ecdsa::SigningKey;
use edgerun_hardware_signing::{HardwareSignatureAlgorithm, HardwareSigningError};

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn test_node_id(v: u8) -> NodeID {
    let mut bytes = [0u8; 64];
    bytes[0] = v;
    NodeID(bytes)
}

fn node_id_with_pattern(v: u8) -> NodeID {
    let mut bytes = [0u8; 64];
    for b in bytes.iter_mut() {
        *b = v;
    }
    NodeID(bytes)
}

/// Creates a real P-256 keypair and returns (NodeID, signing_key).
fn make_real_keypair() -> (NodeID, SigningKey) {
    let mut bytes = [0u8; 32];
    edgerun_crypto::getrandom::fill(&mut bytes).expect("getrandom failed");
    let signing_key = SigningKey::from_bytes(&bytes.into()).unwrap();
    let encoded = signing_key.verifying_key().to_encoded_point(false);
    let bytes = encoded.as_bytes();
    // Skip the 0x04 prefix, take x||y (64 bytes)
    let mut node_bytes = [0u8; 64];
    node_bytes.copy_from_slice(&bytes[1..65]);
    (NodeID(node_bytes), signing_key)
}

// -----------------------------------------------------------------------
// FrameType tests
// -----------------------------------------------------------------------

#[test]
fn frame_type_from_u8_known_values() {
    assert_eq!(FrameType::from_u8(0), FrameType::Data);
    assert_eq!(FrameType::from_u8(1), FrameType::Discovery);
    assert_eq!(FrameType::from_u8(2), FrameType::RouteAdv);
    assert_eq!(FrameType::from_u8(3), FrameType::HandshakeInit);
    assert_eq!(FrameType::from_u8(4), FrameType::HandshakeAccept);
}

#[test]
fn frame_type_from_u8_unknown_values() {
    assert_eq!(FrameType::from_u8(8), FrameType::Unknown(8));
    assert_eq!(FrameType::from_u8(99), FrameType::Unknown(99));
    assert_eq!(FrameType::from_u8(255), FrameType::Unknown(255));
}

#[test]
fn frame_type_wire_encoding() {
    // FrameType has Unknown(u8) so `as u8` doesn't work directly.
    // Test via header encode/decode roundtrip instead.
    let header = MeshFrameHeader {
        dest: test_node_id(1),
        src: test_node_id(2),
        ttl: 1,
        frame_type: FrameType::Data,
    };
    let encoded = header.encode();
    assert_eq!(encoded[129], 0);

    let header = MeshFrameHeader {
        dest: test_node_id(1),
        src: test_node_id(2),
        ttl: 1,
        frame_type: FrameType::Discovery,
    };
    let encoded = header.encode();
    assert_eq!(encoded[129], 1);

    let header = MeshFrameHeader {
        dest: test_node_id(1),
        src: test_node_id(2),
        ttl: 1,
        frame_type: FrameType::RouteAdv,
    };
    let encoded = header.encode();
    assert_eq!(encoded[129], 2);

    let header = MeshFrameHeader {
        dest: test_node_id(1),
        src: test_node_id(2),
        ttl: 1,
        frame_type: FrameType::HandshakeInit,
    };
    let encoded = header.encode();
    assert_eq!(encoded[129], 3);

    let header = MeshFrameHeader {
        dest: test_node_id(1),
        src: test_node_id(2),
        ttl: 1,
        frame_type: FrameType::HandshakeAccept,
    };
    let encoded = header.encode();
    assert_eq!(encoded[129], 4);

    let header = MeshFrameHeader {
        dest: test_node_id(1),
        src: test_node_id(2),
        ttl: 1,
        frame_type: FrameType::Unknown(42),
    };
    let encoded = header.encode();
    assert_eq!(encoded[129], 42);
}

#[test]
fn frame_type_clone_copy_hash_eq() {
    let a = FrameType::Data;
    let b = a;
    assert_eq!(a, b);

    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(FrameType::Data);
    set.insert(FrameType::Discovery);
    assert_eq!(set.len(), 2);
    assert!(set.contains(&FrameType::Data));
}

#[test]
fn frame_type_debug() {
    let debug_str = format!("{:?}", FrameType::Data);
    assert!(debug_str.contains("Data"));
}

// -----------------------------------------------------------------------
// MeshFrameHeader tests
// -----------------------------------------------------------------------

#[test]
fn header_size_constant() {
    assert_eq!(MeshFrameHeader::SIZE, 130);
    assert_eq!(MeshFrameHeader::SIZE, MESH_PUBLIC_KEY_LENGTH * 2 + 2);
}

#[test]
fn header_encode_decode_all_frame_types() {
    let dest = test_node_id(0xAA);
    let src = test_node_id(0xBB);
    let frame_types = [
        FrameType::Data,
        FrameType::Discovery,
        FrameType::RouteAdv,
        FrameType::HandshakeInit,
        FrameType::HandshakeAccept,
        FrameType::Unknown(42),
        FrameType::Unknown(255),
    ];

    for ft in frame_types {
        let header = MeshFrameHeader {
            dest,
            src,
            ttl: 7,
            frame_type: ft,
        };
        let encoded = header.encode();
        let decoded = MeshFrameHeader::decode(&encoded);
        assert_eq!(decoded.dest, dest, "frame_type {:?}", ft);
        assert_eq!(decoded.src, src, "frame_type {:?}", ft);
        assert_eq!(decoded.ttl, 7, "frame_type {:?}", ft);
        assert_eq!(decoded.frame_type, ft, "frame_type {:?}", ft);
    }
}

#[test]
fn header_encode_dest_and_src_layout() {
    let dest_bytes = [0xAAu8; 64];
    let src_bytes = [0xBBu8; 64];
    let header = MeshFrameHeader {
        dest: NodeID(dest_bytes),
        src: NodeID(src_bytes),
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let encoded = header.encode();
    assert_eq!(&encoded[..64], &dest_bytes);
    assert_eq!(&encoded[64..128], &src_bytes);
    assert_eq!(encoded[128], 10);
    assert_eq!(encoded[129], 0); // Data
}

#[test]
fn header_ttl_boundary_values() {
    for ttl in [0u8, 1, 127, 254, 255] {
        let header = MeshFrameHeader {
            dest: test_node_id(1),
            src: test_node_id(2),
            ttl,
            frame_type: FrameType::Data,
        };
        let encoded = header.encode();
        let decoded = MeshFrameHeader::decode(&encoded);
        assert_eq!(decoded.ttl, ttl);
    }
}

#[test]
fn header_clone_copy_eq() {
    let header = MeshFrameHeader {
        dest: test_node_id(1),
        src: test_node_id(2),
        ttl: 5,
        frame_type: FrameType::Discovery,
    };
    let cloned = header.clone();
    assert_eq!(header, cloned);
}

#[test]
fn header_debug_format() {
    let header = MeshFrameHeader {
        dest: test_node_id(1),
        src: test_node_id(2),
        ttl: 3,
        frame_type: FrameType::Data,
    };
    let debug_str = format!("{:?}", header);
    assert!(debug_str.contains("MeshFrameHeader"));
    assert!(debug_str.contains("dest"));
    assert!(debug_str.contains("src"));
    assert!(debug_str.contains("ttl"));
    assert!(debug_str.contains("frame_type"));
}

#[test]
fn header_hash_eq() {
    use std::collections::HashSet;
    let h1 = MeshFrameHeader {
        dest: test_node_id(1),
        src: test_node_id(2),
        ttl: 1,
        frame_type: FrameType::Data,
    };
    let h2 = h1;
    let h3 = MeshFrameHeader {
        dest: test_node_id(3),
        src: test_node_id(2),
        ttl: 1,
        frame_type: FrameType::Data,
    };
    assert_eq!(h1, h2);
    assert_ne!(h1, h3);

    let mut set = HashSet::new();
    set.insert(h1);
    set.insert(h3);
    assert_eq!(set.len(), 2);
}

// -----------------------------------------------------------------------
// MeshFrame tests
// -----------------------------------------------------------------------

#[test]
fn frame_wire_roundtrip_empty_payload() {
    let header = MeshFrameHeader {
        dest: test_node_id(0x01),
        src: test_node_id(0x02),
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let frame = MeshFrame {
        header,
        payload: Vec::new(),
        signature: [0x42u8; MESH_SIGNATURE_LENGTH],
    };
    let wire = frame.to_wire();
    assert_eq!(wire.len(), MeshFrameHeader::SIZE + MESH_SIGNATURE_LENGTH);
    let recovered = MeshFrame::from_wire(&wire).expect("parse should succeed");
    assert_eq!(recovered.header, header);
    assert!(recovered.payload.is_empty());
    assert_eq!(recovered.signature, frame.signature);
}

#[test]
fn frame_wire_roundtrip_with_payload() {
    let header = MeshFrameHeader {
        dest: test_node_id(0x01),
        src: test_node_id(0x02),
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let payload = b"hello mesh".to_vec();
    let sig = [0x42u8; MESH_SIGNATURE_LENGTH];
    let frame = MeshFrame {
        header,
        payload: payload.clone(),
        signature: sig,
    };
    let wire = frame.to_wire();
    assert_eq!(
        wire.len(),
        MeshFrameHeader::SIZE + payload.len() + MESH_SIGNATURE_LENGTH
    );
    let recovered = MeshFrame::from_wire(&wire).expect("parse should succeed");
    assert_eq!(recovered.payload, payload);
    assert_eq!(recovered.signature, sig);
}

#[test]
fn frame_wire_roundtrip_large_payload() {
    let header = MeshFrameHeader {
        dest: test_node_id(0x01),
        src: test_node_id(0x02),
        ttl: 255,
        frame_type: FrameType::RouteAdv,
    };
    let payload = vec![0xABu8; 10_000];
    let sig = [0xFFu8; MESH_SIGNATURE_LENGTH];
    let frame = MeshFrame {
        header,
        payload,
        signature: sig,
    };
    let wire = frame.to_wire();
    let recovered = MeshFrame::from_wire(&wire).unwrap();
    assert_eq!(recovered.header.ttl, 255);
    assert_eq!(recovered.payload.len(), 10_000);
    assert_eq!(recovered.signature, sig);
}

#[test]
fn frame_from_wire_rejects_too_short() {
    assert!(MeshFrame::from_wire(&[0u8; 50]).is_none());
    assert!(MeshFrame::from_wire(&[0u8; 193]).is_none()); // 130 + 64 - 1
}

#[test]
fn frame_from_wire_exact_min_length() {
    let min_len = MeshFrameHeader::SIZE + MESH_SIGNATURE_LENGTH;
    let buf = [0u8; 194]; // 130 + 64
    let result = MeshFrame::from_wire(&buf);
    assert!(result.is_some());
    let frame = result.unwrap();
    assert!(frame.payload.is_empty());
    assert_eq!(frame.signature, [0u8; MESH_SIGNATURE_LENGTH]);
}

#[test]
fn frame_signed_preimage() {
    let header = MeshFrameHeader {
        dest: test_node_id(1),
        src: test_node_id(2),
        ttl: 3,
        frame_type: FrameType::Discovery,
    };
    let payload = b"test".to_vec();
    let frame = MeshFrame {
        header,
        payload,
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    let preimage = frame.signed_preimage();
    assert_eq!(preimage.len(), MeshFrameHeader::SIZE + 4);
    assert_eq!(&preimage[..MeshFrameHeader::SIZE], &header.encode());
    assert_eq!(&preimage[MeshFrameHeader::SIZE..], b"test");
}

#[test]
fn frame_from_payload_defaults() {
    let dest = test_node_id(0xCC);
    let payload = b"data".to_vec();
    let frame = MeshFrame::from_payload(dest, payload.clone());
    assert_eq!(frame.header.dest, dest);
    assert_eq!(frame.header.src, NodeID([0u8; 64]));
    assert_eq!(frame.header.ttl, 16);
    assert_eq!(frame.header.frame_type, FrameType::Data);
    assert_eq!(frame.payload, payload);
    assert_eq!(frame.signature, [0u8; MESH_SIGNATURE_LENGTH]);
}

#[test]
fn frame_clone() {
    let frame = MeshFrame::from_payload(test_node_id(1), b"x".to_vec());
    let cloned = frame.clone();
    assert_eq!(frame, cloned);
}

#[test]
fn frame_debug() {
    let frame = MeshFrame::from_payload(test_node_id(1), b"test".to_vec());
    let debug_str = format!("{:?}", frame);
    assert!(debug_str.contains("MeshFrame"));
}

#[test]
fn frame_eq() {
    let f1 = MeshFrame::from_payload(test_node_id(1), b"same".to_vec());
    let f2 = f1.clone();
    let f3 = MeshFrame::from_payload(test_node_id(2), b"same".to_vec());
    assert_eq!(f1, f2);
    assert_ne!(f1, f3);
}

// -----------------------------------------------------------------------
// MeshRoute tests
// -----------------------------------------------------------------------

#[test]
fn mesh_route_clone_debug_eq() {
    let route = MeshRoute {
        destination: test_node_id(0xAA),
        next_hop: Some(test_node_id(0xBB)),
        cost: 3,
    };
    let cloned = route.clone();
    assert_eq!(route, cloned);

    let debug_str = format!("{:?}", route);
    assert!(debug_str.contains("MeshRoute"));
}

#[test]
fn mesh_route_direct_vs_indirect() {
    let direct = MeshRoute {
        destination: test_node_id(1),
        next_hop: None,
        cost: 1,
    };
    let indirect = MeshRoute {
        destination: test_node_id(1),
        next_hop: Some(test_node_id(2)),
        cost: 3,
    };
    assert!(direct.cost < indirect.cost);
    assert!(direct.next_hop.is_none());
    assert!(indirect.next_hop.is_some());
}

// -----------------------------------------------------------------------
// MeshRoutingTable tests
// -----------------------------------------------------------------------

#[test]
fn routing_table_default_is_empty() {
    let table = MeshRoutingTable::default();
    assert!(table.is_empty());
    assert_eq!(table.len(), 0);
    assert!(table.iter().next().is_none());
}

#[test]
fn routing_table_insert_and_lookup() {
    let dest = test_node_id(0xAA);
    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: dest,
        next_hop: None,
        cost: 1,
    });
    assert_eq!(table.len(), 1);
    assert!(!table.is_empty());

    let route = table.lookup(&dest).expect("should find route");
    assert_eq!(route.destination, dest);
    assert_eq!(route.cost, 1);
    assert!(route.next_hop.is_none());
}

#[test]
fn routing_table_lookup_missing() {
    let table = MeshRoutingTable::default();
    assert!(table.lookup(&test_node_id(0xFF)).is_none());
}

#[test]
fn routing_table_best_path_selection() {
    let a = test_node_id(0xAA);
    let b = test_node_id(0xBB);
    let c = test_node_id(0xCC);

    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: b,
        next_hop: None,
        cost: 1,
    });
    table.update(MeshRoute {
        destination: c,
        next_hop: Some(b),
        cost: 2,
    });

    let route_to_b = table.lookup(&b).expect("should find B");
    assert_eq!(route_to_b.cost, 1);
    assert!(route_to_b.next_hop.is_none());

    let route_to_c = table.lookup(&c).expect("should find C");
    assert_eq!(route_to_c.cost, 2);
    assert_eq!(route_to_c.next_hop, Some(b));
}

#[test]
fn routing_table_rejects_worse_route() {
    let c = test_node_id(0xCC);
    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: c,
        next_hop: Some(test_node_id(0xBB)),
        cost: 2,
    });

    table.update(MeshRoute {
        destination: c,
        next_hop: Some(test_node_id(0xAA)),
        cost: 5,
    });
    let route = table.lookup(&c).unwrap();
    assert_eq!(route.cost, 2);
}

#[test]
fn routing_table_accepts_better_route() {
    let a = test_node_id(0xAA);
    let b = test_node_id(0xBB);
    let c = test_node_id(0xCC);

    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: c,
        next_hop: Some(b),
        cost: 5,
    });

    table.update(MeshRoute {
        destination: c,
        next_hop: Some(a),
        cost: 1,
    });
    let route = table.lookup(&c).unwrap();
    assert_eq!(route.cost, 1);
    assert_eq!(route.next_hop, Some(a));
}

#[test]
fn routing_table_equal_cost_keeps_existing() {
    let b = test_node_id(0xBB);
    let c = test_node_id(0xCC);

    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: c,
        next_hop: Some(b),
        cost: 3,
    });

    table.update(MeshRoute {
        destination: c,
        next_hop: Some(test_node_id(0xAA)),
        cost: 3,
    });
    let route = table.lookup(&c).unwrap();
    assert_eq!(route.cost, 3);
    assert_eq!(route.next_hop, Some(b));
}

#[test]
fn routing_table_best_route() {
    let a = test_node_id(0xAA);
    let b = test_node_id(0xBB);
    let c = test_node_id(0xCC);

    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: a,
        next_hop: None,
        cost: 5,
    });
    table.update(MeshRoute {
        destination: b,
        next_hop: None,
        cost: 1,
    });
    table.update(MeshRoute {
        destination: c,
        next_hop: None,
        cost: 3,
    });

    let best = table.best_route().expect("should have a best route");
    assert_eq!(best.destination, b);
    assert_eq!(best.cost, 1);
}

#[test]
fn routing_table_best_route_empty() {
    let table = MeshRoutingTable::default();
    assert!(table.best_route().is_none());
}

#[test]
fn routing_table_remove_via_clears_indirect_routes() {
    let b = test_node_id(0xBB);
    let c = test_node_id(0xCC);
    let d = test_node_id(0xDD);

    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: b,
        next_hop: None,
        cost: 1,
    });
    table.update(MeshRoute {
        destination: c,
        next_hop: Some(b),
        cost: 2,
    });
    table.update(MeshRoute {
        destination: d,
        next_hop: Some(b),
        cost: 3,
    });

    table.remove_via(&b);

    assert!(table.lookup(&b).is_some());
    assert!(table.lookup(&c).is_none());
    assert!(table.lookup(&d).is_none());
}

#[test]
fn routing_table_remove_via_no_effect_on_other_hops() {
    let b = test_node_id(0xBB);
    let c = test_node_id(0xCC);
    let d = test_node_id(0xDD);
    let x = test_node_id(0xEE);

    let mut table = MeshRoutingTable::default();
    // Route to C via B at cost 2
    table.update(MeshRoute {
        destination: c,
        next_hop: Some(b),
        cost: 2,
    });
    // Route to D via X at cost 3 (different destination, so both stored)
    table.update(MeshRoute {
        destination: d,
        next_hop: Some(x),
        cost: 3,
    });

    table.remove_via(&b);

    // Route to C via B is removed
    assert!(table.lookup(&c).is_none());
    // Route to D via X is unaffected
    let route = table.lookup(&d).unwrap();
    assert_eq!(route.next_hop, Some(x));
    assert_eq!(route.cost, 3);
}

#[test]
fn routing_table_remove_destination() {
    let a = test_node_id(0xAA);
    let b = test_node_id(0xBB);

    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: a,
        next_hop: None,
        cost: 1,
    });
    table.update(MeshRoute {
        destination: b,
        next_hop: None,
        cost: 1,
    });

    table.remove_destination(&a);
    assert!(table.lookup(&a).is_none());
    assert!(table.lookup(&b).is_some());
    assert_eq!(table.len(), 1);
}

#[test]
fn routing_table_remove_destination_nonexistent() {
    let mut table = MeshRoutingTable::default();
    table.remove_destination(&test_node_id(0xFF));
    assert!(table.is_empty());
}

#[test]
fn routing_table_iter() {
    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: test_node_id(1),
        next_hop: None,
        cost: 1,
    });
    table.update(MeshRoute {
        destination: test_node_id(2),
        next_hop: None,
        cost: 2,
    });

    let routes: Vec<_> = table.iter().collect();
    assert_eq!(routes.len(), 2);
}

#[test]
fn routing_table_clone() {
    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: test_node_id(1),
        next_hop: None,
        cost: 1,
    });
    let cloned = table.clone();
    assert_eq!(cloned.len(), table.len());
    assert_eq!(cloned.lookup(&test_node_id(1)).unwrap().cost, 1);
}

#[test]
fn routing_table_multiple_routes_to_same_dest_best_wins() {
    let dest = test_node_id(0xAA);
    let mut table = MeshRoutingTable::default();

    table.update(MeshRoute {
        destination: dest,
        next_hop: Some(test_node_id(0xBB)),
        cost: 5,
    });
    table.update(MeshRoute {
        destination: dest,
        next_hop: Some(test_node_id(0xCC)),
        cost: 2,
    });
    table.update(MeshRoute {
        destination: dest,
        next_hop: Some(test_node_id(0xDD)),
        cost: 3,
    });

    let best = table.lookup(&dest).unwrap();
    assert_eq!(best.cost, 2);
    assert_eq!(best.next_hop, Some(test_node_id(0xCC)));
}

// -----------------------------------------------------------------------
// MeshPeer tests
// -----------------------------------------------------------------------

#[test]
fn mesh_peer_not_dead_with_zero_misses() {
    let peer = MeshPeer {
        node_id: test_node_id(0xDD),
        advertised_routes: Vec::new(),
        last_seen_unix: 1000,
        missed_heartbeats: 0,
    };
    assert!(!peer.is_dead());
}

#[test]
fn mesh_peer_not_dead_with_two_misses() {
    let peer = MeshPeer {
        node_id: test_node_id(0xDD),
        advertised_routes: Vec::new(),
        last_seen_unix: 1000,
        missed_heartbeats: 2,
    };
    assert!(!peer.is_dead());
}

#[test]
fn mesh_peer_dead_at_three_misses() {
    let peer = MeshPeer {
        node_id: test_node_id(0xDD),
        advertised_routes: Vec::new(),
        last_seen_unix: 1000,
        missed_heartbeats: 3,
    };
    assert!(peer.is_dead());
}

#[test]
fn mesh_peer_dead_above_three_misses() {
    let peer = MeshPeer {
        node_id: test_node_id(0xDD),
        advertised_routes: Vec::new(),
        last_seen_unix: 1000,
        missed_heartbeats: 10,
    };
    assert!(peer.is_dead());
}

#[test]
fn mesh_peer_clone() {
    let peer = MeshPeer {
        node_id: test_node_id(0xDD),
        advertised_routes: vec![MeshRoute {
            destination: test_node_id(0xEE),
            next_hop: None,
            cost: 1,
        }],
        last_seen_unix: 1_700_000_000,
        missed_heartbeats: 1,
    };
    let cloned = peer.clone();
    assert_eq!(cloned.node_id, peer.node_id);
    assert_eq!(cloned.advertised_routes.len(), 1);
    assert_eq!(cloned.last_seen_unix, peer.last_seen_unix);
    assert_eq!(cloned.missed_heartbeats, peer.missed_heartbeats);
}

#[test]
fn mesh_peer_debug() {
    let peer = MeshPeer {
        node_id: test_node_id(0xDD),
        advertised_routes: Vec::new(),
        last_seen_unix: 1000,
        missed_heartbeats: 0,
    };
    let debug_str = format!("{:?}", peer);
    assert!(debug_str.contains("MeshPeer"));
}

#[test]
fn mesh_peer_with_advertised_routes() {
    let routes = vec![
        MeshRoute {
            destination: test_node_id(1),
            next_hop: None,
            cost: 1,
        },
        MeshRoute {
            destination: test_node_id(2),
            next_hop: Some(test_node_id(1)),
            cost: 2,
        },
    ];
    let peer = MeshPeer {
        node_id: test_node_id(0xDD),
        advertised_routes: routes.clone(),
        last_seen_unix: 1000,
        missed_heartbeats: 0,
    };
    assert_eq!(peer.advertised_routes.len(), 2);
    assert_eq!(peer.advertised_routes, routes);
}

// -----------------------------------------------------------------------
// DiscoveryPayload tests
// -----------------------------------------------------------------------

#[test]
fn discovery_payload_clone_debug() {
    let payload = DiscoveryPayload {
        sequence: 42,
        routes: vec![MeshRoute {
            destination: test_node_id(1),
            next_hop: None,
            cost: 1,
        }],
    };
    let cloned = payload.clone();
    assert_eq!(cloned.sequence, 42);
    assert_eq!(cloned.routes.len(), 1);

    let debug_str = format!("{:?}", payload);
    assert!(debug_str.contains("DiscoveryPayload"));
}

#[test]
fn discovery_payload_empty_routes() {
    let payload = DiscoveryPayload {
        sequence: 0,
        routes: Vec::new(),
    };
    assert_eq!(payload.sequence, 0);
    assert!(payload.routes.is_empty());
}

#[test]
fn discovery_payload_large_sequence() {
    let payload = DiscoveryPayload {
        sequence: u32::MAX,
        routes: Vec::new(),
    };
    assert_eq!(payload.sequence, u32::MAX);
}

// -----------------------------------------------------------------------
// LocalNode tests
// -----------------------------------------------------------------------

#[test]
fn local_node_initial_state() {
    let node = LocalNode::new(test_node_id(0xEE));
    assert_eq!(node.node_id, test_node_id(0xEE));
    assert_eq!(node.discovery_sequence, 0);
}

#[test]
fn local_node_sequence_monotonically_increases() {
    let mut node = LocalNode::new(test_node_id(0xEE));
    assert_eq!(node.next_sequence(), 1);
    assert_eq!(node.next_sequence(), 2);
    assert_eq!(node.next_sequence(), 3);
}

#[test]
fn local_node_sequence_after_many_calls() {
    let mut node = LocalNode::new(test_node_id(0xEE));
    for i in 1..=1000 {
        assert_eq!(node.next_sequence(), i);
    }
}

#[test]
fn local_node_clone() {
    let mut node = LocalNode::new(test_node_id(0xEE));
    node.next_sequence();
    let cloned = node.clone();
    assert_eq!(cloned.discovery_sequence, 1);
    assert_eq!(cloned.node_id, node.node_id);
}

#[test]
fn local_node_debug() {
    let node = LocalNode::new(test_node_id(0xEE));
    let debug_str = format!("{:?}", node);
    assert!(debug_str.contains("LocalNode"));
}

#[test]
fn local_node_independent_sequences() {
    let mut node_a = LocalNode::new(test_node_id(0xAA));
    let mut node_b = LocalNode::new(test_node_id(0xBB));

    assert_eq!(node_a.next_sequence(), 1);
    assert_eq!(node_b.next_sequence(), 1);
    assert_eq!(node_a.next_sequence(), 2);
    assert_eq!(node_b.next_sequence(), 2);
    assert_eq!(node_a.next_sequence(), 3);

    assert_eq!(node_a.discovery_sequence, 3);
    assert_eq!(node_b.discovery_sequence, 2);
}

// -----------------------------------------------------------------------
// Frame signing and verification with real P-256 keys
// -----------------------------------------------------------------------

#[test]
fn frame_sign_and_verify_with_real_p256_key() {
    let (node_id, signing_key) = make_real_keypair();
    let dest = test_node_id(0xFF);

    let header = MeshFrameHeader {
        dest,
        src: node_id,
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let payload = b"hello mesh".to_vec();
    let mut frame = MeshFrame {
        header,
        payload: payload.clone(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };

    sign_frame(&mut frame, &signing_key);

    assert!(
        frame.verify_signature(),
        "signature should be valid on original frame"
    );

    let wire = frame.to_wire();
    let recovered = MeshFrame::from_wire(&wire).expect("wire parse should succeed");
    assert!(
        recovered.verify_signature(),
        "signature should be valid on wire frame"
    );
    assert_eq!(recovered.payload, payload);
    assert_eq!(recovered.header.src, node_id);
    assert_eq!(recovered.header.dest, dest);
}

#[test]
fn frame_sign_verify_all_frame_types() {
    let (node_id, signing_key) = make_real_keypair();
    let frame_types = [
        FrameType::Data,
        FrameType::Discovery,
        FrameType::RouteAdv,
        FrameType::HandshakeInit,
        FrameType::HandshakeAccept,
    ];

    for ft in frame_types {
        let header = MeshFrameHeader {
            dest: test_node_id(0xFF),
            src: node_id,
            ttl: 5,
            frame_type: ft,
        };
        let mut frame = MeshFrame {
            header,
            payload: b"test".to_vec(),
            signature: [0u8; MESH_SIGNATURE_LENGTH],
        };
        sign_frame(&mut frame, &signing_key);
        assert!(
            frame.verify_signature(),
            "frame_type {:?} should verify",
            ft
        );
    }
}

#[test]
fn frame_sign_verify_empty_payload() {
    let (node_id, signing_key) = make_real_keypair();
    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: node_id,
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let mut frame = MeshFrame {
        header,
        payload: Vec::new(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut frame, &signing_key);
    assert!(frame.verify_signature());
}

#[test]
fn frame_sign_verify_large_payload() {
    let (node_id, signing_key) = make_real_keypair();
    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: node_id,
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let payload = vec![0xCDu8; 8192];
    let mut frame = MeshFrame {
        header,
        payload,
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut frame, &signing_key);
    assert!(frame.verify_signature());
}

#[test]
fn frame_verify_rejects_tampered_payload() {
    let (node_id, signing_key) = make_real_keypair();

    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: node_id,
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let mut frame = MeshFrame {
        header,
        payload: b"original".to_vec(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut frame, &signing_key);

    frame.payload = b"tampered".to_vec();

    assert!(
        !frame.verify_signature(),
        "tampered payload should fail verification"
    );
}

#[test]
fn frame_verify_rejects_tampered_header() {
    let (node_id, signing_key) = make_real_keypair();

    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: node_id,
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let mut frame = MeshFrame {
        header,
        payload: b"data".to_vec(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut frame, &signing_key);

    frame.header.ttl = 5;

    assert!(
        !frame.verify_signature(),
        "tampered header should fail verification"
    );
}

#[test]
fn frame_verify_rejects_tampered_signature() {
    let (node_id, signing_key) = make_real_keypair();

    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: node_id,
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let mut frame = MeshFrame {
        header,
        payload: b"data".to_vec(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut frame, &signing_key);

    frame.signature[0] ^= 0x01;

    assert!(
        !frame.verify_signature(),
        "tampered signature should fail verification"
    );
}

#[test]
fn frame_verify_rejects_spoofed_sender() {
    let (real_node, _real_key) = make_real_keypair();
    let (_fake_node, fake_key) = make_real_keypair();

    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: real_node,
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let mut fake_frame = MeshFrame {
        header,
        payload: b"data".to_vec(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut fake_frame, &fake_key);

    assert!(
        !fake_frame.verify_signature(),
        "spoofed sender should fail verification"
    );
}

#[test]
fn frame_verify_with_invalid_public_key_returns_false() {
    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: NodeID([0u8; 64]),
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let frame = MeshFrame {
        header,
        payload: b"data".to_vec(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    assert!(
        !frame.verify_signature(),
        "invalid public key should fail verification"
    );
}

#[test]
fn frame_verify_with_invalid_signature_bytes_returns_false() {
    let (node_id, _signing_key) = make_real_keypair();

    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: node_id,
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let sig = [0xFFu8; MESH_SIGNATURE_LENGTH];
    let frame = MeshFrame {
        header,
        payload: b"data".to_vec(),
        signature: sig,
    };
    let _ = !frame.verify_signature();
}

#[test]
fn frame_sign_multiple_times_same_signature_deterministic() {
    // p256 uses RFC 6979 deterministic nonces, so same input = same signature
    let (node_id, signing_key) = make_real_keypair();
    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: node_id,
        ttl: 10,
        frame_type: FrameType::Data,
    };

    let mut frame1 = MeshFrame {
        header,
        payload: b"data".to_vec(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut frame1, &signing_key);

    let mut frame2 = MeshFrame {
        header,
        payload: b"data".to_vec(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut frame2, &signing_key);

    assert!(frame1.verify_signature());
    assert!(frame2.verify_signature());
    // RFC 6979 deterministic nonces => same signature
    assert_eq!(frame1.signature, frame2.signature);
}

#[test]
fn frame_sign_different_payload_different_signature() {
    let (node_id, signing_key) = make_real_keypair();
    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: node_id,
        ttl: 10,
        frame_type: FrameType::Data,
    };

    let mut frame1 = MeshFrame {
        header,
        payload: b"data1".to_vec(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut frame1, &signing_key);

    let mut frame2 = MeshFrame {
        header,
        payload: b"data2".to_vec(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut frame2, &signing_key);

    assert!(frame1.verify_signature());
    assert!(frame2.verify_signature());
    assert_ne!(frame1.signature, frame2.signature);
}

// -----------------------------------------------------------------------
// sign_frame function tests
// -----------------------------------------------------------------------

#[test]
fn sign_frame_updates_signature_field() {
    let (node_id, signing_key) = make_real_keypair();
    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: node_id,
        ttl: 1,
        frame_type: FrameType::Discovery,
    };
    let mut frame = MeshFrame {
        header,
        payload: vec![1, 2, 3],
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };

    assert_eq!(frame.signature, [0u8; MESH_SIGNATURE_LENGTH]);
    sign_frame(&mut frame, &signing_key);
    assert_ne!(frame.signature, [0u8; MESH_SIGNATURE_LENGTH]);
}

// -----------------------------------------------------------------------
// NodeID tests
// -----------------------------------------------------------------------

#[test]
fn node_id_display_methods() {
    let id = test_node_id(0xAB);
    let short = id.short();
    assert_eq!(short.len(), 10); // "0x" + 8 hex chars
    assert!(short.starts_with("0x"));

    let full = id.to_hex();
    assert_eq!(full.len(), 130); // "0x" + 128 hex chars
    assert!(full.starts_with("0x"));
}

#[test]
fn node_id_debug() {
    let id = test_node_id(0xCD);
    let debug_str = format!("{:?}", id);
    assert!(debug_str.starts_with("NodeID("));
}

#[test]
fn node_id_as_ref() {
    let id = test_node_id(0xEF);
    let slice: &[u8] = id.as_ref();
    assert_eq!(slice.len(), 64);
    assert_eq!(slice[0], 0xEF);
}

#[test]
fn node_id_clone_copy_eq_hash() {
    let id = test_node_id(0x12);
    let cloned = id.clone();
    assert_eq!(id, cloned);

    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(id);
    assert!(set.contains(&cloned));
}

#[test]
fn node_id_all_zeros() {
    let id = NodeID([0u8; 64]);
    assert_eq!(id.short(), "0x00000000");
}

#[test]
fn node_id_all_ones() {
    let id = NodeID([0xFFu8; 64]);
    assert_eq!(id.short(), "0xffffffff");
}

// -----------------------------------------------------------------------
// Error type and Display tests
// -----------------------------------------------------------------------

#[test]
fn hardware_signing_error_display() {
    let err = HardwareSigningError::UnsupportedAlgorithm(HardwareSignatureAlgorithm::Eddsa);
    let display = format!("{}", err);
    assert!(display.contains("unsupported hardware signature algorithm"));
}

#[test]
fn hardware_signing_error_provider() {
    let err = HardwareSigningError::Provider("test error".to_string());
    let display = format!("{}", err);
    assert_eq!(display, "test error");
}

#[test]
fn hardware_signing_error_is_std_error() {
    let err: HardwareSigningError =
        HardwareSigningError::UnsupportedAlgorithm(HardwareSignatureAlgorithm::Eddsa);
    let _e: &dyn std::error::Error = &err;
}

// -----------------------------------------------------------------------
// Edge cases
// -----------------------------------------------------------------------

#[test]
fn frame_from_wire_buffer_exactly_minimum_size() {
    let min_len = MeshFrameHeader::SIZE + MESH_SIGNATURE_LENGTH;
    let mut buf = vec![0u8; min_len];
    buf[129] = 0; // Data
    let result = MeshFrame::from_wire(&buf);
    assert!(result.is_some());
    let frame = result.unwrap();
    assert_eq!(frame.payload.len(), 0);
}

#[test]
fn frame_from_wire_with_all_frame_type_bytes() {
    let min_len = MeshFrameHeader::SIZE + MESH_SIGNATURE_LENGTH;
    for ft_byte in 0u8..=255 {
        let mut buf = vec![0u8; min_len];
        buf[129] = ft_byte;
        let result = MeshFrame::from_wire(&buf);
        assert!(result.is_some(), "failed for frame_type byte {}", ft_byte);
        let frame = result.unwrap();
        assert_eq!(
            frame.header.frame_type,
            FrameType::from_u8(ft_byte),
            "frame_type mismatch for byte {}",
            ft_byte
        );
    }
}

#[test]
fn routing_table_remove_via_nonexistent_node() {
    let mut table = MeshRoutingTable::default();
    table.update(MeshRoute {
        destination: test_node_id(1),
        next_hop: None,
        cost: 1,
    });
    table.remove_via(&test_node_id(0xFF));
    assert_eq!(table.len(), 1);
}

#[test]
fn routing_table_update_new_destination() {
    let mut table = MeshRoutingTable::default();
    assert!(table.is_empty());

    table.update(MeshRoute {
        destination: test_node_id(1),
        next_hop: Some(test_node_id(2)),
        cost: 5,
    });
    assert_eq!(table.len(), 1);

    table.update(MeshRoute {
        destination: test_node_id(1),
        next_hop: Some(test_node_id(3)),
        cost: 10,
    });
    assert_eq!(table.len(), 1);
    assert_eq!(table.lookup(&test_node_id(1)).unwrap().cost, 5);
}

#[test]
fn mesh_peer_heartbeat_tracking_simulation() {
    let mut peer = MeshPeer {
        node_id: test_node_id(0xDD),
        advertised_routes: Vec::new(),
        last_seen_unix: 1_700_000_000,
        missed_heartbeats: 0,
    };

    assert!(!peer.is_dead());
    peer.missed_heartbeats = 1;
    assert!(!peer.is_dead());
    peer.missed_heartbeats = 2;
    assert!(!peer.is_dead());
    peer.missed_heartbeats = 3;
    assert!(peer.is_dead());
}

#[test]
fn header_encode_produces_exactly_130_bytes() {
    let header = MeshFrameHeader {
        dest: test_node_id(1),
        src: test_node_id(2),
        ttl: 255,
        frame_type: FrameType::Unknown(128),
    };
    let encoded = header.encode();
    assert_eq!(encoded.len(), 130);
    assert_eq!(encoded[128], 255);
    assert_eq!(encoded[129], 128);
}

#[test]
fn frame_wire_roundtrip_preserves_all_header_fields() {
    let header = MeshFrameHeader {
        dest: node_id_with_pattern(0xAA),
        src: node_id_with_pattern(0xBB),
        ttl: 128,
        frame_type: FrameType::HandshakeInit,
    };
    let payload = vec![0x11, 0x22, 0x33, 0x44];
    let sig = [0xCCu8; MESH_SIGNATURE_LENGTH];
    let frame = MeshFrame {
        header,
        payload: payload.clone(),
        signature: sig,
    };

    let wire = frame.to_wire();
    let recovered = MeshFrame::from_wire(&wire).unwrap();

    assert_eq!(recovered.header.dest, header.dest);
    assert_eq!(recovered.header.src, header.src);
    assert_eq!(recovered.header.ttl, 128);
    assert_eq!(recovered.header.frame_type, FrameType::HandshakeInit);
    assert_eq!(recovered.payload, payload);
    assert_eq!(recovered.signature, sig);
}

#[test]
fn verify_signature_is_not_mutating() {
    let (node_id, signing_key) = make_real_keypair();
    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: node_id,
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let mut frame = MeshFrame {
        header,
        payload: b"test".to_vec(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut frame, &signing_key);

    let pre_verify_sig = frame.signature;
    let _ = frame.verify_signature();
    let post_verify_sig = frame.signature;

    assert_eq!(pre_verify_sig, post_verify_sig);
}

#[test]
fn to_wire_does_not_mutate_frame() {
    let (node_id, signing_key) = make_real_keypair();
    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: node_id,
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let mut frame = MeshFrame {
        header,
        payload: b"test".to_vec(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut frame, &signing_key);

    let pre_wire = frame.clone();
    let _ = frame.to_wire();
    assert_eq!(frame, pre_wire);
}

#[test]
fn signed_preimage_does_not_include_signature() {
    let (node_id, signing_key) = make_real_keypair();
    let header = MeshFrameHeader {
        dest: test_node_id(0xFF),
        src: node_id,
        ttl: 10,
        frame_type: FrameType::Data,
    };
    let mut frame = MeshFrame {
        header,
        payload: b"test".to_vec(),
        signature: [0u8; MESH_SIGNATURE_LENGTH],
    };
    sign_frame(&mut frame, &signing_key);

    let preimage = frame.signed_preimage();
    assert_eq!(preimage.len(), MeshFrameHeader::SIZE + 4);
}
