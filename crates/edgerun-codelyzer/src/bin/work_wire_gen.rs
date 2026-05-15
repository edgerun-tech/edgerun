use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

#[derive(Debug)]
struct TypeDef {
    name: String,
    kind: TypeKind,
}

#[derive(Debug)]
enum TypeKind {
    Struct(Vec<Field>),
    Enum(Vec<Variant>),
}

#[derive(Debug)]
struct Field {
    name: String,
    ty: String,
}

#[derive(Debug)]
struct Variant {
    name: String,
    payload: Option<String>,
}

fn run() -> Result<(), String> {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let src = root.join("crates/protocol/edgerun-work/src");
    let out = src.join("generated_wire.rs");
    let modules = [
        "protocol.rs",
        "node_control.rs",
        "channel.rs",
        "storage_payload.rs",
        "recipient_policy.rs",
        "admitted_route.rs",
        "notary_role.rs",
        "message_seal.rs",
        "capability_packet.rs",
        "program_io.rs",
        "chat_index.rs",
    ];
    let mut defs = Vec::new();
    for module in modules {
        let text =
            fs::read_to_string(src.join(module)).map_err(|err| format!("read {module}: {err}"))?;
        defs.extend(parse_defs(&text));
    }
    let generated = emit(&defs);
    fs::write(&out, generated).map_err(|err| format!("write {}: {err}", out.display()))?;
    println!("generated {}", out.display());
    Ok(())
}

fn parse_defs(text: &str) -> Vec<TypeDef> {
    let mut defs = Vec::new();
    let mut pos = 0usize;
    while let Some(next) = find_next_item(text, pos) {
        pos = next;
        let rest = &text[pos..];
        if rest.starts_with("pub struct ") {
            if let Some((def, next)) = parse_struct(text, pos) {
                if wire_name(&def.name) {
                    defs.push(def);
                }
                pos = next;
            }
        } else if rest.starts_with("pub enum ") {
            if let Some((def, next)) = parse_enum(text, pos) {
                if wire_name(&def.name) {
                    defs.push(def);
                }
                pos = next;
            }
        } else {
            pos += 1;
        }
    }
    defs
}

fn find_next_item(text: &str, pos: usize) -> Option<usize> {
    let struct_pos = text[pos..].find("pub struct ").map(|p| pos + p);
    let enum_pos = text[pos..].find("pub enum ").map(|p| pos + p);
    match (struct_pos, enum_pos) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn wire_name(name: &str) -> bool {
    !matches!(
        name,
        "WorkProtocolError"
            | "RecipientPolicyError"
            | "NotaryError"
            | "MessageSealError"
            | "ProgramIoError"
            | "ChatIndexError"
            | "ProgramIoRole"
            | "ProgramIoService"
            | "NotaryRole"
    )
}

fn parse_struct(text: &str, start: usize) -> Option<(TypeDef, usize)> {
    let header = &text[start + "pub struct ".len()..];
    let name = take_ident(header)?;
    let brace = text[start..].find('{').map(|p| start + p)?;
    let end = matching(text, brace, '{', '}')?;
    let body = &text[brace + 1..end];
    let fields = split_top_level(body, ',')
        .into_iter()
        .filter_map(|item| {
            let item = item.trim();
            let item = item.strip_prefix("pub ")?.trim();
            let (name, ty) = item.split_once(':')?;
            Some(Field {
                name: name.trim().to_string(),
                ty: ty.trim().to_string(),
            })
        })
        .collect();
    Some((
        TypeDef {
            name: name.to_string(),
            kind: TypeKind::Struct(fields),
        },
        end + 1,
    ))
}

fn parse_enum(text: &str, start: usize) -> Option<(TypeDef, usize)> {
    let header = &text[start + "pub enum ".len()..];
    let name = take_ident(header)?;
    let brace = text[start..].find('{').map(|p| start + p)?;
    let end = matching(text, brace, '{', '}')?;
    let body = &text[brace + 1..end];
    let variants = split_top_level(body, ',')
        .into_iter()
        .filter_map(|item| {
            let item = item.trim();
            if item.is_empty() {
                return None;
            }
            if let Some(open) = item.find('(') {
                let close = item.rfind(')')?;
                Some(Variant {
                    name: item[..open].trim().to_string(),
                    payload: Some(item[open + 1..close].trim().to_string()),
                })
            } else {
                Some(Variant {
                    name: item.to_string(),
                    payload: None,
                })
            }
        })
        .collect();
    Some((
        TypeDef {
            name: name.to_string(),
            kind: TypeKind::Enum(variants),
        },
        end + 1,
    ))
}

fn take_ident(value: &str) -> Option<&str> {
    let end = value
        .char_indices()
        .find_map(|(i, ch)| (!(ch == '_' || ch.is_ascii_alphanumeric())).then_some(i))?;
    Some(&value[..end])
}

fn matching(text: &str, open: usize, left: char, right: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (offset, ch) in text[open..].char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == left {
            depth += 1;
        } else if ch == right {
            depth -= 1;
            if depth == 0 {
                return Some(open + offset);
            }
        }
    }
    None
}

