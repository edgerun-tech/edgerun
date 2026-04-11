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
/// Generated from proto IR with 352 tree rules across 24 insertion modes.
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
    done: bool,
    parse_errors: usize,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self {
            open_elements: Vec::new(),
            completed: Vec::new(),
            insertion_mode: InsertionMode::Initial,
            pending_tokenizer_mode: TokenizerMode::None,
            done: false,
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
            _ => self.handle_fallback(token),
        }
    }

    /// Initial mode rules.
    /// Generated from 5 rules.
    fn handle_initial(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs, self_closing } => {
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
            Token::Character(text) => {
                // no character rule defined
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
                match &name[..] {
                "html" => {
                    self.insert(name, attrs, *self_closing);
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
            Token::Character(text) => {
                // no character rule defined
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
                match &name[..] {
                "head" => {
                    self.insert(name, attrs, *self_closing);
                }
                "html" => {
                    self.insert(name, attrs, *self_closing);
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
                _ => { self.pop_until(name); }
                }
            }
            Token::Character(text) => {
                // parse error, ignore character
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
                match &name[..] {
                "base" => {
                    self.insert(name, attrs, *self_closing);
                }
                "basefont" => {
                    self.insert(name, attrs, *self_closing);
                }
                "bgsound" => {
                    self.insert(name, attrs, *self_closing);
                }
                "head" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "html" => {
                    self.insert(name, attrs, *self_closing);
                }
                "link" => {
                    self.insert(name, attrs, *self_closing);
                }
                "meta" => {
                    self.insert(name, attrs, *self_closing);
                }
                "noframes" => {
                    self.insert(name, attrs, *self_closing);
                    self.switch_to_rawtext();
                }
                "noscript" => {
                    self.insert(name, attrs, *self_closing);
                    self.switch_to_rawtext();
                }
                "script" => {
                    self.insert(name, attrs, *self_closing);
                    self.switch_to_script_data();
                }
                "style" => {
                    self.insert(name, attrs, *self_closing);
                    self.switch_to_rawtext();
                }
                "title" => {
                    self.insert(name, attrs, *self_closing);
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
                _ => { self.pop_until(name); }
                }
            }
            Token::Character(text) => {
                // parse error, ignore character
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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

    /// AfterHead mode rules.
    /// Generated from 11 rules.
    fn handle_after_head(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs, self_closing } => {
                match &name[..] {
                "body" => {
                    self.insert(name, attrs, *self_closing);
                }
                "frameset" => {
                    self.insert(name, attrs, *self_closing);
                }
                "head" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "html" => {
                    self.insert(name, attrs, *self_closing);
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
                _ => { self.pop_until(name); }
                }
            }
            Token::Character(text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(text.clone())); }
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
                match &name[..] {
                "a" => {
                    self.insert(name, attrs, *self_closing);
                }
                "abbr" => {
                    self.insert(name, attrs, *self_closing);
                }
                "address" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "article" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "aside" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "b" => {
                    self.insert(name, attrs, *self_closing);
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
                    self.insert(name, attrs, *self_closing);
                }
                "blockquote" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "body" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "br" => {
                    self.insert(name, attrs, *self_closing);
                }
                "cite" => {
                    self.insert(name, attrs, *self_closing);
                }
                "code" => {
                    self.insert(name, attrs, *self_closing);
                }
                "dd" => {
                    if self.has_in_scope("dt") { self.pop_until("dt"); }
                    self.pop_until("dt");
                    self.insert(name, attrs, *self_closing);
                }
                "details" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "dfn" => {
                    self.insert(name, attrs, *self_closing);
                }
                "dialog" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "div" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "dl" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "dt" => {
                    if self.has_in_scope("dd") { self.pop_until("dd"); }
                    self.pop_until("dd");
                    self.insert(name, attrs, *self_closing);
                }
                "em" => {
                    self.insert(name, attrs, *self_closing);
                }
                "fieldset" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "figcaption" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "figure" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "footer" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "form" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "frameset" => {
                    self.parse_errors += 1;
                    while self.open_elements.pop().is_some() {}
                }
                "h1" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "h2" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "h3" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "h4" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "h5" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "h6" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "header" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "hgroup" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "hr" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "html" => {
                    self.parse_errors += 1;
                }
                "i" => {
                    self.insert(name, attrs, *self_closing);
                }
                "img" => {
                    self.insert(name, attrs, *self_closing);
                }
                "input" => {
                    self.insert(name, attrs, *self_closing);
                }
                "li" => {
                    if self.has_in_list_item_scope("li") { self.pop_until("li"); }
                    self.pop_until("li");
                    self.insert(name, attrs, *self_closing);
                }
                "link" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "main" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "menu" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "meta" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "nav" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "noframes" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "ol" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "p" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "pre" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "q" => {
                    self.insert(name, attrs, *self_closing);
                }
                "s" => {
                    self.insert(name, attrs, *self_closing);
                }
                "script" => {
                    self.parse_errors += 1;
                    self.insert(name, attrs, *self_closing);
                    self.switch_to_script_data();
                }
                "section" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "small" => {
                    self.insert(name, attrs, *self_closing);
                }
                "span" => {
                    self.insert(name, attrs, *self_closing);
                }
                "strong" => {
                    self.insert(name, attrs, *self_closing);
                }
                "style" => {
                    self.parse_errors += 1;
                    self.insert(name, attrs, *self_closing);
                    self.switch_to_rawtext();
                }
                "sub" => {
                    self.insert(name, attrs, *self_closing);
                }
                "sup" => {
                    self.insert(name, attrs, *self_closing);
                }
                "table" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
                    self.insert(name, attrs, *self_closing);
                }
                "textarea" => {
                    self.parse_errors += 1;
                    self.insert(name, attrs, *self_closing);
                    self.switch_to_rcdata();
                }
                "title" => {
                    self.parse_errors += 1;
                    self.insert(name, attrs, *self_closing);
                    self.switch_to_rcdata();
                }
                "u" => {
                    self.insert(name, attrs, *self_closing);
                }
                "ul" => {
                    if self.has_in_button_scope("p") { self.pop_until("p"); }
                    self.pop_until("p");
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
            Token::Character(text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(text.clone())); }
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
                self.insert(name, attrs, *self_closing);
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
                _ => { self.pop_until(name); }
                }
            }
            Token::Character(text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(text.clone())); }
            }
            Token::Comment(text) => {
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
            Token::StartTag { name, attrs, self_closing } => {
                match &name[..] {
                "caption" => {
                    self.insert(name, attrs, *self_closing);
                }
                "col" => {
                    self.insert(name, attrs, *self_closing);
                }
                "colgroup" => {
                    self.insert(name, attrs, *self_closing);
                }
                "form" => {
                    self.parse_errors += 1;
                    self.insert(name, attrs, *self_closing);
                }
                "input" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "script" => {
                    self.parse_errors += 1;
                    self.insert(name, attrs, *self_closing);
                    self.switch_to_script_data();
                }
                "style" => {
                    self.parse_errors += 1;
                    self.insert(name, attrs, *self_closing);
                    self.switch_to_rawtext();
                }
                "table" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tbody" => {
                    self.insert(name, attrs, *self_closing);
                }
                "td" => {
                    // TODO: TREE_ACTION_INSERT_FOSTER
                }
                "tfoot" => {
                    self.insert(name, attrs, *self_closing);
                }
                "th" => {
                    // TODO: TREE_ACTION_INSERT_FOSTER
                }
                "thead" => {
                    self.insert(name, attrs, *self_closing);
                }
                "tr" => {
                    // TODO: TREE_ACTION_INSERT_FOSTER
                }
                _ => {
                    self.parse_errors += 1;
                    // TODO: TREE_ACTION_INSERT_FOSTER
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
                _ => { self.pop_until(name); }
                }
            }
            Token::Character(text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InTableText;
                self.handle_token(token);
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
                match &name[..] {
                _ => {
                    // reprocess token (TODO)
                }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "table" => {
                    // reprocess token (TODO)
                }
                _ => { self.pop_until(name); }
                }
            }
            Token::Character(text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InBody;
                self.handle_token(token);
            }
            Token::Comment(text) => {
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
            Token::StartTag { name, attrs, self_closing } => {
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
                    self.insert(name, attrs, *self_closing);
                    self.switch_to_script_data();
                }
                "style" => {
                    self.parse_errors += 1;
                    self.insert(name, attrs, *self_closing);
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
                    // TODO: TREE_ACTION_INSERT_FOSTER
                }
                "tfoot" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "th" => {
                    self.parse_errors += 1;
                    // TODO: TREE_ACTION_INSERT_FOSTER
                }
                "thead" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tr" => {
                    // TODO: TREE_ACTION_INSERT_FOSTER
                }
                _ => { self.insert(name, attrs, *self_closing); }
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
                _ => { self.pop_until(name); }
                }
            }
            Token::Character(text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InTableText;
                self.handle_token(token);
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
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
                    self.insert(name, attrs, *self_closing);
                    self.switch_to_script_data();
                }
                "style" => {
                    self.parse_errors += 1;
                    self.insert(name, attrs, *self_closing);
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
                    self.insert(name, attrs, *self_closing);
                }
                "tfoot" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "th" => {
                    self.insert(name, attrs, *self_closing);
                }
                "thead" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                "tr" => {
                    self.parse_errors += 1;
                    // ignore token
                }
                _ => { self.insert(name, attrs, *self_closing); }
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
                _ => { self.pop_until(name); }
                }
            }
            Token::Character(text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InTableText;
                self.handle_token(token);
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
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
                    self.insert(name, attrs, *self_closing);
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
                _ => { self.pop_until(name); }
                }
            }
            Token::Character(text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(text.clone())); }
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
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
                    self.insert(name, attrs, *self_closing);
                }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "caption" => {
                    self.pop_until("caption");
                }
                _ => { self.pop_until(name); }
                }
            }
            Token::Character(text) => {
                if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(text.clone())); }
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
                match &name[..] {
                "col" => {
                    self.insert(name, attrs, *self_closing);
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
                _ => { self.pop_until(name); }
                }
            }
            Token::Character(text) => {
                // parse error, ignore character
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
                self.insert(name, attrs, *self_closing);
            }
            Token::EndTag { name } => {
                self.pop_until(name);
            }
            Token::Character(text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InBody;
                self.handle_token(token);
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
                match &name[..] {
                "frameset" => {
                    self.insert(name, attrs, *self_closing);
                }
                "html" => {
                    self.insert(name, attrs, *self_closing);
                }
                _ => { self.insert(name, attrs, *self_closing); }
                }
            }
            Token::EndTag { name } => {
                match &name[..] {
                "frameset" => {
                    self.pop_until("frameset");
                }
                _ => { self.pop_until(name); }
                }
            }
            Token::Character(text) => {
                // no character rule defined
            }
            Token::Comment(text) => {
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
            Token::StartTag { name, attrs, self_closing } => {
                self.insert(name, attrs, *self_closing);
            }
            Token::EndTag { name } => {
                self.pop_until(name);
            }
            Token::Character(text) => {
                // no character rule defined
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
                match &name[..] {
                "html" => {
                    self.insert(name, attrs, *self_closing);
                }
                _ => { self.insert(name, attrs, *self_closing); }
                }
            }
            Token::EndTag { name } => {
                self.pop_until(name);
            }
            Token::Character(text) => {
                // Reprocess character in next mode
                self.insertion_mode = InsertionMode::InBody;
                self.handle_token(token);
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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
            Token::StartTag { name, attrs, self_closing } => {
                self.insert(name, attrs, *self_closing);
            }
            Token::EndTag { name } => {
                self.pop_until(name);
            }
            Token::Character(text) => {
                // no character rule defined
            }
            Token::Comment(text) => {
                if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
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



    /// Fallback handler for unimplemented insertion modes.
    fn handle_fallback(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs, self_closing } => {
                self.insert(name, attrs, *self_closing);
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

    /// Action: INSERT — create element and push to stack.
    fn insert(&mut self, name: &str, _attrs: &BTreeMap<String, String>, _self_closing: bool) {
        let mut elem = Element::new(name);
        // Void elements are not pushed to the open elements stack
        if !_self_closing && !VOID_ELEMENTS.contains(&name) {
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
}
