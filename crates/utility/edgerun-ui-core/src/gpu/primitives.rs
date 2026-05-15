use std::string::String;

use super::Color4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UiRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn inset(self, dx: f32, dy: f32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            w: (self.w - dx * 2.0).max(0.0),
            h: (self.h - dy * 2.0).max(0.0),
        }
    }

    pub fn inset_ltrb(self, left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            x: self.x + left,
            y: self.y + top,
            w: (self.w - left - right).max(0.0),
            h: (self.h - top - bottom).max(0.0),
        }
    }

    pub fn with_height_centered(self, h: f32) -> Self {
        let h = h.min(self.h).max(0.0);
        Self {
            x: self.x,
            y: self.y + (self.h - h) * 0.5,
            w: self.w,
            h,
        }
    }

    pub fn with_width_centered(self, w: f32) -> Self {
        let w = w.min(self.w).max(0.0);
        Self {
            x: self.x + (self.w - w) * 0.5,
            y: self.y,
            w,
            h: self.h,
        }
    }

    pub fn right(self, w: f32) -> Self {
        Self {
            x: self.x + self.w - w,
            y: self.y,
            w,
            h: self.h,
        }
    }

    pub fn bottom(self, h: f32) -> Self {
        Self {
            x: self.x,
            y: self.y + self.h - h,
            w: self.w,
            h,
        }
    }

    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.w && y <= self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Clone, Debug)]
pub enum UiControlAccessory {
    None,
    Value(String),
    Badge {
        label: String,
        color: Color4,
    },
    Toggle {
        on: bool,
        id: u32,
    },
    Button {
        label: String,
        id: u32,
        style: ButtonStyle,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnifiedContactKind {
    Person,
    CodexClient,
    Node,
}

#[derive(Clone, Copy, Debug)]
pub struct UnifiedContact<'a> {
    pub name: &'a str,
    pub detail: &'a str,
    pub kind: UnifiedContactKind,
    pub unread: u16,
    pub online: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct UnifiedMessage<'a> {
    pub author: &'a str,
    pub body: &'a str,
    pub outgoing: bool,
    pub accent: Color4,
}

#[derive(Clone, Debug)]
pub struct UnifiedChatState<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub contacts: &'a [UnifiedContact<'a>],
    pub selected_contact: usize,
    pub messages: &'a [UnifiedMessage<'a>],
    pub composer_placeholder: &'a str,
    pub connected: bool,
}

impl<'a> UnifiedChatState<'a> {
    pub const fn empty() -> Self {
        Self {
            title: "EdgeRun Chat",
            subtitle: "contact book",
            contacts: &[],
            selected_contact: 0,
            messages: &[],
            composer_placeholder: "Select a contact to start a thread...",
            connected: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UiWorkProjection {
    pub local_node: String,
    pub admission_node: String,
    pub relay_node: String,
    pub channel: String,
    pub policy_hash: String,
    pub request_hash: String,
    pub admission_hash: String,
    pub route_commitment: String,
    pub storage_payload_hash: String,
    pub manifest_hash: String,
    pub admitted_budget: u64,
    pub retrieval_cost: u64,
    pub request_verified: bool,
    pub admission_verified: bool,
    pub storage_payload_verified: bool,
}

impl UiWorkProjection {
    pub fn preview() -> Self {
        Self {
            local_node: "node instance pending".into(),
            admission_node: "admission pending".into(),
            relay_node: "relay pending".into(),
            channel: "channel pending".into(),
            policy_hash: "policy hash pending".into(),
            request_hash: "request hash pending".into(),
            admission_hash: "admission hash pending".into(),
            route_commitment: "route pending".into(),
            storage_payload_hash: "payload hash pending".into(),
            manifest_hash: "manifest pending".into(),
            admitted_budget: 0,
            retrieval_cost: 0,
            request_verified: false,
            admission_verified: false,
            storage_payload_verified: false,
        }
    }

    pub fn parse_key_values(input: &str) -> Option<Self> {
        let mut projection = Self::preview();
        let mut saw_field = false;

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            let value = value.trim();
            saw_field = true;

            match key {
                "local_node" => projection.local_node = value.into(),
                "admission_node" => projection.admission_node = value.into(),
                "relay_node" => projection.relay_node = value.into(),
                "channel" => projection.channel = value.into(),
                "policy_hash" => projection.policy_hash = value.into(),
                "request_hash" => projection.request_hash = value.into(),
                "admission_hash" => projection.admission_hash = value.into(),
                "route_commitment" => projection.route_commitment = value.into(),
                "storage_payload_hash" => projection.storage_payload_hash = value.into(),
                "manifest_hash" => projection.manifest_hash = value.into(),
                "admitted_budget" => projection.admitted_budget = value.parse().ok()?,
                "retrieval_cost" => projection.retrieval_cost = value.parse().ok()?,
                "request_verified" => projection.request_verified = parse_bool_field(value)?,
                "admission_verified" => projection.admission_verified = parse_bool_field(value)?,
                "storage_payload_verified" => {
                    projection.storage_payload_verified = parse_bool_field(value)?;
                }
                _ => return None,
            }
        }

        saw_field.then_some(projection)
    }
}

fn parse_bool_field(value: &str) -> Option<bool> {
    match value {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}
