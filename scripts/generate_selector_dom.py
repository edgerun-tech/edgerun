#!/usr/bin/env python3
"""
Generate edgerun-selectors and edgerun-dom crates from spec catalogs.

Usage: python3 scripts/generate_selector_dom.py \
  scripts/selector_catalog.json scripts/dom_catalog.json \
  crates/edgerun-selectors crates/edgerun-dom
"""
import json, os, re, sys

def load(p):
    with open(p) as f: return json.load(f)

def ri(n): return n.replace("-","_")
def rv(n): return "".join(p.capitalize() for p in n.replace("-","_").split("_"))

def w(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f: f.write(content)

# ===========================================================================
# SELECTORS CRATE
# ===========================================================================
def gen_selectors(sel):
    files = {}
    # Cargo.toml
    files["Cargo.toml"] = '[package]\nname = "edgerun-selectors"\nversion = "0.1.0"\nedition.workspace = true\nlicense.workspace = true\npublish = false\ndescription = "CSS Selectors Level 4 types generated from W3C spec"\n\n[dependencies]\n'

    # lib.rs
    files["src/lib.rs"] = '''//! edgerun-selectors — CSS Selectors Level 4 types.
//! Generated from W3C Selectors Level 4 specification.
#![cfg_attr(not(test), no_std)]

pub mod pseudo_classes;
pub mod pseudo_elements;
pub mod combinators;
pub mod attr_selectors;
pub mod specificity;
'''

    # pseudo_classes.rs
    pcs = sel["pseudo_classes"]
    L = ["//! Pseudo-class selectors \u2014 generated from Selectors Level 4.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_selector_dom.py", "",
         "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
         "pub enum PseudoClass {"]
    for pc in pcs:
        args = f' ({pc["arguments"]})' if pc["arguments"] else ""
        L.append(f"    /// {pc['name']}{args}")
        L.append(f"    {rv(pc['name'][1:])},")
    L.extend(["}", ""])

    L.append("#[derive(Debug, Clone, PartialEq, Eq)]")
    L.append("pub enum PseudoClassSelector {")
    for pc in pcs:
        L.append(f"    {rv(pc['name'][1:])},")
    L.extend(["}", ""])

    L.append("impl PseudoClassSelector {")
    L.append("    pub fn from_name(name: &str) -> Option<Self> {")
    L.append("        match name {")
    for pc in pcs:
        L.append(f'            "{pc["name"]}" => Some(PseudoClassSelector::{rv(pc["name"][1:])}),')
    L.extend(['            _ => None,', '        }', '    }', '}'])
    files["src/pseudo_classes.rs"] = "\n".join(L)

    # pseudo_elements.rs
    pes = sel["pseudo_elements"]
    L = ["//! Pseudo-element selectors \u2014 generated from Selectors Level 4.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_selector_dom.py", "",
         "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
         "pub enum PseudoElement {"]
    for pe in pes:
        L.append(f"    /// {pe['name']}")
        L.append(f"    {rv(pe['name'][2:])},")
    L.extend(["}", ""])

    L.append("impl PseudoElement {")
    L.append("    pub fn from_name(name: &str) -> Option<Self> {")
    L.append("        match name {")
    for pe in pes:
        L.append(f'            "{pe["name"]}" => Some(PseudoElement::{rv(pe["name"][2:])}),')
    L.extend(['            _ => None,', '        }', '    }', '}'])
    files["src/pseudo_elements.rs"] = "\n".join(L)

    # combinators.rs
    combs = sel["combinators"]
    L = ["//! Selector combinators \u2014 from Selectors Level 4.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_selector_dom.py", "",
         "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
         "pub enum Combinator {"]
    for c in combs:
        L.append(f"    /// {c['syntax']} \u2014 {c['description']}")
        L.append(f"    {rv(c['name'])},")
    L.extend(["}", ""])

    L.append("impl Combinator {")
    L.append("    pub fn from_char(c: char) -> Option<Self> {")
    L.append("        match c {")
    for c in combs:
        if c["char"]:
            L.append(f'            \'{c["char"]}\' => Some(Combinator::{rv(c["name"])}),')
    L.extend(['            _ => None,', '        }', '    }', '}'])
    files["src/combinators.rs"] = "\n".join(L)

    # attr_selectors.rs
    ops = sel["attribute_selector_operators"]
    L = ["//! Attribute selector operators \u2014 from Selectors Level 4.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_selector_dom.py", "",
         "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
         "pub enum AttrMatchOp {"]
    for op in ops:
        if "operator" in op:
            L.append(f"    /// {op['operator']} \u2014 {op['description']}")
            L.append(f"    {rv(op['name'])},")
    L.extend(["}", ""])

    L.append("/// Modifier for case sensitivity.")
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]")
    L.append("pub enum CaseSensitivity {")
    L.append("    Default,")
    for op in ops:
        if "modifier" in op:
            L.append(f"    /// {op['modifier']} \u2014 {op['description']}")
            L.append(f"    {rv(op['name'])},")
    L.extend(["}", ""])

    L.append("/// An attribute selector: [attr], [attr=\"val\"], [attr^=\"val\"], etc.")
    L.append("#[derive(Debug, Clone, PartialEq, Eq)]")
    L.append("pub struct AttrSelector {")
    L.append("    pub name: String,")
    L.append("    pub op: Option<AttrMatchOp>,")
    L.append("    pub value: Option<String>,")
    L.append("    pub case: CaseSensitivity,")
    L.append("}")
    files["src/attr_selectors.rs"] = "\n".join(L)

    # specificity.rs
    spec = sel["specificity"]
    L = ["//! Selector specificity \u2014 from Selectors Level 4.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_selector_dom.py", "",
         "/// Specificity triple (A, B, C) as defined in Selectors Level 4.",
         "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]",
         "pub struct Specificity(pub u32, pub u32, pub u32);", "",
         "impl Specificity {"]
    for s in spec:
        L.append(f"    /// {s['component']}: {s['description']}")
    L.extend([
        "",
        "    pub fn add_id(&mut self) { self.0 += 1; }",
        "    pub fn add_class(&mut self) { self.1 += 1; }",
        "    pub fn add_type(&mut self) { self.2 += 1; }",
        "",
        "    /// Check if this specificity is greater than another.",
        "    pub fn gt(&self, other: &Self) -> bool {",
        "        self.0 > other.0 || (self.0 == other.0 && (self.1 > other.1 || (self.1 == other.1 && self.2 > other.2)))",
        "    }",
        "}",
    ])
    files["src/specificity.rs"] = "\n".join(L)

    return files

