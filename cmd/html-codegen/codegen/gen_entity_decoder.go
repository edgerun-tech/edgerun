package htmlcodegen

import (
	"fmt"
	"sort"
	"strings"

	"edgerun-reference-core/gen/go/edgerun/v0/html"
)

// GenerateEntityDecoderRust produces the entity_decoder.rs source file.
func GenerateEntityDecoderRust(catalog *html.EntityCatalog) string {
	type entity struct {
		name    string
		cp1     uint32
		cp2     uint32
		semiReq bool
	}

	var entities []entity
	for _, e := range catalog.Entities {
		entities = append(entities, entity{
			name:    e.Name,
			cp1:     e.CodePoint_1,
			cp2:     e.CodePoint_2,
			semiReq: e.SemicolonRequired,
		})
	}

	// Sort by name for binary search
	sort.Slice(entities, func(i, j int) bool {
		return entities[i].name < entities[j].name
	})

	// Generate entity table entries
	var entries []string
	for _, e := range entities {
		entries = append(entries,
			fmt.Sprintf(`    ("%s", 0x%04X, 0x%04X),`, e.name, e.cp1, e.cp2))
	}
	entityTable := strings.Join(entries, "\n")

	total := len(entities)

	return fmt.Sprintf(`// DO NOT EDIT.
// Auto-generated from Parser IR by cmd/html-codegen
// Regenerate: go run ./cmd/html-codegen
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// WHATWG §13.1.4.22 Named character references — sorted table for binary search.
///
/// Generated from proto IR with %d entities.
/// Scales to 2,231 entities (Phase 4) with the same binary search algorithm.
const ENTITY_TABLE: &[(&str, u32, u32)] = &[
%s
];

/// Look up an entity by name (binary search).
#[allow(dead_code)]
pub fn lookup_entity(name: &str) -> Option<(u32, u32)> {
    let idx = ENTITY_TABLE.binary_search_by_key(&name, |e| e.0).ok()?;
    Some((ENTITY_TABLE[idx].1, ENTITY_TABLE[idx].2))
}

/// Decode entity references in a text string.
///
/// Handles both semicolon-terminated and legacy (no-semicolon) entities.
/// Unknown entities are passed through unchanged.
pub fn decode_entities_in_text(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;
    while i < len {
        if chars[i] == '&' {
            let mut matched: Option<(usize, u32, u32)> = None;
            let max_entity_len = core::cmp::min(33, len - i - 1);
            for entity_len in (1..=max_entity_len).rev() {
                if i + 1 + entity_len > len { continue; }
                let entity_name: String = chars[i + 1..i + 1 + entity_len].iter().collect();
                let has_semicolon = i + 1 + entity_len < len && chars[i + 1 + entity_len] == ';';
                if has_semicolon {
                    if let Some((cp1, cp2)) = lookup_entity(&entity_name) {
                        matched = Some((1 + entity_len + 1, cp1, cp2)); break;
                    }
                }
                if let Some((cp1, cp2)) = lookup_entity(&entity_name) {
                    let next_char_ok = if i + 1 + entity_len < len {
                        let next = chars[i + 1 + entity_len];
                        !next.is_ascii_alphanumeric() && next != '='
                    } else { true };
                    if next_char_ok { matched = Some((1 + entity_len, cp1, cp2)); break; }
                }
            }
            if let Some((consumed, cp1, cp2)) = matched {
                result.push(char::from_u32(cp1).unwrap_or('\u{FFFD}'));
                if cp2 != 0 { result.push(char::from_u32(cp2).unwrap_or('\u{FFFD}')); }
                i += consumed;
            } else { result.push('&'); i += 1; }
        } else { result.push(chars[i]); i += 1; }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_decode_amp() {
        assert_eq!(decode_entities_in_text("&amp;"), "&");
    }
    #[test]
    fn test_decode_lt() {
        assert_eq!(decode_entities_in_text("&lt;"), "<");
    }
    #[test]
    fn test_decode_gt() {
        assert_eq!(decode_entities_in_text("&gt;"), ">");
    }
    #[test]
    fn test_decode_quot() {
        assert_eq!(decode_entities_in_text("&quot;"), "\"");
    }
    #[test]
    fn test_decode_nbsp() {
        assert_eq!(decode_entities_in_text("&nbsp;"), "\u{00A0}");
    }
    #[test]
    fn test_decode_multiple() {
        assert_eq!(decode_entities_in_text("a &amp; b &lt; c"), "a & b < c");
    }
    #[test]
    fn test_unknown_passthrough() {
        assert_eq!(decode_entities_in_text("&unknown;"), "&unknown;");
    }
    #[test]
    fn test_no_entity() {
        assert_eq!(decode_entities_in_text("Hello"), "Hello");
    }
}
`, total, entityTable)
}
