use edgerun_work::*;

fn message_from(sender: &mut SimNode, recipient: &SimNode, relay: &SimNode, payload: &[u8]) -> NetworkMessage {
    let packet = sender.message_to(
        recipient.identity.node_id,
        relay.identity.node_id,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        payload.to_vec(),
    );
    let WorkPacket::NetworkMessage(message) = packet else {
        panic!("expected network message");
    };
    message
}

#[test]
fn recipient_policy_allows_open_message_policy() {
    let mut sender = SimNode::from_seed(210, NODE_ROLE_MESSAGE);
    let recipient = SimNode::from_seed(211, NODE_ROLE_MESSAGE);
    let relay = SimNode::from_seed(212, NODE_ROLE_RELAY);
    let policy = sign_recipient_message_policy(
        &recipient.key,
        open_recipient_message_policy(recipient.identity.clone(), 1, 1_000),
    );
    let message = message_from(&mut sender, &recipient, &relay, b"hello without meta");

    assert!(verify_recipient_message_policy(&policy));
    recipient_message_policy_allows(&policy, &message, 999).expect("message allowed");
}

#[test]
fn recipient_policy_rejects_sender_not_on_allowlist() {
    let allowed = SimNode::from_seed(213, NODE_ROLE_MESSAGE);
    let mut blocked = SimNode::from_seed(214, NODE_ROLE_MESSAGE);
    let recipient = SimNode::from_seed(215, NODE_ROLE_MESSAGE);
    let relay = SimNode::from_seed(216, NODE_ROLE_RELAY);
    let policy = sign_recipient_message_policy(
        &recipient.key,
        allowlist_recipient_message_policy(
            recipient.identity.clone(),
            vec![allowed.identity.node_id],
            1,
            1_000,
        ),
    );
    let message = message_from(&mut blocked, &recipient, &relay, b"not allowed");

    assert_eq!(
        recipient_message_policy_allows(&policy, &message, 999),
        Err(RecipientPolicyError::SenderNotAllowed)
    );
}

#[test]
fn recipient_policy_allows_sender_on_allowlist() {
    let mut allowed = SimNode::from_seed(217, NODE_ROLE_MESSAGE);
    let recipient = SimNode::from_seed(218, NODE_ROLE_MESSAGE);
    let relay = SimNode::from_seed(219, NODE_ROLE_RELAY);
    let policy = sign_recipient_message_policy(
        &recipient.key,
        allowlist_recipient_message_policy(
            recipient.identity.clone(),
            vec![allowed.identity.node_id],
            1,
            1_000,
        ),
    );
    let message = message_from(&mut allowed, &recipient, &relay, b"allowed");

    recipient_message_policy_allows(&policy, &message, 999).expect("allowlisted sender");
}

#[test]
fn recipient_policy_blocklist_overrides_open_policy() {
    let mut sender = SimNode::from_seed(220, NODE_ROLE_MESSAGE);
    let recipient = SimNode::from_seed(221, NODE_ROLE_MESSAGE);
    let relay = SimNode::from_seed(222, NODE_ROLE_RELAY);
    let mut policy = open_recipient_message_policy(recipient.identity.clone(), 1, 1_000);
    policy.blocked_senders.push(sender.identity.node_id);
    let policy = sign_recipient_message_policy(&recipient.key, policy);
    let message = message_from(&mut sender, &recipient, &relay, b"blocked");

    assert_eq!(
        recipient_message_policy_allows(&policy, &message, 999),
        Err(RecipientPolicyError::SenderBlocked)
    );
}

#[test]
fn recipient_policy_restricts_relay() {
    let mut sender = SimNode::from_seed(223, NODE_ROLE_MESSAGE);
    let recipient = SimNode::from_seed(224, NODE_ROLE_MESSAGE);
    let allowed_relay = SimNode::from_seed(225, NODE_ROLE_RELAY);
    let wrong_relay = SimNode::from_seed(226, NODE_ROLE_RELAY);
    let mut policy = open_recipient_message_policy(recipient.identity.clone(), 1, 1_000);
    policy.allowed_relays.push(allowed_relay.identity.node_id);
    let policy = sign_recipient_message_policy(&recipient.key, policy);
    let message = message_from(&mut sender, &recipient, &wrong_relay, b"wrong relay");

    assert_eq!(
        recipient_message_policy_allows(&policy, &message, 999),
        Err(RecipientPolicyError::RelayNotAllowed)
    );
}

#[test]
fn recipient_policy_rejects_expired_policy() {
    let mut sender = SimNode::from_seed(227, NODE_ROLE_MESSAGE);
    let recipient = SimNode::from_seed(228, NODE_ROLE_MESSAGE);
    let relay = SimNode::from_seed(229, NODE_ROLE_RELAY);
    let policy = sign_recipient_message_policy(
        &recipient.key,
        open_recipient_message_policy(recipient.identity.clone(), 1, 100),
    );
    let message = message_from(&mut sender, &recipient, &relay, b"expired");

    assert_eq!(
        recipient_message_policy_allows(&policy, &message, 101),
        Err(RecipientPolicyError::ExpiredPolicy)
    );
}

#[test]
fn recipient_policy_rejects_oversized_payload() {
    let mut sender = SimNode::from_seed(230, NODE_ROLE_MESSAGE);
    let recipient = SimNode::from_seed(231, NODE_ROLE_MESSAGE);
    let relay = SimNode::from_seed(232, NODE_ROLE_RELAY);
    let mut policy = open_recipient_message_policy(recipient.identity.clone(), 1, 1_000);
    policy.max_payload_bytes = 4;
    let policy = sign_recipient_message_policy(&recipient.key, policy);
    let message = message_from(&mut sender, &recipient, &relay, b"12345");

    assert_eq!(
        recipient_message_policy_allows(&policy, &message, 999),
        Err(RecipientPolicyError::PayloadTooLarge)
    );
}

#[test]
fn recipient_policy_signature_binds_recipient_node_id() {
    let recipient = SimNode::from_seed(233, NODE_ROLE_MESSAGE);
    let mut policy = sign_recipient_message_policy(
        &recipient.key,
        open_recipient_message_policy(recipient.identity.clone(), 1, 1_000),
    );
    policy.recipient.node_id = blake3_hash(b"fake recipient");

    assert!(!verify_recipient_message_policy(&policy));
}
