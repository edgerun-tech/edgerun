use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustItemKind {
    Fn,
    Struct,
    Enum,
    Trait,
    Impl,
    Mod,
    Use,
    Const,
    Static,
    Type,
}

#[derive(Debug, Clone)]
pub struct RustItem {
    pub kind: RustItemKind,
    pub name: String,
    pub start: usize,
    pub end: usize,
    pub body: Option<(usize, usize)>,
    pub signature: Option<(usize, usize)>,
}

pub fn list_file(path: &Path) -> Result<Vec<String>, String> {
    let source = read_source(path)?;
    Ok(scan_items(&source)
        .into_iter()
        .map(|item| item_summary(&source, &item))
        .collect())
}

pub fn find_fn(path: &Path, name: &str) -> Result<Option<String>, String> {
    let source = read_source(path)?;
    Ok(find_function(&source, name).map(|item| source[item.start..item.end].to_string()))
}

pub fn replace_fn_body(path: &Path, name: &str, body: &str) -> Result<bool, String> {
    let source = read_source(path)?;
    let Some(item) = find_function(&source, name) else {
        return Ok(false);
    };
    let Some((body_start, body_end)) = item.body else {
        return Err(format!("function has no replaceable body: {name}"));
    };

    let close_indent = line_indent(&source, body_end.saturating_sub(1));
    let replacement = normalize_body(body, close_indent);
    let mut output = source;
    output.replace_range(body_start + 1..body_end - 1, &replacement);
    validate_function_present(&output, name)?;
    fs::write(path, output).map_err(|err| format!("write failed: {err}"))?;
    Ok(true)
}

pub fn add_fn(path: &Path, name: &str, args: &str, ret: &str, body: &str) -> Result<(), String> {
    let mut source = read_source(path)?;
    if find_function(&source, name).is_some() {
        return Err(format!("function already exists: {name}"));
    }
    let ret = ret.trim();
    let ret_part = if ret.is_empty() || ret == "()" {
        String::new()
    } else {
        format!(" -> {ret}")
    };
    let body = normalize_added_body(body);
    if !source.ends_with('\n') {
        source.push('\n');
    }
    source.push('\n');
    source.push_str(&format!(
        "pub fn {name}({}){ret_part} {body}\n",
        args.trim()
    ));
    validate_function_present(&source, name)?;
    fs::write(path, source).map_err(|err| format!("write failed: {err}"))
}

pub fn remove_fn(path: &Path, name: &str) -> Result<bool, String> {
    let source = read_source(path)?;
    let Some(item) = find_function(&source, name) else {
        return Ok(false);
    };
    let end = consume_following_blank_line(&source, item.end);
    let mut output = source;
    output.replace_range(item.start..end, "");
    if find_function(&output, name).is_some() {
        return Err(format!("function still present after removal: {name}"));
    }
    fs::write(path, output).map_err(|err| format!("write failed: {err}"))?;
    Ok(true)
}

pub fn add_use(path: &Path, use_path: &str) -> Result<(), String> {
    let mut source = read_source(path)?;
    let rendered = format!("pub use {};\n", use_path.trim().trim_end_matches(';'));
    if source.lines().any(|line| line.trim() == rendered.trim()) {
        return Ok(());
    }
    let insert_at = use_insert_offset(&source);
    source.insert_str(insert_at, &rendered);
    fs::write(path, source).map_err(|err| format!("write failed: {err}"))
}

pub fn add_derive(path: &Path, name: &str, derive: &str) -> Result<bool, String> {
    let source = read_source(path)?;
    let Some(item) = scan_items(&source).into_iter().find(|item| {
        matches!(item.kind, RustItemKind::Struct | RustItemKind::Enum) && item.name == name
    }) else {
        return Ok(false);
    };
    let derives = derive
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    if derives.is_empty() {
        return Err("derive list is empty".to_string());
    }
    let attr = format!("{}#[derive({derives})]\n", line_indent(&source, item.start));
    let mut output = source;
    output.insert_str(item.start, &attr);
    fs::write(path, output).map_err(|err| format!("write failed: {err}"))?;
    Ok(true)
}

