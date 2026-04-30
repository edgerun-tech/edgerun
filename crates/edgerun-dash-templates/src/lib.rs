#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod css;
pub mod html;
pub mod js;
pub mod surfaces;

pub use css::DASH_STYLE;
pub use html::DASH_BODY;
pub use js::DASH_COHESIVE_JS;
pub use js::WORKSPACE_JS;

pub use css::full_style;
pub use css::theme_toggle_js;
pub use html::selector_for_surface;

pub use surfaces::apps;
pub use surfaces::mail;
pub use surfaces::render_blog_surface;
pub use surfaces::render_code_surface;
pub use surfaces::render_status_footer;
pub use surfaces::render_surface;

#[cfg(feature = "std")]
pub use html::workspace_modules_from_config;
#[cfg(feature = "std")]
pub use surfaces::apps::default_browser_apps;
#[cfg(feature = "std")]
pub use surfaces::apps::render_app_catalog;
#[cfg(feature = "std")]
pub use surfaces::apps::render_apps_surface;
