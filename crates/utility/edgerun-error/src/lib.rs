//! Small zero-dependency `Error` derive macro.
//!
//! This intentionally covers the subset of `thiserror` used by Edgerun crates:
//! unit, tuple, and named-field enum variants; `#[error("...")]`;
//! `#[error(transparent)]`; and field-level `#[source]` / `#[from]`.

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

#[proc_macro_derive(Error, attributes(error, source, from))]
pub fn error_derive(input: TokenStream) -> TokenStream {
    expand_error_derive(input).unwrap_or_else(|message| compile_error(&message))
}

fn expand_error_derive(input: TokenStream) -> Result<TokenStream, String> {
    let item = parse_item(input)?;

    match item.kind {
        ItemKind::Enum => expand_enum_error(item),
        ItemKind::Struct => expand_struct_error(item),
    }
}

fn expand_enum_error(item: Item) -> Result<TokenStream, String> {
    let enum_name = item.name;
    let body = item
        .body
        .ok_or_else(|| "Error derive expected enum body".to_string())?;
    let variants = parse_variants(body.stream())?;

    let mut display_arms = String::new();
    let mut source_arms = String::new();
    let mut from_impls = String::new();

    for variant in &variants {
        display_arms.push_str(&variant.display_arm()?);
        if let Some(arm) = variant.source_arm()? {
            source_arms.push_str(&arm);
        }
        if let Some(from_impl) = variant.from_impl(&enum_name)? {
            from_impls.push_str(&from_impl);
        }
    }

    format!(
        "impl ::core::fmt::Display for {enum_name} {{
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{
                match self {{ {display_arms} }}
            }}
        }}
        impl ::std::error::Error for {enum_name} {{
            fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {{
                match self {{
                    {source_arms}
                    _ => None,
                }}
            }}
        }}
        {from_impls}"
    )
    .parse()
    .map_err(|_| "Error derive failed to generate tokens".to_string())
}

fn expand_struct_error(item: Item) -> Result<TokenStream, String> {
    let struct_name = item.name;
    let Some(body) = item.body else {
        return Err("Error derive expected struct body".into());
    };
    let fields = match body.delimiter() {
        Delimiter::Brace => parse_named_fields(body.stream())?,
        Delimiter::Parenthesis => parse_tuple_fields(body.stream())?,
        _ => Vec::new(),
    };
    let message = match &item.message {
        ErrorMessage::Text { .. } | ErrorMessage::Default => match &item.message {
            ErrorMessage::Text { message, .. } => message.clone(),
            ErrorMessage::Default => struct_name.clone(),
            ErrorMessage::Transparent => unreachable!(),
        },
        ErrorMessage::Transparent => {
            if fields.len() != 1 {
                return Err(format!(
                    "transparent error struct `{struct_name}` must have exactly one field"
                ));
            }
            let pattern = struct_pattern(&struct_name, &fields, body.delimiter());
            let field = &fields[0].name;
            let source_arm =
                format!("{pattern} => Some({field} as &(dyn ::std::error::Error + 'static)),");
            return format!(
                "impl ::core::fmt::Display for {struct_name} {{
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{
                        match self {{ {pattern} => ::core::fmt::Display::fmt({field}, f) }}
                    }}
                }}
                impl ::std::error::Error for {struct_name} {{
                    fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {{
                        match self {{ {source_arm} }}
                    }}
                }}"
            )
            .parse()
            .map_err(|_| "Error derive failed to generate tokens".to_string());
        }
    };
    let pattern = struct_pattern(&struct_name, &fields, body.delimiter());
    let (message, args) = match &item.message {
        ErrorMessage::Text { args, .. } if !args.is_empty() => (message, args.clone()),
        _ if body.delimiter() == Delimiter::Brace => named_format_args(&message, &fields)?,
        _ => {
            let args = tuple_format_args(&message, fields.len());
            (message, args)
        }
    };
    let display_arm = write_arm(&pattern, &message, &args);
    let source_arm = fields
        .iter()
        .find(|field| field.is_source || field.is_from)
        .map(|field| {
            format!(
                "{pattern} => Some({} as &(dyn ::std::error::Error + 'static)),",
                field.name
            )
        })
        .unwrap_or_else(|| "_ => None,".to_string());

    format!(
        "impl ::core::fmt::Display for {struct_name} {{
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{
                match self {{ {display_arm} }}
            }}
        }}
        impl ::std::error::Error for {struct_name} {{
            fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {{
                match self {{ {source_arm} }}
            }}
        }}"
    )
    .parse()
    .map_err(|_| "Error derive failed to generate tokens".to_string())
}

