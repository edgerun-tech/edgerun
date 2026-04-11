package htmlcodegen

import (
	"fmt"
	"sort"
	"strings"

	"edgerun-reference-core/gen/go/edgerun/v0/html"
)

// GenerateTreeBuilderRust produces the full tree_builder.rs source file.
func GenerateTreeBuilderRust(ruleSet *html.TreeBuilderRuleSet, voidElements []string) string {
	// Group rules by mode
	type modeRules struct {
		mode html.InsertionMode
		name string
		rules []*html.TreeRule
	}
	var modeMap = make(map[html.InsertionMode][]*html.TreeRule)
	for _, r := range ruleSet.Rules {
		modeMap[r.Mode] = append(modeMap[r.Mode], r)
	}

	// Extract rules for each mode
	modes := []html.InsertionMode{
		html.InsertionMode_INITIAL_MODE,
		html.InsertionMode_BEFORE_HTML_MODE,
		html.InsertionMode_BEFORE_HEAD_MODE,
		html.InsertionMode_IN_HEAD_MODE,
		html.InsertionMode_IN_HEAD_NOSCRIPT_MODE,
		html.InsertionMode_AFTER_HEAD_MODE,
		html.InsertionMode_IN_BODY_MODE,
		html.InsertionMode_TEXT_MODE,
		html.InsertionMode_IN_TABLE_MODE,
		html.InsertionMode_IN_TABLE_TEXT_MODE,
		html.InsertionMode_IN_TABLE_BODY_MODE,
		html.InsertionMode_IN_ROW_MODE,
		html.InsertionMode_IN_CELL_MODE,
		html.InsertionMode_IN_CAPTION_MODE,
		html.InsertionMode_IN_COLUMN_GROUP_MODE,
		html.InsertionMode_AFTER_BODY_MODE,
		html.InsertionMode_IN_FRAMESET_MODE,
		html.InsertionMode_AFTER_FRAMESET_MODE,
		html.InsertionMode_AFTER_AFTER_BODY_MODE,
		html.InsertionMode_AFTER_AFTER_FRAMESET_MODE,
		html.InsertionMode_IN_SELECT_MODE,
		html.InsertionMode_IN_SELECT_IN_TABLE_MODE,
		html.InsertionMode_IN_TEMPLATE_MODE,
	}

	// Generate mode handlers
	var modeHandlers string
	for _, mode := range modes {
		name := insertionModeToRust(mode)
		rules := modeMap[mode]
		if len(rules) == 0 {
			continue
		}
		modeHandlers += generateModeHandler(name, rules)
	}

	// Generate handle_token dispatch
	var caseArms []string
	for _, mode := range modes {
		name := insertionModeToRust(mode)
		if _, ok := modeMap[mode]; !ok {
			continue
		}
		caseArms = append(caseArms, fmt.Sprintf("            InsertionMode::%s => self.handle_%s(token),", name, snakeCase(name)))
	}
	caseDispatch := strings.Join(caseArms, "\n")

	voidSet := strings.Join(quoteList(voidElements), ", ")

	return fmt.Sprintf(`// DO NOT EDIT.
// Auto-generated from Parser IR by cmd/html-codegen
// Regenerate: go run ./cmd/html-codegen
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

use crate::html_parser::{Node, Element};
use crate::tokenizer::Token;

/// Void element tags — never pushed to the open elements stack.
const VOID_ELEMENTS: &[&str] = &[%s];

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
/// Generated from proto IR with %d tree rules across %d insertion modes.
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
%s
            _ => self.handle_fallback(token),
        }
    }

%s

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

    /// Acknowledge the self-closing flag.
    fn acknowledge_self_closing(&mut self) {
        self.current_token_is_self_closing = false;
    }

    /// Append a comment to the current node.
    fn append_comment(&mut self, text: &str) {
        if let Some(parent) = self.open_elements.last_mut() {
            parent.children.push(Node::Comment(text.to_string()));
        } else if let Some(Node::Element(elem)) = self.completed.last_mut() {
            elem.children.push(Node::Comment(text.to_string()));
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
    fn insert(&mut self, name: &str, attrs: &BTreeMap<String, String>, self_closing: bool) {
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
`,
		voidSet,
		len(ruleSet.Rules), ruleSet.TotalInsertionModes,
		caseDispatch,
		modeHandlers,
	)
}

