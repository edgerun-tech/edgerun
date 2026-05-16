use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn public_key_from_seed(seed: u8) -> PublicKey {
    let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    let mut out = [0u8; 32];
    out.copy_from_slice(key.verifying_key().as_bytes());
    out
}

fn signed_admission(
    admission: &SimNode,
    user: PublicKey,
    request_hash: Hash,
    assigned_route_commitment: Hash,
    assigned_channel: ChannelEndpoint,
    admitted_budget: u64,
    sequence: u64,
) -> WorkAdmission {
    sign_work_admission(
        &admission.key,
        WorkAdmission {
            abi_version: WORK_WIRE_ABI_VERSION,
            admission_id: blake3_hash(&request_hash),
            dao_id: admission.identity.public_key,
            user,
            admission_node: admission.identity.clone(),
            request_hash,
            assigned_route_commitment,
            assigned_channel,
            assigned_relay_path: vec![[0u8; 32]],
            admitted_budget,
            policy_hash: [0u8; 32],
            sequence,
            valid_until_unix_ms: u64::MAX,
            signature: empty_signature(),
        },
    )
}

fn signed_receipt(
    worker: &SimNode,
    request_hash: Hash,
    admission_hash: Hash,
    input_hash: Hash,
    output_hash: Hash,
    claim: u64,
    sequence: u64,
) -> WorkReceipt {
    sign_work_receipt(
        &worker.key,
        WorkReceipt {
            abi_version: WORK_WIRE_ABI_VERSION,
            receipt_id: receipt_id_for_claim(
                request_hash,
                admission_hash,
                worker.identity.node_id,
                input_hash,
                output_hash,
                sequence,
            ),
            request_hash,
            admission_hash,
            worker: worker.identity.clone(),
            relay_node_id: [0u8; 32],
            input_hash,
            output_hash,
            units_used: 1,
            total_claim: claim,
            sequence,
            signature: empty_signature(),
        },
    )
}

