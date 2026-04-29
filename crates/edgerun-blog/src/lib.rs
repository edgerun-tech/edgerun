//! Host-only static blog handler served from a Git checkout.
//!
//! This crate intentionally has no non-Edgerun dependencies and no build step.
//! It scans Markdown and HTML files from a working tree at request time, renders
//! Markdown through a small local renderer, and serves a client-side search
//! index plus a light/dark UI.

#![cfg_attr(target_os = "none", no_std)]

extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
compile_error!("edgerun-blog is host-only because it serves files from a Git checkout");

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use edgerun_http::{Handler, Request, Response, StatusCode};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug)]
pub struct BlogConfig {
    pub root: PathBuf,
    pub bind_addr: String,
    pub title: String,
    pub description: String,
    pub base_url: String,
}

impl BlogConfig {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            bind_addr: "127.0.0.1:8088".to_string(),
            title: "Edgerun Blog".to_string(),
            description: "Notes from the Edgerun project.".to_string(),
            base_url: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BlogHandler {
    config: BlogConfig,
}

#[derive(Clone, Debug)]
pub struct Post {
    pub title: String,
    pub path: String,
    pub source_path: PathBuf,
    pub summary: String,
    pub date: String,
    pub tags: Vec<String>,
    pub body: String,
    pub html: String,
}

impl BlogHandler {
    pub fn new(config: BlogConfig) -> Self {
        Self { config }
    }

    pub fn handle_sync(&self, request: Request) -> Response {
        let target = request.uri().request_target();
        let path = target.split('?').next().unwrap_or("/");

        match path {
            "/" | "/index.html" => self.index_response(),
            "/favicon.svg" => favicon_response(),
            "/robots.txt" => self.robots_response(),
            "/sitemap.xml" => self.sitemap_response(),
            "/site.webmanifest" => manifest_response(&self.config),
            "/style.css" => css_response(),
            "/app.js" => js_response(),
            "/search.json" => self.search_response(),
            "/feed.xml" => self.feed_response(),
            _ if path.starts_with("/posts/") => self.post_response(path),
            _ => not_found_response(&self.config.title),
        }
    }

    fn index_response(&self) -> Response {
        match load_posts(&self.config.root) {
            Ok(posts) => Response::html(StatusCode::OK, &render_index(&self.config, &posts))
                .with_header("Cache-Control", "no-store")
                .with_header("X-Content-Type-Options", "nosniff"),
            Err(error) => server_error(error),
        }
    }

    fn post_response(&self, route: &str) -> Response {
        match load_posts(&self.config.root) {
            Ok(posts) => {
                let slug = route
                    .trim_start_matches("/posts/")
                    .trim_end_matches('/')
                    .trim_end_matches(".html");
                if let Some(post) = posts.iter().find(|post| post.path == slug) {
                    Response::html(StatusCode::OK, &render_post(&self.config, post, &posts))
                        .with_header("Cache-Control", "no-store")
                        .with_header("X-Content-Type-Options", "nosniff")
                } else {
                    not_found_response(&self.config.title)
                }
            }
            Err(error) => server_error(error),
        }
    }

    fn search_response(&self) -> Response {
        match load_posts(&self.config.root) {
            Ok(posts) => Response::json(StatusCode::OK, &render_search_json(&posts))
                .with_header("Cache-Control", "no-store")
                .with_header("X-Content-Type-Options", "nosniff"),
            Err(error) => server_error(error),
        }
    }

    fn feed_response(&self) -> Response {
        match load_posts(&self.config.root) {
            Ok(posts) => Response::new(StatusCode::OK)
                .with_header("Content-Type", "application/atom+xml; charset=utf-8")
                .with_header("Cache-Control", "no-store")
                .with_header("X-Content-Type-Options", "nosniff")
                .with_body(render_feed(&self.config, &posts)),
            Err(error) => server_error(error),
        }
    }

    fn robots_response(&self) -> Response {
        Response::new(StatusCode::OK)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_header("Cache-Control", "public, max-age=300")
            .with_header("X-Content-Type-Options", "nosniff")
            .with_body(render_robots(&self.config))
    }

    fn sitemap_response(&self) -> Response {
        match load_posts(&self.config.root) {
            Ok(posts) => Response::new(StatusCode::OK)
                .with_header("Content-Type", "application/xml; charset=utf-8")
                .with_header("Cache-Control", "public, max-age=300")
                .with_header("X-Content-Type-Options", "nosniff")
                .with_body(render_sitemap(&self.config, &posts)),
            Err(error) => server_error(error),
        }
    }
}

impl Handler for BlogHandler {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move { self.handle_sync(request) })
    }
}

