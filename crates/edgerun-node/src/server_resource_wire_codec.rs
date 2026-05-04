//! Wire-only codec for server-resource command payloads and committed events.
//!
//! This replaces the previous prost-derived payload structs in the live command
//! path. The model types still live in `server_resources`; this module is the
//! single serialization implementation.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::server_resources::{command_type, ContentRef, ServerResourceEvent};
use edgerun_core::protocol::CommandRef;
use edgerun_wire::{
    field, struct_value, text, u64v, WireDecode, WireEncode, WireReader, WireValue,
};

pub fn decode_command_payload(
    command_type_value: i32,
    payload: &[u8],
) -> Result<ServerResourceEvent, String> {
    let fields = decode_top_struct(payload)?;
    match command_type_value {
        command_type::CLAIM_DOMAIN => Ok(ServerResourceEvent::ClaimDomain {
            domain: required_text(&fields, 2, "domain")?,
        }),
        command_type::RELEASE_DOMAIN => Ok(ServerResourceEvent::ReleaseDomain {
            domain: required_text(&fields, 2, "domain")?,
        }),
        command_type::ADD_MAILBOX => Ok(ServerResourceEvent::AddMailbox {
            address: required_text(&fields, 2, "address")?,
        }),
        command_type::REMOVE_MAILBOX => Ok(ServerResourceEvent::RemoveMailbox {
            address: required_text(&fields, 2, "address")?,
        }),
        command_type::ADD_ALIAS => Ok(ServerResourceEvent::AddAlias {
            address: required_text(&fields, 2, "address")?,
            target: required_text(&fields, 3, "target")?,
        }),
        command_type::REMOVE_ALIAS => Ok(ServerResourceEvent::RemoveAlias {
            address: required_text(&fields, 2, "address")?,
        }),
        command_type::AUTHORIZE_CONTENT_SOURCE => Ok(ServerResourceEvent::AuthorizeContentSource {
            repo: required_text(&fields, 2, "repo")?,
            allowed_ref: required_text(&fields, 3, "allowed_ref")?,
            allowed_paths: required_text_list(&fields, 4, "allowed_paths")?,
        }),
        command_type::PUBLISH_WEBSITE => Ok(ServerResourceEvent::PublishWebsite {
            domain: required_text(&fields, 2, "domain")?,
            content_ref: required_content_ref(&fields, 3)?,
        }),
        command_type::UNPUBLISH_WEBSITE => Ok(ServerResourceEvent::UnpublishWebsite {
            domain: required_text(&fields, 2, "domain")?,
        }),
        command_type::SET_AUTHORITATIVE_DNS => Ok(ServerResourceEvent::SetAuthoritativeDns {
            domain: required_text(&fields, 2, "domain")?,
            enabled: required_bool(&fields, 3, "enabled")?,
        }),
        command_type::REQUEST_CERTIFICATE => Ok(ServerResourceEvent::RequestCertificate {
            name: required_text(&fields, 2, "name")?,
        }),
        command_type::SET_SERVICE_POLICY => Ok(ServerResourceEvent::SetServicePolicy {
            service: required_text(&fields, 2, "service")?,
            policy: required_text(&fields, 3, "policy")?,
        }),
        _ => Err("not_server_resource_command".into()),
    }
}

