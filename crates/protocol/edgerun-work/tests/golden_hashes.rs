use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn hex(hash: Hash) -> String {
    let mut out = String::with_capacity(64);
    for byte in hash {
        use core::fmt::Write;
        write!(&mut out, "{byte:02x}").expect("write hex");
    }
    out
}

fn public_key_from_seed(seed: u8) -> PublicKey {
    let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    let mut out = [0u8; 32];
    out.copy_from_slice(key.verifying_key().as_bytes());
    out
}

struct GoldenFixture {
    request: WorkRequest,
    admission: WorkAdmission,
    receipt: WorkReceipt,
    route: RouteAdvertisement,
    route_snapshot: RouteSnapshot,
    ordered: OrderedChannelEnvelope,
    channel_proof: ChannelProof,
    transit_input: PacketTransitHashInput,
    manifest: ErasureManifest,
    shard: ErasureShard,
    recipient_policy: RecipientMessagePolicy,
    batch_receipts: Vec<WorkReceipt>,
}

fn fixture() -> GoldenFixture {
    let user_key = Ed25519SigningKey::from_bytes(&[1u8; 32]);
    let user = public_key_from_seed(1);
    let admission_node = SimNode::from_seed(2, NODE_ROLE_ADMISSION);
    let mut sender = SimNode::from_seed(3, NODE_ROLE_MESSAGE);
    let recipient = SimNode::from_seed(4, NODE_ROLE_MESSAGE);
    let worker = SimNode::from_seed(5, NODE_ROLE_STORAGE);
    let relay = SimNode::from_seed(6, NODE_ROLE_RELAY);

    let payload = b"golden protocol fixture payload".to_vec();
    let payload_hash = blake3_hash(&payload);
    let request = sign_work_request(
        &user_key,
        WorkRequest {
            abi_version: WORK_WIRE_ABI_VERSION,
            request_id: blake3_hash(b"golden-request-id"),
            user,
            user_sequence: 7,
            recipient: recipient.identity.node_id,
            work_type: WORK_TYPE_MESSAGE_DELIVER,
            department: DEPARTMENT_MESSAGE,
            payload_hash,
            input_root: blake3_hash(b"golden-input-root"),
            max_total_cost: 123,
            valid_until_unix_ms: 999_999,
            signature: empty_signature(),
        },
    );
    let request_hash = packet_hash(&WorkPacket::WorkRequest(request.clone())).expect("request hash");

    let route = relay.advertise_memory_route(relay.identity.node_id, vec![DEPARTMENT_RELAY, DEPARTMENT_MESSAGE]);
    let route_root = route_root_hash(core::slice::from_ref(&route));
    let route_snapshot = sign_route_snapshot(
        &admission_node.key,
        RouteSnapshot {
            abi_version: WORK_WIRE_ABI_VERSION,
            issued_by: admission_node.identity.clone(),
            sequence: 11,
            route_root,
            routes: vec![route.clone()],
            signature: empty_signature(),
        },
    );

    let admission = sign_work_admission(
        &admission_node.key,
        WorkAdmission {
            abi_version: WORK_WIRE_ABI_VERSION,
            admission_id: blake3_hash(b"golden-admission-id"),
            dao_id: admission_node.identity.public_key,
            user,
            admission_node: admission_node.identity.clone(),
            request_hash,
            assigned_route_hash: route_commitment(&route),
            assigned_channel: route.endpoint.clone(),
            admitted_budget: 123,
            policy_hash: blake3_hash(b"golden-policy"),
            sequence: 12,
            valid_until_unix_ms: 999_999,
            signature: empty_signature(),
        },
    );
    let admission_hash = work_admission_hash(&admission).expect("admission hash");

    let packet = sender.message_to(
        recipient.identity.node_id,
        relay.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        payload.clone(),
    );
    let mut channel = MemoryChannelEngine::new();
    channel.add_route(route.clone()).expect("route");
    let ordered = deliver_ordered(
        &mut channel,
        &mut sender.order,
        sender.identity.node_id,
        relay.identity.node_id,
        packet,
    )
    .expect("ordered");

    let channel_proof = sign_channel_proof(
        &recipient.key,
        ChannelProof {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: ordered.envelope.channel_id,
            relay_node_id: relay.identity.node_id,
            from: ordered.envelope.from,
            to: recipient.identity.node_id,
            message_hash: ordered_message_hash(&ordered),
            sequence: ordered.sequence,
            signature: empty_signature(),
        },
    );

    let transit_input = PacketTransitHashInput {
        node_id: relay.identity.node_id,
        from: ordered.envelope.from,
        to: recipient.identity.node_id,
        channel_id: ordered.envelope.channel_id,
        route_hash: ordered.envelope.route_hash,
        packet_hash: ordered.envelope.packet_hash,
        sequence: 1,
        previous_transit_hash: [0u8; 32],
    };

    let assigned_nodes = [worker.identity.node_id, relay.identity.node_id, recipient.identity.node_id];
    let (manifest, shards) = encode_xor_2_1(b"golden erasure payload", assigned_nodes).expect("erasure");
    let shard = shards[0].clone();

    let recipient_policy = sign_recipient_message_policy(
        &recipient.key,
        open_recipient_message_policy(recipient.identity.clone(), 1, 999_999),
    );

    let input_hash = blake3_hash(b"golden-receipt-input");
    let output_hash = blake3_hash(b"golden-receipt-output");
    let receipt = sign_work_receipt(
        &worker.key,
        WorkReceipt {
            abi_version: WORK_WIRE_ABI_VERSION,
            receipt_id: receipt_id_for_claim(
                request_hash,
                admission_hash,
                worker.identity.node_id,
                input_hash,
                output_hash,
                1,
            ),
            request_hash,
            admission_hash,
            worker: worker.identity.clone(),
            relay_node_id: relay.identity.node_id,
            input_hash,
            output_hash,
            units_used: 1,
            total_claim: 9,
            sequence: 1,
            signature: empty_signature(),
        },
    );
    let receipt_2 = sign_work_receipt(
        &worker.key,
        WorkReceipt {
            sequence: 2,
            receipt_id: receipt_id_for_claim(
                request_hash,
                admission_hash,
                worker.identity.node_id,
                input_hash,
                output_hash,
                2,
            ),
            ..receipt.clone()
        },
    );

    GoldenFixture {
        request,
        admission,
        receipt: receipt.clone(),
        route,
        route_snapshot,
        ordered,
        channel_proof,
        transit_input,
        manifest,
        shard,
        recipient_policy,
        batch_receipts: vec![receipt, receipt_2],
    }
}