pub fn start_blog(config: BlogConfig) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>> {
    Box::pin(async move {
        let bind_addr = config.bind_addr.clone();
        let handler = BlogHandler::new(config);
        let server = edgerun_http::HttpServer::new(handler)
            .bind(bind_addr.clone())
            .await
            .map_err(to_io_error)?;
        edgerun_log::info!("edgerun-blog: listening on {}", bind_addr);
        server.serve().await.map_err(to_io_error)
    })
}

pub fn load_posts(root: &Path) -> io::Result<Vec<Post>> {
    let mut files = Vec::new();
    collect_content_files(root, root, &mut files)?;
    let mut posts = Vec::new();
    for path in files {
        if let Some(post) = load_post(root, &path)? {
            posts.push(post);
        }
    }
    posts.sort_by(|a, b| {
        b.date
            .cmp(&a.date)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    Ok(posts)
}

fn collect_content_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_content_files(root, &path, files)?;
        } else if is_content_file(&path) && safe_relative(root, &path).is_some() {
            files.push(path);
        }
    }
    Ok(())
}

fn load_post(root: &Path, source_path: &Path) -> io::Result<Option<Post>> {
    let raw = fs::read_to_string(source_path)?;
    let rel = match safe_relative(root, source_path) {
        Some(rel) => rel,
        None => return Ok(None),
    };
    let (front, body) = parse_front_matter(&raw);
    let fallback_title = rel
        .file_stem()
        .and_then(|name| name.to_str())
        .map(title_from_slug)
        .unwrap_or_else(|| "Untitled".to_string());
    let title = front_value(front, "title")
        .unwrap_or_else(|| first_heading(body).unwrap_or(fallback_title));
    let date = front_value(front, "date").unwrap_or_else(|| date_from_path(&rel));
    let tags = front_list(front, "tags");
    let summary = front_value(front, "summary").unwrap_or_else(|| summarize(body));
    let path = slug_for_path(&rel);
    let html = if source_path.extension().and_then(|ext| ext.to_str()) == Some("html") {
        body.to_string()
    } else {
        markdown_to_html(body)
    };

    Ok(Some(Post {
        title,
        path,
        source_path: source_path.to_path_buf(),
        summary,
        date,
        tags,
        body: strip_markdown(body),
        html,
    }))
}

fn render_index(config: &BlogConfig, posts: &[Post]) -> String {
    let mut cards = String::new();
    for post in posts {
        cards.push_str(&format!(
            "<article class=\"post-card\" data-search=\"{}\"><a href=\"/posts/{}.html\"><span class=\"date\">{}</span><h2>{}</h2><p>{}</p><div class=\"tags\">{}</div></a></article>",
            escape_attr(&search_blob(post)),
            escape_attr(&post.path),
            escape_html(&post.date),
            escape_html(&post.title),
            escape_html(&post.summary),
            render_tags(&post.tags)
        ));
    }

    page_shell(
        config,
        &PageMeta::index(config),
        &format!(
            "<section class=\"hero\"><div><p class=\"eyebrow\">Static from Git</p><h1>{}</h1><p>{}</p></div><form class=\"search-panel\" role=\"search\"><label for=\"search\">Search</label><input id=\"search\" type=\"search\" placeholder=\"Search posts, tags, and text\" autocomplete=\"off\" aria-describedby=\"search-count\"><p id=\"search-count\">{} posts</p></form></section><main id=\"content\" class=\"layout\" tabindex=\"-1\"><aside aria-label=\"Post topics\"><h2>Topics</h2>{}</aside><section id=\"posts\" class=\"posts\" aria-label=\"Posts\">{}</section></main>",
            escape_html(&config.title),
            escape_html(&config.description),
            posts.len(),
            render_topic_list(posts),
            cards
        ),
    )
}

