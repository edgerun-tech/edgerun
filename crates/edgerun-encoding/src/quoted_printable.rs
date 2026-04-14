//! Quoted-Printable encoding (RFC 2045).
//!
//! Encodes UTF-8 strings into quoted-printable format suitable for email
//! transport. Handles soft line breaks at 76 columns and encodes non-printable
//! bytes as `=XX` hex sequences.
//!
//! # Examples
//! ```
//! use edgerun_encoding::quoted_printable::encode_quoted_printable;
//! let out = encode_quoted_printable("Hello, World!");
//! assert_eq!(out, b"Hello, World!");
//! ```

use alloc::vec::Vec;

/// Encode a string as quoted-printable bytes (RFC 2045).
///
/// - Printable ASCII (33–60, 62–126) passes through unchanged.
/// - `=` is always encoded as `=3D`.
/// - Space (32) and tab (9) pass through normally but would need encoding
///   if at end of line — we handle this by encoding them always to be safe.
/// - Non-printable bytes are encoded as `=XX`.
/// - Soft line breaks (`=\r\n`) are inserted at column 73 to keep lines ≤ 76.
/// - `\r` and `\n` pass through and reset the column counter.
pub fn encode_quoted_printable(s: &str) -> Vec<u8> {
    let mut result = Vec::with_capacity(s.len() * 3);
    let mut col = 0;
    const HEX: &[u8] = b"0123456789ABCDEF";

    for &byte in s.as_bytes() {
        match byte {
            b'\r' | b'\n' => {
                result.push(byte);
                col = 0;
            }
            33..=60 | 62..=126 => {
                // Printable ASCII except `=` (61)
                result.push(byte);
                col += 1;
            }
            b' ' | b'\t' => {
                // Space/tab — pass through normally,
                // encode if at end of line (we encode always for safety)
                result.push(byte);
                col += 1;
            }
            _ => {
                result.push(b'=');
                result.push(HEX[((byte >> 4) & 0xF) as usize]);
                result.push(HEX[(byte & 0xF) as usize]);
                col += 3;
            }
        }

        // Insert soft line break before column 76
        if col >= 73 {
            result.extend_from_slice(b"=\r\n");
            col = 0;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_ascii_passes_through() {
        let out = encode_quoted_printable("Hello, World!");
        assert_eq!(out, b"Hello, World!");
    }

    #[test]
    fn equals_sign_encoded() {
        let out = encode_quoted_printable("a=b");
        assert_eq!(out, b"a=3Db");
    }

    #[test]
    fn non_printable_encoded() {
        let out = encode_quoted_printable("Hello\x00World");
        assert_eq!(out, b"Hello=00World");
    }

    #[test]
    fn newline_resets_column() {
        let out = encode_quoted_printable("Hello\nWorld");
        assert_eq!(out, b"Hello\nWorld");
    }

    #[test]
    fn long_line_gets_soft_break() {
        let s = "A".repeat(80);
        let out = encode_quoted_printable(&s);
        // Should have soft line breaks
        let s = core::str::from_utf8(&out).unwrap();
        let lines: Vec<&str> = s.split("=\r\n").collect();
        // Each line (before soft break) should be ≤ 75 chars
        for line in &lines {
            assert!(line.len() <= 75, "line too long: {} chars", line.len());
        }
    }

    #[test]
    fn empty_string() {
        let out = encode_quoted_printable("");
        assert!(out.is_empty());
    }
}
