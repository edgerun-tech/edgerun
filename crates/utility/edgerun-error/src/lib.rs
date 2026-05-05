//! Zero-dependency Error derive macro.
//!
//! Supports:
//! - Unit variants: `#[error("message")]`
//! - Tuple variants: `#[error("message: {0}")]`

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

#[proc_macro_derive(Error, attributes(error))]
pub fn error_derive(input: TokenStream) -> TokenStream {
    expand_error_derive(input).unwrap_or_else(|message| compile_error(&message))
}

fn expand_error_derive(input: TokenStream) -> Result<TokenStream, String> {
    let mut tokens = input.into_iter();
    let mut enum_name = None;
    let mut body = None;

    while let Some(token) = tokens.next() {
        if is_ident(&token, "enum") {
            enum_name = match tokens.next() {
                Some(TokenTree::Ident(ident)) => Some(ident.to_string()),
                _ => return Err("Error derive expected enum name".into()),
            };
            break;
        }
    }

    for token in tokens {
        if let TokenTree::Group(group) = token {
            if group.delimiter() == Delimiter::Brace {
                body = Some(group);
                break;
            }
        }
    }

    let enum_name = enum_name.ok_or_else(|| "Error derive only works on enums".to_string())?;
    let body = body.ok_or_else(|| "Error derive expected enum body".to_string())?;
    let variants = parse_variants(body.stream())?;
    let mut arms = String::new();

    for variant in variants {
        let message = variant
            .message
            .unwrap_or_else(|| variant.name.clone())
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        if variant.field_count == 0 {
            arms.push_str(&format!(
                "Self::{} => write!(f, \"{}\"),",
                variant.name, message
            ));
            continue;
        }

        let bindings = (0..variant.field_count)
            .map(|idx| format!("ref __field{idx}"))
            .collect::<Vec<_>>()
            .join(", ");
        let args = (0..format_arg_count(&message).unwrap_or(variant.field_count))
            .map(|idx| format!("__field{idx}"))
            .collect::<Vec<_>>()
            .join(", ");
        if args.is_empty() {
            arms.push_str(&format!(
                "Self::{}({}) => write!(f, \"{}\"),",
                variant.name, bindings, message
            ));
        } else {
            arms.push_str(&format!(
                "Self::{}({}) => write!(f, \"{}\", {}),",
                variant.name, bindings, message, args
            ));
        }
    }

    format!(
        "impl ::core::fmt::Display for {enum_name} {{
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{
                match self {{ {arms} }}
            }}
        }}
        impl ::core::error::Error for {enum_name} {{
            fn source(&self) -> Option<&(dyn ::core::error::Error + 'static)> {{
                None
            }}
        }}"
    )
    .parse()
    .map_err(|_| "Error derive failed to generate tokens".to_string())
}

struct Variant {
    name: String,
    field_count: usize,
    message: Option<String>,
}

fn parse_variants(input: TokenStream) -> Result<Vec<Variant>, String> {
    let mut variants = Vec::new();
    let mut pending_message = None;
    let mut tokens = input.into_iter().peekable();

    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == '#' => {
                if let Some(TokenTree::Group(attr)) = tokens.next() {
                    if let Some(message) = parse_error_attribute(&attr)? {
                        pending_message = Some(message);
                    }
                }
            }
            TokenTree::Ident(ident) => {
                let name = ident.to_string();
                let field_count = match tokens.peek() {
                    Some(TokenTree::Group(group))
                        if group.delimiter() == Delimiter::Parenthesis =>
                    {
                        let group = match tokens.next() {
                            Some(TokenTree::Group(group)) => group,
                            _ => unreachable!(),
                        };
                        count_tuple_fields(group.stream())
                    }
                    _ => 0,
                };
                skip_to_variant_end(&mut tokens);
                variants.push(Variant {
                    name,
                    field_count,
                    message: pending_message.take(),
                });
            }
            _ => {}
        }
    }

    Ok(variants)
}

fn parse_error_attribute(attr: &Group) -> Result<Option<String>, String> {
    if attr.delimiter() != Delimiter::Bracket {
        return Ok(None);
    }

    let mut tokens = attr.stream().into_iter();
    if !matches!(tokens.next(), Some(TokenTree::Ident(ident)) if ident.to_string() == "error") {
        return Ok(None);
    }

    match tokens.next() {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
            parse_single_string_literal(group.stream()).map(Some)
        }
        Some(TokenTree::Punct(punct)) if punct.as_char() == '=' => {
            parse_single_string_literal(tokens.collect()).map(Some)
        }
        _ => Err("Error derive expected #[error(\"message\")]".into()),
    }
}

fn parse_single_string_literal(input: TokenStream) -> Result<String, String> {
    for token in input {
        if let TokenTree::Literal(literal) = token {
            return parse_string_literal(&literal.to_string());
        }
    }
    Err("Error derive expected string literal".into())
}

fn parse_string_literal(raw: &str) -> Result<String, String> {
    let Some(inner) = raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err("Error derive expected ordinary string literal".into());
    };
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => return Err("Error derive string literal has trailing escape".into()),
        }
    }
    Ok(out)
}

fn count_tuple_fields(input: TokenStream) -> usize {
    let mut count = 0usize;
    let mut saw_token = false;
    for token in input {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == ',' => {
                count = count.wrapping_add(1);
                saw_token = false;
            }
            _ => saw_token = true,
        }
    }
    if saw_token {
        count.wrapping_add(1)
    } else {
        count
    }
}

fn skip_to_variant_end(tokens: &mut core::iter::Peekable<impl Iterator<Item = TokenTree>>) {
    while let Some(token) = tokens.peek() {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == ',' => {
                let _ = tokens.next();
                break;
            }
            _ => {
                let _ = tokens.next();
            }
        }
    }
}

fn format_arg_count(format_str: &str) -> Option<usize> {
    let mut max_index = None;
    let bytes = format_str.as_bytes();
    let mut idx = 0;

    while idx < bytes.len() {
        if bytes[idx] != b'{' || idx + 1 >= bytes.len() || bytes[idx + 1] == b'{' {
            idx += 1;
            continue;
        }

        let mut cursor = idx + 1;
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }

        if cursor > start {
            if let Ok(position) = format_str[start..cursor].parse::<usize>() {
                max_index = Some(max_index.map_or(position, |max: usize| max.max(position)));
            }
        } else if cursor < bytes.len() && (bytes[cursor] == b'}' || bytes[cursor] == b':') {
            return None;
        }

        idx = cursor + 1;
    }

    max_index.map(|index| index + 1)
}

fn is_ident(token: &TokenTree, expected: &str) -> bool {
    matches!(token, TokenTree::Ident(ident) if ident.to_string() == expected)
}

fn compile_error(message: &str) -> TokenStream {
    format!("compile_error!(\"{}\");", message.replace('"', "\\\""))
        .parse()
        .expect("compile_error tokens")
}