fn render_post(config: &BlogConfig, post: &Post, posts: &[Post]) -> String {
    let nav = posts
        .iter()
        .take(8)
        .map(|item| {
            format!(
                "<a href=\"/posts/{}.html\">{}</a>",
                escape_attr(&item.path),
                escape_html(&item.title)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    page_shell(
        config,
        &PageMeta::post(config, post),
        &format!(
            "<main id=\"content\" class=\"article-layout\" tabindex=\"-1\"><article class=\"article\" aria-labelledby=\"post-title\"><a class=\"back\" href=\"/\">Back to posts</a><p class=\"date\">{}</p><h1 id=\"post-title\">{}</h1><p class=\"summary\">{}</p><div class=\"tags\">{}</div><div class=\"content\">{}</div></article><aside aria-label=\"Recent posts\"><h2>Recent</h2><nav class=\"recent\" aria-label=\"Recent posts\">{}</nav></aside></main>",
            escape_html(&post.date),
            escape_html(&post.title),
            escape_html(&post.summary),
            render_tags(&post.tags),
            post.html,
            nav
        ),
    )
}

struct PageMeta {
    title: String,
    description: String,
    canonical: String,
    page_type: &'static str,
    published_time: Option<String>,
    noindex: bool,
}

impl PageMeta {
    fn index(config: &BlogConfig) -> Self {
        Self {
            title: config.title.clone(),
            description: config.description.clone(),
            canonical: absolute_url(config, "/"),
            page_type: "website",
            published_time: None,
            noindex: false,
        }
    }

    fn post(config: &BlogConfig, post: &Post) -> Self {
        Self {
            title: post.title.clone(),
            description: post.summary.clone(),
            canonical: absolute_url(config, &format!("/posts/{}.html", post.path)),
            page_type: "article",
            published_time: if post.date.is_empty() {
                None
            } else {
                Some(atom_date(&post.date))
            },
            noindex: false,
        }
    }

    fn not_found(title: &str) -> Self {
        Self {
            title: "Not found".to_string(),
            description: "The requested page does not exist.".to_string(),
            canonical: String::new(),
            page_type: "website",
            published_time: None,
            noindex: true,
        }
        .with_site_title(title)
    }

    fn with_site_title(mut self, site_title: &str) -> Self {
        if !site_title.is_empty() && self.title != site_title {
            self.title = format!("{} | {}", self.title, site_title);
        }
        self
    }
}

fn page_shell(config: &BlogConfig, meta: &PageMeta, body: &str) -> String {
    let mut head = format!(
        "<meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"color-scheme\" content=\"light dark\"><title>{}</title><meta name=\"description\" content=\"{}\">",
        escape_html(&meta.title),
        escape_attr(&meta.description)
    );
    if meta.noindex {
        head.push_str("<meta name=\"robots\" content=\"noindex,nofollow\">");
    }
    if !meta.canonical.is_empty() {
        head.push_str(&format!(
            "<link rel=\"canonical\" href=\"{}\">",
            escape_attr(&meta.canonical)
        ));
        head.push_str(&format!(
            "<meta property=\"og:url\" content=\"{}\">",
            escape_attr(&meta.canonical)
        ));
    }
    head.push_str(&format!(
        "<meta property=\"og:site_name\" content=\"{}\"><meta property=\"og:title\" content=\"{}\"><meta property=\"og:description\" content=\"{}\"><meta property=\"og:type\" content=\"{}\"><meta name=\"twitter:card\" content=\"summary\"><meta name=\"twitter:title\" content=\"{}\"><meta name=\"twitter:description\" content=\"{}\">",
        escape_attr(&config.title),
        escape_attr(&meta.title),
        escape_attr(&meta.description),
        meta.page_type,
        escape_attr(&meta.title),
        escape_attr(&meta.description)
    ));
    if let Some(published_time) = meta.published_time.as_deref() {
        head.push_str(&format!(
            "<meta property=\"article:published_time\" content=\"{}\">",
            escape_attr(published_time)
        ));
    }
    head.push_str("<link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\"><link rel=\"manifest\" href=\"/site.webmanifest\"><link rel=\"alternate\" type=\"application/atom+xml\" href=\"/feed.xml\"><link rel=\"stylesheet\" href=\"/style.css\">");
    format!(
        "<!doctype html><html lang=\"en\"><head>{}</head><body><a class=\"skip-link\" href=\"#content\">Skip to content</a><header class=\"topbar\"><a class=\"brand\" href=\"/\" aria-label=\"{} home\">{}</a><nav aria-label=\"Primary\"><a href=\"/feed.xml\">Feed</a><er-theme-toggle></er-theme-toggle></nav></header>{}<script src=\"/app.js\"></script></body></html>",
        head,
        escape_attr(&config.title),
        escape_html(&config.title),
        body
    )
}

fn render_search_json(posts: &[Post]) -> String {
    let mut out = String::from("[");
    for (index, post) in posts.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"title\":\"{}\",\"url\":\"/posts/{}.html\",\"date\":\"{}\",\"summary\":\"{}\",\"tags\":[{}],\"text\":\"{}\"}}",
            escape_json(&post.title),
            escape_json(&post.path),
            escape_json(&post.date),
            escape_json(&post.summary),
            post.tags.iter().map(|tag| format!("\"{}\"", escape_json(tag))).collect::<Vec<_>>().join(","),
            escape_json(&post.body)
        ));
    }
    out.push(']');
    out
}

