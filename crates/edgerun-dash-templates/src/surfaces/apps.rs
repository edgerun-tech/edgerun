//! Apps surface rendering.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use edgerun_web_ui::escape_attr;
use edgerun_web_ui::escape_html;
use edgerun_web_ui::escape_json;

#[cfg(feature = "std")]
use edgerun_config::BrowserAppSpec;

/// Renders the apps surface with app cards, capability pills, and search.
#[cfg(feature = "std")]
pub fn render_apps_surface(configured_apps: &[BrowserAppSpec]) -> String {
    let fallback = default_browser_apps();
    let apps = if configured_apps.is_empty() {
        &fallback[..]
    } else {
        configured_apps
    };
    let mut cards = String::new();
    for app in apps {
        let surfaces = if app.surfaces.is_empty() {
            String::from("No surfaces")
        } else {
            app.surfaces.join(", ")
        };
        let module_url = if app.module.url.is_empty() {
            String::from("Not configured")
        } else {
            app.module.url.clone()
        };
        let search_text = app_search_text(app);
        let capabilities = render_capability_pills(app);
        cards.push_str(&format!(
            "<article class=\"dash-card dash-app-card\" data-search-card data-search-text=\"{}\">\
            <div class=\"dash-app-head\"><h3>{}</h3><span class=\"dash-app-id\">{}</span></div>\
            <div class=\"dash-app-meta\"><div><span>Module URL</span><p>{}</p></div>\
            <div><span>Surfaces</span><p>{}</p></div></div>{}</article>",
            escape_attr(&search_text),
            escape_html(&app.title),
            escape_html(&app.app_id),
            escape_html(&module_url),
            escape_html(&surfaces),
            capabilities,
        ));
    }
    format!(
        "<div class=\"dash-code\">\
            <section class=\"dash-code-hero\">\
                <p>Browser node</p><h2>Apps</h2>\
                <span>Configured Wasm agents and the capability selectors they request.</span>\
            </section>\
            <section class=\"dash-code-tools\" aria-label=\"App tools\">\
                <label><span>Filter apps</span>\
                    <input type=\"search\" data-workspace-search-scope \
                        placeholder=\"Search apps and capabilities\">\
                </label>\
            </section>\
            <section class=\"dash-code-summary\" aria-label=\"App summary\">\
                <div><span>Apps</span><strong>{}</strong></div>\
                <div><span>Modules</span><strong>{}</strong></div>\
                <div><span>Required caps</span><strong>{}</strong></div>\
                <div><span>Optional caps</span><strong>{}</strong></div>\
            </section>\
            <section><h2>Installed apps</h2><div class=\"dash-grid\">{}</div></section>\
            <p class=\"dash-search-empty\" data-search-empty hidden>No matching apps.</p>\
        </div>",
        apps.len(),
        apps.iter().filter(|a| !a.module.url.is_empty()).count(),
        apps.iter().map(|a| a.required_capabilities.len()).sum::<usize>(),
        apps.iter().map(|a| a.optional_capabilities.len()).sum::<usize>(),
        cards,
    )
}

#[cfg(feature = "std")]
fn render_capability_pills(app: &BrowserAppSpec) -> String {
    let required = render_capability_list(&app.required_capabilities, true);
    let optional = render_capability_list(&app.optional_capabilities, false);
    format!(
        "<div class=\"dash-app-capabilities\">\
            <div class=\"dash-app-capability-group\">\
                <span>Required capabilities</span>{}\
            </div>\
            <div class=\"dash-app-capability-group\">\
                <span>Optional capabilities</span>{}\
            </div>\
        </div>",
        required, optional,
    )
}

#[cfg(feature = "std")]
fn render_capability_list(
    capabilities: &[edgerun_config::BrowserAppCapabilitySpec],
    required: bool,
) -> String {
    if capabilities.is_empty() {
        return "<p class=\"dash-app-capability-empty\">None</p>".to_string();
    }
    let mut out = String::from("<div class=\"dash-app-capability-tags\">");
    for capability in capabilities {
        let mut ops = String::new();
        if capability.operations.is_empty() {
            ops.push_str("access");
        } else {
            ops.push_str(&capability.operations.join(", "));
        }
        let mut constraints = String::new();
        if !capability.constraints.is_empty() {
            constraints.push(' ');
            constraints.push('(');
            constraints.push_str(&capability.constraints.join(", "));
            constraints.push(')');
        }
        let label = format!(
            "{} [{}]{}",
            capability.selector,
            escape_html(&ops),
            escape_html(&constraints),
        );
        let tag_class = if required {
            "dash-app-capability-pill dash-app-capability-pill-required"
        } else {
            "dash-app-capability-pill"
        };
        out.push_str(&format!("<span class=\"{}\">{}</span>", tag_class, label));
    }
    out.push_str("</div>");
    out
}

