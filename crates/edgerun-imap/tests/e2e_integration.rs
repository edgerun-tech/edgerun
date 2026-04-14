//! End-to-end integration tests: real TCP server + raw IMAP protocol

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use edgerun_imap::server::{ImapServer, ImapServerConfig, MemoryStore};
use edgerun_rt::CancellationToken;

/// Test helper: connect to a TCP port and read/write raw IMAP
struct ImapConnection {
    stream: TcpStream,
}

impl ImapConnection {
    fn new(addr: &str) -> Self {
        let stream = TcpStream::connect(addr).expect("Failed to connect to IMAP server");
        stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(10))).unwrap();
        Self { stream }
    }

    /// Read a line from the server (strips \r\n)
    fn read_line(&mut self) -> String {
        let mut buf = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            match self.stream.read(&mut byte) {
                Ok(0) => break, // EOF
                Ok(n) if n > 0 && byte[0] == b'\n' => break,
                Ok(n) if n > 0 => buf.push(byte[0]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => panic!("Failed to read: {}", e),
                _ => {}
            }
        }
        String::from_utf8_lossy(&buf).trim_end_matches('\r').to_string()
    }

    /// Send a raw command to the server
    fn send(&mut self, cmd: &str) {
        self.stream.write_all(format!("{}\r\n", cmd).as_bytes()).expect("Failed to send");
        self.stream.flush().expect("Failed to flush");
    }

    /// Read the greeting (first line from server)
    fn read_greeting(&mut self) -> String {
        self.read_line()
    }

    /// Send a command and read the tagged response
    fn cmd(&mut self, tag: &str, command: &str) -> String {
        self.send(&format!("{} {}", tag, command));
        let mut response = String::new();
        loop {
            let line = self.read_line();
            if !response.is_empty() {
                response.push('\n');
            }
            response.push_str(&line);
            if line.starts_with(tag) {
                break;
            }
        }
        response
    }

    /// Read a continuation response
    fn read_continuation(&mut self) -> String {
        self.read_line()
    }
}

/// Start an IMAP server on a random port and return (port, shutdown_token)
fn start_test_server() -> (u16, CancellationToken) {
    let store = Arc::new(MemoryStore::new());
    let config = ImapServerConfig {
        bind_addr: "127.0.0.1:0".to_string(),
        domain_name: "test.local".to_string(),
        imaps: false,
    };
    let server = ImapServer::with_store(config, store).unwrap();
    let port = server.local_addr().unwrap().port();

    let shutdown = CancellationToken::new();
    let shutdown_clone = shutdown.clone();
    std::thread::spawn(move || {
        let rt = edgerun_rt::Runtime::new_multi_thread().build().unwrap();
        rt.block_on(async move {
            let _ = server.run(shutdown_clone).await;
        });
    });

    // Wait a fixed time for server to start (the listener is bound before run() returns)
    std::thread::sleep(Duration::from_millis(500));

    (port, shutdown)
}

// ===========================================================================
// Tests
// ===========================================================================

#[test]
fn test_e2e_greeting_and_capability() {
    let (port, shutdown) = start_test_server();
    let _cleanup = DropGuard(shutdown);

    let mut conn = ImapConnection::new(&format!("127.0.0.1:{}", port));

    let greeting = conn.read_greeting();
    assert!(greeting.starts_with("* "));
    assert!(greeting.contains("IMAP4rev1"));
    assert!(greeting.contains("OK"));

    let resp = conn.cmd("A001", "CAPABILITY");
    assert!(resp.contains("* CAPABILITY"));
    assert!(resp.contains("IMAP4rev1"));
    assert!(resp.contains("A001 OK"));
}

#[test]
fn test_e2e_login_success_and_failure() {
    let (port, shutdown) = start_test_server();
    let _cleanup = DropGuard(shutdown);

    let mut conn = ImapConnection::new(&format!("127.0.0.1:{}", port));
    conn.read_greeting();

    let resp = conn.cmd("A001", "LOGIN user pass");
    assert!(resp.contains("A001 OK"));
    assert!(resp.contains("LOGIN completed"));

    let resp = conn.cmd("A002", "LOGIN user pass");
    assert!(resp.contains("A002 NO"));

    let mut conn2 = ImapConnection::new(&format!("127.0.0.1:{}", port));
    conn2.read_greeting();
    let resp = conn2.cmd("B001", "LOGIN user wrongpass");
    assert!(resp.contains("B001 NO"));
    assert!(resp.contains("LOGIN failed"));
}

