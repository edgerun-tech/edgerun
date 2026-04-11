package htmlcodegen

import (
	"encoding/json"
	"fmt"
	"sort"
	"strings"

	"edgerun-reference-core/gen/go/edgerun/v0/html"
)

// GenerateTokenizerRust produces the full tokenizer.rs source file.
func GenerateTokenizerRust(machine *html.TokenizerStateMachine, metadata map[string][]string) string {
	voidElements := metadata["void_elements"]
	rawTextElements := metadata["raw_text_elements"]

	// Group transitions by current_state
	type stateTrans struct {
		state  html.TokenizerState
		trans  []*html.StateTransition
	}
	var stateMap = make(map[html.TokenizerState][]*html.StateTransition)
	for _, t := range machine.Transitions {
		stateMap[t.CurrentState] = append(stateMap[t.CurrentState], t)
	}

	usedStates := make([]html.TokenizerState, 0, len(stateMap))
	for s := range stateMap {
		usedStates = append(usedStates, s)
	}
	sort.Slice(usedStates, func(i, j int) bool { return usedStates[i] < usedStates[j] })

	// Build state match arms
	var stateMatchArms []string
	for _, state := range usedStates {
		rustState := TokenizerStateToRust(state)
		transitions := stateMap[state]

		// Build char groups for this state
		type charArm struct {
			cc         int32
			pattern    string
			nextState  string
			actionCode string
		}
		var arms []charArm
		for _, tr := range transitions {
			nextStateName := TokenizerStateToRust(tr.NextState)
			actionLines := actionLinesToRust(tr.Actions)
			arms = append(arms, charArm{
				cc:         int32(tr.CharClass),
				pattern:    CharClassPattern(int32(tr.CharClass)),
				nextState:  nextStateName,
				actionCode: strings.Join(actionLines, "\n                    "),
			})
		}

		// Sort arms: specific first, OTHER last
		sort.SliceStable(arms, func(i, j int) bool {
			return CcSortKey(arms[i].cc) < CcSortKey(arms[j].cc)
		})

		// Deduplicate patterns (skip unreachable)
		var deduped []charArm
		seen := make(map[string]bool)
		for _, a := range arms {
			if a.cc == 1 { // EOF
				continue
			}
			if seen[a.pattern] {
				continue
			}
			seen[a.pattern] = true
			deduped = append(deduped, a)
		}

		var armLines []string
		for _, a := range deduped {
			armLines = append(armLines,
				fmt.Sprintf("                %s => {\n                    self.state = State::%s;\n                    %s\n                }",
					a.pattern, a.nextState, a.actionCode))
		}

		if len(armLines) == 0 {
			stateMatchArms = append(stateMatchArms,
				fmt.Sprintf("            State::%s => {\n                // No transitions defined — stays in state\n            }", rustState))
		} else {
			stateMatchArms = append(stateMatchArms,
				fmt.Sprintf("            State::%s => {\n                match c {\n%s\n                }\n            }",
					rustState, strings.Join(armLines, "\n")))
		}
	}

	armsStr := strings.Join(stateMatchArms, "\n\n")

	// Build State enum from used states
	var stateVariants []string
	for _, s := range usedStates {
		stateVariants = append(stateVariants, TokenizerStateToRust(s))
	}
	stateEnum := strings.Join(stateVariants, ",\n    ")

	totalTransitions := len(machine.Transitions)

	return fmt.Sprintf(`// DO NOT EDIT.
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
    %s,
}

/// WHATWG §13.2.5 HTML tokenizer — table-driven state machine.
///
/// Generated from proto IR with %d transitions across %d states.
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
    /// DOCTYPE force-quirks flag — set when the DOCTYPE is malformed.
    doctype_force_quirks: bool,
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
            doctype_force_quirks: false,
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
%s
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

    #[allow(dead_code)]
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

    /// WHATWG §13.2.5.69 — Flush character reference.
    ///
    /// After a character reference has been accumulated in the text_buffer,
    /// look it up in the entity table and emit the decoded code point(s).
    /// If not found, emit the literal characters as-is.
    fn flush_char_ref(&mut self) {
        let entity_name = self.text_buffer.clone();
        self.text_buffer.clear();

        if let Some((cp1, cp2)) = crate::entity_decoder::lookup_entity(&entity_name) {
            // Entity found — emit decoded code point(s)
            if let Some(ch) = char::from_u32(cp1) {
                self.text_buffer.push(ch);
            }
            if cp2 != 0 {
                if let Some(ch) = char::from_u32(cp2) {
                    self.text_buffer.push(ch);
                }
            }
        } else if entity_name.starts_with('#') {
            // Numeric character reference: &#NNNN; or &#xHHHH;
            if let Some(cp) = Self::parse_numeric_char_ref(&entity_name) {
                if let Some(ch) = char::from_u32(cp) {
                    self.text_buffer.push(ch);
                } else {
                    self.text_buffer.push('\u{FFFD}');
                }
            } else {
                self.text_buffer.push('\u{FFFD}');
            }
        } else {
            // Unknown named entity — emit original characters literally
            self.text_buffer.push('&');
            self.text_buffer.push_str(&entity_name);
        }
    }

    /// Parse a numeric character reference from the text_buffer content.
    /// Input is like "#x3C", "#60", "x3c", etc. (without the leading '&').
    fn parse_numeric_char_ref(s: &str) -> Option<u32> {
        let s = s.trim_start_matches('#');
        let (is_hex, digits) = if let Some(rest) = s.strip_prefix(['x', 'X']) {
            (true, rest)
        } else {
            (false, s)
        };
        u32::from_str_radix(digits, if is_hex { 16 } else { 10 }).ok()
    }

    /// WHATWG §13.2.5.46 — Check if temp buffer is "script".
    ///
    /// After </ in script data double escape start, the temp buffer contains
    /// the tag name. If it is "script", enter double-escaped mode.
    fn check_temp_buffer_is_script(&mut self, double_escaped_state: State) {
        if self.temp_buffer.eq_ignore_ascii_case("script") {
            self.state = double_escaped_state;
        } else {
            self.state = State::ScriptDataEscaped;
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
pub const RAW_TEXT_ELEMENTS: &[&str] = &%s;

/// Void element tags that never have children.
pub const VOID_ELEMENTS: &[&str] = &%s;
`,
		stateEnum,
		totalTransitions, len(usedStates),
		armsStr,
		mustJSONSlice(rawTextElements),
		mustJSONSlice(voidElements),
	)
}

