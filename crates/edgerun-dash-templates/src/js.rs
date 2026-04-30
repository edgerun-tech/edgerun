//! JavaScript constants for the dashboard workspace.
//!
//! All JS is loaded via `include_str!` so it can be edited as plain `.js` files.

/// Workspace interaction JS: fetch headers, node activation, capability grants,
/// hx-get delegation, mail login/compose form handlers, fallback stubs.
pub const WORKSPACE_JS: &str = include_str!("workspace.js");

/// Dashboard cohesive workspace JS: surface hydration, module geometry/clamping,
/// drag-reorder, dock interactions, chat dock, context menus, status refresh.
pub const DASH_COHESIVE_JS: &str = include_str!("dash_cohesive.js");
