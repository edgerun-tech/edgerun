//! CSS Cascade Resolution Engine — generated from css_cascade.proto + selectors.
//!
//! Generated from:
//! - css_cascade.proto (Origins, Importance, Layers)
//! - edgerun-selectors (Specificity, Combinators, PseudoClasses, AttrSelectors)
//! - css_properties.proto (255 known properties)
//! - css_value_types.proto (66 value types)
//!
//! DO NOT EDIT. Regenerate with: scripts/generate_css_cascade.py
#![cfg_attr(not(test), no_std)]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod cascade {
                include!("gen/edgerun.v0.css.cascade.rs");
            }
        }
    }
}
pub use edgerun::v0::css::cascade::{Origin, Importance, CascadeLayer, ScopeProximity};

// ─── Selector Types ───

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorComponent {
    Type(String),
    Universal,
    Class(String),
    Id(String),
    Attr { name: String, op: Option<AttrOp>, value: Option<String> },
    PseudoClass(PseudoClass),
    PseudoElement(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrOp {
    Exists, Equals, ContainsWord, PrefixHyphen, Prefix, Suffix, Contains,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PseudoClass {
    Root, Empty, FirstChild, LastChild, OnlyChild,
    NthChild(i32, i32), NthLastChild(i32, i32),
    NthOfType(i32, i32), NthLastOfType(i32, i32),
    FirstOfType, LastOfType, OnlyOfType,
    Hover, Active, Focus, Visited, Link,
    Lang(String), DirLTR, DirRTL,
}

#[derive(Debug, Clone)]
pub struct CompoundSelector { pub components: Vec<SelectorComponent> }

#[derive(Debug, Clone)]
pub struct Selector { pub sequence: Vec<SelectorSequence> }

#[derive(Debug, Clone)]
pub struct SelectorSequence {
    pub compound: CompoundSelector,
    pub combinator: Option<Combinator>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator { Descendant, Child, NextSibling, SubsequentSibling, Column }

// ─── Specificity ───

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Specificity(pub u32, pub u32, pub u32);

impl Specificity {
    pub fn zero() -> Self { Self(0, 0, 0) }
    pub fn add_id(&mut self) { self.0 += 1; }
    pub fn add_class(&mut self) { self.1 += 1; }
    pub fn add_attr(&mut self) { self.1 += 1; }
    pub fn add_pseudo_class(&mut self) { self.1 += 1; }
    pub fn add_pseudo_element(&mut self) { self.2 += 1; }
    pub fn add_type(&mut self) { self.2 += 1; }
    pub fn add_highest(&mut self, other: Self) { if other > *self { *self = other; } }
}

// ─── Cascade Key ───

#[derive(Debug, Clone)]
pub struct CascadeKey {
    pub origin: Origin,
    pub importance: Importance,
    pub layer_order: u32,
    pub specificity: Specificity,
    pub order: u32,
}

impl PartialEq for CascadeKey {
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin && self.importance == other.importance
            && self.layer_order == other.layer_order && self.specificity == other.specificity
            && self.order == other.order
    }
}
impl Eq for CascadeKey {}

impl PartialOrd for CascadeKey { fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> { Some(self.cmp(other)) } }

impl Ord for CascadeKey {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        use core::cmp::Ordering;
        let self_oi = origin_importance_rank(self.origin, self.importance);
        let other_oi = origin_importance_rank(other.origin, other.importance);
        match self_oi.cmp(&other_oi) { Ordering::Equal => {} ord => return ord }
        match self.layer_order.cmp(&other.layer_order) { Ordering::Equal => {} ord => return ord }
        match self.specificity.cmp(&other.specificity) { Ordering::Equal => {} ord => return ord }
        self.order.cmp(&other.order)
    }
}

fn origin_importance_rank(origin: Origin, importance: Importance) -> u32 {
    let imp = match importance { Importance::Normal | Importance::Unspecified => 0, Importance::Important => 1 };
    let orig = match origin {
        Origin::Unspecified | Origin::Ua => 0, Origin::User => 1,
        Origin::Author => 2, Origin::Animation => 3, Origin::Transition => 4,
    };
    orig * 10 + imp
}

// ─── Declarations and Rules ───

#[derive(Debug, Clone)]
pub struct CascadeDeclaration {
    pub key: CascadeKey,
    pub property: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct CascadeRule {
    pub selector: Selector,
    pub declarations: BTreeMap<String, String>,
    pub origin: Origin,
    pub importance: Importance,
    pub layer_order: u32,
}

#[derive(Debug, Clone)]
pub struct CascadeStylesheet { pub rules: Vec<CascadeRule> }

impl CascadeStylesheet {
    pub fn new() -> Self { Self { rules: Vec::new() } }
    pub fn add_rule(&mut self, selector: Selector, declarations: BTreeMap<String, String>,
                    origin: Origin, importance: Importance, layer_order: u32) {
        self.rules.push(CascadeRule { selector, declarations, origin, importance, layer_order });
    }

    pub fn resolve(&self, element: &DomElement) -> BTreeMap<String, String> {
        let mut all_decls: Vec<CascadeDeclaration> = Vec::new();
        let mut order_counter: u32 = 0;
        for rule in &self.rules {
            let specificity = compute_specificity(&rule.selector);
            if selector_matches_dom(&rule.selector, element) {
                for (property, value) in &rule.declarations {
                    all_decls.push(CascadeDeclaration {
                        key: CascadeKey { origin: rule.origin, importance: rule.importance,
                            layer_order: rule.layer_order, specificity, order: order_counter },
                        property: property.clone(), value: value.clone(),
                    });
                }
                order_counter += 1;
            }
        }
        all_decls.sort_by(|a, b| a.key.cmp(&b.key));
        let mut result = BTreeMap::new();
        for decl in all_decls { result.insert(decl.property, decl.value); }
        result
    }
}

// ─── DOM Element ───

pub struct DomElement<'a> {
    pub tag_name: &'a str,
    pub id: Option<&'a str>,
    pub classes: &'a [&'a str],
    pub attributes: &'a [(&'a str, &'a str)],
    pub parent: Option<&'a DomElement<'a>>,
    pub prev_siblings: &'a [DomElement<'a>],
    pub child_index: usize,
    pub type_index: usize,
}

