//! Spec parser — reads official Wayland protocol XML files to extract
//! the full interface/protocol surface: interfaces, requests, events, enums.
//!
//! This is used to drive conformance tests: for every interface/request/event
//! defined in the spec, the test suite verifies our compositor responds correctly.

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
    pub arg_type: String,    // "int", "uint", "fixed", "string", "object", "new_id", "fd", "array"
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

pub fn parse_spec(path: impl AsRef<Path>) -> Result<Protocol, String> {
    let xml = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let tree = xml_tree::parse(&xml).map_err(|e| format!("XML parse error: {}", e))?;

    let root = tree.root.ok_or("No root element")?;
    if root.name != "protocol" {
        return Err(format!("Expected <protocol>, found <{}>", root.name));
    }

    let name = root.attr("name").unwrap_or("unknown").to_string();
    let copyright = root.children.iter()
        .find(|c| c.name == "copyright")
        .and_then(|c| c.text());

    let description = root.children.iter()
        .find(|c| c.name == "description")
        .and_then(|c| c.text());

    let mut interfaces = HashMap::new();
    for child in &root.children {
        if child.name == "interface" {
            let iface = parse_interface(child)?;
            interfaces.insert(iface.name.clone(), iface);
        }
    }

    Ok(Protocol {
        name,
        copyright: copyright.map(String::from),
        description: description.map(String::from),
        interfaces,
    })
}

fn parse_interface(node: &xml_tree::Node) -> Result<Interface, String> {
    let name = node.attr("name").unwrap_or("unknown").to_string();
    let version: u32 = node.attr("version").unwrap_or("1").parse().unwrap_or(1);

    let description = node.children.iter()
        .find(|c| c.name == "description")
        .and_then(|c| c.text());

    let mut requests = Vec::new();
    let mut events = Vec::new();
    let mut enums = Vec::new();

    for child in &node.children {
        match child.name.as_str() {
            "request" => requests.push(parse_request(child)?),
            "event" => events.push(parse_event(child)?),
            "enum" => enums.push(parse_enum(child)?),
            _ => {}
        }
    }

    Ok(Interface {
        name,
        version,
        description: description.map(String::from),
        requests,
        events,
        enums,
    })
}

fn parse_request(node: &xml_tree::Node) -> Result<Request, String> {
    Ok(Request {
        name: node.attr("name").unwrap_or("").to_string(),
        since: node.attr("since").unwrap_or("1").parse().unwrap_or(1),
        args: node.children.iter()
            .filter(|c| c.name == "arg")
            .filter_map(|c| parse_arg(c).ok())
            .collect(),
        description: node.children.iter()
            .find(|c| c.name == "description")
            .and_then(|c| c.text()).map(String::from),
    })
}

fn parse_event(node: &xml_tree::Node) -> Result<Event, String> {
    Ok(Event {
        name: node.attr("name").unwrap_or("").to_string(),
        since: node.attr("since").unwrap_or("1").parse().unwrap_or(1),
        args: node.children.iter()
            .filter(|c| c.name == "arg")
            .filter_map(|c| parse_arg(c).ok())
            .collect(),
        description: node.children.iter()
            .find(|c| c.name == "description")
            .and_then(|c| c.text()).map(String::from),
    })
}

fn parse_arg(node: &xml_tree::Node) -> Result<Arg, String> {
    let arg_type = node.attr("type").unwrap_or("").to_string();
    let interface = node.attr("interface").map(String::from);
    let summary = node.children.iter()
        .find(|c| c.name == "description")
        .and_then(|c| c.text()).map(String::from);
    let allow_null = node.attr("allow-null") == Some("true");

    Ok(Arg {
        name: node.attr("name").unwrap_or("").to_string(),
        arg_type,
        interface,
        summary,
        allow_null,
    })
}

fn parse_enum(node: &xml_tree::Node) -> Result<Enum, String> {
    Ok(Enum {
        name: node.attr("name").unwrap_or("").to_string(),
        since: node.attr("since").unwrap_or("1").parse().unwrap_or(1),
        entries: node.children.iter()
            .filter(|c| c.name == "entry")
            .filter_map(|c| parse_enum_entry(c).ok())
            .collect(),
    })
}