func snakeCase(s string) string {
	out := ""
	for i, r := range s {
		if i > 0 && r >= 'A' && r <= 'Z' {
			out += "_"
		}
		out += string(r)
	}
	return strings.ToLower(out)
}

func generateModeHandler(modeName string, rules []*html.TreeRule) string {
	// Parse rules into buckets by trigger type
	type tbRule struct {
		tag         string
		actions     []html.TreeAction
		popUntil    string
		isAny       bool
		nextMode    html.InsertionMode
		condition   *html.StackCondition
		hasInScope  string
		hasInButton string
		hasInLI     string
	}
	var startTags, endTags []tbRule
	var charRule, commentRule, doctypeRule []html.TreeAction
	var eofMode html.InsertionMode
	eofFound := false
	var reprocessMode html.InsertionMode
	reprocessFound := false

	for _, r := range rules {
		trigger := r.Trigger
		if trigger == nil {
			continue
		}

		// Extract scope condition
		var hasInScope, hasInButton, hasInLI string
		if r.Condition != nil {
			if list := r.Condition.GetHasInScope(); len(list) > 0 {
				hasInScope = list[0]
			}
			if list := r.Condition.GetHasInButtonScope(); len(list) > 0 {
				hasInButton = list[0]
			}
			if list := r.Condition.GetHasInListScope(); len(list) > 0 {
				hasInLI = list[0]
			}
		}

		switch {
		case trigger.GetStartTag() != "":
			startTags = append(startTags, tbRule{
				tag: trigger.GetStartTag(), actions: r.Actions, popUntil: r.PopUntilTag, nextMode: r.NextMode,
				condition: r.Condition, hasInScope: hasInScope, hasInButton: hasInButton, hasInLI: hasInLI,
			})
		case trigger.GetEndTag() != "":
			endTags = append(endTags, tbRule{
				tag: trigger.GetEndTag(), actions: r.Actions, popUntil: r.PopUntilTag, nextMode: r.NextMode,
				condition: r.Condition, hasInScope: hasInScope, hasInButton: hasInButton, hasInLI: hasInLI,
			})
		case trigger.GetAnyStartTag():
			startTags = append(startTags, tbRule{
				tag: "_", actions: r.Actions, popUntil: r.PopUntilTag, isAny: true, nextMode: r.NextMode,
				condition: r.Condition, hasInScope: hasInScope, hasInButton: hasInButton, hasInLI: hasInLI,
			})
		case trigger.GetAnyEndTag():
			endTags = append(endTags, tbRule{
				tag: "_", actions: r.Actions, popUntil: r.PopUntilTag, isAny: true, nextMode: r.NextMode,
				condition: r.Condition, hasInScope: hasInScope, hasInButton: hasInButton, hasInLI: hasInLI,
			})
		case trigger.GetCharacterToken():
			charRule = r.Actions
			if hasAction(r.Actions, html.TreeAction_TREE_ACTION_REPROCESS) {
				reprocessMode = r.NextMode
				reprocessFound = true
			}
		case trigger.GetTokenType() == html.TokenType_TOKEN_TYPE_COMMENT:
			commentRule = r.Actions
		case trigger.GetTokenType() == html.TokenType_TOKEN_TYPE_DOCTYPE:
			doctypeRule = r.Actions
		case trigger.GetTokenType() == html.TokenType_TOKEN_TYPE_EOF:
			eofMode = r.NextMode
			eofFound = true
		}
	}

	snake := snakeCase(modeName)

	// Sort tags: specific first, catch-all last
	sort.SliceStable(startTags, func(i, j int) bool {
		if startTags[i].isAny != startTags[j].isAny { return !startTags[i].isAny }
		return startTags[i].tag < startTags[j].tag
	})
	sort.SliceStable(endTags, func(i, j int) bool {
		if endTags[i].isAny != endTags[j].isAny { return !endTags[i].isAny }
		return endTags[i].tag < endTags[j].tag
	})

	// Generate match arms - deduplicate by tag (keep first rule per tag)
	type seenTag struct { tag string; hasCondition bool }
	seenTags := make(map[string]bool)
	var startArms, endArms []string
	for _, r := range startTags {
		// Skip if we already generated an arm for this tag
		if seenTags[r.tag] {
			continue
		}
		seenTags[r.tag] = true

		rustActions := tbActionsToRustWithMode(r.actions, r.popUntil, r.isAny, r.nextMode)

		// Add scope check prefix if condition exists
		// For conditional rules: check scope → if true, do pop_until → then always do the rest
		if r.hasInScope != "" {
			rustActions = fmt.Sprintf("if self.has_in_scope(\"%s\") { self.pop_until(\"%s\"); }\n                    %s", r.hasInScope, r.hasInScope, rustActions)
		}
		if r.hasInButton != "" {
			rustActions = fmt.Sprintf("if self.has_in_button_scope(\"%s\") { self.pop_until(\"%s\"); }\n                    %s", r.hasInButton, r.hasInButton, rustActions)
		}
		if r.hasInLI != "" {
			rustActions = fmt.Sprintf("if self.has_in_list_item_scope(\"%s\") { self.pop_until(\"%s\"); }\n                    %s", r.hasInLI, r.hasInLI, rustActions)
		}

		if r.isAny {
			startArms = append(startArms, fmt.Sprintf("                _ => {\n                    %s\n                }", rustActions))
		} else {
			startArms = append(startArms, fmt.Sprintf("                \"%s\" => {\n                    %s\n                }", r.tag, rustActions))
		}
	}
	seenEndTags := make(map[string]bool)
	for _, r := range endTags {
		if seenEndTags[r.tag] { continue }
		seenEndTags[r.tag] = true

		rustActions := tbActionsToRustWithMode(r.actions, r.popUntil, r.isAny, r.nextMode)
		if r.isAny {
			endArms = append(endArms, fmt.Sprintf("                _ => {\n                    %s\n                }", rustActions))
		} else {
			endArms = append(endArms, fmt.Sprintf("                \"%s\" => {\n                    %s\n                }", r.tag, rustActions))
		}
	}

	startDispatch := strings.Join(startArms, "\n")
	endDispatch := strings.Join(endArms, "\n")

	// Ensure catch-all exists
	hasStartCatchAll := false
	hasEndCatchAll := false
	for _, r := range startTags { if r.isAny { hasStartCatchAll = true; break } }
	for _, r := range endTags { if r.isAny { hasEndCatchAll = true; break } }
	if !hasStartCatchAll { startDispatch += "\n                _ => { self.insert(name, attrs, *self_closing); }" }
	if !hasEndCatchAll { endDispatch += "\n                _ => {\n                    if self.has_in_scope(name) { self.pop_until(name); }\n                    // otherwise: parse error, ignore\n                }" }

	// Character handling
	var charCode string
	if reprocessFound {
		charCode = fmt.Sprintf(`// Reprocess character in next mode
                self.insertion_mode = InsertionMode::%s;
                self.handle_token(token);`, insertionModeToRust(reprocessMode))
	} else if hasAction(charRule, html.TreeAction_TREE_ACTION_APPEND_CHARACTER) {
		charCode = "if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(text.clone())); }"
	} else if hasAction(charRule, html.TreeAction_TREE_ACTION_PARSE_ERROR) && hasAction(charRule, html.TreeAction_TREE_ACTION_IGNORE) {
		charCode = "// parse error, ignore character"
	} else {
		charCode = "// no character rule defined"
	}

	// Comment handling
	var commentCode string
	if hasAction(commentRule, html.TreeAction_TREE_ACTION_APPEND_COMMENT) {
		commentCode = `if let Some(parent) = self.open_elements.last_mut() {
                    parent.children.push(Node::Comment(text.clone()));
                } else if !self.completed.is_empty() {
                    // Append to last completed
                }`
	} else if hasAction(commentRule, html.TreeAction_TREE_ACTION_PARSE_ERROR) && hasAction(commentRule, html.TreeAction_TREE_ACTION_IGNORE) {
		commentCode = "// parse error, ignore comment"
	} else {
		commentCode = "// ignore comment"
	}

	// DOCTYPE handling
	var doctypeCode string
	if hasAction(doctypeRule, html.TreeAction_TREE_ACTION_APPEND_DOCTYPE) {
		doctypeCode = `self.open_elements.push(Element::new("html"));
                // TODO: create proper DOCTYPE node`
	} else if hasAction(doctypeRule, html.TreeAction_TREE_ACTION_PARSE_ERROR) && hasAction(doctypeRule, html.TreeAction_TREE_ACTION_IGNORE) {
		doctypeCode = "// parse error, ignore DOCTYPE"
	} else {
		doctypeCode = "// ignore DOCTYPE"
	}

	// EOF handling
	var eofCode string
	if eofFound {
		eofCode = fmt.Sprintf("self.insertion_mode = InsertionMode::%s;", insertionModeToRust(eofMode))
	} else {
		eofCode = "// no EOF rule defined"
	}

	startBlock := ""
	if len(startArms) > 0 {
		// Check if any arm has reprocess (isAny with PARSE_ERROR or POP_UNTIL + Otherwise rule)
		var reprocessNextMode string
		for _, r := range startTags {
			if r.isAny && r.nextMode != 0 &&
				(hasAction(r.actions, html.TreeAction_TREE_ACTION_PARSE_ERROR) ||
					hasAction(r.actions, html.TreeAction_TREE_ACTION_POP_UNTIL)) {
				reprocessNextMode = insertionModeToRust(r.nextMode)
				break
			}
		}
		if reprocessNextMode != "" {
			startBlock = fmt.Sprintf(`match &name[..] {
%s
                }
                // Reprocess in next mode (otherwise rule)
                self.insertion_mode = InsertionMode::%s;
                self.handle_token(token);`, startDispatch, reprocessNextMode)
		} else {
			startBlock = fmt.Sprintf(`match &name[..] {
%s
                }`, startDispatch)
		}
	} else {
		startBlock = "self.insert(name, attrs, *self_closing);"
	}

	endBlock := ""
	if len(endArms) > 0 {
		endBlock = fmt.Sprintf(`match &name[..] {
%s
                }`, endDispatch)
	} else {
		endBlock = "self.pop_until(name);"
	}

	return fmt.Sprintf(`    /// %s mode rules.
    /// Generated from %d rules.
    fn handle_%s(&mut self, token: &Token) {
        match token {
            Token::StartTag { name, attrs, self_closing } => {
                %s
            }
            Token::EndTag { name } => {
                %s
            }
            Token::Character(text) => {
                %s
            }
            Token::Comment(text) => {
                %s
            }
            Token::Doctype => {
                %s
            }
            Token::Eof => {
                %s
            }
        }
    }

`, modeName, len(rules), snake, startBlock, endBlock, charCode, commentCode, doctypeCode, eofCode)
}

