//! Attribute selector operators — from Selectors Level 4.
//! DO NOT EDIT. Regenerate with: scripts/generate_selector_dom.py
extern crate alloc;
use alloc::{string::String, vec::Vec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrMatchOp {
    ///  — [attr] — element has attribute
    Exists,
    /// = — [attr="val"] — exact match
    Equals,
    /// ~= — [attr~="val"] — whitespace-separated word
    ContainsWord,
    /// |= — [attr|="val"] — equals or prefix with hyphen
    PrefixHyphen,
    /// ^= — [attr^="val"] — starts with
    Prefix,
    /// $= — [attr$="val"] — ends with
    Suffix,
    /// *= — [attr*="val"] — contains substring
    Contains,
}

/// Modifier for case sensitivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaseSensitivity {
    #[default]
    Default,
    /// i — Case-insensitive match
    CaseInsensitive,
    /// s — Case-sensitive match
    CaseSensitive,
}

/// An attribute selector: [attr], [attr="val"], [attr^="val"], etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttrSelector {
    pub name: String,
    pub op: Option<AttrMatchOp>,
    pub value: Option<String>,
    pub case: CaseSensitivity,
}