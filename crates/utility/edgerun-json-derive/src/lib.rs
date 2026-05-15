use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

#[proc_macro_derive(ToJson, attributes(json, schemars))]
pub fn derive_to_json(input: TokenStream) -> TokenStream {
    match parse_item(input).and_then(|item| expand_to_json(&item)) {
        Ok(output) => output,
        Err(message) => compile_error(&message),
    }
}

#[proc_macro_derive(FromJson, attributes(json, schemars))]
pub fn derive_from_json(input: TokenStream) -> TokenStream {
    match parse_item(input).and_then(|item| expand_from_json(&item)) {
        Ok(output) => output,
        Err(message) => compile_error(&message),
    }
}

fn expand_to_json(item: &Item) -> Result<TokenStream, String> {
    match item.kind {
        ItemKind::Struct => expand_to_json_struct(item),
        ItemKind::Enum => expand_to_json_enum(item),
    }
}

fn expand_from_json(item: &Item) -> Result<TokenStream, String> {
    match item.kind {
        ItemKind::Struct => expand_from_json_struct(item),
        ItemKind::Enum => expand_from_json_enum(item),
    }
}

fn expand_to_json_struct(item: &Item) -> Result<TokenStream, String> {
    if item.fields.is_none() && item.tuple_fields.is_none() {
        return format!(
            "impl edgerun_json::ToJson for {} {{
                fn to_json(&self) -> edgerun_json::JsonValue {{
                    edgerun_json::JsonValue::Object(edgerun_json::Map::new())
                }}
            }}",
            item.name
        )
        .parse()
        .map_err(|_| "failed to generate unit struct ToJson impl".to_string());
    }
    if let Some(fields) = item.tuple_fields.as_ref() {
        if fields.len() == 1 {
            return format!(
                "impl edgerun_json::ToJson for {} {{
                    fn to_json(&self) -> edgerun_json::JsonValue {{
                        edgerun_json::ToJson::to_json(&self.0)
                    }}
                }}",
                item.name
            )
            .parse()
            .map_err(|_| "failed to generate tuple struct ToJson impl".to_string());
        }
        let values = (0..fields.len())
            .map(|idx| format!("edgerun_json::ToJson::to_json(&self.{idx})"))
            .collect::<Vec<_>>()
            .join(",");
        return format!(
            "impl edgerun_json::ToJson for {} {{
                fn to_json(&self) -> edgerun_json::JsonValue {{
                    edgerun_json::JsonValue::Array(vec![{values}])
                }}
            }}",
            item.name
        )
        .parse()
        .map_err(|_| "failed to generate tuple struct ToJson impl".to_string());
    }
    let fields = item
        .fields
        .as_ref()
        .ok_or_else(|| "ToJson only supports named structs".to_string())?;
    let container = ContainerAttrs::from_attrs(&item.attrs);
    let mut writes = String::new();

    for field in fields {
        let attrs = FieldAttrs::from_attrs(&field.attrs);
        if attrs.skip_serializing {
            continue;
        }
        let key = field_json_key(field, &container);
        let serialize_expr = attrs
            .serialize_with
            .as_ref()
            .map(|path| format!("{path}(&self.{})", field.name))
            .unwrap_or_else(|| format!("edgerun_json::ToJson::to_json(&self.{})", field.name));
        if attrs.skip_serializing_if.as_deref() == Some("Option::is_none")
            && is_option_type(&field.ty)
        {
            writes.push_str(&format!(
                "if self.{}.is_some() {{
                    object.push_field({key:?}, {serialize_expr});
                }}",
                field.name
            ));
        } else {
            writes.push_str(&format!("object.push_field({key:?}, {serialize_expr});"));
        }
    }

    format!(
        "impl edgerun_json::ToJson for {} {{
            fn to_json(&self) -> edgerun_json::JsonValue {{
                let mut object = edgerun_json::Map::new();
                {writes}
                edgerun_json::JsonValue::Object(object)
            }}
        }}",
        item.name
    )
    .parse()
    .map_err(|_| "failed to generate ToJson impl".to_string())
}