func hasAction(actions []html.TreeAction, target html.TreeAction) bool {
	for _, a := range actions {
		if a == target {
			return true
		}
	}
	return false
}

func tbActionsToRustWithMode(actions []html.TreeAction, popUntil string, isOtherwise bool, nextMode html.InsertionMode) string {
	var parts []string
	hasReprocess := hasAction(actions, html.TreeAction_TREE_ACTION_REPROCESS)
	for _, a := range actions {
		rust := treeActionToRust(a, popUntil)
		if rust != "" {
			parts = append(parts, rust)
		}
	}
	// Reprocess takes priority — set mode, re-dispatch, return
	if hasReprocess && nextMode != 0 {
		return fmt.Sprintf("self.parse_errors += 1;\n                    self.insertion_mode = InsertionMode::%s;\n                    self.handle_token(token);\n                    return;", insertionModeToRust(nextMode))
	}
	if isOtherwise {
		// For "otherwise" rules: do actions, set mode, reprocess
		if len(parts) == 0 {
			parts = append(parts, "// parse error (no specific action)")
		}
	} else if len(parts) == 0 {
		parts = append(parts, "self.insert(name, attrs, *self_closing);")
	}
	return strings.Join(parts, "\n                    ")
}

func tbActionsToRust(actions []html.TreeAction, popUntil string) string {
	var parts []string
	for _, a := range actions {
		rust := treeActionToRust(a, popUntil)
		if rust != "" {
			parts = append(parts, rust)
		}
	}
	if len(parts) == 0 {
		// Default: INSERT
		return "self.insert(name, attrs, *self_closing);"
	}
	return strings.Join(parts, "\n                    ")
}

