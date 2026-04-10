//! CSS Syntax types — from CSS Syntax Level 3.
//! DO NOT EDIT. Regenerate with: scripts/generate_batch4.py
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{string::String, vec::Vec};

#[derive(Debug, Clone, PartialEq)]
pub enum CssToken {
    /// Identifier (e.g., font-family)
    Ident(String),
    /// Function name (e.g., rgb(
    Function(String),
    /// @-keyword (e.g., @media
    AtKeyword(String),
    /// Hash/id (e.g., #id
    Hash(String),
    /// Quoted string
    String(String),
    /// Unterminated string
    BadString,
    /// URL token
    Url(String),
    /// Unterminated URL
    BadUrl,
    /// Delimiter (single char)
    Delim(String),
    /// Number value
    Number(String),
    /// Percentage value
    Percentage(String),
    /// Number with unit (e.g., 12px
    Dimension(String),
    /// Unicode range (e.g., U+0025-00FF
    UnicodeRange(String),
    /// Colon :
    Colon,
    /// Semicolon ;
    Semicolon,
    /// Comma ,
    Comma,
    /// Open parenthesis (
    OpenParen,
    /// Close parenthesis )
    CloseParen,
    /// Open square bracket [
    OpenSquare,
    /// Close square bracket ]
    CloseSquare,
    /// Open curly brace {
    OpenCurly,
    /// Close curly brace }
    CloseCurly,
    /// <!-- comment open
    Cdo,
    /// --> comment close
    Cdc,
    /// Whitespace
    WhiteSpace(String),
    /// End of file
    Eof,
}

/// CSS Tokenizer state machine states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenizerState {
    Data, Comment, String, StringStartingEscape,
    Url, UrlBad, Number, NumberStartingDash, NumberStartingDot,
    Ident, AtKeyword, Hash, HashAlphanumeric,
    Cdo, Cdc, HtmlCommentOpen, HtmlCommentClose,
    Percentage, Dimension, UnicodeRange, Eof,
}

/// Parsed unicode range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnicodeRange { pub start: u32, pub end: u32 }

impl UnicodeRange {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim_start_matches("U+").trim_start_matches("u+");
        if s.contains("?") {
            let prefix = s.trim_end_matches("?");
            let start = u32::from_str_radix(prefix, 16).ok()?;
            let end = start | ((1 << (4 * (s.len() - prefix.len()))) - 1);
            Some(Self { start, end })
        } else if let Some((a, b)) = s.split_once("-") {
            let start = u32::from_str_radix(a, 16).ok()?;
            let end = u32::from_str_radix(b, 16).ok()?;
            Some(Self { start, end })
        } else {
            let cp = u32::from_str_radix(s, 16).ok()?;
            Some(Self { start: cp, end: cp })
        }
    }
}