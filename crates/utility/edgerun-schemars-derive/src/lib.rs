use proc_macro::{Delimiter, TokenStream, TokenTree};

#[proc_macro_derive(JsonSchema, attributes(schemars, serde))]
pub fn derive_json_schema(input: TokenStream) -> TokenStream {
    let item = match parse_item(input) {
        Some(item) => item,
        None => return compile_error("JsonSchema derive only supports structs and enums"),
    };

    let impl_generics = if item.generics.is_empty() {
        String::new()
    } else {
        item.generics.clone()
    };
    let ty_generics = impl_generics.clone();
    let where_clause = if item.type_params.is_empty() {
        String::new()
    } else {
        format!(
            " where {}",
            item.type_params
                .iter()
                .map(|param| format!("{param}: schemars::JsonSchema"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let schema_body = match item.kind {
        ItemKind::Struct => struct_schema_body(&item),
        ItemKind::Enum => enum_schema_body(&item),
    };

    let expanded = format!(
        "impl {impl_generics} schemars::JsonSchema for {name}{ty_generics} {where_clause} {{
            fn schema_name() -> ::std::string::String {{
                ::std::string::String::from(\"{title}\")
            }}

            fn json_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {{
                {schema_body}
            }}
        }}",
        name = item.name,
        title = item.schema_name,
    );
    expanded
        .parse()
        .unwrap_or_else(|_| compile_error("failed to generate JsonSchema impl"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    Struct,
    Enum,
}

struct Item {
    kind: ItemKind,
    name: String,
    schema_name: String,
    generics: String,
    type_params: Vec<String>,
    attrs: Vec<String>,
    body: Option<TokenStream>,
}

#[derive(Clone)]
struct Field {
    name: String,
    ty: String,
    attrs: Vec<String>,
}

#[derive(Clone)]
struct Variant {
    name: String,
    attrs: Vec<String>,
    body: Option<(Delimiter, TokenStream)>,
}

fn parse_item(input: TokenStream) -> Option<Item> {
    let tokens = input.into_iter().collect::<Vec<_>>();
    let kind_index = tokens.iter().position(|token| {
        matches!(token, TokenTree::Ident(ident) if ident.to_string() == "struct" || ident.to_string() == "enum")
    })?;
    let kind = match tokens.get(kind_index)? {
        TokenTree::Ident(ident) if ident.to_string() == "struct" => ItemKind::Struct,
        TokenTree::Ident(_) => ItemKind::Enum,
        _ => return None,
    };
    let attrs = collect_attrs(&tokens[..kind_index]);
    let name_index = kind_index + 1;
    let name = match tokens.get(name_index)? {
        TokenTree::Ident(ident) => ident.to_string(),
        _ => return None,
    };
    let schema_name = attr_value(&attrs, "schemars", "rename").unwrap_or_else(|| name.clone());

    let mut generics = String::new();
    let mut type_params = Vec::new();
    let mut after_name = name_index + 1;
    if let Some(TokenTree::Punct(punct)) = tokens.get(after_name)
        && punct.as_char() == '<'
    {
        let mut depth = 0usize;
        let mut parts = Vec::new();
        for (index, token) in tokens.iter().enumerate().skip(after_name) {
            match token {
                TokenTree::Punct(punct) if punct.as_char() == '<' => {
                    depth += 1;
                    parts.push(token.to_string());
                }
                TokenTree::Punct(punct) if punct.as_char() == '>' => {
                    depth = depth.saturating_sub(1);
                    parts.push(token.to_string());
                    if depth == 0 {
                        after_name = index + 1;
                        break;
                    }
                }
                _ => parts.push(token.to_string()),
            }
        }
        generics = parts.join(" ");
        type_params = parse_type_params(&generics);
    }

    let body = tokens
        .iter()
        .skip(after_name)
        .find_map(|token| match token {
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                Some(group.stream())
            }
            TokenTree::Group(group) if kind == ItemKind::Struct => Some(group.stream()),
            _ => None,
        });

    Some(Item {
        kind,
        name,
        schema_name,
        generics,
        type_params,
        attrs,
        body,
    })
}

fn struct_schema_body(item: &Item) -> String {
    let Some(body) = item.body.clone() else {
        return titled_object_schema(&item.schema_name);
    };
    let fields = parse_struct_fields(body);
    let rename_all = attr_value(&item.attrs, "serde", "rename_all");
    let deny_unknown = has_attr_flag(&item.attrs, "serde", "deny_unknown_fields")
        || has_attr_flag(&item.attrs, "schemars", "deny_unknown_fields");
    object_schema_code(
        &item.schema_name,
        &fields,
        rename_all.as_deref(),
        deny_unknown,
    )
}

fn enum_schema_body(item: &Item) -> String {
    let Some(body) = item.body.clone() else {
        return titled_object_schema(&item.schema_name);
    };
    let variants = parse_enum_variants(body);
    let rename_all = attr_value(&item.attrs, "serde", "rename_all");
    let tag = attr_value(&item.attrs, "serde", "tag");
    let untagged = has_attr_flag(&item.attrs, "serde", "untagged");

    if variants.iter().all(|variant| variant.body.is_none()) && tag.is_none() && !untagged {
        let values = variants
            .iter()
            .filter(|variant| !has_skip(&variant.attrs))
            .map(|variant| {
                let name = renamed(&variant.attrs, rename_all.as_deref(), &variant.name);
                format!(
                    "schemars::schema::JsonValue::String(::std::string::String::from(\"{}\"))",
                    escape(&name)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        return format!(
            "schemars::schema::Schema::Object(schemars::schema::SchemaObject {{
                metadata: Some(Box::new(schemars::schema::Metadata {{ title: Some(::std::string::String::from(\"{}\")), ..Default::default() }})),
                instance_type: Some(schemars::schema::InstanceType::String.into()),
                enum_values: Some(vec![{values}]),
                ..Default::default()
            }})",
            escape(&item.schema_name),
        );
    }

    let mut variant_schemas = Vec::new();
    for variant in variants.iter().filter(|variant| !has_skip(&variant.attrs)) {
        let variant_name = renamed(&variant.attrs, rename_all.as_deref(), &variant.name);
        if let Some(tag) = tag.as_deref() {
            let fields = variant_fields(variant);
            variant_schemas.push(wrap_block(tagged_variant_schema_code(
                tag,
                &variant_name,
                &fields,
            )));
        } else if untagged {
            let fields = variant_fields(variant);
            variant_schemas.push(wrap_block(object_schema_code(
                &variant_name,
                &fields,
                None,
                false,
            )));
        } else {
            let fields = variant_fields(variant);
            variant_schemas.push(wrap_block(tagged_variant_schema_code(
                "type",
                &variant_name,
                &fields,
            )));
        }
    }
    format!(
        "schemars::schema::Schema::Object(schemars::schema::SchemaObject {{
            metadata: Some(Box::new(schemars::schema::Metadata {{ title: Some(::std::string::String::from(\"{}\")), ..Default::default() }})),
            subschemas: Some(Box::new(schemars::schema::SubschemaValidation {{ one_of: Some(vec![{}]), ..Default::default() }})),
            ..Default::default()
        }})",
        escape(&item.schema_name),
        variant_schemas.join(", "),
    )
}

fn object_schema_code(
    title: &str,
    fields: &[Field],
    rename_all: Option<&str>,
    deny_unknown: bool,
) -> String {
    let mut code = String::new();
    code.push_str("let mut properties = ::std::collections::BTreeMap::new();");
    code.push_str("let mut required = Vec::new();");
    for field in fields.iter().filter(|field| !has_skip(&field.attrs)) {
        let field_name = renamed(&field.attrs, rename_all, &field.name);
        let ty = attr_value(&field.attrs, "schemars", "with").unwrap_or_else(|| field.ty.clone());
        code.push_str(&format!(
            "properties.insert(::std::string::String::from(\"{}\"), <{} as schemars::JsonSchema>::json_schema(generator));",
            escape(&field_name),
            ty,
        ));
        if !is_optional_type(&field.ty) && !has_attr_flag(&field.attrs, "serde", "default") {
            code.push_str(&format!(
                "required.push(::std::string::String::from(\"{}\"));",
                escape(&field_name),
            ));
        }
    }
    let additional = if deny_unknown {
        "Some(schemars::schema::AdditionalProperties::Bool(false))"
    } else {
        "None"
    };
    code.push_str(&format!(
        "schemars::schema::Schema::Object(schemars::schema::SchemaObject {{
            metadata: Some(Box::new(schemars::schema::Metadata {{ title: Some(::std::string::String::from(\"{}\")), ..Default::default() }})),
            instance_type: Some(schemars::schema::InstanceType::Object.into()),
            properties,
            required,
            additional_properties: {additional},
            ..Default::default()
        }})",
        escape(title),
    ));
    code
}

fn tagged_variant_schema_code(tag: &str, variant_name: &str, fields: &[Field]) -> String {
    let mut code = String::new();
    code.push_str("let mut properties = ::std::collections::BTreeMap::new();");
    code.push_str("let mut required = Vec::new();");
    code.push_str(&format!(
        "properties.insert(::std::string::String::from(\"{}\"), schemars::schema::Schema::Object(schemars::schema::SchemaObject {{
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            const_value: Some(schemars::schema::JsonValue::String(::std::string::String::from(\"{}\"))),
            ..Default::default()
        }}));",
        escape(tag),
        escape(variant_name),
    ));
    code.push_str(&format!(
        "required.push(::std::string::String::from(\"{}\"));",
        escape(tag)
    ));
    for field in fields.iter().filter(|field| !has_skip(&field.attrs)) {
        let field_name = renamed(&field.attrs, Some("camelCase"), &field.name);
        let ty = attr_value(&field.attrs, "schemars", "with").unwrap_or_else(|| field.ty.clone());
        code.push_str(&format!(
            "properties.insert(::std::string::String::from(\"{}\"), <{} as schemars::JsonSchema>::json_schema(generator));",
            escape(&field_name),
            ty,
        ));
        if !is_optional_type(&field.ty) && !has_attr_flag(&field.attrs, "serde", "default") {
            code.push_str(&format!(
                "required.push(::std::string::String::from(\"{}\"));",
                escape(&field_name),
            ));
        }
    }
    code.push_str(
        "schemars::schema::Schema::Object(schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::Object.into()),
            properties,
            required,
            additional_properties: Some(schemars::schema::AdditionalProperties::Bool(false)),
            ..Default::default()
        })",
    );
    code
}

fn titled_object_schema(title: &str) -> String {
    format!(
        "schemars::schema::Schema::Object(schemars::schema::SchemaObject {{
            metadata: Some(Box::new(schemars::schema::Metadata {{ title: Some(::std::string::String::from(\"{}\")), ..Default::default() }})),
            instance_type: Some(schemars::schema::InstanceType::Object.into()),
            ..Default::default()
        }})",
        escape(title)
    )
}

fn variant_fields(variant: &Variant) -> Vec<Field> {
    match &variant.body {
        Some((Delimiter::Brace, body)) => parse_struct_fields(body.clone()),
        Some((Delimiter::Parenthesis, body)) => parse_tuple_fields(body.clone()),
        _ => Vec::new(),
    }
}

fn parse_struct_fields(body: TokenStream) -> Vec<Field> {
    split_top_level(body, ',')
        .into_iter()
        .filter_map(parse_named_field)
        .collect()
}

fn parse_tuple_fields(body: TokenStream) -> Vec<Field> {
    split_top_level(body, ',')
        .into_iter()
        .enumerate()
        .filter_map(|(index, tokens)| {
            let attrs = collect_attrs(&tokens);
            let ty_tokens = strip_attrs_and_visibility(tokens);
            let ty = tokens_to_type(&ty_tokens);
            (!ty.is_empty()).then(|| Field {
                name: format!("field{index}"),
                ty,
                attrs,
            })
        })
        .collect()
}

fn parse_named_field(tokens: Vec<TokenTree>) -> Option<Field> {
    let attrs = collect_attrs(&tokens);
    let tokens = strip_attrs_and_visibility(tokens);
    let colon = tokens
        .iter()
        .position(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ':'))?;
    let name = tokens[..colon].iter().find_map(|token| match token {
        TokenTree::Ident(ident) => Some(ident.to_string()),
        _ => None,
    })?;
    let ty = tokens_to_type(&tokens[colon + 1..]);
    (!ty.is_empty()).then_some(Field { name, ty, attrs })
}

fn parse_enum_variants(body: TokenStream) -> Vec<Variant> {
    split_top_level(body, ',')
        .into_iter()
        .filter_map(|tokens| {
            let attrs = collect_attrs(&tokens);
            let tokens = strip_attrs_and_visibility(tokens);
            let name = tokens.iter().find_map(|token| match token {
                TokenTree::Ident(ident) => Some(ident.to_string()),
                _ => None,
            })?;
            let body = tokens.iter().find_map(|token| match token {
                TokenTree::Group(group) => Some((group.delimiter(), group.stream())),
                _ => None,
            });
            Some(Variant { name, attrs, body })
        })
        .collect()
}

fn split_top_level(stream: TokenStream, delimiter: char) -> Vec<Vec<TokenTree>> {
    let mut parts = Vec::new();
    let mut current = Vec::new();
    let mut angle_depth = 0usize;
    for token in stream {
        match &token {
            TokenTree::Punct(punct) if punct.as_char() == '<' => {
                angle_depth += 1;
                current.push(token);
            }
            TokenTree::Punct(punct) if punct.as_char() == '>' => {
                angle_depth = angle_depth.saturating_sub(1);
                current.push(token);
            }
            TokenTree::Punct(punct) if punct.as_char() == delimiter && angle_depth == 0 => {
                if !current.is_empty() {
                    parts.push(current);
                    current = Vec::new();
                }
            }
            _ => current.push(token),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

fn wrap_block(code: String) -> String {
    format!("{{ {code} }}")
}

fn collect_attrs(tokens: &[TokenTree]) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut iter = tokens.iter();
    while let Some(token) = iter.next() {
        if matches!(token, TokenTree::Punct(punct) if punct.as_char() == '#')
            && let Some(TokenTree::Group(group)) = iter.next()
            && group.delimiter() == Delimiter::Bracket
        {
            attrs.push(group.stream().to_string());
        }
    }
    attrs
}

fn strip_attrs_and_visibility(tokens: Vec<TokenTree>) -> Vec<TokenTree> {
    let mut out = Vec::new();
    let mut skip_group_after_hash = false;
    let mut skip_pub_group = false;
    for token in tokens {
        if skip_group_after_hash {
            skip_group_after_hash = false;
            continue;
        }
        if matches!(&token, TokenTree::Punct(punct) if punct.as_char() == '#') {
            skip_group_after_hash = true;
            continue;
        }
        if matches!(&token, TokenTree::Ident(ident) if ident.to_string() == "pub") {
            skip_pub_group = true;
            continue;
        }
        if skip_pub_group
            && matches!(&token, TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis)
        {
            skip_pub_group = false;
            continue;
        }
        skip_pub_group = false;
        out.push(token);
    }
    out
}

fn tokens_to_type(tokens: &[TokenTree]) -> String {
    let mut out = String::new();
    let mut prev_was_word = false;
    for token in tokens {
        let text = token.to_string();
        let is_word = matches!(token, TokenTree::Ident(_) | TokenTree::Literal(_));
        if prev_was_word && is_word {
            out.push(' ');
        }
        out.push_str(&text);
        prev_was_word = is_word;
    }
    out
}

fn attr_value(attrs: &[String], attr_name: &str, key: &str) -> Option<String> {
    attrs
        .iter()
        .filter(|attr| attr_matches(attr, attr_name))
        .find_map(|attr| find_key_value(attr, key))
}

fn has_attr_flag(attrs: &[String], attr_name: &str, flag: &str) -> bool {
    attrs
        .iter()
        .filter(|attr| attr_matches(attr, attr_name))
        .any(|attr| attr.contains(flag))
}

fn attr_matches(attr: &str, attr_name: &str) -> bool {
    attr == attr_name
        || attr
            .strip_prefix(attr_name)
            .is_some_and(|rest| rest.starts_with([' ', '(']))
}

fn has_skip(attrs: &[String]) -> bool {
    has_attr_flag(attrs, "serde", "skip") || has_attr_flag(attrs, "schemars", "skip")
}

fn find_key_value(attr: &str, key: &str) -> Option<String> {
    let needle = format!("{key} =");
    let start = attr.find(&needle)? + needle.len();
    let rest = attr[start..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn renamed(attrs: &[String], rename_all: Option<&str>, rust_name: &str) -> String {
    attr_value(attrs, "schemars", "rename")
        .or_else(|| attr_value(attrs, "serde", "rename"))
        .unwrap_or_else(|| apply_rename_all(rust_name, rename_all))
}

fn apply_rename_all(name: &str, rename_all: Option<&str>) -> String {
    match rename_all {
        Some("camelCase") => snake_to_camel(name),
        Some("PascalCase") => snake_to_pascal(name),
        Some("kebab-case") => name.replace('_', "-"),
        Some("SCREAMING_SNAKE_CASE") => name.to_ascii_uppercase(),
        Some("lowercase") => name.to_ascii_lowercase(),
        Some("snake_case") | None => name.to_string(),
        _ => name.to_string(),
    }
}

fn snake_to_camel(name: &str) -> String {
    let mut out = String::new();
    let mut uppercase_next = false;
    for ch in name.chars() {
        if ch == '_' {
            uppercase_next = true;
        } else if uppercase_next {
            out.extend(ch.to_uppercase());
            uppercase_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn snake_to_pascal(name: &str) -> String {
    let mut camel = snake_to_camel(name);
    if let Some(first) = camel.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    camel
}

fn is_optional_type(ty: &str) -> bool {
    let compact = ty.replace(' ', "");
    compact.starts_with("Option<")
        || compact.starts_with("std::option::Option<")
        || compact.starts_with("::std::option::Option<")
}

fn parse_type_params(generics: &str) -> Vec<String> {
    generics
        .trim_start_matches('<')
        .trim_end_matches('>')
        .split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() || part.starts_with('\'') || part.starts_with("const ") {
                return None;
            }
            let name = part.split([':', '=', ' ']).next().unwrap_or("").trim();
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn compile_error(message: &str) -> TokenStream {
    let mut stream = TokenStream::new();
    stream.extend([
        TokenTree::Ident(proc_macro::Ident::new(
            "compile_error",
            proc_macro::Span::call_site(),
        )),
        TokenTree::Punct(proc_macro::Punct::new('!', proc_macro::Spacing::Alone)),
        TokenTree::Group(proc_macro::Group::new(
            Delimiter::Parenthesis,
            TokenTree::Literal(proc_macro::Literal::string(message)).into(),
        )),
    ]);
    stream
}
