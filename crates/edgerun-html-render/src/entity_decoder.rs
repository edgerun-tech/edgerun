// DO NOT EDIT.
// Auto-generated from Parser IR by cmd/html-codegen
// Regenerate: go run ./cmd/html-codegen
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// WHATWG §13.1.4.22 Named character references — sorted table for binary search.
///
/// Generated from proto IR with 201 entities.
/// Scales to 2,231 entities (Phase 4) with the same binary search algorithm.
const ENTITY_TABLE: &[(&str, u32, u32)] = &[
    ("AElig", 0x00C6, 0x0000),
    ("Aacute", 0x00C1, 0x0000),
    ("Acirc", 0x00C2, 0x0000),
    ("Agrave", 0x00C0, 0x0000),
    ("Alpha", 0x0391, 0x0000),
    ("Amp", 0x0026, 0x0000),
    ("Aring", 0x00C5, 0x0000),
    ("Atilde", 0x00C3, 0x0000),
    ("Auml", 0x00C4, 0x0000),
    ("Beta", 0x0392, 0x0000),
    ("Ccedil", 0x00C7, 0x0000),
    ("Delta", 0x0394, 0x0000),
    ("ETH", 0x00D0, 0x0000),
    ("Eacute", 0x00C9, 0x0000),
    ("Ecirc", 0x00CA, 0x0000),
    ("Egrave", 0x00C8, 0x0000),
    ("Euml", 0x00CB, 0x0000),
    ("Gamma", 0x0393, 0x0000),
    ("Iacute", 0x00CD, 0x0000),
    ("Icirc", 0x00CE, 0x0000),
    ("Igrave", 0x00CC, 0x0000),
    ("Iuml", 0x00CF, 0x0000),
    ("NewLine", 0x000A, 0x0000),
    ("Ntilde", 0x00D1, 0x0000),
    ("OElig", 0x0152, 0x0000),
    ("Oacute", 0x00D3, 0x0000),
    ("Ocirc", 0x00D4, 0x0000),
    ("Ograve", 0x00D2, 0x0000),
    ("Omega", 0x03A9, 0x0000),
    ("Otilde", 0x00D5, 0x0000),
    ("Ouml", 0x00D6, 0x0000),
    ("Prime", 0x2033, 0x0000),
    ("Scaron", 0x0160, 0x0000),
    ("THORN", 0x00DE, 0x0000),
    ("Tab", 0x0009, 0x0000),
    ("Uacute", 0x00DA, 0x0000),
    ("Ucirc", 0x00DB, 0x0000),
    ("Ugrave", 0x00D9, 0x0000),
    ("Uuml", 0x00DC, 0x0000),
    ("Yacute", 0x00DD, 0x0000),
    ("Yuml", 0x0178, 0x0000),
    ("aacute", 0x00E1, 0x0000),
    ("acirc", 0x00E2, 0x0000),
    ("acute", 0x00B4, 0x0000),
    ("aelig", 0x00E6, 0x0000),
    ("agrave", 0x00E0, 0x0000),
    ("alefsym", 0x2135, 0x0000),
    ("alpha", 0x03B1, 0x0000),
    ("amp", 0x0026, 0x0000),
    ("and", 0x2227, 0x0000),
    ("ang", 0x2220, 0x0000),
    ("apos", 0x0027, 0x0000),
    ("aring", 0x00E5, 0x0000),
    ("asymp", 0x2248, 0x0000),
    ("atilde", 0x00E3, 0x0000),
    ("auml", 0x00E4, 0x0000),
    ("beta", 0x03B2, 0x0000),
    ("brvbar", 0x00A6, 0x0000),
    ("bull", 0x2022, 0x0000),
    ("cap", 0x2229, 0x0000),
    ("ccedil", 0x00E7, 0x0000),
    ("cedil", 0x00B8, 0x0000),
    ("cent", 0x00A2, 0x0000),
    ("circ", 0x02C6, 0x0000),
    ("clubs", 0x2663, 0x0000),
    ("cong", 0x2245, 0x0000),
    ("copy", 0x00A9, 0x0000),
    ("crarr", 0x21B5, 0x0000),
    ("cup", 0x222A, 0x0000),
    ("curren", 0x00A4, 0x0000),
    ("dArr", 0x21D3, 0x0000),
    ("darr", 0x2193, 0x0000),
    ("deg", 0x00B0, 0x0000),
    ("delta", 0x03B4, 0x0000),
    ("diams", 0x2666, 0x0000),
    ("divide", 0x00F7, 0x0000),
    ("eacute", 0x00E9, 0x0000),
    ("ecirc", 0x00EA, 0x0000),
    ("egrave", 0x00E8, 0x0000),
    ("empty", 0x2205, 0x0000),
    ("emsp", 0x2003, 0x0000),
    ("ensp", 0x2002, 0x0000),
    ("equiv", 0x2261, 0x0000),
    ("eth", 0x00F0, 0x0000),
    ("euml", 0x00EB, 0x0000),
    ("euro", 0x20AC, 0x0000),
    ("exist", 0x2203, 0x0000),
    ("fnof", 0x0192, 0x0000),
    ("forall", 0x2200, 0x0000),
    ("frac12", 0x00BD, 0x0000),
    ("frac14", 0x00BC, 0x0000),
    ("frac34", 0x00BE, 0x0000),
    ("frasl", 0x2044, 0x0000),
    ("gamma", 0x03B3, 0x0000),
    ("ge", 0x2265, 0x0000),
    ("gt", 0x003E, 0x0000),
    ("hArr", 0x21D4, 0x0000),
    ("harr", 0x2194, 0x0000),
    ("hearts", 0x2665, 0x0000),
    ("hellip", 0x2026, 0x0000),
    ("iacute", 0x00ED, 0x0000),
    ("icirc", 0x00EE, 0x0000),
    ("iexcl", 0x00A1, 0x0000),
    ("igrave", 0x00EC, 0x0000),
    ("image", 0x2111, 0x0000),
    ("infin", 0x221E, 0x0000),
    ("int", 0x222B, 0x0000),
    ("iquest", 0x00BF, 0x0000),
    ("isin", 0x2208, 0x0000),
    ("iuml", 0x00EF, 0x0000),
    ("lArr", 0x21D0, 0x0000),
    ("lang", 0x2329, 0x0000),
    ("laquo", 0x00AB, 0x0000),
    ("larr", 0x2190, 0x0000),
    ("lceil", 0x2308, 0x0000),
    ("le", 0x2264, 0x0000),
    ("lfloor", 0x230A, 0x0000),
    ("lowast", 0x2217, 0x0000),
    ("loz", 0x25CA, 0x0000),
    ("lrm", 0x200E, 0x0000),
    ("lt", 0x003C, 0x0000),
    ("macr", 0x00AF, 0x0000),
    ("mdash", 0x2014, 0x0000),
    ("micro", 0x00B5, 0x0000),
    ("middot", 0x00B7, 0x0000),
    ("minus", 0x2212, 0x0000),
    ("nabla", 0x2207, 0x0000),
    ("nbsp", 0x00A0, 0x0000),
    ("ndash", 0x2013, 0x0000),
    ("ne", 0x2260, 0x0000),
    ("ni", 0x220B, 0x0000),
    ("not", 0x00AC, 0x0000),
    ("notin", 0x2209, 0x0000),
    ("nsub", 0x2284, 0x0000),
    ("ntilde", 0x00F1, 0x0000),
    ("oacute", 0x00F3, 0x0000),
    ("ocirc", 0x00F4, 0x0000),
    ("oelig", 0x0153, 0x0000),
    ("ograve", 0x00F2, 0x0000),
    ("oline", 0x203E, 0x0000),
    ("omega", 0x03C9, 0x0000),
    ("oplus", 0x2295, 0x0000),
    ("or", 0x2228, 0x0000),
    ("ordf", 0x00AA, 0x0000),
    ("ordm", 0x00BA, 0x0000),
    ("otilde", 0x00F5, 0x0000),
    ("otimes", 0x2297, 0x0000),
    ("ouml", 0x00F6, 0x0000),
    ("para", 0x00B6, 0x0000),
    ("part", 0x2202, 0x0000),
    ("perp", 0x22A5, 0x0000),
    ("plusmn", 0x00B1, 0x0000),
    ("pound", 0x00A3, 0x0000),
    ("prime", 0x2032, 0x0000),
    ("prod", 0x220F, 0x0000),
    ("prop", 0x221D, 0x0000),
    ("quot", 0x0022, 0x0000),
    ("rArr", 0x21D2, 0x0000),
    ("radic", 0x221A, 0x0000),
    ("rang", 0x232A, 0x0000),
    ("raquo", 0x00BB, 0x0000),
    ("rarr", 0x2192, 0x0000),
    ("rceil", 0x2309, 0x0000),
    ("real", 0x211C, 0x0000),
    ("reg", 0x00AE, 0x0000),
    ("rfloor", 0x230B, 0x0000),
    ("rlm", 0x200F, 0x0000),
    ("scaron", 0x0161, 0x0000),
    ("sdot", 0x22C5, 0x0000),
    ("sect", 0x00A7, 0x0000),
    ("shy", 0x00AD, 0x0000),
    ("sim", 0x223C, 0x0000),
    ("spades", 0x2660, 0x0000),
    ("sub", 0x2282, 0x0000),
    ("sube", 0x2286, 0x0000),
    ("sum", 0x2211, 0x0000),
    ("sup", 0x2283, 0x0000),
    ("sup1", 0x00B9, 0x0000),
    ("sup2", 0x00B2, 0x0000),
    ("sup3", 0x00B3, 0x0000),
    ("supe", 0x2287, 0x0000),
    ("szlig", 0x00DF, 0x0000),
    ("there4", 0x2234, 0x0000),
    ("thinsp", 0x2009, 0x0000),
    ("thorn", 0x00FE, 0x0000),
    ("tilde", 0x02DC, 0x0000),
    ("times", 0x00D7, 0x0000),
    ("trade", 0x2122, 0x0000),
    ("uArr", 0x21D1, 0x0000),
    ("uacute", 0x00FA, 0x0000),
    ("uarr", 0x2191, 0x0000),
    ("ucirc", 0x00FB, 0x0000),
    ("ugrave", 0x00F9, 0x0000),
    ("uml", 0x00A8, 0x0000),
    ("uuml", 0x00FC, 0x0000),
    ("weierp", 0x2118, 0x0000),
    ("yacute", 0x00FD, 0x0000),
    ("yen", 0x00A5, 0x0000),
    ("yuml", 0x00FF, 0x0000),
    ("zwj", 0x200D, 0x0000),
    ("zwnj", 0x200C, 0x0000),
];

/// Look up an entity by name (binary search).
fn lookup_entity(name: &str) -> Option<(u32, u32)> {
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