#[must_use]
pub fn encode_command_payload(event: &ServerResourceEvent) -> Vec<u8> {
    let value = match event {
        ServerResourceEvent::ClaimDomain { domain } => {
            struct_value(vec![field(1, u64v(1)), field(2, text(domain))])
        }
        ServerResourceEvent::ReleaseDomain { domain } => {
            struct_value(vec![field(1, u64v(1)), field(2, text(domain))])
        }
        ServerResourceEvent::AddMailbox { address } => {
            struct_value(vec![field(1, u64v(1)), field(2, text(address))])
        }
        ServerResourceEvent::RemoveMailbox { address } => {
            struct_value(vec![field(1, u64v(1)), field(2, text(address))])
        }
        ServerResourceEvent::AddAlias { address, target } => struct_value(vec![
            field(1, u64v(1)),
            field(2, text(address)),
            field(3, text(target)),
        ]),
        ServerResourceEvent::RemoveAlias { address } => {
            struct_value(vec![field(1, u64v(1)), field(2, text(address))])
        }
        ServerResourceEvent::AuthorizeContentSource {
            repo,
            allowed_ref,
            allowed_paths,
        } => struct_value(vec![
            field(1, u64v(1)),
            field(2, text(repo)),
            field(3, text(allowed_ref)),
            field(4, text_list(allowed_paths)),
        ]),
        ServerResourceEvent::PublishWebsite {
            domain,
            content_ref,
        } => struct_value(vec![
            field(1, u64v(1)),
            field(2, text(domain)),
            field(3, embedded_content_ref(content_ref)),
        ]),
        ServerResourceEvent::UnpublishWebsite { domain } => {
            struct_value(vec![field(1, u64v(1)), field(2, text(domain))])
        }
        ServerResourceEvent::SetAuthoritativeDns { domain, enabled } => struct_value(vec![
            field(1, u64v(1)),
            field(2, text(domain)),
            field(3, WireValue::Bool(*enabled)),
        ]),
        ServerResourceEvent::RequestCertificate { name } => {
            struct_value(vec![field(1, u64v(1)), field(2, text(name))])
        }
        ServerResourceEvent::SetServicePolicy { service, policy } => struct_value(vec![
            field(1, u64v(1)),
            field(2, text(service)),
            field(3, text(policy)),
        ]),
    };
    edgerun_wire::canonical_bytes(&value)
}

#[must_use]
pub fn encode_committed_resource_event(
    event: &ServerResourceEvent,
    origin_command: Option<CommandRef>,
) -> Vec<u8> {
    let mut fields = vec![
        field(1, u64v(1)),
        field(2, u64v(event.kind() as u64)),
        field(3, WireValue::Bytes(encode_command_payload(event))),
    ];
    if let Some(command_ref) = origin_command {
        fields.push(field(4, embedded_command_ref(&command_ref)));
    }
    edgerun_wire::canonical_bytes(&struct_value(fields))
}

pub fn decode_committed_resource_event(payload: &[u8]) -> Result<ServerResourceEvent, String> {
    let fields = decode_top_struct(payload)?;
    let event_kind = required_u32(&fields, 2, "event_kind")? as i32;
    let event_payload = required_bytes(&fields, 3, "event_payload")?;
    let command_type_value = match event_kind {
        1 => command_type::CLAIM_DOMAIN,
        2 => command_type::RELEASE_DOMAIN,
        3 => command_type::ADD_MAILBOX,
        4 => command_type::REMOVE_MAILBOX,
        5 => command_type::ADD_ALIAS,
        6 => command_type::REMOVE_ALIAS,
        7 => command_type::AUTHORIZE_CONTENT_SOURCE,
        8 => command_type::PUBLISH_WEBSITE,
        9 => command_type::UNPUBLISH_WEBSITE,
        10 => command_type::SET_AUTHORITATIVE_DNS,
        11 => command_type::REQUEST_CERTIFICATE,
        12 => command_type::SET_SERVICE_POLICY,
        _ => return Err("unknown_server_resource_event_kind".into()),
    };
    decode_command_payload(command_type_value, &event_payload)
}

fn embedded_content_ref(value: &ContentRef) -> WireValue {
    let mut out = Vec::new();
    struct_value(vec![
        field(1, text(&value.repo)),
        field(2, text(&value.commit)),
        field(3, text(&value.path)),
    ])
    .encode_wire(&mut out);
    WireValue::Bytes(out)
}

fn embedded_command_ref(value: &CommandRef) -> WireValue {
    let mut fields = vec![field(1, WireValue::Bytes(value.command_id.clone()))];
    if let Some(hash) = &value.command_hash {
        let mut digest = Vec::new();
        struct_value(vec![
            field(1, u64v(hash.algorithm as u64)),
            field(2, WireValue::Bytes(hash.value.clone())),
        ])
        .encode_wire(&mut digest);
        fields.push(field(2, WireValue::Bytes(digest)));
    }
    let mut out = Vec::new();
    struct_value(fields).encode_wire(&mut out);
    WireValue::Bytes(out)
}

