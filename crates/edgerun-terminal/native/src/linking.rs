// SPDX-License-Identifier: Apache-2.0
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::tab::{Tab, tab_current_dir};
use term_core::terminal::Terminal;

pub(crate) fn open_link(link: &str) {
    #[cfg(target_os = "macos")]
    let candidates = ["open"];
    #[cfg(target_os = "linux")]
    let candidates = ["xdg-open"];
    #[cfg(target_os = "windows")]
    let candidates = ["start"];

    for bin in candidates {
        if Command::new(bin)
            .arg(link)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
        {
            break;
        }
    }
}

fn is_link_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(
            ch,
            '-' | '_'
                | '.'
                | '/'
                | ':'
                | '?'
                | '#'
                | '&'
                | '='
                | '%'
                | '+'
                | '~'
                | '@'
                | '!'
                | '$'
                | '\''
                | '('
                | ')'
                | '*'
                | ','
                | ';'
        )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkMatch {
    pub(crate) text: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

fn is_trim_start(ch: char) -> bool {
    matches!(ch, '(' | '[' | '{' | '<' | '"' | '\'')
}

fn is_trim_end(ch: char) -> bool {
    matches!(
        ch,
        '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '>' | '"' | '\''
    )
}

fn trim_link_token(token: &str) -> String {
    let mut trimmed = token.trim();
    trimmed = trimmed.trim_start_matches(is_trim_start);
    trimmed = trimmed.trim_end_matches(is_trim_end);
    trimmed.to_string()
}

pub(crate) fn looks_like_url(token: &str) -> bool {
    token.starts_with("http://") || token.starts_with("https://") || token.starts_with("file://")
}

fn looks_like_file_path(token: &str) -> bool {
    if token.contains("://") {
        return false;
    }
    token.starts_with('/')
        || token.starts_with("./")
        || token.starts_with("../")
        || token.starts_with("~/")
        || token.contains('/')
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = bytes[i + 1];
            let lo = bytes[i + 2];
            let decode = |b| match b {
                b'0'..=b'9' => Some(b - b'0'),
                b'a'..=b'f' => Some(b - b'a' + 10),
                b'A'..=b'F' => Some(b - b'A' + 10),
                _ => None,
            };
            if let (Some(hi), Some(lo)) = (decode(hi), decode(lo)) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

pub(crate) fn parse_path_candidate(candidate: &str, tab: &Tab) -> Option<PathBuf> {
    if let Some(rest) = candidate.strip_prefix("file://") {
        let rest = rest.strip_prefix("localhost/").unwrap_or(rest);
        let decoded = percent_decode(rest);
        if decoded.starts_with('/') {
            return Some(PathBuf::from(decoded));
        }
        return None;
    }

    if let Some(rest) = candidate.strip_prefix("~/") {
        let home = dirs::home_dir()?;
        return Some(home.join(rest));
    }

    if candidate.starts_with('/') {
        return Some(PathBuf::from(candidate));
    }

    if candidate.starts_with("./") || candidate.starts_with("../") || candidate.contains('/') {
        let base = tab_current_dir(tab)?;
        return Some(base.join(candidate));
    }

    None
}

pub(crate) fn resolve_existing_path(candidate: &str, tab: &Tab) -> Option<PathBuf> {
    let path = parse_path_candidate(candidate, tab)?;
    if path.exists() { Some(path) } else { None }
}

pub(crate) fn detect_link_text_at_cell(
    term: &Terminal,
    col: usize,
    row: usize,
) -> Option<LinkMatch> {
    if row >= term.rows || col >= term.cols {
        return None;
    }
    let mut chars = Vec::with_capacity(term.cols);
    for c in 0..term.cols {
        let cell = term.display_cell(c, row);
        let ch = if cell.wide_continuation || cell.text.is_empty() {
            ' '
        } else {
            cell.text.chars().next().unwrap_or(' ')
        };
        chars.push(ch);
    }
    let ch = *chars.get(col)?;
    if ch.is_whitespace() || !is_link_char(ch) {
        return None;
    }
    let mut start = col;
    while start > 0 && is_link_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end + 1 < chars.len() && is_link_char(chars[end + 1]) {
        end += 1;
    }
    let token: String = chars[start..=end].iter().collect();
    let token = trim_link_token(&token);
    if token.is_empty() {
        return None;
    }
    if looks_like_url(&token) || looks_like_file_path(&token) {
        Some(LinkMatch {
            text: token,
            start,
            end,
        })
    } else {
        None
    }
}

pub(crate) fn detect_link_ranges(term: &Terminal) -> Vec<Vec<(usize, usize)>> {
    let mut ranges = Vec::with_capacity(term.rows);
    for row in 0..term.rows {
        let mut row_ranges = Vec::new();
        let mut chars = Vec::with_capacity(term.cols);
        for col in 0..term.cols {
            let cell = term.display_cell(col, row);
            let ch = if cell.wide_continuation || cell.text.is_empty() {
                ' '
            } else {
                cell.text.chars().next().unwrap_or(' ')
            };
            chars.push(ch);
        }

        let mut col = 0;
        while col < chars.len() {
            if !is_link_char(chars[col]) {
                col += 1;
                continue;
            }
            let start = col;
            while col + 1 < chars.len() && is_link_char(chars[col + 1]) {
                col += 1;
            }
            let end = col;
            let mut trimmed_start = start;
            let mut trimmed_end = end;
            while trimmed_start <= trimmed_end && is_trim_start(chars[trimmed_start]) {
                trimmed_start += 1;
            }
            while trimmed_end >= trimmed_start && is_trim_end(chars[trimmed_end]) {
                if trimmed_end == 0 {
                    break;
                }
                trimmed_end -= 1;
            }
            if trimmed_start <= trimmed_end {
                let token: String = chars[trimmed_start..=trimmed_end].iter().collect();
                if looks_like_url(&token) || looks_like_file_path(&token) {
                    row_ranges.push((trimmed_start, trimmed_end));
                }
            }
            col += 1;
        }
        ranges.push(row_ranges);
    }
    ranges
}

fn is_text_file(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 4096];
    let Ok(read_len) = file.read(&mut buf) else {
        return false;
    };
    if buf[..read_len].contains(&0) {
        return false;
    }
    std::str::from_utf8(&buf[..read_len]).is_ok()
}

pub(crate) fn open_path(path: &Path) {
    #[cfg(target_os = "linux")]
    {
        let bin = if path.is_dir() {
            "nautilus"
        } else if is_text_file(path) {
            "geany"
        } else {
            "nautilus"
        };
        let _ = Command::new(bin)
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
    }
}
