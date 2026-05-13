use core::ptr::{addr_of, addr_of_mut};

use edgerun_crypto::{Aes256GcmCipher, Ed25519SigningKey, Nonce, Tag};
use edgerun_work::{
    derive_node_id, encode_work_packet_once, node_identity_from_key,
    seal_message_for_recipient, sealed_message_object_from_network_message, sign_ed25519,
    sign_network_message_payload, simple_network_message_id, thread_id_for_participants,
    unseal_message_from_recipient_payload, verify_solana_ed25519, ChannelEnvelope, ChannelId, Hash,
    MessageObject, NodeIdentity, PublicKey, WorkPacket, CHANNEL_KIND_WEBSOCKET,
    CHAT_MESSAGE_KIND_TEXT, DEPARTMENT_MESSAGE, NODE_ROLE_MESSAGE, WORK_TYPE_MESSAGE_DELIVER,
    WORK_WIRE_ABI_VERSION,
};

const BUFFER_LEN: usize = 1024 * 1024;
const IDENTITY_OUTPUT_LEN: usize = 64;
const SEAL_FIXED_LEN: usize = 212;
const OPEN_FIXED_LEN: usize = 4;
const SEALED_RECORD_HEADER_LEN: usize = 292;
const IDENTITY_SEAL_MAGIC: &[u8] = b"EDGERUN-CHAT-ID1";
const IDENTITY_SEAL_KDF_INFO: &[u8] = b"edgerun:v1:work:chat-wasm:identity-seal";
const IDENTITY_SEAL_NONCE_LEN: usize = 12;
const IDENTITY_SEAL_TAG_LEN: usize = 16;
const OWNER_SEED_LEN: usize = 32;
const CONTACT_CARD_MAGIC: &[u8] = b"EDGERUN-CHAT-CONTACT1";
const CONTACT_CARD_DOMAIN: &[u8] = b"edgerun:v1:work:chat-contact-card";
const CONTACT_CARD_LEN: usize = 21 + 32 + 32 + 64;

static mut INPUT: [u8; BUFFER_LEN] = [0; BUFFER_LEN];
static mut OUTPUT: [u8; BUFFER_LEN] = [0; BUFFER_LEN];
static mut OWNER_SEED: [u8; 32] = [0; 32];
static mut OWNER_NODE_ID: [u8; 32] = [0; 32];
static mut OWNER_PUBLIC_KEY: [u8; 32] = [0; 32];
static mut OUTPUT_LEN: usize = 0;
static mut STATUS: u32 = 0;
static mut OWNER_READY: bool = false;

#[unsafe(no_mangle)]
pub extern "C" fn input_ptr() -> *mut u8 {
    addr_of_mut!(INPUT).cast::<u8>()
}

#[unsafe(no_mangle)]
pub extern "C" fn input_capacity() -> usize {
    BUFFER_LEN
}

#[unsafe(no_mangle)]
pub extern "C" fn output_ptr() -> *const u8 {
    addr_of!(OUTPUT).cast::<u8>()
}

#[unsafe(no_mangle)]
pub extern "C" fn output_len() -> usize {
    unsafe { OUTPUT_LEN }
}

