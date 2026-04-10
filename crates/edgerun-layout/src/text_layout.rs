//! Text layout — generated from CSS Text + CSS Fonts proto data.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py
extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Apply text-transform.
pub fn apply_text_transform(text: &str, t: u8) -> String {
    match t {
        2 => text.to_uppercase(),
        3 => text.to_lowercase(),
        1 => text.split_whitespace().map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(ch) => ch.to_uppercase().collect::<String>() + c.as_str(),
            }
        }).collect::<Vec<_>>().join(" "),
        _ => text.to_string(),
    }
}

/// Should text wrap at this position?
pub fn should_wrap(wb: u8, ws: u8, ch: char) -> bool {
    if ws == 4 || ws == 3 { return false; }
    if ws == 2 || ws == 5 || ws == 7 { if ch.is_whitespace() { return true; } }
    match wb { 2 => true, 3 => !ch.is_whitespace(), _ => ch.is_whitespace() || ch == '-' }
}

/// Resolve font-weight keyword to numeric.
pub fn resolve_font_weight(wt: u8) -> u32 {
    match wt { 1 => 100, 2 => 200, 3 => 300, 4 => 400, 5 => 500, 6 => 600, 7 => 700, 8 => 800, 9 => 900, _ => 400 }
}
