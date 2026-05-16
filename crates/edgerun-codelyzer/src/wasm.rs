use std::format;
use std::string::String;
use std::sync::Mutex;
use std::vec::Vec;

use crate::generated::codeanalyzer::{
    GraphData, GraphEdge, GraphNode, GraphUpdate, WsMessage, WsMessageEnum, WsResponseResult,
};
use crate::source_analysis::graph_data_from_snapshot;
use crate::xray_wire::{ConnectionsEnvelope, LocalConnection, SourceSnapshot};

static LAST_ERROR: Mutex<Vec<u8>> = Mutex::new(Vec::new());

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_xray_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_xray_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        drop(unsafe { Vec::from_raw_parts(ptr, len, len) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_xray_decode(ptr: *const u8, len: usize) -> u64 {
    if ptr.is_null() {
        set_last_error("null rkyv input");
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    match decode_wire(bytes) {
        Ok(json) => leak_bytes(json.into_bytes()),
        Err(error) => {
            set_last_error(&error);
            0
        }
    }
}

/// Analyze a rkyv SourceSnapshot and return a rkyv WsMessage::GraphData.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn edgerun_xray_analyze_sources(ptr: *const u8, len: usize) -> u64 {
    if ptr.is_null() {
        set_last_error("null source snapshot input");
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    match analyze_source_snapshot(bytes) {
        Ok(graph_bytes) => leak_bytes(graph_bytes),
        Err(error) => {
            set_last_error(&error);
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_xray_last_error() -> u64 {
    let bytes = LAST_ERROR
        .lock()
        .map(|error| error.clone())
        .unwrap_or_default();
    leak_bytes(bytes)
}

fn analyze_source_snapshot(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let snapshot = SourceSnapshot::decode_rkyv(bytes).map_err(|error| error.to_string())?;
    let data = graph_data_from_snapshot(&snapshot);
    let message = WsMessage {
        msg: Some(WsMessageEnum::GraphData(data)),
    };
    message.encode_rkyv().map_err(|error| error.to_string())
}

fn decode_wire(bytes: &[u8]) -> Result<String, String> {
    let message = match WsMessage::decode_rkyv(bytes) {
        Ok(message) => message,
        Err(message_error) => {
            return ConnectionsEnvelope::decode_rkyv(bytes)
                .map(|connections| connections_json(&connections))
                .map_err(|connections_error| {
                    format!("unsupported rkyv payload: WsMessage={message_error}; ConnectionsEnvelope={connections_error}")
                });
        }
    };
    let Some(msg) = message.msg else {
        return Err(String::from("rkyv WsMessage has no payload"));
    };

    match msg {
        WsMessageEnum::GraphData(data) => Ok(graph_data_json(&data)),
        WsMessageEnum::GraphUpdate(update) => Ok(graph_update_json(&update)),
        WsMessageEnum::Response(response) => match response.result {
            WsResponseResult::GraphData(data) => Ok(graph_data_json(&data)),
            WsResponseResult::Error(error) => Err(error),
            other => Err(format!("unsupported rkyv response payload: {other:?}")),
        },
        other => Err(format!("unsupported rkyv message payload: {other:?}")),
    }
}

fn connections_json(data: &ConnectionsEnvelope) -> String {
    let mut out = String::new();
    out.push_str("{\"type\":\"connections\",\"data\":{");
    out.push_str("\"connection_count\":");
    out.push_str(&data.connection_count.to_string());
    out.push_str(",\"connections\":");
    write_connections(&mut out, &data.connections);
    out.push_str("}}");
    out
}

fn leak_bytes(bytes: Vec<u8>) -> u64 {
    if bytes.is_empty() {
        return 0;
    }
    let len = bytes.len();
    let ptr = Box::into_raw(bytes.into_boxed_slice()) as *mut u8 as usize;
    ((ptr as u64) << 32) | (len as u64)
}

fn set_last_error(message: &str) {
    if let Ok(mut error) = LAST_ERROR.lock() {
        error.clear();
        error.extend_from_slice(message.as_bytes());
    }
}

fn graph_data_json(data: &GraphData) -> String {
    let mut out = String::new();
    out.push_str("{\"type\":\"graph_data\",\"data\":{");
    out.push_str("\"nodes\":");
    write_ui_nodes(&mut out, &data.nodes);
    out.push_str(",\"edges\":");
    write_ui_edges(&mut out, &data.edges);
    out.push_str(",\"tag_groups\":[],\"total_bytes\":");
    out.push_str(&data.total_bytes.to_string());
    out.push_str(",\"node_count\":");
    out.push_str(&data.node_count.to_string());
    out.push_str(",\"edge_count\":");
    out.push_str(&data.edge_count.to_string());
    out.push_str("}}");
    out
}

fn graph_update_json(update: &GraphUpdate) -> String {
    let mut out = String::new();
    out.push_str("{\"type\":\"graph_update\",\"data\":{");
    out.push_str("\"is_full\":");
    out.push_str(if update.is_full { "true" } else { "false" });
    out.push_str(",\"timestamp\":");
    out.push_str(&update.timestamp.to_string());
    out.push_str(",\"nodes\":");
    if let Some(nodes) = &update.nodes {
        write_ui_nodes(&mut out, &nodes.added);
    } else {
        out.push_str("[]");
    }
    out.push_str(",\"edges\":");
    if let Some(edges) = &update.edges {
        write_ui_edges(&mut out, &edges.added);
    } else {
        out.push_str("[]");
    }
    out.push_str("}}");
    out
}

fn write_ui_nodes(out: &mut String, nodes: &[GraphNode]) {
    out.push('[');
    for (index, node) in nodes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        write_json_field(out, "id", &node.id, false);
        write_json_field(out, "kind", "function", true);
        write_json_field(
            out,
            "label",
            if node.name.is_empty() {
                &node.id
            } else {
                &node.name
            },
            true,
        );
        out.push_str(",\"tags\":");
        let mut tags = node.tags.clone();
        if node.is_static && !tags.iter().any(|tag| tag == "static") {
            tags.push(String::from("static"));
        }
        if let Some(commit) = &node.commit {
            tags.push(format!("commit:{commit}"));
        }
        tags.push(format!("connections:{}", node.connections));
        write_string_array(out, &tags);
        if let Some(layer) = infer_layer(&node.file) {
            write_json_field(out, "layer", layer, true);
        }
        write_json_field(out, "language", map_language(&node.language), true);
        out.push_str(",\"source\":{");
        write_json_field(out, "file", &node.file, false);
        write_json_field(out, "symbol", &node.name, true);
        out.push('}');
        out.push_str(",\"x\":0,\"y\":0");
        out.push('}');
    }
    out.push(']');
}

fn write_ui_edges(out: &mut String, edges: &[GraphEdge]) {
    out.push('[');
    for (index, edge) in edges.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        write_json_field(
            out,
            "id",
            &format!(
                "{}->{}:{}:{}",
                edge.source,
                edge.target,
                map_edge_kind(&edge.kind),
                index
            ),
            false,
        );
        write_json_field(out, "source", &edge.source, true);
        write_json_field(out, "target", &edge.target, true);
        write_json_field(out, "kind", map_edge_kind(&edge.kind), true);
        out.push_str(",\"tags\":[]");
        out.push('}');
    }
    out.push(']');
}

fn write_connections(out: &mut String, connections: &[LocalConnection]) {
    out.push('[');
    for (index, connection) in connections.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        write_json_field(out, "protocol", &connection.network_transport, false);
        write_json_field(out, "local_address", &connection.local_address, true);
        out.push_str(",\"local_port\":");
        out.push_str(&connection.local_port.to_string());
        write_json_field(out, "remote_address", &connection.remote_address, true);
        out.push_str(",\"remote_port\":");
        out.push_str(&connection.remote_port.to_string());
        write_json_field(out, "state", &connection.state, true);
        out.push('}');
    }
    out.push(']');
}

fn map_language(language: &str) -> &str {
    match language {
        "rust" | "typescript" | "javascript" | "python" | "go" | "java" | "c" => language,
        _ => "unknown",
    }
}

fn map_edge_kind(kind: &str) -> &str {
    match kind.to_ascii_lowercase().as_str() {
        "imports" => "imports",
        "owns" => "owns",
        "implements" => "implements",
        "sends" => "sends",
        "receives" => "receives",
        "stores" => "stores",
        "signs" => "signs",
        "verifies" => "verifies",
        "tests" => "tests",
        "observed_flow" => "observed_flow",
        _ => "calls",
    }
}

fn infer_layer(file: &str) -> Option<&'static str> {
    if file.contains("/ui/") || file.contains("components") {
        Some("ui")
    } else if file.contains("/api/") || file.contains("routes") {
        Some("api")
    } else if file.contains("/runtime/") || file.contains("rt-") {
        Some("runtime")
    } else if file.contains("/proto/") || file.contains("protocol") {
        Some("protocol")
    } else if file.contains("/storage/") || file.contains("store") {
        Some("storage")
    } else if file.contains("/network/") || file.contains("net-") {
        Some("network")
    } else if file.contains("/crypto/") || file.contains("crypto") {
        Some("crypto")
    } else if file.contains("/agent/") || file.contains("agent") {
        Some("agent")
    } else {
        None
    }
}

fn write_json_field(out: &mut String, key: &str, value: &str, prefixed: bool) {
    if prefixed {
        out.push(',');
    }
    out.push('"');
    out.push_str(key);
    out.push_str("\":\"");
    write_json_string(out, value);
    out.push('"');
}

fn write_string_array(out: &mut String, values: &[String]) {
    out.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('"');
        write_json_string(out, value);
        out.push('"');
    }
    out.push(']');
}

fn write_json_string(out: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
}