#[unsafe(no_mangle)]
pub extern "C" fn status() -> u32 {
    unsafe { STATUS }
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_chat_init(input_len: usize) -> usize {
    reset_output();
    if input_len < 32 || input_len > BUFFER_LEN {
        return fail(1);
    }
    let input = input_slice(input_len);
    edgerun_crypto::mix_entropy(input);
    let mut seed = [0u8; 32];
    if edgerun_crypto::fill_random(&mut seed).is_err() {
        return fail(2);
    }
    set_owner_seed(seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_chat_public_identity(_input_len: usize) -> usize {
    reset_output();
    if !owner_ready() {
        return fail(1);
    }
    let (node_id, public_key) = owner_public_identity();
    write_output(&node_id, 0);
    write_output(&public_key, 32);
    finish(IDENTITY_OUTPUT_LEN)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_chat_seal_identity(input_len: usize) -> usize {
    reset_output();
    if input_len < 4 || input_len > BUFFER_LEN {
        return fail(1);
    }
    if !owner_ready() {
        return fail(2);
    }
    let input = input_slice(input_len);
    let unlock_len = u32::from_be_bytes(read_array::<4>(input, 0)) as usize;
    if unlock_len == 0 || 4usize.saturating_add(unlock_len) != input_len {
        return fail(3);
    }
    let unlock = &input[4..];
    let mut nonce = [0u8; IDENTITY_SEAL_NONCE_LEN];
    if edgerun_crypto::fill_random(&mut nonce).is_err() {
        return fail(4);
    }
    let key = identity_seal_key(unlock);
    let Ok(cipher) = Aes256GcmCipher::new(&key) else {
        return fail(5);
    };
    let mut ciphertext = owner_seed_bytes().to_vec();
    let Ok(tag) = cipher.encrypt_in_place_detached(
        &Nonce::from(nonce),
        IDENTITY_SEAL_MAGIC,
        &mut ciphertext,
    ) else {
        return fail(6);
    };
    let total_len =
        IDENTITY_SEAL_MAGIC.len() + IDENTITY_SEAL_NONCE_LEN + ciphertext.len() + IDENTITY_SEAL_TAG_LEN;
    if total_len > BUFFER_LEN {
        return fail(7);
    }
    write_output(IDENTITY_SEAL_MAGIC, 0);
    write_output(&nonce, IDENTITY_SEAL_MAGIC.len());
    write_output(&ciphertext, IDENTITY_SEAL_MAGIC.len() + IDENTITY_SEAL_NONCE_LEN);
    write_output(tag.as_slice(), IDENTITY_SEAL_MAGIC.len() + IDENTITY_SEAL_NONCE_LEN + ciphertext.len());
    finish(total_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_chat_unseal_identity(input_len: usize) -> usize {
    reset_output();
    if input_len < 8 || input_len > BUFFER_LEN {
        return fail(1);
    }
    let input = input_slice(input_len);
    let unlock_len = u32::from_be_bytes(read_array::<4>(input, 0)) as usize;
    let sealed_len = u32::from_be_bytes(read_array::<4>(input, 4)) as usize;
    if unlock_len == 0
        || sealed_len < IDENTITY_SEAL_MAGIC.len() + IDENTITY_SEAL_NONCE_LEN + OWNER_SEED_LEN + IDENTITY_SEAL_TAG_LEN
        || 8usize.saturating_add(unlock_len).saturating_add(sealed_len) != input_len
    {
        return fail(2);
    }
    let unlock = &input[8..8 + unlock_len];
    let sealed = &input[8 + unlock_len..];
    if &sealed[..IDENTITY_SEAL_MAGIC.len()] != IDENTITY_SEAL_MAGIC {
        return fail(3);
    }
    let nonce_start = IDENTITY_SEAL_MAGIC.len();
    let ciphertext_start = nonce_start + IDENTITY_SEAL_NONCE_LEN;
    let tag_start = sealed.len().saturating_sub(IDENTITY_SEAL_TAG_LEN);
    if tag_start < ciphertext_start {
        return fail(4);
    }
    let key = identity_seal_key(unlock);
    let Ok(cipher) = Aes256GcmCipher::new(&key) else {
        return fail(5);
    };
    let mut plaintext = sealed[ciphertext_start..tag_start].to_vec();
    let tag = Tag::from_slice(&sealed[tag_start..]);
    let nonce = Nonce::from_slice(&sealed[nonce_start..ciphertext_start]);
    if cipher
        .decrypt_in_place_detached(&nonce, IDENTITY_SEAL_MAGIC, &mut plaintext, &tag)
        .is_err()
    {
        return fail(6);
    }
    if plaintext.len() != OWNER_SEED_LEN {
        return fail(7);
    }
    let mut seed = [0u8; OWNER_SEED_LEN];
    seed.copy_from_slice(&plaintext);
    set_owner_seed(seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_chat_hash(input_len: usize) -> usize {
    reset_output();
    if input_len > BUFFER_LEN {
        return fail(1);
    }
    let hash = edgerun_work::blake3_hash(input_slice(input_len));
    write_output(&hash, 0);
    finish(32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_chat_thread_id(input_len: usize) -> usize {
    reset_output();
    if input_len != 64 {
        return fail(1);
    }
    let input = input_slice(input_len);
    let first = read_array::<32>(input, 0);
    let second = read_array::<32>(input, 32);
    let thread_id = thread_id_for_participants(first, second);
    write_output(&thread_id, 0);
    finish(32)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_chat_contact_card(_input_len: usize) -> usize {
    reset_output();
    if !owner_ready() {
        return fail(1);
    }
    let (node_id, public_key) = owner_public_identity();
    let preimage = contact_card_preimage(&node_id, &public_key);
    let signature = sign_ed25519(&owner_key(), &preimage);
    if signature.signature.len() != 64 {
        return fail(2);
    }
    write_output(CONTACT_CARD_MAGIC, 0);
    write_output(&node_id, CONTACT_CARD_MAGIC.len());
    write_output(&public_key, CONTACT_CARD_MAGIC.len() + 32);
    write_output(&signature.signature, CONTACT_CARD_MAGIC.len() + 64);
    finish(CONTACT_CARD_LEN)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_chat_import_contact_card(input_len: usize) -> usize {
    reset_output();
    if input_len != CONTACT_CARD_LEN {
        return fail(1);
    }
    let input = input_slice(input_len);
    if &input[..CONTACT_CARD_MAGIC.len()] != CONTACT_CARD_MAGIC {
        return fail(2);
    }
    let node_id = read_array::<32>(input, CONTACT_CARD_MAGIC.len());
    let public_key = read_array::<32>(input, CONTACT_CARD_MAGIC.len() + 32);
    let signature = &input[CONTACT_CARD_MAGIC.len() + 64..];
    if derive_node_id(&public_key, NODE_ROLE_MESSAGE) != node_id {
        return fail(3);
    }
    let preimage = contact_card_preimage(&node_id, &public_key);
    if !verify_solana_ed25519(&public_key, &preimage, signature) {
        return fail(4);
    }
    write_output(&node_id, 0);
    write_output(&public_key, 32);
    finish(IDENTITY_OUTPUT_LEN)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_chat_seal_envelope(input_len: usize) -> usize {
    reset_output();
    if input_len < SEAL_FIXED_LEN || input_len > BUFFER_LEN {
        return fail(1);
    }
    let input = input_slice(input_len);
    if !owner_ready() {
        return fail(7);
    }
    let recipient_node_id = read_array::<32>(input, 0);
    let recipient_public_key = read_array::<32>(input, 32);
    let via_relay = read_array::<32>(input, 64);
    let channel_id = read_array::<32>(input, 96);
    let route_hash = read_array::<32>(input, 128);
    let previous_message_hash = read_array::<32>(input, 160);
    let sequence = u64::from_be_bytes(read_array::<8>(input, 192));
    let created_unix_ms = u64::from_be_bytes(read_array::<8>(input, 200));
    let plaintext_len = u32::from_be_bytes(read_array::<4>(input, 208)) as usize;
    if SEAL_FIXED_LEN.saturating_add(plaintext_len) != input_len {
        return fail(2);
    }

    let sender_key = owner_key();
    let sender = node_identity_from_key(&sender_key, NODE_ROLE_MESSAGE);
    let recipient = NodeIdentity {
        node_id: recipient_node_id,
        role: NODE_ROLE_MESSAGE,
        public_key: recipient_public_key,
    };
    let plaintext = &input[SEAL_FIXED_LEN..];
    let Ok(sealed_payload) = seal_message_for_recipient(&sender, &recipient, plaintext) else {
        return fail(3);
    };

    let payload_hash = edgerun_work::blake3_hash(&sealed_payload);
    let message_id = simple_network_message_id(
        &sender.node_id,
        &recipient.node_id,
        sequence,
        &payload_hash,
    );
    let message = sign_network_message_payload(
        &sender_key,
        message_id,
        previous_message_hash,
        sender.node_id,
        recipient.node_id,
        via_relay,
        DEPARTMENT_MESSAGE,
        WORK_TYPE_MESSAGE_DELIVER,
        sequence,
        sealed_payload,
    );
    let Ok(message_object) = sealed_message_object_from_network_message(
        &message,
        created_unix_ms,
        CHAT_MESSAGE_KIND_TEXT,
    ) else {
        return fail(8);
    };
    let packet = WorkPacket::NetworkMessage(message);
    let Ok(encoded) = encode_work_packet_once(&packet) else {
        return fail(4);
    };
    let envelope = ChannelEnvelope {
        abi_version: WORK_WIRE_ABI_VERSION,
        channel_id,
        from: sender.node_id,
        to: recipient.node_id,
        route_hash,
        packet_hash: encoded.hash,
        packet,
    };
    let Ok(bytes) = edgerun_work::channel_envelope_bytes(&envelope) else {
        return fail(5);
    };
    if bytes.len() > BUFFER_LEN {
        return fail(6);
    }
    let frame_hash = edgerun_work::blake3_hash(&bytes);
    let total_len = SEALED_RECORD_HEADER_LEN + bytes.len();
    if total_len > BUFFER_LEN {
        return fail(9);
    }
    write_sealed_record_header(&message_object, &frame_hash, bytes.len() as u32);
    write_output(&bytes, SEALED_RECORD_HEADER_LEN);
    finish(total_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_chat_open_envelope(input_len: usize) -> usize {
    reset_output();
    if input_len < OPEN_FIXED_LEN || input_len > BUFFER_LEN {
        return fail(1);
    }
    let input = input_slice(input_len);
    if !owner_ready() {
        return fail(8);
    }
    let envelope_len = u32::from_be_bytes(read_array::<4>(input, 0)) as usize;
    if OPEN_FIXED_LEN.saturating_add(envelope_len) != input_len {
        return fail(2);
    }
    let recipient_key = owner_key();
    let recipient = node_identity_from_key(&recipient_key, NODE_ROLE_MESSAGE);
    let Ok(envelope) = edgerun_work::channel_envelope_from_bytes(&input[OPEN_FIXED_LEN..]) else {
        return fail(3);
    };
    if envelope.channel_id == [0u8; 32] || envelope.packet_hash == [0u8; 32] {
        return fail(4);
    }
    let WorkPacket::NetworkMessage(message) = envelope.packet else {
        return fail(5);
    };
    let Ok(plaintext) =
        unseal_message_from_recipient_payload(&recipient_key, &recipient, &message.payload)
    else {
        return fail(6);
    };
    if plaintext.len() > BUFFER_LEN {
        return fail(7);
    }
    write_output(&plaintext, 0);
    finish(plaintext.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_chat_index_envelope(input_len: usize) -> usize {
    reset_output();
    if input_len < OPEN_FIXED_LEN || input_len > BUFFER_LEN {
        return fail(1);
    }
    let input = input_slice(input_len);
    let envelope_len = u32::from_be_bytes(read_array::<4>(input, 0)) as usize;
    if OPEN_FIXED_LEN.saturating_add(envelope_len) != input_len {
        return fail(2);
    }
    let frame = &input[OPEN_FIXED_LEN..];
    let Ok(envelope) = edgerun_work::channel_envelope_from_bytes(frame) else {
        return fail(3);
    };
    let WorkPacket::NetworkMessage(message) = envelope.packet else {
        return fail(4);
    };
    let Ok(message_object) =
        sealed_message_object_from_network_message(&message, 0, CHAT_MESSAGE_KIND_TEXT)
    else {
        return fail(5);
    };
    let frame_hash = edgerun_work::blake3_hash(frame);
    let total_len = SEALED_RECORD_HEADER_LEN + frame.len();
    if total_len > BUFFER_LEN {
        return fail(6);
    }
    write_sealed_record_header(&message_object, &frame_hash, frame.len() as u32);
    write_output(frame, SEALED_RECORD_HEADER_LEN);
    finish(total_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_chat_channel_kind_websocket() -> u16 {
    CHANNEL_KIND_WEBSOCKET
}

fn contact_card_preimage(node_id: &[u8; 32], public_key: &[u8; 32]) -> Vec<u8> {
    let mut preimage = CONTACT_CARD_DOMAIN.to_vec();
    preimage.extend_from_slice(node_id);
    preimage.extend_from_slice(public_key);
    preimage
}

fn write_sealed_record_header(message: &MessageObject, frame_hash: &Hash, frame_len: u32) {
    write_output(&message.message_id, 0);
    write_output(&message.thread_id, 32);
    write_output(&message.payload_hash, 64);
    write_output(&message.sealed_payload_hash, 96);
    write_output(frame_hash, 128);
    write_output(&message.from, 160);
    write_output(&message.to, 192);
    write_output(&message.previous_message_hash, 224);
    write_output(&message.sequence.to_be_bytes(), 256);
    write_output(&message.created_unix_ms.to_be_bytes(), 264);
    write_output(&message.payload_len.to_be_bytes(), 272);
    write_output(&frame_len.to_be_bytes(), 280);
    write_output(&[0u8; 8], 284);
}

fn set_owner_seed(seed: [u8; 32]) -> usize {
    let key = Ed25519SigningKey::from_bytes(&seed);
    let identity = node_identity_from_key(&key, NODE_ROLE_MESSAGE);
    unsafe {
        OWNER_SEED = seed;
        OWNER_NODE_ID = identity.node_id;
        OWNER_PUBLIC_KEY = identity.public_key;
        OWNER_READY = true;
    }
    write_output(&identity.node_id, 0);
    write_output(&identity.public_key, 32);
    finish(IDENTITY_OUTPUT_LEN)
}

fn owner_ready() -> bool {
    unsafe { OWNER_READY }
}

fn owner_key() -> Ed25519SigningKey {
    let seed = unsafe { OWNER_SEED };
    Ed25519SigningKey::from_bytes(&seed)
}

fn owner_seed_bytes() -> [u8; OWNER_SEED_LEN] {
    unsafe { OWNER_SEED }
}

fn owner_public_identity() -> ([u8; 32], [u8; 32]) {
    unsafe { (OWNER_NODE_ID, OWNER_PUBLIC_KEY) }
}

fn identity_seal_key(unlock: &[u8]) -> [u8; 32] {
    let salt = edgerun_work::blake3_hash(IDENTITY_SEAL_MAGIC);
    let key = edgerun_crypto::hkdf_sha256(Some(&salt), unlock, IDENTITY_SEAL_KDF_INFO, 32);
    let mut out = [0u8; 32];
    out.copy_from_slice(&key);
    out
}

fn input_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(addr_of!(INPUT).cast::<u8>(), len) }
}

fn read_array<const N: usize>(input: &[u8], offset: usize) -> [u8; N] {
    let mut out = [0u8; N];
    out.copy_from_slice(&input[offset..offset + N]);
    out
}

fn write_output(bytes: &[u8], offset: usize) {
    unsafe {
        core::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            addr_of_mut!(OUTPUT).cast::<u8>().add(offset),
            bytes.len(),
        );
    }
}

fn reset_output() {
    unsafe {
        OUTPUT_LEN = 0;
        STATUS = 0;
    }
}

fn finish(len: usize) -> usize {
    unsafe {
        OUTPUT_LEN = len;
        STATUS = 0;
    }
    len
}

fn fail(code: u32) -> usize {
    unsafe {
        OUTPUT_LEN = 0;
        STATUS = code;
    }
    0
}

#[allow(dead_code)]
fn _assert_wire_types(_: ChannelId, _: PublicKey) {}
