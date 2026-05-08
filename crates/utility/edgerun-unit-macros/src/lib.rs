use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

#[proc_macro_attribute]
pub fn export(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return compile_error("edgerun_unit::export takes no arguments");
    }
    expand_export(item).unwrap_or_else(|message| compile_error(&message))
}

fn expand_export(item: TokenStream) -> Result<TokenStream, String> {
    let tokens: Vec<TokenTree> = item.clone().into_iter().collect();
    if has_no_mangle_attr(&tokens) {
        return Err("edgerun_unit::export owns #[no_mangle]; remove the manual attribute".into());
    }

    let fn_index = tokens
        .iter()
        .position(|token| is_ident(token, "fn"))
        .ok_or_else(|| "edgerun_unit::export expected a function".to_string())?;
    let name_index = fn_index
        .checked_add(1)
        .ok_or_else(|| "edgerun_unit::export expected a function name".to_string())?;
    let name = match tokens.get(name_index) {
        Some(TokenTree::Ident(ident)) => ident.to_string(),
        _ => return Err("edgerun_unit::export expected a function name".into()),
    };
    let args_index = name_index
        .checked_add(1)
        .ok_or_else(|| "edgerun_unit::export expected a parameter list".to_string())?;
    let args = match tokens.get(args_index) {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => group,
        Some(TokenTree::Punct(punct)) if punct.as_char() == '<' => {
            return Err("edgerun unit exports cannot be generic".into());
        }
        _ => return Err("edgerun_unit::export expected a parameter list".into()),
    };

    validate_prefix(&tokens[..fn_index])?;
    validate_params(args.stream())?;
    validate_return(&tokens[args_index + 1..])?;

    let source = item.to_string();
    rewrite_function(&source, &name)
        .parse()
        .map_err(|_| "edgerun_unit::export failed to generate function".to_string())
}

fn validate_prefix(tokens: &[TokenTree]) -> Result<(), String> {
    if tokens.iter().any(|token| is_ident(token, "const")) {
        return Err("edgerun unit exports cannot be const functions".into());
    }
    if tokens.iter().any(|token| is_ident(token, "async")) {
        return Err("edgerun unit exports cannot be async functions".into());
    }

    let mut saw_extern = false;
    let mut abi_is_c = false;
    for token in tokens {
        if is_ident(token, "extern") {
            saw_extern = true;
        }
        if let TokenTree::Literal(lit) = token {
            if lit.to_string() == "\"C\"" {
                abi_is_c = true;
            }
        }
    }
    if saw_extern && !abi_is_c {
        return Err("edgerun unit exports must use extern \"C\"".into());
    }
    Ok(())
}

fn validate_params(params: TokenStream) -> Result<(), String> {
    let mut current = Vec::new();
    for token in params {
        if matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',') {
            validate_param(&current)?;
            current.clear();
        } else {
            current.push(token);
        }
    }
    if !current.is_empty() {
        validate_param(&current)?;
    }
    Ok(())
}

fn validate_param(tokens: &[TokenTree]) -> Result<(), String> {
    if tokens.len() == 1 && is_ident(&tokens[0], "self") {
        return Err("edgerun unit exports cannot take self receivers".into());
    }
    let colon = tokens
        .iter()
        .position(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ':'))
        .ok_or_else(|| "edgerun unit export parameters must be named".to_string())?;
    let ty = tokens[colon + 1..]
        .iter()
        .map(ToString::to_string)
        .collect::<String>();
    if !is_wasm_scalar_type(&ty) {
        return Err("edgerun unit export parameters must be i32, i64, f32, or f64".into());
    }
    Ok(())
}

fn validate_return(tokens: &[TokenTree]) -> Result<(), String> {
    let Some(arrow) = tokens
        .iter()
        .position(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == '-'))
    else {
        return Ok(());
    };
    if !matches!(tokens.get(arrow + 1), Some(TokenTree::Punct(punct)) if punct.as_char() == '>')
    {
        return Ok(());
    }
    let ty = tokens[arrow + 2..]
        .iter()
        .take_while(|token| !matches!(token, TokenTree::Group(group) if group.delimiter() == Delimiter::Brace))
        .map(ToString::to_string)
        .collect::<String>();
    if !is_wasm_scalar_type(&ty) {
        return Err("edgerun unit export return values must be i32, i64, f32, or f64".into());
    }
    Ok(())
}

fn is_wasm_scalar_type(ty: &str) -> bool {
    matches!(ty.trim(), "i32" | "i64" | "f32" | "f64")
}

fn has_no_mangle_attr(tokens: &[TokenTree]) -> bool {
    tokens.windows(2).any(|pair| {
        matches!(&pair[0], TokenTree::Punct(punct) if punct.as_char() == '#')
            && matches!(&pair[1], TokenTree::Group(group) if group.delimiter() == Delimiter::Bracket && group.stream().to_string().contains("no_mangle"))
    })
}

fn rewrite_function(source: &str, name: &str) -> String {
    let fn_marker = format!("fn {name}");
    let Some(fn_pos) = source.find(&fn_marker) else {
        return source.to_string();
    };
    let before = source[..fn_pos].trim_end();
    let after = &source[fn_pos..];
    let unsafe_fn = before.ends_with("unsafe");
    let extern_fn = before.contains("extern \"C\"");
    let mut prefix = before
        .trim_end_matches("unsafe")
        .trim_end_matches("extern \"C\"")
        .trim_end_matches("pub")
        .trim_end()
        .to_string();
    if !prefix.is_empty() {
        prefix.push(' ');
    }
    prefix.push_str("#[no_mangle] pub ");
    if unsafe_fn {
        prefix.push_str("unsafe ");
    }
    if !extern_fn {
        prefix.push_str("extern \"C\" ");
    }
    prefix.push_str(after);
    prefix
}

fn is_ident(token: &TokenTree, value: &str) -> bool {
    matches!(token, TokenTree::Ident(ident) if ident.to_string() == value)
}

fn compile_error(message: &str) -> TokenStream {
    format!("compile_error!({message:?});")
        .parse()
        .expect("valid compile_error")
}