# ===========================================================================
# DOM CRATE
# ===========================================================================
def gen_dom(dom):
    files = {}
    files["Cargo.toml"] = '[package]\nname = "edgerun-dom"\nversion = "0.1.0"\nedition.workspace = true\nlicense.workspace = true\npublish = false\ndescription = "DOM types generated from WHATWG DOM Living Standard"\n\n[dependencies]\n'

    files["src/lib.rs"] = '''//! edgerun-dom — DOM types from WHATWG DOM Living Standard.
//! Generated from DOM specification.
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{string::String, vec::Vec};

pub mod interfaces;
pub mod events;
'''

    # interfaces.rs
    ifaces = dom["interfaces"]
    L = ["//! DOM Interface definitions \u2014 generated from WHATWG DOM Living Standard.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_selector_dom.py",
         "",
         "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
         "pub enum InterfaceId {"]
    for i, iface in enumerate(ifaces):
        L.append(f"    {rv(iface['name'])} = {i+1},")
    L.extend(["}", ""])

    # Generate trait for each interface
    for iface in ifaces:
        name = iface["name"]
        parent = iface.get("parent", "")
        attrs = iface.get("attributes", [])
        methods = iface.get("methods", [])
        L.append(f"/// DOM interface: {name}" + (f" extends {parent}" if parent else ""))
        base = f"pub trait {name}"
        if parent and parent != "EventTarget":
            base += f": {parent}"
        L.append(f"{base} {{")
        for attr in attrs:
            atype = attr.get("type", "unknown")
            L.append(f"    fn {ri(attr['name'])}(&self) -> {atype};")
        for method in methods:
            params = ", ".join(f"{ri(p['name'])}: {p['type']}" for p in method.get("parameters", []))
            ret = method.get("return_type", "bool")
            if ret in ("bool", "void"):
                pass  # fine
            elif ret == "Node":
                ret = "&'a dyn Node"
                L.append(f"    fn {ri(method['name'])}<'a>(&'a self{', ' + params if params else ''}) -> {ret};")
                continue
            L.append(f"    fn {ri(method['name'])}(&self{', ' + params if params else ''}) -> {ret};")
        L.extend(["}", ""])

    # Node trait (special base)
    L.extend([
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
        "",
    ])

    files["src/interfaces.rs"] = "\n".join(L)

    # events.rs
    events = dom["events"]
    L = ["//! DOM Event types \u2014 generated from WHATWG DOM Living Standard.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_selector_dom.py", "",
         "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
         "pub enum EventPhase {",
         "    None = 0,",
         "    CapturingPhase = 1,",
         "    AtTarget = 2,",
         "    BubblingPhase = 3,",
         "}", "",
         "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
         "pub enum EventType {"]
    for evt in events:
        L.append(f"    {rv(evt['name'])},")
    L.extend(["}", ""])

    L.append("impl EventType {")
    L.append("    pub fn as_str(&self) -> &'static str {")
    L.append("        match self {")
    for evt in events:
        L.append(f'            EventType::{rv(evt["name"])} => "{evt["name"]}",')
    L.extend(['        }', '    }', '}', ''])

    L.append("impl EventType {")
    L.append("    pub fn from_str(name: &str) -> Option<Self> {")
    L.append("        match name {")
    for evt in events:
        L.append(f'            "{evt["name"]}" => Some(EventType::{rv(evt["name"])}),')
    L.extend(['            _ => None,', '        }', '    }', '}', ''])

    files["src/events.rs"] = "\n".join(L)

    return files

# ===========================================================================
def main():
    if len(sys.argv) < 5:
        print(f"Usage: {sys.argv[0]} <selector.json> <dom.json> <out-selectors> <out-dom>", file=sys.stderr)
        sys.exit(1)

    sel = load(sys.argv[1])
    dom = load(sys.argv[2])
    out_sel = sys.argv[3]
    out_dom = sys.argv[4]

    for rel, content in gen_selectors(sel).items():
        p = os.path.join(out_sel, rel)
        w(p, content)
        print(f"  selectors/{rel}: {content.count(chr(10))} lines")

    for rel, content in gen_dom(dom).items():
        p = os.path.join(out_dom, rel)
        w(p, content)
        print(f"  dom/{rel}: {content.count(chr(10))} lines")

    print("\nDone.")

if __name__ == "__main__":
    main()