// actionLinesToRust converts a slice of TransitionAction to Rust code lines.
func actionLinesToRust(actions []*html.TransitionAction) []string {
	var lines []string
	for _, a := range actions {
		rust := transitionActionToRust(a)
		if rust != "" {
			lines = append(lines, rust)
		}
	}
	return lines
}

// transitionActionToRust converts a single TransitionAction to Rust code.
func transitionActionToRust(a *html.TransitionAction) string {
	switch {
	case a.GetEmitCurrentChar():
		return "self.emit_char();"
	case a.GetEmitReplacementChar():
		return "self.emit_replacement();"
	case a.GetEmitNullToken():
		return "self.emit_null();"
	case a.GetEmitEofToken():
		return "self.done = true;"
	case a.GetEmitTokenType() != html.TokenType_TOKEN_TYPE_UNSPECIFIED:
		tt := a.GetEmitTokenType()
		return fmt.Sprintf("self.emit_token(TokenType::%s);", tokenTypeToRust(tt))
	case a.GetReconsume():
		return "self.pos -= 1;  // reconsume"
	case a.GetErrorUnexpectedNull():
		return "self.parse_errors += 1;"
	case a.GetErrorUnexpectedChar():
		return "self.parse_errors += 1;"
	case a.GetErrorMissingSemicolon():
		return "self.parse_errors += 1;"
	case a.GetAppendCurrentToTokenData():
		return "self.text_buffer.push(c);"
	case a.GetAppendCurrentToTagName():
		return "self.append_tag_name(c);"
	case a.GetAppendCurrentToAttrName():
		return "self.append_attr_name(c);"
	case a.GetAppendCurrentToAttrValue():
		return "self.append_attr_value(c);"
	case a.GetAppendCurrentToTempBuffer():
		return "self.temp_buffer.push(c);"
	case a.GetAppendReplacementToTokenData():
		return "self.text_buffer.push('\\u{FFFD}');"
	case a.GetSetSelfClosingFlag():
		return "self.current_token_is_self_closing = true;"
	case a.GetSetReturnState() != html.TokenizerState_TOKENIZER_STATE_UNSPECIFIED:
		// Emit code to store the return state for after character reference decoding.
		nextState := TokenizerStateToRust(a.GetSetReturnState())
		return fmt.Sprintf("self.return_state = State::%s;", nextState)
	case a.GetSwitchTokenizerState() != html.TokenizerState_TOKENIZER_STATE_UNSPECIFIED:
		nextState := TokenizerStateToRust(a.GetSwitchTokenizerState())
		return fmt.Sprintf("self.state = State::%s;", nextState)
	case a.GetFlushCharRef():
		return "self.flush_char_ref();"
	case a.GetCreateStartTagToken():
		return "self.start_tag();"
	case a.GetCreateEndTagToken():
		return "self.end_tag();"
	case a.GetCreateCommentToken():
		return "self.create_comment();"
	case a.GetCreateDoctypeToken():
		return "self.create_doctype();"
	case a.GetSetDoctypeForceQuirks():
		return "self.doctype_force_quirks = true;"
	case a.GetCheckAppropriateEndTag():
		return "self.check_appropriate_end_tag();"
	case a.GetCheckTempBufferIsScript():
		return "self.check_temp_buffer_is_script(State::ScriptDataDoubleEscaped);"
	default:
		return "// TODO: unknown action"
	}
}

func tokenTypeToRust(tt html.TokenType) string {
	switch tt {
	case html.TokenType_TOKEN_TYPE_START_TAG:
		return "StartTag"
	case html.TokenType_TOKEN_TYPE_END_TAG:
		return "EndTag"
	case html.TokenType_TOKEN_TYPE_CHARACTER:
		return "Character"
	case html.TokenType_TOKEN_TYPE_COMMENT:
		return "Comment"
	case html.TokenType_TOKEN_TYPE_DOCTYPE:
		return "Doctype"
	case html.TokenType_TOKEN_TYPE_EOF:
		return "Eof"
	case html.TokenType_TOKEN_TYPE_NULL:
		return "Null"
	default:
		return "Character"
	}
}

// TokenizerStateToRust converts a proto TokenizerState to a Rust variant name.
func TokenizerStateToRust(s html.TokenizerState) string {
	name := html.TokenizerState_name[int32(s)]
	name = strings.TrimPrefix(name, "TOKENIZER_STATE_")
	name = strings.TrimSuffix(name, "_STATE")
	parts := strings.Split(name, "_")
	var result string
	for _, p := range parts {
		if p == "" {
			continue
		}
		result += strings.Title(strings.ToLower(p))
	}
	return result
}

// mustJSONSlice formats a []string as a Rust slice literal.
func mustJSONSlice(items []string) string {
	data, err := json.Marshal(items)
	if err != nil {
		panic(err)
	}
	return string(data)
}
