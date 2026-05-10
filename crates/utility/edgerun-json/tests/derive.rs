use edgerun_json::FromJson;
use edgerun_json::JsonValue;
use edgerun_json::ToJson;

#[derive(Debug, PartialEq, ToJson, FromJson)]
#[json(rename_all = "snake_case")]
struct ToolRequest {
    request_id: u64,
    #[json(rename = "toolName")]
    tool_name: String,
    #[json(default)]
    retries: u32,
    #[json(default = "default_priority")]
    priority: u32,
    #[json(alias = "legacy_timeout_ms")]
    timeout_ms: Option<u64>,
    #[json(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    #[json(default, skip_serializing, skip_deserializing)]
    internal_only: bool,
}

#[derive(Debug, PartialEq, ToJson, FromJson)]
#[json(rename_all = "snake_case")]
enum Mode {
    FastPath,
    #[json(rename = "safe")]
    SafeMode,
}

#[derive(Debug, PartialEq, ToJson, FromJson)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TaggedEvent {
    Created,
    MessageDelta {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing)]
        internal_id: Option<String>,
    },
}

fn default_priority() -> u32 {
    5
}

#[test]
fn derives_struct_json_roundtrip() {
    let request = ToolRequest {
        request_id: 7,
        tool_name: "shell".to_string(),
        retries: 0,
        priority: 5,
        timeout_ms: Some(300),
        note: None,
        internal_only: false,
    };

    let json = request.to_json();
    assert_eq!(json["request_id"].as_u64(), Some(7));
    assert_eq!(json["toolName"].as_str(), Some("shell"));
    assert_eq!(json["priority"].as_u64(), Some(5));
    assert_eq!(json["timeout_ms"].as_u64(), Some(300));
    assert!(json.get("note").is_none());
    assert!(json.get("internal_only").is_none());

    let parsed = ToolRequest::from_json(edgerun_json::json!({
        "request_id": 7,
        "toolName": "shell",
        "legacy_timeout_ms": 300
    }))
    .unwrap();
    assert_eq!(parsed, request);
}

#[test]
fn derives_string_enum_json() {
    assert_eq!(Mode::FastPath.to_json(), JsonValue::from("fast_path"));
    assert_eq!(Mode::SafeMode.to_json(), JsonValue::from("safe"));
    assert_eq!(
        Mode::from_json(JsonValue::from("safe")).unwrap(),
        Mode::SafeMode
    );
}

#[test]
fn derives_tagged_enum_json() {
    let event = TaggedEvent::MessageDelta {
        id: "msg_1".to_string(),
        text: Some("hello".to_string()),
        internal_id: Some("local".to_string()),
    };

    let json = event.to_json();
    assert_eq!(json["type"].as_str(), Some("message_delta"));
    assert_eq!(json["id"].as_str(), Some("msg_1"));
    assert_eq!(json["text"].as_str(), Some("hello"));
    assert!(json.get("internal_id").is_none());

    assert_eq!(
        TaggedEvent::from_json(edgerun_json::json!({
            "type": "message_delta",
            "id": "msg_1"
        }))
        .unwrap(),
        TaggedEvent::MessageDelta {
            id: "msg_1".to_string(),
            text: None,
            internal_id: None,
        }
    );
    assert_eq!(
        TaggedEvent::from_json(edgerun_json::json!({"type": "created"})).unwrap(),
        TaggedEvent::Created
    );
}