#[test]
fn test_e2e_full_message_flow() {
    let (port, shutdown) = start_test_server();
    let _cleanup = DropGuard(shutdown);

    let mut conn = ImapConnection::new(&format!("127.0.0.1:{}", port));
    conn.read_greeting();

    conn.cmd("A001", "LOGIN user pass");
    conn.cmd("A002", "CREATE TestFolder");
    conn.cmd("A003", "LIST \"\" \"*\"");
    conn.cmd("A004", "SELECT INBOX");

    // Append a message
    let msg = "From: alice@test.com\r\nSubject: Hello World\r\n\r\nThis is the body text";
    conn.send(&format!("A005 APPEND INBOX {{{}}}", msg.len()));
    conn.read_continuation();
    conn.send(msg);
    conn.cmd("A006", "NOOP");

    // Verify 1 message
    let resp = conn.cmd("A007", "STATUS INBOX (MESSAGES)");
    assert!(resp.contains("MESSAGES 1"));

    // Fetch
    let resp = conn.cmd("A008", "FETCH 1 (UID FLAGS RFC822.SIZE)");
    assert!(resp.contains("* 1 FETCH"));
    assert!(resp.contains("UID"));
    assert!(resp.contains("FLAGS"));
    assert!(resp.contains("RFC822.SIZE"));

    // Search
    let resp = conn.cmd("A009", "SEARCH ALL");
    assert!(resp.contains("* SEARCH 1"));

    let resp = conn.cmd("A010", "SEARCH SUBJECT \"Hello\"");
    assert!(resp.contains("* SEARCH 1"));

    // Mark as seen
    conn.cmd("A011", "STORE 1 +FLAGS (\\Seen)");

    let resp = conn.cmd("A012", "FETCH 1 (FLAGS)");
    assert!(resp.contains("\\Seen"));

    // Search unseen
    let resp = conn.cmd("A013", "SEARCH UNSEEN");
    let search_line = resp.lines().find(|l| l.starts_with("* SEARCH")).unwrap_or("");
    assert!(!search_line.contains(" 1") || search_line == "* SEARCH");

    let resp = conn.cmd("A014", "LOGOUT");
    assert!(resp.contains("* BYE"));
    assert!(resp.contains("A014 OK"));
}

#[test]
fn test_e2e_copy_messages() {
    let (port, shutdown) = start_test_server();
    let _cleanup = DropGuard(shutdown);

    let mut conn = ImapConnection::new(&format!("127.0.0.1:{}", port));
    conn.read_greeting();
    conn.cmd("A001", "LOGIN user pass");
    conn.cmd("A002", "CREATE Archive");
    conn.cmd("A003", "SELECT INBOX");

    let msg = "From: test@test.com\r\nSubject: Copy Me\r\n\r\nBody";
    conn.send(&format!("A004 APPEND INBOX {{{}}}", msg.len()));
    conn.read_continuation();
    conn.send(msg);
    conn.cmd("A005", "NOOP");

    let resp = conn.cmd("A006", "COPY 1 Archive");
    assert!(resp.contains("A006 OK"));

    let resp = conn.cmd("A007", "STATUS Archive (MESSAGES)");
    assert!(resp.contains("MESSAGES 1"));
}

#[test]
fn test_e2e_expunge() {
    let (port, shutdown) = start_test_server();
    let _cleanup = DropGuard(shutdown);

    let mut conn = ImapConnection::new(&format!("127.0.0.1:{}", port));
    conn.read_greeting();
    conn.cmd("A001", "LOGIN user pass");
    conn.cmd("A002", "SELECT INBOX");

    for i in 1..=3 {
        let msg = format!("From: a@b.com\r\nSubject: Msg {}\r\n\r\nBody {}", i, i);
        conn.send(&format!("A{:02} APPEND INBOX {{{}}}", i * 10, msg.len()));
        conn.read_continuation();
        conn.send(&msg);
        conn.cmd(&format!("A{:02}", i * 10 + 1), "NOOP");
    }

    let resp = conn.cmd("A040", "STATUS INBOX (MESSAGES)");
    assert!(resp.contains("MESSAGES 3"));

    conn.cmd("A041", "STORE 2 +FLAGS (\\Deleted)");

    let resp = conn.cmd("A042", "EXPUNGE");
    assert!(resp.contains("* 2 EXPUNGE"));
    assert!(resp.contains("A042 OK"));

    let resp = conn.cmd("A043", "STATUS INBOX (MESSAGES)");
    assert!(resp.contains("MESSAGES 2"));
}