fn expand_from_json_struct(item: &Item) -> Result<TokenStream, String> {
    if item.fields.is_none() && item.tuple_fields.is_none() {
        return format!(
            "impl edgerun_json::FromJson for {} {{
                fn from_json(value: edgerun_json::JsonValue) -> core::result::Result<Self, edgerun_json::JsonValueError> {{
                    match value {{
                        edgerun_json::JsonValue::Object(_) | edgerun_json::JsonValue::Null => Ok(Self),
                        other => Err(edgerun_json::JsonValueError::WrongType(format!(\"expected object, found {{}}\", other.variant_name()))),
                    }}
                }}
            }}",
            item.name
        )
        .parse()
        .map_err(|_| "failed to generate unit struct FromJson impl".to_string());
    }
    if let Some(fields) = item.tuple_fields.as_ref() {
        if fields.len() == 1 {
            return format!(
                "impl edgerun_json::FromJson for {} {{
                    fn from_json(value: edgerun_json::JsonValue) -> core::result::Result<Self, edgerun_json::JsonValueError> {{
                        Ok(Self(edgerun_json::FromJson::from_json(value)?))
                    }}
                }}",
                item.name
            )
            .parse()
            .map_err(|_| "failed to generate tuple struct FromJson impl".to_string());
        }
        let reads = fields
            .iter()
            .enumerate()
            .map(|(idx, _)| {
                format!("edgerun_json::FromJson::from_json(values.get({idx}).cloned().ok_or_else(|| edgerun_json::JsonValueError::WrongType(edgerun_json::__json_error_message(\"missing tuple field\")))?)?")
            })
            .collect::<Vec<_>>()
            .join(",");
        return format!(
            "impl edgerun_json::FromJson for {} {{
                fn from_json(value: edgerun_json::JsonValue) -> core::result::Result<Self, edgerun_json::JsonValueError> {{
                    let values: Vec<edgerun_json::JsonValue> = value.try_into()?;
                    if values.len() != {} {{
                        return Err(edgerun_json::JsonValueError::WrongType(edgerun_json::__json_error_message(\"wrong tuple field count\")));
                    }}
                    Ok(Self({reads}))
                }}
            }}",
            item.name,
            fields.len()
        )
        .parse()
        .map_err(|_| "failed to generate tuple struct FromJson impl".to_string());
    }
    let fields = item
        .fields
        .as_ref()
        .ok_or_else(|| "FromJson only supports named structs".to_string())?;
    let container = ContainerAttrs::from_attrs(&item.attrs);
    let mut reads = String::new();

    for field in fields {
        let attrs = FieldAttrs::from_attrs(&field.attrs);
        let value = field_read_expr(field, &attrs, &container);
        reads.push_str(&format!("{}: {value},", field.name));
    }
    let reject_unknown = if container.deny_unknown_fields {
        "if !object.into_vec().is_empty() {
            return Err(edgerun_json::JsonValueError::WrongType(
                edgerun_json::__json_error_message(\"unknown JSON field\")
            ));
        }"
    } else {
        ""
    };

    format!(
        "impl edgerun_json::FromJson for {} {{
            fn from_json(value: edgerun_json::JsonValue) -> core::result::Result<Self, edgerun_json::JsonValueError> {{
                let mut object = value.into_object(core::any::type_name::<Self>())?;
                let parsed = Self {{ {reads} }};
                {reject_unknown}
                Ok(parsed)
            }}
        }}",
        item.name
    )
    .parse()
    .map_err(|_| "failed to generate FromJson impl".to_string())
}

