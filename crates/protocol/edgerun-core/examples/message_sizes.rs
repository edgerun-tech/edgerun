use edgerun_core::protocol::{
    CipherSuite, Directness, EncryptedEnvelope, IdentityKind, IdentityRef, NodeRef, ObjectKind,
    ObjectRef, PayloadKind, ProtocolRecord, ReachabilityHint, RecipientKey, RelayEnvelope,
    RouteAdvertisement, SessionAccept, SessionHello, Signature, Timestamp, TransportClass,
    relay_envelope,
};

fn bytes(len: usize, value: u8) -> Vec<u8> {
    vec![value; len]
}

macro_rules! wire_len {
    ($value:expr) => {
        edgerun_wire::to_bytes::<edgerun_wire::WireError>($value)
            .expect("protocol size probe value must serialize")
            .len()
    };
}

fn protocol_len(record: &ProtocolRecord, signable: bool) -> usize {
    edgerun_core::protocol::protocol_wire_bytes(record, signable).len()
}

fn node_ref(tag: u8) -> NodeRef {
    NodeRef {
        node_id: bytes(64, tag),
    }
}

fn identity_ref(tag: u8) -> IdentityRef {
    IdentityRef {
        identity_id: bytes(64, tag),
        identity_kind: Some(IdentityKind::Node as i32),
        key_hint: Some(bytes(64, tag)),
    }
}

fn signature(tag: u8) -> Signature {
    Signature {
        algorithm: 1,
        value: bytes(64, tag),
    }
}

fn object_ref(tag: u8) -> ObjectRef {
    ObjectRef {
        object_id: bytes(32, tag),
        object_kind: Some(ObjectKind::Payload as i32),
    }
}

fn timestamp(seconds: i64) -> Timestamp {
    Timestamp { seconds, nanos: 0 }
}

fn reachability_hint() -> ReachabilityHint {
    ReachabilityHint {
        hint_version: 1,
        subject_node: Some(node_ref(0x10)),
        transport_class: TransportClass::Quic as i32,
        locator_payload: b"quic://relay.example:443".to_vec(),
        directness: Directness::Relayed as i32,
        valid_after: Some(timestamp(1_700_000_000)),
        valid_until: Some(timestamp(1_700_003_600)),
        cost_hint: Some(10),
        quality_hint: Some(90),
        issuer: Some(identity_ref(0x11)),
        signature: Some(signature(0x12)),
    }
}

fn reachability_hint_minimal() -> ReachabilityHint {
    ReachabilityHint {
        hint_version: 1,
        subject_node: Some(node_ref(0x10)),
        transport_class: TransportClass::Quic as i32,
        locator_payload: b"q://r".to_vec(),
        directness: Directness::Relayed as i32,
        valid_after: None,
        valid_until: None,
        cost_hint: None,
        quality_hint: None,
        issuer: None,
        signature: None,
    }
}

fn route_advertisement() -> RouteAdvertisement {
    RouteAdvertisement {
        advertisement_version: 1,
        target_node: Some(node_ref(0x20)),
        advertiser: Some(identity_ref(0x21)),
        next_hop_node: Some(node_ref(0x22)),
        reachability: vec![reachability_hint()],
        metric_hint: Some(object_ref(0x23)),
        advertised_at: Some(timestamp(1_700_000_000)),
        expires_at: Some(timestamp(1_700_003_600)),
        route_metadata: Some(object_ref(0x24)),
        signature: Some(signature(0x25)),
    }
}

fn route_advertisement_minimal_signed() -> RouteAdvertisement {
    RouteAdvertisement {
        advertisement_version: 1,
        target_node: Some(node_ref(0x20)),
        advertiser: Some(identity_ref(0x21)),
        next_hop_node: None,
        reachability: vec![reachability_hint_minimal()],
        metric_hint: None,
        advertised_at: Some(timestamp(1_700_000_000)),
        expires_at: None,
        route_metadata: None,
        signature: Some(signature(0x25)),
    }
}

fn session_hello() -> SessionHello {
    SessionHello {
        message_version: 1,
        initiator: Some(identity_ref(0x30)),
        target_node: Some(node_ref(0x31)),
        supported_transport_features: vec!["datagram".into(), "object-fetch".into()],
        supported_protocol_versions: vec![1],
        session_nonce: bytes(32, 0x32),
        initiator_locators: vec![reachability_hint()],
        hello_metadata: Some(object_ref(0x33)),
        signature: Some(signature(0x34)),
    }
}