fn render_feed(config: &BlogConfig, posts: &[Post]) -> String {
    let base = config.base_url.trim_end_matches('/');
    let mut entries = String::new();
    for post in posts.iter().take(20) {
        let url = format!("{base}/posts/{}.html", post.path);
        entries.push_str(&format!(
            "<entry><title>{}</title><link href=\"{}\"/><id>{}</id><updated>{}</updated><summary>{}</summary></entry>",
            escape_html(&post.title),
            escape_attr(&url),
            escape_html(&url),
            atom_date(&post.date),
            escape_html(&post.summary)
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><feed xmlns=\"http://www.w3.org/2005/Atom\"><title>{}</title><id>{}</id><updated>{}</updated>{}</feed>",
        escape_html(&config.title),
        escape_html(if base.is_empty() { "edgerun-blog" } else { base }),
        posts.first().map(|post| atom_date(&post.date)).unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string()),
        entries
    )
}

fn render_robots(config: &BlogConfig) -> String {
    let sitemap = absolute_url(config, "/sitemap.xml");
    if sitemap.is_empty() {
        "User-agent: *\nAllow: /\n".to_string()
    } else {
        format!("User-agent: *\nAllow: /\nSitemap: {sitemap}\n")
    }
}

fn render_sitemap(config: &BlogConfig, posts: &[Post]) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    xml.push_str(&format!(
        "  <url><loc>{}</loc></url>\n",
        escape_html(&absolute_url(config, "/"))
    ));
    for post in posts {
        xml.push_str("  <url>");
        xml.push_str(&format!(
            "<loc>{}</loc>",
            escape_html(&absolute_url(config, &format!("/posts/{}.html", post.path)))
        ));
        if !post.date.is_empty() {
            xml.push_str(&format!("<lastmod>{}</lastmod>", escape_html(&post.date)));
        }
        xml.push_str("</url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

fn absolute_url(config: &BlogConfig, path: &str) -> String {
    let base = config.base_url.trim_end_matches('/');
    if base.is_empty() {
        path.to_string()
    } else if path.starts_with('/') {
        format!("{base}{path}")
    } else {
        format!("{base}/{path}")
    }
}

fn markdown_to_html(input: &str) -> String {
    let mut html = String::new();
    let mut paragraph = String::new();
    let mut in_code = false;
    let mut code_has_figure = false;
    let mut in_ul = false;
    let mut in_ol = false;
    for line in input.lines() {
        let trimmed = line.trim_end();
        if trimmed.starts_with("```") {
            flush_blocks(&mut html, &mut paragraph, &mut in_ul, &mut in_ol);
            if in_code {
                html.push_str("</code></pre>");
                if code_has_figure {
                    html.push_str("</figure>");
                }
                in_code = false;
                code_has_figure = false;
            } else {
                let info = trimmed.trim_start_matches("```").trim();
                code_has_figure = !info.is_empty();
                html.push_str(&code_block_open(info));
                in_code = true;
            }
            continue;
        }
        if in_code {
            html.push_str(&escape_html(trimmed));
            html.push('\n');
            continue;
        }
        if trimmed.is_empty() {
            flush_blocks(&mut html, &mut paragraph, &mut in_ul, &mut in_ol);
            continue;
        }
        if let Some((level, text)) = heading(trimmed) {
            flush_blocks(&mut html, &mut paragraph, &mut in_ul, &mut in_ol);
            html.push_str(&format!("<h{level}>{}</h{level}>", inline_markdown(text)));
        } else if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush_paragraph(&mut html, &mut paragraph);
            if !in_ul {
                close_ol(&mut html, &mut in_ol);
                html.push_str("<ul>");
                in_ul = true;
            }
            html.push_str(&format!("<li>{}</li>", inline_markdown(item)));
        } else if let Some(item) = ordered_item(trimmed) {
            flush_paragraph(&mut html, &mut paragraph);
            if !in_ol {
                close_ul(&mut html, &mut in_ul);
                html.push_str("<ol>");
                in_ol = true;
            }
            html.push_str(&format!("<li>{}</li>", inline_markdown(item)));
        } else if trimmed.starts_with('>') {
            flush_blocks(&mut html, &mut paragraph, &mut in_ul, &mut in_ol);
            html.push_str(&format!(
                "<blockquote>{}</blockquote>",
                inline_markdown(trimmed.trim_start_matches('>').trim())
            ));
        } else {
            close_ul(&mut html, &mut in_ul);
            close_ol(&mut html, &mut in_ol);
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(trimmed);
        }
    }
    flush_blocks(&mut html, &mut paragraph, &mut in_ul, &mut in_ol);
    if in_code {
        html.push_str("</code></pre>");
        if code_has_figure {
            html.push_str("</figure>");
        }
    }
    html
}

fn code_block_open(info: &str) -> String {
    if info.is_empty() {
        return "<pre><code>".to_string();
    }
    let reference = CodeReference::parse(info);
    let mut html = String::from("<figure class=\"code-ref\">");
    html.push_str(&reference.caption_html());
    html.push_str("<pre><code");
    if let Some(language) = reference.language.as_deref() {
        html.push_str(&format!(" class=\"language-{}\"", escape_attr(language)));
    }
    html.push('>');
    html
}

#[derive(Default)]
struct CodeReference {
    language: Option<String>,
    path: Option<String>,
    commit: Option<String>,
    lines: Option<String>,
}

impl CodeReference {
    fn parse(info: &str) -> Self {
        let mut reference = Self::default();
        for token in info.split_whitespace() {
            if let Some((key, value)) = token.split_once('=') {
                let value = trim_quotes(value).to_string();
                match key {
                    "path" => reference.path = Some(value),
                    "commit" | "hash" => reference.commit = Some(value),
                    "lines" | "line" => reference.lines = Some(value),
                    _ => {}
                }
            } else if reference.language.is_none() {
                reference.language = Some(token.to_string());
            }
        }
        reference
    }

    fn caption_html(&self) -> String {
        let Some(path) = self.path.as_deref() else {
            let label = self.language.as_deref().unwrap_or("code");
            return format!("<figcaption>{}</figcaption>", escape_html(label));
        };
        let commit = self.commit.as_deref().unwrap_or("working-tree");
        let mut label = format!("{path} @ {commit}");
        if let Some(lines) = self.lines.as_deref() {
            label.push_str(&format!(":{lines}"));
        }
        if self.commit.is_some() {
            format!(
                "<figcaption><a href=\"{}\">{}</a></figcaption>",
                escape_attr(&self.url(path)),
                escape_html(&label)
            )
        } else {
            format!("<figcaption>{}</figcaption>", escape_html(&label))
        }
    }

    fn url(&self, path: &str) -> String {
        let commit = self.commit.as_deref().unwrap_or("HEAD");
        let mut url = format!(
            "https://git.edgerun.tech/edgerun_core/src/{}/{}",
            commit,
            path.trim_start_matches('/')
        );
        if let Some(lines) = self.lines.as_deref() {
            let first = lines.split(['-', ':']).next().unwrap_or(lines);
            if !first.is_empty() {
                url.push_str("#L");
                url.push_str(first);
            }
        }
        url
    }
}

fn inline_markdown(input: &str) -> String {
    let mut out = String::new();
    let mut rest = input;
    while let Some(start) = rest.find('[') {
        out.push_str(&escape_html(&rest[..start]));
        if let Some(mid) = rest[start..].find("](") {
            if let Some(end) = rest[start + mid + 2..].find(')') {
                let label = &rest[start + 1..start + mid];
                let href = &rest[start + mid + 2..start + mid + 2 + end];
                out.push_str(&format!(
                    "<a href=\"{}\">{}</a>",
                    escape_attr(href),
                    escape_html(label)
                ));
                rest = &rest[start + mid + 3 + end..];
                continue;
            }
        }
        out.push_str(&escape_html(&rest[start..start + 1]));
        rest = &rest[start + 1..];
    }
    out.push_str(&escape_html(rest));
    out
}

fn flush_blocks(html: &mut String, paragraph: &mut String, in_ul: &mut bool, in_ol: &mut bool) {
    flush_paragraph(html, paragraph);
    close_ul(html, in_ul);
    close_ol(html, in_ol);
}

fn flush_paragraph(html: &mut String, paragraph: &mut String) {
    if !paragraph.is_empty() {
        html.push_str(&format!("<p>{}</p>", inline_markdown(paragraph)));
        paragraph.clear();
    }
}

fn close_ul(html: &mut String, in_ul: &mut bool) {
    if *in_ul {
        html.push_str("</ul>");
        *in_ul = false;
    }
}

fn close_ol(html: &mut String, in_ol: &mut bool) {
    if *in_ol {
        html.push_str("</ol>");
        *in_ol = false;
    }
}

fn parse_front_matter(raw: &str) -> (&str, &str) {
    let rest = raw
        .strip_prefix("---\n")
        .or_else(|| raw.strip_prefix("---\r\n"));
    let Some(rest) = rest else {
        return ("", raw);
    };
    if let Some(end) = rest.find("\n---\n") {
        (&rest[..end], &rest[end + 5..])
    } else if let Some(end) = rest.find("\r\n---\r\n") {
        (&rest[..end], &rest[end + 7..])
    } else {
        ("", raw)
    }
}

fn front_value(front: &str, key: &str) -> Option<String> {
    for line in front.lines() {
        let (name, value) = line.split_once(':')?;
        if name.trim() == key {
            return Some(trim_quotes(value.trim()).to_string());
        }
    }
    None
}

fn front_list(front: &str, key: &str) -> Vec<String> {
    front_value(front, key)
        .map(|value| {
            value
                .trim_matches(['[', ']'])
                .split(',')
                .map(|tag| trim_quotes(tag.trim()).to_string())
                .filter(|tag| !tag.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn trim_quotes(value: &str) -> &str {
    value.trim_matches('"').trim_matches('\'')
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|ch| *ch == '#').count();
    if (1..=6).contains(&level) && line.as_bytes().get(level) == Some(&b' ') {
        Some((level, line[level + 1..].trim()))
    } else {
        None
    }
}

fn ordered_item(line: &str) -> Option<&str> {
    let dot = line.find('.')?;
    if dot > 0 && line[..dot].chars().all(|ch| ch.is_ascii_digit()) {
        line[dot + 1..]
            .trim()
            .strip_prefix(' ')
            .or(Some(line[dot + 1..].trim()))
    } else {
        None
    }
}

fn first_heading(input: &str) -> Option<String> {
    input
        .lines()
        .find_map(|line| heading(line.trim()).map(|(_, text)| text.to_string()))
}

fn summarize(input: &str) -> String {
    strip_markdown(input)
        .split_whitespace()
        .take(28)
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_markdown(input: &str) -> String {
    input
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && !line.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join(" ")
        .replace(['`', '*', '_', '[', ']', '(', ')', '>', '#'], "")
}

fn safe_relative(root: &Path, path: &Path) -> Option<PathBuf> {
    let rel = path.strip_prefix(root).ok()?;
    if rel
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        None
    } else {
        Some(rel.to_path_buf())
    }
}

fn slug_for_path(path: &Path) -> String {
    let without_ext = path.with_extension("");
    without_ext
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => name.to_str().map(slugify),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn title_from_slug(input: &str) -> String {
    input
        .replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            dash = false;
        } else if !dash {
            out.push('-');
            dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn date_from_path(path: &Path) -> String {
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if name.len() >= 10
        && name.as_bytes()[0..10]
            .iter()
            .enumerate()
            .all(|(i, b)| matches!((i, *b), (4, b'-') | (7, b'-')) || b.is_ascii_digit())
    {
        name[..10].to_string()
    } else {
        String::new()
    }
}

fn atom_date(date: &str) -> String {
    if date.len() == 10 {
        format!("{date}T00:00:00Z")
    } else {
        "1970-01-01T00:00:00Z".to_string()
    }
}

fn render_tags(tags: &[String]) -> String {
    tags.iter()
        .map(|tag| format!("<span>{}</span>", escape_html(tag)))
        .collect::<Vec<_>>()
        .join("")
}

fn render_topic_list(posts: &[Post]) -> String {
    let mut tags = Vec::<String>::new();
    for post in posts {
        for tag in &post.tags {
            if !tags.iter().any(|seen| seen == tag) {
                tags.push(tag.clone());
            }
        }
    }
    if tags.is_empty() {
        return "<p class=\"muted\">No tags yet.</p>".to_string();
    }
    tags.sort();
    format!(
        "<div class=\"topic-list\">{}</div>",
        tags.iter()
            .map(|tag| format!(
                "<button type=\"button\" data-topic=\"{}\">{}</button>",
                escape_attr(tag),
                escape_html(tag)
            ))
            .collect::<Vec<_>>()
            .join("")
    )
}

fn search_blob(post: &Post) -> String {
    format!(
        "{} {} {} {}",
        post.title,
        post.summary,
        post.tags.join(" "),
        post.body
    )
    .to_lowercase()
}

fn is_content_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("md") | Some("markdown") | Some("html")
    )
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(input: &str) -> String {
    escape_html(input).replace('"', "&quot;")
}

fn escape_json(input: &str) -> String {
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

fn not_found_response(title: &str) -> Response {
    let config = BlogConfig {
        root: PathBuf::new(),
        bind_addr: String::new(),
        title: title.to_string(),
        description: "Not found".to_string(),
        base_url: String::new(),
    };
    Response::html(
        StatusCode::NOT_FOUND,
        &page_shell(
            &config,
            &PageMeta::not_found(title),
            "<main id=\"content\" class=\"empty\" tabindex=\"-1\"><h1>Not found</h1><p>The requested post does not exist.</p><a href=\"/\">Back to posts</a></main>",
        ),
    )
}

fn server_error(error: io::Error) -> Response {
    Response::text(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("edgerun-blog: {error}"),
    )
}

fn favicon_response() -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "image/svg+xml")
        .with_header("Cache-Control", "public, max-age=86400")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(FAVICON_SVG)
}

fn manifest_response(config: &BlogConfig) -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "application/manifest+json")
        .with_header("Cache-Control", "public, max-age=300")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(format!(
            "{{\"name\":\"{}\",\"short_name\":\"{}\",\"start_url\":\"/\",\"scope\":\"/\",\"display\":\"minimal-ui\",\"background_color\":\"#f7f3eb\",\"theme_color\":\"#146c63\",\"icons\":[{{\"src\":\"/favicon.svg\",\"sizes\":\"any\",\"type\":\"image/svg+xml\"}}]}}",
            escape_json(&config.title),
            escape_json(&config.title)
        ))
}

fn css_response() -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "text/css; charset=utf-8")
        .with_header("Cache-Control", "public, max-age=300")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(STYLE)
}

fn js_response() -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "application/javascript; charset=utf-8")
        .with_header("Cache-Control", "public, max-age=300")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(APP_JS)
}

