//! Fixed JSON-RPC 2.0 boundary records.
//!
//! This module intentionally does not expose a generic JSON object API. Unknown
//! JSON-RPC payloads are carried as validated raw JSON spans until the owning
//! protocol record projects them through the JSON WAT modules.

use codex_protocol::protocol::W3cTraceContext;
use schemars::JsonSchema;
use std::fmt;

pub const JSONRPC_VERSION: &str = "2.0";

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Hash, Eq, JsonSchema)]
#[schemars(untagged)]
pub enum RequestId {
    String(String),
    Integer(i64),
}

impl RequestId {
    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        self.write_json(&mut out);
        out
    }

    fn write_json(&self, out: &mut String) {
        match self {
            Self::String(value) => write_json_string(out, value),
            Self::Integer(value) => out.push_str(&value.to_string()),
        }
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(value) => f.write_str(value),
            Self::Integer(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(transparent)]
pub struct RawJson {
    bytes: String,
}

impl RawJson {
    pub fn new(bytes: impl Into<String>) -> std::result::Result<Self, JsonrpcParseError> {
        let bytes = bytes.into();
        let mut cursor = Cursor::new(&bytes);
        let end = cursor.skip_value(0)?;
        if cursor.skip_ws(end) != bytes.len() {
            return Err(JsonrpcParseError::new(
                "trailing bytes after raw JSON payload",
            ));
        }
        Ok(Self { bytes })
    }

    pub fn from_trusted_bytes(bytes: impl Into<String>) -> Self {
        Self {
            bytes: bytes.into(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.bytes
    }
}

impl fmt::Display for RawJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.bytes)
    }
}

pub type Result = RawJson;

/// Refers to any valid JSON-RPC object that can be decoded off the wire, or encoded to be sent.
#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[schemars(untagged)]
pub enum JSONRPCMessage {
    Request(JSONRPCRequest),
    Notification(JSONRPCNotification),
    Response(JSONRPCResponse),
    Error(JSONRPCError),
}

impl JSONRPCMessage {
    pub fn from_json_str(input: &str) -> std::result::Result<Self, JsonrpcParseError> {
        let fields = parse_jsonrpc_fields(input)?;
        validate_jsonrpc_version(fields.jsonrpc)?;
        match (fields.id, fields.method, fields.result, fields.error) {
            (Some(id), Some(method), _, None) => Ok(Self::Request(JSONRPCRequest {
                id: parse_request_id(id)?,
                method: parse_required_string(method, "method")?,
                params: fields.params.map(raw_span),
                trace: fields.trace.map(parse_trace).transpose()?,
            })),
            (None, Some(method), _, None) => Ok(Self::Notification(JSONRPCNotification {
                method: parse_required_string(method, "method")?,
                params: fields.params.map(raw_span),
            })),
            (Some(id), None, Some(result), None) => Ok(Self::Response(JSONRPCResponse {
                id: parse_request_id(id)?,
                result: raw_span(result),
            })),
            (Some(id), None, _, Some(error)) => Ok(Self::Error(JSONRPCError {
                error: parse_error_object(error)?,
                id: parse_request_id(id)?,
            })),
            _ => Err(JsonrpcParseError::new(
                "unrecognized JSON-RPC message shape",
            )),
        }
    }

    pub fn to_json_string(&self) -> String {
        match self {
            Self::Request(value) => value.to_json_string(),
            Self::Notification(value) => value.to_json_string(),
            Self::Response(value) => value.to_json_string(),
            Self::Error(value) => value.to_json_string(),
        }
    }
}

/// A request that expects a response.
#[derive(Debug, Clone, PartialEq, JsonSchema)]
pub struct JSONRPCRequest {
    pub id: RequestId,
    pub method: String,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<RawJson>,
    /// Optional W3C Trace Context for distributed tracing.
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<W3cTraceContext>,
}

impl JSONRPCRequest {
    pub fn to_json_string(&self) -> String {
        let mut out = jsonrpc_prefix();
        out.push_str(",\"id\":");
        self.id.write_json(&mut out);
        out.push_str(",\"method\":");
        write_json_string(&mut out, &self.method);
        if let Some(params) = &self.params {
            out.push_str(",\"params\":");
            out.push_str(params.as_str());
        }
        if let Some(trace) = &self.trace {
            write_trace_field(&mut out, trace);
        }
        out.push('}');
        out
    }
}

