use alloc::vec::Vec;

use edgerun_crypto::x25519::{PublicKey as X25519PublicKey, StaticSecret as X25519Secret};
use edgerun_crypto::{Aes256GcmCipher, Ed25519SigningKey, Ed25519VerifyingKey, Nonce, Tag};
use rkyv::{Archive, Deserialize, Serialize};

use crate::chat_index::{
    MessageObject, chat_payload_hash, finalize_message_object, thread_id_for_participants,
};
use crate::codec::{blake3_hash, wire_bytes, wire_from_bytes};
use crate::identity::verify_node_identity;
use crate::preimage::PreimageBuilder;
use crate::protocol::{
    Hash, NetworkMessage, NodeId, NodeIdentity, PublicKey, WORK_WIRE_ABI_VERSION,
};

const MESSAGE_SEAL_DOMAIN: &[u8] = b"edgerun:v1:work:message-seal";
const MESSAGE_SEAL_KDF_INFO: &[u8] = b"edgerun:v1:work:message-seal:key";
const X25519_KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

pub type EncryptionPublicKey = [u8; X25519_KEY_LEN];
pub type EncryptionSecretKey = [u8; X25519_KEY_LEN];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageSealError {
    InvalidIdentity,
    InvalidEnvelope,
    InvalidKey,
    RandomFailed,
    SealFailed,
    UnsealFailed,
    WrongRecipient,
    PacketTooLarge,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct SealedMessagePayload {
    pub abi_version: u16,
    pub from: NodeId,
    pub to: NodeId,
    pub recipient_encryption_key: EncryptionPublicKey,
    pub ephemeral_public_key: EncryptionPublicKey,
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext: Vec<u8>,
    pub tag: [u8; TAG_LEN],
    pub plaintext_hash: Hash,
}

pub fn encryption_secret_from_ed25519_key(key: &Ed25519SigningKey) -> EncryptionSecretKey {
    key.to_scalar_bytes()
}

pub fn encryption_public_from_ed25519_key(
    key: &Ed25519SigningKey,
) -> Result<EncryptionPublicKey, MessageSealError> {
    encryption_public_from_ed25519_public(key.verifying_key().as_bytes())
}

pub fn encryption_public_from_ed25519_public(
    public_key: &PublicKey,
) -> Result<EncryptionPublicKey, MessageSealError> {
    let verifying_key =
        Ed25519VerifyingKey::from_bytes(public_key).map_err(|_| MessageSealError::InvalidKey)?;
    Ok(verifying_key.to_montgomery_bytes())
}

pub fn sealed_message_payload_bytes(
    payload: &SealedMessagePayload,
) -> Result<Vec<u8>, MessageSealError> {
    wire_bytes(payload).map_err(|_| MessageSealError::InvalidEnvelope)
}

pub fn sealed_message_payload_from_bytes(
    bytes: &[u8],
) -> Result<SealedMessagePayload, MessageSealError> {
    if bytes.is_empty() {
        return Err(MessageSealError::InvalidEnvelope);
    }
    wire_from_bytes::<SealedMessagePayload, ArchivedSealedMessagePayload>(bytes)
        .map_err(|_| MessageSealError::InvalidEnvelope)
}

pub fn sealed_message_object_from_network_message(
    message: &NetworkMessage,
    created_unix_ms: u64,
    message_kind: u16,
) -> Result<MessageObject, MessageSealError> {
    let sealed = sealed_message_payload_from_bytes(&message.payload)?;
    if sealed.from != message.from || sealed.to != message.to {
        return Err(MessageSealError::InvalidEnvelope);
    }
    Ok(finalize_message_object(MessageObject {
        abi_version: WORK_WIRE_ABI_VERSION,
        message_id: [0u8; 32],
        thread_id: thread_id_for_participants(message.from, message.to),
        from: message.from,
        to: message.to,
        sequence: message.sequence,
        created_unix_ms,
        message_kind,
        payload_hash: sealed.plaintext_hash,
        payload_len: sealed.ciphertext.len() as u64,
        sealed_payload_hash: chat_payload_hash(&message.payload),
        storage_ref: Vec::new(),
        previous_message_hash: message.prev_hash,
    }))
}

pub fn seal_message_for_recipient(
    sender: &NodeIdentity,
    recipient: &NodeIdentity,
    plaintext: &[u8],
) -> Result<Vec<u8>, MessageSealError> {
    let mut ephemeral_secret = [0u8; X25519_KEY_LEN];
    edgerun_crypto::fill_random(&mut ephemeral_secret)
        .map_err(|_| MessageSealError::RandomFailed)?;
    let mut nonce = [0u8; NONCE_LEN];
    edgerun_crypto::fill_random(&mut nonce).map_err(|_| MessageSealError::RandomFailed)?;
    seal_message_for_recipient_with_ephemeral(sender, recipient, plaintext, ephemeral_secret, nonce)
}

pub fn seal_message_for_recipient_with_ephemeral(
    sender: &NodeIdentity,
    recipient: &NodeIdentity,
    plaintext: &[u8],
    ephemeral_secret: EncryptionSecretKey,
    nonce: [u8; NONCE_LEN],
) -> Result<Vec<u8>, MessageSealError> {
    if !verify_node_identity(sender) || !verify_node_identity(recipient) {
        return Err(MessageSealError::InvalidIdentity);
    }
    let recipient_encryption_key = encryption_public_from_ed25519_public(&recipient.public_key)?;
    let ephemeral_secret = X25519Secret::from(ephemeral_secret);
    let ephemeral_public_key = X25519PublicKey::from(&ephemeral_secret).to_bytes();
    let shared = ephemeral_secret.diffie_hellman(&X25519PublicKey::from(recipient_encryption_key));
    let key = message_key(
        shared.as_bytes(),
        &ephemeral_public_key,
        &recipient_encryption_key,
    );
    let mut sealed = SealedMessagePayload {
        abi_version: WORK_WIRE_ABI_VERSION,
        from: sender.node_id,
        to: recipient.node_id,
        recipient_encryption_key,
        ephemeral_public_key,
        nonce,
        ciphertext: plaintext.to_vec(),
        tag: [0u8; TAG_LEN],
        plaintext_hash: blake3_hash(plaintext),
    };
    let cipher = Aes256GcmCipher::new(&key).map_err(|_| MessageSealError::InvalidKey)?;
    let tag = cipher
        .encrypt_in_place_detached(
            &Nonce::from(nonce),
            &sealed_message_aad(&sealed),
            &mut sealed.ciphertext,
        )
        .map_err(|_| MessageSealError::SealFailed)?;
    sealed.tag.copy_from_slice(tag.as_slice());
    sealed_message_payload_bytes(&sealed)
}

pub fn unseal_message_from_recipient_payload(
    recipient_key: &Ed25519SigningKey,
    recipient: &NodeIdentity,
    bytes: &[u8],
) -> Result<Vec<u8>, MessageSealError> {
    let sealed = sealed_message_payload_from_bytes(bytes)?;
    unseal_message_payload(recipient_key, recipient, &sealed)
}

pub fn unseal_message_payload(
    recipient_key: &Ed25519SigningKey,
    recipient: &NodeIdentity,
    sealed: &SealedMessagePayload,
) -> Result<Vec<u8>, MessageSealError> {
    if !verify_node_identity(recipient) {
        return Err(MessageSealError::InvalidIdentity);
    }
    if sealed.abi_version != WORK_WIRE_ABI_VERSION || sealed.to != recipient.node_id {
        return Err(MessageSealError::WrongRecipient);
    }
    let expected_recipient_key = encryption_public_from_ed25519_public(&recipient.public_key)?;
    if sealed.recipient_encryption_key != expected_recipient_key {
        return Err(MessageSealError::WrongRecipient);
    }
    if encryption_public_from_ed25519_key(recipient_key)? != expected_recipient_key {
        return Err(MessageSealError::WrongRecipient);
    }
    let recipient_secret = X25519Secret::from(encryption_secret_from_ed25519_key(recipient_key));
    let shared =
        recipient_secret.diffie_hellman(&X25519PublicKey::from(sealed.ephemeral_public_key));
    let key = message_key(
        shared.as_bytes(),
        &sealed.ephemeral_public_key,
        &sealed.recipient_encryption_key,
    );
    let cipher = Aes256GcmCipher::new(&key).map_err(|_| MessageSealError::InvalidKey)?;
    let mut plaintext = sealed.ciphertext.clone();
    let tag = Tag::from_slice(&sealed.tag);
    cipher
        .decrypt_in_place_detached(
            &Nonce::from(sealed.nonce),
            &sealed_message_aad(sealed),
            &mut plaintext,
            &tag,
        )
        .map_err(|_| MessageSealError::UnsealFailed)?;
    if blake3_hash(&plaintext) != sealed.plaintext_hash {
        return Err(MessageSealError::UnsealFailed);
    }
    Ok(plaintext)
}

pub fn sealed_message_aad(value: &SealedMessagePayload) -> Vec<u8> {
    PreimageBuilder::domain(MESSAGE_SEAL_DOMAIN)
        .node_id(&value.from)
        .node_id(&value.to)
        .bytes(&value.recipient_encryption_key)
        .bytes(&value.ephemeral_public_key)
        .bytes(&value.nonce)
        .hash(&value.plaintext_hash)
        .finish()
}

fn message_key(
    shared_secret: &[u8; X25519_KEY_LEN],
    ephemeral_public_key: &EncryptionPublicKey,
    recipient_encryption_key: &EncryptionPublicKey,
) -> [u8; X25519_KEY_LEN] {
    let salt = PreimageBuilder::domain(MESSAGE_SEAL_DOMAIN)
        .bytes(ephemeral_public_key)
        .bytes(recipient_encryption_key)
        .finish();
    let key = edgerun_crypto::hkdf_sha256(Some(&salt), shared_secret, MESSAGE_SEAL_KDF_INFO, 32);
    let mut out = [0u8; X25519_KEY_LEN];
    out.copy_from_slice(&key);
    out
}
