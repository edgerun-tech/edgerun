// Generate edgerun-selectors and edgerun-dom crates from spec catalogs.
//
// Usage: go run ./cmd/generate-selector-dom \
//   scripts/selector_catalog.json scripts/dom_catalog.json \
//   crates/edgerun-selectors crates/edgerun-dom
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

type SelectorCatalog struct {
	PseudoClasses   []PseudoClass   `json:"pseudo_classes"`
	PseudoElements  []PseudoElement `json:"pseudo_elements"`
	Combinators     []Combinator    `json:"combinators"`
	AttrOps         []AttrOp        `json:"attribute_selector_operators"`
	Specificity     []Specificity   `json:"specificity"`
}

type PseudoClass struct {
	Name      string `json:"name"`
	Arguments string `json:"arguments"`
}

type PseudoElement struct {
	Name string `json:"name"`
}

type Combinator struct {
	Name        string `json:"name"`
	Syntax      string `json:"syntax"`
	Char        string `json:"char"`
	Description string `json:"description"`
}

type AttrOp struct {
	Operator   string `json:"operator,omitempty"`
	Name       string `json:"name"`
	Description string `json:"description"`
	Modifier   string `json:"modifier,omitempty"`
}

type Specificity struct {
	Component   string `json:"component"`
	Name        string `json:"name"`
	Description string `json:"description"`
}

type DOMCatalog struct {
	Interfaces []DOMInterface `json:"interfaces"`
	Events     []DOMEvent     `json:"events"`
}

type DOMInterface struct {
	Name       string     `json:"name"`
	Parent     string     `json:"parent"`
	Attributes []IDLAttr  `json:"attributes"`
	Methods    []IDLMethod `json:"methods"`
}

type IDLAttr struct {
	Name     string `json:"name"`
	Type     string `json:"type"`
	Readonly bool   `json:"readonly"`
}

type IDLMethod struct {
	Name       string     `json:"name"`
	ReturnType string     `json:"return_type"`
	Parameters []IDLParam `json:"parameters"`
}

type IDLParam struct {
	Type string `json:"type"`
	Name string `json:"name"`
}