fn parse_enum_entry(node: &xml_tree::Node) -> Result<EnumEntry, String> {
    Ok(EnumEntry {
        name: node.attr("name").unwrap_or("").to_string(),
        value: node.attr("value").unwrap_or("0").parse().unwrap_or(0),
        summary: node.children.iter()
            .find(|c| c.name == "description")
            .and_then(|c| c.text()).map(String::from),
        since: node.attr("since").unwrap_or("1").parse().unwrap_or(1),
    })
}

// ─── Minimal XML parser (no external deps) ──────────────────

mod xml_tree {
    #[derive(Debug, Clone)]
    pub struct Node {
        pub name: String,
        pub attrs: Vec<(String, String)>,
        pub children: Vec<Node>,
        pub text: Option<String>,
    }

    impl Node {
        pub fn attr(&self, key: &str) -> Option<&str> {
            self.attrs.iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        }

        pub fn text(&self) -> Option<&str> {
            self.text.as_deref()
        }
    }

    pub fn parse(input: &str) -> Result<Node, String> {
        let mut parser = Parser { input, pos: 0 };
        parser.skip_ws();
        // Skip XML declaration if present
        if parser.input[parser.pos..].starts_with("<?xml") {
            parser.skip_to("?>").ok_or("Missing ?>")?;
            parser.pos += 2;
            parser.skip_ws();
        }
        parser.parse_node()
    }

    struct Parser<'a> {
        input: &'a str,
        pos: usize,
    }

    impl<'a> Parser<'a> {
        fn peek(&self) -> Option<char> {
            self.input[self.pos..].chars().next()
        }

        fn skip_ws(&mut self) {
            while self.pos < self.input.len() {
                let c = self.input[self.pos..].chars().next().unwrap();
                if c.is_whitespace() {
                    self.pos += c.len_utf8();
                } else {
                    break;
                }
            }
        }

        fn skip_to(&mut self, target: &str) -> Option<()> {
            self.input[self.pos..].find(target).map(|i| {
                self.pos += i;
            })
        }

        fn read_until(&mut self, end: char) -> String {
            let start = self.pos;
            while self.pos < self.input.len() && self.input[self.pos..].chars().next() != Some(end) {
                self.pos += self.input[self.pos..].chars().next().unwrap().len_utf8();
            }
            self.input[start..self.pos].to_string()
        }

        fn parse_node(&mut self) -> Result<Node, String> {
            self.skip_ws();
            if self.peek() != Some('<') {
                // Text node
                let text = self.read_until('<').trim().to_string();
                if text.is_empty() {
                    return Err("Expected element".to_string());
                }
                return Ok(Node {
                    name: String::new(),
                    attrs: Vec::new(),
                    children: Vec::new(),
                    text: Some(text),
                });
            }
            self.pos += 1; // skip '<'

            // Check for comment
            if self.input[self.pos..].starts_with("!--") {
                self.skip_to("-->").ok_or("Unclosed comment")?;
                self.pos += 3;
                return self.parse_node();
            }

            // Check for processing instruction
            if self.input[self.pos..].starts_with('?') {
                self.skip_to("?>").ok_or("Unclosed PI")?;
                self.pos += 2;
                return self.parse_node();
            }

            let name = self.read_until(|c: char| c.is_whitespace() || c == '/' || c == '>');
            self.pos += name.chars().next().map(|c| c.len_utf8()).unwrap_or(0);

            let mut attrs = Vec::new();
            loop {
                self.skip_ws();
                match self.peek() {
                    Some('/') | Some('>') => break,
                    Some(_) => {
                        let attr_name = self.read_until('=');
                        self.pos += 1; // skip '='
                        self.skip_ws();
                        let quote = self.peek().unwrap();
                        self.pos += 1;
                        let attr_val = self.read_until(quote);
                        self.pos += 1;
                        attrs.push((attr_name.trim().to_string(), attr_val.trim().to_string()));
                    }
                    None => break,
                }
            }

            let self_closing = self.peek() == Some('/');
            if self_closing {
                self.pos += 1;
                if self.peek() == Some('>') {
                    self.pos += 1;
                }
                return Ok(Node { name, attrs, children: Vec::new(), text: None });
            }

            if self.peek() == Some('>') {
                self.pos += 1;
            }

            let mut children = Vec::new();
            let mut text_accum = String::new();

            loop {
                if self.pos >= self.input.len() {
                    break;
                }
                if self.input[self.pos..].starts_with("</") {
                    // Parse text content
                    if !text_accum.trim().is_empty() {
                        if children.is_empty() {
                            return Ok(Node { name, attrs, children, text: Some(text_accum.trim().to_string()) });
                        }
                    }
                    // Skip closing tag
                    self.pos += 2;
                    let _close_name = self.read_until('>');
                    self.pos += 1;
                    break;
                }

                if self.input[self.pos..].starts_with("<!--") {
                    if !text_accum.trim().is_empty() {
                        // push accumulated text
                    }
                    self.pos += 4;
                    self.skip_to("-->").ok_or("Unclosed comment")?;
                    self.pos += 3;
                    continue;
                }

                if self.peek() == Some('<') {
                    if !text_accum.trim().is_empty() {
                        // We could create text nodes but for our use case
                        // we only care about element children
                    }
                    let child = self.parse_node()?;
                    children.push(child);
                } else {
                    let c = self.input[self.pos..].chars().next().unwrap();
                    text_accum.push(c);
                    self.pos += c.len_utf8();
                }
            }

            // If we have text but no element children, return text node
            if children.is_empty() && !text_accum.trim().is_empty() {
                return Ok(Node { name, attrs, children, text: Some(text_accum.trim().to_string()) });
            }

            // For mixed content, extract text from description-like elements
            let text = if !text_accum.trim().is_empty() {
                Some(text_accum.trim().to_string())
            } else {
                None
            };

            Ok(Node { name, attrs, children, text })
        }
    }
}

