extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;
use edgerun_config::{BrowserAppModuleSpec, BrowserAppSpec};

#[derive(Clone, Copy, Debug)]
pub struct FooterLink<'a> {
    pub href: &'a str,
    pub label: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct PageShell<'a> {
    pub lang: &'a str,
    pub title: &'a str,
    pub description: &'a str,
    pub theme_color: &'a str,
    pub generator: &'a str,
    pub extra_head: &'a str,
    pub style: &'a str,
    pub brand_href: &'a str,
    pub brand_label: &'a str,
    pub brand_text: &'a str,
    pub header_center: &'a str,
    pub header_actions: &'a str,
    pub footer: &'a str,
    pub body: &'a str,
    pub script_src: Option<&'a str>,
    pub workspace_modules: &'a [WorkspaceModule<'a>],
}

#[derive(Clone, Copy, Debug)]
pub struct WorkspaceModule<'a> {
    pub app_id: &'a str,
    pub title: &'a str,
    pub surface: &'a str,
    pub selector: &'a str,
    pub wasm: &'a str,
}

pub const BASE_STYLE: &str = r#"
:root {
  color-scheme: light dark;
  --foreground: #0d1f24;
  --background: #f7f3eb;
  --muted: #6b7280;
}
* { box-sizing: border-box; }
body { margin: 0; font-family: Inter, ui-sans-serif, system-ui, sans-serif; color: var(--foreground); background: var(--background); }
#header { display: flex; gap: 1rem; padding: 0.8rem 1rem; align-items: center; border-bottom: 1px solid #d7d1c7; background: rgba(255,255,255,.82); position: sticky; top: 0; }
.brand { font-weight: 700; text-decoration: none; color: inherit; }
.page-footer { margin-top: 3rem; padding: 1rem; border-top: 1px solid #d7d1c7; color: var(--muted); font-size: 0.85rem; }
.footer-links { display: flex; flex-wrap: wrap; gap: .75rem; }
.workspace-actions { margin-left: auto; display: flex; gap: .75rem; align-items: center; }
.workspace-module-list { padding: 1rem; display: grid; grid-template-columns: repeat(auto-fit,minmax(180px,1fr)); gap: .75rem; }
"#;

pub const DASH_STYLE: &str = r#"
.app-card {
  border: 1px solid #d7d1c7;
  border-radius: 8px;
  padding: 0.75rem;
  background: #fffdf6;
}
.app-title { font-weight: 600; }
.app-surface { color: var(--muted); font-size: 0.85rem; margin-bottom: 0.5rem; }
.app-open { color: #0b5ed7; text-decoration: none; }
@media (max-width: 640px) {
  .workspace-module-list {
    grid-template-columns: 1fr;
  }
}
"#;

pub const DASH_BODY: &str = r#"
<section class="surface-home">
  <p>Edgerun dashboard home.</p>
</section>
<section id="workspace" class="workspace-module-list"></section>
"#;

pub const DASH_COHESIVE_JS: &str = r#"
document.addEventListener("click", (event) => {
  const action = event.target instanceof Element ? event.target.closest("[data-surface]") : null;
  if (!action) {
    return;
  }
  const surface = action.getAttribute("data-surface") || "";
  if (!surface) {
    return;
  }
  window.history.replaceState({}, "", "/" + surface.replace(/^\\/+/, ""));
});
"#;

pub const THEME_TOGGLE_JS: &str = r#"
document.documentElement.setAttribute("data-theme", "light");
function __edgerunSetTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme);
}
function __edgerunToggleTheme() {
  const current = document.documentElement.getAttribute("data-theme") || "light";
  __edgerunSetTheme(current === "light" ? "dark" : "light");
}
window.__edgerunSetTheme = __edgerunSetTheme;
window.__edgerunToggleTheme = __edgerunToggleTheme;
"#;

pub const fn theme_toggle_js() -> &'static str {
    THEME_TOGGLE_JS
}

