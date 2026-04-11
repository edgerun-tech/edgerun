// DO NOT EDIT.
// Auto-generated from Parser IR by scripts/generate_html_parser.py
// Regenerate: python3 scripts/generate_parser_ir.py && python3 scripts/generate_html_parser.py

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

use crate::html_parser::{Node, Element};
use crate::tokenizer::Token;

/// Insertion modes for the tree builder (Phase 1: IN_BODY only).
#[derive(Debug, Clone, PartialEq, Eq)]
enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    AfterHead,
    InBody,
    AfterBody,
}

/// Tree builder — constructs a DOM tree from tokens.
///
/// Generated from Parser IR with 16 tree rules.
/// Each rule maps (insertion_mode, token_type) → (actions[], next_mode).
pub struct TreeBuilder {
    mode: InsertionMode,
    /// Stack of indices into the nodes arena.
    stack: Vec<usize>,
    /// Arena of all nodes. Index 0 is the root.
    nodes: Vec<Node>,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self {
            mode: InsertionMode::Initial,
            stack: Vec::new(),
            nodes: Vec::new(),
        }
    }

    /// Process a token and return any DOM changes.
    pub fn process(&mut self, token: &Token) {
        match self.mode {
            InsertionMode::Initial => self.handle_initial(token),
            InsertionMode::InBody => self.handle_in_body(token),
            InsertionMode::AfterBody => { /* ignore */ }
            _ => { /* Phase 1: other modes not yet implemented */ }
        }
    }

    fn alloc_node(&mut self, elem: Element) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(Node::Element(elem));
        idx
    }

    fn handle_initial(&mut self, token: &Token) {
        // Always transition to InBody on first token (simplified)
        self.mode = InsertionMode::InBody;
        // Create root element
        let root = Element::new("html");
        self.nodes.push(Node::Element(root));
        let body = Element::new("body");
        self.nodes.push(Node::Element(body));
        self.stack.push(1); // index of body
        self.handle_in_body(token);
    }

    fn handle_in_body(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs, self_closing } => {
                let mut elem = Element::new(name);
                for (k, v) in attrs {
                    elem.attrs.insert(k.clone(), v.clone());
                }
                let idx = self.nodes.len();
                self.nodes.push(Node::Element(elem.clone()));
                // Add to parent
                if let Some(&parent_idx) = self.stack.last() {
                    if let Some(Node::Element(parent)) = self.nodes.get_mut(parent_idx) {
                        parent.children.push(Node::Element(elem));
                    }
                }
                if !*self_closing && !VOID_ELEMENTS.contains(&name.as_str()) {
                    self.stack.push(idx);
                }
            }
            Token::EndTag { name } => {
                // Pop until we find the matching tag
                while let Some(idx) = self.stack.pop() {
                    if let Some(Node::Element(e)) = self.nodes.get(idx) {
                        if e.tag == *name {
                            break;
                        }
                    }
                }
            }
            Token::Character(text) => {
                if let Some(&parent_idx) = self.stack.last() {
                    if let Some(Node::Element(parent)) = self.nodes.get_mut(parent_idx) {
                        if !text.trim().is_empty() {
                            parent.children.push(Node::Text(text.clone()));
                        }
                    }
                }
            }
            Token::Comment(text) => {
                if let Some(&parent_idx) = self.stack.last() {
                    if let Some(Node::Element(parent)) = self.nodes.get_mut(parent_idx) {
                        parent.children.push(Node::Comment(text.clone()));
                    }
                }
            }
            Token::Eof => {
                self.mode = InsertionMode::AfterBody;
            }
        }
    }

    /// Take the constructed DOM tree.
    pub fn take_root(&mut self) -> Option<Node> {
        self.nodes.first().cloned()
    }
}

/// Void elements — never pushed to the stack.
pub const VOID_ELEMENTS: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];
