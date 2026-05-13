use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn public_key_from_seed(seed: u8) -> PublicKey {
    let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    let mut out = [0u8; 32];
    out.copy_from_slice(key.verifying_key().as_bytes());
    out
}

fn signed_relay_admission(
    admission: &SimNode,
    user: PublicKey,
    request_hash: Hash,
    assigned_route_commitment: Hash,
    assigned_channel: ChannelEndpoint,
    admitted_budget: u64,
    policy_hash: Hash,
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
            policy_hash,
            sequence: 1,
            valid_until_unix_ms: u64::MAX,
            signature: empty_signature(),
        },
    )
}

struct DeliveryFixture {
    admission: WorkAdmission,
    user: PublicKey,
    relay_input: OrderedChannelEnvelope,
    recipient_delivery: OrderedChannelEnvelope,
    recipient: SimNode,
    recipient_policy: RecipientMessagePolicy,
    recipient_proof: ChannelProof,
    result: RelayDeliveryResult,
    payable_receipt: WorkReceipt,
}

fn fixture() -> DeliveryFixture {
    let admission_node = SimNode::from_seed(181, NODE_ROLE_ADMISSION);
    let user = public_key_from_seed(182);
    let mut sender = SimNode::from_seed(183, NODE_ROLE_MESSAGE);
    let recipient = SimNode::from_seed(184, NODE_ROLE_MESSAGE);
    let relay_node = SimNode::from_seed(185, NODE_ROLE_RELAY);
    let mut relay = RelayRole::from_seed(185, 3);
    let mut channel = MemoryChannelEngine::new();

    let relay_route =
        relay_node.bind_memory_route(relay_node.identity.node_id, vec![DEPARTMENT_RELAY]);
    let recipient_route =
        recipient.bind_memory_route(relay_node.identity.node_id, vec![DEPARTMENT_MESSAGE]);
    let relay_route_hash = channel.add_route(relay_route.clone()).expect("relay route");
    let recipient_route_hash = channel.add_route(recipient_route).expect("recipient route");
    let mut recipient_policy = open_recipient_message_policy(recipient.identity.clone(), 1, 1_000);
    recipient_policy.allowed_relays.push(relay.identity.node_id);
    recipient_policy = sign_recipient_message_policy(&recipient.key, recipient_policy);

    let request_hash = blake3_hash(b"delivery-settlement-request");
    let admission = signed_relay_admission(
        &admission_node,
        user,
        request_hash,
        relay_route_hash,
        relay_route.endpoint.clone(),
        10,
        recipient_message_policy_hash(&recipient_policy),
    );
    let admission_hash = work_admission_hash(&admission).expect("admission hash");

    let packet = sender.message_to(
        recipient.identity.node_id,
        relay_node.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        b"pay relay only when recipient proves delivery".to_vec(),
    );
    let relay_input = deliver_ordered(
        &mut channel,
        &mut sender.order,
        sender.identity.node_id,
        relay_node.identity.node_id,
        packet,
    )
    .expect("sender to relay delivery");

    let result = relay
        .forward_ordered(&mut channel, &relay_input, request_hash, admission_hash)
        .expect("relay forwards");

    let mut inbox = channel.drain_inbox(recipient.identity.node_id);
    assert_eq!(inbox.len(), 1);
    let recipient_delivery = OrderedChannelEnvelope {
        envelope: inbox.remove(0),
        sequence: 1,
        previous_message_hash: [0u8; 32],
    };
    let mut recipient_order = ChannelOrderBook::new();
    recipient_order
        .accept(&recipient_delivery, recipient_route_hash)
        .expect("recipient accepts delivery");
    let recipient_proof = channel_proof_for_allowed_ordered_message(
        &recipient.key,
        &recipient.identity,
        relay.identity.node_id,
        &recipient_delivery,
        &recipient_policy,
        999,
    )
    .expect("policy-gated recipient proof");
    let payable_receipt =
        relay.finalized_delivery_receipt(&result, channel_proof_hash(&recipient_proof));

    DeliveryFixture {
        admission,
        user,
        relay_input,
        recipient_delivery,
        recipient,
        recipient_policy,
        recipient_proof,
        result,
        payable_receipt,
    }
}