struct Item {
    kind: ItemKind,
    name: String,
    body: Option<Group>,
    message: ErrorMessage,
}

enum ItemKind {
    Enum,
    Struct,
}

#[derive(Clone)]
struct Variant {
    name: String,
    message: ErrorMessage,
    fields: Fields,
}

fn parse_item(input: TokenStream) -> Result<Item, String> {
    let mut tokens = input.into_iter().peekable();
    let mut pending_message = ErrorMessage::Default;

    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == '#' => {
                if let Some(TokenTree::Group(attr)) = tokens.next() {
                    if let Some(message) = parse_error_attribute(&attr)? {
                        pending_message = message;
                    }
                }
            }
            TokenTree::Ident(ident)
                if ident.to_string() == "enum" || ident.to_string() == "struct" =>
            {
                let kind = if ident.to_string() == "enum" {
                    ItemKind::Enum
                } else {
                    ItemKind::Struct
                };
                let name = match tokens.next() {
                    Some(TokenTree::Ident(ident)) => ident.to_string(),
                    _ => return Err("Error derive expected item name".into()),
                };
                let body = tokens.find_map(|token| match token {
                    TokenTree::Group(group)
                        if matches!(
                            group.delimiter(),
                            Delimiter::Brace | Delimiter::Parenthesis
                        ) =>
                    {
                        Some(group)
                    }
                    _ => None,
                });
                return Ok(Item {
                    kind,
                    name,
                    body,
                    message: pending_message,
                });
            }
            _ => {}
        }
    }

    Err("Error derive only works on enums and structs".into())
}

#[derive(Clone)]
enum ErrorMessage {
    Text { message: String, args: String },
    Transparent,
    Default,
}

#[derive(Clone)]
enum Fields {
    Unit,
    Tuple(Vec<Field>),
    Named(Vec<Field>),
}

#[derive(Clone)]
struct Field {
    name: String,
    ty: String,
    is_source: bool,
    is_from: bool,
}