/// A notification which does not expect a response.
#[derive(Debug, Clone, PartialEq, JsonSchema)]
pub struct JSONRPCNotification {
    pub method: String,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<RawJson>,
}

impl JSONRPCNotification {
    pub fn to_json_string(&self) -> String {
        let mut out = jsonrpc_prefix();
        out.push_str(",\"method\":");
        write_json_string(&mut out, &self.method);
        if let Some(params) = &self.params {
            out.push_str(",\"params\":");
            out.push_str(params.as_str());
        }
        out.push('}');
        out
    }
}

/// A successful (non-error) response to a request.
#[derive(Debug, Clone, PartialEq, JsonSchema)]
pub struct JSONRPCResponse {
    pub id: RequestId,
    pub result: Result,
}

impl JSONRPCResponse {
    pub fn to_json_string(&self) -> String {
        let mut out = jsonrpc_prefix();
        out.push_str(",\"id\":");
        self.id.write_json(&mut out);
        out.push_str(",\"result\":");
        out.push_str(self.result.as_str());
        out.push('}');
        out
    }
}

/// A response to a request that indicates an error occurred.
#[derive(Debug, Clone, PartialEq, JsonSchema)]
pub struct JSONRPCError {
    pub error: JSONRPCErrorError,
    pub id: RequestId,
}

impl JSONRPCError {
    pub fn to_json_string(&self) -> String {
        let mut out = jsonrpc_prefix();
        out.push_str(",\"id\":");
        self.id.write_json(&mut out);
        out.push_str(",\"error\":");
        self.error.write_json(&mut out);
        out.push('}');
        out
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
pub struct JSONRPCErrorError {
    pub code: i64,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<RawJson>,
    pub message: String,
}

impl JSONRPCErrorError {
    fn write_json(&self, out: &mut String) {
        out.push_str("{\"code\":");
        out.push_str(&self.code.to_string());
        out.push_str(",\"message\":");
        write_json_string(out, &self.message);
        if let Some(data) = &self.data {
            out.push_str(",\"data\":");
            out.push_str(data.as_str());
        }
        out.push('}');
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonrpcParseError {
    message: String,
}

impl JsonrpcParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for JsonrpcParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for JsonrpcParseError {}

#[derive(Default)]
struct JsonrpcFields<'a> {
    jsonrpc: Option<&'a str>,
    id: Option<&'a str>,
    method: Option<&'a str>,
    params: Option<&'a str>,
    result: Option<&'a str>,
    error: Option<&'a str>,
    trace: Option<&'a str>,
}

#[derive(Default)]
struct ErrorFields<'a> {
    code: Option<&'a str>,
    message: Option<&'a str>,
    data: Option<&'a str>,
}

fn jsonrpc_prefix() -> String {
    "{\"jsonrpc\":\"2.0\"".to_string()
}

fn raw_span(span: &str) -> RawJson {
    RawJson::from_trusted_bytes(span.to_string())
}

fn parse_jsonrpc_fields(input: &str) -> std::result::Result<JsonrpcFields<'_>, JsonrpcParseError> {
    let mut fields = JsonrpcFields::default();
    parse_object_fields(input, |key, span| {
        match key.as_str() {
            "jsonrpc" => fields.jsonrpc = Some(span),
            "id" => fields.id = Some(span),
            "method" => fields.method = Some(span),
            "params" => fields.params = Some(span),
            "result" => fields.result = Some(span),
            "error" => fields.error = Some(span),
            "trace" => fields.trace = Some(span),
            _ => {}
        }
        Ok(())
    })?;
    Ok(fields)
}

fn parse_error_fields(input: &str) -> std::result::Result<ErrorFields<'_>, JsonrpcParseError> {
    let mut fields = ErrorFields::default();
    parse_object_fields(input, |key, span| {
        match key.as_str() {
            "code" => fields.code = Some(span),
            "message" => fields.message = Some(span),
            "data" => fields.data = Some(span),
            _ => {}
        }
        Ok(())
    })?;
    Ok(fields)
}