fn to_io_error(error: edgerun_http::io::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error.to_string())
}

const FAVICON_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text y="76" font-size="76">🧭</text></svg>"#;

const APP_JS: &str = r#"
const root=document.documentElement;
const stored=localStorage.getItem('theme');
if(stored){root.dataset.theme=stored}
if(!customElements.get('er-theme-toggle')){customElements.define('er-theme-toggle',class extends HTMLElement{connectedCallback(){this.attachShadow({mode:'open'}).innerHTML='<style>button{width:36px;height:36px;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--text);cursor:pointer;font:inherit}button:hover{border-color:var(--accent)}</style><button type="button" aria-label="Toggle color theme">◐</button>';this.shadowRoot.querySelector('button').onclick=()=>{const next=root.dataset.theme==='dark'?'light':'dark';root.dataset.theme=next;localStorage.setItem('theme',next)}}})}
const search=document.getElementById('search');
const cards=[...document.querySelectorAll('.post-card')];
const count=document.getElementById('search-count');
function applyFilter(term){const q=term.trim().toLowerCase();let shown=0;for(const card of cards){const ok=!q||card.dataset.search.includes(q);card.hidden=!ok;if(ok)shown++}if(count){count.textContent=shown+' post'+(shown===1?'':'s')}}
if(search){search.addEventListener('input',e=>applyFilter(e.target.value))}
for(const btn of document.querySelectorAll('[data-topic]')){btn.addEventListener('click',()=>{if(search){search.value=btn.dataset.topic;applyFilter(btn.dataset.topic);search.focus()}})}
"#;

