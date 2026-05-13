//! Small owned text-encoding surface used by EdgeRun shell-output decoding.
//!
//! This intentionally mirrors the tiny `chardetng` / `encoding_rs` API slice
//! Codex used, without depending on those crates. It is not a full browser
//! encoding database; add concrete decoders here as EdgeRun call sites need
//! them.

use alloc::borrow::Cow;
use alloc::string::String;

pub mod encoding_rs {
    use super::{Cow, String};

    #[derive(Debug, Eq, PartialEq)]
    pub struct Encoding {
        kind: EncodingKind,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum EncodingKind {
        Utf8Fallback,
        Windows1251,
        Windows1252,
        Ibm866,
    }

    pub static UTF_8_FALLBACK: &Encoding = &Encoding {
        kind: EncodingKind::Utf8Fallback,
    };
    pub static WINDOWS_1251: &Encoding = &Encoding {
        kind: EncodingKind::Windows1251,
    };
    pub static WINDOWS_1252: &Encoding = &Encoding {
        kind: EncodingKind::Windows1252,
    };
    pub static IBM866: &Encoding = &Encoding {
        kind: EncodingKind::Ibm866,
    };

    impl Encoding {
        pub fn decode<'a>(&'static self, bytes: &'a [u8]) -> (Cow<'a, str>, bool, bool) {
            match self.kind {
                EncodingKind::Utf8Fallback => match core::str::from_utf8(bytes) {
                    Ok(text) => (Cow::Borrowed(text), false, false),
                    Err(_) => (String::new().into(), false, true),
                },
                EncodingKind::Windows1251 => decode_single_byte(bytes, decode_windows_1251),
                EncodingKind::Windows1252 => decode_single_byte(bytes, decode_windows_1252),
                EncodingKind::Ibm866 => decode_single_byte(bytes, decode_ibm866),
            }
        }
    }

    fn decode_single_byte<'a>(
        bytes: &'a [u8],
        decode: fn(u8) -> Option<char>,
    ) -> (Cow<'a, str>, bool, bool) {
        let mut output = String::with_capacity(bytes.len());
        for &byte in bytes {
            match decode(byte) {
                Some(ch) => output.push(ch),
                None => return (String::new().into(), false, true),
            }
        }
        (output.into(), false, false)
    }

    fn decode_windows_1251(byte: u8) -> Option<char> {
        match byte {
            0x00..=0x7F => Some(byte as char),
            0xA8 => Some('\u{0401}'),
            0xB8 => Some('\u{0451}'),
            0xC0..=0xFF => char::from_u32(0x0410 + u32::from(byte - 0xC0)),
            _ => None,
        }
    }

    fn decode_ibm866(byte: u8) -> Option<char> {
        match byte {
            0x00..=0x7F => Some(byte as char),
            0x80..=0xAF => char::from_u32(0x0410 + u32::from(byte - 0x80)),
            0xE0..=0xEF => char::from_u32(0x0440 + u32::from(byte - 0xE0)),
            0xF0 => Some('\u{0401}'),
            0xF1 => Some('\u{0451}'),
            _ => None,
        }
    }

    fn decode_windows_1252(byte: u8) -> Option<char> {
        match byte {
            0x00..=0x7F | 0xA0..=0xFF => Some(byte as char),
            0x80 => Some('\u{20AC}'),
            0x82 => Some('\u{201A}'),
            0x83 => Some('\u{0192}'),
            0x84 => Some('\u{201E}'),
            0x85 => Some('\u{2026}'),
            0x86 => Some('\u{2020}'),
            0x87 => Some('\u{2021}'),
            0x88 => Some('\u{02C6}'),
            0x89 => Some('\u{2030}'),
            0x8A => Some('\u{0160}'),
            0x8B => Some('\u{2039}'),
            0x8C => Some('\u{0152}'),
            0x8E => Some('\u{017D}'),
            0x91 => Some('\u{2018}'),
            0x92 => Some('\u{2019}'),
            0x93 => Some('\u{201C}'),
            0x94 => Some('\u{201D}'),
            0x95 => Some('\u{2022}'),
            0x96 => Some('\u{2013}'),
            0x97 => Some('\u{2014}'),
            0x98 => Some('\u{02DC}'),
            0x99 => Some('\u{2122}'),
            0x9A => Some('\u{0161}'),
            0x9B => Some('\u{203A}'),
            0x9C => Some('\u{0153}'),
            0x9E => Some('\u{017E}'),
            0x9F => Some('\u{0178}'),
            _ => None,
        }
    }
}

pub mod chardetng {
    use super::encoding_rs::{Encoding, IBM866, UTF_8_FALLBACK, WINDOWS_1251, WINDOWS_1252};

    #[derive(Default)]
    pub struct EncodingDetector {
        saw_high: bool,
        saw_cp866_upper: bool,
        saw_windows_1252_punctuation: bool,
        saw_ascii_word: bool,
        invalid_probe: bool,
    }

    impl EncodingDetector {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn feed(&mut self, bytes: &[u8], _last: bool) {
            for &byte in bytes {
                if byte >= 0x80 {
                    self.saw_high = true;
                }
                if (0x80..=0xAF).contains(&byte) {
                    self.saw_cp866_upper = true;
                }
                if matches!(byte, 0x91 | 0x92 | 0x93 | 0x94 | 0x95 | 0x96 | 0x97 | 0x99) {
                    self.saw_windows_1252_punctuation = true;
                }
                if byte.is_ascii_alphabetic() {
                    self.saw_ascii_word = true;
                }
            }

            self.invalid_probe = matches!(bytes, [0xFF, 0xFE, ..] | [0xFE, 0xFF, ..]);
        }

        pub fn guess_assess(
            &self,
            _top_level_domain: Option<&[u8]>,
            _allow_utf8: bool,
        ) -> (&'static Encoding, bool) {
            if self.invalid_probe {
                return (UTF_8_FALLBACK, false);
            }
            if self.saw_windows_1252_punctuation && self.saw_ascii_word {
                return (WINDOWS_1252, true);
            }
            if self.saw_cp866_upper {
                return (IBM866, true);
            }
            if self.saw_high {
                return (WINDOWS_1251, true);
            }
            (UTF_8_FALLBACK, true)
        }
    }
}
