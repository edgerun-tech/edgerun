#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

pub use edgerun_sdk_macro::main;

#[derive(Debug, Clone)]
pub struct Request {
    pub body: Vec<u8>,
    pub content_type: Option<String>,
}

impl Request {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            body: bytes.to_vec(),
            content_type: None,
        }
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn body_as_string(&self) -> Option<String> {
        alloc::string::String::from_utf8(self.body.clone()).ok()
    }
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
}

impl Response {
    pub fn ok(body: impl Into<String>) -> Self {
        let body = body.into();
        Self {
            status: 200,
            body: body.into_bytes(),
            content_type: Some("text/plain".into()),
        }
    }

    pub fn html(body: impl Into<String>) -> Self {
        let body = body.into();
        Self {
            status: 200,
            body: body.into_bytes(),
            content_type: Some("text/html".into()),
        }
    }

    pub fn json(body: impl Into<String>) -> Self {
        let body = body.into();
        Self {
            status: 200,
            body: body.into_bytes(),
            content_type: Some("application/json".into()),
        }
    }

    pub fn not_found(body: impl Into<String>) -> Self {
        let body = body.into();
        Self {
            status: 404,
            body: body.into_bytes(),
            content_type: Some("text/plain".into()),
        }
    }

    pub fn error(body: impl Into<String>) -> Self {
        let body = body.into();
        Self {
            status: 500,
            body: body.into_bytes(),
            content_type: Some("text/plain".into()),
        }
    }

    pub fn with_status(status: u16, body: impl Into<String>) -> Self {
        let body = body.into();
        Self {
            status,
            body: body.into_bytes(),
            content_type: Some("text/plain".into()),
        }
    }

    pub fn with_body(status: u16, body: Vec<u8>) -> Self {
        Self {
            status,
            body,
            content_type: None,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.status.to_le_bytes());
        if let Some(ref ct) = self.content_type {
            let ct_bytes = ct.as_bytes();
            let ct_len = ct_bytes.len() as u32;
            out.extend_from_slice(&ct_len.to_le_bytes());
            out.extend_from_slice(ct_bytes);
        } else {
            out.extend_from_slice(&0u32.to_le_bytes());
        }
        let body_len = self.body.len() as u32;
        out.extend_from_slice(&body_len.to_le_bytes());
        out.extend_from_slice(&self.body);
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 10 {
            return Self {
                status: 500,
                body: b"invalid response".to_vec(),
                content_type: None,
            };
        }

        let status = u16::from_le_bytes([bytes[0], bytes[1]]);
        let ct_len = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]) as usize;
        let content_type = if ct_len > 0 {
            let ct_start = 6;
            let ct_end = ct_start + ct_len;
            if ct_end <= bytes.len() {
                Some(String::from_utf8_lossy(&bytes[ct_start..ct_end]).into_owned())
            } else {
                None
            }
        } else {
            None
        };

        let body_offset = 6 + ct_len;
        if body_offset + 4 > bytes.len() {
            return Self {
                status: 500,
                body: b"invalid response".to_vec(),
                content_type: None,
            };
        }

        let body_len = u32::from_le_bytes([
            bytes[body_offset],
            bytes[body_offset + 1],
            bytes[body_offset + 2],
            bytes[body_offset + 3],
        ]) as usize;

        let body_start = body_offset + 4;
        let body_end = body_start + body_len;
        let body = if body_end <= bytes.len() {
            bytes[body_start..body_end].to_vec()
        } else {
            Vec::new()
        };

        Self {
            status,
            body,
            content_type,
        }
    }
}

