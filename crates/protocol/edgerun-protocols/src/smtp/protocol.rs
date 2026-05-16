//! Shared SMTP protocol constants.

use alloc::vec::Vec;

/// Default ESMTP extensions advertised in EHLO.
///
/// NOTE: PIPELINING is NOT advertised because the server processes commands
/// synchronously (one at a time). Advertising PIPELINING when the server
/// cannot handle pipelined commands is a protocol violation (RFC 2931).
pub const ESMTP_EXTENSIONS: &[&str] = &[
    "SIZE 35882577",
    "8BITMIME",
    "ENHANCEDSTATUSCODES",
    "SMTPUTF8",
    "CHUNKING",
];

/// Format SMTP DATA body bytes with RFC 5321 dot-stuffing and terminator.
pub fn dot_stuffed_data(message: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(message.len() + 8);
    let mut start = 0;
    let mut i = 0;

    while i < message.len() {
        match message[i] {
            b'\r' => {
                push_dot_stuffed_line(&message[start..i], &mut out);
                i += 1;
                if i < message.len() && message[i] == b'\n' {
                    i += 1;
                }
                start = i;
            }
            b'\n' => {
                push_dot_stuffed_line(&message[start..i], &mut out);
                i += 1;
                start = i;
            }
            _ => i += 1,
        }
    }

    push_dot_stuffed_line(&message[start..], &mut out);
    out.extend_from_slice(b".\r\n");
    out
}

fn push_dot_stuffed_line(line: &[u8], out: &mut Vec<u8>) {
    if line.first() == Some(&b'.') {
        out.push(b'.');
    }
    out.extend_from_slice(line);
    out.extend_from_slice(b"\r\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_stuffs_leading_dots() {
        assert_eq!(
            dot_stuffed_data(b"hello\r\n.world"),
            b"hello\r\n..world\r\n.\r\n".to_vec()
        );
    }

    #[test]
    fn normalizes_line_endings() {
        assert_eq!(
            dot_stuffed_data(b"a\rb\nc"),
            b"a\r\nb\r\nc\r\n.\r\n".to_vec()
        );
    }
}