fn expand_to_json_enum(item: &Item) -> Result<TokenStream, String> {
    let variants = item
        .variants
        .as_ref()
        .ok_or_else(|| "ToJson expected enum variants".to_string())?;
    let container = ContainerAttrs::from_attrs(&item.attrs);

    if let Some(tag) = container.tag.as_ref() {
        let mut arms = String::new();
        for variant in variants {
            let value = variant_json_value(variant, &container);
            match &variant.fields {
                Some(fields) => {
                    let field_names = fields
                        .iter()
                        .map(|field| field.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    let mut writes = String::new();
                    for field in fields {
                        let attrs = FieldAttrs::from_attrs(&field.attrs);
                        if attrs.skip_serializing {
                            continue;
                        }
                        let key = field_json_key(field, &container);
                        let serialize_expr = attrs
                            .serialize_with
                            .as_ref()
                            .map(|path| format!("{path}({})", field.name))
                            .unwrap_or_else(|| {
                                format!("edgerun_json::ToJson::to_json({})", field.name)
                            });
                        if attrs.skip_serializing_if.as_deref() == Some("Option::is_none")
                            && is_option_type(&field.ty)
                        {
                            writes.push_str(&format!(
                                "if {field}.is_some() {{
                                    object.push_field({key:?}, {serialize_expr});
                                }}",
                                field = field.name
                            ));
                        } else {
                            writes.push_str(&format!(
                                "object.push_field({key:?}, {serialize_expr});"
                            ));
                        }
                    }
                    arms.push_str(&format!(
                        "Self::{} {{ {field_names} }} => {{
                            let mut object = edgerun_json::Map::new();
                            object.push_field({tag:?}, edgerun_json::JsonValue::String({value:?}.to_string()));
                            {writes}
                            edgerun_json::JsonValue::Object(object)
                        }},",
                        variant.name
                    ));
                }
                None if variant.tuple_fields.len() == 1 => {
                    arms.push_str(&format!(
                        "Self::{}(value) => {{
                            let mut object = match edgerun_json::ToJson::to_json(value) {{
                                edgerun_json::JsonValue::Object(object) => object,
                                value => {{
                                    let mut object = edgerun_json::Map::new();
                                    object.push_field(\"value\", value);
                                    object
                                }}
                            }};
                            object.push_field({tag:?}, edgerun_json::JsonValue::String({value:?}.to_string()));
                            edgerun_json::JsonValue::Object(object)
                        }},",
                        variant.name
                    ));
                }
                None => {
                    arms.push_str(&format!(
                        "Self::{} => {{
                            let mut object = edgerun_json::Map::new();
                            object.push_field({tag:?}, edgerun_json::JsonValue::String({value:?}.to_string()));
                            edgerun_json::JsonValue::Object(object)
                        }},",
                        variant.name
                    ));
                }
            }
        }
        return format!(
            "impl edgerun_json::ToJson for {} {{
                fn to_json(&self) -> edgerun_json::JsonValue {{
                    match self {{ {arms} }}
                }}
            }}",
            item.name
        )
        .parse()
        .map_err(|_| "failed to generate enum ToJson impl".to_string());
    }

    if container.untagged {
        let mut arms = String::new();
        for variant in variants {
            match &variant.fields {
                Some(fields) => {
                    let field_names = fields
                        .iter()
                        .map(|field| field.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    let mut writes = String::new();
                    for field in fields {
                        let attrs = FieldAttrs::from_attrs(&field.attrs);
                        if attrs.skip_serializing {
                            continue;
                        }
                        let key = field_json_key(field, &container);
                        let serialize_expr = attrs
                            .serialize_with
                            .as_ref()
                            .map(|path| format!("{path}({})", field.name))
                            .unwrap_or_else(|| {
                                format!("edgerun_json::ToJson::to_json({})", field.name)
                            });
                        if attrs.skip_serializing_if.as_deref() == Some("Option::is_none")
                            && is_option_type(&field.ty)
                        {
                            writes.push_str(&format!(
                                "if {field}.is_some() {{
                                    object.push_field({key:?}, {serialize_expr});
                                }}",
                                field = field.name
                            ));
                        } else {
                            writes.push_str(&format!(
                                "object.push_field({key:?}, {serialize_expr});",
                            ));
                        }
                    }
                    arms.push_str(&format!(
                        "Self::{} {{ {field_names} }} => {{
                            let mut object = edgerun_json::Map::new();
                            {writes}
                            edgerun_json::JsonValue::Object(object)
                        }},",
                        variant.name
                    ));
                }
                None if variant.tuple_fields.len() == 1 => {
                    arms.push_str(&format!(
                        "Self::{}(value) => edgerun_json::ToJson::to_json(value),",
                        variant.name
                    ));
                }
                None => {
                    arms.push_str(&format!(
                        "Self::{} => edgerun_json::JsonValue::Null,",
                        variant.name
                    ));
                }
            }
        }
        return format!(
            "impl edgerun_json::ToJson for {} {{
                fn to_json(&self) -> edgerun_json::JsonValue {{
                    match self {{ {arms} }}
                }}
            }}",
            item.name
        )
        .parse()
        .map_err(|_| "failed to generate untagged enum ToJson impl".to_string());
    }

    let mut arms = String::new();
    for variant in variants {
        let value = variant_json_value(variant, &container);
        if variant.fields.is_some() {
            arms.push_str(&format!("Self::{} {{ .. }} => {value:?},", variant.name));
        } else if variant.tuple_fields.len() == 1 {
            arms.push_str(&format!("Self::{}(_) => {value:?},", variant.name));
        } else {
            arms.push_str(&format!("Self::{} => {value:?},", variant.name));
        }
    }

    format!(
        "impl edgerun_json::ToJson for {} {{
            fn to_json(&self) -> edgerun_json::JsonValue {{
                edgerun_json::JsonValue::String(match self {{ {arms} }}.to_string())
            }}
        }}",
        item.name
    )
    .parse()
    .map_err(|_| "failed to generate enum ToJson impl".to_string())
}

