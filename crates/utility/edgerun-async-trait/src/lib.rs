use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn async_trait(_attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_async_trait(item).unwrap_or_else(|message| compile_error(&message))
}

fn expand_async_trait(item: TokenStream) -> Result<TokenStream, String> {
    let source = item.to_string();
    let Some(open) = source.find('{') else {
        return Ok(source
            .parse()
            .map_err(|_| "failed to parse item".to_string())?);
    };
    let close = find_matching(&source, open, '{', '}')
        .ok_or_else(|| "async_trait expected matching item body".to_string())?;
    let header = &source[..open];
    let body = &source[open + 1..close];
    let trailer = &source[close + 1..];
    let rewritten = rewrite_body(body)?;

    format!("{header}{{{rewritten}}}{trailer}")
        .parse()
        .map_err(|_| "failed to generate async_trait expansion".to_string())
}

fn rewrite_body(body: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut cursor = 0usize;

    while let Some(relative) = body[cursor..].find("async fn") {
        let start = cursor + relative;
        if !has_word_boundary(body, start, "async fn".len()) {
            out.push_str(&body[cursor..start + "async fn".len()]);
            cursor = start + "async fn".len();
            continue;
        }
        out.push_str(&body[cursor..start]);
        let (replacement, next) = rewrite_method(body, start)?;
        out.push_str(&replacement);
        cursor = next;
    }

    out.push_str(&body[cursor..]);
    Ok(out)
}

fn rewrite_method(source: &str, start: usize) -> Result<(String, usize), String> {
    let fn_start = start + "async ".len();
    let args_open = source[fn_start..]
        .find('(')
        .map(|idx| fn_start + idx)
        .ok_or_else(|| "async_trait expected method argument list".to_string())?;
    let args_close = find_matching(source, args_open, '(', ')')
        .ok_or_else(|| "async_trait expected matching argument list".to_string())?;

    let fn_prefix = source[fn_start..args_open].trim_end();
    let args = rewrite_args(&source[args_open + 1..args_close]);
    let mut idx = skip_ws(source, args_close + 1);
    let return_type;

    if source[idx..].starts_with("->") {
        idx = skip_ws(source, idx + 2);
        let ret_start = idx;
        idx = scan_until_method_end(source, idx);
        return_type = source[ret_start..idx].trim().to_string();
    } else {
        return_type = "()".to_string();
    }

    idx = skip_ws(source, idx);
    let mut where_clause = String::new();
    if source[idx..].starts_with("where") && has_word_boundary(source, idx, "where".len()) {
        let where_start = idx;
        idx = scan_until_method_end(source, idx);
        where_clause = source[where_start..idx].trim().to_string();
        idx = skip_ws(source, idx);
    }

    let future = format!(
        "::core::pin::Pin<Box<dyn ::core::future::Future<Output = {return_type}> + Send + 'async_trait>>"
    );
    let where_clause = merge_where_clause(&where_clause);

    if source[idx..].starts_with(';') {
        let replacement = format!("{fn_prefix}<'async_trait>({args}) -> {future} {where_clause};");
        return Ok((replacement, idx + 1));
    }

    if !source[idx..].starts_with('{') {
        return Err("async_trait expected method body or semicolon".to_string());
    }
    let body_close = find_matching(source, idx, '{', '}')
        .ok_or_else(|| "async_trait expected matching method body".to_string())?;
    let body = &source[idx..=body_close];
    let replacement = format!(
        "{fn_prefix}<'async_trait>({args}) -> {future} {where_clause} {{
            Box::pin(async move {body})
        }}"
    );
    Ok((replacement, body_close + 1))
}

fn rewrite_args(args: &str) -> String {
    split_top_level(args, ',')
        .into_iter()
        .map(|arg| rewrite_arg_lifetime(arg.trim()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn rewrite_arg_lifetime(arg: &str) -> String {
    if arg == "&self" {
        return "&'async_trait self".to_string();
    }
    if arg == "&mut self" {
        return "&'async_trait mut self".to_string();
    }
    if let Some((name, ty)) = arg.split_once(':') {
        return format!("{name}: {}", rewrite_type_refs(ty.trim()));
    }
    arg.to_string()
}

fn rewrite_type_refs(ty: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = ty.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '&' {
            out.push('&');
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                out.push(chars[i]);
                i += 1;
            }
            if i < chars.len() && chars[i] == '\'' {
                continue;
            }
            out.push_str("'async_trait ");
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn merge_where_clause(existing: &str) -> String {
    if existing.is_empty() {
        "where Self: 'async_trait".to_string()
    } else {
        format!("{existing}, Self: 'async_trait")
    }
}

fn scan_until_method_end(source: &str, mut idx: usize) -> usize {
    let mut angle = 0i32;
    let mut paren = 0i32;
    let mut bracket = 0i32;
    while idx < source.len() {
        let ch = source[idx..].chars().next().unwrap();
        match ch {
            '<' => angle += 1,
            '>' if angle > 0 => angle -= 1,
            '(' => paren += 1,
            ')' if paren > 0 => paren -= 1,
            '[' => bracket += 1,
            ']' if bracket > 0 => bracket -= 1,
            '{' | ';' if angle == 0 && paren == 0 && bracket == 0 => break,
            _ => {}
        }
        idx += ch.len_utf8();
    }
    idx
}

fn split_top_level(input: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut angle = 0i32;
    let mut paren = 0i32;
    let mut bracket = 0i32;
    for (idx, ch) in input.char_indices() {
        match ch {
            '<' => angle += 1,
            '>' if angle > 0 => angle -= 1,
            '(' => paren += 1,
            ')' if paren > 0 => paren -= 1,
            '[' => bracket += 1,
            ']' if bracket > 0 => bracket -= 1,
            c if c == delimiter && angle == 0 && paren == 0 && bracket == 0 => {
                parts.push(input[start..idx].trim());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    let tail = input[start..].trim();
    if !tail.is_empty() {
        parts.push(tail);
    }
    parts
}

fn find_matching(source: &str, open: usize, left: char, right: char) -> Option<usize> {
    let mut depth = 0i32;
    for (idx, ch) in source[open..].char_indices() {
        if ch == left {
            depth += 1;
        } else if ch == right {
            depth -= 1;
            if depth == 0 {
                return Some(open + idx);
            }
        }
    }
    None
}

fn skip_ws(source: &str, mut idx: usize) -> usize {
    while idx < source.len() {
        let ch = source[idx..].chars().next().unwrap();
        if !ch.is_whitespace() {
            break;
        }
        idx += ch.len_utf8();
    }
    idx
}

fn has_word_boundary(source: &str, start: usize, len: usize) -> bool {
    let before = source[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| !ch.is_alphanumeric() && ch != '_');
    let after_idx = start + len;
    let after = source[after_idx..]
        .chars()
        .next()
        .is_none_or(|ch| !ch.is_alphanumeric() && ch != '_');
    before && after
}

fn compile_error(message: &str) -> TokenStream {
    format!("compile_error!({message:?});")
        .parse()
        .unwrap_or_else(|_| TokenStream::new())
}