func treeActionToRust(a html.TreeAction, popUntil string) string {
	switch a {
	case html.TreeAction_TREE_ACTION_INSERT:
		return "self.insert(name, attrs, *self_closing);"
	case html.TreeAction_TREE_ACTION_POP_UNTIL:
		if popUntil != "" {
			return fmt.Sprintf(`self.pop_until("%s");`, popUntil)
		}
		return "self.pop_until(name);"
	case html.TreeAction_TREE_ACTION_POP:
		return "if let Some(_) = self.open_elements.pop() {}"
	case html.TreeAction_TREE_ACTION_IGNORE:
		return "// ignore token"
	case html.TreeAction_TREE_ACTION_PARSE_ERROR:
		return "self.parse_errors += 1;"
	case html.TreeAction_TREE_ACTION_REPROCESS:
		return "" // handled by codegen via next_mode + loop
	case html.TreeAction_TREE_ACTION_APPEND_CHARACTER:
		return `if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Text(text.clone())); }`
	case html.TreeAction_TREE_ACTION_SWITCH_TO_RCDATA:
		return "self.switch_to_rcdata();"
	case html.TreeAction_TREE_ACTION_SWITCH_TO_RAWTEXT:
		return "self.switch_to_rawtext();"
	case html.TreeAction_TREE_ACTION_SWITCH_TO_SCRIPT_DATA:
		return "self.switch_to_script_data();"
	case html.TreeAction_TREE_ACTION_RESET_INSERTION_MODE:
		return "self.reset_insertion_mode();"
	case html.TreeAction_TREE_ACTION_INSERT_FOSTER:
		return "self.insert_foster(name, attrs);"
	case html.TreeAction_TREE_ACTION_ACKNOWLEDGE_SELF_CLOSING:
		return "" // self-closing flag is not tracked
	case html.TreeAction_TREE_ACTION_APPEND_COMMENT:
		return `if let Some(parent) = self.open_elements.last_mut() { parent.children.push(Node::Comment(text.clone())); }`
	case html.TreeAction_TREE_ACTION_APPEND_DOCTYPE:
		return "// DOCTYPE handled in Initial mode"
	case html.TreeAction_TREE_ACTION_SET_FRAMESET_NOT_OK:
		return "" // frameset_ok flag is not tracked
	case html.TreeAction_TREE_ACTION_POP_ALL:
		return "while self.open_elements.pop().is_some() {}"
	default:
		return "// TODO: unknown tree action"
	}
}

func insertionModeToRust(m html.InsertionMode) string {
	name := html.InsertionMode_name[int32(m)]
	name = strings.TrimPrefix(name, "INSERTION_MODE_")
	name = strings.TrimSuffix(name, "_MODE")
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

func quoteList(items []string) []string {
	out := make([]string, len(items))
	for i, s := range items {
		out[i] = fmt.Sprintf(`"%s"`, s)
	}
	return out
}