pub const WORKSPACE_JS: &str = r#"
function __edgerunLoadWorkspace(moduleWasm, selector, surface, title) {
  const root = document.getElementById("workspace");
  if (!root) return;
  const frame = document.createElement("div");
  frame.className = "workspace-module";
  frame.dataset.wasm = moduleWasm || "";
  frame.dataset.surface = surface || "";
  frame.dataset.selector = selector || "";
  frame.dataset.title = title || "";
  frame.textContent = `workspace module ${title || ""} loaded`;
  root.appendChild(frame);
}
document.addEventListener("click", (event) => {
  const el = event.target instanceof Element ? event.target.closest("[data-workspace-module]") : null;
  if (!el) return;
  const moduleWasm = el.dataset.wasm || "";
  const selector = el.dataset.selector || "";
  const surface = el.dataset.surface || "";
  const title = el.dataset.title || el.textContent || "";
  __edgerunLoadWorkspace(moduleWasm, selector, surface, title);
});
"#;

pub const fn workspace_js() -> &'static str {
    WORKSPACE_JS
}

pub const FAVICON_COMPASS_SVG: &str = r#"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\"><text y=\"76\" font-size=\"76\">&#8984;</text></svg>"#;
pub const FAVICON_COMMAND_SVG: &str = r#"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\"><text y=\"76\" font-size=\"76\">&#9889;</text></svg>"#;

pub fn escape_html(input: &str) -> String {
    html_escape(input)
}

pub fn escape_attr(input: &str) -> String {
    html_escape(input)
}

pub fn escape_json(input: &str) -> String {
    json_escape(input)
}

pub fn render_header_search(
    action: &str,
    id: &str,
    param: &str,
    placeholder: &str,
    aria: &str,
) -> String {
    format!(
        r#"<form class=\"header-search\" method=\"get\" action=\"{}\"><input id=\"{}\" name=\"{}\" placeholder=\"{}\" aria-label=\"{}\" type=\"search\" /></form>"#,
        escape_attr(action),
        escape_attr(id),
        escape_attr(param),
        escape_attr(placeholder),
        escape_attr(aria)
    )
}

pub fn render_header_search_input(id: &str, placeholder: &str, label: &str) -> String {
    format!(
        r#"<input id=\"{}\" placeholder=\"{}\" aria-label=\"{}\" type=\"search\" />"#,
        escape_attr(id),
        escape_attr(placeholder),
        escape_attr(label)
    )
}

pub fn render_workspace_actions(scope: &str, active: &str) -> String {
    let scope = if scope.is_empty() { "default" } else { scope };
    if active.is_empty() {
        format!(
            r#"<a href=\"/surface/{0}\" class=\"workspace-action active\">{0}</a><a href=\"/surface/{1}\" class=\"workspace-action\">{1}</a><a href=\"/surface/{2}\" class=\"workspace-action\">{2}</a>"#,
            escape_html(scope),
            escape_html("blog"),
            escape_html("git")
        )
    } else {
        format!(
            r#"<a href=\"/surface/{0}\" class=\"workspace-action active\">{0}</a><a href=\"{1}\" class=\"workspace-action\">{1}</a>"#,
            escape_html(scope),
            escape_html(active)
        )
    }
}

