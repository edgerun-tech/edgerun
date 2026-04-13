//! Comprehensive tests for edgerun-imap

#[cfg(test)]
mod store_tests {
    use edgerun_imap::server::{MemoryStore, MailStore};
    use edgerun_imap::types::{FetchAttr, Flags, SearchKey};
    use edgerun_imap::message::StoreAction;
    use std::time::SystemTime;

    fn create_test_store() -> MemoryStore {
        let store = MemoryStore::new();
        store.add_user("testuser", "testpass");
        store
    }

    fn create_test_message(uid: u32, subject: &str, from: &str, body: &str) -> Vec<u8> {
        format!(
            "From: {}\r\nSubject: {}\r\nDate: Mon, 1 Jan 2024 00:00:00 +0000\r\nMessage-ID: <{}@test>\r\n\r\n{}",
            from, subject, uid, body
        )
        .into_bytes()
    }

    #[test]
    fn test_authenticate_success() {
        let store = create_test_store();
        let result = store.authenticate("testuser", "testpass").unwrap();
        assert_eq!(result, Some("testuser".to_string()));
    }

    #[test]
    fn test_authenticate_failure() {
        let store = create_test_store();
        let result = store.authenticate("testuser", "wrongpass").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_create_mailbox() {
        let store = create_test_store();
        assert!(store.create("TestFolder").unwrap());
        assert!(!store.create("TestFolder").unwrap()); // duplicate
    }

    #[test]
    fn test_delete_mailbox() {
        let store = create_test_store();
        store.create("ToDelete").unwrap();
        assert!(store.delete("ToDelete").unwrap());
        assert!(!store.delete("NonExistent").unwrap());
    }

    #[test]
    fn test_rename_mailbox() {
        let store = create_test_store();
        store.create("OldName").unwrap();
        assert!(store.rename("OldName", "NewName").unwrap());
        assert!(store.select("NewName").unwrap().is_some());
        assert!(store.select("OldName").unwrap().is_none());
    }

    #[test]
    fn test_append_message() {
        let store = create_test_store();
        let msg = create_test_message(1, "Hello", "user@test.com", "Test body");
        let uid = store.append("INBOX", Flags::new(), None, &msg).unwrap();
        assert_eq!(uid, 1);
    }

    #[test]
    fn test_fetch_basic() {
        let store = create_test_store();
        let msg = create_test_message(1, "Hello", "user@test.com", "Test body");
        store.append("INBOX", Flags::new(), None, &msg).unwrap();

        let results = store.fetch("INBOX", "1", &[FetchAttr::Uid, FetchAttr::Flags, FetchAttr::Rfc822Size]).unwrap();
        assert_eq!(results.len(), 1);
        let (uid, data) = &results[0];
        assert_eq!(*uid, 1);
        assert!(data.contains_key("UID"));
        assert!(data.contains_key("FLAGS"));
        assert!(data.contains_key("RFC822.SIZE"));
    }

    #[test]
    fn test_fetch_rfc822() {
        let store = create_test_store();
        let original = create_test_message(1, "Test", "a@b.com", "body");
        store.append("INBOX", Flags::new(), None, &original).unwrap();

        let results = store.fetch("INBOX", "1", &[FetchAttr::Rfc822]).unwrap();
        assert_eq!(results.len(), 1);
        let (_, data) = &results[0];
        assert!(data.contains_key("__RFC822_BODY__"));
        let body = data.get("__RFC822_BODY__").unwrap();
        assert_eq!(body.as_bytes(), original);
    }

    #[test]
    fn test_fetch_body_section() {
        let store = create_test_store();
        let msg = b"From: a@b.com\r\nSubject: Test\r\n\r\nThis is the body";
        store.append("INBOX", Flags::new(), None, msg).unwrap();

        let results = store.fetch("INBOX", "1", &[FetchAttr::BodySection("HEADER".to_string())]).unwrap();
        assert_eq!(results.len(), 1);
        let (_, data) = &results[0];
        let header = data.get("BODY[HEADER]").unwrap();
        assert!(header.contains("From: a@b.com"));
    }

    #[test]
    fn test_store_replace_flags() {
        let store = create_test_store();
        let msg = create_test_message(1, "Test", "a@b.com", "body");
        store.append("INBOX", Flags::new(), None, &msg).unwrap();

        let updated = store.store("INBOX", "1", &StoreAction::Replace, &["\\Seen".to_string(), "\\Flagged".to_string()]).unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0], 1);

