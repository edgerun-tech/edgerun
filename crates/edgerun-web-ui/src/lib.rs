#![cfg_attr(target_os = "none", no_std)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};

pub struct FooterLink<'a> {
    pub href: &'a str,
    pub label: &'a str,
}

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
    pub header_extra: &'a str,
    pub footer: &'a str,
    pub body: &'a str,
    pub script_src: Option<&'a str>,
}

pub fn render_page(shell: &PageShell<'_>) -> String {
    let generator = if shell.generator.is_empty() {
        String::new()
    } else {
        format!(
            "<meta name=\"generator\" content=\"{}\">",
            escape_attr(shell.generator)
        )
    };
    let script = shell
        .script_src
        .map(|src| format!("<script src=\"{}\" defer></script>", escape_attr(src)))
        .unwrap_or_default();
    format!(
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"color-scheme\" content=\"light dark\"><meta name=\"theme-color\" content=\"{}\"><meta name=\"referrer\" content=\"strict-origin-when-cross-origin\">{}<title>{}</title><meta name=\"description\" content=\"{}\">{}<style>{}</style></head><body><a class=\"skip-link\" href=\"#content\">Skip to content</a><header class=\"topbar\"><a class=\"brand\" href=\"{}\" aria-label=\"{}\">{}</a>{}</header>{}{}{}</body></html>",
        escape_attr(shell.lang),
        escape_attr(shell.theme_color),
        generator,
        escape_html(shell.title),
        escape_attr(shell.description),
        shell.extra_head,
        shell.style,
        escape_attr(shell.brand_href),
        escape_attr(shell.brand_label),
        escape_html(shell.brand_text),
        shell.header_extra,
        shell.body,
        shell.footer,
        script
    )
}

pub fn render_common_footer(
    current_surface: &str,
    local_links: &[FooterLink<'_>],
    trailing_html: &str,
) -> String {
    let surface_links = [
        ("blog", "Build Log", "https://blog.edgerun.tech/"),
        ("git", "Code", "https://git.edgerun.tech/"),
        ("mail", "Mail", "https://mail.edgerun.tech/"),
    ];
    let mut surfaces = String::new();
    for (surface, label, href) in surface_links {
        surfaces.push_str(&format!(
            "<a href=\"{}\"{}>{}</a>",
            escape_attr(href),
            if surface == current_surface {
                " aria-current=\"page\""
            } else {
                ""
            },
            escape_html(label)
        ));
    }
    let mut local = String::new();
    for link in local_links {
        local.push_str(&format!(
            "<a href=\"{}\">{}</a>",
            escape_attr(link.href),
            escape_html(link.label)
        ));
    }
    if local.is_empty() {
        local.push_str("<span>Project surfaces</span>");
    }
    format!(
        "<footer class=\"site-footer\"><nav class=\"footer-primary\" aria-label=\"Edgerun surfaces\">{surfaces}</nav><nav class=\"footer-local\" aria-label=\"Page links\">{local}</nav>{}</footer>",
        trailing_html
    )
}

pub fn escape_html(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

pub fn escape_attr(input: &str) -> String {
    escape_html(input)
}

pub fn escape_json(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {}
            ch => out.push(ch),
        }
    }
    out
}

pub const FAVICON_COMPASS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text y="76" font-size="76">🧭</text></svg>"#;

pub const FAVICON_COMMAND_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><text x="32" y="45" text-anchor="middle" font-size="42">⌘</text></svg>"#;

pub const THEME_TOGGLE_JS: &str = r#"
const root=document.documentElement;
const stored=localStorage.getItem('theme');
if(stored){root.dataset.theme=stored}
if(!customElements.get('er-theme-toggle')){customElements.define('er-theme-toggle',class extends HTMLElement{connectedCallback(){this.attachShadow({mode:'open'}).innerHTML='<style>button{width:44px;height:44px;display:grid;place-items:center;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--text);cursor:pointer;font:24px/1 system-ui}button:hover{border-color:var(--accent)}</style><button type="button"></button>';const btn=this.shadowRoot.querySelector('button');const current=()=>root.dataset.theme||(matchMedia('(prefers-color-scheme:dark)').matches?'dark':'light');const render=()=>{const dark=current()==='dark';btn.textContent=dark?'☾':'☀';btn.title=dark?'Dark mode: switch to light mode':'Light mode: switch to dark mode';btn.setAttribute('aria-label',btn.title)};btn.onclick=()=>{const next=current()==='dark'?'light':'dark';root.dataset.theme=next;localStorage.setItem('theme',next);render()};render()}})}
"#;

pub const BASE_STYLE: &str = r#"
:root{color-scheme:light dark;--bg:#f7f3eb;--panel:#fffdf8;--text:#1c2430;--muted:#627084;--line:#d8cfc0;--accent:#146c63;--accent-ink:#f4fffb;--accent-2:#8b3f2f;--code:#eee6d8}
:root[data-theme=dark]{--bg:#101418;--panel:#171d22;--text:#f2ede4;--muted:#a5b2bf;--line:#2b353d;--accent:#6fc7b8;--accent-ink:#06201d;--accent-2:#dfa06b;--code:#232b31}
*{box-sizing:border-box}body{min-height:100vh;display:flex;flex-direction:column;margin:0;background:var(--bg);color:var(--text);font:16px/1.6 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}body>main{flex:0 0 auto}a{color:inherit}:focus-visible{outline:3px solid var(--accent);outline-offset:3px}.skip-link{position:absolute;left:12px;top:-60px;z-index:10;background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:8px 12px}.skip-link:focus{top:12px}.topbar{position:sticky;top:0;z-index:2;display:flex;gap:14px;align-items:center;padding:12px clamp(14px,3vw,44px);background:color-mix(in srgb,var(--bg) 88%,transparent);border-bottom:1px solid var(--line);backdrop-filter:blur(12px)}.brand{font-weight:800;text-decoration:none;white-space:nowrap}.topbar nav{display:flex;align-items:center;justify-content:end}.topbar nav a{color:var(--muted);text-decoration:none}.hero{padding:64px clamp(18px,4vw,56px) 42px;border-bottom:1px solid var(--line)}.hero h1{margin:0;font-size:clamp(42px,7vw,82px);line-height:.95;letter-spacing:0}.hero p{max-width:760px;color:var(--muted);font-size:19px}.eyebrow{margin:0 0 12px;color:var(--accent);font-weight:800;text-transform:uppercase;font-size:13px;letter-spacing:.08em}.empty{max-width:720px;margin:80px auto;padding:0 18px;color:var(--muted)}.site-footer{display:grid;grid-template-columns:max-content minmax(0,1fr) max-content;gap:18px;align-items:center;margin-top:auto;border-top:1px solid var(--line);padding:22px clamp(18px,4vw,56px);color:var(--muted)}.site-footer nav,.language-links{display:flex;gap:14px;align-items:center;flex-wrap:wrap}.site-footer a{text-decoration:none}.site-footer a:hover{color:var(--accent)}.site-footer a[aria-current=page],.site-footer a[aria-current=true]{color:var(--accent);font-weight:800}.footer-primary{font-weight:800}.footer-local{justify-content:center}.language-links{justify-content:end;text-transform:uppercase}.language-links a{font-weight:750}@media(max-width:720px){.site-footer{grid-template-columns:1fr}.footer-local,.language-links{justify-content:flex-start}}
"#;
