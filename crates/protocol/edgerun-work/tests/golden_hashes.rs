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

fn hash_from_hex(input: &str) -> Hash {
    assert_eq!(input.len(), 64, "golden hash must be 32 bytes");
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&input[i * 2..i * 2 + 2], 16).expect("valid golden hash hex");
    }
    out
}

const EXPECTED_GOLDEN_HASHES: &[(&str, &str)] = &[
    (
        "node_id:user_message",
        "ca93ac1705187071d67b83c7ff0efe8108e8ec4530575d7726879333dbdabe7c",
    ),
    (
        "work_request_preimage",
        "028c173e9077896de78bb21d04c04d8f5e7595f0ed328c5587211a41f41b0894",
    ),
    (
        "work_admission_preimage",
        "f4179e670b6338adcc9d892d6609c8cf0489d3f9f494d748021ca718cb5e5927",
    ),
    (
        "work_receipt_preimage",
        "516363d14cb2b9e25a993e83013d50974b58708ea0b47f0bce23633c470cd6ca",
    ),
    (
        "route_binding_preimage",
        "8da7a2d16a2eb31c21daf7f2e3c88d6c4a524bbc2a957cf016c47a6c69f59ef4",
    ),
    (
        "route_root_hash",
        "5714767d2c57d802f3f9f6460f822769cd1de2726f405877d59efe3e0b68e0dc",
    ),
    (
        "ordered_message_hash",
        "1a908d9dd29b5a944d81ba2e1322b8a5a3812fc912f0a7c0d8f6f8dd0d30c817",
    ),
    (
        "channel_proof_preimage",
        "886383b38d79d0afffb6685d630e701209fa12838aa6d897c0c806d8d7685912",
    ),
    (
        "channel_proof_hash",
        "55c6f385b644538fca863caf78d86fd89ad3868f0432b2f93084e10daf826a71",
    ),
    (
        "packet_transit_hash",
        "4193c7ad098049fa7ae3aa2faf19706cf02b4522c09491210f36173a3e0b4e94",
    ),
    (
        "relay_delivery_output_hash",
        "6e548268df87d305590bb98e893b6326de69eda937e249decf7099d6ba617be7",
    ),
    (
        "receipt_id_for_claim",
        "250f9c8bd38ee24a1238a328aaa54e673180e16265b7d49e7065f400e49ba320",
    ),
    (
        "work_admission_hash",
        "422028742c410603d53e13296732b471491a0ccd236b56d1419c1406dbe4a09a",
    ),
    (
        "work_receipt_hash",
        "daa829860824b389b8277b2d325e8d1c1090e7a7b740b807c16978f93607b110",
    ),
    (
        "receipt_batch_root",
        "46af630d2fddfbb53aed24f86066e2d7462f22265c9de2e453a1b734283408c9",
    ),
    (
        "erasure_job_id",
        "ef239665208e5628d83c3d6fa04b4412e138020547f95569c9eeb06806c8b5e0",
    ),
    (
        "erasure_shard_hash",
        "57a6f601b9bce81a0d8b61752932fb77ec03f2e659dec2a53560d52ae35cc8e3",
    ),
    (
        "manifest_hash",
        "43e77f2db96e502873df9ae211643ef165e7e6a069464cb1835ccf9fb0bb1f50",
    ),
    (
        "recipient_message_policy_hash",
        "6da3168e82dd880a2a91ff571d95fbed429ff611caf7a89c00005ce20efa220a",
    ),
];

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
    route: RouteBinding,
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
    let request_hash =
        packet_hash(&WorkPacket::WorkRequest(request.clone())).expect("request hash");

    let route = relay.bind_memory_route(
        relay.identity.node_id,
        vec![DEPARTMENT_RELAY, DEPARTMENT_MESSAGE],
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
            assigned_route_commitment: route_commitment(&route),
            assigned_channel: route.endpoint.clone(),
            assigned_relay_path: vec![relay.identity.node_id],
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

    let assigned_nodes = [
        worker.identity.node_id,
        relay.identity.node_id,
        recipient.identity.node_id,
    ];
    let (manifest, shards) =
        encode_xor_2_1(b"golden erasure payload", assigned_nodes).expect("erasure");
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
        (
            "node_id:user_message",
            derive_node_id(&public_key_from_seed(4), NODE_ROLE_MESSAGE),
        ),
        (
            "work_request_preimage",
            blake3_hash(&work_request_preimage(&fixture.request)),
        ),
        (
            "work_admission_preimage",
            blake3_hash(&work_admission_preimage(&fixture.admission)),
        ),
        (
            "work_receipt_preimage",
            blake3_hash(&work_receipt_preimage(&fixture.receipt)),
        ),
        (
            "route_binding_preimage",
            blake3_hash(&route_binding_preimage(&fixture.route)),
        ),
        (
            "route_root_hash",
            route_root_hash(core::slice::from_ref(&fixture.route)),
        ),
        (
            "ordered_message_hash",
            ordered_message_hash(&fixture.ordered),
        ),
        (
            "channel_proof_preimage",
            blake3_hash(&channel_proof_preimage(&fixture.channel_proof)),
        ),
        (
            "channel_proof_hash",
            channel_proof_hash(&fixture.channel_proof),
        ),
        (
            "packet_transit_hash",
            packet_transit_hash(&fixture.transit_input),
        ),
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
        (
            "recipient_message_policy_hash",
            recipient_message_policy_hash(&fixture.recipient_policy),
        ),
    ]
}

#[test]
fn golden_hash_fixtures_match_locked_constants() {
    let actual = golden_hashes();
    assert_eq!(actual.len(), EXPECTED_GOLDEN_HASHES.len());
    for ((actual_name, actual_hash), (expected_name, expected_hash)) in
        actual.iter().zip(EXPECTED_GOLDEN_HASHES)
    {
        assert_eq!(actual_name, expected_name, "golden hash order changed");
        assert_eq!(
            *actual_hash,
            hash_from_hex(expected_hash),
            "{actual_name} changed"
        );
    }
}

#[test]
fn golden_hash_fixtures_are_valid_and_stable_within_run() {
    let first = golden_hashes();
    assert_eq!(first, golden_hashes());
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
