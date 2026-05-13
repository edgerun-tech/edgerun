use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn node(seed: u8, role: u16) -> (Ed25519SigningKey, NodeIdentity) {
    let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
    let identity = node_identity_from_key(&key, role);
    (key, identity)
}

#[test]
fn sealed_message_opens_for_recipient() {
    let (_sender_key, sender) = node(11, NODE_ROLE_MESSAGE);
    let (recipient_key, recipient) = node(12, NODE_ROLE_MESSAGE);
    let plaintext = b"encrypted to the recipient, not the relay";

    let sealed = seal_message_for_recipient_with_ephemeral(
        &sender, &recipient, plaintext, [77u8; 32], [88u8; 12],
    )
    .expect("seal");

    assert_ne!(sealed, plaintext);
    let opened = unseal_message_from_recipient_payload(&recipient_key, &recipient, &sealed)
        .expect("recipient opens");
    assert_eq!(opened, plaintext);
}

#[test]
fn sealed_message_rejects_wrong_recipient() {
    let (_sender_key, sender) = node(21, NODE_ROLE_MESSAGE);
    let (_recipient_key, recipient) = node(22, NODE_ROLE_MESSAGE);
    let (wrong_key, wrong_recipient) = node(23, NODE_ROLE_MESSAGE);
    let sealed = seal_message_for_recipient_with_ephemeral(
        &sender,
        &recipient,
        b"for recipient only",
        [77u8; 32],
        [88u8; 12],
    )
    .expect("seal");

    assert_eq!(
        unseal_message_from_recipient_payload(&wrong_key, &wrong_recipient, &sealed),
        Err(MessageSealError::WrongRecipient)
    );
}

#[test]
fn sealed_message_rejects_tampered_ciphertext() {
    let (_sender_key, sender) = node(31, NODE_ROLE_MESSAGE);
    let (recipient_key, recipient) = node(32, NODE_ROLE_MESSAGE);
    let sealed = seal_message_for_recipient_with_ephemeral(
        &sender,
        &recipient,
        b"detect tampering",
        [77u8; 32],
        [88u8; 12],
    )
    .expect("seal");
    let mut payload = sealed_message_payload_from_bytes(&sealed).expect("decode sealed payload");
    payload.ciphertext[0] ^= 0x01;

    assert_eq!(
        unseal_message_payload(&recipient_key, &recipient, &payload),
        Err(MessageSealError::UnsealFailed)
    );
}
