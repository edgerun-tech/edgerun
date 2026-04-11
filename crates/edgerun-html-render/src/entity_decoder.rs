// DO NOT EDIT.
// Auto-generated from Parser IR by scripts/generate_html_parser.py
// Regenerate: python3 scripts/generate_parser_ir.py && python3 scripts/generate_html_parser.py

extern crate alloc;

use alloc::string::String;

/// Entity decoder — maps named character references to Unicode code points.
///
/// Generated from Parser IR with 6 entities.
/// Phase 1: minimal set (5 entities). Full spec has 2,231.
pub struct EntityDecoder;

impl EntityDecoder {
    /// Decode a named entity (without & or ;).
    /// Returns (primary_char, optional_secondary_char) or None.
    pub fn decode(name: &str) -> Option<(char, Option<char>)> {
        match name {
        "amp" => Some((char::from_u32(38).unwrap(), None)),
        "lt" => Some((char::from_u32(60).unwrap(), None)),
        "gt" => Some((char::from_u32(62).unwrap(), None)),
        "quot" => Some((char::from_u32(34).unwrap(), None)),
        "apos" => Some((char::from_u32(39).unwrap(), None)),
        "nbsp" => Some((char::from_u32(160).unwrap(), None)),
            _ => None,
        }
    }

    /// Decode a full entity reference including & and optionally ;.
    pub fn decode_full(input: &str) -> Option<(String, usize)> {
        if !input.starts_with('&') {
            return None;
        }
        // Try to find the entity name
        let rest = &input[1..];
        let semi_pos = rest.find(';');
        if let Some(pos) = semi_pos {
            let name = &rest[..pos];
            if let Some((c1, c2)) = Self::decode(name) {
                let s = if let Some(c2) = c2 {
                    format!("{}{}", c1, c2)
                } else {
                    format!("{}", c1)
                };
                return Some((s, pos + 2)); // +2 for & and ;
            }
        }
        // Try without semicolon (for legacy entities)
        for len in (1..=rest.len()).rev() {
            let name = &rest[..len];
            if let Some((c1, c2)) = Self::decode(name) {
                let s = if let Some(c2) = c2 {
                    format!("{}{}", c1, c2)
                } else {
                    format!("{}", c1)
                };
                return Some((s, len + 1)); // +1 for &
            }
        }
        None
    }
}
