use edgerun_work::*;

fn node(byte: u8) -> NodeId {
    [byte; 32]
}

fn message(
    thread: &ThreadObject,
    from: NodeId,
    to: NodeId,
    sequence: u64,
    previous: Hash,
    body: &[u8],
) -> MessageObject {
    finalize_message_object(MessageObject {
        abi_version: WORK_WIRE_ABI_VERSION,
        message_id: [0u8; 32],
        thread_id: thread.thread_id,
        from,
        to,
        sequence,
        created_unix_ms: 1_000 + sequence,
        message_kind: CHAT_MESSAGE_KIND_TEXT,
        payload_hash: chat_payload_hash(body),
        payload_len: body.len() as u64,
        sealed_payload_hash: chat_payload_hash(body),
        storage_ref: Vec::new(),
        previous_message_hash: previous,
    })
}

#[test]
fn thread_id_is_stable_for_fixed_pair() {
    let alice = node(1);
    let bob = node(2);

    assert_eq!(
        thread_id_for_participants(alice, bob),
        thread_id_for_participants(bob, alice)
    );
}

#[test]
fn thread_appends_only_fixed_participants_in_order() {
    let alice = node(1);
    let bob = node(2);
    let mut thread = empty_thread(alice, bob, 1_000);
    let first = message(&thread, alice, bob, 1, thread.head_message_hash, b"hello");
    append_thread_message(&mut thread, &first).expect("append first");

    let second = message(&thread, bob, alice, 2, thread.head_message_hash, b"reply");
    append_thread_message(&mut thread, &second).expect("append second");

    assert_eq!(thread.message_count, 2);
    assert_eq!(thread.tail_message_hash, first.message_id);
    assert_eq!(thread.head_message_hash, second.message_id);
    assert_eq!(thread.entries.len(), 2);
}

#[test]
fn thread_rejects_wrong_participant_and_previous_hash() {
    let alice = node(1);
    let bob = node(2);
    let charlie = node(3);
    let mut thread = empty_thread(alice, bob, 1_000);
    let first = message(&thread, alice, bob, 1, thread.head_message_hash, b"hello");
    append_thread_message(&mut thread, &first).expect("append first");

    let wrong_participant = message(
        &thread,
        charlie,
        alice,
        2,
        thread.head_message_hash,
        b"not in thread",
    );
    assert_eq!(
        append_thread_message(&mut thread, &wrong_participant),
        Err(ChatIndexError::WrongParticipant)
    );

    let wrong_previous = message(&thread, bob, alice, 2, [9u8; 32], b"bad prev");
    assert_eq!(
        append_thread_message(&mut thread, &wrong_previous),
        Err(ChatIndexError::WrongPreviousMessage)
    );
}

#[test]
fn contact_book_summarizes_contacts_and_threads() {
    let owner = node(1);
    let contact = finalize_contact_object(ContactObject {
        abi_version: WORK_WIRE_ABI_VERSION,
        contact_id: [0u8; 32],
        primary_node_id: node(2),
        display_name: b"Bob".to_vec(),
        avatar_payload_hash: chat_payload_hash(b"avatar bytes"),
        public_ids: vec![ContactPublicId {
            kind: CHAT_PUBLIC_ID_KIND_EDGERUN_NODE,
            value: node(2).to_vec(),
            label: b"message node".to_vec(),
            verified_at_unix_ms: 1_000,
        }],
        external_contacts: vec![ExternalContactRef {
            kind: CHAT_EXTERNAL_CONTACT_KIND_EMAIL,
            value: b"bob@example.test".to_vec(),
            label: b"email".to_vec(),
        }],
        updated_unix_ms: 1_000,
    });
    let thread = empty_thread(owner, contact.primary_node_id, 1_000);
    let book = finalize_contact_book(ContactBook {
        abi_version: WORK_WIRE_ABI_VERSION,
        owner,
        contacts: vec![contact_book_ref_for_contact(&contact)],
        threads: vec![contact_book_ref_for_thread(&thread)],
        updated_unix_ms: 1_000,
        contact_book_hash: [0u8; 32],
    });

    assert_eq!(book.contacts[0].contact_id, contact.contact_id);
    assert_eq!(book.threads[0].thread_id, thread.thread_id);
    assert_eq!(book.contact_book_hash, contact_book_hash(&book));
}