fn evidence<'a>(fixture: &'a DeliveryFixture) -> DeliverySettlementEvidence<'a> {
    DeliverySettlementEvidence {
        admission: &fixture.admission,
        receipt: &fixture.payable_receipt,
        relay_input: &fixture.relay_input,
        recipient_delivery: &fixture.recipient_delivery,
        recipient: &fixture.recipient.identity,
        recipient_policy: &fixture.recipient_policy,
        recipient_proof: &fixture.recipient_proof,
        previous_transit_hash: [0u8; 32],
        now_unix_ms: 999,
    }
}

#[test]
fn settle_delivery_requires_policy_bound_recipient_proof_and_transit_hash() {
    let fixture = fixture();
    let evidence = evidence(&fixture);

    assert_eq!(
        verify_delivery_evidence(&evidence).expect("delivery evidence verifies"),
        fixture.result.transit_hash,
    );
    assert_eq!(
        fixture.payable_receipt.output_hash,
        relay_delivery_output_hash(
            fixture.result.transit_hash,
            fixture.result.forwarded_packet_hash,
            channel_proof_hash(&fixture.recipient_proof),
        ),
    );

    let mut ledger = SettlementLedger::new();
    ledger.deposit_user_credit(fixture.user, 10);
    let settled = ledger
        .settle_delivery(&evidence)
        .expect("proof-carrying delivery settles");

    assert_eq!(settled.amount, 3);
    assert_eq!(ledger.user_balance(&fixture.user), 7);
    assert_eq!(
        ledger.worker_balance(&fixture.payable_receipt.worker.node_id),
        3
    );
}

#[test]
fn receiver_helper_refuses_to_sign_disallowed_message() {
    let mut fixture = fixture();
    fixture
        .recipient_policy
        .blocked_senders
        .push(fixture.relay_input.envelope.from);
    let recipient_key = Ed25519SigningKey::from_bytes(&[184u8; 32]);
    fixture.recipient_policy =
        sign_recipient_message_policy(&recipient_key, fixture.recipient_policy.clone());

    assert!(matches!(
        channel_proof_for_allowed_ordered_message(
            &fixture.recipient.key,
            &fixture.recipient.identity,
            fixture.payable_receipt.worker.node_id,
            &fixture.recipient_delivery,
            &fixture.recipient_policy,
            999,
        ),
        Err(PolicyBoundChannelProofError::PolicyRejected(
            RecipientPolicyError::SenderBlocked
        ))
    ));
}

#[test]
fn settle_delivery_rejects_legacy_policy_unbound_recipient_proof() {
    let mut fixture = fixture();
    fixture.recipient_proof = channel_proof_for_ordered(
        &fixture.recipient.key,
        &fixture.recipient.identity,
        fixture.payable_receipt.worker.node_id,
        &fixture.recipient_delivery,
    )
    .expect("legacy recipient proof");
    fixture.payable_receipt = sign_work_receipt(
        &Ed25519SigningKey::from_bytes(&[185u8; 32]),
        WorkReceipt {
            output_hash: relay_delivery_output_hash(
                fixture.result.transit_hash,
                fixture.result.forwarded_packet_hash,
                channel_proof_hash(&fixture.recipient_proof),
            ),
            ..fixture.payable_receipt.clone()
        },
    );

    assert_eq!(
        verify_delivery_evidence(&evidence(&fixture)),
        Err(SettlementError::InvalidRecipientProof)
    );
}