fn split_top_level(value: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    for (idx, ch) in value.char_indices() {
        match ch {
            '<' | '[' | '(' | '{' => depth += 1,
            '>' | ']' | ')' | '}' => depth -= 1,
            ch if ch == sep && depth == 0 => {
                out.push(&value[start..idx]);
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&value[start..]);
    out
}

fn emit(defs: &[TypeDef]) -> String {
    let mut out = String::new();
    out.push_str("// This file is @generated by edgerun-work-wire-gen.\n");
    out.push_str("// Do not hand edit; update protocol structs and regenerate.\n\n");
    out.push_str("use alloc::string::String;\nuse alloc::vec::Vec;\n\n");
    out.push_str("use crate::admitted_route::*;\n");
    out.push_str("use crate::capability_packet::*;\n");
    out.push_str("use crate::channel::*;\n");
    out.push_str("use crate::chat_index::*;\n");
    out.push_str("use crate::codec::{EdgeWire, WireCursor, WireWriter};\n");
    out.push_str("use crate::message_seal::*;\n");
    out.push_str("use crate::node_control::*;\n");
    out.push_str("use crate::notary_role::*;\n");
    out.push_str("use crate::program_io::*;\n");
    out.push_str("use crate::protocol::*;\n");
    out.push_str("use crate::protocol::WorkProtocolError;\n\n");
    out.push_str("use crate::recipient_policy::*;\n");
    out.push_str("use crate::storage_payload::*;\n\n");
    for def in defs {
        out.push_str(&format!("pub type Archived{} = {};\n", def.name, def.name));
    }
    out.push('\n');
    for def in defs {
        match &def.kind {
            TypeKind::Struct(fields) => emit_struct(&mut out, &def.name, fields),
            TypeKind::Enum(variants) => emit_enum(&mut out, &def.name, variants),
        }
    }
    out
}

fn emit_struct(out: &mut String, name: &str, fields: &[Field]) {
    out.push_str(&format!("impl EdgeWire for {name} {{\n"));
    out.push_str("    fn encode_wire(&self, out: &mut WireWriter) {\n");
    for field in fields {
        out.push_str(&format!("        self.{}.encode_wire(out);\n", field.name));
    }
    out.push_str("    }\n\n");
    out.push_str(
        "    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {\n",
    );
    out.push_str("        Ok(Self {\n");
    for field in fields {
        out.push_str(&format!(
            "            {}: <{} as EdgeWire>::decode_wire(input)?,\n",
            field.name,
            normalize_ty(&field.ty)
        ));
    }
    out.push_str("        })\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
}

fn emit_enum(out: &mut String, name: &str, variants: &[Variant]) {
    out.push_str(&format!("impl EdgeWire for {name} {{\n"));
    out.push_str("    fn encode_wire(&self, out: &mut WireWriter) {\n");
    out.push_str("        match self {\n");
    for (idx, variant) in variants.iter().enumerate() {
        match &variant.payload {
            Some(_) => out.push_str(&format!(
                "            Self::{}(value) => {{ ({}u16).encode_wire(out); value.encode_wire(out); }}\n",
                variant.name, idx
            )),
            None => out.push_str(&format!(
                "            Self::{} => {{ ({}u16).encode_wire(out); }}\n",
                variant.name, idx
            )),
        }
    }
    out.push_str("        }\n");
    out.push_str("    }\n\n");
    out.push_str(
        "    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {\n",
    );
    out.push_str("        match u16::decode_wire(input)? {\n");
    for (idx, variant) in variants.iter().enumerate() {
        match &variant.payload {
            Some(ty) => out.push_str(&format!(
                "            {idx} => Ok(Self::{}(<{} as EdgeWire>::decode_wire(input)?)),\n",
                variant.name,
                normalize_ty(ty)
            )),
            None => out.push_str(&format!(
                "            {idx} => Ok(Self::{}),\n",
                variant.name
            )),
        }
    }
    out.push_str("            _ => Err(WorkProtocolError::InvalidShape),\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
}

fn normalize_ty(ty: &str) -> String {
    ty.split_whitespace().collect::<String>()
}