// ─── Selector Matching ───

fn selector_matches_dom(selector: &Selector, element: &DomElement) -> bool {
    if selector.sequence.is_empty() { return true; }
    let mut current_element: Option<&DomElement> = Some(element);
    let mut seq_iter = selector.sequence.iter();
    if let Some(first_seq) = seq_iter.next() {
        if !compound_matches(&first_seq.compound, element) { return false; }
        for seq in seq_iter {
            if let Some(combinator) = seq.combinator {
                current_element = match combinator {
                    Combinator::Child => current_element.and_then(|e| e.parent).filter(|p| compound_matches(&seq.compound, p)),
                    Combinator::Descendant => current_element.and_then(|e| find_ancestor_matching(e, &seq.compound)),
                    Combinator::NextSibling => find_prev_element_sibling(current_element).filter(|p| compound_matches(&seq.compound, p)),
                    Combinator::SubsequentSibling => find_any_prev_element_sibling(current_element, &seq.compound),
                    Combinator::Column => None,
                };
                if current_element.is_none() { return false; }
            }
        }
    }
    true
}

fn compound_matches(compound: &CompoundSelector, element: &DomElement) -> bool {
    for comp in &compound.components {
        match comp {
            SelectorComponent::Type(name) => { if element.tag_name != name.to_lowercase().as_str() { return false; } }
            SelectorComponent::Universal => {}
            SelectorComponent::Class(name) => { if !element.classes.iter().any(|c| c.eq_ignore_ascii_case(name)) { return false; } }
            SelectorComponent::Id(name) => { if element.id != Some(name) { return false; } }
            SelectorComponent::Attr { name, op, value } => { if !attr_matches(element.attributes, name, op, value) { return false; } }
            SelectorComponent::PseudoClass(pc) => { if !pseudo_class_matches(pc, element) { return false; } }
            SelectorComponent::PseudoElement(_) => {}
        }
    }
    true
}

