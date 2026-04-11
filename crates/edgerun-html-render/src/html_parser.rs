// DO NOT EDIT.
// Auto-generated from Parser IR by scripts/generate_html_parser.py
// Regenerate: python3 scripts/generate_parser_ir.py && python3 scripts/generate_html_parser.py

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

pub use crate::tokenizer::{Tokenizer, Token, VOID_ELEMENTS, RAW_TEXT_ELEMENTS};
pub use crate::tree_builder::TreeBuilder;

// Re-export for downstream consumers (edgerun-demo, layout_builder, etc.)
pub use crate::entity_decoder::EntityDecoder;

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
        Self {
            tag: tag.to_lowercase(),
            attrs: BTreeMap::new(),
            children: Vec::new(),
        }
    }

    pub fn is_void(&self) -> bool {
        VOID_ELEMENTS.contains(&self.tag.as_str())
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(|s| s.as_str())
    }

    pub fn class(&self) -> Option<&str> {
        self.attr("class")
    }

    pub fn id(&self) -> Option<&str> {
        self.attr("id")
    }
}

/// Parse an HTML string into a DOM tree.
///
/// Pipeline: &str → Tokenizer → token stream → DOM tree
///
/// Generated from Parser IR — replaces the previous ad-hoc recursive descent parser.
pub fn parse_html(html: &str) -> Node {
    let mut tokenizer = Tokenizer::new(html);
    let tokens = tokenizer.tokenize();

    // Simple stack-based DOM builder from token stream.
    #[derive(Debug)]
    struct Frame {
        elem: Element,
        children: Vec<Node>,
    }

    let mut stack: Vec<Frame> = Vec::new();
    let mut roots: Vec<Node> = Vec::new();

    for token in tokens {
        match token {
            Token::StartTag { name, attrs, self_closing: _ } => {
                let mut elem = Element::new(&name);
                for (k, v) in attrs {
                    elem.attrs.insert(k, v);
                }
                stack.push(Frame { elem, children: Vec::new() });
            }
            Token::EndTag { name } => {
                // Pop frames until we find the matching tag
                let mut found = None;
                while let Some(frame) = stack.pop() {
                    if frame.elem.tag == name {
                        let mut elem = frame.elem;
                        elem.children = frame.children;
                        found = Some(Node::Element(elem));
                        break;
                    } else {
                        // Implicit close
                        let mut elem = frame.elem;
                        elem.children = frame.children;
                        let node = Node::Element(elem);
                        if let Some(parent) = stack.last_mut() {
                            parent.children.push(node);
                        } else {
                            roots.push(node);
                        }
                    }
                }
                if let Some(node) = found {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else {
                        roots.push(node);
                    }
                }
            }
            Token::Character(text) => {
                if !text.trim().is_empty() {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(Node::Text(text));
                    } else {
                        roots.push(Node::Text(text));
                    }
                }
            }
            Token::Comment(text) => {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node::Comment(text));
                } else {
                    roots.push(Node::Comment(text));
                }
            }
            Token::Eof => {
                // Close all open elements
                while let Some(frame) = stack.pop() {
                    let mut elem = frame.elem;
                    elem.children = frame.children;
                    let node = Node::Element(elem);
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else {
                        roots.push(node);
                    }
                }
            }
        }
    }

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

/// Count nodes in the DOM tree (for rendering).
pub fn count_nodes(node: &Node) -> usize {
    match node {
        Node::Text(_) => 1,
        Node::Comment(_) => 1,
        Node::Element(elem) => {
            1 + elem.children.iter().map(count_nodes).sum::<usize>()
        }
    }
}
