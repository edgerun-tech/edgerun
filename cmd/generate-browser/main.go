// Generate a complete browser engine crate from spec catalogs.
// Outputs: crates/edgerun-browser/
//
// Usage: go run ./cmd/generate-browser html.json css.json js.json crates/edgerun-browser/
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

type HTMLCatalog struct {
	Elements []HTMLElement `json:"elements"`
}

type HTMLElement struct {
	TagName               string       `json:"tag_name"`
	Categories            []string     `json:"categories"`
	ContentModel          string       `json:"content_model"`
	HasGlobalAttributes   bool         `json:"has_global_attributes"`
	DOMInterface          string       `json:"dom_interface"`
	ElementSpecificAttrs  []AttrDef    `json:"element_specific_attributes"`
}

type AttrDef struct {
	Name        string `json:"name"`
	Description string `json:"description"`
}

type CSSCatalog struct {
	Properties []CSSProp `json:"properties"`
	AtRules    []CSSAtRule `json:"at_rules"`
}

type CSSProp struct {
	Name        string `json:"name"`
	Value       string `json:"value"`
	Initial     string `json:"initial"`
	Inherited   string `json:"inherited"`
}

type CSSAtRule struct {
	Name string `json:"name"`
}

type JSCatalog struct {
	BuiltInObjects []JSObj `json:"built_in_objects"`
	AbstractOps    []JSOp  `json:"abstract_operations"`
}

type JSObj struct {
	Name             string     `json:"name"`
	PrototypeMethods []JSMethod `json:"prototype_methods"`
	StaticProperties []JSProp   `json:"static_properties"`
	PrototypeProperties []JSProp `json:"prototype_properties"`
}

type JSMethod struct {
	Name       string   `json:"name"`
	Parameters []string `json:"parameters"`
}

type JSProp struct {
	Name string `json:"name"`
	Kind string `json:"kind"`
}

type JSOp struct {
	Name       string   `json:"name"`
	Parameters []string `json:"parameters"`
}

func ri(name string) string { return strings.ReplaceAll(name, "-", "_") }

func rv(name string) string {
	parts := strings.Split(strings.ReplaceAll(name, "-", "_"), "_")
	for i, p := range parts {
		if len(p) > 0 {
			parts[i] = strings.ToUpper(p[:1]) + p[1:]
		}
	}
	return strings.Join(parts, "")
}

func classifyCM(raw string) string {
	l := strings.ToLower(raw)
	if l == "" {
		return "Custom"
	}
	if l == "nothing" || l == "nothing." || l == "void" {
		return "Nothing"
	}
	if strings.Contains(l, "transparent") {
		return "Transparent"
	}
	if strings.HasPrefix(l, "text") {
		return "Text"
	}
	if strings.Contains(l, "metadata content") {
		return "Metadata"
	}
	if strings.Contains(l, "heading") {
		return "Heading"
	}
	if strings.Contains(l, "sectioning") {
		return "Sectioning"
	}
	if strings.Contains(l, "flow content") {
		return "Flow"
	}
	if strings.Contains(l, "phrasing content") || strings.Contains(l, "text that is not") {
		return "Phrasing"
	}
	if strings.Contains(l, "embedded content") {
		return "Embedded"
	}
	if strings.Contains(l, "interactive content") {
		return "Interactive"
	}
	if strings.Contains(l, "palatable content") {
		return "Palatable"
	}
	if strings.Contains(l, "script-supporting") {
		return "ScriptSupporting"
	}
	return "Custom"
}

func w(path, content string) {
	os.MkdirAll(filepath.Dir(path), 0755)
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		fmt.Fprintf(os.Stderr, "error writing %s: %v\n", path, err)
	}
	fmt.Printf("  %s: %d lines\n", path, strings.Count(content, "\n"))
}