pub fn rename_identifier_in_file(path: &Path, old: &str, new: &str) -> Result<bool, String> {
    let source = read_source(path)?;
    if !contains_identifier(&source, old) {
        return Ok(false);
    }
    let code = code_mask(&source);
    let mut output = String::with_capacity(source.len());
    let mut last = 0;
    let mut changed = false;
    let mut index = 0;
    while index < source.len() {
        if code[index]
            && source[index..].starts_with(old)
            && ident_boundary(&source, index, old.len())
        {
            output.push_str(&source[last..index]);
            output.push_str(new);
            index += old.len();
            last = index;
            changed = true;
            continue;
        }
        index += next_char_len(&source[index..]);
    }
    if !changed {
        return Ok(false);
    }
    output.push_str(&source[last..]);
    fs::write(path, output).map_err(|err| format!("write failed: {err}"))?;
    Ok(true)
}

pub fn file_contains_identifier(path: &Path, name: &str) -> Result<bool, String> {
    let source = read_source(path)?;
    Ok(contains_identifier(&source, name))
}

pub fn new_file(path: &Path, content: &str) -> Result<(), String> {
    if path.exists() {
        return Err(format!("file already exists: {}", path.display()));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("mkdir failed: {err}"))?;
    }
    if !content.trim().is_empty() {
        let _ = scan_items(content);
    }
    fs::write(path, content).map_err(|err| format!("write failed: {err}"))
}

pub fn remove_file(path: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|err| format!("remove failed: {err}"))
}

pub fn incoming_refs(project_files: &[PathBuf], file: &Path) -> usize {
    let Ok(source) = fs::read_to_string(file) else {
        return 0;
    };
    let defined = scan_items(&source)
        .into_iter()
        .filter(|item| !item.name.is_empty())
        .map(|item| item.name)
        .collect::<Vec<_>>();
    if defined.is_empty() {
        return 0;
    }

    project_files
        .iter()
        .filter(|candidate| candidate.as_path() != file)
        .filter_map(|candidate| fs::read_to_string(candidate).ok())
        .map(|content| {
            defined
                .iter()
                .map(|name| count_identifier_refs(&content, name))
                .sum::<usize>()
        })
        .sum()
}

pub fn scan_items(source: &str) -> Vec<RustItem> {
    let code = code_mask(source);
    let mut items = Vec::new();
    let mut index = 0;
    while index < source.len() {
        let Some((kind, keyword)) = item_keyword_at(source, &code, index) else {
            index += next_char_len(&source[index..]);
            continue;
        };
        let Some((name, name_end)) = item_name_after(source, &code, index + keyword.len(), kind)
        else {
            index += keyword.len();
            continue;
        };
        let item_start = item_start_with_attrs(source, index);
        let (end, body) = item_end(source, &code, name_end).unwrap_or_else(|| {
            let line_end = source[name_end..]
                .find('\n')
                .map(|offset| name_end + offset + 1)
                .unwrap_or(source.len());
            (line_end, None)
        });
        let signature = body.map(|(open, _)| (index, open)).or(Some((index, end)));
        items.push(RustItem {
            kind,
            name,
            start: item_start,
            end,
            body,
            signature,
        });
        index = end.max(index + keyword.len());
    }
    items
}

fn read_source(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("read {} failed: {err}", path.display()))
}

fn find_function(source: &str, name: &str) -> Option<RustItem> {
    scan_items(source)
        .into_iter()
        .find(|item| item.kind == RustItemKind::Fn && item.name == name)
}

fn validate_function_present(source: &str, name: &str) -> Result<(), String> {
    if find_function(source, name).is_some() {
        Ok(())
    } else {
        Err(format!(
            "post-edit validation failed: function not found: {name}"
        ))
    }
}

