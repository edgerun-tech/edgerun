// DO NOT EDIT.
// Auto-generated from Parser IR by cmd/html-codegen
// Regenerate: go run ./cmd/html-codegen
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

use super::html_parser::{Node, Element};
use super::tokenizer::Token;

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

/// Tokenizer state overrides — set by tree builder when entering raw text elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenizerMode {
    None,
    Rawtext,
    Rcdata,
    ScriptData,
}

/// WHATWG §13.2.6 HTML tree builder — rule-driven DOM construction.
///
/// Generated from proto IR with 431 tree rules across 24 insertion modes.
///
/// The tree builder consumes tokens from the tokenizer and produces a DOM tree
/// by applying insertion mode rules from the WHATWG spec.
pub struct TreeBuilder {
    open_elements: Vec<Element>,
    /// Completed root elements (popped with no parent on stack).
    completed: Vec<Node>,
    insertion_mode: InsertionMode,
    /// Pending tokenizer state override (set by switch_to_rawtext/rcdata/script_data).
    pending_tokenizer_mode: TokenizerMode,
    parse_errors: usize,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self {
            open_elements: Vec::new(),
            completed: Vec::new(),
            insertion_mode: InsertionMode::Initial,
            pending_tokenizer_mode: TokenizerMode::None,
            parse_errors: 0,
        }
    }

    /// Handle a single token, applying tree builder rules.
    pub fn handle_token(&mut self, token: &Token) {
        match self.insertion_mode {
            InsertionMode::Initial => self.handle_initial(token),
            InsertionMode::BeforeHtml => self.handle_before_html(token),
            InsertionMode::BeforeHead => self.handle_before_head(token),
            InsertionMode::InHead => self.handle_in_head(token),
            InsertionMode::InHeadNoscript => self.handle_in_head_noscript(token),
            InsertionMode::AfterHead => self.handle_after_head(token),
            InsertionMode::InBody => self.handle_in_body(token),
            InsertionMode::Text => self.handle_text(token),
            InsertionMode::InTable => self.handle_in_table(token),
            InsertionMode::InTableText => self.handle_in_table_text(token),
            InsertionMode::InTableBody => self.handle_in_table_body(token),
            InsertionMode::InRow => self.handle_in_row(token),
            InsertionMode::InCell => self.handle_in_cell(token),
            InsertionMode::InCaption => self.handle_in_caption(token),
            InsertionMode::InColumnGroup => self.handle_in_column_group(token),
            InsertionMode::AfterBody => self.handle_after_body(token),
            InsertionMode::InFrameset => self.handle_in_frameset(token),
            InsertionMode::AfterFrameset => self.handle_after_frameset(token),
            InsertionMode::AfterAfterBody => self.handle_after_after_body(token),
            InsertionMode::AfterAfterFrameset => self.handle_after_after_frameset(token),
            InsertionMode::InSelect => self.handle_in_select(token),
            InsertionMode::InSelectInTable => self.handle_in_select_in_table(token),
            InsertionMode::InTemplate => self.handle_in_template(token),
        }
    }

    /// Initial mode rules.
    /// Generated from 5 rules.
    fn handle_initial(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                _ => {
                    self.parse_errors += 1;
                }
                }
                // Reprocess in next mode (otherwise rule)
                self.insertion_mode = InsertionMode::BeforeHtml;
                self.handle_token(token);
            }
            Token::EndTag { name } => {
                self.pop_until(name);
            }
            Token::Character(_text) => {
                // no character rule defined
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                self.open_elements.push(Element::new("html"));
                // TODO: create proper DOCTYPE node
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// BeforeHtml mode rules.
    /// Generated from 6 rules.
    fn handle_before_html(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "html" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                _ => {
                    self.parse_errors += 1;
                }
                }
                // Reprocess in next mode (otherwise rule)
                self.insertion_mode = InsertionMode::BeforeHead;
                self.handle_token(token);
            }
            Token::EndTag { name } => {
                self.pop_until(name);
            }
            Token::Character(_text) => {
                // no character rule defined
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // parse error, ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// BeforeHead mode rules.
    /// Generated from 8 rules.
    fn handle_before_head(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "head" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "html" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                _ => {
                    self.parse_errors += 1;
                }
                }
                // Reprocess in next mode (otherwise rule)
                self.insertion_mode = InsertionMode::InHead;
                self.handle_token(token);
            }
            Token::EndTag { name } => {
                match &name[..] {
                "head" => {
                    self.pop_until("head");
                }
                _ => {
                    if self.has_in_scope(name) { self.pop_until(name); }
                    // otherwise: parse error, ignore
                }
                }
            }
            Token::Character(_text) => {
                // parse error, ignore character
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // parse error, ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InHead mode rules.
    /// Generated from 20 rules.
    fn handle_in_head(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "base" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "basefont" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "bgsound" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "head" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "html" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "link" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "meta" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "noframes" => {
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_rawtext();
                }
                "noscript" => {
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_rawtext();
                }
                "script" => {
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_script_data();
                }
                "style" => {
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_rawtext();
                }
                "title" => {
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_rcdata();
                }
                _ => {
                    self.pop_until("head");
                }
                }
                // Reprocess in next mode (otherwise rule)
                self.insertion_mode = InsertionMode::AfterHead;
                self.handle_token(token);
            }
            Token::EndTag { name } => {
                match &name[..] {
                "body" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "head" => {
                    self.pop_until("head");
                }
                "html" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                _ => {
                    if self.has_in_scope(name) { self.pop_until(name); }
                    // otherwise: parse error, ignore
                }
                }
            }
            Token::Character(_text) => {
                // parse error, ignore character
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // parse error, ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InHeadNoscript mode rules.
    /// Generated from 20 rules.
    fn handle_in_head_noscript(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "a" => {
                    self.parse_errors += 1;
                }
                "b" => {
                    self.parse_errors += 1;
                }
                "big" => {
                    self.parse_errors += 1;
                }
                "code" => {
                    self.parse_errors += 1;
                }
                "em" => {
                    self.parse_errors += 1;
                }
                "font" => {
                    self.parse_errors += 1;
                }
                "i" => {
                    self.parse_errors += 1;
                }
                "noscript" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "s" => {
                    self.parse_errors += 1;
                }
                "small" => {
                    self.parse_errors += 1;
                }
                "strike" => {
                    self.parse_errors += 1;
                }
                "strong" => {
                    self.parse_errors += 1;
                }
                "tt" => {
                    self.parse_errors += 1;
                }
                "u" => {
                    self.parse_errors += 1;
                }
                _ => {
                    self.parse_errors += 1;
                    // ignore token
                }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "noscript" => {
                    if let Some(_) = self.open_elements.pop() {}
                }
                _ => {
                    self.parse_errors += 1;
                    // ignore token
                }
                }
            }
            Token::Character(_text) => {
                // parse error, ignore character
            }
            Token::Comment(_text) => {
                // ignore comment
            }
            Token::Doctype => {
                // parse error, ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// AfterHead mode rules.
    /// Generated from 11 rules.
    fn handle_after_head(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "body" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "frameset" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "head" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "html" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                _ => {
                    self.parse_errors += 1;
                }
                }
                // Reprocess in next mode (otherwise rule)
                self.insertion_mode = InsertionMode::InBody;
                self.handle_token(token);
            }
            Token::EndTag { name } => {
                match &name[..] {
                "body" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "html" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                _ => {
                    if self.has_in_scope(name) { self.pop_until(name); }
                    // otherwise: parse error, ignore
                }
                }
            }
            Token::Character(_text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(_text.clone())); }
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // parse error, ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InBody mode rules.
    /// Generated from 171 rules.
    fn handle_in_body(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "a" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "abbr" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "address" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "article" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "aside" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "b" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "base" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "basefont" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "bgsound" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "big" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "blockquote" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "body" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "br" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "cite" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "code" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "dd" => {
                    if self.has_in_scope("dt") { self.pop_until("dt"); }
                    self.pop_until("dt");
                    self.insert(name, _attrs, *_self_closing);
                }
                "details" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "dfn" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "dialog" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "div" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "dl" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "dt" => {
                    if self.has_in_scope("dd") { self.pop_until("dd"); }
                    self.pop_until("dd");
                    self.insert(name, _attrs, *_self_closing);
                }
                "em" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "fieldset" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "figcaption" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "figure" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "footer" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "form" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "frameset" => {
                    self.parse_errors += 1;
                    while self.open_elements.pop().is_some() {}
                }
                "h1" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "h2" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "h3" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "h4" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "h5" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "h6" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "header" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "hgroup" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "hr" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "html" => {
                    self.parse_errors += 1;
                }
                "i" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "img" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "input" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "li" => {
                    if self.has_in_list_item_scope("li") { self.pop_until("li"); }
                    self.pop_until("li");
                    self.insert(name, _attrs, *_self_closing);
                }
                "link" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "main" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "menu" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "meta" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "nav" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "noframes" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "ol" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "p" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "pre" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "q" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "s" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "script" => {
                    self.parse_errors += 1;
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_script_data();
                }
                "section" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "small" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "span" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "strong" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "style" => {
                    self.parse_errors += 1;
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_rawtext();
                }
                "sub" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "sup" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "table" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                "textarea" => {
                    self.parse_errors += 1;
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_rcdata();
                }
                "title" => {
                    self.parse_errors += 1;
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_rcdata();
                }
                "u" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "ul" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, _attrs, *_self_closing);
                }
                _ => {
                    self.insert(name, _attrs, *_self_closing);
                }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "a" => {
                    self.pop_until("a");
                }
                "abbr" => {
                    self.pop_until("abbr");
                }
                "article" => {
                    self.pop_until("article");
                }
                "aside" => {
                    self.pop_until("aside");
                }
                "b" => {
                    self.pop_until("b");
                }
                "big" => {
                    self.pop_until("big");
                }
                "blockquote" => {
                    self.pop_until("blockquote");
                }
                "body" => {
                    self.pop_until("body");
                }
                "cite" => {
                    self.pop_until("cite");
                }
                "code" => {
                    self.pop_until("code");
                }
                "dd" => {
                    self.pop_until("dd");
                }
                "dfn" => {
                    self.pop_until("dfn");
                }
                "div" => {
                    self.pop_until("div");
                }
                "dl" => {
                    self.pop_until("dl");
                }
                "dt" => {
                    self.pop_until("dt");
                }
                "em" => {
                    self.pop_until("em");
                }
                "footer" => {
                    self.pop_until("footer");
                }
                "form" => {
                    self.pop_until("form");
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
                "h4" => {
                    self.pop_until("h4");
                }
                "h5" => {
                    self.pop_until("h5");
                }
                "h6" => {
                    self.pop_until("h6");
                }
                "header" => {
                    self.pop_until("header");
                }
                "html" => {
                    self.pop_until("html");
                }
                "i" => {
                    self.pop_until("i");
                }
                "li" => {
                    self.pop_until("li");
                }
                "main" => {
                    self.pop_until("main");
                }
                "nav" => {
                    self.pop_until("nav");
                }
                "ol" => {
                    self.pop_until("ol");
                }
                "p" => {
                    self.pop_until("p");
                }
                "pre" => {
                    self.pop_until("pre");
                }
                "q" => {
                    self.pop_until("q");
                }
                "s" => {
                    self.pop_until("s");
                }
                "script" => {
                    self.pop_until("script");
                }
                "section" => {
                    self.pop_until("section");
                }
                "small" => {
                    self.pop_until("small");
                }
                "span" => {
                    self.pop_until("span");
                }
                "strong" => {
                    self.pop_until("strong");
                }
                "style" => {
                    self.pop_until("style");
                }
                "sub" => {
                    self.pop_until("sub");
                }
                "sup" => {
                    self.pop_until("sup");
                }
                "table" => {
                    self.pop_until("table");
                }
                "tbody" => {
                    self.pop_until("tbody");
                }
                "td" => {
                    self.pop_until("td");
                }
                "textarea" => {
                    self.pop_until("textarea");
                }
                "tfoot" => {
                    self.pop_until("tfoot");
                }
                "th" => {
                    self.pop_until("th");
                }
                "thead" => {
                    self.pop_until("thead");
                }
                "title" => {
                    self.pop_until("title");
                }
                "tr" => {
                    self.pop_until("tr");
                }
                "u" => {
                    self.pop_until("u");
                }
                "ul" => {
                    self.pop_until("ul");
                }
                _ => {
                    self.pop_until(name);
                }
                }
            }
            Token::Character(_text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(_text.clone())); }
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // parse error, ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterBody;
            }
        }
    }

    /// Text mode rules.
    /// Generated from 8 rules.
    fn handle_text(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                self.insert(name, _attrs, *_self_closing);
            }
            Token::EndTag { name } => {
                match &name[..] {
                "noframes" => {
                    self.pop_until("noframes");
                }
                "noscript" => {
                    self.pop_until("noscript");
                }
                "script" => {
                    self.pop_until("script");
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
                _ => {
                    if self.has_in_scope(name) { self.pop_until(name); }
                    // otherwise: parse error, ignore
                }
                }
            }
            Token::Character(_text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(_text.clone())); }
            }
            Token::Comment(_text) => {
                // ignore comment
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InTable mode rules.
    /// Generated from 31 rules.
    fn handle_in_table(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "caption" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "col" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "colgroup" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "form" => {
                    self.parse_errors += 1;
                    self.insert(name, _attrs, *_self_closing);
                }
                "input" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "script" => {
                    self.parse_errors += 1;
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_script_data();
                }
                "style" => {
                    self.parse_errors += 1;
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_rawtext();
                }
                "table" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tbody" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "td" => {
                    self.insert_foster(name, _attrs);
                }
                "tfoot" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "th" => {
                    self.insert_foster(name, _attrs);
                }
                "thead" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "tr" => {
                    self.insert_foster(name, _attrs);
                }
                _ => {
                    self.parse_errors += 1;
                    self.insert_foster(name, _attrs);
                }
                }
                // Reprocess in next mode (otherwise rule)
                self.insertion_mode = InsertionMode::InTable;
                self.handle_token(token);
            }
            Token::EndTag { name } => {
                match &name[..] {
                "body" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "caption" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "col" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "colgroup" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "html" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "table" => {
                    self.pop_until("table");
                }
                "tbody" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "td" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tfoot" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "th" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "thead" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tr" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                _ => {
                    if self.has_in_scope(name) { self.pop_until(name); }
                    // otherwise: parse error, ignore
                }
                }
            }
            Token::Character(_text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InTableText;
                self.handle_token(token);
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // parse error, ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InTableText mode rules.
    /// Generated from 6 rules.
    fn handle_in_table_text(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                _ => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InTable;
                    self.handle_token(token);
                    return;
                }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "table" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InTable;
                    self.handle_token(token);
                    return;
                }
                _ => {
                    if self.has_in_scope(name) { self.pop_until(name); }
                    // otherwise: parse error, ignore
                }
                }
            }
            Token::Character(_text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InBody;
                self.handle_token(token);
            }
            Token::Comment(_text) => {
                // ignore comment
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InTableBody mode rules.
    /// Generated from 18 rules.
    fn handle_in_table_body(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "caption" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "col" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "colgroup" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "script" => {
                    self.parse_errors += 1;
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_script_data();
                }
                "style" => {
                    self.parse_errors += 1;
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_rawtext();
                }
                "table" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tbody" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "td" => {
                    self.parse_errors += 1;
                    self.insert_foster(name, _attrs);
                }
                "tfoot" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "th" => {
                    self.parse_errors += 1;
                    self.insert_foster(name, _attrs);
                }
                "thead" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tr" => {
                    self.insert_foster(name, _attrs);
                }
                _ => { self.insert(name, _attrs, *_self_closing); }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "tbody" => {
                    self.pop_until("tbody");
                }
                "tfoot" => {
                    self.pop_until("tfoot");
                }
                "thead" => {
                    self.pop_until("thead");
                }
                _ => {
                    if self.has_in_scope(name) { self.pop_until(name); }
                    // otherwise: parse error, ignore
                }
                }
            }
            Token::Character(_text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InTableText;
                self.handle_token(token);
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InRow mode rules.
    /// Generated from 18 rules.
    fn handle_in_row(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "caption" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "col" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "colgroup" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "script" => {
                    self.parse_errors += 1;
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_script_data();
                }
                "style" => {
                    self.parse_errors += 1;
                    self.insert(name, _attrs, *_self_closing);
                    self.switch_to_rawtext();
                }
                "table" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tbody" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "td" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "tfoot" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "th" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "thead" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tr" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                _ => { self.insert(name, _attrs, *_self_closing); }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "td" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "th" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tr" => {
                    self.pop_until("tr");
                }
                _ => {
                    if self.has_in_scope(name) { self.pop_until(name); }
                    // otherwise: parse error, ignore
                }
                }
            }
            Token::Character(_text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InTableText;
                self.handle_token(token);
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InCell mode rules.
    /// Generated from 15 rules.
    fn handle_in_cell(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "caption" => {
                    self.parse_errors += 1;
                }
                "col" => {
                    self.parse_errors += 1;
                }
                "colgroup" => {
                    self.parse_errors += 1;
                }
                "tbody" => {
                    self.parse_errors += 1;
                }
                "td" => {
                    self.parse_errors += 1;
                }
                "tfoot" => {
                    self.parse_errors += 1;
                }
                "th" => {
                    self.parse_errors += 1;
                }
                "thead" => {
                    self.parse_errors += 1;
                }
                "tr" => {
                    self.parse_errors += 1;
                }
                _ => {
                    self.insert(name, _attrs, *_self_closing);
                }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "td" => {
                    self.pop_until("td");
                }
                "th" => {
                    self.pop_until("th");
                }
                _ => {
                    if self.has_in_scope(name) { self.pop_until(name); }
                    // otherwise: parse error, ignore
                }
                }
            }
            Token::Character(_text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(_text.clone())); }
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InCaption mode rules.
    /// Generated from 14 rules.
    fn handle_in_caption(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "caption" => {
                    self.parse_errors += 1;
                }
                "col" => {
                    self.parse_errors += 1;
                }
                "colgroup" => {
                    self.parse_errors += 1;
                }
                "tbody" => {
                    self.parse_errors += 1;
                }
                "td" => {
                    self.parse_errors += 1;
                }
                "tfoot" => {
                    self.parse_errors += 1;
                }
                "th" => {
                    self.parse_errors += 1;
                }
                "thead" => {
                    self.parse_errors += 1;
                }
                "tr" => {
                    self.parse_errors += 1;
                }
                _ => {
                    self.insert(name, _attrs, *_self_closing);
                }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "caption" => {
                    self.pop_until("caption");
                }
                _ => {
                    if self.has_in_scope(name) { self.pop_until(name); }
                    // otherwise: parse error, ignore
                }
                }
            }
            Token::Character(_text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(_text.clone())); }
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InColumnGroup mode rules.
    /// Generated from 6 rules.
    fn handle_in_column_group(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "col" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                _ => {
                    self.parse_errors += 1;
                }
                }
                // Reprocess in next mode (otherwise rule)
                self.insertion_mode = InsertionMode::InTable;
                self.handle_token(token);
            }
            Token::EndTag { name } => {
                match &name[..] {
                "colgroup" => {
                    self.pop_until("colgroup");
                }
                _ => {
                    if self.has_in_scope(name) { self.pop_until(name); }
                    // otherwise: parse error, ignore
                }
                }
            }
            Token::Character(_text) => {
                // parse error, ignore character
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// AfterBody mode rules.
    /// Generated from 3 rules.
    fn handle_after_body(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                self.insert(name, _attrs, *_self_closing);
            }
            Token::EndTag { name } => {
                self.pop_until(name);
            }
            Token::Character(_text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InBody;
                self.handle_token(token);
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InFrameset mode rules.
    /// Generated from 4 rules.
    fn handle_in_frameset(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "frameset" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                "html" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                _ => { self.insert(name, _attrs, *_self_closing); }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "frameset" => {
                    self.pop_until("frameset");
                }
                _ => {
                    if self.has_in_scope(name) { self.pop_until(name); }
                    // otherwise: parse error, ignore
                }
                }
            }
            Token::Character(_text) => {
                // no character rule defined
            }
            Token::Comment(_text) => {
                // ignore comment
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// AfterFrameset mode rules.
    /// Generated from 2 rules.
    fn handle_after_frameset(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                self.insert(name, _attrs, *_self_closing);
            }
            Token::EndTag { name } => {
                self.pop_until(name);
            }
            Token::Character(_text) => {
                // no character rule defined
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterFrameset;
            }
        }
    }

    /// AfterAfterBody mode rules.
    /// Generated from 4 rules.
    fn handle_after_after_body(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "html" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                _ => { self.insert(name, _attrs, *_self_closing); }
                }
            }
            Token::EndTag { name } => {
                self.pop_until(name);
            }
            Token::Character(_text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InBody;
                self.handle_token(token);
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// AfterAfterFrameset mode rules.
    /// Generated from 2 rules.
    fn handle_after_after_frameset(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                self.insert(name, _attrs, *_self_closing);
            }
            Token::EndTag { name } => {
                self.pop_until(name);
            }
            Token::Character(_text) => {
                // no character rule defined
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterFrameset;
            }
        }
    }

    /// InSelect mode rules.
    /// Generated from 9 rules.
    fn handle_in_select(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "noscript" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "template" => {
                    self.insert(name, _attrs, *_self_closing);
                }
                _ => {
                    self.parse_errors += 1;
                    // ignore token
                }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "noscript" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "select" => {
                    self.parse_errors += 1;
                    self.pop_until("select");
                }
                _ => {
                    self.parse_errors += 1;
                    // ignore token
                }
                }
            }
            Token::Character(_text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(_text.clone())); }
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InSelectInTable mode rules.
    /// Generated from 20 rules.
    fn handle_in_select_in_table(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "caption" => {
                    self.parse_errors += 1;
                }
                "table" => {
                    self.parse_errors += 1;
                }
                "tbody" => {
                    self.parse_errors += 1;
                }
                "td" => {
                    self.parse_errors += 1;
                }
                "tfoot" => {
                    self.parse_errors += 1;
                }
                "th" => {
                    self.parse_errors += 1;
                }
                "thead" => {
                    self.parse_errors += 1;
                }
                "tr" => {
                    self.parse_errors += 1;
                }
                _ => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InSelect;
                    self.handle_token(token);
                    return;
                }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "caption" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "table" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tbody" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "td" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tfoot" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "th" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "thead" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tr" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                _ => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InSelect;
                    self.handle_token(token);
                    return;
                }
                }
            }
            Token::Character(_text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InSelect;
                self.handle_token(token);
            }
            Token::Comment(_text) => {
                // ignore comment
            }
            Token::Doctype => {
                // ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }

    /// InTemplate mode rules.
    /// Generated from 30 rules.
    fn handle_in_template(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                match &name[..] {
                "base" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InHead;
                    self.handle_token(token);
                    return;
                }
                "basefont" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InHead;
                    self.handle_token(token);
                    return;
                }
                "bgsound" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InHead;
                    self.handle_token(token);
                    return;
                }
                "body" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InBody;
                    self.handle_token(token);
                    return;
                }
                "caption" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InTable;
                    self.handle_token(token);
                    return;
                }
                "col" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InColumnGroup;
                    self.handle_token(token);
                    return;
                }
                "colgroup" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InTable;
                    self.handle_token(token);
                    return;
                }
                "frameset" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InBody;
                    self.handle_token(token);
                    return;
                }
                "html" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InBody;
                    self.handle_token(token);
                    return;
                }
                "link" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InHead;
                    self.handle_token(token);
                    return;
                }
                "meta" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InHead;
                    self.handle_token(token);
                    return;
                }
                "noframes" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InHead;
                    self.handle_token(token);
                    return;
                }
                "script" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InHead;
                    self.handle_token(token);
                    return;
                }
                "style" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InHead;
                    self.handle_token(token);
                    return;
                }
                "tbody" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InTable;
                    self.handle_token(token);
                    return;
                }
                "td" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InRow;
                    self.handle_token(token);
                    return;
                }
                "template" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InHead;
                    self.handle_token(token);
                    return;
                }
                "tfoot" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InTable;
                    self.handle_token(token);
                    return;
                }
                "th" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InRow;
                    self.handle_token(token);
                    return;
                }
                "thead" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InTable;
                    self.handle_token(token);
                    return;
                }
                "title" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InHead;
                    self.handle_token(token);
                    return;
                }
                "tr" => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InTableBody;
                    self.handle_token(token);
                    return;
                }
                _ => {
                    self.parse_errors += 1;
                    self.insertion_mode = InsertionMode::InBody;
                    self.handle_token(token);
                    return;
                }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "template" => {
                    if let Some(_) = self.open_elements.pop() {}
                    self.reset_insertion_mode();
                }
                _ => {
                    self.parse_errors += 1;
                    // ignore token
                }
                }
            }
            Token::Character(_text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(_text.clone())); }
            }
            Token::Comment(_text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(_text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }
            }
            Token::Doctype => {
                // parse error, ignore DOCTYPE
            }
            Token::Eof => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
        }
    }



    /// Fallback handler for unimplemented insertion modes.
    /// All 24 WHATWG modes are implemented — this is unreachable.
    #[allow(dead_code, unreachable_code)]
    fn handle_fallback(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs: _attrs, self_closing: _self_closing } => {
                self.insert(name, _attrs, *_self_closing);
            }
            Token::EndTag { name } => {
                self.pop_until(name);
            }
            Token::Character(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Text(text.clone()));
                }
            }
            Token::Comment(_) | Token::Eof | Token::Doctype => {}
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
            if let Some(parent) = self.open_elements.last_mut() {
                parent.children.push(node);
            } else {
                self.completed.push(node);
            }
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
        roots.reverse();
        // Prepend completed roots (already in document order)
        let mut all_roots = self.completed;
        all_roots.extend(roots);
        all_roots
    }

    /// Check if an element with the given tag name exists in the stack.
    fn has_in_scope(&self, tag: &str) -> bool {
        self.open_elements.iter().any(|e| e.tag == tag)
    }

    /// Check if an element exists in "button scope" (in scope, stopping at scope blockers).
    /// Button scope blockers: applet, button, marquee, object, table, td, th.
    fn has_in_button_scope(&self, tag: &str) -> bool {
        let blockers = &["applet", "button", "marquee", "object", "table", "td", "th"];
        for elem in self.open_elements.iter().rev() {
            if elem.tag == tag { return true; }
            if blockers.contains(&elem.tag.as_str()) { return false; }
        }
        false
    }

    /// Check if an element exists in "list item scope".
    /// List item scope blockers: all button scope blockers + ol, ul.
    fn has_in_list_item_scope(&self, tag: &str) -> bool {
        let blockers = &["applet", "button", "marquee", "object", "table", "td", "th", "ol", "ul"];
        for elem in self.open_elements.iter().rev() {
            if elem.tag == tag { return true; }
            if blockers.contains(&elem.tag.as_str()) { return false; }
        }
        false
    }

    pub fn insertion_mode(&self) -> InsertionMode { self.insertion_mode }
    pub fn parse_errors(&self) -> usize { self.parse_errors }

    /// Signal the tokenizer to switch to RAWTEXT mode (for <style>, <noscript>, <noframes>).
    pub fn switch_to_rawtext(&mut self) {
        self.pending_tokenizer_mode = TokenizerMode::Rawtext;
    }

    /// Signal the tokenizer to switch to RCDATA mode (for <title>, <textarea>).
    pub fn switch_to_rcdata(&mut self) {
        self.pending_tokenizer_mode = TokenizerMode::Rcdata;
    }

    /// Signal the tokenizer to switch to SCRIPT DATA mode (for <script>).
    pub fn switch_to_script_data(&mut self) {
        self.pending_tokenizer_mode = TokenizerMode::ScriptData;
    }

    /// Clear any pending tokenizer mode switch.
    pub fn take_tokenizer_mode(&mut self) -> Option<TokenizerMode> {
        let m = self.pending_tokenizer_mode;
        self.pending_tokenizer_mode = TokenizerMode::None;
        match m {
            TokenizerMode::None => None,
            _ => Some(m),
        }
    }

    /// WHATWG §13.2.6.4.1 — Foster parent insertion.
    ///
    /// When content appears where it is not allowed (e.g., text directly
    /// inside <table>), insert it outside the table element instead.
    fn insert_foster(&mut self, name: &str, attrs: &BTreeMap<String, String>) {
        let mut elem = Element::new(name);
        for (k, v) in attrs { elem.attrs.insert(k.clone(), v.clone()); }

        let table_idx = self.open_elements.iter().rposition(|e| e.tag == "table");
        let template_idx = self.open_elements.iter().rposition(|e| e.tag == "template");

        if let Some(ti) = table_idx {
            if let Some(templ_idx) = template_idx {
                if templ_idx < ti {
                    self.open_elements[templ_idx].children.push(Node::Element(elem));
                    return;
                }
            }
            if ti == 0 {
                // Table is root — insert into html element (foster parent outside table)
                if let Some(html_idx) = self.open_elements.iter().position(|e| e.tag == "html") {
                    self.open_elements[html_idx].children.push(Node::Element(elem));
                } else {
                    self.open_elements[0].children.push(Node::Element(elem));
                }
            } else {
                let parent = &mut self.open_elements[ti - 1];
                parent.children.push(Node::Element(elem));
            }
        } else if let Some(templ_idx) = template_idx {
            self.open_elements[templ_idx].children.push(Node::Element(elem));
        } else if let Some(html_idx) = self.open_elements.iter().rposition(|e| e.tag == "html") {
            self.open_elements[html_idx].children.push(Node::Element(elem));
        } else if let Some(parent) = self.open_elements.last_mut() {
            parent.children.push(Node::Element(elem));
        }
    }

    /// WHATWG §13.2.6.4.10 — Reset insertion mode appropriately.
    fn reset_insertion_mode(&mut self) {
        let last = self.open_elements.len().saturating_sub(1);
        for i in (0..=last).rev() {
            match self.open_elements[i].tag.as_str() {
                "select" => {
                    for j in (0..i).rev() {
                        if self.open_elements[j].tag == "table" {
                            self.insertion_mode = InsertionMode::InSelectInTable;
                            return;
                        }
                        if j == 0 { break; }
                    }
                    self.insertion_mode = InsertionMode::InSelect;
                    return;
                }
                "td" | "th" => { self.insertion_mode = InsertionMode::InCell; return; }
                "tr" => { self.insertion_mode = InsertionMode::InRow; return; }
                "tbody" | "thead" | "tfoot" => { self.insertion_mode = InsertionMode::InTableBody; return; }
                "caption" => { self.insertion_mode = InsertionMode::InCaption; return; }
                "colgroup" => { self.insertion_mode = InsertionMode::InColumnGroup; return; }
                "table" => { self.insertion_mode = InsertionMode::InTable; return; }
                "template" => { return; }
                "head" => { self.insertion_mode = InsertionMode::InHead; return; }
                "body" => { self.insertion_mode = InsertionMode::InBody; return; }
                "frameset" => { self.insertion_mode = InsertionMode::InFrameset; return; }
                "html" => {
                    if self.open_elements.iter().any(|e| e.tag == "head") {
                        self.insertion_mode = InsertionMode::AfterHead;
                    } else {
                        self.insertion_mode = InsertionMode::BeforeHead;
                    }
                    return;
                }
                _ => {}
            }
        }
        self.insertion_mode = InsertionMode::InBody;
    }

    /// Action: INSERT — create element and push to stack.
    fn insert(&mut self, name: &str, attrs: &BTreeMap<String, String>, _self_closing: bool) {
        let mut elem = Element::new(name);
        for (k, v) in attrs { elem.attrs.insert(k.clone(), v.clone()); }
        if VOID_ELEMENTS.contains(&name) {
            // Void elements: attach to parent but don't push to stack
            if let Some(parent) = self.open_elements.last_mut() {
                parent.children.push(Node::Element(elem));
            }
        } else {
            self.open_elements.push(elem);
        }
    }
}