fn expand_from_json_enum(item: &Item) -> Result<TokenStream, String> {
    let variants = item
        .variants
        .as_ref()
        .ok_or_else(|| "FromJson expected enum variants".to_string())?;
    let container = ContainerAttrs::from_attrs(&item.attrs);

    if let Some(tag) = container.tag.as_ref() {
        let mut arms = String::new();
        for variant in variants {
            let value = variant_json_value(variant, &container);
            match &variant.fields {
                Some(fields) => {
                    let mut reads = String::new();
                    for field in fields {
                        let attrs = FieldAttrs::from_attrs(&field.attrs);
                        let value = field_read_expr(field, &attrs, &container);
                        reads.push_str(&format!("{}: {value},", field.name));
                    }
                    arms.push_str(&format!(
                        "{value:?} => Ok(Self::{} {{ {reads} }}),",
                        variant.name
                    ));
                }
                None if variant.tuple_fields.len() == 1 => {
                    let ty = &variant.tuple_fields[0];
                    arms.push_str(&format!(
                        "{value:?} => Ok(Self::{}(<{ty} as edgerun_json::FromJson>::from_json(edgerun_json::JsonValue::Object(object))?)),",
                        variant.name
                    ));
                }
                None => arms.push_str(&format!("{value:?} => Ok(Self::{}),", variant.name)),
            }
        }

        return format!(
            "impl edgerun_json::FromJson for {} {{
                fn from_json(value: edgerun_json::JsonValue) -> core::result::Result<Self, edgerun_json::JsonValueError> {{
                    let mut object = value.into_object(core::any::type_name::<Self>())?;
                    let tag = <String as edgerun_json::FromJson>::from_json(
                        object.remove({tag:?}).ok_or_else(|| {{
                            edgerun_json::JsonValueError::WrongType(
                                edgerun_json::__json_error_message(\"missing JSON tag field\")
                            )
                        }})?
                    )?;
                    match tag.as_str() {{
                        {arms}
                        _ => Err(edgerun_json::JsonValueError::WrongType(
                            edgerun_json::__json_error_message(\"unknown enum tag\")
                        )),
                    }}
                }}
            }}",
            item.name
        )
        .parse()
        .map_err(|_| "failed to generate enum FromJson impl".to_string());
    }

    if container.untagged {
        let mut attempts = String::new();
        for variant in variants {
            match &variant.fields {
                Some(fields) => {
                    let mut reads = String::new();
                    for field in fields {
                        let attrs = FieldAttrs::from_attrs(&field.attrs);
                        let value = field_read_expr(field, &attrs, &container);
                        reads.push_str(&format!("{}: {value},", field.name));
                    }
                    attempts.push_str(&format!(
                        "if let edgerun_json::JsonValue::Object(mut object) = value.clone() {{
                            if let Ok(parsed) = (|| -> core::result::Result<Self, edgerun_json::JsonValueError> {{
                                Ok(Self::{} {{ {reads} }})
                            }})() {{
                                return Ok(parsed);
                            }}
                        }}",
                        variant.name
                    ));
                }
                None if variant.tuple_fields.len() == 1 => {
                    let ty = &variant.tuple_fields[0];
                    attempts.push_str(&format!(
                        "if let Ok(parsed) = <{ty} as edgerun_json::FromJson>::from_json(value.clone()) {{
                            return Ok(Self::{}(parsed));
                        }}",
                        variant.name
                    ));
                }
                None => {
                    attempts.push_str(&format!(
                        "if matches!(value, edgerun_json::JsonValue::Null) {{
                            return Ok(Self::{});
                        }}",
                        variant.name
                    ));
                }
            }
        }
        return format!(
            "impl edgerun_json::FromJson for {} {{
                fn from_json(value: edgerun_json::JsonValue) -> core::result::Result<Self, edgerun_json::JsonValueError> {{
                    {attempts}
                    Err(edgerun_json::JsonValueError::WrongType(
                        edgerun_json::__json_error_message(\"data did not match any untagged enum variant\")
                    ))
                }}
            }}",
            item.name
        )
        .parse()
        .map_err(|_| "failed to generate untagged enum FromJson impl".to_string());
    }

    let mut arms = String::new();
    for variant in variants {
        if variant.fields.is_some() || !variant.tuple_fields.is_empty() {
            continue;
        }
        let value = variant_json_value(variant, &container);
        arms.push_str(&format!("{value:?} => Ok(Self::{}),", variant.name));
    }

    format!(
        "impl edgerun_json::FromJson for {} {{
            fn from_json(value: edgerun_json::JsonValue) -> core::result::Result<Self, edgerun_json::JsonValueError> {{
                let value = <String as edgerun_json::FromJson>::from_json(value)?;
                match value.as_str() {{
                    {arms}
                    _ => Err(edgerun_json::JsonValueError::WrongType(
                        edgerun_json::__json_error_message(\"unknown enum value\")
                    )),
                }}
            }}
        }}",
        item.name
    )
    .parse()
    .map_err(|_| "failed to generate enum FromJson impl".to_string())
}