fn golden_hashes() -> Vec<(&'static str, Hash)> {
    let fixture = fixture();
    let admission_hash = work_admission_hash(&fixture.admission).expect("admission hash");
    let receipt_hash = work_receipt_hash(&fixture.receipt).expect("receipt hash");
    let batch = build_receipt_batch(&fixture.admission, &fixture.batch_receipts).expect("batch");

    vec![
        ("node_id:user_message", derive_node_id(&public_key_from_seed(4), NODE_ROLE_MESSAGE)),
        ("work_request_preimage", blake3_hash(&work_request_preimage(&fixture.request))),
        ("work_admission_preimage", blake3_hash(&work_admission_preimage(&fixture.admission))),
        ("work_receipt_preimage", blake3_hash(&work_receipt_preimage(&fixture.receipt))),
        ("route_advertisement_preimage", blake3_hash(&route_advertisement_preimage(&fixture.route))),
        ("route_root_hash", route_root_hash(core::slice::from_ref(&fixture.route))),
        ("route_snapshot_preimage", blake3_hash(&route_snapshot_preimage(&fixture.route_snapshot))),
        ("ordered_message_hash", ordered_message_hash(&fixture.ordered)),
        ("channel_proof_preimage", blake3_hash(&channel_proof_preimage(&fixture.channel_proof))),
        ("channel_proof_hash", channel_proof_hash(&fixture.channel_proof)),
        ("packet_transit_hash", packet_transit_hash(&fixture.transit_input)),
        (
            "relay_delivery_output_hash",
            relay_delivery_output_hash(
                packet_transit_hash(&fixture.transit_input),
                fixture.ordered.envelope.packet_hash,
                channel_proof_hash(&fixture.channel_proof),
            ),
        ),
        ("receipt_id_for_claim", fixture.receipt.receipt_id),
        ("work_admission_hash", admission_hash),
        ("work_receipt_hash", receipt_hash),
        ("receipt_batch_root", batch.batch_root),
        ("erasure_job_id", fixture.manifest.job_id),
        ("erasure_shard_hash", fixture.shard.hash),
        ("manifest_hash", manifest_hash(&fixture.manifest)),
        ("recipient_message_policy_hash", recipient_message_policy_hash(&fixture.recipient_policy)),
    ]
}

#[test]
fn golden_hash_fixtures_are_valid_and_stable_within_run() {
    let first = golden_hashes();
    let second = golden_hashes();
    assert_eq!(first, second);
    assert_eq!(first.len(), 20);
    for (name, hash) in first {
        assert_ne!(hash, [0u8; 32], "{name} must not be zero");
    }
}

#[ignore = "prints current golden hash constants for lock-in"]
#[test]
fn print_current_golden_hashes() {
    for (name, hash) in golden_hashes() {
        println!("{name}={}", hex(hash));
    }
}
