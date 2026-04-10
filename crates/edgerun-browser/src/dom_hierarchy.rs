
//! DOM Node Trait Hierarchy — generated from WHATWG DOM + Web IDL.
//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py
extern crate alloc;
use alloc::{string::String, vec::Vec};


use crate::element_registry::ElementDef;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Element = 1, Text = 3, Comment = 8, Document = 9,
    DocumentType = 10, DocumentFragment = 11,
}

pub trait Node {
    fn node_type(&self) -> NodeType;
    fn node_name(&self) -> &str;
    fn parent_node(&self) -> Option<&dyn Node>;
    fn first_child(&self) -> Option<&dyn Node>;
    fn last_child(&self) -> Option<&dyn Node>;
    fn previous_sibling(&self) -> Option<&dyn Node>;
    fn next_sibling(&self) -> Option<&dyn Node>;
    fn child_nodes(&self) -> Vec<&dyn Node>;
}

pub trait Element: Node {
    fn tag_name(&self) -> &str;
    fn element_def(&self) -> &'static ElementDef;
    fn get_attribute(&self, name: &str) -> Option<&str>;
    fn set_attribute(&mut self, name: &str, value: &str);
    fn remove_attribute(&mut self, name: &str);
    fn has_attribute(&self, name: &str) -> bool;
    fn id(&self) -> &str;
    fn class_list(&self) -> &[String];
}

pub trait HTMLElement: Element {
    fn title(&self) -> &str;
    fn lang(&self) -> &str;
    fn hidden(&self) -> bool;
}

pub trait FormAssociated: HTMLElement {
    fn form(&self) -> Option<&dyn Element>;
    fn disabled(&self) -> bool;
}

pub trait Labelable: FormAssociated {}

pub trait MediaElement: HTMLElement {
    fn src(&self) -> &str;
    fn autoplay(&self) -> bool;
    fn controls(&self) -> bool;
    fn muted(&self) -> bool;
}