pub mod host {
    use super::Event;
    #[link(wasm_import_module = "env")]
    extern "C" {
        pub fn write_output(ptr: i32, len: i32);

        pub fn send_message(
            target_ptr: i32,
            target_len: i32,
            payload_ptr: i32,
            payload_len: i32,
        ) -> i32;

        pub fn read_blob(hash_ptr: i32, hash_len: i32, dst_ptr: i32) -> i32;

        pub fn write_blob(ptr: i32, len: i32, hash_out_ptr: i32) -> i32;

        pub fn poll_event(buf_ptr: i32, buf_len: i32) -> i32;

        pub fn request_user_presence(
            reason_ptr: i32,
            reason_len: i32,
            session_ptr: i32,
            session_len: i32,
            ttl_seconds: u32,
            out_token_ptr: i32,
            out_token_len: i32,
        ) -> i32;

        pub fn request_signature(
            payload_ptr: i32,
            payload_len: i32,
            action_ptr: i32,
            action_len: i32,
            human_ptr: i32,
            human_len: i32,
            session_ptr: i32,
            session_len: i32,
            token_ptr: i32,
            token_len: i32,
            out_sig_ptr: i32,
            out_sig_len: i32,
        ) -> i32;
    }

    pub fn send_message_safe(target: &str, payload: &[u8]) -> i32 {
        let target_ptr = target.as_ptr() as i32;
        let target_len = target.len() as i32;
        let payload_ptr = payload.as_ptr() as i32;
        let payload_len = payload.len() as i32;
        unsafe { send_message(target_ptr, target_len, payload_ptr, payload_len) }
    }

    pub fn write_blob_safe(data: &[u8]) -> Option<[u8; 32]> {
        let mut hash_out = [0u8; 32];
        let ptr = data.as_ptr() as i32;
        let len = data.len() as i32;
        let hash_ptr = hash_out.as_mut_ptr() as i32;
        unsafe {
            let result = write_blob(ptr, len, hash_ptr);
            if result == 0 {
                Some(hash_out)
            } else {
                None
            }
        }
    }

    pub fn read_blob_safe(hash: &[u8], dst: &mut [u8]) -> i32 {
        let hash_ptr = hash.as_ptr() as i32;
        let hash_len = hash.len() as i32;
        let dst_ptr = dst.as_mut_ptr() as i32;
        unsafe { read_blob(hash_ptr, hash_len, dst_ptr) }
    }

    pub fn poll_event_safe(buf: &mut [u8]) -> Option<Event> {
        let ptr = buf.as_mut_ptr() as i32;
        let len = buf.len() as i32;
        let bytes_written = unsafe { poll_event(ptr, len) };
        if bytes_written < 0 {
            return None;
        }
        Event::from_bytes(&buf[..bytes_written as usize])
    }

    pub fn request_user_presence_safe(
        reason: &str,
        session_id: &[u8],
        ttl_seconds: u32,
    ) -> Option<[u8; 32]> {
        let mut out_token = [0u8; 32];
        let reason_ptr = reason.as_ptr() as i32;
        let reason_len = reason.len() as i32;
        let session_ptr = session_id.as_ptr() as i32;
        let session_len = session_id.len() as i32;
        let out_token_ptr = out_token.as_mut_ptr() as i32;
        unsafe {
            let result = request_user_presence(
                reason_ptr,
                reason_len,
                session_ptr,
                session_len,
                ttl_seconds,
                out_token_ptr,
                32,
            );
            if result == 0 {
                Some(out_token)
            } else {
                None
            }
        }
    }

