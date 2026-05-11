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
    route_hash: Hash,
    channel: ChannelEndpoint,
    budget: u64,
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
            assigned_route_hash: route_hash,
            assigned_channel: channel,
            admitted_budget: budget,
            policy_hash: [0u8; 32],
            sequence: 1,
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
fn typed_storage_store_retrieve_and_reconstruct_over_work_channel() {
    let admission = SimNode::from_seed(41, NODE_ROLE_ADMISSION);
    let user = public_key_from_seed(42);
    let mut client = SimNode::from_seed(43, NODE_ROLE_MESSAGE);
    let mut storage_nodes = [
        SimNode::from_seed(44, NODE_ROLE_STORAGE),
        SimNode::from_seed(45, NODE_ROLE_STORAGE),
        SimNode::from_seed(46, NODE_ROLE_STORAGE),
    ];
    let mut roles = [
        TypedObjectStoreRole::default(),
        TypedObjectStoreRole::default(),
        TypedObjectStoreRole::default(),
    ];

    let mut channel = MemoryWorkChannel::new();
    for storage in &storage_nodes {
        let route = storage.advertise_memory_route(storage.identity.node_id, vec![DEPARTMENT_STORAGE, DEPARTMENT_RETRIEVAL]);
        channel.add_route(route).expect("storage route");
    }

    let file = b"typed storage retrieval lifecycle over work channel".to_vec();
    let assigned_nodes = [
        storage_nodes[0].identity.node_id,
        storage_nodes[1].identity.node_id,
        storage_nodes[2].identity.node_id,
    ];
    let (manifest, shards) = encode_xor_2_1(&file, assigned_nodes).expect("encode");
    let manifest_hash = manifest_hash(&manifest);
    verify_manifest(&manifest, &shards).expect("manifest");

    let first_route = channel.engine().route_for(&storage_nodes[0].identity.node_id).unwrap().clone();
    let admission_doc = signed_admission(
        &admission,
        user,
        manifest.job_id,
        route_hash(&first_route),
        first_route.endpoint,
        60,
    );
    let admission_hash = work_admission_hash(&admission_doc).expect("admission hash");
    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(user, 100);

    for (index, shard) in shards.iter().enumerate() {
        let store_payload = StoragePayload::StoreRequest(store_request_from_shard(&manifest, shard));
        let packet = client.message_to(
            storage_nodes[index].identity.node_id,
            storage_nodes[index].identity.node_id,
            DEPARTMENT_STORAGE,
            WORK_TYPE_OBJECT_STORE,
            storage_payload_bytes(&store_payload).expect("store payload bytes"),
        );
        let ordered = channel
            .send_ordered(
                &mut client.order,
                client.identity.node_id,
                storage_nodes[index].identity.node_id,
                packet,
            )
            .expect("ordered store request");
        let route_hash = channel.route_hash_for(&storage_nodes[index].identity.node_id).unwrap();
        storage_nodes[index]
            .accept_ordered(&ordered, route_hash)
            .expect("storage accepts store order");
        let output = roles[index].handle(
            &RoleContext {
                now_unix_ms: 1,
                local_node: storage_nodes[index].identity.clone(),
                policy_hash: [0u8; 32],
            },
            RoleInput {
                packet: ordered.envelope.packet,
                previous_hash: ordered.previous_message_hash,
                channel_hash: ordered.envelope.channel_id,
            },
        );
        assert_eq!(output.status, ROLE_STATUS_ACCEPTED);
        assert!(roles[index].has_shard(&shard.hash));

        let receipt = signed_receipt(
            &storage_nodes[index],
            manifest.job_id,
            admission_hash,
            shard.hash,
            shard.hash,
            10,
            index as u64 + 1,
        );
        ledger.settle_receipt(&admission_doc, &receipt).expect("store receipt settles");
    }

    let mut retrieved_shards = Vec::new();
    for index in 0..3u16 {
        let req = retrieve_request_from_manifest(&manifest, index).expect("retrieve request");
        let packet = client.message_to(
            storage_nodes[index as usize].identity.node_id,
            storage_nodes[index as usize].identity.node_id,
            DEPARTMENT_RETRIEVAL,
            WORK_TYPE_OBJECT_RETRIEVE,
            storage_payload_bytes(&StoragePayload::RetrieveRequest(req)).expect("retrieve payload bytes"),
        );
        let ordered = channel
            .send_ordered(
                &mut client.order,
                client.identity.node_id,
                storage_nodes[index as usize].identity.node_id,
                packet,
            )
            .expect("ordered retrieve request");
        let route_hash = channel.route_hash_for(&storage_nodes[index as usize].identity.node_id).unwrap();
        storage_nodes[index as usize]
            .accept_ordered(&ordered, route_hash)
            .expect("storage accepts retrieve order");
        let output = roles[index as usize].handle(
            &RoleContext {
                now_unix_ms: 2,
                local_node: storage_nodes[index as usize].identity.clone(),
                policy_hash: [0u8; 32],
            },
            RoleInput {
                packet: ordered.envelope.packet,
                previous_hash: ordered.previous_message_hash,
                channel_hash: ordered.envelope.channel_id,
            },
        );
        assert_eq!(output.status, ROLE_STATUS_ACCEPTED);
        let response = match storage_payload_from_bytes(&output.bytes).expect("response payload") {
            StoragePayload::RetrieveResponse(response) => response,
            _ => panic!("expected retrieve response"),
        };
        assert_eq!(response.manifest_hash, manifest_hash);
        assert!(verify_retrieve_response(&response));
        retrieved_shards.push(ErasureShard {
            job_id: response.job_id,
            index: response.shard_index,
            is_parity: response.shard_index == 2,
            original_len: manifest.original_len,
            bytes: response.bytes,
            hash: response.shard_hash,
            assigned_node: manifest.assigned_nodes[index as usize],
        });

        let receipt = signed_receipt(
            &storage_nodes[index as usize],
            manifest.job_id,
            admission_hash,
            manifest.shard_hashes[index as usize],
            manifest.original_hash,
            10,
            100 + index as u64,
        );
        ledger.settle_receipt(&admission_doc, &receipt).expect("retrieve receipt settles");
    }

    let reconstructed = reconstruct_xor_2_1(&manifest, &retrieved_shards).expect("reconstruct typed retrieved shards");
    assert_eq!(reconstructed, file);
    assert_eq!(ledger.user_balance(&user), 40);
}