#[test]
fn scale_many_routes_messages_files_and_payments() {
    const RELAY_COUNT: usize = 3;
    const MESSAGE_COUNT: usize = 24;
    const FILE_COUNT: usize = 8;

    let admission = SimNode::from_seed(171, NODE_ROLE_ADMISSION);
    let user = public_key_from_seed(172);
    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, 10_000);

    let mut channel = MemoryChannelEngine::new();
    let relays = [
        SimNode::from_seed(173, NODE_ROLE_RELAY),
        SimNode::from_seed(174, NODE_ROLE_RELAY),
        SimNode::from_seed(175, NODE_ROLE_RELAY),
    ];
    let mut relay_roles = [
        RelayRole::from_seed(173, 2),
        RelayRole::from_seed(174, 2),
        RelayRole::from_seed(175, 2),
    ];
    let mut route_bindings = Vec::new();
    for relay in &relays {
        let route = relay.bind_memory_route(relay.identity.node_id, vec![DEPARTMENT_RELAY]);
        channel.add_route(route.clone()).expect("relay route");
        route_bindings.push(route);
    }

    let mut senders = (0..MESSAGE_COUNT)
        .map(|i| SimNode::from_seed(176 + (i % 40) as u8, NODE_ROLE_MESSAGE))
        .collect::<Vec<_>>();
    let mut receivers = (0..MESSAGE_COUNT)
        .map(|i| SimNode::from_seed(216 + (i % 40) as u8, NODE_ROLE_MESSAGE))
        .collect::<Vec<_>>();

    for (i, receiver) in receivers.iter().enumerate() {
        let relay = &relays[i % RELAY_COUNT];
        let route = receiver.bind_memory_route(relay.identity.node_id, vec![DEPARTMENT_MESSAGE]);
        channel.add_route(route.clone()).expect("receiver route");
        route_bindings.push(route);
    }

    let storage_nodes = [
        SimNode::from_seed(10, NODE_ROLE_STORAGE),
        SimNode::from_seed(11, NODE_ROLE_STORAGE),
        SimNode::from_seed(12, NODE_ROLE_STORAGE),
        SimNode::from_seed(13, NODE_ROLE_STORAGE),
        SimNode::from_seed(14, NODE_ROLE_STORAGE),
        SimNode::from_seed(15, NODE_ROLE_STORAGE),
    ];
    for (i, storage) in storage_nodes.iter().enumerate() {
        let relay = &relays[i % RELAY_COUNT];
        let route = storage.bind_memory_route(relay.identity.node_id, vec![DEPARTMENT_STORAGE]);
        channel.add_route(route.clone()).expect("storage route");
        route_bindings.push(route);
    }

    assert_eq!(
        route_bindings
            .iter()
            .filter(|route| route.departments.contains(&DEPARTMENT_RELAY))
            .count(),
        RELAY_COUNT
    );
    assert_eq!(
        route_bindings
            .iter()
            .filter(|route| route.departments.contains(&DEPARTMENT_STORAGE))
            .count(),
        storage_nodes.len()
    );

    let mut relay_paid_total = 0u64;
    for i in 0..MESSAGE_COUNT {
        let relay_index = i % RELAY_COUNT;
        let relay = &relays[relay_index];
        let receiver_route_hash = expected_route_hash(&channel, &receivers[i].identity.node_id)
            .expect("receiver route hash");
        let relay_route = channel.route_for(&relay.identity.node_id).unwrap().clone();
        let request_hash = blake3_hash(format!("message-request-{i}").as_bytes());
        let admission_doc = signed_admission(
            &admission,
            user,
            request_hash,
            route_hash(&relay_route),
            relay_route.endpoint.clone(),
            2,
            i as u64 + 1,
        );
        let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");
        ledger
            .reserve_admission_budget(&admission_doc)
            .expect("reserve message admission budget");
        let sender = &mut senders[i];
        let packet = sender.message_to(
            receivers[i].identity.node_id,
            relay.identity.node_id,
            DEPARTMENT_MESSAGE,
            WORK_TYPE_MESSAGE_DELIVER,
            format!("message payload {i}").into_bytes(),
        );
        let to_relay = deliver_ordered(
            &mut channel,
            &mut sender.order,
            sender.identity.node_id,
            relay.identity.node_id,
            packet,
        )
        .expect("sender to relay");
        let relay_result = relay_roles[relay_index]
            .forward_ordered(&mut channel, &to_relay, request_hash, admission_hash)
            .expect("relay forwards message");
        assert_eq!(relay_result.destination_route_hash, receiver_route_hash);
        ledger
            .settle_receipt(&admission_doc, &relay_result.receipt)
            .expect("relay payment settles");
        relay_paid_total += relay_result.receipt.total_claim;

        let mut inbox = channel.drain_inbox(receivers[i].identity.node_id);
        assert_eq!(inbox.len(), 1);
        let ordered = OrderedChannelEnvelope {
            envelope: inbox.remove(0),
            sequence: 1,
            previous_message_hash: [0u8; 32],
        };
        receivers[i]
            .accept_ordered(&ordered, receiver_route_hash)
            .expect("receiver accepts forwarded message");
    }

    assert_eq!(relay_paid_total, (MESSAGE_COUNT as u64) * 2);
    let relay_balances = relay_roles
        .iter()
        .map(|relay| ledger.worker_balance(&relay.identity.node_id))
        .collect::<Vec<_>>();
    assert_eq!(relay_balances.iter().sum::<u64>(), relay_paid_total);

    let mut shard_store = MemoryShardStore::new();
    let storage_routes = route_bindings
        .iter()
        .filter(|route| route.departments.contains(&DEPARTMENT_STORAGE))
        .collect::<Vec<_>>();
    let mut storage_paid_total = 0u64;
    for file_index in 0..FILE_COUNT {
        let assigned_nodes = [
            storage_routes[(file_index * 3) % storage_routes.len()]
                .node
                .node_id,
            storage_routes[(file_index * 3 + 1) % storage_routes.len()]
                .node
                .node_id,
            storage_routes[(file_index * 3 + 2) % storage_routes.len()]
                .node
                .node_id,
        ];
        let file =
            format!("scaled erasure file {file_index} with deterministic payload").into_bytes();
        let (manifest, shards) = encode_xor_2_1(&file, assigned_nodes).expect("encode file");
        verify_manifest(&manifest, &shards).expect("manifest verifies");
        let request_hash = manifest.job_id;
        let storage_route = route_bindings
            .iter()
            .find(|route| route.node.node_id == assigned_nodes[0])
            .unwrap();
        let admission_doc = signed_admission(
            &admission,
            user,
            request_hash,
            route_commitment(storage_route),
            storage_route.endpoint.clone(),
            15,
            10_000 + file_index as u64,
        );
        let admission_hash = work_admission_hash(&admission_doc).expect("storage admission hash");
        ledger
            .reserve_admission_budget(&admission_doc)
            .expect("reserve storage admission budget");
        for (shard_index, shard) in shards.iter().enumerate() {
            shard_store.put(shard.clone());
            let worker = storage_nodes
                .iter()
                .find(|node| node.identity.node_id == shard.assigned_node)
                .expect("assigned storage node");
            let receipt = signed_receipt(
                worker,
                request_hash,
                admission_hash,
                shard.hash,
                shard.hash,
                5,
                shard_index as u64 + 1,
            );
            ledger
                .settle_receipt(&admission_doc, &receipt)
                .expect("storage receipt settles");
            storage_paid_total += 5;
        }
        let retrieved = retrieve_shards(&shard_store, &manifest, &[0, 1, 2]);
        assert_eq!(
            reconstruct_xor_2_1(&manifest, &retrieved).expect("reconstruct"),
            file
        );
    }

    assert_eq!(storage_paid_total, (FILE_COUNT as u64) * 15);
    assert_eq!(shard_store.len(), FILE_COUNT * 3);
    let expected_spend = relay_paid_total + storage_paid_total;
    assert_eq!(ledger.user_balance(&user), 10_000 - expected_spend);
}
