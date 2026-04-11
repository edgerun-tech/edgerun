// DO NOT EDIT.
// Auto-generated from Parser IR by scripts/generate_html_parser.py
// Regenerate: python3 scripts/generate_parser_ir.py && python3 scripts/generate_html_parser.py

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

use crate::html_parser::{Node, Element};

/// Token types emitted by the tokenizer.
#[derive(Debug, Clone)]
pub enum Token {
    StartTag { name: String, attrs: BTreeMap<String, String>, self_closing: bool },
    EndTag { name: String },
    Character(String),
    Comment(String),
    Eof,
}

/// Tokenizer states from WHATWG §13.2.5.
#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    SelfClosingStartTag,
    Eof,
}

/// WHATWG §13.2.5 HTML tokenizer — table-driven state machine.
///
/// Generated from Parser IR with 73 transitions across 13 states.
/// Each input character maps to a char class, which indexes into the transition table.
pub struct Tokenizer {
    input: Vec<char>,
    pos: usize,
    state: State,
    current_tag_name: String,
    current_attr_name: String,
    current_attr_value: String,
    current_attr_map: BTreeMap<String, String>,
    current_token_is_self_closing: bool,
    text_buffer: String,
    pending_token: Option<Token>,
    is_start_tag: bool,
    parse_errors: usize,
    done: bool,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            state: State::Data,
            current_tag_name: String::new(),
            current_attr_name: String::new(),
            current_attr_value: String::new(),
            current_attr_map: BTreeMap::new(),
            current_token_is_self_closing: false,
            text_buffer: String::new(),
            pending_token: None,
            is_start_tag: false,
            parse_errors: 0,
            done: false,
        }
    }

    /// Run the tokenizer to completion, returning all tokens.
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(token) = self.step() {
            tokens.push(token);
        }
        tokens
    }

    /// Step the state machine until a token is emitted or we're done.
    /// Returns None only when fully finished (EOF emitted).
    fn step(&mut self) -> Option<Token> {
        loop {
            // Return any pending token from previous step
            if let Some(t) = self.pending_token.take() {
                return Some(t);
            }

            if self.pos >= self.input.len() {
                if !self.done {
                    // Flush buffered text
                    if !self.text_buffer.is_empty() {
                        let t = Token::Character(self.text_buffer.clone());
                        self.text_buffer.clear();
                        return Some(t);
                    }
                    self.done = true;
                    return Some(Token::Eof);
                }
                return None;
            }

            let c = self.input[self.pos];

            // When in DATA state and we see '<', flush buffered text first
            if self.state == State::Data && c == '<' && !self.text_buffer.is_empty() {
                let t = Token::Character(self.text_buffer.clone());
                self.text_buffer.clear();
                // Don't consume '<' yet — process it on next step
                return Some(t);
            }

            self.pos += 1;

            match self.state {
            State::Data => {
                match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                'a'..='z' | 'A'..='Z' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '&' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '<' => {
                    self.state = State::TagOpen;
                    
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                _ => {
                    self.state = State::Data;
                    self.emit_char();
                }
                }
            }

            State::TagOpen => {
                match c {
                'a'..='z' | 'A'..='Z' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                '/' => {
                    self.state = State::EndTagOpen;
                    
                }
                _ => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    self.emit_char();
                }
                }
            }

            State::EndTagOpen => {
                match c {
                'a'..='z' | 'A'..='Z' => {
                    self.state = State::TagName;
                    self.end_tag();
                    self.append_tag_name(c);
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                }
                _ => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    self.emit_char();
                }
                }
            }

            State::TagName => {
                match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_tag();
                }
                '\0' => {
                    self.state = State::TagName;
                    self.parse_errors += 1;
                    self.append_tag_name(c);
                }
                '/' => {
                    self.state = State::SelfClosingStartTag;
                    
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeName;
                    
                }
                _ => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                }
            }

            State::BeforeAttributeName => {
                match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '\'' => {
                    self.state = State::AttributeName;
                    
                }
                '=' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_tag();
                }
                '"' => {
                    self.state = State::AttributeName;
                    
                }
                '/' => {
                    self.state = State::SelfClosingStartTag;
                    
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeName;
                    
                }
                _ => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                }
            }

            State::AttributeName => {
                match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '\'' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '=' => {
                    self.state = State::BeforeAttributeValue;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_tag();
                }
                '"' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '/' => {
                    self.state = State::SelfClosingStartTag;
                    
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::AfterAttributeName;
                    
                }
                _ => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                }
            }

            State::AfterAttributeName => {
                match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '\'' => {
                    self.state = State::AttributeName;
                    
                }
                '=' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_tag();
                }
                '"' => {
                    self.state = State::AttributeName;
                    
                }
                '/' => {
                    self.state = State::SelfClosingStartTag;
                    
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::AfterAttributeName;
                    
                }
                _ => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                }
            }

            State::BeforeAttributeValue => {
                match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '\'' => {
                    self.state = State::AttributeValueSingleQuoted;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_tag();
                }
                '"' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeValue;
                    
                }
                _ => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                }
            }

            State::AttributeValueDoubleQuoted => {
                match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '&' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '"' => {
                    self.state = State::AfterAttributeName;
                    
                }
                _ => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                }
            }

            State::AttributeValueSingleQuoted => {
                match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '&' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '\'' => {
                    self.state = State::AfterAttributeName;
                    
                }
                _ => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                }
            }

            State::AttributeValueUnquoted => {
                match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '&' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_tag();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeName;
                    
                }
                _ => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                }
            }

            State::SelfClosingStartTag => {
                match c {
                '>' => {
                    self.state = State::Data;
                    self.current_token_is_self_closing = true;
                    self.emit_tag();
                }
                _ => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                }
                }
            }

            State::Eof => {
                // No transitions defined — stays in state
            }
            }
            // If no pending token was set, continue loop to next character
        }
    }

    fn emit_char(&mut self) {
        // Only emit non-whitespace or if we have content
        self.text_buffer.push(self.input[self.pos - 1]);
    }

    fn start_tag(&mut self) {
        self.current_tag_name.clear();
        self.current_attr_name.clear();
        self.current_attr_value.clear();
        self.current_attr_map.clear();
        self.current_token_is_self_closing = false;
        self.is_start_tag = true;
    }

    fn end_tag(&mut self) {
        self.current_tag_name.clear();
        self.is_start_tag = false;
    }

    fn append_tag_name(&mut self, c: char) {
        self.current_tag_name.push(c.to_ascii_lowercase());
    }

    fn append_attr_name(&mut self, c: char) {
        self.current_attr_name.push(c);
    }

    fn append_attr_value(&mut self, c: char) {
        self.current_attr_value.push(c);
    }

    fn emit_tag(&mut self) {
        let tag = self.current_tag_name.clone();
        let attrs = self.current_attr_map.clone();
        let self_closing = self.current_token_is_self_closing;

        if self.is_start_tag {
            self.pending_token = Some(Token::StartTag { name: tag, attrs, self_closing });
        } else {
            self.pending_token = Some(Token::EndTag { name: tag });
        }
    }

    pub fn parse_errors(&self) -> usize {
        self.parse_errors
    }
}

/// Raw text element tags that switch the tokenizer to RAWTEXT mode.
pub const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style"];

/// Void element tags that never have children.
pub const VOID_ELEMENTS: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];