#[test]
fn settle_delivery_rejects_policy_hash_mismatch() {
    let mut fixture = fixture();
    fixture.admission.policy_hash = [4u8; 32];
    let admission_key = Ed25519SigningKey::from_bytes(&[181u8; 32]);
    fixture.admission = sign_work_admission(&admission_key, fixture.admission.clone());
    assert_eq!(
        verify_delivery_evidence(&evidence(&fixture)),
        Err(SettlementError::PolicyHashMismatch)
    );
}

#[test]
fn settle_delivery_rejects_message_disallowed_by_recipient_policy() {
    let mut fixture = fixture();
    fixture
        .recipient_policy
        .blocked_senders
        .push(fixture.relay_input.envelope.from);
    let recipient_key = Ed25519SigningKey::from_bytes(&[184u8; 32]);
    fixture.recipient_policy =
        sign_recipient_message_policy(&recipient_key, fixture.recipient_policy.clone());
    fixture.admission.policy_hash = recipient_message_policy_hash(&fixture.recipient_policy);
    let admission_key = Ed25519SigningKey::from_bytes(&[181u8; 32]);
    fixture.admission = sign_work_admission(&admission_key, fixture.admission.clone());

    assert_eq!(
        verify_delivery_evidence(&evidence(&fixture)),
        Err(SettlementError::MessagePolicyRejected)
    );
}

#[test]
fn settle_delivery_rejects_admission_route_mismatch() {
    let mut fixture = fixture();
    fixture.admission.assigned_route_commitment = [5u8; 32];
    let admission_key = Ed25519SigningKey::from_bytes(&[181u8; 32]);
    fixture.admission = sign_work_admission(&admission_key, fixture.admission.clone());

    assert_eq!(
        verify_delivery_evidence(&evidence(&fixture)),
        Err(SettlementError::AdmissionRouteMismatch)
    );
}

#[test]
fn settle_delivery_rejects_wrong_previous_transit_hash() {
    let fixture = fixture();
    let mut evidence = evidence(&fixture);
    evidence.previous_transit_hash = [9u8; 32];

    assert_eq!(
        verify_delivery_evidence(&evidence),
        Err(SettlementError::ReceiptOutputMismatch)
    );
}

#[test]
fn settle_delivery_rejects_tampered_recipient_proof() {
    let mut fixture = fixture();
    fixture.recipient_proof.sequence = fixture.recipient_proof.sequence.saturating_add(1);

    assert_eq!(
        verify_delivery_evidence(&evidence(&fixture)),
        Err(SettlementError::InvalidRecipientProof)
    );
}

#[test]
fn settle_delivery_rejects_wrong_recipient_delivery_packet_hash() {
    let mut fixture = fixture();
    fixture.recipient_delivery.envelope.packet_hash = [8u8; 32];

    assert_eq!(
        verify_delivery_evidence(&evidence(&fixture)),
        Err(SettlementError::DeliveryPacketMismatch)
    );
}

#[test]
fn settle_delivery_rejects_receipt_with_wrong_input_hash() {
    let mut fixture = fixture();
    let relay_key = Ed25519SigningKey::from_bytes(&[185u8; 32]);
    fixture.payable_receipt.input_hash = [6u8; 32];
    fixture.payable_receipt = sign_work_receipt(&relay_key, fixture.payable_receipt.clone());

    assert_eq!(
        verify_delivery_evidence(&evidence(&fixture)),
        Err(SettlementError::ReceiptInputMismatch)
    );
}

#[test]
fn settle_delivery_rejects_receipt_with_wrong_output_hash() {
    let mut fixture = fixture();
    let relay_key = Ed25519SigningKey::from_bytes(&[185u8; 32]);
    fixture.payable_receipt.output_hash = [7u8; 32];
    fixture.payable_receipt = sign_work_receipt(&relay_key, fixture.payable_receipt.clone());

    assert_eq!(
        verify_delivery_evidence(&evidence(&fixture)),
        Err(SettlementError::ReceiptOutputMismatch)
    );
}
