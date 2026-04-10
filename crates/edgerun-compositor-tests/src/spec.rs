//! Spec parser — reads official Wayland protocol XML files to extract
//! the full interface/protocol surface: interfaces, requests, events, enums.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Protocol {
    pub name: String,
    pub copyright: Option<String>,
    pub description: Option<String>,
    pub interfaces: HashMap<String, Interface>,
}

#[derive(Debug, Clone)]
pub struct Interface {
    pub name: String,
    pub version: u32,
    pub description: Option<String>,
    pub requests: Vec<Request>,
    pub events: Vec<Event>,
    pub enums: Vec<Enum>,
}

#[derive(Debug, Clone)]
pub struct Request {
    pub name: String,
    pub since: u32,
    pub args: Vec<Arg>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub name: String,
    pub since: u32,
    pub args: Vec<Arg>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Arg {
    pub name: String,
    pub arg_type: String,
    pub interface: Option<String>,
    pub summary: Option<String>,
    pub allow_null: bool,
}

#[derive(Debug, Clone)]
pub struct Enum {
    pub name: String,
    pub since: u32,
    pub entries: Vec<EnumEntry>,
}

#[derive(Debug, Clone)]
pub struct EnumEntry {
    pub name: String,
    pub value: i32,
    pub summary: Option<String>,
    pub since: u32,
}

/// Minimal XML tree node.
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    pub text_content: Option<String>,
}

impl Node {
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn text(&self) -> Option<&str> {
        self.text_content.as_deref()
    }

    fn find_child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    fn find_children(&self, name: &str) -> Vec<&Node> {
        self.children.iter().filter(|c| c.name == name).collect()
    }
}

pub fn parse_spec(path: impl AsRef<Path>) -> Result<Protocol, String> {
    let xml = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let root = parse_xml(&xml)?;

    if root.name != "protocol" {
        return Err(format!("Expected <protocol>, found <{}>", root.name));
    }

    let name = root.attr("name").unwrap_or("unknown").to_string();
    let copyright = root.find_child("copyright").and_then(|c| c.text());
    let description = root.find_child("description").and_then(|c| c.text());

    let mut interfaces = HashMap::new();
    for child in root.find_children("interface") {
        let iface = parse_interface(child)?;
        interfaces.insert(iface.name.clone(), iface);
    }

    Ok(Protocol { name, copyright: copyright.map(String::from), description: description.map(String::from), interfaces })
}

fn parse_interface(node: &Node) -> Result<Interface, String> {
    let name = node.attr("name").unwrap_or("unknown").to_string();
    let version: u32 = node.attr("version").unwrap_or("1").parse().unwrap_or(1);
    let description = node.find_child("description").and_then(|c| c.text());

    let requests = node.find_children("request").iter().filter_map(|c| parse_request(c).ok()).collect();
    let events = node.find_children("event").iter().filter_map(|c| parse_event(c).ok()).collect();
    let enums = node.find_children("enum").iter().filter_map(|c| parse_enum(c).ok()).collect();

    Ok(Interface { name, version, description: description.map(String::from), requests, events, enums })
}

fn parse_request(node: &Node) -> Result<Request, String> {
    Ok(Request {
        name: node.attr("name").unwrap_or("").to_string(),
        since: node.attr("since").unwrap_or("1").parse().unwrap_or(1),
        args: node.find_children("arg").iter().filter_map(|c| parse_arg(c).ok()).collect(),
        description: node.find_child("description").and_then(|c| c.text()).map(String::from),
    })
}

fn parse_event(node: &Node) -> Result<Event, String> {
    Ok(Event {
        name: node.attr("name").unwrap_or("").to_string(),
        since: node.attr("since").unwrap_or("1").parse().unwrap_or(1),
        args: node.find_children("arg").iter().filter_map(|c| parse_arg(c).ok()).collect(),
        description: node.find_child("description").and_then(|c| c.text()).map(String::from),
    })
}

fn parse_arg(node: &Node) -> Result<Arg, String> {
    Ok(Arg {
        name: node.attr("name").unwrap_or("").to_string(),
        arg_type: node.attr("type").unwrap_or("").to_string(),
        interface: node.attr("interface").map(String::from),
        summary: node.find_child("description").and_then(|c| c.text()).map(String::from),
        allow_null: node.attr("allow-null") == Some("true"),
    })
}

fn parse_enum(node: &Node) -> Result<Enum, String> {
    Ok(Enum {
        name: node.attr("name").unwrap_or("").to_string(),
        since: node.attr("since").unwrap_or("1").parse().unwrap_or(1),
        entries: node.find_children("entry").iter().filter_map(|c| parse_enum_entry(c).ok()).collect(),
    })
}

fn parse_enum_entry(node: &Node) -> Result<EnumEntry, String> {
    Ok(EnumEntry {
        name: node.attr("name").unwrap_or("").to_string(),
        value: node.attr("value").unwrap_or("0").parse().unwrap_or(0),
        summary: node.find_child("description").and_then(|c| c.text()).map(String::from),
        since: node.attr("since").unwrap_or("1").parse().unwrap_or(1),
    })
}

// ─── Minimal XML parser (no external deps) ──────────────────