const STYLE: &str = r#"
:root{color-scheme:light dark;--bg:#f7f3eb;--panel:#fffdf8;--text:#1c2430;--muted:#627084;--line:#d8cfc0;--accent:#146c63;--accent-ink:#f4fffb;--accent-2:#8b3f2f;--code:#eee6d8}
:root[data-theme=dark]{--bg:#101418;--panel:#171d22;--text:#f2ede4;--muted:#a5b2bf;--line:#2b353d;--accent:#6fc7b8;--accent-ink:#06201d;--accent-2:#dfa06b;--code:#232b31}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:16px/1.6 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}a{color:inherit}.skip-link{position:absolute;left:12px;top:-60px;z-index:10;background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:8px 12px}.skip-link:focus{top:12px}.topbar{position:sticky;top:0;z-index:2;display:flex;justify-content:space-between;align-items:center;padding:14px clamp(18px,4vw,56px);background:color-mix(in srgb,var(--bg) 88%,transparent);border-bottom:1px solid var(--line);backdrop-filter:blur(12px)}.brand{font-weight:800;text-decoration:none}.topbar nav{display:flex;gap:16px;align-items:center}.topbar nav a{color:var(--muted);text-decoration:none}button,input{font:inherit}.hero{display:grid;grid-template-columns:minmax(0,1.25fr) minmax(280px,.75fr);gap:28px;padding:64px clamp(18px,4vw,56px) 42px;border-bottom:1px solid var(--line)}.hero h1{margin:0;font-size:clamp(42px,7vw,82px);line-height:.95;letter-spacing:0}.hero p{max-width:720px;color:var(--muted);font-size:19px}.eyebrow{margin:0 0 12px;color:var(--accent);font-weight:800;text-transform:uppercase;font-size:13px;letter-spacing:.08em}.search-panel{align-self:end;background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:18px}.search-panel label{display:block;font-weight:800;margin-bottom:8px}.search-panel input{width:100%;border:1px solid var(--line);border-radius:8px;background:var(--bg);color:var(--text);padding:12px 13px}.search-panel p{margin:10px 0 0;font-size:14px}.layout,.article-layout{display:grid;grid-template-columns:240px minmax(0,1fr);gap:32px;max-width:1180px;margin:0 auto;padding:34px 18px 80px}aside{color:var(--muted)}aside h2{margin:0 0 12px;color:var(--text);font-size:15px;text-transform:uppercase;letter-spacing:.08em}.topic-list{display:flex;flex-wrap:wrap;gap:8px}.topic-list button{border:1px solid var(--line);background:var(--panel);color:var(--text);border-radius:999px;padding:7px 10px;cursor:pointer}.posts{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px}.post-card{min-height:220px;background:var(--panel);border:1px solid var(--line);border-radius:8px;transition:transform .15s ease,border-color .15s ease}.post-card:hover{transform:translateY(-2px);border-color:var(--accent)}.post-card a{display:flex;min-height:100%;flex-direction:column;padding:22px;text-decoration:none}.date{color:var(--accent-2);font-size:14px;font-weight:750}.post-card h2{margin:12px 0 10px;font-size:24px;line-height:1.15;letter-spacing:0}.post-card p{margin:0 0 20px;color:var(--muted)}.tags{display:flex;gap:7px;flex-wrap:wrap;margin-top:auto}.tags span{border:1px solid var(--line);border-radius:999px;padding:3px 8px;color:var(--muted);font-size:13px}.article{max-width:780px;background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:clamp(22px,5vw,48px)}.article h1{font-size:clamp(34px,5vw,58px);line-height:1;margin:10px 0 14px;letter-spacing:0}.summary{font-size:20px;color:var(--muted)}.back{color:var(--accent);font-weight:800;text-decoration:none}.content{margin-top:32px}.content h1,.content h2,.content h3{line-height:1.15;margin:32px 0 10px;letter-spacing:0}.content p{margin:14px 0}.content pre{overflow:auto;background:var(--code);border-radius:8px;padding:16px}.code-ref{margin:22px 0}.code-ref figcaption{border:1px solid var(--line);border-bottom:0;border-radius:8px 8px 0 0;background:var(--panel);color:var(--muted);font-size:13px;padding:8px 12px}.code-ref figcaption a{color:var(--accent);font-weight:750;text-decoration:none}.code-ref pre{margin:0;border-radius:0 0 8px 8px}.content code{font-family:ui-monospace,SFMono-Regular,Consolas,monospace}.content blockquote{margin:22px 0;padding:4px 0 4px 18px;border-left:4px solid var(--accent);color:var(--muted)}.recent{display:grid;gap:10px}.recent a{color:var(--muted);text-decoration:none}.empty{max-width:720px;margin:80px auto;padding:0 18px}.muted{color:var(--muted)}
@media(max-width:820px){.hero,.layout,.article-layout{grid-template-columns:1fr}.hero{padding-top:42px}.posts{grid-template-columns:1fr}.article{padding:22px}.topbar{position:static}}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_basic_markdown() {
        let html =
            markdown_to_html("# Title\n\nHello [site](https://example.com).\n\n- one\n- two");
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<a href=\"https://example.com\">site</a>"));
        assert!(html.contains("<ul><li>one</li><li>two</li></ul>"));
    }

    #[test]
    fn parses_front_matter_values() {
        let (front, body) = parse_front_matter("---\ntitle: Test\ntags: [one, two]\n---\n# Body");
        assert_eq!(front_value(front, "title").as_deref(), Some("Test"));
        assert_eq!(front_list(front, "tags"), vec!["one", "two"]);
        assert_eq!(body.trim(), "# Body");
    }

    #[test]
    fn builds_nested_slug() {
        assert_eq!(
            slug_for_path(Path::new("posts/2026-04-30 Hello World.md")),
            "posts/2026-04-30-hello-world"
        );
    }

    #[test]
    fn renders_commit_pinned_code_reference() {
        let html = markdown_to_html(
            "```rust path=crates/edgerun-blog/src/lib.rs commit=abc123 lines=10-20\nfn demo() {}\n```",
        );
        assert!(html.contains("class=\"code-ref\""));
        assert!(html.contains("crates/edgerun-blog/src/lib.rs @ abc123:10-20"));
        assert!(html.contains(
            "https://git.edgerun.tech/edgerun_core/src/abc123/crates/edgerun-blog/src/lib.rs#L10"
        ));
    }
}