fn attr_matches(attrs: &[(&str, &str)], name: &str, op: &Option<AttrOp>, value: &Option<String>) -> bool {
    let attr_val = attrs.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| *v);
    match op {
        None | Some(AttrOp::Exists) => attr_val.is_some(),
        Some(AttrOp::Equals) => attr_val.is_some() && value.as_ref().map(|v| attr_val.unwrap() == v.as_str()).unwrap_or(false),
        Some(AttrOp::ContainsWord) => attr_val.is_some() && value.as_ref().map(|v| attr_val.unwrap().split_whitespace().any(|w| w == v.as_str())).unwrap_or(false),
        Some(AttrOp::PrefixHyphen) => attr_val.is_some() && value.as_ref().map(|v| { let av = attr_val.unwrap(); av == v.as_str() || (av.starts_with(v.as_str()) && av.as_bytes().get(v.len()) == Some(&b'-')) }).unwrap_or(false),
        Some(AttrOp::Prefix) => attr_val.is_some() && value.as_ref().map(|v| attr_val.unwrap().starts_with(v.as_str())).unwrap_or(false),
        Some(AttrOp::Suffix) => attr_val.is_some() && value.as_ref().map(|v| attr_val.unwrap().ends_with(v.as_str())).unwrap_or(false),
        Some(AttrOp::Contains) => attr_val.is_some() && value.as_ref().map(|v| attr_val.unwrap().contains(v.as_str())).unwrap_or(false),
    }
}

fn pseudo_class_matches(_pc: &PseudoClass, _element: &DomElement) -> bool { true }

fn find_ancestor_matching<'a>(element: &'a DomElement<'a>, compound: &CompoundSelector) -> Option<&'a DomElement<'a>> {
    let mut current = element.parent;
    while let Some(el) = current { if compound_matches(compound, el) { return Some(el); } current = el.parent; }
    None
}

fn find_prev_element_sibling<'a>(current: Option<&'a DomElement<'a>>) -> Option<&'a DomElement<'a>> {
    current.and_then(|e| { if e.child_index > 0 { None } else { None } })
}

fn find_any_prev_element_sibling<'a>(current: Option<&'a DomElement<'a>>, _compound: &CompoundSelector) -> Option<&'a DomElement<'a>> {
    find_prev_element_sibling(current)
}

// ─── Specificity Computation ───

fn compute_specificity(selector: &Selector) -> Specificity {
    let mut spec = Specificity::zero();
    for seq in &selector.sequence {
        for comp in &seq.compound.components {
            match comp {
                SelectorComponent::Type(_) => spec.add_type(),
                SelectorComponent::Universal => {}
                SelectorComponent::Class(_) => spec.add_class(),
                SelectorComponent::Id(_) => spec.add_id(),
                SelectorComponent::Attr { .. } => spec.add_attr(),
                SelectorComponent::PseudoClass(_) => spec.add_pseudo_class(),
                SelectorComponent::PseudoElement(_) => spec.add_pseudo_element(),
            }
        }
    }
    spec
}

// ─── CSS Parser ───

pub fn parse_stylesheet(css: &str, origin: Origin) -> CascadeStylesheet {
    let mut sheet = CascadeStylesheet::new();
    let mut parser = CssParser { input: css, pos: 0 };
    let mut layer_counter: u32 = 0;
    while parser.skip_whitespace_and_check_end() {
        if parser.peek_str("@layer ") {
            parser.consume("@layer ");
            layer_counter += 1;
            if parser.consume("{") {
                while !parser.peek_str("}") && !parser.at_end() {
                    if let Some(rule) = parser.parse_rule(origin, layer_counter) {
                        sheet.add_rule(rule.selector, rule.declarations, rule.origin, rule.importance, rule.layer_order);
                    }
                }
                parser.consume("}");
            }
            continue;
        }
        if let Some(rule) = parser.parse_rule(origin, 0) {
            sheet.add_rule(rule.selector, rule.declarations, rule.origin, rule.importance, rule.layer_order);
        } else { break; }
    }
    sheet
}