#[test]
fn test_e2e_search_flags() {
    let (port, shutdown) = start_test_server();
    let _cleanup = DropGuard(shutdown);

    let mut conn = ImapConnection::new(&format!("127.0.0.1:{}", port));
    conn.read_greeting();
    conn.cmd("A001", "LOGIN user pass");
    conn.cmd("A002", "SELECT INBOX");

    for i in 1..=3 {
        let msg = format!("From: user{}@test.com\r\nSubject: Test {}\r\n\r\nContent {}", i, i, i);
        conn.send(&format!("A{:02} APPEND INBOX {{{}}}", (i + 1) * 10, msg.len()));
        conn.read_continuation();
        conn.send(&msg);
        conn.cmd(&format!("A{:02}", (i + 1) * 10 + 1), "NOOP");
    }

    conn.cmd("A040", "STORE 1 +FLAGS (\\Seen)");
    conn.cmd("A041", "STORE 3 +FLAGS (\\Seen)");

    let resp = conn.cmd("A042", "SEARCH SEEN");
    let search_line = resp.lines().find(|l| l.starts_with("* SEARCH")).unwrap_or("");
    assert!(search_line.contains("1"));
    assert!(search_line.contains("3"));
    assert!(!search_line.contains("2"));

    let resp = conn.cmd("A043", "SEARCH UNSEEN");
    let search_line = resp.lines().find(|l| l.starts_with("* SEARCH")).unwrap_or("");
    assert!(search_line.contains("2"));
}

#[test]
fn test_e2e_search_subject_and_from() {
    let (port, shutdown) = start_test_server();
    let _cleanup = DropGuard(shutdown);

    let mut conn = ImapConnection::new(&format!("127.0.0.1:{}", port));
    conn.read_greeting();
    conn.cmd("A001", "LOGIN user pass");
    conn.cmd("A002", "SELECT INBOX");

    let msgs = vec![
        ("alice@example.com", "Meeting Tomorrow", "Let's meet"),
        ("bob@example.com", "Lunch plans", "Want lunch?"),
        ("charlie@test.com", "Meeting Notes", "Notes here"),
    ];

    for (i, (from, subject, body)) in msgs.iter().enumerate() {
        let msg = format!("From: {}\r\nSubject: {}\r\n\r\n{}", from, subject, body);
        conn.send(&format!("A{:02} APPEND INBOX {{{}}}", (i + 1) * 10, msg.len()));
        conn.read_continuation();
        conn.send(&msg);
        conn.cmd(&format!("A{:02}", (i + 1) * 10 + 1), "NOOP");
    }

    let resp = conn.cmd("A040", "SEARCH SUBJECT \"Meeting\"");
    let search_line = resp.lines().find(|l| l.starts_with("* SEARCH")).unwrap_or("");
    assert!(search_line.contains("1"));
    assert!(search_line.contains("3"));
    assert!(!search_line.contains("2"));

    let resp = conn.cmd("A041", "SEARCH FROM \"bob\"");
    let search_line = resp.lines().find(|l| l.starts_with("* SEARCH")).unwrap_or("");
    assert!(search_line.contains("2"));
    assert!(!search_line.contains("1"));
    assert!(!search_line.contains("3"));
}

#[test]
fn test_e2e_create_delete_rename() {
    let (port, shutdown) = start_test_server();
    let _cleanup = DropGuard(shutdown);

    let mut conn = ImapConnection::new(&format!("127.0.0.1:{}", port));
    conn.read_greeting();
    conn.cmd("A001", "LOGIN user pass");

    let resp = conn.cmd("A002", "CREATE MyFolder");
    assert!(resp.contains("A002 OK"));

    let resp = conn.cmd("A003", "LIST \"\" MyFolder");
    assert!(resp.contains("* LIST"));
    assert!(resp.contains("MyFolder"));

    let resp = conn.cmd("A004", "RENAME MyFolder YourFolder");
    assert!(resp.contains("A004 OK"));

    let resp = conn.cmd("A005", "LIST \"\" YourFolder");
    assert!(resp.contains("* LIST"));
    assert!(resp.contains("YourFolder"));

    let resp = conn.cmd("A006", "LIST \"\" MyFolder");
    assert!(!resp.contains("* LIST"));

    let resp = conn.cmd("A007", "DELETE YourFolder");
    assert!(resp.contains("A007 OK"));

    let resp = conn.cmd("A008", "LIST \"\" YourFolder");
    assert!(!resp.contains("* LIST"));
}

#[test]
fn test_e2e_fetch_body_sections() {
    let (port, shutdown) = start_test_server();
    let _cleanup = DropGuard(shutdown);

    let mut conn = ImapConnection::new(&format!("127.0.0.1:{}", port));
    conn.read_greeting();
    conn.cmd("A001", "LOGIN user pass");
    conn.cmd("A002", "SELECT INBOX");

    let msg = "From: sender@test.com\r\nSubject: Body Test\r\n\r\nThis is the message body text";
    conn.send(&format!("A003 APPEND INBOX {{{}}}", msg.len()));
    conn.read_continuation();
    conn.send(msg);
    conn.cmd("A004", "NOOP");

    let resp = conn.cmd("A005", "FETCH 1 (BODY[HEADER])");
    eprintln!("FETCH BODY[HEADER]: {:?}", resp);
    assert!(resp.contains("BODY[HEADER]"));
    assert!(resp.contains("From: sender@test.com"));
    assert!(resp.contains("Subject: Body Test"));

    let resp = conn.cmd("A006", "FETCH 1 (BODY[TEXT])");
    assert!(resp.contains("BODY[TEXT]"));
    assert!(resp.contains("This is the message body text"));

    let resp = conn.cmd("A007", "FETCH 1 (ENVELOPE)");
    assert!(resp.contains("ENVELOPE"));
    assert!(resp.contains("Body Test"));
}

