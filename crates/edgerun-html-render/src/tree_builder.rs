// DO NOT EDIT.
// Auto-generated from Parser IR by cmd/html-codegen
// Regenerate: go run ./cmd/html-codegen
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

use crate::html_parser::{Node, Element};
use crate::tokenizer::Token;

/// Void element tags — never pushed to the open elements stack.
const VOID_ELEMENTS: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];

/// Insertion modes from WHATWG §13.2.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InTableText,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InSelect,
    InSelectInTable,
    InTemplate,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset,
}

/// WHATWG §13.2.6 HTML tree builder — rule-driven DOM construction.
///
/// Generated from proto IR with 42 tree rules across 24 insertion modes.
///
/// The tree builder consumes tokens from the tokenizer and produces a DOM tree
/// by applying insertion mode rules from the WHATWG spec.
pub struct TreeBuilder {
    open_elements: Vec<Element>,
    insertion_mode: InsertionMode,
    done: bool,
    parse_errors: usize,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self {
            open_elements: Vec::new(),
            insertion_mode: InsertionMode::InBody,
            done: false,
            parse_errors: 0,
        }
    }

    /// Handle a single token, applying tree builder rules.
    pub fn handle_token(&mut self, token: &Token) {
        match self.insertion_mode {
            InsertionMode::InBody => self.handle_in_body(token),
            _ => self.handle_fallback(token),
        }
    }

    /// IN_BODY mode rules (§13.2.6.4.16.7).
    /// Generated from 36 rules.
    fn handle_in_body(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs, self_closing } => {
                match &name[..] {
                "a" => {
                    self.insert(name, attrs, *self_closing);
                }
                "br" => {
                    self.insert(name, attrs, *self_closing);
                }
                "div" => {
                    self.insert(name, attrs, *self_closing);
                }
                "h1" => {
                    self.insert(name, attrs, *self_closing);
                }
                "h2" => {
                    self.insert(name, attrs, *self_closing);
                }
                "h3" => {
                    self.insert(name, attrs, *self_closing);
                }
                "hr" => {
                    self.insert(name, attrs, *self_closing);
                }
                "img" => {
                    self.insert(name, attrs, *self_closing);
                }
                "input" => {
                    self.insert(name, attrs, *self_closing);
                }
                "li" => {
                    self.insert(name, attrs, *self_closing);
                }
                "link" => {
                    self.insert(name, attrs, *self_closing);
                }
                "meta" => {
                    self.insert(name, attrs, *self_closing);
                }
                "p" => {
                    self.insert(name, attrs, *self_closing);
                }
                "script" => {
                    self.insert(name, attrs, *self_closing);
                    // TODO: TREE_ACTION_SWITCH_TO_SCRIPT_DATA
                }
                "span" => {
                    self.insert(name, attrs, *self_closing);
                }
                "style" => {
                    self.insert(name, attrs, *self_closing);
                    // TODO: TREE_ACTION_SWITCH_TO_RAWTEXT
                }
                "textarea" => {
                    self.insert(name, attrs, *self_closing);
                    // TODO: TREE_ACTION_SWITCH_TO_RCDATA
                }
                "title" => {
                    self.insert(name, attrs, *self_closing);
                    // TODO: TREE_ACTION_SWITCH_TO_RCDATA
                }
                "ul" => {
                    self.insert(name, attrs, *self_closing);
                }
                _ => {
                    self.insert(name, attrs, *self_closing);
                }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "a" => {
                    self.pop_until("a");
                }
                "div" => {
                    self.pop_until("div");
                }
                "h1" => {
                    self.pop_until("h1");
                }
                "h2" => {
                    self.pop_until("h2");
                }
                "h3" => {
                    self.pop_until("h3");
                }
                "li" => {
                    self.pop_until("li");
                }
                "p" => {
                    self.pop_until("p");
                }
                "script" => {
                    self.pop_until("script");
                }
                "span" => {
                    self.pop_until("span");
                }
                "style" => {
                    self.pop_until("style");
                }
                "textarea" => {
                    self.pop_until("textarea");
                }
                "title" => {
                    self.pop_until("title");
                }
                "ul" => {
                    self.pop_until("ul");
                }
                _ => {
                    self.pop_until(name);
                }
                }
            }
            Token::Character(text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(text.clone())); }
            }
            Token::Comment(_) => {
                // Comments are appended to the current node in IN_BODY mode
                if let Some(parent) = self.open_elements.last_mut() {
                    // TODO: append comment
                }
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterBody;
            }
        }
    }

    /// Fallback handler for unimplemented insertion modes.
    fn handle_fallback(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs, self_closing: _ } => {
                // INSERT (generic)
                let mut elem = Element::new(name);
                for (k, v) in attrs { elem.attrs.insert(k.clone(), v.clone()); }
                self.open_elements.push(elem);
            }
            Token::EndTag { name } => {
                // POP until matching tag (generic)
                self.pop_until(name);
            }
            Token::Character(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Text(text.clone()));
                }
            }
            Token::Comment(_) | Token::Eof => {}
        }
    }

    /// Action: INSERT — create element and push to stack.
    fn insert(&mut self, name: &str, attrs: &BTreeMap<String, String>, self_closing: bool) {
        let mut elem = Element::new(name);
        for (k, v) in attrs { elem.attrs.insert(k.clone(), v.clone()); }
        // Void elements are not pushed to the open elements stack
        if !self_closing && !VOID_ELEMENTS.contains(&name) {
            self.open_elements.push(elem);
        }
    }

    /// Action: POP_UNTIL — pop elements until the named tag is found.
    fn pop_until(&mut self, name: &str) {
        let mut found: Option<Node> = None;
        while let Some(elem) = self.open_elements.pop() {
            if elem.tag == name { found = Some(Node::Element(elem)); break; } else {
                let node = Node::Element(elem);
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(node); }
            }
        }
        if let Some(node) = found {
            if let Some(parent) = self.open_elements.last_mut() { parent.children.push(node); }
        }
    }

    /// Finish tree building, closing all open elements.
    pub fn finish(mut self) -> Vec<Node> {
        let mut roots: Vec<Node> = Vec::new();
        while let Some(elem) = self.open_elements.pop() {
            let node = Node::Element(elem);
            if let Some(parent) = self.open_elements.last_mut() { parent.children.push(node); }
            else { roots.push(node); }
        }
        roots.reverse(); roots
    }

    pub fn insertion_mode(&self) -> InsertionMode { self.insertion_mode }
    pub fn parse_errors(&self) -> usize { self.parse_errors }
}