fn field_read_expr(field: &Field, attrs: &FieldAttrs, container: &ContainerAttrs) -> String {
    if attrs.skip_deserializing {
        return "Default::default()".to_string();
    }
    let keys = field_json_keys(field, container)
        .into_iter()
        .map(|key| format!("{key:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    if let Some(deserialize_with) = attrs.deserialize_with.as_ref() {
        if let Some(default_fn) = attrs.default_fn.as_ref() {
            format!(
                "match object.remove_any(&[{keys}]) {{
                    Some(value) => {deserialize_with}(value)?,
                    None => {default_fn}(),
                }}"
            )
        } else if attrs.default {
            format!(
                "match object.remove_any(&[{keys}]) {{
                    Some(value) => {deserialize_with}(value)?,
                    None => Default::default(),
                }}"
            )
        } else if is_option_type(&field.ty) {
            format!(
                "match object.remove_any(&[{keys}]) {{
                    Some(value) => {deserialize_with}(value)?,
                    None => None,
                }}"
            )
        } else {
            format!(
                "{deserialize_with}(object.remove_any(&[{keys}]).ok_or_else(|| edgerun_json::JsonValueError::WrongType(format!(\"missing required field `{{}}`\", [{keys}][0])))?)?"
            )
        }
    } else if let Some(default_fn) = attrs.default_fn.as_ref() {
        format!("object.take_optional_any(&[{keys}])?.unwrap_or_else({default_fn})")
    } else if attrs.default {
        format!("object.take_optional_any(&[{keys}])?.unwrap_or_default()")
    } else if is_option_type(&field.ty) {
        format!("object.take_optional_any(&[{keys}])?")
    } else {
        format!("object.take_required_any(&[{keys}])?")
    }
}