fn item_summary(source: &str, item: &RustItem) -> String {
    match item.kind {
        RustItemKind::Fn => {
            let signature = item
                .signature
                .map(|(start, end)| compact_ws(&source[start..end]))
                .unwrap_or_else(|| format!("fn {}", item.name));
            signature.trim_end_matches('{').trim().to_string()
        }
        RustItemKind::Struct => format!("struct {}", item.name),
        RustItemKind::Enum => format!("enum {}", item.name),
        RustItemKind::Trait => format!("trait {}", item.name),
        RustItemKind::Impl => format!("impl {}", item.name),
        RustItemKind::Mod => format!("mod {}", item.name),
        RustItemKind::Use => compact_ws(&source[item.start..item.end]),
        RustItemKind::Const => format!("const {}", item.name),
        RustItemKind::Static => format!("static {}", item.name),
        RustItemKind::Type => format!("type {}", item.name),
    }
}

fn compact_ws(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn item_keyword_at(
    source: &str,
    code: &[bool],
    index: usize,
) -> Option<(RustItemKind, &'static str)> {
    const KEYWORDS: &[(&str, RustItemKind)] = &[
        ("fn", RustItemKind::Fn),
        ("struct", RustItemKind::Struct),
        ("enum", RustItemKind::Enum),
        ("trait", RustItemKind::Trait),
        ("impl", RustItemKind::Impl),
        ("mod", RustItemKind::Mod),
        ("use", RustItemKind::Use),
        ("const", RustItemKind::Const),
        ("static", RustItemKind::Static),
        ("type", RustItemKind::Type),
    ];
    if !code.get(index).copied().unwrap_or(false) {
        return None;
    }
    for (keyword, kind) in KEYWORDS {
        if source[index..].starts_with(keyword) && ident_boundary(source, index, keyword.len()) {
            return Some((*kind, keyword));
        }
    }
    None
}

fn item_name_after(
    source: &str,
    code: &[bool],
    mut index: usize,
    kind: RustItemKind,
) -> Option<(String, usize)> {
    index = skip_ws_code(source, code, index);
    if kind == RustItemKind::Impl {
        let end = find_item_header_end(source, code, index).unwrap_or(index);
        return Some((compact_ws(&source[index..end]), end));
    }
    if kind == RustItemKind::Use {
        let end = source[index..]
            .find(';')
            .map(|offset| index + offset + 1)
            .unwrap_or(source.len());
        return Some((compact_ws(&source[index..end]), end));
    }
    let start = index;
    while index < source.len() {
        let ch = source[index..].chars().next()?;
        if !(ch == '_' || ch.is_ascii_alphanumeric()) {
            break;
        }
        index += ch.len_utf8();
    }
    (index > start).then(|| (source[start..index].to_string(), index))
}

fn item_end(source: &str, code: &[bool], index: usize) -> Option<(usize, Option<(usize, usize)>)> {
    let mut i = index;
    while i < source.len() {
        if !code[i] {
            i += next_char_len(&source[i..]);
            continue;
        }
        match source.as_bytes()[i] {
            b';' => return Some((i + 1, None)),
            b'{' => {
                let close = matching_brace(source, code, i)?;
                return Some((close + 1, Some((i, close + 1))));
            }
            _ => i += next_char_len(&source[i..]),
        }
    }
    None
}

fn find_item_header_end(source: &str, code: &[bool], index: usize) -> Option<usize> {
    let mut i = index;
    while i < source.len() {
        if code[i] && matches!(source.as_bytes()[i], b'{' | b';') {
            return Some(i);
        }
        i += next_char_len(&source[i..]);
    }
    None
}

fn matching_brace(source: &str, code: &[bool], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = open;
    while index < source.len() {
        if code[index] {
            match source.as_bytes()[index] {
                b'{' => depth += 1,
                b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            }
        }
        index += next_char_len(&source[index..]);
    }
    None
}

fn skip_ws_code(source: &str, code: &[bool], mut index: usize) -> usize {
    while index < source.len() {
        let ch = source[index..].chars().next().unwrap_or('\0');
        if code[index] && !ch.is_whitespace() {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

fn ident_boundary(source: &str, start: usize, len: usize) -> bool {
    let before = source[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| !(ch == '_' || ch.is_ascii_alphanumeric()));
    let after = source[start + len..]
        .chars()
        .next()
        .is_none_or(|ch| !(ch == '_' || ch.is_ascii_alphanumeric()));
    before && after
}

fn code_mask(source: &str) -> Vec<bool> {
    let mut code = vec![true; source.len()];
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
            let start = i;
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            code[start..i].fill(false);
        } else if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            code[start..i].fill(false);
        } else if bytes[i] == b'"' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            code[start..i].fill(false);
        } else if bytes[i] == b'\'' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                } else if bytes[i] == b'\'' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            code[start..i].fill(false);
        } else {
            i += 1;
        }
    }
    code
}

