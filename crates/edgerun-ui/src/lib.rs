//! EdgeRun UI renderers — native (egui) and browser (DOM).
//!
//! Renders `UINode` trees produced by WASM into platform-native widgets.
//! WASM never contains platform-specific UI logic.

use edgerun_proto::edgerun::v0::ui::UINode;
use prost::Message;
use std::collections::BTreeMap;

pub type ActionCallback = Box<dyn Fn(String) + Send + Sync>;

// ---------------------------------------------------------------------------
// Native renderer (egui)
// ---------------------------------------------------------------------------

pub struct NativeRenderer {
    pub action_fn: Option<ActionCallback>,
    current_ui: Option<UINode>,
}

impl NativeRenderer {
    pub fn new() -> Self {
        Self {
            action_fn: None,
            current_ui: None,
        }
    }

    pub fn with_action_callback<F>(mut self, f: F) -> Self
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        self.action_fn = Some(Box::new(f));
        self
    }

    pub fn set_ui(&mut self, bytes: &[u8]) -> bool {
        match UINode::decode(bytes) {
            Ok(node) => {
                self.current_ui = Some(node);
                true
            }
            Err(_) => false,
        }
    }

    pub fn render(&self, ctx: &egui::Context) {
        if let Some(ref root) = self.current_ui {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_node(ui, root, &self.action_fn);
            });
        }
    }

    pub fn current_ui(&self) -> Option<&UINode> {
        self.current_ui.as_ref()
    }
}

fn render_node(ui: &mut egui::Ui, node: &UINode, action_fn: &Option<ActionCallback>) {
    match node.node_type.as_str() {
        "text" => {
            if let Some(value) = node.props.get("value") {
                ui.label(value.as_str());
            }
        }
        "heading" => {
            if let Some(value) = node.props.get("value") {
                let level = node
                    .props
                    .get("level")
                    .and_then(|l| l.parse::<u32>().ok())
                    .unwrap_or(1);
                let heading = egui::RichText::new(value.as_str())
                    .size(match level {
                        1 => 24.0,
                        2 => 20.0,
                        3 => 16.0,
                        _ => 14.0,
                    })
                    .strong();
                ui.label(heading);
            }
        }
        "button" => {
            if let Some(label) = node.props.get("label") {
                if ui.button(label.as_str()).clicked() {
                    if let Some(ref action) = node.action {
                        if let Some(ref cb) = action_fn {
                            cb(action.clone());
                        }
                    }
                }
            }
        }
        "column" => {
            ui.vertical(|ui| {
                for child in &node.children {
                    render_node(ui, child, action_fn);
                }
            });
        }
        "row" => {
            ui.horizontal(|ui| {
                for child in &node.children {
                    render_node(ui, child, action_fn);
                }
            });
        }
        "spacer" => {
            let height = node
                .props
                .get("height")
                .and_then(|h| h.parse::<f32>().ok())
                .unwrap_or(8.0);
            ui.add_space(height);
        }
        "input" => {
            let placeholder = node.props.get("placeholder").map(|s| s.as_str());
            let mut text = String::new();
            let response = if let Some(p) = placeholder {
                egui::TextEdit::singleline(&mut text)
                    .hint_text(p)
                    .desired_width(f32::INFINITY)
                    .show(ui)
                    .response
            } else {
                egui::TextEdit::singleline(&mut text)
                    .desired_width(f32::INFINITY)
                    .show(ui)
                    .response
            };
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some(ref action) = node.action {
                    if let Some(ref cb) = action_fn {
                        cb(format!("input:{}", text));
                        cb(action.clone());
                    }
                }
            }
        }
        _ => {
            ui.label(format!("[unknown: {}]", node.node_type));
        }
    }

    for child in &node.children {
        render_node(ui, child, action_fn);
    }
}

// ---------------------------------------------------------------------------
// Serialize / deserialize helpers
// ---------------------------------------------------------------------------

pub fn encode_ui(node: &UINode) -> Vec<u8> {
    use prost::Message;
    let mut buf = Vec::new();
    node.encode(&mut buf).expect("UINode encode failed");
    buf
}

pub fn decode_ui(bytes: &[u8]) -> Option<UINode> {
    use prost::Message;
    UINode::decode(bytes).ok()
}

// ---------------------------------------------------------------------------
// Convenience builders (Rust-side UI construction for tests / default UI)
// ---------------------------------------------------------------------------

pub fn text(value: &str) -> UINode {
    let mut props = BTreeMap::new();
    props.insert(String::from("value"), String::from(value));
    UINode {
        node_type: String::from("text"),
        props,
        children: Vec::new(),
        action: None,
    }
}

pub fn button(label: &str, action: &str) -> UINode {
    let mut props = BTreeMap::new();
    props.insert(String::from("label"), String::from(label));
    UINode {
        node_type: String::from("button"),
        props,
        children: Vec::new(),
        action: Some(String::from(action)),
    }
}

pub fn column(children: Vec<UINode>) -> UINode {
    UINode {
        node_type: String::from("column"),
        props: BTreeMap::new(),
        children,
        action: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let node = column(vec![
            text("Hello"),
            button("Click me", "click"),
        ]);

        let bytes = encode_ui(&node);
        let decoded = decode_ui(&bytes).expect("decode should succeed");

        assert_eq!(decoded.node_type, "column");
        assert_eq!(decoded.children.len(), 2);
        assert_eq!(decoded.children[0].node_type, "text");
        assert_eq!(decoded.children[1].node_type, "button");
        assert_eq!(decoded.children[1].action, Some(String::from("click")));
    }

    #[test]
    fn test_decode_invalid_bytes() {
        let result = decode_ui(&[0xDE, 0xAD, 0xBE, 0xEF]);
        assert!(result.is_none());
    }

    #[test]
    fn test_text_node_props() {
        let node = text("world");
        assert_eq!(node.props.get("value"), Some(&String::from("world")));
        assert!(node.children.is_empty());
        assert!(node.action.is_none());
    }

    #[test]
    fn test_button_node_action() {
        let node = button("Go", "navigate:home");
        assert_eq!(node.props.get("label"), Some(&String::from("Go")));
        assert_eq!(node.action, Some(String::from("navigate:home")));
    }
}