fn parse_object_fields<'a, F>(
    input: &'a str,
    mut visit: F,
) -> std::result::Result<(), JsonrpcParseError>
where
    F: FnMut(String, &'a str) -> std::result::Result<(), JsonrpcParseError>,
{
    let cursor = Cursor::new(input);
    let bytes = input.as_bytes();
    let mut pos = cursor.skip_ws(0);
    if bytes.get(pos) != Some(&b'{') {
        return Err(JsonrpcParseError::new("expected JSON object"));
    }
    pos += 1;
    loop {
        pos = cursor.skip_ws(pos);
        match bytes.get(pos) {
            Some(b'}') => {
                pos += 1;
                break;
            }
            Some(b'"') => {}
            _ => return Err(JsonrpcParseError::new("expected JSON object key")),
        }
        let (key, next) = cursor.parse_string(pos)?;
        pos = cursor.skip_ws(next);
        if bytes.get(pos) != Some(&b':') {
            return Err(JsonrpcParseError::new("expected ':' after JSON object key"));
        }
        pos += 1;
        let value_start = cursor.skip_ws(pos);
        let value_end = cursor.skip_value(value_start)?;
        visit(key, &input[value_start..value_end])?;
        pos = cursor.skip_ws(value_end);
        match bytes.get(pos) {
            Some(b',') => pos += 1,
            Some(b'}') => {
                pos += 1;
                break;
            }
            _ => {
                return Err(JsonrpcParseError::new(
                    "expected ',' or '}' after JSON value",
                ));
            }
        }
    }
    if cursor.skip_ws(pos) != input.len() {
        return Err(JsonrpcParseError::new("trailing bytes after JSON object"));
    }
    Ok(())
}

fn validate_jsonrpc_version(span: Option<&str>) -> std::result::Result<(), JsonrpcParseError> {
    match span {
        Some(value) if parse_required_string(value, "jsonrpc")? == JSONRPC_VERSION => Ok(()),
        Some(_) => Err(JsonrpcParseError::new("unsupported JSON-RPC version")),
        None => Ok(()),
    }
}

fn parse_request_id(span: &str) -> std::result::Result<RequestId, JsonrpcParseError> {
    let bytes = span.as_bytes();
    if bytes.first() == Some(&b'"') {
        return Ok(RequestId::String(parse_required_string(span, "id")?));
    }
    parse_i64(span).map(RequestId::Integer)
}

fn parse_error_object(span: &str) -> std::result::Result<JSONRPCErrorError, JsonrpcParseError> {
    let fields = parse_error_fields(span)?;
    Ok(JSONRPCErrorError {
        code: parse_i64(required(fields.code, "error.code")?)?,
        data: fields.data.map(raw_span),
        message: parse_required_string(
            required(fields.message, "error.message")?,
            "error.message",
        )?,
    })
}

fn parse_trace(span: &str) -> std::result::Result<W3cTraceContext, JsonrpcParseError> {
    let mut traceparent = None;
    let mut tracestate = None;
    parse_object_fields(span, |key, value| {
        match key.as_str() {
            "traceparent" => traceparent = Some(parse_required_string(value, "trace.traceparent")?),
            "tracestate" => tracestate = Some(parse_required_string(value, "trace.tracestate")?),
            _ => {}
        }
        Ok(())
    })?;
    Ok(W3cTraceContext {
        traceparent,
        tracestate,
    })
}

fn required<'a>(
    value: Option<&'a str>,
    field: &str,
) -> std::result::Result<&'a str, JsonrpcParseError> {
    value.ok_or_else(|| JsonrpcParseError::new(format!("missing required field {field}")))
}

fn parse_required_string(
    span: &str,
    field: &str,
) -> std::result::Result<String, JsonrpcParseError> {
    let cursor = Cursor::new(span);
    let pos = cursor.skip_ws(0);
    if span.as_bytes().get(pos) != Some(&b'"') {
        return Err(JsonrpcParseError::new(format!(
            "expected string field {field}"
        )));
    }
    let (value, end) = cursor.parse_string(pos)?;
    if cursor.skip_ws(end) != span.len() {
        return Err(JsonrpcParseError::new(format!(
            "trailing bytes in string field {field}"
        )));
    }
    Ok(value)
}

