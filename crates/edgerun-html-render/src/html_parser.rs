//! HTML Parser — generated from html_elements.proto + html_attributes.proto.
//! DO NOT EDIT. Regenerate with: scripts/generate_html_parser.py
extern crate alloc;
use alloc::format;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

pub const VOID_ELEMENTS: &[&str] = &["area","base","br","col","embed","hr","img","input","link","meta","source","track","wbr"];
pub const BLOCK_ELEMENTS: &[&str] = &["address","article","aside","blockquote","caption","col","colgroup","dd","details","dialog","div","dl","dt","figcaption","figure","fieldset","footer","form","h1","h2","h3","h4","h5","h6","header","hr","legend","li","main","nav","ol","p","pre","section","summary","table","tbody","td","tfoot","th","thead","tr","ul"];
pub const INLINE_ELEMENTS: &[&str] = &["a","abbr","audio","b","big","br","button","canvas","cite","code","data","datalist","del","dfn","em","iframe","img","input","ins","kbd","label","link","map","mark","meter","object","output","picture","progress","q","ruby","samp","script","select","slot","small","source","span","strong","sub","sup","svg","textarea","time","track","u","var","video","wbr"];
pub const RAW_TEXT_ELEMENTS: &[&str] = &["script","style"];

#[derive(Debug, Clone)]
pub enum Node {
    Element(Element),
    Text(String),
    Comment(String),
}

#[derive(Debug, Clone)]
pub struct Element {
    pub tag: String,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<Node>,
}

impl Element {
    pub fn new(tag: &str) -> Self {
        Self { tag: tag.to_lowercase(), attrs: BTreeMap::new(), children: Vec::new() }
    }
    pub fn is_void(&self) -> bool { VOID_ELEMENTS.contains(&self.tag.as_str()) }
    pub fn attr(&self, name: &str) -> Option<&str> { self.attrs.get(name).map(|s| s.as_str()) }
    pub fn class(&self) -> Option<&str> { self.attr("class") }
    pub fn id(&self) -> Option<&str> { self.attr("id") }
}

pub fn parse_html(html: &str) -> Node {
    let mut parser = Parser { input: html, pos: 0 };
    let nodes = parser.parse_nodes();
    if nodes.len() == 1 {
        nodes.into_iter().next().unwrap()
    } else {
        let mut root = Element::new("div");
        root.attrs.insert("data-root".into(), "true".into());
        root.children = nodes;
        Node::Element(root)
    }
}

struct Parser<'a> { input: &'a str, pos: usize }

impl<'a> Parser<'a> {
    fn parse_nodes(&mut self) -> Vec<Node> {
        let mut nodes = Vec::new();
        while self.pos < self.input.len() {
            if self.input[self.pos..].starts_with("</") { break; }
            if self.input[self.pos..].starts_with("<!--") {
                if let Some(c) = self.parse_comment() { nodes.push(Node::Comment(c)); }
            } else if self.input[self.pos..].starts_with('<') {
                if let Some(e) = self.parse_element() { nodes.push(Node::Element(e)); }
            } else {
                if let Some(t) = self.parse_text() {
                    let t = t.trim();
                    if !t.is_empty() { nodes.push(Node::Text(t.into())); }
                }
            }
        }
        nodes
    }

    fn parse_comment(&mut self) -> Option<String> {
        self.pos += 4;
        let s = self.pos;
        if let Some(end) = self.input[self.pos..].find("-->") {
            let c = self.input[s..self.pos+end].into();
            self.pos += end + 3;
            Some(c)
        } else { None }
    }

    fn parse_element(&mut self) -> Option<Element> {
        if self.pos >= self.input.len() || self.input.as_bytes()[self.pos] != b'<' { return None; }
        self.pos += 1;
        let tag = self.read_while(|c| c.is_alphanumeric() || c == '-' || c == '_');
        if tag.is_empty() { return None; }
        let tag = tag.to_lowercase();
        let mut attrs = BTreeMap::new();
        self.skip_ws();
        while self.pos < self.input.len() {
            let b = self.input.as_bytes()[self.pos];
            if b == b'>' || b == b'/' { break; }
            if b.is_ascii_whitespace() { self.skip_ws(); continue; }
            let name = self.read_while(|c| c != '=' && !c.is_whitespace() && c != '>' && c != '/');
            if name.is_empty() { self.pos += 1; continue; }
            self.skip_ws();
            let val = if self.pos < self.input.len() && self.input.as_bytes()[self.pos] == b'=' {
                self.pos += 1; self.skip_ws(); self.read_quoted()
            } else { String::new() };
            attrs.insert(name.to_lowercase(), val);
            self.skip_ws();
        }
        let self_close = self.pos < self.input.len() && self.input.as_bytes()[self.pos] == b'/';
        if self_close { self.pos += 1; }
        if self.pos < self.input.len() && self.input.as_bytes()[self.pos] == b'>' { self.pos += 1; }
        let mut elem = Element { tag: tag.clone(), attrs, children: Vec::new() };
        if VOID_ELEMENTS.contains(&tag.as_str()) || self_close { return Some(elem); }
        if RAW_TEXT_ELEMENTS.contains(&tag.as_str()) {
            let close = format!("</{}>", tag);
            if let Some(end) = self.input[self.pos..].find(&close) {
                elem.children.push(Node::Text(self.input[self.pos..self.pos+end].into()));
                self.pos += end + close.len();
            }
            return Some(elem);
        }
        elem.children = self.parse_nodes();
        let close_tag = format!("</{}", tag);
        if self.input[self.pos..].starts_with(&close_tag) {
            self.pos += close_tag.len();
            if self.pos < self.input.len() && self.input.as_bytes()[self.pos] == b'>' { self.pos += 1; }
        }
        Some(elem)
    }

    fn parse_text(&mut self) -> Option<String> {
        let s = self.pos;
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != b'<' { self.pos += 1; }
        if self.pos > s { Some(self.input[s..self.pos].into()) } else { None }
    }

    fn read_while<F: Fn(char) -> bool>(&mut self, f: F) -> String {
        let s = self.pos;
        while self.pos < self.input.len() {
            if let Some(c) = self.input[self.pos..].chars().next() {
                if !f(c) { break; }
                self.pos += c.len_utf8();
            } else { break; }
        }
        self.input[s..self.pos].into()
    }

    fn read_quoted(&mut self) -> String {
        if self.pos >= self.input.len() { return String::new(); }
        let q = self.input.as_bytes()[self.pos];
        if q == b'"' || q == b'\'' {
            self.pos += 1; let s = self.pos;
            while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != q { self.pos += 1; }
            let v = self.input[s..self.pos].into();
            if self.pos < self.input.len() { self.pos += 1; }
            v
        } else {
            self.read_while(|c| !c.is_whitespace() && c != '>' && c != '/')
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos].is_ascii_whitespace() { self.pos += 1; }
    }
}