pub fn render_app_catalog(apps: &[BrowserAppSpec]) -> String {
    let mut out = String::from("[");
    for (index, app) in apps.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(r#"{"app_id":""#);
        out.push_str(&escape_json(&app.app_id));
        out.push_str(r#"","title":""#);
        out.push_str(&escape_json(&app.title));
        out.push_str(r#"","surfaces":["#);
        for (idx, surface) in app.surfaces.iter().enumerate() {
            if idx > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&escape_json(surface));
            out.push('"');
        }
        out.push_str(r#"],"module":{"url":""#);
        out.push_str(&escape_json(&app.module.url));
        out.push_str(r#"","sha256":""#);
        out.push_str(&escape_json(app.module.sha256.as_deref().unwrap_or("")));
        out.push_str(r#""},"required_capabilities":[],"optional_capabilities":[]}}"#);
    }
    out.push(']');
    out
}

pub fn render_apps_surface(apps: &[BrowserAppSpec]) -> String {
    let mut out = String::new();
    out.push_str(r#"<section class="surface-home">"#);
    if apps.is_empty() {
        out.push_str("<p>No apps are configured.</p>");
    } else {
        for app in apps {
            let surface = app.surfaces.first().map(|value| value.as_str()).unwrap_or("");
            out.push_str("<article class=\"app-card\" data-app-id=\"");
            out.push_str(&escape_attr(&app.app_id));
            out.push_str("\" data-surface=\"");
            out.push_str(&escape_attr(surface));
            out.push_str("\" data-title=\"");
            out.push_str(&escape_attr(&app.title));
            out.push_str("\" data-wasm=\"");
            out.push_str(&escape_attr(&app.module.url));
            out.push_str("\"><div class=\"app-title\">");
            out.push_str(&escape_html(&app.title));
            out.push_str("</div><div class=\"app-surface\">");
            out.push_str(&escape_html(surface));
            out.push_str("</div><a class=\"app-open\" href=\"/");
            out.push_str(&escape_attr(surface));
            out.push_str(r#"\">open</a></article>"#);
        }
    }
    out.push_str("</section>");
    out
}

pub fn render_surface(_title: &str, surface: &str, body: &str) -> String {
    let footer = render_status_footer();
    let style = full_style();
    let body_with_scripts = format!(
        "{}<script>{}{}{}</script>",
        body,
        theme_toggle_js(),
        WORKSPACE_JS,
        DASH_COHESIVE_JS
    );
    render_page(&PageShell {
        lang: "en",
        title: &escape_html(surface),
        description: "EdgeRun browser surface.",
        theme_color: "#146c63",
        generator: "edgerun-server",
        extra_head: "",
        style: &style,
        brand_href: "/",
        brand_label: "Edgerun Dashboard",
        brand_text: "Edgerun Dashboard",
        header_center: "",
        header_actions: "",
        footer: &footer,
        body: &body_with_scripts,
        script_src: None,
        workspace_modules: &[],
    })
}

pub fn render_status_footer() -> String {
    let local_links = [
        FooterLink {
            href: "/apps/catalog.json",
            label: "App catalog",
        },
        FooterLink {
            href: "/surface/apps",
            label: "Surfaces",
        },
    ];
    render_common_footer("dashboard", &local_links, "")
}

pub fn full_style() -> String {
    format!("{BASE_STYLE}{DASH_STYLE}")
}

pub fn default_browser_apps() -> Vec<BrowserAppSpec> {
    vec![BrowserAppSpec {
        app_id: "edgerun.mail".to_string(),
        title: "Mail".to_string(),
        module: BrowserAppModuleSpec {
            url: "/modules/mail.wasm".to_string(),
            sha256: None,
        },
        surfaces: vec!["mail".to_string()],
        required_capabilities: Vec::new(),
        optional_capabilities: Vec::new(),
    }]
}

pub fn selector_for_surface(surface: &str) -> &'static str {
    match surface {
        "surface/mail" => "mail://edgerun/*",
        "mail" => "mail://edgerun/*",
        "surface/apps" => "apps://edgerun/*",
        "apps" => "apps://edgerun/*",
        _ => "edgerun://local/*",
    }
}

pub fn workspace_modules_from_config<'a>(apps: &'a [BrowserAppSpec]) -> Vec<WorkspaceModule<'a>> {
    apps.iter()
        .map(|app| {
            let surface = app.surfaces.first().map(String::as_str).unwrap_or("");
            WorkspaceModule {
                app_id: &app.app_id,
                title: &app.title,
                surface,
                selector: selector_for_surface(surface),
                wasm: app.module.url.as_str(),
            }
        })
        .collect()
}

pub fn render_common_footer(
    context: &str,
    local_links: &[FooterLink<'_>],
    language_links: &str,
) -> String {
    let mut out = String::new();
    out.push_str("<div class=\"page-footer\">");
    out.push_str("<div>");
    let _ = core::fmt::write(&mut out, format_args!("Surface: {}", escape_html(context)));
    out.push_str("</div>");
    if !local_links.is_empty() {
        out.push_str("<div class=\"footer-links\">");
        for link in local_links {
            out.push_str(&format!(
                "<a href=\"{}\">{}</a>",
                escape_attr(link.href),
                escape_html(link.label),
            ));
        }
        out.push_str("</div>");
    }
    if !language_links.is_empty() {
        out.push_str("<div class=\"footer-links\">");
        out.push_str(language_links);
        out.push_str("</div>");
    }
    out.push_str("</div>");
    out
}

pub fn render_page(shell: &PageShell<'_>) -> String {
    let mut out = String::new();
    out.push_str("<!doctype html><html lang=\"");
    out.push_str(&escape_attr(shell.lang));
    out.push_str("\"><head><meta charset=\"utf-8\" />");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />");
    out.push_str("<meta name=\"description\" content=\"");
    out.push_str(&escape_attr(shell.description));
    out.push_str("\" /><meta name=\"generator\" content=\"");
    out.push_str(&escape_attr(shell.generator));
    out.push_str("\" /><meta name=\"theme-color\" content=\"");
    out.push_str(&escape_attr(shell.theme_color));
    out.push_str("\" />");
    out.push_str("<title>");
    out.push_str(&escape_html(shell.title));
    out.push_str("</title>");
    out.push_str(shell.extra_head);
    out.push_str("<style>");
    out.push_str(BASE_STYLE);
    if !shell.style.is_empty() {
        out.push_str("</style><style>");
        out.push_str(shell.style);
    }
    out.push_str("</style></head><body>");

    out.push_str("<header id=\"header\">");
    out.push_str("<a class=\"brand\" href=\"");
    out.push_str(&escape_attr(shell.brand_href));
    out.push_str("\">");
    out.push_str(&escape_html(shell.brand_text));
    out.push_str("</a>");
    out.push_str("<div class=\"header-center\">");
    out.push_str(shell.header_center);
    out.push_str("</div>");
    out.push_str("<div class=\"workspace-actions\">");
    out.push_str(shell.header_actions);
    out.push_str("</div></header>");
    out.push_str("<main>");
    out.push_str(shell.body);
    if !shell.workspace_modules.is_empty() {
        out.push_str("<section id=\"workspace\" class=\"workspace-module-list\">");
        for module in shell.workspace_modules {
            out.push_str("<article class=\"module-card\" data-workspace-module data-wasm=\"");
            out.push_str(&escape_attr(module.wasm));
            out.push_str("\" data-surface=\"");
            out.push_str(&escape_attr(module.surface));
            out.push_str("\" data-selector=\"");
            out.push_str(&escape_attr(module.selector));
            out.push_str("\" data-title=\"");
            out.push_str(&escape_attr(module.title));
            out.push_str("\"><h3>");
            out.push_str(&escape_html(module.app_id));
            out.push_str("</h3><p>");
            out.push_str(&escape_html(module.title));
            out.push_str("</p><a href=\"/");
            out.push_str(&escape_attr(module.surface));
            out.push_str("\">open</a></article>");
        }
        out.push_str("</section>");
    }
    out.push_str("</main>");
    out.push_str(shell.footer);
    out.push_str("<script>");
    out.push_str(theme_toggle_js());
    out.push_str("</script><script>");
    out.push_str(workspace_js());
    out.push_str("</script>");
    if let Some(src) = shell.script_src {
        out.push_str("<script type=\"module\" src=\"");
        out.push_str(&escape_attr(src));
        out.push_str("\"></script>");
    }
    out.push_str("</body></html>");
    out
}

fn html_escape(input: &str) -> String {
    let mut out = String::new();
    for byte in input.bytes() {
        match byte {
            b'&' => out.push_str("&amp;"),
            b'<' => out.push_str("&lt;"),
            b'>' => out.push_str("&gt;"),
            b'"' => out.push_str("&quot;"),
            b'\'' => out.push_str("&#x27;"),
            _ => out.push(byte as char),
        }
    }
    out
}

fn json_escape(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' as char => {
                let _ = out.write_fmt(format_args!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}
