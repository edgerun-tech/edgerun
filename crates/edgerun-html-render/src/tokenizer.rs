// DO NOT EDIT.
// Auto-generated from Parser IR by cmd/html-codegen
// Regenerate: go run ./cmd/html-codegen
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// Token types emitted by the tokenizer.
#[derive(Debug, Clone)]
pub enum Token {
    StartTag { name: String, attrs: BTreeMap<String, String>, self_closing: bool },
    EndTag { name: String },
    Character(String),
    Comment(String),
    Doctype,
    Eof,
}

#[allow(dead_code)]
/// Tokenizer states from WHATWG §13.2.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Data,
    Rcdata,
    Rawtext,
    ScriptData,
    Plaintext,
    TagOpen,
    EndTagOpen,
    TagName,
    RcdataLessThanSign,
    RcdataEndTagOpen,
    RcdataEndTagName,
    RcdataEndTagNameStateAfter,
    RawtextLessThanSign,
    RawtextEndTagOpen,
    RawtextEndTagName,
    RawtextEndTagNameStateAfter,
    ScriptDataLessThanSign,
    ScriptDataEndTagOpen,
    ScriptDataEndTagName,
    ScriptDataEndTagNameStateAfter,
    ScriptDataEscapeStart,
    ScriptDataEscapeStartDash,
    ScriptDataEscaped,
    ScriptDataEscapedDash,
    ScriptDataEscapedDashDash,
    ScriptDataEscapedLessThanSign,
    ScriptDataEscapedEndTagOpen,
    ScriptDataEscapedEndTagName,
    ScriptDataEscapedEndTagNameStateAfter,
    ScriptDataDoubleEscapeStart,
    ScriptDataDoubleEscaped,
    ScriptDataDoubleEscapedDash,
    ScriptDataDoubleEscapedDashDash,
    ScriptDataDoubleEscapedLessThanSign,
    ScriptDataDoubleEscapeEnd,
    BeforeTagName,
    AfterTagName,
    SelfClosingStartTag,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    CharacterReference,
    NamedCharacterReference,
    AmbiguousAmpersand,
    NumericCharacterReference,
    HexademicalCharacterReference,
    DecimalCharacterReference,
    NumericCharacterReferenceEnd,
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
    AfterDoctypePublicKeyword,
    BeforeDoctypePublicIdentifier,
    DoctypePublicIdentifierDoubleQuoted,
    DoctypePublicIdentifierSingleQuoted,
    AfterDoctypePublicIdentifier,
    BetweenDoctypePublicAndSystemIdentifiers,
    AfterDoctypeSystemKeyword,
    BeforeDoctypeSystemIdentifier,
    DoctypeSystemIdentifierDoubleQuoted,
    DoctypeSystemIdentifierSingleQuoted,
    BogusDoctype,
    MarkupDeclarationOpen,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentLessThanSign,
    CommentLessThanSignBang,
    CommentLessThanSignBangDash,
    CommentLessThanSignBangDashDash,
    CommentEndDash,
    CommentEnd,
    CommentEndBang,
    BogusComment,
    CdataSection,
    CdataSectionBracket,
    CdataSectionEnd,
}

