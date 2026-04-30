//! CSS constants for the dashboard.
//!
//! All CSS is loaded via `include_str!` so it can be edited as plain `.css` files.

use edgerun_web_ui::BASE_STYLE;
use edgerun_web_ui::THEME_TOGGLE_JS;

/// Dashboard-specific CSS (layout, modules, dock, mail, chat, responsive).
pub const DASH_STYLE: &str = include_str!("dash_style.css");

/// Complete style bundle: base styles + dashboard styles.
pub fn full_style() -> String {
    let mut out = String::with_capacity(BASE_STYLE.len() + DASH_STYLE.len());
    out.push_str(BASE_STYLE);
    out.push_str(DASH_STYLE);
    out
}

/// Theme toggle web component JS.
pub fn theme_toggle_js() -> &'static str {
    THEME_TOGGLE_JS
}