fn parse_xml(input: &str) -> Result<Node, String> {
    let mut p = Parser { input, pos: 0 };
    p.skip_ws();
    // Skip XML declaration
    if p.rest().starts_with("<?xml") {
        p.skip_to("?>").ok_or("Missing ?>")?;
        p.pos += 2;
        p.skip_ws();
    }
    p.parse_node()
}

struct Parser<'a> { input: &'a str, pos: usize }

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str { &self.input[self.pos..] }
    fn peek(&self) -> Option<char> { self.rest().chars().next() }
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() { self.pos += c.len_utf8(); } else { break; }
        }
    }
    fn skip_to(&mut self, target: &str) -> Option<()> {
        self.rest().find(target).map(|i| { self.pos += i; })
    }
    fn read_until_char(&mut self, end: char) -> String {
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos..].chars().next() != Some(end) {
            self.pos += self.input[self.pos..].chars().next().unwrap().len_utf8();
        }
        self.input[start..self.pos].to_string()
    }

    fn parse_node(&mut self) -> Result<Node, String> {
        self.skip_ws();
        // Skip comments
        while self.rest().starts_with("<!--") {
            self.pos += 4;
            self.skip_to("-->").ok_or("Unclosed comment")?;
            self.pos += 3;
            self.skip_ws();
        }
        // Skip processing instructions
        while self.rest().starts_with("<?") {
            self.skip_to("?>").ok_or("Unclosed PI")?;
            self.pos += 2;
            self.skip_ws();
        }

        if self.peek() != Some('<') {
            let text = self.read_until_char('<').trim().to_string();
            if text.is_empty() {
                return self.parse_node();
            }
            return Ok(Node { name: String::new(), attrs: Vec::new(), children: Vec::new(), text_content: Some(text) });
        }
        self.pos += 1;

        let name = {
            let start = self.pos;
            while self.pos < self.input.len() {
                let c = self.input[self.pos..].chars().next().unwrap();
                if c.is_whitespace() || c == '/' || c == '>' { break; }
                self.pos += c.len_utf8();
            }
            self.input[start..self.pos].to_string()
        };

        let mut attrs = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some('/') | Some('>') => break,
                Some(_) => {
                    let attr_name = self.read_until_char('=');
                    self.pos += 1;
                    self.skip_ws();
                    let quote = self.peek().unwrap();
                    self.pos += 1;
                    let attr_val = self.read_until_char(quote);
                    self.pos += 1;
                    attrs.push((attr_name.trim().to_string(), attr_val.trim().to_string()));
                }
                None => break,
            }
        }

        let self_closing = self.peek() == Some('/');
        if self_closing {
            self.pos += 1;
            if self.peek() == Some('>') { self.pos += 1; }
            return Ok(Node { name, attrs, children: Vec::new(), text_content: None });
        }
        if self.peek() == Some('>') { self.pos += 1; }

        let mut children = Vec::new();
        let mut text_buf = String::new();

        loop {
            if self.pos >= self.input.len() { break; }
            if self.rest().starts_with("</") {
                self.pos += 2;
                let _ = self.read_until_char('>');
                self.pos += 1;
                break;
            }
            if self.rest().starts_with("<!--") {
                self.pos += 4;
                self.skip_to("-->").ok_or("Unclosed comment")?;
                self.pos += 3;
                continue;
            }
            if self.peek() == Some('<') {
                children.push(self.parse_node()?);
            } else {
                let c = self.input[self.pos..].chars().next().unwrap();
                text_buf.push(c);
                self.pos += c.len_utf8();
            }
        }

        let text = if children.is_empty() && !text_buf.trim().is_empty() {
            Some(text_buf.trim().to_string())
        } else {
            None
        };

        Ok(Node { name, attrs, children, text_content: text })
    }
}

// ─── Load all specs ──────────────────────────────────────────

pub fn load_all_specs(specs_dir: impl AsRef<Path>) -> Vec<(String, Protocol)> {
    let mut specs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(specs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(subs) = std::fs::read_dir(&path) {
                    for sub in subs.flatten() {
                        let sub_path = sub.path();
                        if sub_path.extension().and_then(|e| e.to_str()) == Some("xml") {
                            if let Ok(protocol) = parse_spec(&sub_path) {
                                specs.push((sub_path.file_stem().unwrap().to_string_lossy().to_string(), protocol));
                            }
                        }
                    }
                }
            }
        }
    }
    specs.sort_by(|a, b| a.0.cmp(&b.0));
    specs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wl_core() {
        let proto = parse_spec("../../docs/protocol-specs/core/wl_core.xml").unwrap();
        assert_eq!(proto.name, "wayland");
        assert!(proto.interfaces.contains_key("wl_display"));
        assert!(proto.interfaces.contains_key("wl_registry"));
        assert!(proto.interfaces.contains_key("wl_compositor"));
        let display = proto.interfaces.get("wl_display").unwrap();
        assert_eq!(display.version, 1);
        assert!(display.requests.iter().any(|r| r.name == "sync"));
    }

    #[test]
    fn test_parse_xdg_shell() {
        let proto = parse_spec("../../docs/protocol-specs/xdg/xdg-shell.xml").unwrap();
        assert!(proto.interfaces.contains_key("xdg_wm_base"));
        assert!(proto.interfaces.contains_key("xdg_toplevel"));
        let toplevel = proto.interfaces.get("xdg_toplevel").unwrap();
        assert!(toplevel.requests.iter().any(|r| r.name == "set_title"));
    }
}