/// WHATWG §13.2.5 HTML tokenizer — table-driven state machine.
///
/// Generated from proto IR with 1168 transitions across 83 states.
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
    temp_buffer: String,
    /// Tag name to look for when in raw text / RCDATA / script data mode.
    raw_text_end_tag: String,
    /// The raw text state we entered (Rawtext, Rcdata, or ScriptData).
    raw_text_state: State,
    /// State to return to after character reference decoding.
    return_state: State,
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
            temp_buffer: String::new(),
            raw_text_end_tag: String::new(),
            raw_text_state: State::Data,
            return_state: State::Data,
        }
    }

    /// Run the tokenizer to completion, returning all tokens.
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(token) = self.step() { tokens.push(token); }
        tokens
    }

    /// Step the state machine until a token is emitted or we're done.
    pub fn step(&mut self) -> Option<Token> {
        loop {
            if let Some(t) = self.pending_token.take() { return Some(t); }

            if self.pos >= self.input.len() {
                if !self.done {
                    if !self.text_buffer.is_empty() {
                        let t = Token::Character(self.text_buffer.clone());
                        self.text_buffer.clear(); return Some(t);
                    }
                    self.done = true; return Some(Token::Eof);
                }
                return None;
            }

            let c = self.input[self.pos];

            // Flush buffered text before processing '<' — applies to Data and raw text states.
            // In raw text modes, '<' may start an end tag, so buffered text must be emitted first.
            if !self.text_buffer.is_empty() && c == '<' {
                let t = Token::Character(self.text_buffer.clone());
                self.text_buffer.clear(); return Some(t);
            }
            self.pos += 1;

            match self.state {
            State::Data => {
                match c {
                'a'..='z' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '<' => {
                    self.state = State::TagOpen;
                    
                }
                '&' => {
                    self.state = State::CharacterReference;
                    self.return_state = State::Data;
                }
                '\0' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    self.emit_replacement();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::Data;
                    self.emit_char();
                }
                _ => {
                    self.state = State::Data;
                    self.emit_char();
                }
                }
            }

            State::Rcdata => {
                match c {
                'a'..='z' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                '&' => {
                    self.state = State::CharacterReference;
                    self.return_state = State::Rcdata;
                }
                '<' => {
                    self.state = State::RcdataLessThanSign;
                    
                }
                '\0' => {
                    self.state = State::Rcdata;
                    self.parse_errors += 1;
                    self.emit_replacement();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                _ => {
                    self.state = State::Rcdata;
                    self.emit_char();
                }
                }
            }

            State::Rawtext => {
                match c {
                'a'..='z' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                '<' => {
                    self.state = State::RawtextLessThanSign;
                    
                }
                '\0' => {
                    self.state = State::Rawtext;
                    self.parse_errors += 1;
                    self.emit_replacement();
                }
                '&' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                _ => {
                    self.state = State::Rawtext;
                    self.emit_char();
                }
                }
            }

            State::ScriptData => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '<' => {
                    self.state = State::ScriptDataLessThanSign;
                    
                }
                '\0' => {
                    self.state = State::ScriptData;
                    self.parse_errors += 1;
                    self.emit_replacement();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                _ => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                }
            }

            State::Plaintext => {
                match c {
                'a'..='z' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                '\0' => {
                    self.state = State::Plaintext;
                    self.parse_errors += 1;
                    self.emit_replacement();
                }
                '&' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                '<' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                _ => {
                    self.state = State::Plaintext;
                    self.emit_char();
                }
                }
            }

            State::TagOpen => {
                match c {
                'a'..='z' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                'A'..='Z' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                '!' => {
                    self.state = State::MarkupDeclarationOpen;
                    
                }
                '/' => {
                    self.state = State::EndTagOpen;
                    
                }
                '?' => {
                    self.state = State::BogusComment;
                    self.parse_errors += 1;
                    self.create_comment();
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
                'a'..='z' => {
                    self.state = State::TagName;
                    self.end_tag();
                    self.append_tag_name(c);
                }
                'A'..='Z' => {
                    self.state = State::TagName;
                    self.end_tag();
                    self.append_tag_name(c);
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                }
                _ => {
                    self.state = State::BogusComment;
                    self.parse_errors += 1;
                    self.create_comment();
                }
                }
            }

            State::TagName => {
                match c {
                'a'..='z' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                'A'..='Z' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                '0'..='9' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                '-' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeName;
                    
                }
                '/' => {
                    self.state = State::AfterTagName;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_token(TokenType::StartTag);
                }
                '\0' => {
                    self.state = State::TagName;
                    self.parse_errors += 1;
                    self.append_tag_name(c);
                }
                '"' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                '\'' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                '=' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                '?' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                ';' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                '!' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                ']' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                '%' => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                _ => {
                    self.state = State::TagName;
                    self.append_tag_name(c);
                }
                }
            }

            State::RcdataLessThanSign => {
                match c {
                '/' => {
                    self.state = State::RcdataEndTagOpen;
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::Rcdata;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::RcdataEndTagOpen => {
                match c {
                'a'..='z' => {
                    self.state = State::RcdataEndTagName;
                    self.end_tag();
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::RcdataEndTagName;
                    self.end_tag();
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::Rcdata;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::RcdataEndTagName => {
                match c {
                'a'..='z' => {
                    self.state = State::RcdataEndTagName;
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::RcdataEndTagName;
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::RcdataEndTagNameStateAfter;
                    self.temp_buffer.push(c);
                    self.check_appropriate_end_tag();
                }
                '/' => {
                    self.state = State::RcdataEndTagNameStateAfter;
                    self.temp_buffer.push(c);
                    self.check_appropriate_end_tag();
                }
                '>' => {
                    self.state = State::Rcdata;
                    self.check_appropriate_end_tag();
                }
                _ => {
                    self.state = State::Rcdata;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::RcdataEndTagNameStateAfter => {
                match c {
                'a'..='z' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                '/' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                '>' => {
                    self.state = State::Rcdata;
                    self.emit_token(TokenType::StartTag);
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::Rcdata;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::RawtextLessThanSign => {
                match c {
                '/' => {
                    self.state = State::RawtextEndTagOpen;
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::Rawtext;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::RawtextEndTagOpen => {
                match c {
                'a'..='z' => {
                    self.state = State::RawtextEndTagName;
                    self.end_tag();
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::RawtextEndTagName;
                    self.end_tag();
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::Rawtext;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::RawtextEndTagName => {
                match c {
                'a'..='z' => {
                    self.state = State::RawtextEndTagName;
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::RawtextEndTagName;
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::RawtextEndTagNameStateAfter;
                    self.temp_buffer.push(c);
                    self.check_appropriate_end_tag();
                }
                '/' => {
                    self.state = State::RawtextEndTagNameStateAfter;
                    self.temp_buffer.push(c);
                    self.check_appropriate_end_tag();
                }
                '>' => {
                    self.state = State::Rawtext;
                    self.check_appropriate_end_tag();
                }
                _ => {
                    self.state = State::Rawtext;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::RawtextEndTagNameStateAfter => {
                match c {
                'a'..='z' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                '/' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                '>' => {
                    self.state = State::Rawtext;
                    self.emit_token(TokenType::StartTag);
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::Rawtext;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::ScriptDataLessThanSign => {
                match c {
                '/' => {
                    self.state = State::ScriptDataEndTagOpen;
                    self.temp_buffer.push(c);
                }
                '!' => {
                    self.state = State::ScriptDataEscapeStart;
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::ScriptData;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::ScriptDataEndTagOpen => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataEndTagName;
                    self.end_tag();
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataEndTagName;
                    self.end_tag();
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::ScriptData;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::ScriptDataEndTagName => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataEndTagName;
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataEndTagName;
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::ScriptDataEndTagNameStateAfter;
                    self.temp_buffer.push(c);
                    self.check_appropriate_end_tag();
                }
                '/' => {
                    self.state = State::ScriptDataEndTagNameStateAfter;
                    self.temp_buffer.push(c);
                    self.check_appropriate_end_tag();
                }
                '>' => {
                    self.state = State::ScriptData;
                    self.check_appropriate_end_tag();
                }
                _ => {
                    self.state = State::ScriptData;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::ScriptDataEndTagNameStateAfter => {
                match c {
                'a'..='z' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                '/' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                '>' => {
                    self.state = State::ScriptData;
                    self.emit_token(TokenType::StartTag);
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::ScriptData;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::ScriptDataEscapeStart => {
                match c {
                '-' => {
                    self.state = State::ScriptDataEscapeStartDash;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '<' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::ScriptData;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::ScriptDataEscapeStartDash => {
                match c {
                '-' => {
                    self.state = State::ScriptDataEscapedDashDash;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '<' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::ScriptData;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::ScriptDataEscaped => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '0'..='9' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '-' => {
                    self.state = State::ScriptDataEscapedDash;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '<' => {
                    self.state = State::ScriptDataEscapedLessThanSign;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '\0' => {
                    self.state = State::ScriptDataEscaped;
                    self.parse_errors += 1;
                    self.emit_replacement();
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '"' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '\'' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '/' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '>' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '=' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '?' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                ';' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '!' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                ']' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '%' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                }
            }

            State::ScriptDataEscapedDash => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '0'..='9' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '-' => {
                    self.state = State::ScriptDataEscapedDashDash;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '<' => {
                    self.state = State::ScriptDataEscapedLessThanSign;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '\0' => {
                    self.state = State::ScriptDataEscaped;
                    self.parse_errors += 1;
                    self.emit_replacement();
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '"' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '\'' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '/' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '>' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '=' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '?' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                ';' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '!' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                ']' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '%' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                }
            }

            State::ScriptDataEscapedDashDash => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '0'..='9' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '-' => {
                    self.state = State::ScriptDataEscapedDashDash;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '<' => {
                    self.state = State::ScriptDataEscapedLessThanSign;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '>' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '\0' => {
                    self.state = State::ScriptDataEscaped;
                    self.parse_errors += 1;
                    self.emit_replacement();
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '"' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '\'' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '/' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '&' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '=' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '?' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                ';' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '!' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                ']' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '%' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                }
            }

            State::ScriptDataEscapedLessThanSign => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataDoubleEscapeStart;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataDoubleEscapeStart;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '/' => {
                    self.state = State::ScriptDataEscapedEndTagOpen;
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::ScriptDataEscaped;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::ScriptDataEscapedEndTagOpen => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataEscapedEndTagName;
                    self.end_tag();
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataEscapedEndTagName;
                    self.end_tag();
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::ScriptDataEscaped;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::ScriptDataEscapedEndTagName => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataEscapedEndTagName;
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataEscapedEndTagName;
                    self.append_tag_name(c);
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::ScriptDataEscapedEndTagNameStateAfter;
                    self.temp_buffer.push(c);
                    self.check_appropriate_end_tag();
                }
                '/' => {
                    self.state = State::ScriptDataEscapedEndTagNameStateAfter;
                    self.temp_buffer.push(c);
                    self.check_appropriate_end_tag();
                }
                '>' => {
                    self.state = State::ScriptData;
                    self.check_appropriate_end_tag();
                }
                _ => {
                    self.state = State::ScriptDataEscaped;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::ScriptDataEscapedEndTagNameStateAfter => {
                match c {
                'a'..='z' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                '/' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                '>' => {
                    self.state = State::ScriptData;
                    self.emit_token(TokenType::StartTag);
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeName;
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::ScriptDataEscaped;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::ScriptDataDoubleEscapeStart => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataDoubleEscapeStart;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataDoubleEscapeStart;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                    // check_temp_buffer_is_script (TODO)
                }
                '/' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                    // check_temp_buffer_is_script (TODO)
                }
                '>' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                    // check_temp_buffer_is_script (TODO)
                }
                _ => {
                    self.state = State::ScriptDataEscaped;
                    self.pos -= 1;  // reconsume
                }
                }
            }

            State::ScriptDataDoubleEscaped => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::ScriptDataDoubleEscapedDash;
                    self.emit_char();
                }
                '<' => {
                    self.state = State::ScriptDataDoubleEscapedLessThanSign;
                    self.emit_char();
                }
                '\0' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.parse_errors += 1;
                    self.emit_replacement();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                _ => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                }
            }

            State::ScriptDataDoubleEscapedDash => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::ScriptDataDoubleEscapedDashDash;
                    self.emit_char();
                }
                '<' => {
                    self.state = State::ScriptDataDoubleEscapedLessThanSign;
                    self.emit_char();
                }
                '\0' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.parse_errors += 1;
                    self.emit_replacement();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                _ => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                }
            }

            State::ScriptDataDoubleEscapedDashDash => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::ScriptDataDoubleEscapedDashDash;
                    self.emit_char();
                }
                '<' => {
                    self.state = State::ScriptDataDoubleEscapedLessThanSign;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::ScriptData;
                    self.emit_char();
                }
                '\0' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.parse_errors += 1;
                    self.emit_replacement();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '&' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                _ => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.emit_char();
                }
                }
            }

            State::ScriptDataDoubleEscapedLessThanSign => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataDoubleEscapeEnd;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataDoubleEscapeEnd;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                '/' => {
                    self.state = State::ScriptDataDoubleEscapeEnd;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                _ => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::ScriptDataDoubleEscapeEnd => {
                match c {
                'a'..='z' => {
                    self.state = State::ScriptDataDoubleEscapeEnd;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::ScriptDataDoubleEscapeEnd;
                    self.emit_char();
                    self.temp_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                    // check_temp_buffer_is_script (TODO)
                }
                '/' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                    // check_temp_buffer_is_script (TODO)
                }
                '>' => {
                    self.state = State::ScriptDataEscaped;
                    self.emit_char();
                    self.temp_buffer.push(c);
                    // check_temp_buffer_is_script (TODO)
                }
                _ => {
                    self.state = State::ScriptDataDoubleEscaped;
                    self.pos -= 1;  // reconsume
                }
                }
            }

            State::BeforeTagName => {
                match c {
                'a'..='z' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                'A'..='Z' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                '0'..='9' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                '-' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeTagName;
                    
                }
                '/' => {
                    self.state = State::SelfClosingStartTag;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    self.emit_token(TokenType::StartTag);
                }
                '"' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                '\'' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                '=' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                '?' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                '!' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                ']' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                '%' => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                '\0' => {
                    self.state = State::TagName;
                    self.parse_errors += 1;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                _ => {
                    self.state = State::TagName;
                    self.start_tag();
                    self.append_tag_name(c);
                }
                }
            }

            State::AfterTagName => {
                match c {
                'a'..='z' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                'A'..='Z' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '0'..='9' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '-' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::AfterTagName;
                    
                }
                '/' => {
                    self.state = State::SelfClosingStartTag;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_token(TokenType::StartTag);
                }
                '"' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '\'' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '=' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '\0' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '?' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '!' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                ']' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '%' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                _ => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                }
            }

            State::SelfClosingStartTag => {
                match c {
                'a'..='z' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                'A'..='Z' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '0'..='9' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '-' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '>' => {
                    self.state = State::Data;
                    self.current_token_is_self_closing = true;
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '"' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '\'' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '/' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '&' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '=' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '?' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                ';' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '!' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                ']' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '%' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '\0' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                _ => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                }
            }

            State::BeforeAttributeName => {
                match c {
                'a'..='z' => {
                    self.state = State::AttributeName;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                'A'..='Z' => {
                    self.state = State::AttributeName;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                '0'..='9' => {
                    self.state = State::AttributeName;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                '-' => {
                    self.state = State::AttributeName;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeName;
                    
                }
                '/' => {
                    self.state = State::SelfClosingStartTag;
                    
                }
                '=' => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_token(TokenType::StartTag);
                }
                '"' => {
                    self.state = State::AttributeName;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                '\'' => {
                    self.state = State::AttributeName;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                '\0' => {
                    self.state = State::AttributeName;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                '?' => {
                    self.state = State::AttributeName;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                '!' => {
                    self.state = State::AttributeName;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                ']' => {
                    self.state = State::AttributeName;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                '%' => {
                    self.state = State::AttributeName;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                _ => {
                    self.state = State::AttributeName;
                    self.start_tag();
                    self.append_attr_name(c);
                }
                }
            }

            State::AttributeName => {
                match c {
                'a'..='z' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                'A'..='Z' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '0'..='9' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '-' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::AfterAttributeName;
                    
                }
                '/' => {
                    self.state = State::AfterAttributeName;
                    
                }
                '=' => {
                    self.state = State::BeforeAttributeValue;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_token(TokenType::StartTag);
                }
                '"' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '\'' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '\0' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '?' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '!' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                ']' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                '%' => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                _ => {
                    self.state = State::AttributeName;
                    self.append_attr_name(c);
                }
                }
            }

            State::AfterAttributeName => {
                match c {
                'a'..='z' => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                'A'..='Z' => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '0'..='9' => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '-' => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::AfterAttributeName;
                    
                }
                '/' => {
                    self.state = State::SelfClosingStartTag;
                    
                }
                '=' => {
                    self.state = State::BeforeAttributeValue;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_token(TokenType::StartTag);
                }
                '"' => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '\'' => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '\0' => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '?' => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '!' => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                ']' => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '%' => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                _ => {
                    self.state = State::AttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                }
            }

            State::BeforeAttributeValue => {
                match c {
                'a'..='z' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                'A'..='Z' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '0'..='9' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '-' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeValue;
                    
                }
                '"' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    
                }
                '\'' => {
                    self.state = State::AttributeValueSingleQuoted;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    self.emit_token(TokenType::StartTag);
                }
                '=' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '?' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '!' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '%' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '\0' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '/' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                ';' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                ']' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                _ => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                }
            }

            State::AttributeValueDoubleQuoted => {
                match c {
                'a'..='z' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                'A'..='Z' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '0'..='9' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '-' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '"' => {
                    self.state = State::AfterAttributeName;
                    
                }
                '&' => {
                    self.state = State::CharacterReference;
                    self.return_state = State::AttributeValueDoubleQuoted;
                }
                '\0' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.parse_errors += 1;
                    self.append_attr_value(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '\'' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '/' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '<' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '>' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '=' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '?' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                ';' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '!' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                ']' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                '%' => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                _ => {
                    self.state = State::AttributeValueDoubleQuoted;
                    self.append_attr_value(c);
                }
                }
            }

            State::AttributeValueSingleQuoted => {
                match c {
                'a'..='z' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                'A'..='Z' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '0'..='9' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '-' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '\'' => {
                    self.state = State::AfterAttributeName;
                    
                }
                '&' => {
                    self.state = State::CharacterReference;
                    self.return_state = State::AttributeValueSingleQuoted;
                }
                '\0' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.parse_errors += 1;
                    self.append_attr_value(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '"' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '/' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '<' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '>' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '=' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '?' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                ';' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '!' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                ']' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                '%' => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                _ => {
                    self.state = State::AttributeValueSingleQuoted;
                    self.append_attr_value(c);
                }
                }
            }

            State::AttributeValueUnquoted => {
                match c {
                'a'..='z' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                'A'..='Z' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '0'..='9' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '-' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeAttributeName;
                    
                }
                '&' => {
                    self.state = State::CharacterReference;
                    self.return_state = State::AttributeValueUnquoted;
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_token(TokenType::StartTag);
                }
                '\0' => {
                    self.state = State::AttributeValueUnquoted;
                    self.parse_errors += 1;
                    self.append_attr_value(c);
                }
                '\'' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '/' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '=' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '?' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '!' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '%' => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                '"' => {
                    self.state = State::AttributeValueUnquoted;
                    self.parse_errors += 1;
                    self.append_attr_value(c);
                }
                ';' => {
                    self.state = State::AttributeValueUnquoted;
                    self.parse_errors += 1;
                    self.append_attr_value(c);
                }
                ']' => {
                    self.state = State::AttributeValueUnquoted;
                    self.parse_errors += 1;
                    self.append_attr_value(c);
                }
                _ => {
                    self.state = State::AttributeValueUnquoted;
                    self.append_attr_value(c);
                }
                }
            }

            State::AfterAttributeValueQuoted => {
                match c {
                'a'..='z' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                'A'..='Z' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '0'..='9' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '-' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::AfterAttributeName;
                    
                }
                '/' => {
                    self.state = State::SelfClosingStartTag;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_token(TokenType::StartTag);
                }
                '"' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '\'' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '=' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '?' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '!' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                ']' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '%' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                '\0' => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                _ => {
                    self.state = State::BeforeAttributeName;
                    self.parse_errors += 1;
                    self.append_attr_name(c);
                }
                }
            }

            State::CharacterReference => {
                match c {
                'a'..='z' => {
                    self.state = State::NamedCharacterReference;
                    self.text_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::NamedCharacterReference;
                    self.text_buffer.push(c);
                }
                '0'..='9' => {
                    self.state = State::NumericCharacterReference;
                    self.text_buffer.push(c);
                }
                _ => {
                    self.state = State::AmbiguousAmpersand;
                    
                }
                }
            }

            State::NamedCharacterReference => {
                match c {
                'a'..='z' => {
                    self.state = State::NamedCharacterReference;
                    self.text_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::NamedCharacterReference;
                    self.text_buffer.push(c);
                }
                '0'..='9' => {
                    self.state = State::NamedCharacterReference;
                    self.text_buffer.push(c);
                }
                '-' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                ';' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '=' => {
                    self.state = State::AmbiguousAmpersand;
                    
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '\0' => {
                    self.state = State::AmbiguousAmpersand;
                    
                }
                '"' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '\'' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '/' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '<' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '?' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '!' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                ']' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '%' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                _ => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                }
            }

            State::AmbiguousAmpersand => {
                match c {
                'a'..='z' => {
                    self.state = State::AmbiguousAmpersand;
                    self.text_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::AmbiguousAmpersand;
                    self.text_buffer.push(c);
                }
                '0'..='9' => {
                    self.state = State::AmbiguousAmpersand;
                    self.text_buffer.push(c);
                }
                '-' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                ';' => {
                    self.state = State::AmbiguousAmpersand;
                    // flush_char_ref (TODO)
                }
                '>' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '"' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '\'' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '/' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '<' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '=' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '?' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '!' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                ']' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '%' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '\0' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                _ => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                }
            }

            State::NumericCharacterReference => {
                match c {
                'a'..='z' => {
                    self.state = State::HexademicalCharacterReference;
                    self.text_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::HexademicalCharacterReference;
                    self.text_buffer.push(c);
                }
                '0'..='9' => {
                    self.state = State::DecimalCharacterReference;
                    self.text_buffer.push(c);
                }
                _ => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                }
            }

            State::HexademicalCharacterReference => {
                match c {
                'a'..='z' => {
                    self.state = State::HexademicalCharacterReference;
                    self.text_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::HexademicalCharacterReference;
                    self.text_buffer.push(c);
                }
                '0'..='9' => {
                    self.state = State::HexademicalCharacterReference;
                    self.text_buffer.push(c);
                }
                '-' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                ';' => {
                    self.state = State::NumericCharacterReferenceEnd;
                    
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '"' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '\'' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '/' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '<' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '=' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '?' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '!' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                ']' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '%' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '\0' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                _ => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                }
            }

            State::DecimalCharacterReference => {
                match c {
                'a'..='z' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                'A'..='Z' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '0'..='9' => {
                    self.state = State::DecimalCharacterReference;
                    self.text_buffer.push(c);
                }
                '-' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                ';' => {
                    self.state = State::NumericCharacterReferenceEnd;
                    
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '"' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '\'' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '/' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '<' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '=' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '?' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '!' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                ']' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '%' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                '\0' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                _ => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // flush_char_ref (TODO)
                }
                }
            }

            State::NumericCharacterReferenceEnd => {
                match c {
                'a'..='z' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                'A'..='Z' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '0'..='9' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '-' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '\0' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    self.emit_replacement();
                    // flush_char_ref (TODO)
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '"' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '\'' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '/' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '<' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '>' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '=' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '?' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                ';' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '!' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                ']' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                '%' => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                _ => {
                    self.state = State::Data;
                    // flush_char_ref (TODO)
                }
                }
            }

            State::Doctype => {
                match c {
                'a'..='z' => {
                    self.state = State::DoctypeName;
                    self.create_doctype();
                    self.text_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::DoctypeName;
                    self.create_doctype();
                    self.text_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeDoctypeName;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                    self.emit_token(TokenType::StartTag);
                }
                '\0' => {
                    self.state = State::DoctypeName;
                    self.parse_errors += 1;
                    self.create_doctype();
                    self.text_buffer.push(c);
                }
                _ => {
                    self.state = State::DoctypeName;
                    self.create_doctype();
                    self.text_buffer.push(c);
                }
                }
            }

            State::BeforeDoctypeName => {
                match c {
                'a'..='z' => {
                    self.state = State::DoctypeName;
                    self.text_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::DoctypeName;
                    self.text_buffer.push(c);
                }
                '-' => {
                    self.state = State::DoctypeName;
                    self.text_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeDoctypeName;
                    
                }
                '\0' => {
                    self.state = State::DoctypeName;
                    self.parse_errors += 1;
                    self.text_buffer.push(c);
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                    self.emit_token(TokenType::StartTag);
                }
                _ => {
                    self.state = State::DoctypeName;
                    self.text_buffer.push(c);
                }
                }
            }

            State::DoctypeName => {
                match c {
                'a'..='z' => {
                    self.state = State::DoctypeName;
                    self.text_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::DoctypeName;
                    self.text_buffer.push(c);
                }
                '-' => {
                    self.state = State::DoctypeName;
                    self.text_buffer.push(c);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::AfterDoctypeName;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_token(TokenType::StartTag);
                }
                '\0' => {
                    self.state = State::DoctypeName;
                    self.parse_errors += 1;
                    self.text_buffer.push(c);
                }
                _ => {
                    self.state = State::DoctypeName;
                    self.text_buffer.push(c);
                }
                }
            }

            State::AfterDoctypeName => {
                match c {
                'a'..='z' => {
                    self.state = State::AfterDoctypePublicKeyword;
                    self.text_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::AfterDoctypePublicKeyword;
                    self.text_buffer.push(c);
                }
                '0'..='9' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                '-' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::AfterDoctypeName;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_token(TokenType::StartTag);
                }
                '"' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                '\'' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                '/' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                '&' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                '<' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                '=' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                '?' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                ';' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                '!' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                ']' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                '%' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                '\0' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                _ => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                }
            }

            State::AfterDoctypePublicKeyword => {
                match c {
                'a'..='z' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                'A'..='Z' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                '\0' => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                _ => {
                    self.state = State::BogusDoctype;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                }
                }
            }

            State::BeforeDoctypePublicIdentifier => {
                match c {
                'a'..='z' => {
                    self.state = State::BeforeDoctypePublicIdentifier;
                    
                }
                'A'..='Z' => {
                    self.state = State::BeforeDoctypePublicIdentifier;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeDoctypePublicIdentifier;
                    
                }
                '\0' => {
                    self.state = State::BeforeDoctypePublicIdentifier;
                    
                }
                _ => {
                    self.state = State::BeforeDoctypePublicIdentifier;
                    
                }
                }
            }

            State::DoctypePublicIdentifierDoubleQuoted => {
                match c {
                'a'..='z' => {
                    self.state = State::DoctypePublicIdentifierDoubleQuoted;
                    
                }
                'A'..='Z' => {
                    self.state = State::DoctypePublicIdentifierDoubleQuoted;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::DoctypePublicIdentifierDoubleQuoted;
                    
                }
                '\0' => {
                    self.state = State::DoctypePublicIdentifierDoubleQuoted;
                    
                }
                _ => {
                    self.state = State::DoctypePublicIdentifierDoubleQuoted;
                    
                }
                }
            }

            State::DoctypePublicIdentifierSingleQuoted => {
                match c {
                'a'..='z' => {
                    self.state = State::DoctypePublicIdentifierSingleQuoted;
                    
                }
                'A'..='Z' => {
                    self.state = State::DoctypePublicIdentifierSingleQuoted;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::DoctypePublicIdentifierSingleQuoted;
                    
                }
                '\0' => {
                    self.state = State::DoctypePublicIdentifierSingleQuoted;
                    
                }
                _ => {
                    self.state = State::DoctypePublicIdentifierSingleQuoted;
                    
                }
                }
            }

            State::AfterDoctypePublicIdentifier => {
                match c {
                'a'..='z' => {
                    self.state = State::AfterDoctypePublicIdentifier;
                    
                }
                'A'..='Z' => {
                    self.state = State::AfterDoctypePublicIdentifier;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::AfterDoctypePublicIdentifier;
                    
                }
                '\0' => {
                    self.state = State::AfterDoctypePublicIdentifier;
                    
                }
                _ => {
                    self.state = State::AfterDoctypePublicIdentifier;
                    
                }
                }
            }

            State::BetweenDoctypePublicAndSystemIdentifiers => {
                match c {
                'a'..='z' => {
                    self.state = State::BetweenDoctypePublicAndSystemIdentifiers;
                    
                }
                'A'..='Z' => {
                    self.state = State::BetweenDoctypePublicAndSystemIdentifiers;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BetweenDoctypePublicAndSystemIdentifiers;
                    
                }
                '\0' => {
                    self.state = State::BetweenDoctypePublicAndSystemIdentifiers;
                    
                }
                _ => {
                    self.state = State::BetweenDoctypePublicAndSystemIdentifiers;
                    
                }
                }
            }

            State::AfterDoctypeSystemKeyword => {
                match c {
                'a'..='z' => {
                    self.state = State::AfterDoctypeSystemKeyword;
                    
                }
                'A'..='Z' => {
                    self.state = State::AfterDoctypeSystemKeyword;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::AfterDoctypeSystemKeyword;
                    
                }
                '\0' => {
                    self.state = State::AfterDoctypeSystemKeyword;
                    
                }
                _ => {
                    self.state = State::AfterDoctypeSystemKeyword;
                    
                }
                }
            }

            State::BeforeDoctypeSystemIdentifier => {
                match c {
                'a'..='z' => {
                    self.state = State::BeforeDoctypeSystemIdentifier;
                    
                }
                'A'..='Z' => {
                    self.state = State::BeforeDoctypeSystemIdentifier;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BeforeDoctypeSystemIdentifier;
                    
                }
                '\0' => {
                    self.state = State::BeforeDoctypeSystemIdentifier;
                    
                }
                _ => {
                    self.state = State::BeforeDoctypeSystemIdentifier;
                    
                }
                }
            }

            State::DoctypeSystemIdentifierDoubleQuoted => {
                match c {
                'a'..='z' => {
                    self.state = State::DoctypeSystemIdentifierDoubleQuoted;
                    
                }
                'A'..='Z' => {
                    self.state = State::DoctypeSystemIdentifierDoubleQuoted;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::DoctypeSystemIdentifierDoubleQuoted;
                    
                }
                '\0' => {
                    self.state = State::DoctypeSystemIdentifierDoubleQuoted;
                    
                }
                _ => {
                    self.state = State::DoctypeSystemIdentifierDoubleQuoted;
                    
                }
                }
            }

            State::DoctypeSystemIdentifierSingleQuoted => {
                match c {
                'a'..='z' => {
                    self.state = State::DoctypeSystemIdentifierSingleQuoted;
                    
                }
                'A'..='Z' => {
                    self.state = State::DoctypeSystemIdentifierSingleQuoted;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    // set_doctype_force_quirks (TODO)
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::DoctypeSystemIdentifierSingleQuoted;
                    
                }
                '\0' => {
                    self.state = State::DoctypeSystemIdentifierSingleQuoted;
                    
                }
                _ => {
                    self.state = State::DoctypeSystemIdentifierSingleQuoted;
                    
                }
                }
            }

            State::BogusDoctype => {
                match c {
                'a'..='z' => {
                    self.state = State::BogusDoctype;
                    
                }
                'A'..='Z' => {
                    self.state = State::BogusDoctype;
                    
                }
                '0'..='9' => {
                    self.state = State::BogusDoctype;
                    
                }
                '-' => {
                    self.state = State::BogusDoctype;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BogusDoctype;
                    
                }
                '"' => {
                    self.state = State::BogusDoctype;
                    
                }
                '\'' => {
                    self.state = State::BogusDoctype;
                    
                }
                '/' => {
                    self.state = State::BogusDoctype;
                    
                }
                '&' => {
                    self.state = State::BogusDoctype;
                    
                }
                '<' => {
                    self.state = State::BogusDoctype;
                    
                }
                '=' => {
                    self.state = State::BogusDoctype;
                    
                }
                '?' => {
                    self.state = State::BogusDoctype;
                    
                }
                ';' => {
                    self.state = State::BogusDoctype;
                    
                }
                '!' => {
                    self.state = State::BogusDoctype;
                    
                }
                ']' => {
                    self.state = State::BogusDoctype;
                    
                }
                '%' => {
                    self.state = State::BogusDoctype;
                    
                }
                '\0' => {
                    self.state = State::BogusDoctype;
                    
                }
                _ => {
                    self.state = State::BogusDoctype;
                    
                }
                }
            }

            State::MarkupDeclarationOpen => {
                match c {
                'a'..='z' => {
                    self.state = State::Doctype;
                    self.create_doctype();
                    self.text_buffer.push(c);
                }
                'A'..='Z' => {
                    self.state = State::Doctype;
                    self.create_doctype();
                    self.text_buffer.push(c);
                }
                '-' => {
                    self.state = State::CommentStart;
                    self.create_comment();
                    self.text_buffer.push(c);
                }
                ']' => {
                    self.state = State::CdataSection;
                    self.text_buffer.push(c);
                }
                _ => {
                    self.state = State::BogusComment;
                    self.parse_errors += 1;
                    self.create_comment();
                }
                }
            }

            State::CommentStart => {
                match c {
                'a'..='z' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::CommentStartDash;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    self.emit_token(TokenType::StartTag);
                }
                '\0' => {
                    self.state = State::Comment;
                    self.parse_errors += 1;
                    self.emit_replacement();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '&' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '<' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                _ => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                }
            }

            State::CommentStartDash => {
                match c {
                'a'..='z' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::CommentEnd;
                    
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    self.emit_token(TokenType::StartTag);
                }
                '\0' => {
                    self.state = State::Comment;
                    self.parse_errors += 1;
                    self.emit_replacement();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '&' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '<' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                _ => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                }
            }

            State::Comment => {
                match c {
                'a'..='z' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::CommentEndDash;
                    
                }
                '<' => {
                    self.state = State::CommentLessThanSign;
                    self.emit_char();
                }
                '\0' => {
                    self.state = State::Comment;
                    self.parse_errors += 1;
                    self.emit_replacement();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '&' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                _ => {
                    self.state = State::Comment;
                    self.emit_char();
                }
                }
            }

            State::CommentLessThanSign => {
                match c {
                '!' => {
                    self.state = State::CommentLessThanSignBang;
                    self.text_buffer.push(c);
                }
                '<' => {
                    self.state = State::CommentLessThanSign;
                    self.emit_char();
                }
                _ => {
                    self.state = State::Comment;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::CommentLessThanSignBang => {
                match c {
                '-' => {
                    self.state = State::CommentLessThanSignBangDash;
                    
                }
                _ => {
                    self.state = State::Comment;
                    self.pos -= 1;  // reconsume
                }
                }
            }

            State::CommentLessThanSignBangDash => {
                match c {
                '-' => {
                    self.state = State::CommentLessThanSignBangDashDash;
                    
                }
                _ => {
                    self.state = State::CommentEndDash;
                    self.pos -= 1;  // reconsume
                }
                }
            }

            State::CommentLessThanSignBangDashDash => {
                match c {
                '>' => {
                    self.state = State::CommentEnd;
                    
                }
                _ => {
                    self.state = State::CommentEnd;
                    self.pos -= 1;  // reconsume
                }
                }
            }

            State::CommentEndDash => {
                match c {
                '-' => {
                    self.state = State::CommentEnd;
                    
                }
                _ => {
                    self.state = State::Comment;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::CommentEnd => {
                match c {
                '-' => {
                    self.state = State::CommentEnd;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_token(TokenType::StartTag);
                }
                '!' => {
                    self.state = State::CommentEndBang;
                    
                }
                _ => {
                    self.state = State::Comment;
                    self.parse_errors += 1;
                    self.pos -= 1;  // reconsume
                }
                }
            }

            State::CommentEndBang => {
                match c {
                '-' => {
                    self.state = State::CommentEndDash;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::Data;
                    self.parse_errors += 1;
                    self.emit_token(TokenType::StartTag);
                }
                _ => {
                    self.state = State::Comment;
                    self.parse_errors += 1;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::BogusComment => {
                match c {
                'a'..='z' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::Data;
                    self.emit_token(TokenType::StartTag);
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '&' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '<' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                '\0' => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                _ => {
                    self.state = State::BogusComment;
                    self.emit_char();
                }
                }
            }

            State::CdataSection => {
                match c {
                'a'..='z' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                'A'..='Z' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '0'..='9' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '-' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                ']' => {
                    self.state = State::CdataSectionBracket;
                    self.emit_char();
                }
                ' ' | '\t' | '\n' | '\r' | '\x0C' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '"' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '\'' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '/' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '&' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '<' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '>' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '=' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '?' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                ';' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '!' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '%' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                '\0' => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                _ => {
                    self.state = State::CdataSection;
                    self.emit_char();
                }
                }
            }

            State::CdataSectionBracket => {
                match c {
                ']' => {
                    self.state = State::CdataSectionEnd;
                    self.emit_char();
                }
                _ => {
                    self.state = State::CdataSection;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                }
                }
            }

            State::CdataSectionEnd => {
                match c {
                '>' => {
                    self.state = State::Data;
                    
                }
                _ => {
                    self.state = State::CdataSection;
                    self.pos -= 1;  // reconsume
                    self.emit_char();
                    self.emit_char();
                    self.emit_char();
                }
                }
            }
            }
        }
    }

    fn emit_char(&mut self) {
        self.text_buffer.push(self.input[self.pos - 1]);
    }

    fn emit_replacement(&mut self) {
        self.text_buffer.push('\u{FFFD}');
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

    fn create_comment(&mut self) {
        self.current_attr_map.clear();
    }

    fn create_doctype(&mut self) {
        self.current_attr_map.clear();
    }

    fn emit_null(&mut self) {
        self.text_buffer.push('\u{0000}');
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

    fn emit_token(&mut self, _token_type: TokenType) {
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

    /// Set the tokenizer state — used when tree builder signals a mode switch
    /// for raw text elements (style, script, noscript, noframes, title, textarea).
    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }

    /// Set the tag name and state to look for when exiting a raw text / RCDATA / script element.
    /// Called by the parser loop when the tree builder signals a mode switch.
    pub fn set_raw_text_tag(&mut self, tag: &str, state: State) {
        self.raw_text_end_tag = tag.to_ascii_lowercase();
        self.raw_text_state = state;
    }

    /// Check whether the accumulated tag name matches the raw text end tag name.
    /// If matched: emit end tag token, clear buffers, switch to Data state.
    /// If not matched: switch back to raw_text_state, emit accumulated raw text, reconsume.
    fn check_appropriate_end_tag(&mut self) {
        if !self.raw_text_end_tag.is_empty() && self.current_tag_name.eq_ignore_ascii_case(&self.raw_text_end_tag) {
            // Matched — emit end tag and exit raw text mode
            self.pending_token = Some(Token::EndTag { name: self.current_tag_name.clone() });
            self.current_tag_name.clear();
            self.raw_text_end_tag.clear();
            self.temp_buffer.clear();
            self.state = State::Data;
        } else {
            // Not matched — emit everything as raw text and reconsume
            self.raw_text_end_tag.clear();
            let saved = self.temp_buffer.clone();
            self.temp_buffer.clear();
            self.current_tag_name.clear();
            self.state = self.raw_text_state;
            // Emit the raw text prefix (</tagname-so-far)
            self.text_buffer.push('<');
            self.text_buffer.push_str(&saved);
            self.pos -= 1;  // reconsume current character
        }
    }
}

#[derive(Debug, Clone)]
enum TokenType {
    StartTag,
    EndTag,
    Character,
    Comment,
    Doctype,
    Eof,
    Null,
}

/// Raw text element tags that switch the tokenizer to RAWTEXT mode.
pub const RAW_TEXT_ELEMENTS: &[&str] = &["script","style"];

/// Void element tags that never have children.
pub const VOID_ELEMENTS: &[&str] = &["area","base","br","col","embed","hr","img","input","link","meta","source","track","wbr"];
