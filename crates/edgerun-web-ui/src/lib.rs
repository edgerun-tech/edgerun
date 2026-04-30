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
    pub header_center: &'a str,
    pub header_actions: &'a str,
    pub footer: &'a str,
    pub body: &'a str,
    pub script_src: Option<&'a str>,
    pub workspace_modules: &'a [WorkspaceModule<'a>],
}

pub struct WorkspaceModule<'a> {
    pub surface: &'a str,
    pub selector: &'a str,
    pub wasm: &'a str,
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
    let modules = render_workspace_modules(shell.workspace_modules);
    format!(
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"color-scheme\" content=\"light dark\"><meta name=\"theme-color\" content=\"{}\"><meta name=\"referrer\" content=\"strict-origin-when-cross-origin\">{}<title>{}</title><meta name=\"description\" content=\"{}\">{}<style>{}</style></head><body><a class=\"skip-link\" href=\"#content\">Skip to content</a><header class=\"topbar\"><div class=\"topbar-brand\"><a class=\"brand\" href=\"{}\" aria-label=\"{}\">{}</a></div><div class=\"topbar-center\">{}</div><div class=\"topbar-actions\">{}</div></header>{}{}{}{}</body></html>",
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
        shell.header_center,
        shell.header_actions,
        shell.body,
        modules,
        shell.footer,
        script
    )
}

fn render_workspace_modules(modules: &[WorkspaceModule<'_>]) -> String {
    if modules.is_empty() {
        return String::new();
    }
    let mut out =
        String::from("<script type=\"application/json\" id=\"edgerun-workspace-modules\">[");
    for (index, module) in modules.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"surface\":\"{}\",\"selector\":\"{}\",\"wasm\":\"{}\"}}",
            escape_json(module.surface),
            escape_json(module.selector),
            escape_json(module.wasm)
        ));
    }
    out.push_str("]</script>");
    out
}