fn item_start_with_attrs(source: &str, keyword_start: usize) -> usize {
    let mut start = line_start(source, keyword_start);
    loop {
        let prev_end = start.saturating_sub(1);
        if start == 0 {
            return start;
        }
        let prev_start = line_start(source, prev_end);
        let line = source[prev_start..prev_end].trim();
        if line.is_empty()
            || line.starts_with("#[")
            || line.starts_with("///")
            || line.starts_with("//!")
            || line.starts_with("//")
        {
            start = prev_start;
            continue;
        }
        return start;
    }
}

fn line_start(source: &str, index: usize) -> usize {
    source[..index].rfind('\n').map_or(0, |pos| pos + 1)
}

fn line_indent(source: &str, index: usize) -> &str {
    let start = line_start(source, index);
    let end = source[start..]
        .find(|ch: char| !ch.is_whitespace() || ch == '\n')
        .map(|offset| start + offset)
        .unwrap_or(start);
    &source[start..end]
}

fn normalize_body(body: &str, close_indent: &str) -> String {
    let trimmed = body.trim();
    let inner = trimmed
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .unwrap_or(trimmed)
        .trim();
    if inner.is_empty() {
        format!("\n{close_indent}")
    } else {
        format!("\n{}\n{close_indent}", inner)
    }
}

fn normalize_added_body(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        trimmed.to_string()
    } else {
        format!("{{\n{}\n}}", trimmed)
    }
}

fn consume_following_blank_line(source: &str, mut end: usize) -> usize {
    while end < source.len() {
        let line_end = source[end..]
            .find('\n')
            .map(|offset| end + offset + 1)
            .unwrap_or(source.len());
        if source[end..line_end].trim().is_empty() {
            end = line_end;
        } else {
            break;
        }
    }
    end
}

fn use_insert_offset(source: &str) -> usize {
    let mut offset = 0;
    let mut last_use_end = None;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with("#!") || trimmed.is_empty() {
            offset += line.len();
            continue;
        }
        if trimmed.starts_with("use ") || trimmed.starts_with("pub use ") {
            offset += line.len();
            last_use_end = Some(offset);
            continue;
        }
        break;
    }
    last_use_end.unwrap_or(offset)
}

fn count_identifier_refs(source: &str, name: &str) -> usize {
    if name.is_empty() {
        return 0;
    }
    let code = code_mask(source);
    let mut count = 0;
    let mut index = 0;
    while index < source.len() {
        if code[index]
            && source[index..].starts_with(name)
            && ident_boundary(source, index, name.len())
        {
            count += 1;
            index += name.len();
        } else {
            index += next_char_len(&source[index..]);
        }
    }
    count
}

fn contains_identifier(source: &str, name: &str) -> bool {
    count_identifier_refs(source, name) > 0
}

fn next_char_len(input: &str) -> usize {
    input.chars().next().map(char::len_utf8).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_function_ranges() {
        let source = "/// docs\npub fn one() {\n    two();\n}\nfn two() {}\n";
        let items = scan_items(source);
        assert_eq!(
            items
                .iter()
                .filter(|item| item.kind == RustItemKind::Fn)
                .count(),
            2
        );
        let one = items.iter().find(|item| item.name == "one").unwrap();
        assert!(source[one.start..one.end].starts_with("/// docs"));
        assert_eq!(
            one.body.map(|(start, _)| &source[start..start + 1]),
            Some("{")
        );
    }

    #[test]
    fn ignores_keywords_in_strings_and_comments() {
        let source = "fn real() { let s = \"fn fake() {}\"; /* fn nope() {} */ }\n";
        let items = scan_items(source);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "real");
    }
}