type DOMEvent struct {
	Name string `json:"name"`
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

func w(path, content string) {
	os.MkdirAll(filepath.Dir(path), 0755)
	os.WriteFile(path, []byte(content), 0644)
	fmt.Printf("  %s: %d lines\n", path, strings.Count(content, "\n"))
}

func genSelectors(sel SelectorCatalog) map[string]string {
	files := make(map[string]string)

	files["Cargo.toml"] = `[package]
name = "edgerun-selectors"
version = "0.1.0"
edition.workspace = true
license.workspace = true
publish = false
description = "CSS Selectors Level 4 types generated from W3C spec"

[dependencies]
`

	files["src/lib.rs"] = `//! edgerun-selectors -- CSS Selectors Level 4 types.
//! Generated from W3C Selectors Level 4 specification.
#![cfg_attr(not(test), no_std)]

pub mod pseudo_classes;
pub mod pseudo_elements;
pub mod combinators;
pub mod attr_selectors;
pub mod specificity;
`

	// pseudo_classes.rs
	var L []string
	L = append(L, "//! Pseudo-class selectors -- generated from Selectors Level 4.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-selector-dom",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
		"pub enum PseudoClass {")
	for _, pc := range sel.PseudoClasses {
		args := pc.Arguments
		if args != "" {
			args = " (" + args + ")"
		}
		L = append(L, fmt.Sprintf("    /// %s%s", pc.Name, args))
		L = append(L, fmt.Sprintf("    %s,", rv(pc.Name[1:])))
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone, PartialEq, Eq)]",
		"pub enum PseudoClassSelector {")
	for _, pc := range sel.PseudoClasses {
		L = append(L, fmt.Sprintf("    %s,", rv(pc.Name[1:])))
	}
	L = append(L, "}", "",
		"impl PseudoClassSelector {",
		"    pub fn from_name(name: &str) -> Option<Self> {",
		"        match name {")
	for _, pc := range sel.PseudoClasses {
		L = append(L, fmt.Sprintf(`            "%s" => Some(PseudoClassSelector::%s),`, pc.Name, rv(pc.Name[1:])))
	}
	L = append(L, `            _ => None,`,
		`        }`,
		`    }`,
		`}`)
	files["src/pseudo_classes.rs"] = strings.Join(L, "\n")

	// pseudo_elements.rs
	L = nil
	L = append(L, "//! Pseudo-element selectors -- generated from Selectors Level 4.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-selector-dom",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
		"pub enum PseudoElement {")
	for _, pe := range sel.PseudoElements {
		L = append(L, fmt.Sprintf("    /// %s", pe.Name))
		L = append(L, fmt.Sprintf("    %s,", rv(pe.Name[2:])))
	}
	L = append(L, "}", "",
		"impl PseudoElement {",
		"    pub fn from_name(name: &str) -> Option<Self> {",
		"        match name {")
	for _, pe := range sel.PseudoElements {
		L = append(L, fmt.Sprintf(`            "%s" => Some(PseudoElement::%s),`, pe.Name, rv(pe.Name[2:])))
	}
	L = append(L, `            _ => None,`,
		`        }`,
		`    }`,
		`}`)
	files["src/pseudo_elements.rs"] = strings.Join(L, "\n")

	// combinators.rs
	L = nil
	L = append(L, "//! Selector combinators -- from Selectors Level 4.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-selector-dom",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum Combinator {")
	for _, c := range sel.Combinators {
		L = append(L, fmt.Sprintf("    /// %s -- %s", c.Syntax, c.Description))
		L = append(L, fmt.Sprintf("    %s,", rv(c.Name)))
	}
	L = append(L, "}", "",
		"impl Combinator {",
		"    pub fn from_char(c: char) -> Option<Self> {",
		"        match c {")
	for _, c := range sel.Combinators {
		if c.Char != "" {
			L = append(L, fmt.Sprintf(`            '%s' => Some(Combinator::%s),`, c.Char, rv(c.Name)))
		}
	}
	L = append(L, `            _ => None,`,
		`        }`,
		`    }`,
		`}`)
	files["src/combinators.rs"] = strings.Join(L, "\n")

	// attr_selectors.rs
	L = nil
	L = append(L, "//! Attribute selector operators -- from Selectors Level 4.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-selector-dom",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum AttrMatchOp {")
	for _, op := range sel.AttrOps {
		if op.Operator != "" || op.Name == "exists" {
			L = append(L, fmt.Sprintf("    /// %s -- %s", op.Operator, op.Description))
			L = append(L, fmt.Sprintf("    %s,", rv(op.Name)))
		}
	}
	L = append(L, "}", "",
		"/// Modifier for case sensitivity.",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]",
		"pub enum CaseSensitivity {",
		"    Default,")
	for _, op := range sel.AttrOps {
		if op.Modifier != "" {
			L = append(L, fmt.Sprintf("    /// %s -- %s", op.Modifier, op.Description))
			L = append(L, fmt.Sprintf("    %s,", rv(op.Name)))
		}
	}
	L = append(L, "}", "",
		"/// An attribute selector: [attr], [attr=\"val\"], [attr^=\"val\"], etc.",
		"#[derive(Debug, Clone, PartialEq, Eq)]",
		"pub struct AttrSelector {",
		"    pub name: String,",
		"    pub op: Option<AttrMatchOp>,",
		"    pub value: Option<String>,",
		"    pub case: CaseSensitivity,",
		"}")
	files["src/attr_selectors.rs"] = strings.Join(L, "\n")

	// specificity.rs
	L = nil
	L = append(L, "//! Selector specificity -- from Selectors Level 4.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-selector-dom",
		"",
		"/// Specificity triple (A, B, C) as defined in Selectors Level 4.",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]",
		"pub struct Specificity(pub u32, pub u32, pub u32);",
		"",
		"impl Specificity {")
	for _, s := range sel.Specificity {
		L = append(L, fmt.Sprintf("    /// %s: %s", s.Component, s.Description))
	}
	L = append(L, "",
		"    pub fn add_id(&mut self) { self.0 += 1; }",
		"    pub fn add_class(&mut self) { self.1 += 1; }",
		"    pub fn add_type(&mut self) { self.2 += 1; }",
		"",
		"    /// Check if this specificity is greater than another.",
		"    pub fn gt(&self, other: &Self) -> bool {",
		"        self.0 > other.0 || (self.0 == other.0 && (self.1 > other.1 || (self.1 == other.1 && self.2 > other.2)))",
		"    }",
		"}")
	files["src/specificity.rs"] = strings.Join(L, "\n")

	return files
}

func genDOM(dom DOMCatalog) map[string]string {
	files := make(map[string]string)

	files["Cargo.toml"] = `[package]
name = "edgerun-dom"
version = "0.1.0"
edition.workspace = true
license.workspace = true
publish = false
description = "DOM types generated from WHATWG DOM Living Standard"

[dependencies]
`

	files["src/lib.rs"] = `//! edgerun-dom -- DOM types from WHATWG DOM Living Standard.
//! Generated from DOM specification.
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{string::String, vec::Vec};

pub mod interfaces;
pub mod events;
`

	// interfaces.rs
	var L []string
	L = append(L, "//! DOM Interface definitions -- generated from WHATWG DOM Living Standard.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-selector-dom",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum InterfaceId {")
	for i, iface := range dom.Interfaces {
		L = append(L, fmt.Sprintf("    %s = %d,", rv(iface.Name), i+1))
	}
	L = append(L, "}")

	// Generate trait for each interface
	for _, iface := range dom.Interfaces {
		name := iface.Name
		parent := iface.Parent
		L = append(L, "")
		if parent != "" {
			L = append(L, fmt.Sprintf("/// DOM interface: %s extends %s", name, parent))
		} else {
			L = append(L, fmt.Sprintf("/// DOM interface: %s", name))
		}
		base := fmt.Sprintf("pub trait %s", name)
		if parent != "" && parent != "EventTarget" {
			base += fmt.Sprintf(": %s", rv(parent))
		}
		L = append(L, base+" {")
		for _, attr := range iface.Attributes {
			L = append(L, fmt.Sprintf("    fn %s(&self) -> %s;", ri(attr.Name), attr.Type))
		}
		for _, method := range iface.Methods {
			params := ""
			for _, p := range method.Parameters {
				if params != "" {
					params += ", "
				}
				params += fmt.Sprintf("%s: %s", ri(p.Name), p.Type)
			}
			ret := method.ReturnType
			L = append(L, fmt.Sprintf("    fn %s(&self%s) -> %s;", ri(method.Name), func() string {
				if params != "" {
					return ", " + params
				}
				return ""
			}(), ret))
		}
		L = append(L, "}")
	}

	// Node trait
	L = append(L, "",
		"/// Base trait for all DOM nodes.",
		"pub trait Node {",
		"    fn node_type(&self) -> NodeType;",
		"    fn node_name(&self) -> &str;",
		"    fn parent_node(&self) -> Option<&dyn Node>;",
		"    fn first_child(&self) -> Option<&dyn Node>;",
		"    fn last_child(&self) -> Option<&dyn Node>;",
		"    fn previous_sibling(&self) -> Option<&dyn Node>;",
		"    fn next_sibling(&self) -> Option<&dyn Node>;",
		"    fn child_nodes(&self) -> Vec<&dyn Node>;",
		"}",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum NodeType {",
		"    Element = 1,",
		"    Text = 3,",
		"    Comment = 8,",
		"    Document = 9,",
		"    DocumentType = 10,",
		"    DocumentFragment = 11,",
		"}",
		"")
	files["src/interfaces.rs"] = strings.Join(L, "\n")

	// events.rs
	L = nil
	L = append(L, "//! DOM Event types -- generated from WHATWG DOM Living Standard.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-selector-dom",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum EventPhase {",
		"    None = 0,",
		"    CapturingPhase = 1,",
		"    AtTarget = 2,",
		"    BubblingPhase = 3,",
		"}",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum EventType {")
	for _, evt := range dom.Events {
		L = append(L, fmt.Sprintf("    %s,", rv(evt.Name)))
	}
	L = append(L, "}", "",
		"impl EventType {",
		"    pub fn as_str(&self) -> &'static str {",
		"        match self {")
	for _, evt := range dom.Events {
		L = append(L, fmt.Sprintf(`            EventType::%s => "%s",`, rv(evt.Name), evt.Name))
	}
	L = append(L, `        }`,
		`    }`,
		`}`,
		"",
		"impl EventType {",
		"    pub fn from_str(name: &str) -> Option<Self> {",
		"        match name {")
	for _, evt := range dom.Events {
		L = append(L, fmt.Sprintf(`            "%s" => Some(EventType::%s),`, evt.Name, rv(evt.Name)))
	}
	L = append(L, `            _ => None,`,
		`        }`,
		`    }`,
		`}`,
		"")
	files["src/events.rs"] = strings.Join(L, "\n")

	return files
}

func main() {
	if len(os.Args) < 5 {
		fmt.Fprintln(os.Stderr, "Usage: generate-selector-dom <selector.json> <dom.json> <out-selectors> <out-dom>")
		os.Exit(1)
	}

	selData, err := os.ReadFile(os.Args[1])
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
	domData, err := os.ReadFile(os.Args[2])
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}

	var sel SelectorCatalog
	json.Unmarshal(selData, &sel)
	var dom DOMCatalog
	json.Unmarshal(domData, &dom)

	outSel := os.Args[3]
	outDom := os.Args[4]

	for rel, content := range genSelectors(sel) {
		p := filepath.Join(outSel, rel)
		w(p, content)
	}

	for rel, content := range genDOM(dom) {
		p := filepath.Join(outDom, rel)
		w(p, content)
	}

	fmt.Println("\nDone.")
}