impl Variant {
    fn display_arm(&self) -> Result<String, String> {
        match &self.fields {
            Fields::Unit => match self.message_text()? {
                Some(message) => Ok(write_arm(
                    &format!("Self::{}", self.name),
                    &message,
                    self.explicit_format_args(),
                )),
                None => Err(format!(
                    "transparent error variant `{}` must have one field",
                    self.name
                )),
            },
            Fields::Tuple(fields) => {
                let bindings = fields
                    .iter()
                    .enumerate()
                    .map(|(idx, _)| format!("__field{idx}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                if matches!(self.message, ErrorMessage::Transparent) {
                    let field = self.transparent_field(fields)?;
                    return Ok(format!(
                        "Self::{}({bindings}) => ::core::fmt::Display::fmt({}, f),",
                        self.name, field.name
                    ));
                }
                let message = self.message_text()?.unwrap_or_else(|| self.name.clone());
                let args = if self.explicit_format_args().is_empty() {
                    tuple_format_args(&message, fields.len())
                } else {
                    self.explicit_format_args().to_string()
                };
                Ok(write_arm(
                    &format!("Self::{}({bindings})", self.name),
                    &message,
                    &args,
                ))
            }
            Fields::Named(fields) => {
                let bindings = fields
                    .iter()
                    .map(|field| format!("{}: {}", field.name, field.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                if matches!(self.message, ErrorMessage::Transparent) {
                    let field = self.transparent_field(fields)?;
                    return Ok(format!(
                        "Self::{} {{ {bindings} }} => ::core::fmt::Display::fmt({}, f),",
                        self.name, field.name
                    ));
                }
                let message = self.message_text()?.unwrap_or_else(|| self.name.clone());
                let (message, args) = if self.explicit_format_args().is_empty() {
                    named_format_args(&message, fields)?
                } else {
                    (message, self.explicit_format_args().to_string())
                };
                Ok(write_arm(
                    &format!("Self::{} {{ {bindings} }}", self.name),
                    &message,
                    &args,
                ))
            }
        }
    }

    fn source_arm(&self) -> Result<Option<String>, String> {
        let (pattern, source) = match &self.fields {
            Fields::Unit => return Ok(None),
            Fields::Tuple(fields) => {
                let bindings = fields
                    .iter()
                    .enumerate()
                    .map(|(idx, _)| format!("__field{idx}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let source = if matches!(self.message, ErrorMessage::Transparent) {
                    self.transparent_field(fields)?.name.clone()
                } else if let Some(field) =
                    fields.iter().find(|field| field.is_source || field.is_from)
                {
                    field.name.clone()
                } else {
                    return Ok(None);
                };
                (format!("Self::{}({bindings})", self.name), source)
            }
            Fields::Named(fields) => {
                let bindings = fields
                    .iter()
                    .map(|field| format!("{}: {}", field.name, field.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                let source = if matches!(self.message, ErrorMessage::Transparent) {
                    self.transparent_field(fields)?.name.clone()
                } else if let Some(field) =
                    fields.iter().find(|field| field.is_source || field.is_from)
                {
                    field.name.clone()
                } else {
                    return Ok(None);
                };
                (format!("Self::{} {{ {bindings} }}", self.name), source)
            }
        };

        Ok(Some(format!(
            "{pattern} => Some({source} as &(dyn ::std::error::Error + 'static)),"
        )))
    }

    fn from_impl(&self, enum_name: &str) -> Result<Option<String>, String> {
        let from_fields = match &self.fields {
            Fields::Unit => return Ok(None),
            Fields::Tuple(fields) | Fields::Named(fields) => fields
                .iter()
                .filter(|field| field.is_from)
                .collect::<Vec<_>>(),
        };
        if from_fields.is_empty() {
            return Ok(None);
        }
        if from_fields.len() > 1 {
            return Err(format!(
                "variant `{}` has multiple #[from] fields",
                self.name
            ));
        }
        let field = from_fields[0];
        let body = match &self.fields {
            Fields::Tuple(fields) if fields.len() == 1 => {
                format!("Self::{}(value)", self.name)
            }
            Fields::Named(fields) if fields.len() == 1 => {
                format!("Self::{} {{ {}: value }}", self.name, field.name)
            }
            _ => {
                return Err(format!(
                    "variant `{}` uses #[from] but does not have exactly one field",
                    self.name
                ));
            }
        };
        Ok(Some(format!(
            "impl ::core::convert::From<{}> for {enum_name} {{
                fn from(value: {}) -> Self {{ {body} }}
            }}",
            field.ty, field.ty
        )))
    }

    fn message_text(&self) -> Result<Option<String>, String> {
        match &self.message {
            ErrorMessage::Text { message, .. } => Ok(Some(message.clone())),
            ErrorMessage::Default => Ok(Some(self.name.clone())),
            ErrorMessage::Transparent => Ok(None),
        }
    }

    fn explicit_format_args(&self) -> &str {
        match &self.message {
            ErrorMessage::Text { args, .. } => args,
            ErrorMessage::Default | ErrorMessage::Transparent => "",
        }
    }

    fn transparent_field<'a>(&self, fields: &'a [Field]) -> Result<&'a Field, String> {
        if fields.len() == 1 {
            Ok(&fields[0])
        } else {
            Err(format!(
                "transparent error variant `{}` must have exactly one field",
                self.name
            ))
        }
    }
}

fn parse_variants(input: TokenStream) -> Result<Vec<Variant>, String> {
    let mut variants = Vec::new();
    let mut pending_message = ErrorMessage::Default;
    let mut tokens = input.into_iter().peekable();

    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == '#' => {
                if let Some(TokenTree::Group(attr)) = tokens.next() {
                    if let Some(message) = parse_error_attribute(&attr)? {
                        pending_message = message;
                    }
                }
            }
            TokenTree::Ident(ident) => {
                let name = ident.to_string();
                let fields = match tokens.peek() {
                    Some(TokenTree::Group(group))
                        if group.delimiter() == Delimiter::Parenthesis =>
                    {
                        let group = match tokens.next() {
                            Some(TokenTree::Group(group)) => group,
                            _ => unreachable!(),
                        };
                        Fields::Tuple(parse_tuple_fields(group.stream())?)
                    }
                    Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
                        let group = match tokens.next() {
                            Some(TokenTree::Group(group)) => group,
                            _ => unreachable!(),
                        };
                        Fields::Named(parse_named_fields(group.stream())?)
                    }
                    _ => Fields::Unit,
                };
                skip_to_variant_end(&mut tokens);
                variants.push(Variant {
                    name,
                    message: pending_message.clone(),
                    fields,
                });
                pending_message = ErrorMessage::Default;
            }
            _ => {}
        }
    }

    Ok(variants)
}