// ─── Convenience: load all specs from the docs directory ────

pub fn load_all_specs(specs_dir: impl AsRef<Path>) -> Vec<(String, Protocol)> {
    let mut specs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(specs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                for sub in std::fs::read_dir(&path).into_iter().flatten().flatten() {
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
        assert!(proto.interfaces.contains_key("wl_shm"));
        assert!(proto.interfaces.contains_key("wl_seat"));
        assert!(proto.interfaces.contains_key("wl_output"));

        let display = proto.interfaces.get("wl_display").unwrap();
        assert_eq!(display.version, 1);
        assert!(display.requests.iter().any(|r| r.name == "sync"));
        assert!(display.requests.iter().any(|r| r.name == "get_registry"));
    }

    #[test]
    fn test_parse_xdg_shell() {
        let proto = parse_spec("../../docs/protocol-specs/xdg/xdg-shell.xml").unwrap();
        assert!(proto.interfaces.contains_key("xdg_wm_base"));
        assert!(proto.interfaces.contains_key("xdg_surface"));
        assert!(proto.interfaces.contains_key("xdg_toplevel"));
        assert!(proto.interfaces.contains_key("xdg_popup"));

        let toplevel = proto.interfaces.get("xdg_toplevel").unwrap();
        assert!(toplevel.requests.iter().any(|r| r.name == "set_title"));
        assert!(toplevel.requests.iter().any(|r| r.name == "set_app_id"));
        assert!(toplevel.requests.iter().any(|r| r.name == "set_maximized"));
        assert!(toplevel.requests.iter().any(|r| r.name == "set_minimized"));
    }

    #[test]
    fn test_parse_viewporter() {
        let proto = parse_spec("../../docs/protocol-specs/wp/viewporter.xml").unwrap();
        assert!(proto.interfaces.contains_key("wp_viewporter"));
        assert!(proto.interfaces.contains_key("wp_viewport"));
    }

    #[test]
    fn test_parse_cursor_shape() {
        let proto = parse_spec("../../docs/protocol-specs/wp/cursor-shape-v1.xml").unwrap();
        assert!(proto.interfaces.contains_key("wp_cursor_shape_manager_v1"));
        assert!(proto.interfaces.contains_key("wp_cursor_shape_device_v1"));
    }
}