#[cfg(feature = "std")]
fn app_search_text(app: &BrowserAppSpec) -> String {
    let mut text = String::new();
    let mut append = |value: &str| {
        if value.is_empty() {
            return;
        }
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(value);
    };
    append(&app.app_id);
    append(&app.title);
    append(&app.module.url);
    for surface in &app.surfaces {
        append(surface);
    }
    for capability in app
        .required_capabilities
        .iter()
        .chain(app.optional_capabilities.iter())
    {
        append(&capability.selector);
        for operation in &capability.operations {
            append(operation);
        }
        for constraint in &capability.constraints {
            append(constraint);
        }
    }
    if text.is_empty() {
        String::from("app")
    } else {
        text
    }
}

/// Renders the browser app catalog JSON for `/apps/catalog.json`.
#[cfg(feature = "std")]
pub fn render_app_catalog(configured_apps: &[BrowserAppSpec]) -> String {
    let fallback = default_browser_apps();
    let apps = if configured_apps.is_empty() {
        &fallback[..]
    } else {
        configured_apps
    };
    let mut out = String::from("{\"apps\":[");
    for (index, app) in apps.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"app_id\":\"{}\",\"title\":\"{}\",\"module\":{{\"url\":\"{}\"",
            escape_json(&app.app_id),
            escape_json(&app.title),
            escape_json(&app.module.url),
        ));
        if let Some(sha256) = &app.module.sha256 {
            out.push_str(&format!(
                ",\"sha256\":\"{}\"",
                escape_json(sha256),
            ));
        }
        out.push_str("},\"surfaces\":[");
        for (surface_index, surface) in app.surfaces.iter().enumerate() {
            if surface_index > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&escape_json(surface));
            out.push('"');
        }
        out.push_str("],\"required_capabilities\":");
        render_capabilities_json(&mut out, &app.required_capabilities);
        out.push_str(",\"optional_capabilities\":");
        render_capabilities_json(&mut out, &app.optional_capabilities);
        out.push('}');
    }
    out.push_str("]}");
    out
}

#[cfg(feature = "std")]
fn render_capabilities_json(
    out: &mut String,
    capabilities: &[edgerun_config::BrowserAppCapabilitySpec],
) {
    out.push('[');
    for (index, capability) in capabilities.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"selector\":\"{}\",\"operations\":[",
            escape_json(&capability.selector),
        ));
        for (op_index, op) in capability.operations.iter().enumerate() {
            if op_index > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&escape_json(op));
            out.push('"');
        }
        out.push_str("],\"constraints\":[");
        for (ci, constraint) in capability.constraints.iter().enumerate() {
            if ci > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&escape_json(constraint));
            out.push('"');
        }
        out.push_str("]}");
    }
    out.push(']');
}

/// Fallback app specs used when no config provides browser apps.
#[cfg(feature = "std")]
pub fn default_browser_apps() -> Vec<BrowserAppSpec> {
    use edgerun_config::BrowserAppModuleSpec;
    use edgerun_config::BrowserAppSpec;

    vec![
        BrowserAppSpec {
            app_id: "edgerun.mail".to_string(),
            title: "Mail".to_string(),
            module: BrowserAppModuleSpec {
                url: "/modules/mail.wasm".to_string(),
                sha256: None,
            },
            surfaces: vec!["mail".to_string()],
            required_capabilities: Vec::new(),
            optional_capabilities: Vec::new(),
        },
        BrowserAppSpec {
            app_id: "edgerun.git".to_string(),
            title: "Code".to_string(),
            module: BrowserAppModuleSpec {
                url: "/modules/git.wasm".to_string(),
                sha256: None,
            },
            surfaces: vec!["git".to_string()],
            required_capabilities: Vec::new(),
            optional_capabilities: Vec::new(),
        },
        BrowserAppSpec {
            app_id: "edgerun.blog".to_string(),
            title: "Build Log".to_string(),
            module: BrowserAppModuleSpec {
                url: "/modules/blog.wasm".to_string(),
                sha256: None,
            },
            surfaces: vec!["blog".to_string()],
            required_capabilities: Vec::new(),
            optional_capabilities: Vec::new(),
        },
    ]
}