fn session_accept() -> SessionAccept {
    SessionAccept {
        message_version: 1,
        responder: Some(identity_ref(0x40)),
        echoed_session_nonce: bytes(32, 0x32),
        selected_protocol_version: 1,
        selected_transport_features: vec!["datagram".into(), "object-fetch".into()],
        responder_locators: vec![reachability_hint()],
        accept_metadata: Some(object_ref(0x43)),
        signature: Some(signature(0x44)),
    }
}

fn relay_envelope_inline(payload_len: usize) -> RelayEnvelope {
    RelayEnvelope {
        envelope_version: 1,
        relay_message_id: bytes(32, 0x50),
        original_sender: Some(identity_ref(0x51)),
        intended_recipient_node: Some(node_ref(0x52)),
        relay_chain: vec![identity_ref(0x53), identity_ref(0x54)],
        payload_kind: PayloadKind::SessionMessage as i32,
        store_until: Some(timestamp(1_700_003_600)),
        relay_metadata: Some(object_ref(0x55)),
        signature: Some(signature(0x56)),
        payload: Some(relay_envelope::Payload::InlinePayload(bytes(
            payload_len,
            0x57,
        ))),
    }
}

fn relay_envelope_object() -> RelayEnvelope {
    let mut envelope = relay_envelope_inline(0);
    envelope.payload = Some(relay_envelope::Payload::PayloadObject(object_ref(0x58)));
    envelope
}

fn relay_envelope_object_minimal() -> RelayEnvelope {
    RelayEnvelope {
        envelope_version: 1,
        relay_message_id: bytes(32, 0x50),
        original_sender: Some(identity_ref(0x51)),
        intended_recipient_node: Some(node_ref(0x52)),
        relay_chain: Vec::new(),
        payload_kind: PayloadKind::ObjectFragment as i32,
        store_until: None,
        relay_metadata: None,
        signature: Some(signature(0x56)),
        payload: Some(relay_envelope::Payload::PayloadObject(object_ref(0x58))),
    }
}

fn recipient_key(tag: u8) -> RecipientKey {
    RecipientKey {
        identity: bytes(65, tag),
        encrypted_key: bytes(48, tag.wrapping_add(1)),
        key_nonce: bytes(12, tag.wrapping_add(2)),
    }
}

fn encrypted_envelope(plaintext_len: usize, recipients: usize) -> EncryptedEnvelope {
    EncryptedEnvelope {
        version: 1,
        sender: bytes(65, 0x60),
        recipients: (0..recipients)
            .map(|idx| recipient_key(0x61 + idx as u8))
            .collect(),
        ephemeral_pubkey: bytes(65, 0x70),
        cipher: CipherSuite::Aes256Gcm as i32,
        nonce: bytes(12, 0x71),
        ciphertext: bytes(plaintext_len + 16, 0x72),
        tag: Vec::new(),
        signature: bytes(64, 0x73),
    }
}

fn print_row(name: &str, bytes: usize) {
    println!("{name:42} {bytes:5} B");
}

fn main() {
    let hint = reachability_hint();
    let route = route_advertisement();

    println!("Edgerun current rkyv message size probe");
    println!("assumptions: 64 B node/identity ids, 64 B P-256 signatures, 32 B object ids");
    println!();

    print_row("ReachabilityHint", wire_len!(&hint));
    print_row(
        "ReachabilityHint minimal",
        wire_len!(&reachability_hint_minimal()),
    );
    print_row(
        "RouteAdvertisement full",
        protocol_len(&ProtocolRecord::RouteAdvertisement(route.clone()), false),
    );
    print_row(
        "RouteAdvertisement signable",
        protocol_len(&ProtocolRecord::RouteAdvertisement(route), true),
    );
    print_row(
        "RouteAdvertisement min signed",
        protocol_len(
            &ProtocolRecord::RouteAdvertisement(route_advertisement_minimal_signed()),
            false,
        ),
    );
    print_row("SessionHello", wire_len!(&session_hello()));
    print_row("SessionAccept", wire_len!(&session_accept()));
    print_row(
        "RelayEnvelope object payload",
        wire_len!(&relay_envelope_object()),
    );
    print_row(
        "RelayEnvelope object minimal",
        wire_len!(&relay_envelope_object_minimal()),
    );

    for payload_len in [0, 256, 512, 1024, 4096] {
        print_row(
            &format!("RelayEnvelope inline {payload_len} B"),
            wire_len!(&relay_envelope_inline(payload_len)),
        );
    }

    for recipients in [1, 2, 5] {
        for plaintext_len in [0, 256, 512, 1024, 4096] {
            print_row(
                &format!("EncryptedEnvelope {recipients}r {plaintext_len} B"),
                wire_len!(&encrypted_envelope(plaintext_len, recipients)),
            );
        }
    }
}