fn parse_item(input: TokenStream) -> Result<Item, String> {
    let mut tokens = input.into_iter().peekable();
    let mut attrs = Vec::new();

    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == '#' => {
                if let Some(TokenTree::Group(group)) = tokens.next() {
                    attrs.push(group);
                }
            }
            TokenTree::Ident(ident) if ident.to_string() == "struct" => {
                let name = next_ident(&mut tokens)?;
                let mut fields = None;
                let mut tuple_fields = None;
                for token in tokens.by_ref() {
                    if let TokenTree::Group(group) = token {
                        match group.delimiter() {
                            Delimiter::Brace => {
                                fields = Some(parse_named_fields(group.stream())?);
                                break;
                            }
                            Delimiter::Parenthesis => {
                                tuple_fields = Some(parse_unnamed_fields(group.stream()));
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                return Ok(Item {
                    kind: ItemKind::Struct,
                    name,
                    attrs,
                    fields,
                    tuple_fields,
                    variants: None,
                });
            }
            TokenTree::Ident(ident) if ident.to_string() == "enum" => {
                let name = next_ident(&mut tokens)?;
                let body = tokens.find_map(named_body_group);
                let variants = body
                    .map(|group| parse_variants(group.stream()))
                    .transpose()?;
                return Ok(Item {
                    kind: ItemKind::Enum,
                    name,
                    attrs,
                    fields: None,
                    tuple_fields: None,
                    variants,
                });
            }
            _ => {}
        }
    }

    Err("edgerun-json derives only support structs and enums".to_string())
}

fn named_body_group(token: TokenTree) -> Option<Group> {
    match token {
        TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => Some(group),
        _ => None,
    }
}

fn parse_unnamed_fields(input: TokenStream) -> Vec<String> {
    split_top_level(input, ',')
        .into_iter()
        .filter(|segment| !segment.is_empty())
        .map(tokens_to_string)
        .collect()
}

fn next_ident<I>(tokens: &mut I) -> Result<String, String>
where
    I: Iterator<Item = TokenTree>,
{
    for token in tokens {
        if let TokenTree::Ident(ident) = token {
            return Ok(ident.to_string());
        }
    }
    Err("expected item name".to_string())
}

fn parse_named_fields(input: TokenStream) -> Result<Vec<Field>, String> {
    let mut fields = Vec::new();
    let mut attrs = Vec::new();

    for segment in split_top_level(input, ',') {
        if segment.is_empty() {
            continue;
        }
        let mut tokens = segment.into_iter().peekable();
        while matches!(tokens.peek(), Some(TokenTree::Punct(p)) if p.as_char() == '#') {
            tokens.next();
            if let Some(TokenTree::Group(group)) = tokens.next() {
                attrs.push(group);
            }
        }
        let mut name = None;
        while let Some(token) = tokens.next() {
            match token {
                TokenTree::Ident(ident) if ident.to_string() == "pub" => continue,
                TokenTree::Ident(ident) => {
                    name = Some(ident.to_string());
                    break;
                }
                _ => {}
            }
        }
        let Some(name) = name else {
            continue;
        };
        match tokens.next() {
            Some(TokenTree::Punct(punct)) if punct.as_char() == ':' => {}
            _ => continue,
        }
        let ty = tokens_to_string(tokens.collect());
        fields.push(Field {
            name,
            ty,
            attrs: core::mem::take(&mut attrs),
        });
    }

    Ok(fields)
}

fn parse_variants(input: TokenStream) -> Result<Vec<Variant>, String> {
    let mut variants = Vec::new();
    let mut pending_attrs = Vec::new();

    for segment in split_top_level(input, ',') {
        if segment.is_empty() {
            continue;
        }
        let mut tokens = segment.into_iter().peekable();
        while matches!(tokens.peek(), Some(TokenTree::Punct(p)) if p.as_char() == '#') {
            tokens.next();
            if let Some(TokenTree::Group(group)) = tokens.next() {
                pending_attrs.push(group);
            }
        }
        let Some(TokenTree::Ident(ident)) = tokens.next() else {
            continue;
        };
        let name = ident.to_string();
        let mut fields = None;
        if let Some(TokenTree::Group(group)) = tokens.next() {
            match group.delimiter() {
                Delimiter::Brace => {
                    fields = Some(parse_named_fields(group.stream())?);
                }
                Delimiter::Parenthesis => {
                    variants.push(Variant {
                        name,
                        attrs: core::mem::take(&mut pending_attrs),
                        fields: None,
                        tuple_fields: parse_unnamed_fields(group.stream()),
                    });
                    continue;
                }
                _ => {}
            }
        }
        variants.push(Variant {
            name,
            attrs: core::mem::take(&mut pending_attrs),
            fields,
            tuple_fields: Vec::new(),
        });
    }

    Ok(variants)
}

fn split_top_level(input: TokenStream, delimiter: char) -> Vec<Vec<TokenTree>> {
    let mut out = Vec::new();
    let mut current = Vec::new();
    let mut angle_depth = 0i32;

    for token in input {
        match &token {
            TokenTree::Punct(punct) if punct.as_char() == '<' => angle_depth += 1,
            TokenTree::Punct(punct) if punct.as_char() == '>' && angle_depth > 0 => {
                angle_depth -= 1
            }
            TokenTree::Punct(punct) if punct.as_char() == delimiter && angle_depth == 0 => {
                out.push(core::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(token);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn tokens_to_string(tokens: Vec<TokenTree>) -> String {
    let mut out = String::new();
    for token in tokens {
        out.push_str(&token.to_string());
        out.push(' ');
    }
    out.trim().to_string()
}

fn field_json_key(field: &Field, container: &ContainerAttrs) -> String {
    FieldAttrs::from_attrs(&field.attrs)
        .rename
        .unwrap_or_else(|| {
            apply_rename_all(&json_ident(&field.name), container.rename_all.as_deref())
        })
}

fn field_json_keys(field: &Field, container: &ContainerAttrs) -> Vec<String> {
    let attrs = FieldAttrs::from_attrs(&field.attrs);
    let mut keys = Vec::with_capacity(attrs.aliases.len() + 1);
    keys.push(attrs.rename.unwrap_or_else(|| {
        apply_rename_all(&json_ident(&field.name), container.rename_all.as_deref())
    }));
    keys.extend(attrs.aliases);
    keys
}

fn variant_json_value(variant: &Variant, container: &ContainerAttrs) -> String {
    FieldAttrs::from_attrs(&variant.attrs)
        .rename
        .unwrap_or_else(|| apply_rename_all(&variant.name, container.rename_all.as_deref()))
}

fn is_option_type(ty: &str) -> bool {
    compact_type(ty)
        .rsplit("::")
        .next()
        .is_some_and(|ty| ty.starts_with("Option<"))
}

fn compact_type(ty: &str) -> String {
    ty.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn json_ident(ident: &str) -> String {
    ident.strip_prefix("r#").unwrap_or(ident).to_string()
}

fn to_snake_case(value: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if ch.is_uppercase() {
            if idx > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn to_kebab_case(value: &str) -> String {
    to_snake_case(value).replace('_', "-")
}

fn to_camel_case(value: &str) -> String {
    let mut out = String::new();
    let mut upper_next = false;
    for ch in value.chars() {
        if ch == '_' || ch == '-' {
            upper_next = true;
        } else if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn apply_rename_all(value: &str, rename_all: Option<&str>) -> String {
    match rename_all {
        Some("snake_case") => to_snake_case(value),
        Some("kebab-case") => to_kebab_case(value),
        Some("camelCase") => to_camel_case(&to_snake_case(value)),
        Some("lowercase") => value.to_ascii_lowercase(),
        Some("UPPERCASE") => value.to_ascii_uppercase(),
        _ => value.to_string(),
    }
}

#[derive(Default)]
struct ContainerAttrs {
    rename_all: Option<String>,
    tag: Option<String>,
    deny_unknown_fields: bool,
    untagged: bool,
}

impl ContainerAttrs {
    fn from_attrs(attrs: &[Group]) -> Self {
        let mut out = Self::default();
        for attr in attrs {
            if let Some(content) = attr_content(attr) {
                let pairs = parse_attr_pairs(content);
                if let Some(rename_all) = pairs.value("rename_all") {
                    out.rename_all = Some(unquote_string(rename_all));
                }
                if let Some(tag) = pairs.value("tag") {
                    out.tag = Some(unquote_string(tag));
                }
                if pairs.has_flag("deny_unknown_fields") {
                    out.deny_unknown_fields = true;
                }
                if pairs.has_flag("untagged") {
                    out.untagged = true;
                }
            }
        }
        out
    }
}

#[derive(Default)]
struct FieldAttrs {
    rename: Option<String>,
    aliases: Vec<String>,
    default: bool,
    default_fn: Option<String>,
    skip_serializing_if: Option<String>,
    serialize_with: Option<String>,
    deserialize_with: Option<String>,
    skip_serializing: bool,
    skip_deserializing: bool,
}

impl FieldAttrs {
    fn from_attrs(attrs: &[Group]) -> Self {
        let mut out = Self::default();
        for attr in attrs {
            if let Some(content) = attr_content(attr) {
                let pairs = parse_attr_pairs(content);
                if let Some(rename) = pairs.value("rename") {
                    out.rename = Some(unquote_string(rename));
                }
                for alias in pairs.values("alias") {
                    out.aliases.push(unquote_string(alias));
                }
                if pairs.has_flag("default") {
                    out.default = true;
                }
                if let Some(default_fn) = pairs.value("default") {
                    out.default_fn = Some(unquote_string(default_fn));
                }
                if let Some(skip) = pairs.value("skip_serializing_if") {
                    out.skip_serializing_if = Some(unquote_string(skip));
                }
                if let Some(path) = pairs.value("serialize_with") {
                    out.serialize_with = Some(unquote_string(path));
                }
                if let Some(path) = pairs.value("deserialize_with") {
                    out.deserialize_with = Some(unquote_string(path));
                }
                out.skip_serializing |=
                    pairs.has_flag("skip_serializing") || pairs.has_flag("skip");
                out.skip_deserializing |=
                    pairs.has_flag("skip_deserializing") || pairs.has_flag("skip");
            }
        }
        out
    }
}

fn attr_content(attr: &Group) -> Option<TokenStream> {
    if attr.delimiter() != Delimiter::Bracket {
        return None;
    }
    let mut tokens = attr.stream().into_iter();
    match tokens.next() {
        Some(TokenTree::Ident(ident))
            if ident.to_string() == "json" || ident.to_string() == "schemars" =>
        {
            match tokens.next() {
                Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
                    Some(group.stream())
                }
                _ => None,
            }
        }
        _ => None,
    }
}

struct AttrPairs {
    pairs: Vec<(String, Option<String>)>,
}

impl AttrPairs {
    fn has_flag(&self, key: &str) -> bool {
        self.pairs
            .iter()
            .any(|(name, value)| name == key && value.is_none())
    }

    fn value(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find_map(|(name, value)| (name == key).then_some(value.as_deref()).flatten())
    }

    fn values<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a str> {
        self.pairs
            .iter()
            .filter_map(move |(name, value)| (name == key).then_some(value.as_deref()).flatten())
    }
}

fn parse_attr_pairs(input: TokenStream) -> AttrPairs {
    let mut tokens = input.into_iter().peekable();
    let mut pairs = Vec::new();

    while let Some(token) = tokens.next() {
        let TokenTree::Ident(ident) = token else {
            continue;
        };
        let key = ident.to_string();
        let value = if matches!(tokens.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == '=')
        {
            tokens.next();
            let mut value = Vec::new();
            while let Some(next) = tokens.peek() {
                if matches!(next, TokenTree::Punct(punct) if punct.as_char() == ',') {
                    break;
                }
                value.push(tokens.next().unwrap());
            }
            Some(tokens_to_attr_value(value))
        } else {
            None
        };
        pairs.push((key, value));
        if matches!(tokens.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == ',') {
            tokens.next();
        }
    }

    AttrPairs { pairs }
}

fn tokens_to_attr_value(tokens: Vec<TokenTree>) -> String {
    let mut out = String::new();
    for token in tokens {
        out.push_str(&token.to_string());
    }
    out
}

fn unquote_string(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
        .to_string()
}

fn compile_error(message: &str) -> TokenStream {
    format!("compile_error!({message:?});")
        .parse()
        .unwrap_or_else(|_| TokenStream::new())
}

struct Item {
    kind: ItemKind,
    name: String,
    attrs: Vec<Group>,
    fields: Option<Vec<Field>>,
    tuple_fields: Option<Vec<String>>,
    variants: Option<Vec<Variant>>,
}

enum ItemKind {
    Struct,
    Enum,
}

struct Field {
    name: String,
    ty: String,
    attrs: Vec<Group>,
}

struct Variant {
    name: String,
    attrs: Vec<Group>,
    fields: Option<Vec<Field>>,
    tuple_fields: Vec<String>,
}