struct CssParser<'a> { input: &'a str, pos: usize }

impl<'a> CssParser<'a> {
    fn peek_str(&self, s: &str) -> bool { self.input[self.pos..].starts_with(s) }
    fn consume(&mut self, s: &str) -> bool {
        if self.peek_str(s) { self.pos += s.len(); true } else { false }
    }
    fn at_end(&self) -> bool { self.pos >= self.input.len() }
    fn skip_whitespace_and_check_end(&mut self) -> bool {
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos].is_ascii_whitespace() { self.pos += 1; }
        !self.at_end()
    }
    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos].is_ascii_whitespace() { self.pos += 1; }
    }

    fn parse_rule(&mut self, origin: Origin, layer_order: u32) -> Option<ParsedRule> {
        let selector_str = self.parse_until("{").trim().to_string();
        if selector_str.is_empty() { return None; }
        if !self.consume("{") { return None; }
        let declarations = self.parse_declarations();
        // FIX: skip whitespace before checking for "}"
        self.skip_whitespace();
        if !self.consume("}") { return None; }
        let selector = parse_selector(&selector_str);
        let importance = if declarations.iter().any(|(_, v)| v.ends_with(" !important")) { Importance::Important } else { Importance::Normal };
        let declarations: BTreeMap<String, String> = declarations.into_iter()
            .map(|(k, v)| (k, v.replace(" !important", "").trim().to_string())).collect();
        Some(ParsedRule { selector, declarations, origin, importance, layer_order })
    }

    fn parse_until(&mut self, delim: &str) -> String {
        let start = self.pos;
        while self.pos < self.input.len() && !self.peek_str(delim) { self.pos += 1; }
        self.input[start..self.pos].to_string()
    }

    fn parse_declarations(&mut self) -> Vec<(String, String)> {
        let mut decls = Vec::new();
        while !self.peek_str("}") && !self.at_end() {
            self.skip_whitespace();
            if self.peek_str("}") || self.at_end() { break; }
            let prop = self.parse_until(":").trim().to_string();
            if !self.consume(":") { break; }
            let value = self.parse_until(";").trim().to_string();
            self.consume(";");
            if !prop.is_empty() && !value.is_empty() { decls.push((prop, value)); }
        }
        decls
    }
}

struct ParsedRule { selector: Selector, declarations: BTreeMap<String, String>, origin: Origin, importance: Importance, layer_order: u32 }

// ─── Selector Parser ───