fn parse_tuple_fields(input: TokenStream) -> Result<Vec<Field>, String> {
    split_fields(input)
        .into_iter()
        .enumerate()
        .map(|(idx, tokens)| {
            let attrs = parse_field_attrs(&tokens)?;
            Ok(Field {
                name: format!("__field{idx}"),
                ty: tokens_without_attrs(&tokens).to_string(),
                is_source: attrs.is_source,
                is_from: attrs.is_from,
            })
        })
        .collect()
}

fn parse_named_fields(input: TokenStream) -> Result<Vec<Field>, String> {
    split_fields(input)
        .into_iter()
        .map(|tokens| {
            let attrs = parse_field_attrs(&tokens)?;
            let tokens = tokens_without_attrs(&tokens);
            let mut iter = tokens.into_iter().peekable();
            if matches!(iter.peek(), Some(TokenTree::Ident(ident)) if ident.to_string() == "pub") {
                let _ = iter.next();
            }
            let name = match iter.next() {
                Some(TokenTree::Ident(ident)) => ident.to_string(),
                _ => return Err("Error derive expected named field".into()),
            };
            match iter.next() {
                Some(TokenTree::Punct(punct)) if punct.as_char() == ':' => {}
                _ => return Err(format!("Error derive expected type for field `{name}`")),
            }
            let ty = iter.collect::<TokenStream>().to_string();
            Ok(Field {
                name,
                ty,
                is_source: attrs.is_source,
                is_from: attrs.is_from,
            })
        })
        .collect()
}

#[derive(Default)]
struct FieldAttrs {
    is_source: bool,
    is_from: bool,
}

fn parse_field_attrs(tokens: &[TokenTree]) -> Result<FieldAttrs, String> {
    let mut attrs = FieldAttrs::default();
    let mut iter = tokens.iter();
    while let Some(token) = iter.next() {
        if !matches!(token, TokenTree::Punct(punct) if punct.as_char() == '#') {
            continue;
        }
        let Some(TokenTree::Group(group)) = iter.next() else {
            continue;
        };
        if group.delimiter() != Delimiter::Bracket {
            continue;
        }
        let mut attr_tokens = group.stream().into_iter();
        let Some(TokenTree::Ident(ident)) = attr_tokens.next() else {
            continue;
        };
        match ident.to_string().as_str() {
            "source" => attrs.is_source = true,
            "from" => attrs.is_from = true,
            "error" => {}
            _ => {}
        }
    }
    Ok(attrs)
}