fn parse_i64(span: &str) -> std::result::Result<i64, JsonrpcParseError> {
    let cursor = Cursor::new(span);
    let start = cursor.skip_ws(0);
    let end = cursor.skip_number(start)?;
    if cursor.skip_ws(end) != span.len() {
        return Err(JsonrpcParseError::new("trailing bytes in integer field"));
    }
    span[start..end]
        .parse::<i64>()
        .map_err(|_| JsonrpcParseError::new("invalid integer field"))
}

fn write_trace_field(out: &mut String, trace: &W3cTraceContext) {
    if trace.traceparent.is_none() && trace.tracestate.is_none() {
        return;
    }
    out.push_str(",\"trace\":{");
    let mut first = true;
    if let Some(traceparent) = &trace.traceparent {
        out.push_str("\"traceparent\":");
        write_json_string(out, traceparent);
        first = false;
    }
    if let Some(tracestate) = &trace.tracestate {
        if !first {
            out.push(',');
        }
        out.push_str("\"tracestate\":");
        write_json_string(out, tracestate);
    }
    out.push('}');
}

fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch <= '\u{1f}' => {
                out.push_str("\\u");
                out.push_str(&format!("{:04x}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

struct Cursor<'a> {
    input: &'a str,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a str) -> Self {
        Self { input }
    }

    fn skip_ws(&self, mut pos: usize) -> usize {
        while matches!(
            self.input.as_bytes().get(pos),
            Some(b' ' | b'\n' | b'\r' | b'\t')
        ) {
            pos += 1;
        }
        pos
    }

    fn skip_value(&self, pos: usize) -> std::result::Result<usize, JsonrpcParseError> {
        match self.input.as_bytes().get(pos).copied() {
            Some(b'"') => self.parse_string(pos).map(|(_, end)| end),
            Some(b'{') => self.skip_object(pos),
            Some(b'[') => self.skip_array(pos),
            Some(b't') => self.expect_literal(pos, "true"),
            Some(b'f') => self.expect_literal(pos, "false"),
            Some(b'n') => self.expect_literal(pos, "null"),
            Some(b'-' | b'0'..=b'9') => self.skip_number(pos),
            _ => Err(JsonrpcParseError::new("expected JSON value")),
        }
    }

    fn skip_object(&self, mut pos: usize) -> std::result::Result<usize, JsonrpcParseError> {
        let bytes = self.input.as_bytes();
        pos += 1;
        loop {
            pos = self.skip_ws(pos);
            if bytes.get(pos) == Some(&b'}') {
                return Ok(pos + 1);
            }
            let (_, next) = self.parse_string(pos)?;
            pos = self.skip_ws(next);
            if bytes.get(pos) != Some(&b':') {
                return Err(JsonrpcParseError::new("expected ':' in JSON object"));
            }
            pos = self.skip_value(self.skip_ws(pos + 1))?;
            pos = self.skip_ws(pos);
            match bytes.get(pos) {
                Some(b',') => pos += 1,
                Some(b'}') => return Ok(pos + 1),
                _ => return Err(JsonrpcParseError::new("expected ',' or '}' in JSON object")),
            }
        }
    }

    fn skip_array(&self, mut pos: usize) -> std::result::Result<usize, JsonrpcParseError> {
        let bytes = self.input.as_bytes();
        pos += 1;
        loop {
            pos = self.skip_ws(pos);
            if bytes.get(pos) == Some(&b']') {
                return Ok(pos + 1);
            }
            pos = self.skip_value(pos)?;
            pos = self.skip_ws(pos);
            match bytes.get(pos) {
                Some(b',') => pos += 1,
                Some(b']') => return Ok(pos + 1),
                _ => return Err(JsonrpcParseError::new("expected ',' or ']' in JSON array")),
            }
        }
    }

    fn parse_string(&self, pos: usize) -> std::result::Result<(String, usize), JsonrpcParseError> {
        let mut out = String::new();
        let bytes = self.input.as_bytes();
        if bytes.get(pos) != Some(&b'"') {
            return Err(JsonrpcParseError::new("expected JSON string"));
        }
        let mut cursor = pos + 1;
        while let Some(byte) = bytes.get(cursor).copied() {
            match byte {
                b'"' => return Ok((out, cursor + 1)),
                b'\\' => {
                    cursor += 1;
                    match bytes.get(cursor).copied() {
                        Some(b'"') => out.push('"'),
                        Some(b'\\') => out.push('\\'),
                        Some(b'/') => out.push('/'),
                        Some(b'b') => out.push('\u{08}'),
                        Some(b'f') => out.push('\u{0c}'),
                        Some(b'n') => out.push('\n'),
                        Some(b'r') => out.push('\r'),
                        Some(b't') => out.push('\t'),
                        Some(b'u') => {
                            let end = cursor + 5;
                            let hex = self.input.get(cursor + 1..end).ok_or_else(|| {
                                JsonrpcParseError::new("short JSON unicode escape")
                            })?;
                            let code = u16::from_str_radix(hex, 16).map_err(|_| {
                                JsonrpcParseError::new("invalid JSON unicode escape")
                            })?;
                            let ch = char::from_u32(code as u32).ok_or_else(|| {
                                JsonrpcParseError::new("invalid JSON unicode scalar")
                            })?;
                            out.push(ch);
                            cursor = end - 1;
                        }
                        _ => return Err(JsonrpcParseError::new("invalid JSON escape")),
                    }
                }
                0x00..=0x1f => return Err(JsonrpcParseError::new("control byte in JSON string")),
                _ => {
                    let rest = &self.input[cursor..];
                    let ch = rest
                        .chars()
                        .next()
                        .ok_or_else(|| JsonrpcParseError::new("invalid UTF-8 string"))?;
                    out.push(ch);
                    cursor += ch.len_utf8() - 1;
                }
            }
            cursor += 1;
        }
        Err(JsonrpcParseError::new("unterminated JSON string"))
    }

    fn expect_literal(
        &self,
        pos: usize,
        literal: &str,
    ) -> std::result::Result<usize, JsonrpcParseError> {
        if self.input[pos..].starts_with(literal) {
            Ok(pos + literal.len())
        } else {
            Err(JsonrpcParseError::new("invalid JSON literal"))
        }
    }

    fn skip_number(&self, mut pos: usize) -> std::result::Result<usize, JsonrpcParseError> {
        let bytes = self.input.as_bytes();
        if bytes.get(pos) == Some(&b'-') {
            pos += 1;
        }
        match bytes.get(pos) {
            Some(b'0') => pos += 1,
            Some(b'1'..=b'9') => {
                pos += 1;
                while matches!(bytes.get(pos), Some(b'0'..=b'9')) {
                    pos += 1;
                }
            }
            _ => return Err(JsonrpcParseError::new("invalid JSON number")),
        }
        if bytes.get(pos) == Some(&b'.') {
            return Err(JsonrpcParseError::new("integer field cannot be fractional"));
        }
        if matches!(bytes.get(pos), Some(b'e' | b'E')) {
            return Err(JsonrpcParseError::new(
                "integer field cannot use exponent notation",
            ));
        }
        Ok(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_request_with_raw_params_and_trace() {
        let msg = JSONRPCMessage::from_json_str(
            r#"{"jsonrpc":"2.0","id":7,"method":"thread/start","params":{"x":[1,true]},"trace":{"traceparent":"00-a-b-c"}}"#,
        )
        .expect("request");
        let JSONRPCMessage::Request(request) = msg else {
            panic!("expected request");
        };
        assert_eq!(request.id, RequestId::Integer(7));
        assert_eq!(request.method, "thread/start");
        assert_eq!(request.params.unwrap().as_str(), r#"{"x":[1,true]}"#);
        assert_eq!(
            request.trace.unwrap().traceparent.as_deref(),
            Some("00-a-b-c")
        );
    }

    #[test]
    fn emits_error_without_generic_json_model() {
        let err = JSONRPCError {
            id: RequestId::String("abc".to_string()),
            error: JSONRPCErrorError {
                code: -32602,
                message: "bad params".to_string(),
                data: Some(RawJson::from_trusted_bytes(r#"{"field":"id"}"#)),
            },
        };
        assert_eq!(
            err.to_json_string(),
            r#"{"jsonrpc":"2.0","id":"abc","error":{"code":-32602,"message":"bad params","data":{"field":"id"}}}"#
        );
    }
}