fn parse_selector(selector_str: &str) -> Selector {
    let mut selector = Selector { sequence: Vec::new() };
    let mut current_compound = CompoundSelector { components: Vec::new() };
    let mut current_combinator: Option<Combinator> = None;
    let mut chars = selector_str.chars().peekable();
    let mut in_attr = false;
    while let Some(c) = chars.next() {
        if in_attr { if c == ']' { in_attr = false; } continue; }
        match c {
            '#' => { let mut id = String::new(); while let Some(&nc) = chars.peek() { if nc.is_alphanumeric() || nc == '-' || nc == '_' { id.push(nc); chars.next(); } else { break; } } current_compound.components.push(SelectorComponent::Id(id)); }
            '.' => { let mut cls = String::new(); while let Some(&nc) = chars.peek() { if nc.is_alphanumeric() || nc == '-' || nc == '_' { cls.push(nc); chars.next(); } else { break; } } current_compound.components.push(SelectorComponent::Class(cls)); }
            '[' => { in_attr = true; let mut a = String::new(); while let Some(&nc) = chars.peek() { if nc == ']' { break; } a.push(nc); chars.next(); } current_compound.components.push(parse_attr_selector(&a)); }
            ':' => {
                if chars.peek() == Some(&':') { chars.next(); let mut n = String::new(); while let Some(&nc) = chars.peek() { if nc.is_alphanumeric() || nc == '-' { n.push(nc); chars.next(); } else { break; } } current_compound.components.push(SelectorComponent::PseudoElement(n)); }
                else { let mut n = String::new(); while let Some(&nc) = chars.peek() { if nc.is_alphanumeric() || nc == '-' || nc == '(' || nc == ')' || nc == ' ' { n.push(nc); chars.next(); } else { break; } } current_compound.components.push(SelectorComponent::PseudoClass(parse_pseudo_class(n.trim()))); }
            }
            '>' => { if !current_compound.components.is_empty() { selector.sequence.push(SelectorSequence { compound: core::mem::replace(&mut current_compound, CompoundSelector { components: Vec::new() }), combinator: current_combinator.take() }); } current_combinator = Some(Combinator::Child); }
            '+' => { if !current_compound.components.is_empty() { selector.sequence.push(SelectorSequence { compound: core::mem::replace(&mut current_compound, CompoundSelector { components: Vec::new() }), combinator: current_combinator.take() }); } current_combinator = Some(Combinator::NextSibling); }
            '~' => { if !current_compound.components.is_empty() { selector.sequence.push(SelectorSequence { compound: core::mem::replace(&mut current_compound, CompoundSelector { components: Vec::new() }), combinator: current_combinator.take() }); } current_combinator = Some(Combinator::SubsequentSibling); }
            ' ' => { if !current_compound.components.is_empty() && current_combinator.is_none() { current_combinator = Some(Combinator::Descendant); } }
            c if c.is_alphanumeric() || c == '-' || c == '_' => { let mut n = String::from(c); while let Some(&nc) = chars.peek() { if nc.is_alphanumeric() || nc == '-' || nc == '_' { n.push(nc); chars.next(); } else { break; } } current_compound.components.push(SelectorComponent::Type(n)); }
            '*' => { current_compound.components.push(SelectorComponent::Universal); }
            _ => {}
        }
    }
    if !current_compound.components.is_empty() { selector.sequence.push(SelectorSequence { compound: current_compound, combinator: current_combinator }); }
    selector.sequence.reverse();
    if let Some(first) = selector.sequence.first_mut() { first.combinator = None; }
    selector
}

fn parse_attr_selector(s: &str) -> SelectorComponent {
    let s = s.trim();
    if let Some(eq_pos) = s.find('=') {
        let name = s[..eq_pos].trim().to_string();
        let rest = s[eq_pos + 1..].trim();
        let op = if eq_pos > 0 { match s.as_bytes()[eq_pos - 1] { b'~' => AttrOp::ContainsWord, b'|' => AttrOp::PrefixHyphen, b'^' => AttrOp::Prefix, b'$' => AttrOp::Suffix, b'*' => AttrOp::Contains, _ => AttrOp::Equals } } else { AttrOp::Equals };
        let value = rest.trim_matches('"').trim_matches('\'').to_string();
        SelectorComponent::Attr { name, op: Some(op), value: Some(value) }
    } else { SelectorComponent::Attr { name: s.trim().to_string(), op: None, value: None } }
}

fn parse_pseudo_class(name: &str) -> PseudoClass {
    match name {
        "root" => PseudoClass::Root, "empty" => PseudoClass::Empty,
        "first-child" => PseudoClass::FirstChild, "last-child" => PseudoClass::LastChild, "only-child" => PseudoClass::OnlyChild,
        "first-of-type" => PseudoClass::FirstOfType, "last-of-type" => PseudoClass::LastOfType, "only-of-type" => PseudoClass::OnlyOfType,
        "hover" => PseudoClass::Hover, "active" => PseudoClass::Active, "focus" => PseudoClass::Focus,
        "visited" => PseudoClass::Visited, "link" => PseudoClass::Link,
        "dir(ltr)" => PseudoClass::DirLTR, "dir(rtl)" => PseudoClass::DirRTL,
        _ => { if let Some(lang) = name.strip_prefix("lang(") { PseudoClass::Lang(lang.trim_end_matches(')').to_string()) } else if let Some(nth) = name.strip_prefix("nth-child(") { parse_nth(nth.trim_end_matches(')')) } else { PseudoClass::FirstChild } }
    }
}