func genElementRegistry(html HTMLCatalog) string {
	var L []string
	L = append(L, "//! HTML Element Registry -- generated from WHATWG HTML Living Standard.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-browser",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum ContentModel {")
	for _, cm := range []string{"Flow", "Phrasing", "Metadata", "Heading", "Sectioning", "Embedded", "Interactive", "Palatable", "ScriptSupporting", "Nothing", "Transparent", "Text", "Custom"} {
		L = append(L, fmt.Sprintf("    %s,", cm))
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone)]",
		"pub struct AttrDef {",
		"    pub name: &'static str,",
		"    pub description: &'static str,",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub struct ElementDef {",
		"    pub tag: &'static str,",
		"    pub content_model: ContentModel,",
		"    pub has_global_attributes: bool,",
		"    pub dom_interface: &'static str,",
		"    pub specific_attributes: &'static [AttrDef],",
		"}", "",
		"pub struct ElementRegistry;", "",
		"impl ElementRegistry {",
		"    pub fn by_tag(tag: &str) -> Option<&'static ElementDef> {",
		"        match tag {")

	for _, el := range html.Elements {
		L = append(L, fmt.Sprintf(`            "%s" => Some(&E_%s),`, el.TagName, strings.ToUpper(ri(el.TagName))))
	}
	L = append(L, `            _ => None,`,
		`        }`,
		`    }`,
		`    pub fn all() -> impl Iterator<Item = &'static ElementDef> {`,
		`        ALL.iter().copied()`,
		`    }`,
		`}`, "")

	for _, el := range html.Elements {
		cm := classifyCM(el.ContentModel)
		iface := el.DOMInterface
		if iface == "" {
			iface = "HTMLElement"
		}
		ga := "false"
		if el.HasGlobalAttributes {
			ga = "true"
		}
		var attrs []string
		for _, a := range el.ElementSpecificAttrs {
			desc := strings.ReplaceAll(a.Description, `\`, `\\`)
			desc = strings.ReplaceAll(desc, `"`, `\"`)
			attrs = append(attrs, fmt.Sprintf(`AttrDef { name: "%s", description: "%s" }`, a.Name, desc))
		}
		alist := strings.Join(attrs, ", ")
		L = append(L, fmt.Sprintf("pub const E_%s: ElementDef = ElementDef {", strings.ToUpper(ri(el.TagName))))
		L = append(L, fmt.Sprintf(`    tag: "%s",`, el.TagName))
		L = append(L, fmt.Sprintf("    content_model: ContentModel::%s,", cm))
		L = append(L, fmt.Sprintf("    has_global_attributes: %s,", ga))
		L = append(L, fmt.Sprintf(`    dom_interface: "%s",`, iface))
		L = append(L, fmt.Sprintf("    specific_attributes: &[%s],", alist))
		L = append(L, "};", "")
	}

	L = append(L, "const ALL: &[&ElementDef] = &[")
	for _, el := range html.Elements {
		L = append(L, fmt.Sprintf("    &E_%s,", strings.ToUpper(ri(el.TagName))))
	}
	L = append(L, "];")

	return strings.Join(L, "\n")
}

func genTokenizerStates() string {
	states := []string{
		"Data", "RcData", "Rawtext", "ScriptData", "Plaintext", "TagOpen",
		"EndTagOpen", "TagName", "RcDataLessthanSign", "RcDataEndTagOpen",
		"RcDataEndTagName", "RawtextLessthanSign", "RawtextEndTagOpen",
		"RawtextEndTagName", "ScriptDataLessthanSign", "ScriptDataEndTagOpen",
		"ScriptDataEndTagName", "ScriptDataEscapeStart", "ScriptDataEscapeStartDash",
		"ScriptDataEscaped", "ScriptDataEscapedDash", "ScriptDataEscapedDashDash",
		"ScriptDataEscapedLessthanSign", "ScriptDataEscapedEndTagOpen",
		"ScriptDataEscapedEndTagName", "ScriptDataDoubleEscapeStart",
		"ScriptDataDoubleEscaped", "ScriptDataDoubleEscapedDash",
		"ScriptDataDoubleEscapedDashDash", "ScriptDataDoubleEscapedLessthanSign",
		"ScriptDataDoubleEscapeEnd", "BeforeAttributeName", "AttributeName",
		"AfterAttributeName", "BeforeAttributeValue", "AttributeValueDoubleQuoted",
		"AttributeValueSingleQuoted", "AttributeValueUnquoted",
		"AfterAttributeValueQuoted", "SelfClosingStartTag", "BogusComment",
		"MarkupDeclarationOpen", "CommentStart", "CommentStartDash", "Comment",
		"CommentLessthanSign", "CommentLessthanSignBang",
		"CommentLessthanSignBangDash", "CommentLessthanSignBangDashDash",
		"CommentEndDash", "CommentEnd", "CommentEndBang", "Doctype",
		"BeforeDoctypeName", "DoctypeName", "AfterDoctypeName",
		"AfterDoctypePublicKeyword", "BeforeDoctypePublicIdentifier",
		"DoctypePublicIdentifierDoubleQuoted", "DoctypePublicIdentifierSingleQuoted",
		"AfterDoctypePublicIdentifier", "BetweenDoctypePublicAndSystemIdentifiers",
		"AfterDoctypeSystemKeyword", "BeforeDoctypeSystemIdentifier",
		"DoctypeSystemIdentifierDoubleQuoted", "DoctypeSystemIdentifierSingleQuoted",
		"AfterDoctypeSystemIdentifier", "BogusDoctype", "CdataSection",
		"CdataSectionBracket", "CdataSectionEnd",
		"CharacterReferenceInAttributeValue", "NamedCharacterReference",
		"NamedCharacterReferenceStart", "NamedCharacterReferenceNoSemicolon",
		"NumericCharacterReference", "NumericCharacterReferenceStart",
		"NumericCharacterReferenceEnd", "HexademicalCharacterReferenceStart",
		"AmbiguousAmpersand", "Eof",
	}

	var L []string
	L = append(L, "//! HTML Tokenizer States -- WHATWG §13.2.5.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-browser",
		"extern crate alloc;",
		"use alloc::{string::String, vec::Vec};",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum TokenizerState {")
	for _, s := range states {
		L = append(L, fmt.Sprintf("    %s,", s))
	}
	L = append(L, "}", "",
		"impl TokenizerState {",
		"    pub fn initial() -> Self { Self::Data }",
		"}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum TokenKind {",
		"    DoctypeTag,",
		"    StartTag,",
		"    EndTag,",
		"    Comment,",
		"    Character(u32),",
		"    Eof,",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub struct Token {",
		"    pub kind: TokenKind,",
		"    pub tag_name: String,",
		"    pub attributes: Vec<(String, String)>,",
		"    pub self_closing: bool,",
		"}")

	return strings.Join(L, "\n")
}

func genTreeBuilder() string {
	modes := []string{
		"Initial", "BeforeHtml", "BeforeHead", "InHead", "InHeadNoscript",
		"AfterHead", "InBody", "Text", "InTable", "InTableText",
		"InCaption", "InRow", "InCell", "InColgroup", "InTableBody",
		"AfterBody", "InFrameset", "AfterFrameset", "AfterAfterBody",
		"AfterAfterFrameset",
	}

	var L []string
	L = append(L, "extern crate alloc;",
		"use alloc::vec::Vec;", "",
		"//! HTML Tree Builder Insertion Modes -- WHATWG §13.2.6.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-browser",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum InsertionMode {")
	for _, m := range modes {
		L = append(L, fmt.Sprintf("    %s,", m))
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone)]",
		"pub enum InsertionAction {",
		"    InsertNode,",
		"    Ignore,",
		"    FosterParent,",
		"    Reprocess(InsertionMode),",
		"    ChangeMode(InsertionMode),",
		`    InScope { tag: &'static str, result: bool },`,
		"}", "",
		"pub struct TreeBuilderState {",
		"    pub insertion_mode: InsertionMode,",
		"    pub original_insertion_mode: Option<InsertionMode>,",
		"    pub frameset_ok: bool,",
		"    pub head_element: Option<usize>,",
		"    pub form_element: Option<usize>,",
		"    pub template_insertion_modes: Vec<InsertionMode>,",
		"}", "",
		"impl TreeBuilderState {",
		"    pub fn new() -> Self {",
		"        Self {",
		"            insertion_mode: InsertionMode::Initial,",
		"            original_insertion_mode: None,",
		"            frameset_ok: true,",
		"            head_element: None,",
		"            form_element: None,",
		"            template_insertion_modes: Vec::new(),",
		"        }",
		"    }",
		"}")

	return strings.Join(L, "\n")
}

func genPropertyRegistry(css CSSCatalog) string {
	var L []string
	L = append(L, "//! CSS Property Registry -- generated from W3C CSS specifications.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-browser",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum CssPropertyId {",
		"    Unspecified = 0,")
	for i, p := range css.Properties {
		L = append(L, fmt.Sprintf("    %s = %d,", rv(p.Name), i+1))
	}
	L = append(L, "}", "",
		"impl CssPropertyId {",
		"    pub fn from_name(name: &str) -> Option<Self> {",
		"        match name {")
	for _, p := range css.Properties {
		L = append(L, fmt.Sprintf(`            "%s" => Some(CssPropertyId::%s),`, p.Name, rv(p.Name)))
	}
	L = append(L, `            _ => None,`,
		`        }`,
		`    }`,
		`    pub fn name(&self) -> &'static str {`,
		`        match self {`)
	for _, p := range css.Properties {
		L = append(L, fmt.Sprintf(`            CssPropertyId::%s => "%s",`, rv(p.Name), p.Name))
	}
	L = append(L, `        }`,
		`    }`,
		`}`, "",
		"#[derive(Debug, Clone)]",
		"pub struct PropertyDef {",
		"    pub id: CssPropertyId,",
		"    pub name: &'static str,",
		"    pub value_syntax: &'static str,",
		"    pub initial_value: &'static str,",
		"    pub inherited: bool,",
		"    pub animates: bool,",
		"}", "",
		"pub struct PropertyRegistry;", "",
		"impl PropertyRegistry {",
		"    pub fn by_name(name: &str) -> Option<&'static PropertyDef> {",
		"        CssPropertyId::from_name(name).and_then(Self::by_id)",
		"    }",
		"    pub fn by_id(id: CssPropertyId) -> Option<&'static PropertyDef> {",
		"        match id {")

	for _, p := range css.Properties {
		L = append(L, fmt.Sprintf(`            CssPropertyId::%s => Some(&P_%s),`, rv(p.Name), strings.ToUpper(ri(p.Name))))
	}
	L = append(L, `            CssPropertyId::Unspecified => None,`,
		`        }`,
		`    }`,
		`}`, "")

	for _, p := range css.Properties {
		inh := strings.HasPrefix(strings.ToLower(p.Inherited), "yes")
		init := strings.ReplaceAll(p.Initial, `"`, `\"`)
		syn := strings.ReplaceAll(p.Value, `"`, `\"`)
		syn = strings.ReplaceAll(syn, `\`, `\\`)
		L = append(L, fmt.Sprintf("pub const P_%s: PropertyDef = PropertyDef {", strings.ToUpper(ri(p.Name))))
		L = append(L, fmt.Sprintf("    id: CssPropertyId::%s,", rv(p.Name)))
		L = append(L, fmt.Sprintf(`    name: "%s",`, p.Name))
		L = append(L, fmt.Sprintf(`    value_syntax: "%s",`, syn))
		L = append(L, fmt.Sprintf(`    initial_value: "%s",`, init))
		L = append(L, fmt.Sprintf("    inherited: %v,", inh))
		L = append(L, "    animates: false,")
		L = append(L, "};", "")
	}

	return strings.Join(L, "\n")
}

func genValueTypes() string {
	var L []string
	L = append(L, "extern crate alloc;",
		"use alloc::string::String;",
		"use alloc::vec::Vec;", "",
		"//! CSS Value Types -- generated from W3C CSS specifications.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-browser",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum CssValueType {",
		"    Keyword, Dimension, Number, Percentage, String, Url, Function, Color,",
		"}", "",
		"#[derive(Debug, Clone, PartialEq)]",
		"pub enum CssValue {",
		"    Keyword(&'static str),",
		"    Dimension(f64, &'static str),",
		"    Number(f64),",
		"    Percentage(f64),",
		"    StringValue(String),",
		"    Url(String),",
		"    Function { name: String, args: Vec<CssValue> },",
		"    Color { r: f64, g: f64, b: f64, a: f64 },",
		"    List(Vec<CssValue>),",
		"}", "",
		"impl CssValue {",
		"    pub fn value_type(&self) -> CssValueType {",
		"        match self {",
		"            Self::Keyword(_) => CssValueType::Keyword,",
		"            Self::Dimension(_, _) => CssValueType::Dimension,",
		"            Self::Number(_) => CssValueType::Number,",
		"            Self::Percentage(_) => CssValueType::Percentage,",
		"            Self::StringValue(_) => CssValueType::String,",
		"            Self::Url(_) => CssValueType::Url,",
		"            Self::Function { .. } => CssValueType::Function,",
		"            Self::Color { .. } => CssValueType::Color,",
		"            Self::List(_) => CssValueType::Keyword,",
		"        }",
		"    }",
		`    pub fn is_inherit(&self) -> bool { matches!(self, Self::Keyword("inherit")) }`,
		`    pub fn is_initial(&self) -> bool { matches!(self, Self::Keyword("initial")) }`,
		`    pub fn is_unset(&self) -> bool { matches!(self, Self::Keyword("unset")) }`,
		`    pub fn is_auto(&self) -> bool { matches!(self, Self::Keyword("auto")) }`,
		`    pub fn is_none(&self) -> bool { matches!(self, Self::Keyword("none")) }`,
		"}")

	return strings.Join(L, "\n")
}

func genAtRuleRegistry(css CSSCatalog) string {
	var L []string
	L = append(L, "extern crate alloc;",
		"use alloc::vec::Vec;", "",
		"//! CSS At-Rule Registry -- generated from W3C CSS specifications.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-browser",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum AtRuleId {",
		"    Unspecified = 0,")
	for i, r := range css.AtRules {
		L = append(L, fmt.Sprintf("    %s = %d,", rv(r.Name), i+1))
	}
	L = append(L, "}", "",
		"impl AtRuleId {",
		"    pub fn from_name(name: &str) -> Option<Self> {",
		"        match name {")
	for _, r := range css.AtRules {
		L = append(L, fmt.Sprintf(`            "%s" => Some(AtRuleId::%s),`, r.Name, rv(r.Name)))
	}
	L = append(L, `            _ => None,`,
		`        }`,
		`    }`,
		`    pub fn name(&self) -> &'static str {`,
		`        match self {`)
	for _, r := range css.AtRules {
		L = append(L, fmt.Sprintf(`            AtRuleId::%s => "@%s",`, rv(r.Name), r.Name))
	}
	L = append(L, `        }`,
		`    }`,
		`}`, "",
		"#[derive(Debug, Clone)]",
		"pub enum AtRulePrelude {")
	for _, r := range css.AtRules {
		L = append(L, fmt.Sprintf("    %s,", rv(r.Name)))
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone)]",
		"pub struct AtRuleBlock {",
		"    pub at_rule: AtRuleId,",
		"    pub declarations: Vec<(crate::property_registry::CssPropertyId, crate::value_types::CssValue)>,",
		"}")

	return strings.Join(L, "\n")
}

func genJSObjectRegistry(js JSCatalog) string {
	objs := make([]JSObj, 0, len(js.BuiltInObjects))
	for _, o := range js.BuiltInObjects {
		if len(o.PrototypeMethods) > 0 || len(o.StaticProperties) > 0 {
			objs = append(objs, o)
		}
	}

	var L []string
	L = append(L, "//! ECMAScript Built-in Object Registry -- generated from ECMA-262.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-browser",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum JsObjectId {",
		"    Unspecified = 0,")
	for i, o := range objs {
		L = append(L, fmt.Sprintf("    %s = %d,", rv(o.Name), i+1))
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum JsMemberKind { Method, Getter, Setter, Data }", "",
		"#[derive(Debug, Clone)]",
		"pub struct JsMemberDef {",
		"    pub name: &'static str,",
		"    pub kind: JsMemberKind,",
		"    pub param_count: u8,",
		"    pub is_prototype: bool,",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub struct JsObjectDef {",
		"    pub id: JsObjectId,",
		"    pub name: &'static str,",
		"    pub members: &'static [JsMemberDef],",
		"}", "",
		"pub struct JsObjectRegistry;")

	var matchArms []string
	var consts []string
	for _, o := range objs {
		var members []string
		for _, m := range o.PrototypeMethods {
			pc := len(m.Parameters)
			members = append(members, fmt.Sprintf(`JsMemberDef { name: "%s", kind: JsMemberKind::Method, param_count: %d, is_prototype: true }`, m.Name, pc))
		}
		for _, p := range o.StaticProperties {
			members = append(members, fmt.Sprintf(`JsMemberDef { name: "%s", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }`, p.Name))
		}
		for _, p := range o.PrototypeProperties {
			members = append(members, fmt.Sprintf(`JsMemberDef { name: "%s", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }`, p.Name))
		}
		memberStr := strings.Join(members, ", ")
		matchArms = append(matchArms, fmt.Sprintf(`            "%s" => Some(&JSO_%s),`, o.Name, strings.ToUpper(ri(o.Name))))
		consts = append(consts, fmt.Sprintf("pub const JSO_%s: JsObjectDef = JsObjectDef {", strings.ToUpper(ri(o.Name))))
		consts = append(consts, fmt.Sprintf("    id: JsObjectId::%s,", rv(o.Name)))
		consts = append(consts, fmt.Sprintf(`    name: "%s",`, o.Name))
		consts = append(consts, fmt.Sprintf("    members: &[%s],", memberStr))
		consts = append(consts, "};", "")
	}

	L = append(L, "")
	L = append(L, "impl JsObjectRegistry {",
		"    pub fn by_name(name: &str) -> Option<&'static JsObjectDef> {",
		"        match name {")
	L = append(L, matchArms...)
	L = append(L, `            _ => None,`,
		`        }`,
		`    }`,
		`}`)
	return strings.Join(append(L[:len(L)-5], append(consts, L[len(L)-5:]...)...), "\n")
}

func genJSAbstractOps(js JSCatalog) string {
	var L []string
	L = append(L, "//! ECMAScript Abstract Operations -- generated from ECMA-262.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-browser",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum AbstractOpId {",
		"    Unspecified = 0,")
	for i, op := range js.AbstractOps {
		L = append(L, fmt.Sprintf("    %s = %d,", rv(op.Name), i+1))
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone)]",
		"pub struct AbstractOpDef {",
		"    pub id: AbstractOpId,",
		"    pub name: &'static str,",
		"    pub params: &'static [&'static str],",
		"}", "",
		"pub struct AbstractOpRegistry;")

	var matchArms []string
	var consts []string
	for _, op := range js.AbstractOps {
		var cleanPS []string
		for _, p := range op.Parameters {
			p = strings.ReplaceAll(p, `\`, `\\`)
			cleanPS = append(cleanPS, fmt.Sprintf(`"%s"`, p))
		}
		plist := strings.Join(cleanPS, ", ")
		matchArms = append(matchArms, fmt.Sprintf(`            "%s" => Some(&AOP_%s),`, op.Name, strings.ToUpper(ri(op.Name))))
		consts = append(consts, fmt.Sprintf("pub const AOP_%s: AbstractOpDef = AbstractOpDef {", strings.ToUpper(ri(op.Name))))
		consts = append(consts, fmt.Sprintf("    id: AbstractOpId::%s,", rv(op.Name)))
		consts = append(consts, fmt.Sprintf(`    name: "%s",`, op.Name))
		consts = append(consts, fmt.Sprintf("    params: &[%s],", plist))
		consts = append(consts, "};", "")
	}

	L = append(L, "")
	L = append(L, "impl AbstractOpRegistry {",
		"    pub fn by_name(name: &str) -> Option<&'static AbstractOpDef> {",
		"        match name {")
	L = append(L, matchArms...)
	L = append(L, `            _ => None,`,
		`        }`,
		`    }`,
		`}`)
	return strings.Join(append(L[:len(L)-5], append(consts, L[len(L)-5:]...)...), "\n")
}

func main() {
	if len(os.Args) < 5 {
		fmt.Fprintln(os.Stderr, "Usage: generate-browser <html.json> <css.json> <js.json> <out_dir>")
		os.Exit(1)
	}

	var html HTMLCatalog
	if d, err := os.ReadFile(os.Args[1]); err == nil {
		json.Unmarshal(d, &html)
	}
	var css CSSCatalog
	if d, err := os.ReadFile(os.Args[2]); err == nil {
		json.Unmarshal(d, &css)
	}
	var js JSCatalog
	if d, err := os.ReadFile(os.Args[3]); err == nil {
		json.Unmarshal(d, &js)
	}
	outDir := os.Args[4]

	// Cargo.toml
	w(filepath.Join(outDir, "Cargo.toml"), `[package]
name = "edgerun-browser"
version = "0.1.0"
edition.workspace = true
license.workspace = true
publish = false
description = "Browser engine types generated from web specifications"

[dependencies]
`)

	// lib.rs
	w(filepath.Join(outDir, "src", "lib.rs"), `//! edgerun-browser — Browser engine types from web specifications.
//! Generated from WHATWG/W3C/ECMA specs.
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod element_registry;
pub mod tokenizer_states;
pub mod tree_builder;
pub mod property_registry;
pub mod value_types;
pub mod at_rule_registry;
pub mod js_object_registry;
pub mod js_abstract_ops;
`)

	w(filepath.Join(outDir, "src", "element_registry.rs"), genElementRegistry(html))
	w(filepath.Join(outDir, "src", "tokenizer_states.rs"), genTokenizerStates())
	w(filepath.Join(outDir, "src", "tree_builder.rs"), genTreeBuilder())
	w(filepath.Join(outDir, "src", "property_registry.rs"), genPropertyRegistry(css))
	w(filepath.Join(outDir, "src", "value_types.rs"), genValueTypes())
	w(filepath.Join(outDir, "src", "at_rule_registry.rs"), genAtRuleRegistry(css))
	w(filepath.Join(outDir, "src", "js_object_registry.rs"), genJSObjectRegistry(js))
	w(filepath.Join(outDir, "src", "js_abstract_ops.rs"), genJSAbstractOps(js))

	fmt.Println("\nDone.")
}