fn parse_error_attribute(attr: &Group) -> Result<Option<ErrorMessage>, String> {
    if attr.delimiter() != Delimiter::Bracket {
        return Ok(None);
    }

    let mut tokens = attr.stream().into_iter();
    if !matches!(tokens.next(), Some(TokenTree::Ident(ident)) if ident.to_string() == "error") {
        return Ok(None);
    }

    match tokens.next() {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
            if single_ident(group.stream().clone(), "transparent") {
                Ok(Some(ErrorMessage::Transparent))
            } else {
                parse_error_message(group.stream()).map(Some)
            }
        }
        Some(TokenTree::Punct(punct)) if punct.as_char() == '=' => {
            parse_error_message(tokens.collect()).map(Some)
        }
        _ => Err("Error derive expected #[error(\"message\")] or #[error(transparent)]".into()),
    }
}

fn parse_error_message(input: TokenStream) -> Result<ErrorMessage, String> {
    let mut tokens = input.into_iter();
    let mut message = None;
    let mut args = Vec::new();
    let mut saw_literal = false;
    let mut skip_first_comma = true;

    while let Some(token) = tokens.next() {
        if !saw_literal {
            if let TokenTree::Literal(literal) = token {
                message = Some(parse_string_literal(&literal.to_string())?);
                saw_literal = true;
            }
            continue;
        }

        if skip_first_comma {
            skip_first_comma = false;
            if matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',') {
                continue;
            }
        }
        args.push(token);
        args.extend(tokens);
        break;
    }

    let message = message.ok_or_else(|| "Error derive expected string literal".to_string())?;
    let args =
        normalize_explicit_format_args(&args.into_iter().collect::<TokenStream>().to_string());
    Ok(ErrorMessage::Text { message, args })
}

fn normalize_explicit_format_args(args: &str) -> String {
    let mut out = String::with_capacity(args.len());
    let mut chars = args.chars().peekable();
    let mut at_expr_start = true;

    while let Some(ch) = chars.next() {
        if ch == '.' && at_expr_start {
            while matches!(chars.peek(), Some(next) if next.is_whitespace()) {
                let _ = chars.next();
            }
            at_expr_start = false;
            continue;
        }

        if ch == ',' {
            at_expr_start = true;
        } else if !ch.is_whitespace() {
            at_expr_start = false;
        }
        out.push(ch);
    }

    out
}

fn split_fields(input: TokenStream) -> Vec<Vec<TokenTree>> {
    let mut fields = Vec::new();
    let mut field = Vec::new();
    for token in input {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == ',' => {
                if !field.is_empty() {
                    fields.push(core::mem::take(&mut field));
                }
            }
            token => field.push(token),
        }
    }
    if !field.is_empty() {
        fields.push(field);
    }
    fields
}

fn tokens_without_attrs(tokens: &[TokenTree]) -> TokenStream {
    let mut out = Vec::new();
    let mut iter = tokens.iter().peekable();
    while let Some(token) = iter.next() {
        if matches!(token, TokenTree::Punct(punct) if punct.as_char() == '#')
            && matches!(iter.peek(), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket)
        {
            let _ = iter.next();
            continue;
        }
        out.push(token.clone());
    }
    out.into_iter().collect()
}

fn write_arm(pattern: &str, message: &str, args: &str) -> String {
    let message = escape_format_literal(message);
    if args.is_empty() {
        format!("{pattern} => write!(f, \"{message}\"),")
    } else {
        format!("{pattern} => write!(f, \"{message}\", {args}),")
    }
}

