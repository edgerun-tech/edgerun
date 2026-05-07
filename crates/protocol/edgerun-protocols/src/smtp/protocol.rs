//! Shared SMTP protocol constants.

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