fn parse_nth(s: &str) -> PseudoClass {
    let s = s.trim();
    if s == "odd" { return PseudoClass::NthChild(2, 1); }
    if s == "even" { return PseudoClass::NthChild(2, 0); }
    let (a, b) = if let Some(n_pos) = s.find('n') {
        let a_str = &s[..n_pos];
        let a = if a_str.is_empty() || a_str == "+" { 1 } else if a_str == "-" { -1 } else { a_str.parse().unwrap_or(0) };
        let b_str = s[n_pos + 1..].trim().trim_start_matches('+');
        let b = if b_str.is_empty() { 0 } else { b_str.parse().unwrap_or(0) };
        (a, b)
    } else {
        (0, s.parse().unwrap_or(0))
    };
    PseudoClass::NthChild(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_specificity() { let mut s = Specificity::zero(); s.add_id(); s.add_class(); s.add_class(); s.add_type(); assert_eq!(s, Specificity(1, 2, 1)); }
    #[test]
    fn test_cascade_order() {
        let k1 = CascadeKey { origin: Origin::Author, importance: Importance::Normal, layer_order: 0, specificity: Specificity(0, 1, 0), order: 1 };
        let k2 = CascadeKey { origin: Origin::Author, importance: Importance::Normal, layer_order: 0, specificity: Specificity(0, 1, 0), order: 2 };
        assert!(k2 > k1);
    }
    #[test]
    fn test_important_wins() {
        let k1 = CascadeKey { origin: Origin::Author, importance: Importance::Important, layer_order: 0, specificity: Specificity(0, 0, 1), order: 1 };
        let k2 = CascadeKey { origin: Origin::Author, importance: Importance::Normal, layer_order: 0, specificity: Specificity(1, 0, 0), order: 2 };
        assert!(k1 > k2);
    }
    #[test]
    fn test_parse_stylesheet() {
        let css = "p { color: red; }";
        let sheet = parse_stylesheet(css, Origin::Author);
        assert_eq!(sheet.rules.len(), 1, "Should parse 1 rule");
        let classes: Vec<&str> = vec![];
        let el = DomElement { tag_name: "p", id: None, classes: &classes, attributes: &[], parent: None, prev_siblings: &[], child_index: 0, type_index: 0 };
        let styles = sheet.resolve(&el);
        assert_eq!(styles.get("color"), Some(&"red".to_string()));
    }
    #[test]
    fn test_parse_multiple_rules() {
        let css = "p { color: #AAA; } h2 { color: #BBB; font-size: 24px; }";
        let sheet = parse_stylesheet(css, Origin::Author);
        assert_eq!(sheet.rules.len(), 2);
    }
    #[test]
    fn test_parse_whitespace_around_braces() {
        let css = "  p  {  color : red ;  }  ";
        let sheet = parse_stylesheet(css, Origin::Author);
        assert_eq!(sheet.rules.len(), 1);
    }
    #[test]
    fn test_cascade_order_wins() {
        let css = "p { color: #AAAAAA; } p { color: #DDDDDD; }";
        let sheet = parse_stylesheet(css, Origin::Author);
        let classes: Vec<&str> = vec![];
        let el = DomElement { tag_name: "p", id: None, classes: &classes, attributes: &[], parent: None, prev_siblings: &[], child_index: 0, type_index: 0 };
        let styles = sheet.resolve(&el);
        assert_eq!(styles.get("color"), Some(&"#DDDDDD".to_string()), "Later rule should win");
    }
}