    pub fn request_signature_safe(
        payload: &[u8],
        action: &str,
        human_readable: &str,
        session_id: &[u8],
        presence_token: &[u8],
    ) -> Option<[u8; 64]> {
        let mut out_sig = [0u8; 64];
        let payload_ptr = payload.as_ptr() as i32;
        let payload_len = payload.len() as i32;
        let action_ptr = action.as_ptr() as i32;
        let action_len = action.len() as i32;
        let human_ptr = human_readable.as_ptr() as i32;
        let human_len = human_readable.len() as i32;
        let session_ptr = session_id.as_ptr() as i32;
        let session_len = session_id.len() as i32;
        let token_ptr = presence_token.as_ptr() as i32;
        let token_len = presence_token.len() as i32;
        let out_sig_ptr = out_sig.as_mut_ptr() as i32;
        unsafe {
            let result = request_signature(
                payload_ptr,
                payload_len,
                action_ptr,
                action_len,
                human_ptr,
                human_len,
                session_ptr,
                session_len,
                token_ptr,
                token_len,
                out_sig_ptr,
                64,
            );
            if result == 0 {
                Some(out_sig)
            } else {
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    Network,
    Disk,
    Timer,
}

impl EventType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(EventType::Network),
            2 => Some(EventType::Disk),
            3 => Some(EventType::Timer),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            EventType::Network => 1,
            EventType::Disk => 2,
            EventType::Timer => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkSubtype {
    Connected,
    Disconnected,
    Received,
    Error,
}

impl NetworkSubtype {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(NetworkSubtype::Connected),
            2 => Some(NetworkSubtype::Disconnected),
            3 => Some(NetworkSubtype::Received),
            4 => Some(NetworkSubtype::Error),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            NetworkSubtype::Connected => 1,
            NetworkSubtype::Disconnected => 2,
            NetworkSubtype::Received => 3,
            NetworkSubtype::Error => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiskSubtype {
    ReadDone,
    WriteDone,
    Error,
}

impl DiskSubtype {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(DiskSubtype::ReadDone),
            2 => Some(DiskSubtype::WriteDone),
            3 => Some(DiskSubtype::Error),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            DiskSubtype::ReadDone => 1,
            DiskSubtype::WriteDone => 2,
            DiskSubtype::Error => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimerSubtype {
    Fired,
}

impl TimerSubtype {
    pub fn from_u8(_v: u8) -> Option<Self> {
        Some(TimerSubtype::Fired)
    }

    pub fn to_u8(&self) -> u8 {
        1
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub data: Vec<u8>,
}

impl Event {
    pub fn new(event_type: EventType, data: Vec<u8>) -> Self {
        Self { event_type, data }
    }

    pub fn network_connected(sock_id: u32) -> Self {
        Self::new(EventType::Network, {
            let mut d = Vec::new();
            d.extend_from_slice(&sock_id.to_le_bytes());
            d.push(NetworkSubtype::Connected.to_u8());
            d
        })
    }

    pub fn network_disconnected(sock_id: u32) -> Self {
        Self::new(EventType::Disk, {
            let mut d = Vec::new();
            d.extend_from_slice(&sock_id.to_le_bytes());
            d.push(NetworkSubtype::Disconnected.to_u8());
            d
        })
    }

    pub fn network_received(sock_id: u32, data: &[u8]) -> Self {
        Self::new(EventType::Network, {
            let mut d = Vec::new();
            d.extend_from_slice(&sock_id.to_le_bytes());
            d.push(NetworkSubtype::Received.to_u8());
            d.extend_from_slice(&(data.len() as u32).to_le_bytes());
            d.extend_from_slice(data);
            d
        })
    }

    pub fn disk_read_done(op_id: u32, hash: &[u8; 32], data: &[u8]) -> Self {
        Self::new(EventType::Disk, {
            let mut d = Vec::new();
            d.extend_from_slice(&op_id.to_le_bytes());
            d.push(DiskSubtype::ReadDone.to_u8());
            d.extend_from_slice(hash);
            d.extend_from_slice(data);
            d
        })
    }

    pub fn disk_write_done(op_id: u32, hash: &[u8; 32]) -> Self {
        Self::new(EventType::Disk, {
            let mut d = Vec::new();
            d.extend_from_slice(&op_id.to_le_bytes());
            d.push(DiskSubtype::WriteDone.to_u8());
            d.extend_from_slice(hash);
            d
        })
    }

    pub fn timer_fired(timer_id: u64) -> Self {
        let mut data = vec![TimerSubtype::Fired.to_u8()];
        data.extend_from_slice(&timer_id.to_le_bytes());
        Self::new(EventType::Timer, data)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(self.event_type.to_u8());
        out.extend_from_slice(&self.data);
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }
        let event_type = EventType::from_u8(bytes[0])?;
        let data = bytes[1..].to_vec();
        Some(Self { event_type, data })
    }

    pub fn network_sock_id(&self) -> Option<u32> {
        if self.event_type != EventType::Network || self.data.len() < 4 {
            return None;
        }
        Some(u32::from_le_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
        ]))
    }

    pub fn network_subtype(&self) -> Option<NetworkSubtype> {
        if self.event_type != EventType::Network || self.data.len() < 5 {
            return None;
        }
        NetworkSubtype::from_u8(self.data[4])
    }

    pub fn network_payload(&self) -> Option<&[u8]> {
        if self.event_type != EventType::Network
            || self.data.len() < 9
            || self.network_subtype()? != NetworkSubtype::Received
        {
            return None;
        }
        let payload_len = u32::from_le_bytes([
            self.data[5],
            self.data[6],
            self.data[7],
            self.data[8],
        ]) as usize;
        if self.data.len() < 9 + payload_len {
            return None;
        }
        Some(&self.data[9..9 + payload_len])
    }

    pub fn disk_op_id(&self) -> Option<u32> {
        if self.event_type != EventType::Disk || self.data.len() < 4 {
            return None;
        }
        Some(u32::from_le_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
        ]))
    }

    pub fn disk_subtype(&self) -> Option<DiskSubtype> {
        if self.event_type != EventType::Disk || self.data.len() < 5 {
            return None;
        }
        DiskSubtype::from_u8(self.data[4])
    }

    pub fn disk_hash(&self) -> Option<[u8; 32]> {
        if self.event_type != EventType::Disk
            || self.data.len() < 37
            || self.disk_subtype()? == DiskSubtype::Error
        {
            return None;
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&self.data[5..37]);
        Some(hash)
    }

    pub fn disk_data(&self) -> Option<&[u8]> {
        if self.event_type != EventType::Disk
            || self.data.len() < 37
            || self.disk_subtype()? != DiskSubtype::ReadDone
        {
            return None;
        }
        Some(&self.data[37..])
    }

    pub fn timer_id(&self) -> Option<u64> {
        if self.event_type != EventType::Timer || self.data.len() < 9 {
            return None;
        }
        Some(u64::from_le_bytes([
            self.data[1],
            self.data[2],
            self.data[3],
            self.data[4],
            self.data[5],
            self.data[6],
            self.data[7],
            self.data[8],
        ]))
    }
}

// ---------------------------------------------------------------------------
// User Authority Acquisition — PresenceToken and SignatureContext
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PresenceToken {
    pub token: [u8; 32],
    pub expires_at_micros: u64,
    pub session_id: Vec<u8>,
}

impl PresenceToken {
    pub fn new(token: [u8; 32], expires_at_micros: u64, session_id: Vec<u8>) -> Self {
        Self {
            token,
            expires_at_micros,
            session_id,
        }
    }