#[test]
fn test_e2e_uid_operations() {
    let (port, shutdown) = start_test_server();
    let _cleanup = DropGuard(shutdown);

    let mut conn = ImapConnection::new(&format!("127.0.0.1:{}", port));
    conn.read_greeting();
    conn.cmd("A001", "LOGIN user pass");
    conn.cmd("A002", "SELECT INBOX");

    for i in 1..=3 {
        let msg = format!("From: a@b.com\r\nSubject: UID Test {}\r\n\r\nBody {}", i, i);
        conn.send(&format!("A{:02} APPEND INBOX {{{}}}", i * 10, msg.len()));
        conn.read_continuation();
        conn.send(&msg);
        conn.cmd(&format!("A{:02}", i * 10 + 1), "NOOP");
    }

    let resp = conn.cmd("A040", "UID FETCH 1 (UID FLAGS)");
    assert!(resp.contains("A040 OK"));
    let fetch_line = resp.lines().find(|l| l.contains("FETCH")).unwrap_or("");
    assert!(fetch_line.contains("UID 1"));

    conn.cmd("A041", "UID SEARCH 1:2");

    let resp = conn.cmd("A042", "CREATE Dest");
    let resp = conn.cmd("A043", "UID COPY 1 Dest");
    assert!(resp.contains("A043 OK"));

    let resp = conn.cmd("A044", "STATUS Dest (MESSAGES)");
    assert!(resp.contains("MESSAGES 1"));
}

#[test]
fn test_e2e_error_handling() {
    let (port, shutdown) = start_test_server();
    let _cleanup = DropGuard(shutdown);

    let mut conn = ImapConnection::new(&format!("127.0.0.1:{}", port));
    conn.read_greeting();

    let resp = conn.cmd("A001", "FOOBAR");
    assert!(resp.contains("A001 BAD") || resp.contains("A001 NO"));

    let resp = conn.cmd("A002", "SELECT INBOX");
    assert!(resp.contains("A002 NO"));

    conn.cmd("A003", "LOGIN user pass");
    let resp = conn.cmd("A004", "SELECT NonExistent");
    assert!(resp.contains("A004 NO"));

    let resp = conn.cmd("A005", "FETCH 1 (UID)");
    assert!(resp.contains("A005 NO"));

    conn.cmd("A006", "SELECT INBOX");
    let resp = conn.cmd("A007", "FETCH 999 (UID)");
    assert!(resp.contains("A007 OK"));
}

#[test]
fn test_e2e_close_and_unselect() {
    let (port, shutdown) = start_test_server();
    let _cleanup = DropGuard(shutdown);

    let mut conn = ImapConnection::new(&format!("127.0.0.1:{}", port));
    conn.read_greeting();
    conn.cmd("A001", "LOGIN user pass");
    conn.cmd("A002", "SELECT INBOX");

    let msg = "From: a@b.com\r\nSubject: Delete Me\r\n\r\nBye";
    conn.send(&format!("A003 APPEND INBOX {{{}}}", msg.len()));
    conn.read_continuation();
    conn.send(msg);
    conn.cmd("A004", "NOOP");

    conn.cmd("A005", "STORE 1 +FLAGS (\\Deleted)");
    let resp = conn.cmd("A006", "CLOSE");
    assert!(resp.contains("A006 OK"));

    let resp = conn.cmd("A007", "STATUS INBOX (MESSAGES)");
    assert!(resp.contains("MESSAGES 0"));

    // CLOSE transitions to Authenticated state, so we need SELECT again before UNSELECT
    conn.cmd("A007b", "SELECT INBOX");
    let resp = conn.cmd("A008", "UNSELECT");
    assert!(resp.contains("A008 OK"));

    let resp = conn.cmd("A009", "FETCH 1 (UID)");
    assert!(resp.contains("A009 NO"));
}

/// Helper to ensure shutdown on test exit
struct DropGuard(CancellationToken);
impl Drop for DropGuard {
    fn drop(&mut self) {
        self.0.cancel();
        std::thread::sleep(Duration::from_millis(100));
    }
}
