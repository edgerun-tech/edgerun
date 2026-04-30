//! HTML template fragments for the dashboard shell.

use edgerun_web_ui::WorkspaceModule;

/// Main dash body: workspace with module slots and dock buttons.
/// Rendered inside the PageShell, with JS and styles appended.
pub const DASH_BODY: &str = include_str!("dash_body.html");

/// Selects the appropriate CSS selector for a surface name.
pub fn selector_for_surface(surface: &str) -> &'static str {
    match surface {
        "mail" => "er-mail-surface",
        "git" | "code" => "er-git-surface",
        "blog" | "build-log" => "er-blog-surface",
        _ => "er-app-surface",
    }
}

/// Converts config app specs to workspace modules.
#[cfg(feature = "std")]
pub fn workspace_modules_from_config<'a>(
    apps: &'a [edgerun_config::BrowserAppSpec],
) -> Vec<WorkspaceModule<'a>> {
    let mut modules = Vec::new();
    for app in apps {
        for surface in &app.surfaces {
            modules.push(WorkspaceModule {
                app_id: app.app_id.as_str(),
                title: app.title.as_str(),
                surface: surface.as_str(),
                selector: selector_for_surface(surface),
                wasm: app.module.url.as_str(),
            });
        }
    }
    modules
}