pub fn render_common_footer(
    current_surface: &str,
    local_links: &[FooterLink<'_>],
    trailing_html: &str,
) -> String {
    let surface_links = [
        ("dash", "Dash", "https://dash.edgerun.tech/"),
        ("blog", "Build Log", "https://dash.edgerun.tech/#build-log"),
        ("git", "Code", "https://dash.edgerun.tech/#code"),
        ("mail", "Mail", "https://dash.edgerun.tech/#mail"),
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

pub fn render_header_search(
    action: &str,
    input_id: &str,
    name: &str,
    label: &str,
    placeholder: &str,
) -> String {
    format!(
        "<form class=\"header-search\" role=\"search\" action=\"{}\" method=\"get\"><label for=\"{}\">{}</label><input id=\"{}\" name=\"{}\" type=\"search\" placeholder=\"{}\" autocomplete=\"off\"><button type=\"submit\" title=\"{}\" aria-label=\"{}\"><svg aria-hidden=\"true\" viewBox=\"0 0 24 24\"><circle cx=\"11\" cy=\"11\" r=\"7\"></circle><path d=\"m16 16 4 4\"></path></svg></button></form>",
        escape_attr(action),
        escape_attr(input_id),
        escape_html(label),
        escape_attr(input_id),
        escape_attr(name),
        escape_attr(placeholder),
        escape_attr(label),
        escape_attr(label)
    )
}

pub fn render_header_search_input(input_id: &str, label: &str, placeholder: &str) -> String {
    format!(
        "<div class=\"header-search\" role=\"search\"><label for=\"{}\">{}</label><input id=\"{}\" type=\"search\" placeholder=\"{}\" autocomplete=\"off\"><span class=\"search-icon\" aria-hidden=\"true\"><svg viewBox=\"0 0 24 24\"><circle cx=\"11\" cy=\"11\" r=\"7\"></circle><path d=\"m16 16 4 4\"></path></svg></span></div>",
        escape_attr(input_id),
        escape_html(label),
        escape_attr(input_id),
        escape_attr(placeholder)
    )
}

pub fn render_workspace_actions(current_surface: &str, leading_html: &str) -> String {
    let mail_action = if current_surface == "mail" {
        String::new()
    } else {
        "<button class=\"workspace-action\" type=\"button\" data-workspace-mail title=\"Open mail in dash\" aria-label=\"Open mail in dash\"><span aria-hidden=\"true\">@</span><span>Mail</span></button>".to_string()
    };
    format!(
        "{}{}<nav aria-label=\"Theme\"><er-theme-toggle></er-theme-toggle></nav>",
        leading_html, mail_action
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

pub const WORKSPACE_JS: &str = r#"
const workspaceHeaders=path=>{const headers={'HX-Request':'true'};const auth=sessionStorage.getItem('dashMailAuth');if(auth&&path&&path.startsWith('/surface/mail'))headers.Authorization='Basic '+auth;return headers};
document.addEventListener('click',async event=>{const trigger=event.target.closest('[hx-get]');if(!trigger)return;event.preventDefault();const target=document.querySelector(trigger.getAttribute('hx-target')||'');if(!target)return;const path=trigger.getAttribute('hx-get');const response=await fetch(path,{headers:workspaceHeaders(path)});if(!response.ok)return;const html=await response.text();const swap=trigger.getAttribute('hx-swap')||'innerHTML';if(swap==='outerHTML')target.outerHTML=html;else target.innerHTML=html;const hash=trigger.getAttribute('data-dash-hash');if(hash&&location.hash!==hash)history.pushState(null,'',hash);if(trigger.classList.contains('dash-tab')){const hashes={'/surface/blog':'#build-log','/surface/git':'#code','/surface/mail':'#mail'};const next=hashes[path]||location.pathname;if(location.hash!==next)history.pushState(null,'',next);document.querySelectorAll('.dash-tab[aria-current]').forEach(node=>node.removeAttribute('aria-current'));trigger.setAttribute('aria-current','page')}else if(path.startsWith('/surface/git')||path.startsWith('/surface/blog')){document.querySelectorAll('.dash-tab[aria-current]').forEach(node=>node.removeAttribute('aria-current'));const tab=document.querySelector(path.startsWith('/surface/git')?'[hx-get="/surface/git"]':'[hx-get="/surface/blog"]');if(tab)tab.setAttribute('aria-current','page')}});
document.addEventListener('click',async event=>{const trigger=event.target.closest('[data-workspace-mail]');if(!trigger)return;const slot=document.querySelector('#surfaceSlot');if(!slot){location.href='https://dash.edgerun.tech/#mail';return}event.preventDefault();const response=await fetch('/surface/mail',{headers:workspaceHeaders('/surface/mail')});if(!response.ok)return;slot.outerHTML=await response.text();if(location.hash!=='#mail')history.pushState(null,'','#mail');document.querySelectorAll('.dash-tab[aria-current]').forEach(node=>node.removeAttribute('aria-current'));const mailTab=document.querySelector('[hx-get="/surface/mail"]');if(mailTab)mailTab.setAttribute('aria-current','page')});
document.addEventListener('submit',async event=>{const form=event.target.closest('[data-mail-login]');if(!form)return;event.preventDefault();const data=new FormData(form);sessionStorage.setItem('dashMailAuth',btoa((data.get('username')||'')+':'+(data.get('password')||'')));const slot=document.querySelector('#surfaceSlot');if(!slot)return;const response=await fetch('/surface/mail',{headers:workspaceHeaders('/surface/mail')});if(response.ok)slot.outerHTML=await response.text()});
document.addEventListener('submit',async event=>{const form=event.target.closest('[data-dash-mail-compose]');if(!form)return;event.preventDefault();const slot=document.querySelector('#surfaceSlot');if(!slot)return;const payload={to:form.elements.to.value,subject:form.elements.subject.value,body:form.elements.body.value,attachments:[]};const response=await fetch(form.getAttribute('action'),{method:'POST',headers:{...workspaceHeaders(form.getAttribute('action')),'Content-Type':'application/json'},body:JSON.stringify(payload)});if(response.ok)slot.outerHTML=await response.text()});
"#;

pub const BASE_STYLE: &str = r#"
:root{color-scheme:light dark;--bg:#f7f3eb;--panel:#fffdf8;--text:#1c2430;--muted:#627084;--line:#d8cfc0;--accent:#146c63;--accent-ink:#f4fffb;--accent-2:#8b3f2f;--code:#eee6d8}
:root[data-theme=dark]{--bg:#101418;--panel:#171d22;--text:#f2ede4;--muted:#a5b2bf;--line:#2b353d;--accent:#6fc7b8;--accent-ink:#06201d;--accent-2:#dfa06b;--code:#232b31}
*{box-sizing:border-box}body{--topbar-h:76px;--footer-h:68px;min-height:100vh;display:flex;flex-direction:column;margin:0;padding:var(--topbar-h) 0 var(--footer-h);background:var(--bg);color:var(--text);font:16px/1.6 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}body>main{flex:1 0 auto}a{color:inherit}:focus-visible{outline:3px solid var(--accent);outline-offset:3px}.skip-link{position:fixed;left:12px;top:-60px;z-index:30;background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:8px 12px}.skip-link:focus{top:12px}.topbar{position:fixed;top:0;left:0;right:0;z-index:20;min-height:var(--topbar-h);display:grid;grid-template-columns:minmax(140px,1fr) minmax(220px,560px) minmax(140px,1fr);gap:14px;align-items:center;padding:12px clamp(14px,3vw,44px);background:color-mix(in srgb,var(--bg) 92%,transparent);border-bottom:1px solid var(--line);backdrop-filter:blur(12px)}.topbar-brand{min-width:0}.topbar-center{min-width:0;justify-self:center;width:100%}.topbar-actions{min-width:0;display:flex;gap:14px;align-items:center;justify-content:flex-end}.brand{font-weight:800;text-decoration:none;white-space:nowrap}.topbar nav{display:flex;gap:14px;align-items:center;justify-content:end}.topbar nav a{color:var(--muted);text-decoration:none}.workspace-action{height:44px;display:inline-flex;align-items:center;gap:8px;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--text);padding:0 12px;cursor:pointer}.workspace-action:hover{border-color:var(--accent);color:var(--accent)}.header-search{position:relative;min-width:0;width:100%}.header-search label{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}.header-search input{width:100%;min-width:0;height:44px;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--text);padding:10px 44px 10px 13px;font:inherit}.header-search input:focus{border-color:var(--accent)}.header-search button,.header-search .search-icon{position:absolute;right:4px;top:4px;width:36px;height:36px;display:grid;place-items:center;border:0;border-radius:6px;background:transparent;color:var(--muted)}.header-search button{cursor:pointer}.header-search .search-icon{pointer-events:none}.header-search button:hover{color:var(--accent);background:color-mix(in srgb,var(--accent) 10%,transparent)}.header-search svg{width:20px;height:20px;fill:none;stroke:currentColor;stroke-width:2;stroke-linecap:round}button,input,select{font:inherit}.hero{padding:64px clamp(18px,4vw,56px) 42px;border-bottom:1px solid var(--line)}.hero h1{margin:0;font-size:clamp(42px,7vw,82px);line-height:.95;letter-spacing:0}.hero p{max-width:760px;color:var(--muted);font-size:19px}.eyebrow{margin:0 0 12px;color:var(--accent);font-weight:800;text-transform:uppercase;font-size:13px;letter-spacing:.08em}.empty{max-width:720px;margin:80px auto;padding:0 18px;color:var(--muted)}.site-footer{position:fixed;left:0;right:0;bottom:0;z-index:20;min-height:var(--footer-h);display:grid;grid-template-columns:max-content minmax(0,1fr) max-content;gap:18px;align-items:center;border-top:1px solid var(--line);padding:16px clamp(18px,4vw,56px);background:color-mix(in srgb,var(--bg) 92%,transparent);backdrop-filter:blur(12px);color:var(--muted)}.site-footer nav,.language-links{display:flex;gap:14px;align-items:center;flex-wrap:wrap}.site-footer a{text-decoration:none}.site-footer a:hover{color:var(--accent)}.site-footer a[aria-current=page],.site-footer a[aria-current=true]{color:var(--accent);font-weight:800}.footer-primary{font-weight:800}.footer-local{justify-content:center}.language-links{justify-content:end;text-transform:uppercase}.language-links a{font-weight:750}@media(max-width:720px){body{--topbar-h:128px;--footer-h:108px}.topbar{grid-template-columns:minmax(0,1fr) max-content}.topbar-center{grid-column:1/-1;grid-row:2;max-width:none}.topbar-actions{grid-column:2;grid-row:1}.workspace-action span+span{display:none}.site-footer{grid-template-columns:1fr;gap:8px}.footer-local,.language-links{justify-content:flex-start}}
"#;
