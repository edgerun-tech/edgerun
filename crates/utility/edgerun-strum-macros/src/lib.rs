use proc_macro::Delimiter;
use proc_macro::Group;
use proc_macro::TokenStream;
use proc_macro::TokenTree;

#[proc_macro_derive(Display, attributes(strum))]
pub fn display(input: TokenStream) -> TokenStream {
    expand_display(input).unwrap_or_else(|message| compile_error(&message))
}

#[proc_macro_derive(EnumIter, attributes(strum))]
pub fn enum_iter(input: TokenStream) -> TokenStream {
    expand_enum_iter(input).unwrap_or_else(|message| compile_error(&message))
}

#[derive(Clone, Debug)]
struct EnumDef {
    name: String,
    attrs: Vec<Group>,
    variants: Vec<Variant>,
}

#[derive(Clone, Debug)]
struct Variant {
    name: String,
    attrs: Vec<Group>,
    style: VariantStyle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VariantStyle {
    Unit,
    Tuple,
    Struct,
}

fn expand_display(input: TokenStream) -> Result<TokenStream, String> {
    let def = parse_enum(input)?;
    let rename_all = strum_rename_all(&def.attrs);
    let arms = def
        .variants
        .iter()
        .map(|variant| {
            let text = strum_serialize(&variant.attrs)
                .unwrap_or_else(|| rename_ident(&variant.name, rename_all.as_deref()));
            let pattern = match variant.style {
                VariantStyle::Unit => format!("Self::{}", variant.name),
                VariantStyle::Tuple => format!("Self::{}(..)", variant.name),
                VariantStyle::Struct => format!("Self::{} {{ .. }}", variant.name),
            };
            format!("{pattern} => f.write_str({text:?}),")
        })
        .collect::<String>();

    parse_quote(&format!(
        "impl ::core::fmt::Display for {} {{
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{
                match self {{ {arms} }}
            }}
        }}",
        def.name
    ))
}

fn expand_enum_iter(input: TokenStream) -> Result<TokenStream, String> {
    let def = parse_enum(input)?;
    let mut values = Vec::new();
    for variant in &def.variants {
        if variant.style != VariantStyle::Unit {
            return Err("EnumIter only supports unit variants in edgerun-strum-macros".to_string());
        }
        values.push(format!("{}::{}", def.name, variant.name));
    }
    let values = values.join(",");
    let iter_name = format!("{}Iter", def.name);
    parse_quote(&format!(
        "pub struct {iter_name} {{
            index: usize,
        }}

        impl ::core::iter::Iterator for {iter_name} {{
            type Item = {name};

            fn next(&mut self) -> ::core::option::Option<Self::Item> {{
                let values = [{values}];
                let value = values.get(self.index).copied();
                self.index += 1;
                value
            }}
        }}

        impl ::edgerun_strum::IntoEnumIterator for {name} {{
            type Iterator = {iter_name};

            fn iter() -> Self::Iterator {{
                {iter_name} {{ index: 0 }}
            }}
        }}",
        name = def.name,
    ))
}

fn parse_enum(input: TokenStream) -> Result<EnumDef, String> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let mut attrs = Vec::new();
    let mut idx = 0;
    while idx + 1 < tokens.len() {
        if is_pound_attr(&tokens, idx) {
            if let TokenTree::Group(group) = &tokens[idx + 1] {
                attrs.push(group.clone());
                idx += 2;
                continue;
            }
        }
        break;
    }

    while idx < tokens.len() {
        if token_text(&tokens[idx]) == "enum" {
            break;
        }
        idx += 1;
    }
    if idx >= tokens.len() {
        return Err("expected enum".to_string());
    }
    idx += 1;
    let name = tokens
        .get(idx)
        .map(token_text)
        .ok_or_else(|| "expected enum name".to_string())?;

    let body = tokens
        .iter()
        .skip(idx + 1)
        .find_map(|token| match token {
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => Some(group.clone()),
            _ => None,
        })
        .ok_or_else(|| "expected enum body".to_string())?;

    Ok(EnumDef {
        name,
        attrs,
        variants: parse_variants(body.stream()),
    })
}