fn required_content_ref(
    fields: &[edgerun_wire::WireField],
    tag: u32,
) -> Result<ContentRef, String> {
    let bytes = required_bytes(fields, tag, "content_ref")?;
    let fields = decode_embedded_struct(&bytes)?;
    Ok(ContentRef {
        repo: required_text(&fields, 1, "content_ref.repo")?,
        commit: required_text(&fields, 2, "content_ref.commit")?,
        path: required_text(&fields, 3, "content_ref.path")?,
    })
}

fn decode_top_struct(bytes: &[u8]) -> Result<Vec<edgerun_wire::WireField>, String> {
    match WireValue::from_wire_bytes(bytes).map_err(|e| format!("wire_decode_failed: {e:?}"))? {
        WireValue::Struct(fields) => Ok(fields),
        other => Err(format!("expected_struct_got_{other:?}")),
    }
}

fn decode_embedded_struct(bytes: &[u8]) -> Result<Vec<edgerun_wire::WireField>, String> {
    let mut reader = WireReader::new(bytes);
    let value = WireValue::decode_wire(&mut reader)
        .map_err(|e| format!("embedded_wire_decode_failed: {e:?}"))?;
    if !reader.is_empty() {
        return Err("embedded_wire_trailing_bytes".into());
    }
    match value {
        WireValue::Struct(fields) => Ok(fields),
        other => Err(format!("expected_embedded_struct_got_{other:?}")),
    }
}

fn find_field(fields: &[edgerun_wire::WireField], tag: u32) -> Option<&WireValue> {
    fields
        .iter()
        .find(|field| field.tag == tag)
        .map(|field| &field.value)
}

fn required_u32(fields: &[edgerun_wire::WireField], tag: u32, name: &str) -> Result<u32, String> {
    match find_field(fields, tag) {
        Some(WireValue::U64(value)) => {
            u32::try_from(*value).map_err(|_| format!("{name}_overflow"))
        }
        Some(other) => Err(format!("{name}_expected_u64_got_{other:?}")),
        None => Err(format!("{name}_required")),
    }
}

fn required_bool(fields: &[edgerun_wire::WireField], tag: u32, name: &str) -> Result<bool, String> {
    match find_field(fields, tag) {
        Some(WireValue::Bool(value)) => Ok(*value),
        Some(other) => Err(format!("{name}_expected_bool_got_{other:?}")),
        None => Err(format!("{name}_required")),
    }
}

fn required_text(
    fields: &[edgerun_wire::WireField],
    tag: u32,
    name: &str,
) -> Result<String, String> {
    match find_field(fields, tag) {
        Some(WireValue::Text(value)) => Ok(value.clone()),
        Some(other) => Err(format!("{name}_expected_text_got_{other:?}")),
        None => Err(format!("{name}_required")),
    }
}

fn required_bytes(
    fields: &[edgerun_wire::WireField],
    tag: u32,
    name: &str,
) -> Result<Vec<u8>, String> {
    match find_field(fields, tag) {
        Some(WireValue::Bytes(value)) => Ok(value.clone()),
        Some(other) => Err(format!("{name}_expected_bytes_got_{other:?}")),
        None => Err(format!("{name}_required")),
    }
}

fn required_text_list(
    fields: &[edgerun_wire::WireField],
    tag: u32,
    name: &str,
) -> Result<Vec<String>, String> {
    match find_field(fields, tag) {
        Some(WireValue::List(values)) => values
            .iter()
            .map(|value| match value {
                WireValue::Text(text) => Ok(text.clone()),
                other => Err(format!("{name}_entry_expected_text_got_{other:?}")),
            })
            .collect(),
        Some(other) => Err(format!("{name}_expected_list_got_{other:?}")),
        None => Err(format!("{name}_required")),
    }
}

fn text_list(values: &[String]) -> WireValue {
    WireValue::List(values.iter().map(|value| text(value)).collect())
}