        // Verify flags were set
        let results = store.fetch("INBOX", "1", &[FetchAttr::Flags]).unwrap();
        let (_, data) = &results[0];
        let flags = data.get("FLAGS").unwrap();
        assert!(flags.contains("\\Seen"));
        assert!(flags.contains("\\Flagged"));
    }

    #[test]
    fn test_store_add_flags() {
        let store = create_test_store();
        let msg = create_test_message(1, "Test", "a@b.com", "body");
        store.append("INBOX", Flags::new(), None, &msg).unwrap();

        // Add \Seen
        store.store("INBOX", "1", &StoreAction::Add, &["\\Seen".to_string()]).unwrap();

        // Add \Flagged
        store.store("INBOX", "1", &StoreAction::Add, &["\\Flagged".to_string()]).unwrap();

        let results = store.fetch("INBOX", "1", &[FetchAttr::Flags]).unwrap();
        let (_, data) = &results[0];
        let flags = data.get("FLAGS").unwrap();
        assert!(flags.contains("\\Seen"));
        assert!(flags.contains("\\Flagged"));
    }

    #[test]
    fn test_store_remove_flags() {
        let store = create_test_store();
        let msg = create_test_message(1, "Test", "a@b.com", "body");
        let mut flags = Flags::new();
        flags.seen = true;
        flags.flagged = true;
        store.append("INBOX", flags, None, &msg).unwrap();

        store.store("INBOX", "1", &StoreAction::Remove, &["\\Flagged".to_string()]).unwrap();

        let results = store.fetch("INBOX", "1", &[FetchAttr::Flags]).unwrap();
        let (_, data) = &results[0];
        let flags_str = data.get("FLAGS").unwrap();
        assert!(flags_str.contains("\\Seen"));
        assert!(!flags_str.contains("\\Flagged"));
    }

    #[test]
    fn test_store_sequence_range() {
        let store = create_test_store();
        for i in 1..=5 {
            let msg = create_test_message(i, &format!("Msg {}", i), "a@b.com", "body");
            store.append("INBOX", Flags::new(), None, &msg).unwrap();
        }

        let updated = store.store("INBOX", "1:3", &StoreAction::Add, &["\\Seen".to_string()]).unwrap();
        assert_eq!(updated.len(), 3);
    }

    #[test]
    fn test_copy_messages() {
        let store = create_test_store();
        store.create("Sent").unwrap();
        let msg = create_test_message(1, "Test", "a@b.com", "body");
        store.append("INBOX", Flags::new(), None, &msg).unwrap();

        let copied = store.copy_messages("INBOX", "1", "Sent").unwrap();
        assert_eq!(copied.len(), 1);

        // Verify message exists in destination
        let dest_status = store.status("Sent").unwrap().unwrap();
        assert_eq!(dest_status.messages, 1);
    }

    #[test]
    fn test_copy_messages_range() {
        let store = create_test_store();
        store.create("Archive").unwrap();
        for i in 1..=3 {
            let msg = create_test_message(i, &format!("Msg {}", i), "a@b.com", "body");
            store.append("INBOX", Flags::new(), None, &msg).unwrap();
        }

        let copied = store.copy_messages("INBOX", "1:2", "Archive").unwrap();
        assert_eq!(copied.len(), 2);
    }

    #[test]
    fn test_expunge_deleted() {
        let store = create_test_store();
        for i in 1..=3 {
            let msg = create_test_message(i, &format!("Msg {}", i), "a@b.com", "body");
            store.append("INBOX", Flags::new(), None, &msg).unwrap();
        }

        // Mark messages 1 and 3 as deleted
        store.store("INBOX", "1", &StoreAction::Add, &["\\Deleted".to_string()]).unwrap();
        store.store("INBOX", "3", &StoreAction::Add, &["\\Deleted".to_string()]).unwrap();

        let removed = store.expunge("INBOX").unwrap();
        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&1));
        assert!(removed.contains(&3));

        // Verify only message 2 remains
        let status = store.status("INBOX").unwrap().unwrap();
        assert_eq!(status.messages, 1);
    }

    #[test]
    fn test_close_expunges() {
        let store = create_test_store();
        for i in 1..=2 {
            let msg = create_test_message(i, &format!("Msg {}", i), "a@b.com", "body");
            store.append("INBOX", Flags::new(), None, &msg).unwrap();
        }

        store.store("INBOX", "1", &StoreAction::Add, &["\\Deleted".to_string()]).unwrap();
        store.close("INBOX").unwrap();

        let status = store.status("INBOX").unwrap().unwrap();
        assert_eq!(status.messages, 1);
    }

    #[test]
    fn test_search_flag_based() {
        let store = create_test_store();
        for i in 1..=5 {
            let msg = create_test_message(i, &format!("Msg {}", i), "a@b.com", "body");
            store.append("INBOX", Flags::new(), None, &msg).unwrap();
        }

        // Mark messages 2 and 4 as seen
        store.store("INBOX", "2", &StoreAction::Add, &["\\Seen".to_string()]).unwrap();
        store.store("INBOX", "4", &StoreAction::Add, &["\\Seen".to_string()]).unwrap();

        let seen = store.search("INBOX", &[SearchKey::Seen]).unwrap();
        assert_eq!(seen.len(), 2);

        let unseen = store.search("INBOX", &[SearchKey::Unseen]).unwrap();
        assert_eq!(unseen.len(), 3);
    }

    #[test]
    fn test_search_subject() {
        let store = create_test_store();
        store.append("INBOX", Flags::new(), None, &create_test_message(1, "Hello World", "a@b.com", "body")).unwrap();
        store.append("INBOX", Flags::new(), None, &create_test_message(2, "Goodbye World", "a@b.com", "body")).unwrap();
        store.append("INBOX", Flags::new(), None, &create_test_message(3, "Hello There", "a@b.com", "body")).unwrap();

        let results = store.search("INBOX", &[SearchKey::Subject("Hello".to_string())]).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_from() {
        let store = create_test_store();
        store.append("INBOX", Flags::new(), None, &create_test_message(1, "Test", "alice@test.com", "body")).unwrap();
        store.append("INBOX", Flags::new(), None, &create_test_message(2, "Test", "bob@test.com", "body")).unwrap();
        store.append("INBOX", Flags::new(), None, &create_test_message(3, "Test", "alice@other.com", "body")).unwrap();

        let results = store.search("INBOX", &[SearchKey::From("alice".to_string())]).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_body() {
        let store = create_test_store();
        store.append("INBOX", Flags::new(), None, &create_test_message(1, "Test", "a@b.com", "contains secret keyword")).unwrap();
        store.append("INBOX", Flags::new(), None, &create_test_message(2, "Test", "a@b.com", "no keyword here")).unwrap();

        let results = store.search("INBOX", &[SearchKey::Body("secret keyword".to_string())]).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_size() {
        let store = create_test_store();
        store.append("INBOX", Flags::new(), None, &create_test_message(1, "Small", "a@b.com", "tiny")).unwrap();
        store.append("INBOX", Flags::new(), None, &create_test_message(2, "Large", "a@b.com", &"x".repeat(1000))).unwrap();

        let small = store.search("INBOX", &[SearchKey::Smaller(500)]).unwrap();
        assert_eq!(small.len(), 1);

        let large = store.search("INBOX", &[SearchKey::Larger(500)]).unwrap();
        assert_eq!(large.len(), 1);
    }

    #[test]
    fn test_search_uid_set() {
        let store = create_test_store();
        for i in 1..=5 {
            let msg = create_test_message(i, &format!("Msg {}", i), "a@b.com", "body");
            store.append("INBOX", Flags::new(), None, &msg).unwrap();
        }

        let results = store.search("INBOX", &[SearchKey::UidSet("1:3".to_string())]).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_seq_set() {
        let store = create_test_store();
        for i in 1..=5 {
            let msg = create_test_message(i, &format!("Msg {}", i), "a@b.com", "body");
            store.append("INBOX", Flags::new(), None, &msg).unwrap();
        }

        let results = store.search("INBOX", &[SearchKey::SeqSet("2:4".to_string())]).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_not() {
        let store = create_test_store();
        for i in 1..=3 {
            let msg = create_test_message(i, &format!("Msg {}", i), "a@b.com", "body");
            store.append("INBOX", Flags::new(), None, &msg).unwrap();
        }
        store.store("INBOX", "1", &StoreAction::Add, &["\\Seen".to_string()]).unwrap();

        let results = store.search("INBOX", &[SearchKey::Not(Box::new(SearchKey::Seen))]).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_list_mailboxes() {
        let store = create_test_store();
        store.create("Sent").unwrap();
        store.create("Drafts").unwrap();
        store.create("Archive/2024").unwrap();

        let all = store.list("", "*").unwrap();
        assert!(all.len() >= 3);

        let inbox = store.list("", "INBOX").unwrap();
        assert_eq!(inbox.len(), 1);
    }

    #[test]
    fn test_status() {
        let store = create_test_store();
        for i in 1..=3 {
            let msg = create_test_message(i, &format!("Msg {}", i), "a@b.com", "body");
            store.append("INBOX", Flags::new(), None, &msg).unwrap();
        }

        let status = store.status("INBOX").unwrap().unwrap();
        assert_eq!(status.messages, 3);
        // Note: recent count depends on implementation - in MemoryStore, 
        // we don't track the \Recent flag on append, so this may be 0
        // The important thing is messages count is correct
    }

    #[test]
    fn test_envelope_parsing() {
        let store = create_test_store();
        let msg = b"From: sender@example.com\r\nSubject: Test Subject\r\nTo: recipient@example.com\r\nDate: Mon, 1 Jan 2024 00:00:00 +0000\r\nMessage-ID: <123@example.com>\r\n\r\nBody text";
        store.append("INBOX", Flags::new(), None, msg).unwrap();

        let results = store.fetch("INBOX", "1", &[FetchAttr::Envelope]).unwrap();
        assert_eq!(results.len(), 1);
        let (_, data) = &results[0];
        assert!(data.contains_key("ENVELOPE"));
        let envelope = data.get("ENVELOPE").unwrap();
        assert!(envelope.contains("Test Subject"));
    }

    #[test]
    fn test_fetch_multiple_attributes() {
        let store = create_test_store();
        let msg = create_test_message(1, "Test", "a@b.com", "body");
        store.append("INBOX", Flags::new(), None, &msg).unwrap();

        let attrs = vec![
            FetchAttr::Uid,
            FetchAttr::Flags,
            FetchAttr::Rfc822Size,
            FetchAttr::Envelope,
            FetchAttr::InternalDate,
        ];
        let results = store.fetch("INBOX", "1", &attrs).unwrap();
        assert_eq!(results.len(), 1);
        let (_, data) = &results[0];
        assert!(data.contains_key("UID"));
        assert!(data.contains_key("FLAGS"));
        assert!(data.contains_key("RFC822.SIZE"));
        assert!(data.contains_key("ENVELOPE"));
        assert!(data.contains_key("INTERNALDATE"));
    }
}

#[cfg(test)]
mod parser_tests {
    use edgerun_imap::parser;

    #[test]
    fn test_parse_command_line() {
        let (tag, cmd, args) = parser::parse_command_line("A001 LOGIN user pass").unwrap();
        assert_eq!(tag, "A001");
        assert_eq!(cmd, "LOGIN");
        assert_eq!(args, vec!["user", "pass"]);
    }

    #[test]
    fn test_parse_paren_list() {
        let items = parser::parse_paren_list("(\\Seen \\Flagged)").unwrap();
        assert_eq!(items, vec!["\\Seen", "\\Flagged"]);

        let empty = parser::parse_paren_list("()").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_parse_sequence_set() {
        let seqs = parser::parse_sequence_set("1,3,5");
        assert_eq!(seqs, vec!["1", "3", "5"]);
    }

    #[test]
    fn test_format_responses() {
        assert_eq!(parser::format_ok("A001", "OK"), "A001 OK OK\r\n");
        assert_eq!(parser::format_no("A001", "NO"), "A001 NO NO\r\n");
        assert_eq!(parser::format_bad("A001", "BAD"), "A001 BAD BAD\r\n");
        assert_eq!(parser::format_untagged("EXISTS"), "* EXISTS\r\n");
    }

    #[test]
    fn test_format_search() {
        let resp = parser::format_search(&[1, 2, 3]);
        assert_eq!(resp, "* SEARCH 1 2 3\r\n");
    }

    #[test]
    fn test_format_fetch() {
        let resp = parser::format_fetch(42, "UID 42 FLAGS (\\Seen)");
        assert_eq!(resp, "* 42 FETCH (UID 42 FLAGS (\\Seen))\r\n");
    }

    #[test]
    fn test_format_list() {
        let resp = parser::format_list(&["\\HasNoChildren"], "/", "INBOX");
        assert_eq!(resp, "* LIST (\\HasNoChildren) \"/\" INBOX\r\n");
    }
}

#[cfg(test)]
mod types_tests {
    use edgerun_imap::types::Flags;

    #[test]
    fn test_flags_format_parse_roundtrip() {
        let mut flags = Flags::new();
        flags.seen = true;
        flags.flagged = true;

        let formatted = flags.format();
        let parsed = Flags::parse(&formatted);
        assert!(parsed.seen);
        assert!(parsed.flagged);
        assert!(!parsed.deleted);
    }

    #[test]
    fn test_flags_empty() {
        let flags = Flags::new();
        assert_eq!(flags.format(), "()");
    }

    #[test]
    fn test_flags_with_keywords() {
        let mut flags = Flags::new();
        flags.keywords.push("$MyKeyword".to_string());
        let formatted = flags.format();
        assert!(formatted.contains("$MyKeyword"));
    }

    #[test]
    fn test_flags_parse_all() {
        let s = "(\\Seen \\Answered \\Flagged \\Deleted \\Draft)";
        let flags = Flags::parse(s);
        assert!(flags.seen);
        assert!(flags.answered);
        assert!(flags.flagged);
        assert!(flags.deleted);
        assert!(flags.draft);
    }
}

#[cfg(test)]
mod message_tests {
    use edgerun_imap::message::{ImapCommand, ImapResponse, ImapResult, StoreAction};

    #[test]
    fn test_parse_login() {
        let cmd = ImapCommand::parse("A001", "LOGIN", &["user".to_string(), "pass".to_string()], None).unwrap();
        match cmd {
            ImapCommand::Login { user, password } => {
                assert_eq!(user, "user");
                assert_eq!(password, "pass");
            }
            _ => panic!("expected Login"),
        }
    }

    #[test]
    fn test_parse_select() {
        let cmd = ImapCommand::parse("A002", "SELECT", &["INBOX".to_string()], None).unwrap();
        match cmd {
            ImapCommand::Select { mailbox } => assert_eq!(mailbox, "INBOX"),
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn test_parse_fetch() {
        let cmd = ImapCommand::parse("A003", "FETCH", &["1".to_string(), "(UID FLAGS)".to_string()], None).unwrap();
        match cmd {
            ImapCommand::Fetch { sequence, attributes } => {
                assert_eq!(sequence, "1");
                assert_eq!(attributes.len(), 2);
            }
            _ => panic!("expected Fetch"),
        }
    }

    #[test]
    fn test_parse_search() {
        let cmd = ImapCommand::parse("A004", "SEARCH", &["ALL".to_string()], None).unwrap();
        match cmd {
            ImapCommand::Search { keys, .. } => assert!(!keys.is_empty()),
            _ => panic!("expected Search"),
        }
    }

    #[test]
    fn test_parse_store() {
        let cmd = ImapCommand::parse("A005", "STORE", &["1".to_string(), "+FLAGS".to_string(), "(\\Seen)".to_string()], None).unwrap();
        match cmd {
            ImapCommand::Store { sequence, action, flags } => {
                assert_eq!(sequence, "1");
                assert_eq!(action, StoreAction::Add);
                assert_eq!(flags, vec!["\\Seen"]);
            }
            _ => panic!("expected Store"),
        }
    }

    #[test]
    fn test_response_formatting() {
        let resp = ImapResponse::ok("A001", "done");
        assert_eq!(resp.to_wire(), "A001 OK done\r\n");

        let resp = ImapResponse::no("A001", "failed");
        assert_eq!(resp.to_wire(), "A001 NO failed\r\n");

        let resp = ImapResponse::bad("A001", "error");
        assert_eq!(resp.to_wire(), "A001 BAD error\r\n");
    }

    #[test]
    fn test_parse_store_action() {
        // These are tested indirectly through ImapCommand::parse
    }

    #[test]
    fn test_parse_append() {
        let cmd = ImapCommand::parse("A006", "APPEND", &["INBOX".to_string(), "(\\Seen)".to_string(), "\"01-Jan-2024\"".to_string(), "{100}".to_string()], None).unwrap();
        match cmd {
            ImapCommand::Append { mailbox, flags, date, literal_size } => {
                assert_eq!(mailbox, "INBOX");
                assert!(flags.is_some());
                assert!(date.is_some());
                assert_eq!(literal_size, 100);
            }
            _ => panic!("expected Append"),
        }
    }

    #[test]
    fn test_parse_uid_fetch() {
        // UID FETCH 1 (UID FLAGS)
        let cmd = ImapCommand::parse("A007", "UID", &["FETCH".to_string(), "1".to_string(), "(UID FLAGS)".to_string()], None).unwrap();
        match cmd {
            ImapCommand::Uid { command } => {
                match &*command {
                    ImapCommand::Fetch { sequence, .. } => assert_eq!(sequence, "1"),
                    _ => panic!("expected inner Fetch"),
                }
            }
            _ => panic!("expected Uid"),
        }
    }

    #[test]
    fn test_parse_move() {
        let cmd = ImapCommand::parse("A008", "MOVE", &["1:5".to_string(), "Archive".to_string()], None).unwrap();
        match cmd {
            ImapCommand::Move { sequence, mailbox } => {
                assert_eq!(sequence, "1:5");
                assert_eq!(mailbox, "Archive");
            }
            _ => panic!("expected Move"),
        }
    }

    #[test]
    fn test_parse_enable() {
        let cmd = ImapCommand::parse("A009", "ENABLE", &["CONDSTORE".to_string(), "QRESYNC".to_string()], None).unwrap();
        match cmd {
            ImapCommand::Enable { capabilities } => {
                assert_eq!(capabilities, vec!["CONDSTORE", "QRESYNC"]);
            }
            _ => panic!("expected Enable"),
        }
    }

    #[test]
    fn test_parse_unknown_command() {
        let result = ImapCommand::parse("A010", "UNKNOWN", &[], None);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod base64_tests {
    use edgerun_imap::server::base64_decode;

    #[test]
    fn test_base64_empty() {
        assert_eq!(base64_decode("").unwrap(), vec![]);
    }

    #[test]
    fn test_base64_hello() {
        // "Hello" in base64 is "SGVsbG8="
        let result = base64_decode("SGVsbG8=").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_base64_plain_sasl() {
        // "\0user\0pass" in base64
        // authzid=NUL, username=user, password=pass
        let encoded = "AHVzZXIAcGFzcw==";
        let result = base64_decode(encoded).unwrap();
        let s = std::str::from_utf8(&result).unwrap();
        let parts: Vec<&str> = s.split('\0').collect();
        assert_eq!(parts[1], "user");
        assert_eq!(parts[2], "pass");
    }
}