fn parse_variants(stream: TokenStream) -> Vec<Variant> {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut variants = Vec::new();
    let mut attrs = Vec::new();
    let mut idx = 0;
    while idx < tokens.len() {
        if is_pound_attr(&tokens, idx) {
            if let TokenTree::Group(group) = &tokens[idx + 1] {
                attrs.push(group.clone());
                idx += 2;
                continue;
            }
        }

        let TokenTree::Ident(ident) = &tokens[idx] else {
            idx += 1;
            continue;
        };
        let name = ident.to_string();
        idx += 1;
        let mut style = VariantStyle::Unit;
        if let Some(TokenTree::Group(group)) = tokens.get(idx) {
            style = match group.delimiter() {
                Delimiter::Parenthesis => VariantStyle::Tuple,
                Delimiter::Brace => VariantStyle::Struct,
                _ => VariantStyle::Unit,
            };
            if style != VariantStyle::Unit {
                idx += 1;
            }
        }
        variants.push(Variant {
            name,
            attrs: core::mem::take(&mut attrs),
            style,
        });
        while idx < tokens.len() && token_text(&tokens[idx]) != "," {
            idx += 1;
        }
        if idx < tokens.len() {
            idx += 1;
        }
    }
    variants
}

fn is_pound_attr(tokens: &[TokenTree], idx: usize) -> bool {
    matches!(tokens.get(idx), Some(TokenTree::Punct(punct)) if punct.as_char() == '#')
        && matches!(tokens.get(idx + 1), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket)
}

fn strum_rename_all(attrs: &[Group]) -> Option<String> {
    attr_value(attrs, "strum", "serialize_all")
}

fn strum_serialize(attrs: &[Group]) -> Option<String> {
    attr_value(attrs, "strum", "serialize")
}

fn attr_value(attrs: &[Group], attr_name: &str, key: &str) -> Option<String> {
    for attr in attrs {
        let text = attr.stream().to_string();
        let Some(rest) = text.strip_prefix(attr_name) else {
            continue;
        };
        let Some(start) = rest.find('(') else {
            continue;
        };
        let Some(end) = rest.rfind(')') else {
            continue;
        };
        for part in split_top_level(&rest[start + 1..end]) {
            let mut pieces = part.splitn(2, '=');
            let Some(left) = pieces.next() else {
                continue;
            };
            if compact(left) == key {
                if let Some(right) = pieces.next() {
                    return Some(unquote(right.trim()));
                }
            }
        }
    }
    None
}

fn split_top_level(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in input.char_indices() {
        if in_string {
            escaped = ch == '\\' && !escaped;
            if ch == '"' && !escaped {
                in_string = false;
            }
            if ch != '\\' {
                escaped = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(input[start..idx].trim());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(input[start..].trim());
    parts
}

fn rename_ident(ident: &str, rename_all: Option<&str>) -> String {
    match rename_all {
        Some("lowercase") => split_words(ident).join("").to_ascii_lowercase(),
        Some("snake_case") => split_words(ident).join("_").to_ascii_lowercase(),
        Some("kebab-case") => split_words(ident).join("-").to_ascii_lowercase(),
        Some("camelCase") => {
            let mut words = split_words(ident);
            if words.is_empty() {
                return String::new();
            }
            let mut out = words.remove(0).to_ascii_lowercase();
            for word in words {
                let lower = word.to_ascii_lowercase();
                let mut chars = lower.chars();
                if let Some(first) = chars.next() {
                    out.push(first.to_ascii_uppercase());
                    out.extend(chars);
                }
            }
            out
        }
        _ => ident.to_string(),
    }
}

fn split_words(ident: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for ch in ident.trim_start_matches("r#").chars() {
        if ch == '_' || ch == '-' {
            if !current.is_empty() {
                words.push(core::mem::take(&mut current));
            }
            continue;
        }
        if ch.is_uppercase() && !current.is_empty() {
            words.push(core::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn compact(input: &str) -> String {
    input.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn unquote(input: &str) -> String {
    input.trim().trim_matches('"').to_string()
}

fn token_text(token: &TokenTree) -> String {
    token.to_string()
}

fn parse_quote(source: &str) -> Result<TokenStream, String> {
    source
        .parse()
        .map_err(|_| format!("failed to parse generated code: {source}"))
}

fn compile_error(message: &str) -> TokenStream {
    format!("compile_error!({message:?});")
        .parse()
        .unwrap_or_else(|_| TokenStream::new())
}