    pub fn is_expired(&self, now_micros: u64) -> bool {
        now_micros >= self.expires_at_micros
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.token);
        out.extend_from_slice(&self.expires_at_micros.to_le_bytes());
        let session_len = self.session_id.len() as u8;
        out.push(session_len);
        out.extend_from_slice(&self.session_id);
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 32 + 8 + 1 {
            return None;
        }
        let mut token = [0u8; 32];
        token.copy_from_slice(&bytes[0..32]);
        let expires_at_micros = u64::from_le_bytes([
            bytes[32], bytes[33], bytes[34], bytes[35],
            bytes[36], bytes[37], bytes[38], bytes[39],
        ]);
        let session_len = bytes[40] as usize;
        if bytes.len() < 41 + session_len {
            return None;
        }
        let session_id = bytes[41..41 + session_len].to_vec();
        Some(Self {
            token,
            expires_at_micros,
            session_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SignatureContext {
    pub app_id: [u8; 32],
    pub action: String,
    pub human_readable: String,
}

impl SignatureContext {
    pub fn new(app_id: [u8; 32], action: String, human_readable: String) -> Self {
        Self {
            app_id,
            action,
            human_readable,
        }
    }
}

// ---------------------------------------------------------------------------
// UI system — deterministic UI tree produced by WASM, rendered by adapters
// ---------------------------------------------------------------------------

pub use edgerun_proto::edgerun::v0::ui::{UiNode, UiActionEvent, UiRenderRequest};

pub mod ui {
    use super::*;
    use alloc::collections::BTreeMap;
    use alloc::string::ToString;

    pub fn text(value: &str) -> UiNode {
        let mut props = BTreeMap::new();
        props.insert(String::from("value"), String::from(value));
        UiNode {
            r#type: String::from("text"),
            props,
            children: Vec::new(),
            action: None,
        }
    }

    pub fn heading(value: &str, level: u32) -> UiNode {
        let mut props = BTreeMap::new();
        props.insert(String::from("value"), String::from(value));
        props.insert(String::from("level"), level.to_string());
        UiNode {
            r#type: String::from("heading"),
            props,
            children: Vec::new(),
            action: None,
        }
    }

    pub fn button(label: &str, action: &str) -> UiNode {
        let mut props = BTreeMap::new();
        props.insert(String::from("label"), String::from(label));
        UiNode {
            r#type: String::from("button"),
            props,
            children: Vec::new(),
            action: Some(String::from(action)),
        }
    }

    pub fn column(children: Vec<UiNode>) -> UiNode {
        UiNode {
            r#type: String::from("column"),
            props: BTreeMap::new(),
            children,
            action: None,
        }
    }

    pub fn row(children: Vec<UiNode>) -> UiNode {
        UiNode {
            r#type: String::from("row"),
            props: BTreeMap::new(),
            children,
            action: None,
        }
    }

    pub fn spacer(height: u32) -> UiNode {
        let mut props = BTreeMap::new();
        props.insert(String::from("height"), height.to_string());
        UiNode {
            r#type: String::from("spacer"),
            props,
            children: Vec::new(),
            action: None,
        }
    }

    pub fn input(placeholder: &str, action: &str) -> UiNode {
        let mut props = BTreeMap::new();
        props.insert(String::from("placeholder"), String::from(placeholder));
        UiNode {
            r#type: String::from("input"),
            props,
            children: Vec::new(),
            action: Some(String::from(action)),
        }
    }
}

/// Serialize a UiNode tree to bytes and wrap in a Response.
/// Content type is "application/x-edgerun-ui-v0+protobuf".
pub fn render(root: UiNode) -> Response {
    let mut bytes = Vec::new();
    prost::Message::encode(&root, &mut bytes).expect("UiNode encode failed");

    Response {
        status: 200,
        body: bytes,
        content_type: Some(String::from("application/x-edgerun-ui-v0+protobuf")),
    }
}

/// Render raw JSX/HTML from WASM. The host sanitizes and renders in a
/// sandboxed iframe with Tailwind CSS. No JS execution — event handlers
/// become data-action attributes routed back to WASM.
pub fn render_jsx(jsx: impl Into<String>) -> Response {
    let body = jsx.into();
    Response {
        status: 200,
        body: body.into_bytes(),
        content_type: Some(String::from("application/x-edgerun-jsx-v0+html")),
    }
}

/// Serialize a UiNode tree to bytes with a custom status code.
pub fn render_with_status(status: u16, root: UiNode) -> Response {
    let mut bytes = Vec::new();
    prost::Message::encode(&root, &mut bytes).expect("UiNode encode failed");

    Response {
        status,
        body: bytes,
        content_type: Some(String::from("application/x-edgerun-ui-v0+protobuf")),
    }
}

/// Deserialize UiNode from response bytes.
pub fn parse_ui(bytes: &[u8]) -> Option<UiNode> {
    use prost::Message;
    UiNode::decode(bytes).ok()
}
