// DO NOT EDIT.
// Auto-generated from Parser IR by cmd/html-codegen
// Regenerate: go run ./cmd/html-codegen

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

pub use crate::tokenizer::{Tokenizer, Token, State, VOID_ELEMENTS, RAW_TEXT_ELEMENTS};
pub use crate::tree_builder::{TreeBuilder, InsertionMode, TokenizerMode};
pub use crate::entity_decoder::decode_entities_in_text;

/// Block-level elements — used by layout for block vs inline distinction.
pub const BLOCK_ELEMENTS: &[&str] = &[
    "address", "article", "aside", "blockquote", "caption", "col",
    "colgroup", "dd", "details", "dialog", "div", "dl", "dt",
    "figcaption", "figure", "fieldset", "footer", "form", "h1", "h2",
    "h3", "h4", "h5", "h6", "header", "hr", "legend", "li", "main",
    "nav", "ol", "p", "pre", "section", "summary", "table", "tbody",
    "td", "tfoot", "th", "thead", "tr", "ul",
];

/// Inline elements.
pub const INLINE_ELEMENTS: &[&str] = &[
    "a", "abbr", "audio", "b", "big", "br", "button", "canvas", "cite",
    "code", "data", "datalist", "del", "dfn", "em", "iframe", "img",
    "input", "ins", "kbd", "label", "link", "map", "mark", "meter",
    "object", "output", "picture", "progress", "q", "ruby", "samp",
    "script", "select", "slot", "small", "source", "span", "strong",
    "sub", "sup", "svg", "textarea", "time", "track", "u", "var",
    "video", "wbr",
];

/// DOM node types.
#[derive(Debug, Clone)]
pub enum Node {
    Element(Element),
    Text(String),
    Comment(String),
}

/// HTML element with tag, attributes, and children.
#[derive(Debug, Clone)]
pub struct Element {
    pub tag: String,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<Node>,
}

impl Element {
    pub fn new(tag: &str) -> Self {
        Self { tag: tag.to_lowercase(), attrs: BTreeMap::new(), children: Vec::new() }
    }
    pub fn is_void(&self) -> bool { VOID_ELEMENTS.contains(&self.tag.as_str()) }
    pub fn attr(&self, name: &str) -> Option<&str> { self.attrs.get(name).map(|s| s.as_str()) }
    pub fn class(&self) -> Option<&str> { self.attr("class") }
    pub fn id(&self) -> Option<&str> { self.attr("id") }
}

/// Parse an HTML string into a DOM tree.
///
/// Pipeline: &str → Tokenizer → TreeBuilder → DOM tree
///
/// Generated from proto IR — the tokenizer is table-driven from §13.2.5,
/// and the tree builder applies rule-based insertion modes from §13.2.6.
pub fn parse_html(html: &str) -> Node {
    let mut tokenizer = Tokenizer::new(html);
    let mut tree_builder = TreeBuilder::new();

    loop {
        match tokenizer.step() {
            Some(token) => {
                tree_builder.handle_token(&token);
                // Check if tree builder signaled a tokenizer mode switch
                // (e.g., entering <script>, <style>, <noscript>, <textarea>, <title>)
                if let Some(mode) = tree_builder.take_tokenizer_mode() {
                    // Extract the tag name from the token so the tokenizer knows
                    // which end tag to look for
                    if let Token::StartTag { name, .. } = &token {
                        let state = match mode {
                            TokenizerMode::Rawtext => State::Rawtext,
                            TokenizerMode::Rcdata => State::Rcdata,
                            TokenizerMode::ScriptData => State::ScriptData,
                            _ => continue,
                        };
                        tokenizer.set_raw_text_tag(name, state);
                        tokenizer.set_state(state);
                    }
                }
                if matches!(token, Token::Eof) { break; }
            }
            None => break,
        }
    }

    let mut roots = tree_builder.finish();
    for root in &mut roots { decode_entities_in_node(root); }

    if roots.len() == 1 {
        roots.remove(0)
    } else if roots.is_empty() {
        let mut root = Element::new("div");
        root.attrs.insert("data-root".into(), "true".into());
        Node::Element(root)
    } else {
        let mut root = Element::new("div");
        root.attrs.insert("data-root".into(), "true".into());
        root.children = roots;
        Node::Element(root)
    }
}

fn decode_entities_in_node(node: &mut Node) {
    match node {
        Node::Text(text) => { *text = decode_entities_in_text(text); }
        Node::Element(elem) => { for child in &mut elem.children { decode_entities_in_node(child); } }
        Node::Comment(_) => {}
    }
}

/// Count nodes in the DOM tree (for rendering).
pub fn count_nodes(node: &Node) -> usize {
    match node {
        Node::Text(_) => 1,
        Node::Comment(_) => 1,
        Node::Element(elem) => 1 + elem.children.iter().map(count_nodes).sum::<usize>()
    }
}