fn struct_pattern(struct_name: &str, fields: &[Field], delimiter: Delimiter) -> String {
    match delimiter {
        Delimiter::Brace => {
            let bindings = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, field.name))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Self {{ {bindings} }}")
        }
        Delimiter::Parenthesis => {
            let bindings = fields
                .iter()
                .map(|field| field.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            format!("Self({bindings})")
        }
        _ => struct_name.to_string(),
    }
}

fn tuple_format_args(message: &str, field_count: usize) -> String {
    (0..format_arg_count(message).unwrap_or(field_count))
        .map(|idx| format!("__field{idx}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn named_format_args(message: &str, fields: &[Field]) -> Result<(String, String), String> {
    let mut out = String::with_capacity(message.len());
    let mut args = Vec::new();
    let bytes = message.as_bytes();
    let mut idx = 0usize;

    while idx < bytes.len() {
        if bytes[idx] != b'{' || idx + 1 >= bytes.len() || bytes[idx + 1] == b'{' {
            out.push(bytes[idx] as char);
            idx += 1;
            continue;
        }

        let name_start = idx + 1;
        let mut cursor = name_start;
        if cursor >= bytes.len() || !is_ident_start(bytes[cursor]) {
            out.push(bytes[idx] as char);
            idx += 1;
            continue;
        }
        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
            cursor += 1;
        }

        let name = &message[name_start..cursor];
        let Some(field) = fields.iter().find(|field| field.name == name) else {
            out.push_str(&message[idx..cursor]);
            idx = cursor;
            continue;
        };

        match bytes.get(cursor).copied() {
            Some(b'}') => {
                out.push_str("{}");
                args.push(format_arg_for_field(field));
                idx = cursor + 1;
            }
            Some(b':') => {
                let Some(end) = message[cursor..].find('}') else {
                    return Err("Error derive found unclosed named format argument".into());
                };
                out.push('{');
                out.push_str(&message[cursor..cursor + end + 1]);
                args.push(format_arg_for_field(field));
                idx = cursor + end + 1;
            }
            _ => {
                out.push_str(&message[idx..cursor]);
                idx = cursor;
            }
        }
    }

    Ok((out, args.join(", ")))
}

fn format_arg_for_field(field: &Field) -> String {
    let ty = field.ty.replace(' ', "");
    if ty.ends_with("PathBuf") || ty.ends_with("Path") {
        format!("{}.display()", field.name)
    } else {
        field.name.clone()
    }
}

fn is_ident_start(value: u8) -> bool {
    value == b'_' || value.is_ascii_alphabetic()
}

fn is_ident_continue(value: u8) -> bool {
    is_ident_start(value) || value.is_ascii_digit()
}

fn escape_format_literal(message: &str) -> String {
    message.replace('\\', "\\\\").replace('"', "\\\"")
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
    let mut saw_placeholder = false;
    let bytes = format_str.as_bytes();
    let mut idx = 0;

    while idx < bytes.len() {
        if bytes[idx] != b'{' || idx + 1 >= bytes.len() || bytes[idx + 1] == b'{' {
            idx += 1;
            continue;
        }
        saw_placeholder = true;

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

    if let Some(index) = max_index {
        Some(index + 1)
    } else if saw_placeholder {
        None
    } else {
        Some(0)
    }
}

fn single_ident(input: TokenStream, expected: &str) -> bool {
    let mut tokens = input.into_iter();
    matches!(tokens.next(), Some(TokenTree::Ident(ident)) if ident.to_string() == expected)
        && tokens.next().is_none()
}

fn is_ident(token: &TokenTree, expected: &str) -> bool {
    matches!(token, TokenTree::Ident(ident) if ident.to_string() == expected)
}

fn compile_error(message: &str) -> TokenStream {
    format!("compile_error!(\"{}\");", message.replace('"', "\\\""))
        .parse()
        .expect("compile_error tokens")
}
