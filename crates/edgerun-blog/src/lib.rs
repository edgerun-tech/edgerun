//! Host-only static blog handler served from a Git checkout.
//!
//! This crate intentionally has no non-Edgerun dependencies and no build step.
//! It can scan Markdown and HTML files from a working tree at request time, or
//! render the same deterministic files into an output directory for a Git hook
//! or other host-local publication path.

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
use edgerun_web_ui::{FooterLink, PageShell};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct BlogConfig {
    pub root: PathBuf,
    pub content_dir: PathBuf,
    pub static_root: Option<PathBuf>,
    pub bind_addr: String,
    pub title: String,
    pub description: String,
    pub base_url: String,
}

impl BlogConfig {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            content_dir: PathBuf::from("."),
            static_root: None,
            bind_addr: "127.0.0.1:8088".to_string(),
            title: "EdgeRun Build Log".to_string(),
            description: "Feature-by-feature notes on building Edgerun from its source tree."
                .to_string(),
            base_url: String::new(),
        }
    }

    pub fn content_root(&self) -> PathBuf {
        if self.content_dir.as_os_str().is_empty() || self.content_dir == Path::new(".") {
            self.root.clone()
        } else {
            self.root.join(&self.content_dir)
        }
    }

    fn language_content_root(&self, language: Language) -> PathBuf {
        self.content_root().join(language.code)
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
    pub author: String,
    pub tags: Vec<String>,
    pub body: String,
    pub html: String,
    pub missing_front_matter: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Page {
    pub title: String,
    pub source_path: PathBuf,
    pub summary: String,
    pub body: String,
    pub html: String,
    pub missing_front_matter: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct GitStats {
    commit_count: Option<String>,
    last_commit_epoch: Option<u64>,
    last_commit_message: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Language {
    code: &'static str,
    html_lang: &'static str,
    og_locale: &'static str,
    path_prefix: &'static str,
    title: &'static str,
    description: &'static str,
    hero_eyebrow: &'static str,
    about_label: &'static str,
    topics_label: &'static str,
    all_label: &'static str,
    repo_commits_label: &'static str,
    last_commit_label: &'static str,
    back_label: &'static str,
    recent_label: &'static str,
    related_label: &'static str,
    search_label: &'static str,
    feed_label: &'static str,
}

const LANGUAGES: &[Language] = &[
    Language {
        code: "en",
        html_lang: "en",
        og_locale: "en_US",
        path_prefix: "",
        title: "EdgeRun Build Log",
        description: "Feature-by-feature notes on building Edgerun from its source tree.",
        hero_eyebrow: "Behind the scenes of Git",
        about_label: "About Edgerun",
        topics_label: "Topics",
        all_label: "All",
        repo_commits_label: "Repo commits",
        last_commit_label: "Last commit",
        back_label: "Back to posts",
        recent_label: "Recent",
        related_label: "Related articles",
        search_label: "Search posts",
        feed_label: "Feed",
    },
    Language {
        code: "th",
        html_lang: "th",
        og_locale: "th_TH",
        path_prefix: "/th",
        title: "บันทึกการสร้าง EdgeRun",
        description: "บันทึกทีละฟีเจอร์เกี่ยวกับการสร้าง Edgerun จากซอร์สโค้ด.",
        hero_eyebrow: "เบื้องหลัง Git",
        about_label: "เกี่ยวกับ Edgerun",
        topics_label: "หัวข้อ",
        all_label: "ทั้งหมด",
        repo_commits_label: "คอมมิตในรีโป",
        last_commit_label: "คอมมิตล่าสุด",
        back_label: "กลับไปที่โพสต์",
        recent_label: "ล่าสุด",
        related_label: "บทความที่เกี่ยวข้อง",
        search_label: "ค้นหาโพสต์",
        feed_label: "ฟีด",
    },
    Language {
        code: "et",
        html_lang: "et",
        og_locale: "et_EE",
        path_prefix: "/et",
        title: "EdgeRuni Ehituslogi",
        description: "Funktsioonide kaupa märkmed Edgeruni ehitamisest lähtekoodist.",
        hero_eyebrow: "Giti telgitagused",
        about_label: "Edgerunist",
        topics_label: "Teemad",
        all_label: "Kõik",
        repo_commits_label: "Repo commit'id",
        last_commit_label: "Viimane commit",
        back_label: "Tagasi postituste juurde",
        recent_label: "Viimased",
        related_label: "Seotud artiklid",
        search_label: "Otsi postitusi",
        feed_label: "Voog",
    },
];

fn default_language() -> Language {
    LANGUAGES[0]
}

impl BlogHandler {
    pub fn new(config: BlogConfig) -> Self {
        Self { config }
    }

    pub fn handle_sync(&self, request: Request) -> Response {
        let target = request.uri().request_target();
        let path = target.split('?').next().unwrap_or("/");

        if let Some(static_root) = self.config.static_root.as_deref() {
            if let Some(response) = static_file_response(static_root, path) {
                return response;
            }
        }

        let (language, localized_path) = route_language(path);

        match localized_path.as_str() {
            "/" | "/index.html" => self.index_response(language),
            "/about.html" => self.about_response(language),
            "/favicon.svg" => favicon_response(),
            "/robots.txt" => self.robots_response(),
            "/sitemap.xml" => self.sitemap_response(),
            "/opensearch.xml" => opensearch_response(&self.config),
            "/site.webmanifest" => manifest_response(&self.config),
            "/style.css" => css_response(),
            "/app.js" => js_response(),
            "/search.json" => self.search_response(language),
            "/feed.xml" => self.feed_response(language),
            _ if localized_path.starts_with("/posts/") => {
                self.post_response(language, &localized_path)
            }
            _ => not_found_response(&self.config.title),
        }
    }

    pub fn render_dash_content(&self, surface_path: &str) -> io::Result<String> {
        let tail = surface_path
            .strip_prefix("/surface/blog")
            .unwrap_or(surface_path);
        let path = if tail.is_empty() { "/" } else { tail };
        let (language, localized_path) = route_language(path);
        let root = self.config.language_content_root(language);
        match localized_path.as_str() {
            "/" | "/index.html" => {
                render_dash_blog_index(&self.config, language, &load_posts(&root)?)
            }
            "/about.html" | "/about" => {
                let Some(page) = load_about_page(&root)? else {
                    return Ok(
                        "<p class=\"empty\">About page is not published yet.</p>".to_string()
                    );
                };
                Ok(render_dash_about(language, &page))
            }
            "/feed.xml" | "/feed" => Ok(render_dash_feed(&self.config, language)),
            _ if localized_path.starts_with("/posts/") => {
                let posts = load_posts(&root)?;
                let slug = localized_path
                    .trim_start_matches("/posts/")
                    .trim_end_matches('/')
                    .trim_end_matches(".html");
                if let Some(post) = posts.iter().find(|post| post.path == slug) {
                    Ok(render_dash_post(&self.config, language, post, &posts))
                } else {
                    Ok("<p class=\"empty\">Post not found.</p>".to_string())
                }
            }
            _ => render_dash_blog_index(&self.config, language, &load_posts(&root)?),
        }
    }

    fn index_response(&self, language: Language) -> Response {
        match load_posts(&self.config.language_content_root(language)) {
            Ok(posts) => Response::html(
                StatusCode::OK,
                &render_index(&self.config, language, &posts),
            )
            .with_header("Cache-Control", "no-store")
            .with_header("X-Content-Type-Options", "nosniff"),
            Err(error) => server_error(error),
        }
    }

    fn post_response(&self, language: Language, route: &str) -> Response {
        match load_posts(&self.config.language_content_root(language)) {
            Ok(posts) => {
                let slug = route
                    .trim_start_matches("/posts/")
                    .trim_end_matches('/')
                    .trim_end_matches(".html");
                if let Some(post) = posts.iter().find(|post| post.path == slug) {
                    Response::html(
                        StatusCode::OK,
                        &render_post(&self.config, language, post, &posts),
                    )
                    .with_header("Cache-Control", "no-store")
                    .with_header("X-Content-Type-Options", "nosniff")
                } else {
                    not_found_response(&self.config.title)
                }
            }
            Err(error) => server_error(error),
        }
    }

    fn about_response(&self, language: Language) -> Response {
        match load_about_page(&self.config.language_content_root(language)) {
            Ok(Some(page)) => {
                Response::html(StatusCode::OK, &render_about(&self.config, language, &page))
                    .with_header("Cache-Control", "no-store")
                    .with_header("X-Content-Type-Options", "nosniff")
            }
            Ok(None) => not_found_response(&self.config.title),
            Err(error) => server_error(error),
        }
    }

    fn search_response(&self, language: Language) -> Response {
        match load_posts(&self.config.language_content_root(language)) {
            Ok(posts) => Response::json(StatusCode::OK, &render_search_json(&posts))
                .with_header("Cache-Control", "no-store")
                .with_header("X-Content-Type-Options", "nosniff"),
            Err(error) => server_error(error),
        }
    }

    fn feed_response(&self, language: Language) -> Response {
        match load_posts(&self.config.language_content_root(language)) {
            Ok(posts) => Response::new(StatusCode::OK)
                .with_header("Content-Type", "application/atom+xml; charset=utf-8")
                .with_header("Cache-Control", "no-store")
                .with_header("X-Content-Type-Options", "nosniff")
                .with_body(render_feed(&self.config, language, &posts)),
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
        match load_language_sites(&self.config) {
            Ok(sites) => Response::new(StatusCode::OK)
                .with_header("Content-Type", "application/xml; charset=utf-8")
                .with_header("Cache-Control", "public, max-age=300")
                .with_header("X-Content-Type-Options", "nosniff")
                .with_body(render_sitemap(&self.config, &sites)),
            Err(error) => server_error(error),
        }
    }
}

fn route_language(path: &str) -> (Language, String) {
    for language in LANGUAGES.iter().copied().skip(1) {
        if path == language.path_prefix || path == format!("{}/", language.path_prefix) {
            return (language, "/".to_string());
        }
        if let Some(rest) = path.strip_prefix(&format!(
            "{}/",
            language.path_prefix.trim_start_matches('/')
        )) {
            return (language, format!("/{rest}"));
        }
        if let Some(rest) = path.strip_prefix(&format!("{}/", language.path_prefix)) {
            return (language, format!("/{rest}"));
        }
    }
    (default_language(), path.to_string())
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

pub fn load_about_page(root: &Path) -> io::Result<Option<Page>> {
    for name in ["about.md", "about.markdown", "about.html"] {
        let path = root.join(name);
        if path.is_file() {
            return load_page(root, &path);
        }
    }
    Ok(None)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerateMode {
    Write,
    Check,
}

pub fn generate_static_site(config: &BlogConfig, output: &Path) -> io::Result<GeneratedSite> {
    generate_static_site_with_mode(config, output, GenerateMode::Write)
}

pub fn check_static_site(config: &BlogConfig, output: &Path) -> io::Result<GeneratedSite> {
    generate_static_site_with_mode(config, output, GenerateMode::Check)
}

fn generate_static_site_with_mode(
    config: &BlogConfig,
    output: &Path,
    mode: GenerateMode,
) -> io::Result<GeneratedSite> {
    let source = config
        .root
        .canonicalize()
        .unwrap_or_else(|_| config.root.clone());
    if output.exists() {
        let destination = output
            .canonicalize()
            .unwrap_or_else(|_| output.to_path_buf());
        if source == destination {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "static output directory must be separate from the post source root",
            ));
        }
    }

    let sites = load_language_sites(config)?;
    validate_language_sites(&sites)?;

    let plan = build_generated_site(config, output, &sites);
    if mode == GenerateMode::Check {
        check_generated_files(output, &plan)?;
    } else {
        write_generated_site(output, &plan)?;
    }

    Ok(GeneratedSite {
        posts: sites.first().map(|site| site.posts.len()).unwrap_or(0),
        files: plan.files.iter().map(|file| file.path.clone()).collect(),
    })
}

#[derive(Clone, Debug)]
struct LanguageSite {
    language: Language,
    posts: Vec<Post>,
    about: Option<Page>,
}

fn load_language_sites(config: &BlogConfig) -> io::Result<Vec<LanguageSite>> {
    let mut sites = Vec::new();
    for language in LANGUAGES.iter().copied() {
        let content_root = config.language_content_root(language);
        let posts = load_posts(&content_root)?;
        let about = load_about_page(&content_root)?;
        validate_content(&posts, about.as_ref())?;
        sites.push(LanguageSite {
            language,
            posts,
            about,
        });
    }
    Ok(sites)
}

fn validate_language_sites(sites: &[LanguageSite]) -> io::Result<()> {
    let mut errors = Vec::new();
    if let Some(default_site) = sites.first() {
        let default_posts = default_site
            .posts
            .iter()
            .map(|post| post.path.as_str())
            .collect::<Vec<_>>();
        let default_has_about = default_site.about.is_some();

        for site in sites.iter().skip(1) {
            for slug in &default_posts {
                if !site.posts.iter().any(|post| post.path == *slug) {
                    errors.push(format!("{} is missing post '{}'", site.language.code, slug));
                }
            }
            for post in &site.posts {
                if !default_posts.iter().any(|slug| *slug == post.path) {
                    errors.push(format!(
                        "{} has post '{}' missing from {}",
                        site.language.code, post.path, default_site.language.code
                    ));
                }
            }
            if default_has_about && site.about.is_none() {
                errors.push(format!("{} is missing about page", site.language.code));
            }
            if !default_has_about && site.about.is_some() {
                errors.push(format!(
                    "{} has about page missing from {}",
                    site.language.code, default_site.language.code
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            errors.join("; "),
        ))
    }
}

#[derive(Clone, Debug)]
struct GeneratedFile {
    path: PathBuf,
    body: Vec<u8>,
}

#[derive(Clone, Debug)]
struct GeneratedPlan {
    files: Vec<GeneratedFile>,
}

fn build_generated_site(
    config: &BlogConfig,
    output: &Path,
    sites: &[LanguageSite],
) -> GeneratedPlan {
    let mut files = Vec::new();
    files.push(generated_file(
        output.join("favicon.svg"),
        edgerun_web_ui::FAVICON_COMPASS_SVG,
    ));
    files.push(generated_file(
        output.join("robots.txt"),
        render_robots(config),
    ));
    files.push(generated_file(
        output.join("sitemap.xml"),
        render_sitemap(config, sites),
    ));
    files.push(generated_file(
        output.join("opensearch.xml"),
        render_opensearch(config),
    ));
    files.push(generated_file(
        output.join("site.webmanifest"),
        render_manifest(config),
    ));
    files.push(generated_file(output.join("style.css"), blog_style()));
    files.push(generated_file(output.join("app.js"), blog_js()));

    for site in sites {
        let root = language_output_root(output, site.language);
        files.push(generated_file(
            root.join("index.html"),
            render_index(config, site.language, &site.posts),
        ));
        files.push(generated_file(
            root.join("feed.xml"),
            render_feed(config, site.language, &site.posts),
        ));
        files.push(generated_file(
            root.join("search.json"),
            render_search_json(&site.posts),
        ));
        for post in &site.posts {
            files.push(generated_file(
                root.join("posts").join(format!("{}.html", post.path)),
                render_post(config, site.language, post, &site.posts),
            ));
        }
        if let Some(about) = site.about.as_ref() {
            files.push(generated_file(
                root.join("about.html"),
                render_about(config, site.language, about),
            ));
        }
    }

    GeneratedPlan { files }
}

fn language_output_root(output: &Path, language: Language) -> PathBuf {
    if language.path_prefix.is_empty() {
        output.to_path_buf()
    } else {
        output.join(language.path_prefix.trim_start_matches('/'))
    }
}

fn generated_file(path: PathBuf, body: impl AsRef<[u8]>) -> GeneratedFile {
    GeneratedFile {
        path,
        body: body.as_ref().to_vec(),
    }
}

fn write_generated_site(output: &Path, plan: &GeneratedPlan) -> io::Result<()> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("site");
    let tmp = parent.join(format!(".{name}.tmp-{}", std::process::id()));
    if tmp.exists() {
        fs::remove_dir_all(&tmp)?;
    }
    fs::create_dir_all(&tmp)?;

    let write_result = (|| -> io::Result<()> {
        for file in &plan.files {
            let relative = file.path.strip_prefix(output).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "generated file escaped output directory",
                )
            })?;
            write_generated_bytes(tmp.join(relative), &file.body)?;
        }
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_dir_all(&tmp);
        return Err(error);
    }

    let old = parent.join(format!(".{name}.old-{}", std::process::id()));
    if old.exists() {
        fs::remove_dir_all(&old)?;
    }
    if output.exists() {
        fs::rename(output, &old)?;
    }
    if let Err(error) = fs::rename(&tmp, output) {
        if old.exists() {
            let _ = fs::rename(&old, output);
        }
        let _ = fs::remove_dir_all(&tmp);
        return Err(error);
    }
    if old.exists() {
        fs::remove_dir_all(old)?;
    }
    Ok(())
}

fn check_generated_files(output: &Path, plan: &GeneratedPlan) -> io::Result<()> {
    let mut stale = Vec::new();
    let expected = plan
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    for file in &plan.files {
        match fs::read(&file.path) {
            Ok(existing) if existing == file.body => {}
            Ok(_) => stale.push(file.path.display().to_string()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                stale.push(file.path.display().to_string())
            }
            Err(error) => return Err(error),
        }
    }
    if output.exists() {
        for path in collect_regular_files(output)? {
            if !expected.iter().any(|expected| expected == &path) {
                stale.push(path.display().to_string());
            }
        }
    }
    if !stale.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "generated output is stale; run edgerun-blog generate. First stale files: {}",
                stale.into_iter().take(5).collect::<Vec<_>>().join(", ")
            ),
        ));
    }
    Ok(())
}

fn collect_regular_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_regular_files_inner(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_regular_files_inner(dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_regular_files_inner(&path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn validate_content(posts: &[Post], about: Option<&Page>) -> io::Result<()> {
    let mut errors = Vec::new();
    validate_posts(posts, &mut errors);
    if let Some(about) = about {
        validate_about_page(about, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            errors.join("; "),
        ))
    }
}

fn validate_posts(posts: &[Post], errors: &mut Vec<String>) {
    for (index, post) in posts.iter().enumerate() {
        if !post.missing_front_matter.is_empty() {
            errors.push(format!(
                "{} is missing required front matter: {}",
                post.source_path.display(),
                post.missing_front_matter.join(", ")
            ));
        }
        if post.title.trim().is_empty() {
            errors.push(format!("{} has an empty title", post.source_path.display()));
        }
        if post.summary.trim().is_empty() {
            errors.push(format!(
                "{} has an empty summary",
                post.source_path.display()
            ));
        }
        if !is_valid_date(&post.date) {
            errors.push(format!(
                "{} has invalid or missing date '{}'; use YYYY-MM-DD",
                post.source_path.display(),
                post.date
            ));
        }
        if post.author.trim().is_empty() {
            errors.push(format!(
                "{} has an empty author",
                post.source_path.display()
            ));
        }
        if post.tags.is_empty() || post.tags.iter().any(|tag| tag.trim().is_empty()) {
            errors.push(format!(
                "{} has missing or empty tags",
                post.source_path.display()
            ));
        }
        if post.path.trim().is_empty() {
            errors.push(format!(
                "{} produced an empty slug",
                post.source_path.display()
            ));
        }
        for other in posts.iter().skip(index + 1) {
            if post.path == other.path {
                errors.push(format!(
                    "duplicate blog slug '{}': {} and {}",
                    post.path,
                    post.source_path.display(),
                    other.source_path.display()
                ));
            }
        }
    }
}

fn validate_about_page(page: &Page, errors: &mut Vec<String>) {
    if !page.missing_front_matter.is_empty() {
        errors.push(format!(
            "{} is missing required front matter: {}",
            page.source_path.display(),
            page.missing_front_matter.join(", ")
        ));
    }
    if page.title.trim().is_empty() {
        errors.push(format!("{} has an empty title", page.source_path.display()));
    }
    if page.summary.trim().is_empty() {
        errors.push(format!(
            "{} has an empty summary",
            page.source_path.display()
        ));
    }
}

fn is_valid_date(date: &str) -> bool {
    date.len() == 10
        && date.as_bytes()[0..10]
            .iter()
            .enumerate()
            .all(|(i, b)| matches!((i, *b), (4, b'-') | (7, b'-')) || b.is_ascii_digit())
}

fn write_generated_bytes(path: PathBuf, body: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, body)
}

#[derive(Clone, Debug)]
pub struct GeneratedSite {
    pub posts: usize,
    pub files: Vec<PathBuf>,
}

fn collect_content_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if should_skip_source_entry(&name) {
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

fn should_skip_source_entry(name: &str) -> bool {
    name.starts_with('.') || matches!(name, "target" | "node_modules" | "dist" | "public")
}

fn is_about_path(path: &Path) -> bool {
    let parts = path
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    matches!(
        parts.as_slice(),
        ["about.md"] | ["about.markdown"] | ["about.html"]
    )
}

fn load_post(root: &Path, source_path: &Path) -> io::Result<Option<Post>> {
    let raw = fs::read_to_string(source_path)?;
    let rel = match safe_relative(root, source_path) {
        Some(rel) => rel,
        None => return Ok(None),
    };
    if is_about_path(&rel) {
        return Ok(None);
    }
    let (front, body) = parse_front_matter(&raw);
    let fallback_title = rel
        .file_stem()
        .and_then(|name| name.to_str())
        .map(title_from_slug)
        .unwrap_or_else(|| "Untitled".to_string());
    let title_value = front_value(front, "title");
    let date_value = front_value(front, "date");
    let summary_value = front_value(front, "summary");
    let author_value = front_value(front, "author");
    let mut missing_front_matter = Vec::new();
    for key in ["title", "date", "summary", "author", "tags"] {
        if !front_has_key(front, key) {
            missing_front_matter.push(key.to_string());
        }
    }
    let title = title_value.unwrap_or_else(|| first_heading(body).unwrap_or(fallback_title));
    let date = date_value.unwrap_or_else(|| date_from_path(&rel));
    let author = author_value.unwrap_or_default();
    let tags = front_list(front, "tags");
    let summary = summary_value.unwrap_or_else(|| summarize(body));
    let path = slug_for_path(&rel);
    let content_body = remove_leading_heading(body, &title);
    let html = if source_path.extension().and_then(|ext| ext.to_str()) == Some("html") {
        body.to_string()
    } else {
        markdown_to_html(content_body)
    };

    Ok(Some(Post {
        title,
        path,
        source_path: source_path.to_path_buf(),
        summary,
        date,
        author,
        tags,
        body: strip_markdown(content_body),
        html,
        missing_front_matter,
    }))
}

fn load_page(root: &Path, source_path: &Path) -> io::Result<Option<Page>> {
    let raw = fs::read_to_string(source_path)?;
    let (front, body) = parse_front_matter(&raw);
    let rel = match safe_relative(root, source_path) {
        Some(rel) => rel,
        None => return Ok(None),
    };
    let fallback_title = rel
        .file_stem()
        .and_then(|name| name.to_str())
        .map(title_from_slug)
        .unwrap_or_else(|| "Untitled".to_string());
    let title_value = front_value(front, "title");
    let summary_value = front_value(front, "summary");
    let mut missing_front_matter = Vec::new();
    for key in ["title", "summary"] {
        if !front_has_key(front, key) {
            missing_front_matter.push(key.to_string());
        }
    }
    let title = title_value.unwrap_or_else(|| first_heading(body).unwrap_or(fallback_title));
    let summary = summary_value.unwrap_or_else(|| summarize(body));
    let content_body = remove_leading_heading(body, &title);
    let html = if source_path.extension().and_then(|ext| ext.to_str()) == Some("html") {
        body.to_string()
    } else {
        markdown_to_html(content_body)
    };
    Ok(Some(Page {
        title,
        source_path: source_path.to_path_buf(),
        summary,
        body: strip_markdown(content_body),
        html,
        missing_front_matter,
    }))
}

fn render_index(config: &BlogConfig, language: Language, posts: &[Post]) -> String {
    let mut cards = String::new();
    for post in posts {
        cards.push_str(&format!(
            "<article class=\"post-card\" data-search=\"{}\"><a href=\"{}\"><span class=\"date\">{}</span><h2>{}</h2><p>{}</p><div class=\"tags\">{}</div></a></article>",
            escape_attr(&search_blob(post)),
            escape_attr(&localized_path(language, &format!("/posts/{}.html", post.path))),
            escape_html(&post.date),
            escape_html(&post.title),
            escape_html(&post.summary),
            render_tags(&post.tags)
        ));
    }

    let git_stats = render_git_stats(language, &load_git_stats(&config.root));
    page_shell(
        config,
        language,
        "/",
        &PageMeta::index(config, language),
        &format!(
            "<section class=\"hero\"><div><p class=\"eyebrow\">{}</p><h1>{}</h1><p><a class=\"inline-link\" href=\"{}\">{}</a></p>{}</div></section><main id=\"content\" class=\"layout\" tabindex=\"-1\"><aside aria-label=\"{}\"><h2>{}</h2>{}</aside><section id=\"posts\" class=\"posts\" aria-label=\"Posts\">{}</section></main>",
            escape_html(language.hero_eyebrow),
            escape_html(language.title),
            escape_attr(&localized_path(language, "/about.html")),
            escape_html(language.about_label),
            git_stats,
            escape_attr(language.topics_label),
            escape_html(language.topics_label),
            render_topic_list(language, posts),
            cards
        ),
    )
}

fn render_dash_blog_index(
    config: &BlogConfig,
    language: Language,
    posts: &[Post],
) -> io::Result<String> {
    let mut cards = String::new();
    for post in posts {
        cards.push_str(&format!(
            "<button class=\"dash-card dash-card-button dash-post-card\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog{}\" data-search-card data-topic=\"{}\" data-search-text=\"{}\"><span class=\"date\">{}</span><strong>{}</strong><span>{}</span><div class=\"tags\">{}</div></button>",
            escape_attr(&localized_path(
                language,
                &format!("/posts/{}.html", post.path)
            )),
            escape_attr(&post.tags.join(" ")),
            escape_attr(&dash_search_blob(post)),
            escape_html(&post.date),
            escape_html(&post.title),
            escape_html(&post.summary),
            render_tags(&post.tags)
        ));
    }
    let stats = load_git_stats(&config.root);
    let tag_count = unique_tag_count(posts);
    let last_post = posts
        .first()
        .map(|post| post.date.as_str())
        .unwrap_or("unavailable");
    let commit_count = stats.commit_count.as_deref().unwrap_or("unavailable");
    let last_commit = stats
        .last_commit_message
        .as_deref()
        .unwrap_or("unavailable");
    let last_commit_age = stats
        .last_commit_epoch
        .map(relative_time)
        .unwrap_or_else(|| "unknown age".to_string());
    let post_count_label = if posts.len() == 1 {
        "post".to_string()
    } else {
        "posts".to_string()
    };
    let month_chart = render_dash_month_chart(posts);
    let topic_chart = render_dash_topic_chart(posts, 7);

    Ok(format!(
        "<div class=\"dash-blog\"><section class=\"dash-code-hero\"><p>{}</p><h2>{}</h2><div class=\"dash-code-tools\"><div class=\"dash-quick-links\"><button class=\"dash-link-button\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog/about\">About Edgerun</button><button class=\"dash-mail-button\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog/posts/edgerun-onboarding.html\">Onboarding guide</button><button class=\"dash-link-button\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog/posts/build-your-own-edgerun-app.html\">Build your first app</button></div></div></section><section class=\"dash-code-tools\" aria-label=\"Post tools\"><label><span>{}</span><input type=\"search\" data-workspace-search-scope placeholder=\"{}\"></label>{}</section><section class=\"dash-code-summary\" aria-label=\"Build log summary\"><div><span>Posts</span><strong>{}</strong></div><div><span>Topics</span><strong>{}</strong></div><div><span>Latest post</span><strong>{}</strong></div><div><span>Commits</span><strong>{}</strong></div><div class=\"summary-wide\"><span>Last commit</span><strong title=\"{}\">{}</strong></div><div><span>Age</span><strong>{}</strong></div></section><section class=\"dash-analytics\" aria-label=\"Build log analytics\">{}{}</section><section><h2>Posts</h2><p class=\"dash-search-count\" data-search-count>{} {}</p><div class=\"dash-grid\">{cards}</div></section><p class=\"dash-search-empty\" data-search-empty hidden>No matching posts.</p></div>",
        escape_html(language.hero_eyebrow),
        escape_html(language.title),
        escape_html(language.search_label),
        escape_attr(language.search_label),
        render_dash_topic_list(language, posts),
        posts.len(),
        tag_count,
        escape_html(last_post),
        escape_html(commit_count),
        escape_attr(last_commit),
        escape_html(last_commit),
        escape_html(&last_commit_age),
        month_chart,
        topic_chart,
        posts.len(),
        post_count_label,
    ))
}

fn render_dash_month_chart(posts: &[Post]) -> String {
    let months = collect_month_counts(posts, 8);
    if months.is_empty() {
        return "<section class=\"dash-chart-card\"><h3>Posts by month</h3><p class=\"muted\">No post date metadata available yet.</p></section>"
            .to_string();
    }
    let max = months.iter().map(|(_, count)| *count).max().unwrap_or(1);
    let rows = months
        .into_iter()
        .map(|(month, count)| {
            let width = (count.saturating_mul(100) / max.max(1)).max(12);
            format!(
                "<article class=\"dash-stat-row\"><div class=\"dash-stat-label-wrap\"><span class=\"dash-stat-label\">{}</span><span class=\"dash-stat-meta\">{} posts</span></div><span class=\"dash-stat-track\"><span class=\"dash-stat-fill\" style=\"--dash-stat-fill:{}%\"></span></span><strong>{}</strong></article>",
                escape_html(&month),
                count,
                width,
                count
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        "<section class=\"dash-chart-card\" aria-label=\"Posts by month\"><h3>Posts by month</h3><div class=\"dash-stat-chart\">{rows}</div></section>"
    )
}

fn render_dash_topic_chart(posts: &[Post], top: usize) -> String {
    let mut tags = collect_tag_counts(posts);
    if tags.is_empty() {
        return "<section class=\"dash-chart-card\"><h3>Tag distribution</h3><p class=\"muted\">No tags have been added yet.</p></section>"
            .to_string();
    }
    tags.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
    let max = tags.iter().map(|(_, count)| *count).max().unwrap_or(1);
    let rows = tags
        .into_iter()
        .take(top)
        .map(|(tag, count)| {
            let width = (count.saturating_mul(100) / max.max(1)).max(12);
            format!(
                "<article class=\"dash-stat-row\"><div class=\"dash-stat-label-wrap\"><span class=\"dash-stat-label\">{}</span><span class=\"dash-stat-meta\">{} posts</span></div><span class=\"dash-stat-track\"><span class=\"dash-stat-fill\" style=\"--dash-stat-fill:{}%\"></span></span><strong>{}</strong></article>",
                escape_html(&tag),
                count,
                width,
                count
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        "<section class=\"dash-chart-card\" aria-label=\"Tag distribution\"><h3>Tag distribution</h3><div class=\"dash-stat-chart\">{rows}</div></section>"
    )
}

fn render_dash_post(
    config: &BlogConfig,
    language: Language,
    post: &Post,
    posts: &[Post],
) -> String {
    let visible_crates = load_visible_crates(&config.root);
    let content = link_visible_crates_for_dash(&post.html, &visible_crates);
    let related = render_dash_related_posts(language, post, posts);
    let content = if related.is_empty() {
        format!("<div class=\"dash-article-content\">{}</div>", content)
    } else {
        format!("<div class=\"dash-article-content\">{}</div>{}", content, related)
    };
    format!(
        "<div class=\"dash-blog dash-article\"><button class=\"dash-link-button\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog\">{}</button><article class=\"article\" aria-labelledby=\"post-title\"><p class=\"date\"><time datetime=\"{}\">{}</time> by <span class=\"author\">{}</span></p><h1 id=\"post-title\">{}</h1><p class=\"summary\">{}</p><div class=\"tags\">{}</div><div class=\"content\">{}</div></article></div>",
        escape_html(language.back_label),
        escape_attr(&post.date),
        escape_html(&post.date),
        escape_html(&post.author),
        escape_html(&post.title),
        escape_html(&post.summary),
        render_tags(&post.tags),
        content,
    )
}

fn render_dash_about(language: Language, page: &Page) -> String {
    format!(
        "<div class=\"dash-blog dash-article\"><button class=\"dash-link-button\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog\">{}</button><article class=\"article\" aria-labelledby=\"page-title\"><h1 id=\"page-title\">{}</h1><p class=\"summary\">{}</p><div class=\"content\">{}</div></article></div>",
        escape_html(language.back_label),
        escape_html(&page.title),
        escape_html(&page.summary),
        page.html
    )
}

fn render_dash_feed(config: &BlogConfig, language: Language) -> String {
    let feed_url = absolute_url(config, &localized_path(language, "/feed.xml"));
    format!(
        "<div class=\"dash-blog\"><button class=\"dash-link-button\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog\">{}</button><section class=\"dash-code-hero\"><p>{}</p><h2>{}</h2><span>Atom feed remains a direct machine-readable endpoint.</span><a class=\"dash-mail-button\" href=\"{}\">{}</a></section></div>",
        escape_html(language.back_label),
        escape_html(language.feed_label),
        escape_html(language.feed_label),
        escape_attr(&feed_url),
        escape_html(language.feed_label)
    )
}

fn load_git_stats(root: &Path) -> GitStats {
    GitStats {
        commit_count: git_output(root, &["rev-list", "--count", "HEAD"]),
        last_commit_epoch: git_output(root, &["log", "-1", "--format=%ct"])
            .and_then(|value| value.parse().ok()),
        last_commit_message: git_output(root, &["log", "-1", "--format=%s"]),
    }
}

fn load_visible_crates(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let crates_root = root.join("crates");
    if let Ok(entries) = fs::read_dir(&crates_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.join(".gitvisible").exists() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                if name.starts_with("edgerun-") && !names.iter().any(|existing| existing == name) {
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort_by_key(|name| core::cmp::Reverse(name.len()));
    names
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-c")
        .arg(format!("safe.directory={}", root.display()))
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn render_git_stats(language: Language, stats: &GitStats) -> String {
    let commit_count = stats.commit_count.as_deref().unwrap_or("unavailable");
    let last_commit_message = stats
        .last_commit_message
        .as_deref()
        .unwrap_or("unavailable");
    let last_commit_age = stats
        .last_commit_epoch
        .map(relative_time)
        .unwrap_or_else(|| "unknown age".to_string());

    format!(
        "<dl class=\"repo-stats\" aria-label=\"Repository stats\"><div><dt>{}</dt><dd>{}</dd></div><div class=\"wide\"><dt>{}</dt><dd class=\"message\">{} <span>{}</span></dd></div></dl>",
        escape_html(language.repo_commits_label),
        escape_html(commit_count),
        escape_html(language.last_commit_label),
        escape_html(last_commit_message),
        escape_html(&last_commit_age)
    )
}

fn relative_time(epoch_seconds: u64) -> String {
    let commit_time = UNIX_EPOCH + Duration::from_secs(epoch_seconds);
    let elapsed = SystemTime::now()
        .duration_since(commit_time)
        .unwrap_or_else(|_| Duration::from_secs(0));
    let seconds = elapsed.as_secs();
    if seconds < 60 {
        return "just now".to_string();
    }

    let units = [
        ("year", 365 * 24 * 60 * 60),
        ("month", 30 * 24 * 60 * 60),
        ("week", 7 * 24 * 60 * 60),
        ("day", 24 * 60 * 60),
        ("hour", 60 * 60),
        ("minute", 60),
    ];
    for (name, unit_seconds) in units {
        if seconds >= unit_seconds {
            let count = seconds / unit_seconds;
            let suffix = if count == 1 { "" } else { "s" };
            return format!("{count} {name}{suffix} ago");
        }
    }

    "just now".to_string()
}

fn render_post(config: &BlogConfig, language: Language, post: &Post, posts: &[Post]) -> String {
    let visible_crates = load_visible_crates(&config.root);
    let content = link_visible_crates(&post.html, &visible_crates);
    let related = render_related_posts(language, post, posts);
    let nav = posts
        .iter()
        .take(8)
        .map(|item| {
            format!(
                "<a href=\"{}\">{}</a>",
                escape_attr(&localized_path(
                    language,
                    &format!("/posts/{}.html", item.path)
                )),
                escape_html(&item.title)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    page_shell(
        config,
        language,
        &format!("/posts/{}.html", post.path),
        &PageMeta::post(config, language, post),
        &format!(
        "<main id=\"content\" class=\"article-layout\" tabindex=\"-1\"><div class=\"article-stack\"><a class=\"back\" href=\"{}\">{}</a><article class=\"article\" aria-labelledby=\"post-title\"><p class=\"date\"><time datetime=\"{}\">{}</time> by <span class=\"author\">{}</span></p><h1 id=\"post-title\">{}</h1><p class=\"summary\">{}</p><div class=\"tags\">{}</div><div class=\"content\">{}</div></article>{}</div><aside aria-label=\"{}\"><h2>{}</h2><nav class=\"recent\" aria-label=\"{}\">{}</nav></aside></main>",
        escape_attr(&localized_path(language, "/")),
        escape_html(language.back_label),
        escape_attr(&post.date),
        escape_html(&post.date),
            escape_html(&post.author),
            escape_html(&post.title),
            escape_html(&post.summary),
            render_tags(&post.tags),
            content,
            related,
            escape_attr(language.recent_label),
            escape_html(language.recent_label),
            escape_attr(language.recent_label),
            nav
        ),
    )
}

fn render_related_posts(language: Language, post: &Post, posts: &[Post]) -> String {
    let mut related = posts
        .iter()
        .filter(|candidate| candidate.path != post.path)
        .map(|candidate| {
            let shared_tags = candidate
                .tags
                .iter()
                .filter(|tag| post.tags.iter().any(|current| current == *tag))
                .count();
            let text_match = usize::from(
                post.body.contains(&candidate.title) || candidate.body.contains(&post.title),
            );
            (shared_tags * 2 + text_match, candidate)
        })
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();

    related.sort_by(|(left_score, left), (right_score, right)| {
        right_score
            .cmp(left_score)
            .then_with(|| right.date.cmp(&left.date))
    });

    let items = if related.is_empty() {
        posts
            .iter()
            .filter(|candidate| candidate.path != post.path)
            .take(3)
            .collect::<Vec<_>>()
    } else {
        related
            .into_iter()
            .take(3)
            .map(|(_, item)| item)
            .collect::<Vec<_>>()
    };

    let cards = items
        .into_iter()
        .map(|item| {
            format!(
                "<article><a href=\"{}\"><span>{}</span><strong>{}</strong><small>{}</small></a></article>",
                escape_attr(&localized_path(
                    language,
                    &format!("/posts/{}.html", item.path)
                )),
                escape_html(&item.date),
                escape_html(&item.title),
                escape_html(&item.summary)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    if cards.is_empty() {
        String::new()
    } else {
        format!(
            "<section class=\"related-posts\" aria-labelledby=\"related-posts-title\"><h2 id=\"related-posts-title\">{}</h2><div>{}</div></section>",
            escape_html(language.related_label),
            cards
        )
    }
}

fn render_dash_related_posts(language: Language, post: &Post, posts: &[Post]) -> String {
    let mut related = posts
        .iter()
        .filter(|candidate| candidate.path != post.path)
        .map(|candidate| {
            let shared_tags = candidate
                .tags
                .iter()
                .filter(|tag| post.tags.iter().any(|current| current == *tag))
                .count();
            (shared_tags, candidate)
        })
        .filter(|(shared, _)| *shared > 0)
        .collect::<Vec<_>>();
    related.sort_by(|left, right| right.0.cmp(&left.0));
    if related.is_empty() {
        related = posts
            .iter()
            .filter(|candidate| candidate.path != post.path)
            .take(3)
            .map(|candidate| (0, candidate))
            .collect();
    }
    if related.is_empty() {
        String::new()
    } else {
        let cards = related
            .into_iter()
            .take(3)
            .map(|(_, item)| {
                let localized = localized_path(language, &format!("/posts/{}.html", item.path));
                format!(
                    "<article class=\"dash-related-post\"><button class=\"dash-card dash-card-button\" type=\"button\" data-surface-module=\"build-log\" data-surface=\"build-log\" hx-get=\"/surface/blog{}\"><span class=\"date\">{}</span><strong>{}</strong><small>{}</small></button></article>",
                    escape_attr(&localized),
                    escape_html(&item.date),
                    escape_html(&item.title),
                    escape_html(&item.summary)
                )
            })
            .collect::<Vec<_>>()
            .join("");
        format!(
            "<section class=\"related-posts\" aria-labelledby=\"related-posts-title\"><h2 id=\"related-posts-title\">{}</h2><div>{}</div></section>",
            escape_html(language.related_label),
            cards
        )
    }
}

fn render_about(config: &BlogConfig, language: Language, page: &Page) -> String {
    page_shell(
        config,
        language,
        "/about.html",
        &PageMeta::about(config, language, page),
        &format!(
            "<main id=\"content\" class=\"article-layout article-layout-single\" tabindex=\"-1\"><div class=\"article-stack\"><a class=\"back\" href=\"{}\">{}</a><article class=\"article\" aria-labelledby=\"page-title\"><h1 id=\"page-title\">{}</h1><p class=\"summary\">{}</p><div class=\"content\">{}</div></article></div></main>",
            escape_attr(&localized_path(language, "/")),
            escape_html(language.back_label),
            escape_html(&page.title),
            escape_html(&page.summary),
            page.html
        ),
    )
}

struct PageMeta {
    title: String,
    schema_name: String,
    description: String,
    canonical: String,
    page_type: &'static str,
    schema_type: &'static str,
    author: Option<String>,
    published_time: Option<String>,
    noindex: bool,
}

impl PageMeta {
    fn index(config: &BlogConfig, language: Language) -> Self {
        Self {
            title: language.title.to_string(),
            schema_name: language.title.to_string(),
            description: language.description.to_string(),
            canonical: absolute_url(config, &localized_path(language, "/")),
            page_type: "website",
            schema_type: "Blog",
            author: None,
            published_time: None,
            noindex: false,
        }
    }

    fn post(config: &BlogConfig, language: Language, post: &Post) -> Self {
        Self {
            title: post.title.clone(),
            schema_name: post.title.clone(),
            description: post.summary.clone(),
            canonical: absolute_url(
                config,
                &localized_path(language, &format!("/posts/{}.html", post.path)),
            ),
            page_type: "article",
            schema_type: "BlogPosting",
            author: Some(post.author.clone()),
            published_time: if post.date.is_empty() {
                None
            } else {
                Some(atom_date(&post.date))
            },
            noindex: false,
        }
        .with_site_title(language.title)
    }

    fn about(config: &BlogConfig, language: Language, page: &Page) -> Self {
        Self {
            title: page.title.clone(),
            schema_name: page.title.clone(),
            description: page.summary.clone(),
            canonical: absolute_url(config, &localized_path(language, "/about.html")),
            page_type: "website",
            schema_type: "AboutPage",
            author: None,
            published_time: None,
            noindex: false,
        }
        .with_site_title(language.title)
    }

    fn not_found(title: &str) -> Self {
        Self {
            title: "Not found".to_string(),
            schema_name: "Not found".to_string(),
            description: "The requested page does not exist.".to_string(),
            canonical: String::new(),
            page_type: "website",
            schema_type: "WebPage",
            author: None,
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

fn page_shell(
    config: &BlogConfig,
    language: Language,
    route: &str,
    meta: &PageMeta,
    body: &str,
) -> String {
    let mut head = String::new();
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
        "<meta property=\"og:locale\" content=\"{}\"><meta property=\"og:site_name\" content=\"{}\"><meta property=\"og:title\" content=\"{}\"><meta property=\"og:description\" content=\"{}\"><meta property=\"og:type\" content=\"{}\"><meta name=\"twitter:card\" content=\"summary\"><meta name=\"twitter:title\" content=\"{}\"><meta name=\"twitter:description\" content=\"{}\">",
        escape_attr(language.og_locale),
        escape_attr(language.title),
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
    if let Some(author) = meta.author.as_deref() {
        head.push_str(&format!(
            "<meta name=\"author\" content=\"{}\"><meta property=\"article:author\" content=\"{}\">",
            escape_attr(author),
            escape_attr(author)
        ));
    }
    head.push_str(&format!(
        "<script type=\"application/ld+json\">{}</script>",
        render_json_ld(language, meta)
    ));
    let script_src = format!("/app.js?v={}", asset_version(&blog_js()));
    for alternate in LANGUAGES.iter().copied() {
        head.push_str(&format!(
            "<link rel=\"alternate\" hreflang=\"{}\" href=\"{}\">",
            escape_attr(alternate.html_lang),
            escape_attr(&absolute_url(config, &localized_path(alternate, route)))
        ));
    }
    head.push_str(&format!(
        "<link rel=\"alternate\" hreflang=\"x-default\" href=\"{}\">",
        escape_attr(&absolute_url(
            config,
            &localized_path(default_language(), route)
        ))
    ));
    head.push_str("<link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\"><link rel=\"manifest\" href=\"/site.webmanifest\"><link rel=\"search\" type=\"application/opensearchdescription+xml\" href=\"/opensearch.xml\"><link rel=\"alternate\" type=\"application/atom+xml\" href=\"/feed.xml\">");
    let language_links = render_language_links(language, route);
    let header_center = edgerun_web_ui::render_header_search(
        &localized_path(language, "/"),
        "search",
        "q",
        language.search_label,
        language.search_label,
    );
    let header_actions = edgerun_web_ui::render_workspace_actions("blog", "");
    let about_path = localized_path(language, "/about.html");
    let feed_path = localized_path(language, "/feed.xml");
    let local_links = [
        FooterLink {
            href: &about_path,
            label: language.about_label,
        },
        FooterLink {
            href: &feed_path,
            label: language.feed_label,
        },
    ];
    let footer = edgerun_web_ui::render_common_footer("blog", &local_links, &language_links);
    let style = blog_style();
    let brand_href = localized_path(language, "/");
    let brand_label = format!("{} home", language.title);
    edgerun_web_ui::render_page(&PageShell {
        lang: language.html_lang,
        title: &meta.title,
        description: &meta.description,
        theme_color: "#146c63",
        generator: "edgerun-blog",
        extra_head: &head,
        style: &style,
        brand_href: &brand_href,
        brand_label: &brand_label,
        brand_text: language.title,
        header_center: &header_center,
        header_actions: &header_actions,
        footer: &footer,
        body,
        script_src: Some(&script_src),
        workspace_modules: &[],
    })
}

fn render_search_json(posts: &[Post]) -> String {
    let mut out = String::from("[");
    for (index, post) in posts.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"title\":\"{}\",\"url\":\"/posts/{}.html\",\"date\":\"{}\",\"author\":\"{}\",\"summary\":\"{}\",\"tags\":[{}],\"text\":\"{}\"}}",
            escape_json(&post.title),
            escape_json(&post.path),
            escape_json(&post.date),
            escape_json(&post.author),
            escape_json(&post.summary),
            post.tags.iter().map(|tag| format!("\"{}\"", escape_json(tag))).collect::<Vec<_>>().join(","),
            escape_json(&post.body)
        ));
    }
    out.push(']');
    out
}

fn render_feed(config: &BlogConfig, language: Language, posts: &[Post]) -> String {
    let base = config.base_url.trim_end_matches('/');
    let mut entries = String::new();
    for post in posts.iter().take(20) {
        let url = absolute_url(
            config,
            &localized_path(language, &format!("/posts/{}.html", post.path)),
        );
        entries.push_str(&format!(
            "<entry><title>{}</title><link href=\"{}\"/><id>{}</id><updated>{}</updated><author><name>{}</name></author><summary>{}</summary></entry>",
            escape_html(&post.title),
            escape_attr(&url),
            escape_html(&url),
            atom_date(&post.date),
            escape_html(&post.author),
            escape_html(&post.summary)
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><feed xmlns=\"http://www.w3.org/2005/Atom\"><title>{}</title><id>{}</id><updated>{}</updated>{}</feed>",
        escape_html(language.title),
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

fn render_sitemap(config: &BlogConfig, sites: &[LanguageSite]) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for site in sites {
        xml.push_str(&format!(
            "  <url><loc>{}</loc></url>\n",
            escape_html(&absolute_url(config, &localized_path(site.language, "/")))
        ));
        if site.about.is_some() {
            xml.push_str(&format!(
                "  <url><loc>{}</loc></url>\n",
                escape_html(&absolute_url(
                    config,
                    &localized_path(site.language, "/about.html")
                ))
            ));
        }
        for post in &site.posts {
            xml.push_str("  <url>");
            xml.push_str(&format!(
                "<loc>{}</loc>",
                escape_html(&absolute_url(
                    config,
                    &localized_path(site.language, &format!("/posts/{}.html", post.path))
                ))
            ));
            if !post.date.is_empty() {
                xml.push_str(&format!("<lastmod>{}</lastmod>", escape_html(&post.date)));
            }
            xml.push_str("</url>\n");
        }
    }
    xml.push_str("</urlset>\n");
    xml
}

fn render_opensearch(config: &BlogConfig) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><OpenSearchDescription xmlns=\"http://a9.com/-/spec/opensearch/1.1/\"><ShortName>{}</ShortName><Description>{}</Description><InputEncoding>UTF-8</InputEncoding><Url type=\"text/html\" template=\"{}?q={{searchTerms}}\"/><Url type=\"application/json\" template=\"{}/search.json\"/></OpenSearchDescription>",
        escape_html(&config.title),
        escape_html(&config.description),
        escape_attr(&absolute_url(config, "/")),
        escape_attr(config.base_url.trim_end_matches('/'))
    )
}

fn render_json_ld(language: Language, meta: &PageMeta) -> String {
    let mut fields = vec![
        "\"@context\":\"https://schema.org\"".to_string(),
        format!("\"@type\":\"{}\"", meta.schema_type),
        format!("\"name\":\"{}\"", escape_json(&meta.schema_name)),
        format!("\"description\":\"{}\"", escape_json(&meta.description)),
        format!(
            "\"publisher\":{{\"@type\":\"Organization\",\"name\":\"{}\"}}",
            escape_json(language.title)
        ),
    ];
    if meta.schema_type == "BlogPosting" {
        fields.push(format!(
            "\"headline\":\"{}\"",
            escape_json(&meta.schema_name)
        ));
    }
    if !meta.canonical.is_empty() {
        fields.push(format!("\"url\":\"{}\"", escape_json(&meta.canonical)));
        fields.push(format!(
            "\"mainEntityOfPage\":\"{}\"",
            escape_json(&meta.canonical)
        ));
    }
    if let Some(published_time) = meta.published_time.as_deref() {
        fields.push(format!(
            "\"datePublished\":\"{}\"",
            escape_json(published_time)
        ));
        fields.push(format!(
            "\"dateModified\":\"{}\"",
            escape_json(published_time)
        ));
    }
    if let Some(author) = meta.author.as_deref() {
        fields.push(format!(
            "\"author\":{{\"@type\":\"Person\",\"name\":\"{}\"}}",
            escape_json(author)
        ));
    }
    format!("{{{}}}", fields.join(","))
}

fn localized_path(language: Language, path: &str) -> String {
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    if language.path_prefix.is_empty() {
        path
    } else if path == "/" {
        format!("{}/", language.path_prefix)
    } else {
        format!("{}{}", language.path_prefix, path)
    }
}

fn render_language_links(current: Language, route: &str) -> String {
    let links = LANGUAGES
        .iter()
        .copied()
        .map(|language| {
            let aria = if language == current {
                " aria-current=\"true\""
            } else {
                ""
            };
            format!(
                "<a href=\"{}\" lang=\"{}\"{}>{}</a>",
                escape_attr(&localized_path(language, route)),
                escape_attr(language.html_lang),
                aria,
                escape_html(language.code)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!("<nav class=\"language-links\" aria-label=\"Languages\">{links}</nav>")
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

fn asset_version(input: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn markdown_to_html(input: &str) -> String {
    let mut html = String::new();
    let mut paragraph = String::new();
    let mut in_code = false;
    let mut code_has_figure = false;
    let mut in_ul = false;
    let mut in_ol = false;
    let lines = input.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        let trimmed = lines[index].trim_end();
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
            index += 1;
            continue;
        }
        if in_code {
            html.push_str(&escape_html(trimmed));
            html.push('\n');
            index += 1;
            continue;
        }
        if trimmed.is_empty() {
            flush_blocks(&mut html, &mut paragraph, &mut in_ul, &mut in_ol);
            index += 1;
            continue;
        }
        if is_table_row(trimmed)
            && lines
                .get(index + 1)
                .is_some_and(|line| is_table_separator(line.trim()))
        {
            flush_blocks(&mut html, &mut paragraph, &mut in_ul, &mut in_ol);
            let (table, next_index) = render_markdown_table(&lines, index);
            html.push_str(&table);
            index = next_index;
            continue;
        } else if is_trusted_html_block_line(trimmed) {
            flush_blocks(&mut html, &mut paragraph, &mut in_ul, &mut in_ol);
            html.push_str(trimmed);
        } else if let Some((level, text)) = heading(trimmed) {
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
        index += 1;
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

fn render_markdown_table(lines: &[&str], start: usize) -> (String, usize) {
    let headers = parse_table_cells(lines[start].trim());
    let alignments = parse_table_alignments(lines[start + 1].trim(), headers.len());
    let mut html = String::from("<table><thead><tr>");
    for (index, header) in headers.iter().enumerate() {
        html.push_str(&table_cell(
            "th",
            header,
            alignments.get(index).copied().flatten(),
        ));
    }
    html.push_str("</tr></thead><tbody>");

    let mut index = start + 2;
    while let Some(line) = lines.get(index) {
        let trimmed = line.trim();
        if !is_table_row(trimmed) || is_table_separator(trimmed) {
            break;
        }
        let cells = parse_table_cells(trimmed);
        html.push_str("<tr>");
        for cell_index in 0..headers.len() {
            let cell = cells.get(cell_index).map(String::as_str).unwrap_or("");
            html.push_str(&table_cell(
                "td",
                cell,
                alignments.get(cell_index).copied().flatten(),
            ));
        }
        html.push_str("</tr>");
        index += 1;
    }
    html.push_str("</tbody></table>");
    (html, index)
}

fn table_cell(tag: &str, value: &str, alignment: Option<TableAlignment>) -> String {
    let align_attr = alignment
        .map(|alignment| format!(" style=\"text-align:{}\"", alignment.as_css()))
        .unwrap_or_default();
    format!(
        "<{tag}{align_attr}>{}</{tag}>",
        inline_markdown(value.trim())
    )
}

#[derive(Clone, Copy)]
enum TableAlignment {
    Left,
    Center,
    Right,
}

impl TableAlignment {
    fn as_css(self) -> &'static str {
        match self {
            TableAlignment::Left => "left",
            TableAlignment::Center => "center",
            TableAlignment::Right => "right",
        }
    }
}

fn parse_table_alignments(line: &str, len: usize) -> Vec<Option<TableAlignment>> {
    let mut alignments = parse_table_cells(line)
        .into_iter()
        .map(|cell| {
            let cell = cell.trim();
            let left = cell.starts_with(':');
            let right = cell.ends_with(':');
            match (left, right) {
                (true, true) => Some(TableAlignment::Center),
                (true, false) => Some(TableAlignment::Left),
                (false, true) => Some(TableAlignment::Right),
                (false, false) => None,
            }
        })
        .collect::<Vec<_>>();
    alignments.resize(len, None);
    alignments
}

fn parse_table_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim().trim_matches('|');
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut escaped = false;
    for ch in trimmed.chars() {
        if escaped {
            cell.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '|' {
            cells.push(cell.trim().to_string());
            cell.clear();
        } else {
            cell.push(ch);
        }
    }
    cells.push(cell.trim().to_string());
    cells
}

fn is_table_row(line: &str) -> bool {
    line.contains('|') && parse_table_cells(line).len() >= 2
}

fn is_table_separator(line: &str) -> bool {
    let cells = parse_table_cells(line);
    cells.len() >= 2
        && cells.iter().all(|cell| {
            let stripped = cell.trim().trim_matches(':').trim();
            stripped.len() >= 3 && stripped.chars().all(|ch| ch == '-')
        })
}

fn is_trusted_html_block_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('<') {
        return false;
    }
    let tag_source = trimmed.trim_start_matches('<').trim_start_matches('/');
    let tag = trimmed
        .trim_start_matches('<')
        .trim_start_matches('/')
        .split(|ch: char| ch == '>' || ch == '/' || ch.is_ascii_whitespace())
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if tag_source.starts_with('!') || tag_source.starts_with('?') || tag.is_empty() {
        return false;
    }
    matches!(
        tag.as_str(),
        "aside"
            | "blockquote"
            | "caption"
            | "circle"
            | "details"
            | "div"
            | "figcaption"
            | "figure"
            | "g"
            | "line"
            | "path"
            | "polyline"
            | "rect"
            | "section"
            | "summary"
            | "svg"
            | "table"
            | "tbody"
            | "td"
            | "text"
            | "tfoot"
            | "th"
            | "thead"
            | "tr"
    )
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
    while !rest.is_empty() {
        let link_start = rest.find('[');
        let code_start = rest.find('`');
        let next = match (link_start, code_start) {
            (Some(link), Some(code)) => link.min(code),
            (Some(link), None) => link,
            (None, Some(code)) => code,
            (None, None) => {
                out.push_str(&escape_html(rest));
                break;
            }
        };
        out.push_str(&escape_html(&rest[..next]));
        rest = &rest[next..];

        if let Some(after_tick) = rest.strip_prefix('`') {
            if let Some(end) = after_tick.find('`') {
                out.push_str(&format!("<code>{}</code>", escape_html(&after_tick[..end])));
                rest = &after_tick[end + 1..];
                continue;
            }
            out.push_str("`");
            rest = after_tick;
            continue;
        }

        if let Some(mid) = rest.find("](") {
            if let Some(end) = rest[mid + 2..].find(')') {
                let label = &rest[1..mid];
                let href = &rest[mid + 2..mid + 2 + end];
                out.push_str(&format!(
                    "<a href=\"{}\">{}</a>",
                    escape_attr(href),
                    escape_html(label)
                ));
                rest = &rest[mid + 3 + end..];
                continue;
            }
        }
        out.push_str(&escape_html(&rest[..1]));
        rest = &rest[1..];
    }
    out
}

fn flush_blocks(html: &mut String, paragraph: &mut String, in_ul: &mut bool, in_ol: &mut bool) {
    flush_paragraph(html, paragraph);
    close_ul(html, in_ul);
    close_ol(html, in_ol);
}

fn flush_paragraph(html: &mut String, paragraph: &mut String) {
    if !paragraph.is_empty() {
        if let Some(embed) = youtube_embed_html(paragraph) {
            html.push_str(&embed);
        } else {
            html.push_str(&format!("<p>{}</p>", inline_markdown(paragraph)));
        }
        paragraph.clear();
    }
}

fn youtube_embed_html(input: &str) -> Option<String> {
    let video_id = youtube_video_id(input.trim())?;
    Some(format!(
        "<figure class=\"video-embed\"><iframe src=\"https://www.youtube-nocookie.com/embed/{}\" title=\"Embedded video\" loading=\"lazy\" allow=\"accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share\" allowfullscreen></iframe></figure>",
        escape_attr(&video_id)
    ))
}

fn youtube_video_id(input: &str) -> Option<String> {
    let id = input
        .strip_prefix("https://www.youtube.com/watch?v=")
        .or_else(|| input.strip_prefix("https://youtube.com/watch?v="))
        .or_else(|| input.strip_prefix("https://youtu.be/"))?
        .split(['&', '?', '#'])
        .next()
        .unwrap_or("");
    if id.len() == 11
        && id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        Some(id.to_string())
    } else {
        None
    }
}

fn link_visible_crates(html: &str, crates: &[String]) -> String {
    link_visible_crates_with(html, crates, false)
}

fn link_visible_crates_for_dash(html: &str, crates: &[String]) -> String {
    link_visible_crates_with(html, crates, true)
}

fn link_visible_crates_with(html: &str, crates: &[String], dash: bool) -> String {
    if crates.is_empty() {
        return html.to_string();
    }

    let mut out = String::new();
    let mut rest = html;
    let mut skipped_tags = Vec::<String>::new();
    while let Some(tag_start) = rest.find('<') {
        let text = &rest[..tag_start];
        if skipped_tags.is_empty() {
            out.push_str(&link_crates_in_text(text, crates, dash));
        } else {
            out.push_str(text);
        }
        let Some(tag_end) = rest[tag_start..].find('>') else {
            out.push_str(&rest[tag_start..]);
            return out;
        };
        let tag = &rest[tag_start..tag_start + tag_end + 1];
        update_skipped_tags(tag, &mut skipped_tags);
        out.push_str(tag);
        rest = &rest[tag_start + tag_end + 1..];
    }
    if skipped_tags.is_empty() {
        out.push_str(&link_crates_in_text(rest, crates, dash));
    } else {
        out.push_str(rest);
    }
    out
}

fn update_skipped_tags(tag: &str, skipped_tags: &mut Vec<String>) {
    let trimmed = tag.trim_start_matches('<').trim_start();
    let closing = trimmed.starts_with('/');
    let name = trimmed
        .trim_start_matches('/')
        .split(|ch: char| ch == '>' || ch == '/' || ch.is_ascii_whitespace())
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(name.as_str(), "a" | "code" | "pre" | "script" | "style") {
        return;
    }
    if closing {
        if let Some(index) = skipped_tags.iter().rposition(|tag| tag == &name) {
            skipped_tags.remove(index);
        }
    } else if !tag.ends_with("/>") {
        skipped_tags.push(name);
    }
}

fn link_crates_in_text(text: &str, crates: &[String], dash: bool) -> String {
    let mut out = String::new();
    let mut rest = text;
    let mut previous = None;
    while !rest.is_empty() {
        let mut matched = None;
        for name in crates {
            if rest.starts_with(name)
                && has_left_text_boundary(previous)
                && has_right_text_boundary(rest, name.len())
            {
                matched = Some(name.as_str());
                break;
            }
        }
        if let Some(name) = matched {
            if dash {
                out.push_str(&format!(
                    "<a class=\"crate-link\" href=\"#code/edgerun_core/crates/{}\">{}</a>",
                    escape_attr(name),
                    escape_html(name)
                ));
            } else {
                out.push_str(&format!(
                    "<a class=\"crate-link\" href=\"https://git.edgerun.tech/edgerun_core/crates/{}\">{}</a>",
                    escape_attr(name),
                    escape_html(name)
                ));
            }
            rest = &rest[name.len()..];
            previous = name.chars().last();
        } else {
            let ch = rest.chars().next().unwrap_or_default();
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
            previous = Some(ch);
        }
    }
    out
}

fn has_left_text_boundary(previous: Option<char>) -> bool {
    previous
        .map(|ch| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
        .unwrap_or(true)
}

fn has_right_text_boundary(rest: &str, matched_len: usize) -> bool {
    rest[matched_len..]
        .chars()
        .next()
        .map(|ch| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
        .unwrap_or(true)
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

fn front_has_key(front: &str, key: &str) -> bool {
    front
        .lines()
        .filter_map(|line| line.split_once(':').map(|(name, _)| name.trim()))
        .any(|name| name == key)
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

fn remove_leading_heading<'a>(input: &'a str, title: &str) -> &'a str {
    let start = input.trim_start_matches(|ch| ch == '\n' || ch == '\r');
    let first_line_end = start.find('\n').unwrap_or(start.len());
    let first_line = start[..first_line_end].trim_end_matches('\r').trim();
    if heading(first_line).is_some_and(|(_, text)| text == title.trim()) {
        start[first_line_end..].trim_start_matches(|ch| ch == '\n' || ch == '\r')
    } else {
        input
    }
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
    let mut parts = without_ext
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => name.to_str().map(slugify),
            _ => None,
        })
        .collect::<Vec<_>>();
    if parts.first().map(String::as_str) == Some("posts") {
        let _ = parts.remove(0);
    }
    parts.join("/")
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

fn render_topic_list(language: Language, posts: &[Post]) -> String {
    let tags = collect_tag_counts(posts);
    let mut buttons = vec![format!(
        "<button type=\"button\" data-topic=\"\" aria-pressed=\"true\">{} <span>{}</span></button>",
        escape_html(language.all_label),
        posts.len()
    )];
    buttons.extend(tags.iter().map(|(tag, count)| {
        format!(
            "<button type=\"button\" data-topic=\"{}\" aria-pressed=\"false\">{} <span>{}</span></button>",
            escape_attr(tag),
            escape_html(tag),
            count
        )
    }));
    format!("<div class=\"topic-list\">{}</div>", buttons.join(""))
}

fn render_dash_topic_list(language: Language, posts: &[Post]) -> String {
    let tags = collect_tag_counts(posts);
    let mut buttons = vec![format!(
        "<button class=\"pill-button\" type=\"button\" data-topic-filter=\"\" aria-pressed=\"true\">{} <span>{}</span></button>",
        escape_html(language.all_label),
        posts.len()
    )];
    buttons.extend(tags.iter().map(|(tag, count)| {
        format!(
            "<button class=\"pill-button\" type=\"button\" data-topic-filter=\"{}\">{} <span>{}</span></button>",
            escape_attr(tag),
            escape_html(tag),
            count
        )
    }));
    format!("<div class=\"pill-row\">{}</div>", buttons.join(""))
}

fn unique_tag_count(posts: &[Post]) -> usize {
    collect_tag_counts(posts).len()
}

fn collect_tag_counts(posts: &[Post]) -> Vec<(String, usize)> {
    let mut tags = Vec::<(String, usize)>::new();
    for post in posts {
        for tag in &post.tags {
            if let Some((_, count)) = tags.iter_mut().find(|(seen, _)| seen == tag) {
                *count += 1;
            } else {
                tags.push((tag.clone(), 1));
            }
        }
    }
    tags.sort_by(|left, right| left.0.cmp(&right.0));
    tags
}

fn collect_month_counts(posts: &[Post], limit: usize) -> Vec<(String, usize)> {
    let mut counts = Vec::<(String, usize)>::new();
    for post in posts {
        let Some(month) = post_month(&post.date) else {
            continue;
        };
        if let Some((_, count)) = counts.iter_mut().find(|(entry, _)| entry == &month) {
            *count += 1;
            continue;
        }
        if counts.len() >= limit {
            continue;
        }
        counts.push((month, 1));
    }
    counts.sort_by(|left, right| right.0.cmp(&left.0));
    if counts.len() > limit {
        counts.truncate(limit);
    }
    counts
}

fn post_month(value: &str) -> Option<String> {
    let mut parts = value.splitn(3, '-');
    let year = parts.next()?;
    let month = parts.next()?;
    if year.len() != 4
        || !year.chars().all(|ch| ch.is_ascii_digit())
        || month.len() != 2
        || !month.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    Some(format!("{}-{}", year, month))
}

fn search_blob(post: &Post) -> String {
    format!(
        "{} {} {} {} {}",
        post.title,
        post.summary,
        post.author,
        post.tags.join(" "),
        post.body
    )
    .to_lowercase()
}

fn dash_search_blob(post: &Post) -> String {
    format!(
        "{} {} {} {}",
        post.title,
        post.summary,
        post.author,
        post.tags.join(" ")
    )
    .to_lowercase()
}

fn is_content_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("md") | Some("markdown") | Some("mdx") | Some("html")
    )
}

fn escape_html(input: &str) -> String {
    edgerun_web_ui::escape_html(input)
}

fn escape_attr(input: &str) -> String {
    edgerun_web_ui::escape_attr(input)
}

fn escape_json(input: &str) -> String {
    edgerun_web_ui::escape_json(input)
}

fn not_found_response(title: &str) -> Response {
    let config = BlogConfig {
        root: PathBuf::new(),
        content_dir: PathBuf::from("."),
        static_root: None,
        bind_addr: String::new(),
        title: title.to_string(),
        description: "Not found".to_string(),
        base_url: String::new(),
    };
    Response::html(
        StatusCode::NOT_FOUND,
        &page_shell(
            &config,
            default_language(),
            "/404.html",
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

fn static_file_response(root: &Path, route: &str) -> Option<Response> {
    let path = static_file_path(root, route)?;
    let body = fs::read(&path).ok()?;
    let content_type = content_type_for(&path);
    Some(
        Response::new(StatusCode::OK)
            .with_header("Content-Type", content_type)
            .with_header("Cache-Control", cache_control_for(&path))
            .with_header("X-Content-Type-Options", "nosniff")
            .with_body(body),
    )
}

fn static_file_path(root: &Path, route: &str) -> Option<PathBuf> {
    let trimmed = route.trim_start_matches('/');
    let relative = if trimmed.is_empty() {
        "index.html"
    } else {
        trimmed
    };
    let mut path = root.to_path_buf();
    for part in Path::new(relative).components() {
        match part {
            Component::Normal(name) => path.push(name),
            _ => return None,
        }
    }
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

fn content_type_for(path: &Path) -> &'static str {
    match path.file_name().and_then(|name| name.to_str()) {
        Some("feed.xml") => return "application/atom+xml; charset=utf-8",
        Some("opensearch.xml") => {
            return "application/opensearchdescription+xml; charset=utf-8";
        }
        Some("sitemap.xml") => return "application/xml; charset=utf-8",
        _ => {}
    }
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("txt") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        Some("webmanifest") => "application/manifest+json",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

fn cache_control_for(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("css") | Some("js") | Some("svg") => "public, max-age=31536000, immutable",
        Some("html") => "public, max-age=60",
        _ => "public, max-age=300",
    }
}

fn opensearch_response(config: &BlogConfig) -> Response {
    Response::new(StatusCode::OK)
        .with_header(
            "Content-Type",
            "application/opensearchdescription+xml; charset=utf-8",
        )
        .with_header("Cache-Control", "public, max-age=300")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(render_opensearch(config))
}

fn favicon_response() -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "image/svg+xml")
        .with_header("Cache-Control", "public, max-age=86400")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(edgerun_web_ui::FAVICON_COMPASS_SVG)
}

fn manifest_response(config: &BlogConfig) -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "application/manifest+json")
        .with_header("Cache-Control", "public, max-age=300")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(render_manifest(config))
}

fn render_manifest(config: &BlogConfig) -> String {
    format!(
        "{{\"name\":\"{}\",\"short_name\":\"{}\",\"start_url\":\"/\",\"scope\":\"/\",\"display\":\"minimal-ui\",\"background_color\":\"#f7f3eb\",\"theme_color\":\"#146c63\",\"icons\":[{{\"src\":\"/favicon.svg\",\"sizes\":\"any\",\"type\":\"image/svg+xml\"}}]}}",
        escape_json(&config.title),
        escape_json(&config.title)
    )
}

fn css_response() -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "text/css; charset=utf-8")
        .with_header("Cache-Control", "public, max-age=300")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(blog_style())
}

fn js_response() -> Response {
    Response::new(StatusCode::OK)
        .with_header("Content-Type", "application/javascript; charset=utf-8")
        .with_header("Cache-Control", "public, max-age=300")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_body(blog_js())
}

fn to_io_error(error: edgerun_http::io::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error.to_string())
}

fn blog_js() -> String {
    format!(
        "{}\n{}\n{}",
        edgerun_web_ui::THEME_TOGGLE_JS,
        edgerun_web_ui::WORKSPACE_JS,
        BLOG_JS
    )
}

fn blog_style() -> String {
    format!(
        "{}\n{}\n{}\n{}",
        edgerun_web_ui::BASE_STYLE,
        BLOG_STYLE,
        BLOG_UX_STYLE,
        BLOG_DASH_STYLE
    )
}

const BLOG_JS: &str = r#"
const searchInputs=[...document.querySelectorAll('#search, [data-workspace-search-scope]')];
const allCards=[...document.querySelectorAll('.post-card, .dash-post-card[data-search-card]')].map((card)=>({
  card,
  text:(card.dataset.searchText||card.dataset.search||'').toLowerCase(),
  topics:(card.dataset.topic||'').toLowerCase()
}));
const searchCounter=document.getElementById('search-count');
const dashboardCounters=[...document.querySelectorAll('[data-search-count]')];
const emptyMessages=[...document.querySelectorAll('[data-search-empty]')];
const topicButtons=[...document.querySelectorAll('[data-topic], [data-topic-filter]')];
let activeTopic='';

function updateButtons(topic){
  const normalized = (topic||'').toLowerCase();
  for(const button of topicButtons){
    const candidate=(button.dataset.topic||button.dataset.topicFilter||'').toLowerCase();
    button.setAttribute('aria-pressed', candidate===normalized ? 'true':'false');
  }
}

function matchTopics(topicList, topic){
  if(!topic) return true;
  if(!topicList) return false;
  return topicList===topic || topicList.split(' ').includes(topic);
}

function applyFilter(raw){
  const query=(raw||'').trim().toLowerCase();
  let shown=0;
  for(const item of allCards){
    const termMatch=!query || item.text.includes(query);
    const topicMatch=matchTopics(item.topics, activeTopic);
    const visible=termMatch && topicMatch;
    item.card.hidden=!visible;
    if(visible) shown += 1;
  }
  const counterLabel=shown === 1 ? ' post' : ' posts';
  if(searchCounter){ searchCounter.textContent=shown+' '+counterLabel; }
  for(const counter of dashboardCounters){ counter.textContent=shown+' '+counterLabel; }
  if(emptyMessages.length){
    for(const node of emptyMessages){ node.hidden=shown!==0; }
  }
  return shown;
}

function primaryInput(){
  return searchInputs.length ? searchInputs[0] : null;
}

function syncFromInput(event){
  const source = event.target;
  if(source && source.matches('[data-workspace-search-scope]')){ activeTopic=''; }
  const value = source ? (source.value || '') : '';
  applyFilter(value);
}

for(const input of searchInputs){
  input.addEventListener('input', syncFromInput);
}

const initialQuery=(new URLSearchParams(location.search)).get('q');
if(initialQuery && searchInputs.length){
  for(const input of searchInputs){ input.value=initialQuery; }
  applyFilter(initialQuery);
} else {
  applyFilter('');
}

for(const button of topicButtons){
  button.addEventListener('click',()=>{
    const topic=(button.dataset.topic||button.dataset.topicFilter||'').toLowerCase();
    activeTopic=topic;
    updateButtons(activeTopic);
    const input = primaryInput();
    if(input){
      input.value=topic;
      applyFilter(input.value);
      if(input.focus) input.focus();
    } else {
      applyFilter('');
    }
  });
}
updateButtons('');
"#;

const BLOG_STYLE: &str = r#"
.repo-stats{display:grid;grid-template-columns:minmax(130px,180px) minmax(0,1fr);gap:12px;max-width:760px;margin:28px 0 0}.repo-stats div{min-width:0;border:1px solid var(--line);border-radius:8px;background:var(--panel);padding:12px 14px}.repo-stats dt{color:var(--muted);font-size:12px;font-weight:800;text-transform:uppercase;letter-spacing:.08em}.repo-stats dd{margin:4px 0 0;font-weight:800;overflow-wrap:anywhere}.repo-stats .message{font-weight:650;color:var(--text)}.repo-stats .message span{color:var(--muted);font-weight:750;white-space:nowrap}.layout{display:grid;grid-template-columns:minmax(180px,240px) minmax(0,720px);gap:40px;align-items:start;margin:0;padding:34px clamp(18px,4vw,56px) 80px}.article-layout{display:grid;grid-template-columns:minmax(0,780px) 220px;gap:42px;align-items:start;max-width:1060px;margin:0 auto;padding:44px 18px 90px}.article-layout-single{display:block;max-width:820px}aside{color:var(--muted)}aside h2{margin:0 0 12px;color:var(--text);font-size:15px;text-transform:uppercase;letter-spacing:.08em}.topic-list{display:flex;flex-wrap:wrap;gap:8px}.topic-list button{display:inline-flex;gap:7px;align-items:center;border:1px solid var(--line);background:var(--panel);color:var(--text);border-radius:999px;padding:7px 10px;cursor:pointer}.topic-list button[aria-pressed=true]{border-color:var(--accent);background:color-mix(in srgb,var(--accent) 12%,var(--panel))}.topic-list span{color:var(--muted);font-size:13px;font-weight:750}.posts{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,320px),1fr));gap:20px;max-width:720px}.post-card{min-height:220px;background:var(--panel);border:1px solid var(--line);border-radius:8px;transition:transform .15s ease,border-color .15s ease}.post-card:hover{transform:translateY(-2px);border-color:var(--accent)}.post-card a{display:flex;min-height:100%;flex-direction:column;padding:22px;text-decoration:none}.date{color:var(--accent-2);font-size:14px;font-weight:750}.post-card h2{margin:12px 0 10px;font-size:24px;line-height:1.15;letter-spacing:0}.post-card p{margin:0 0 20px;color:var(--muted)}.tags{display:flex;gap:7px;flex-wrap:wrap;margin-top:auto}.tags span{border:1px solid var(--line);border-radius:999px;padding:3px 8px;color:var(--muted);font-size:13px}.article-stack{display:grid;gap:22px}.article{width:100%;background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:clamp(28px,5vw,52px)}.article h1{font-size:clamp(34px,5vw,58px);line-height:1;margin:10px 0 14px;letter-spacing:0}.summary{font-size:20px;color:var(--muted)}.back{justify-self:start;color:var(--accent);font-weight:800;text-decoration:none}.content{margin-top:32px}.content h1,.content h2,.content h3{line-height:1.15;margin:32px 0 10px;letter-spacing:0}.content p{margin:14px 0}.video-embed,.bench-chart{margin:28px 0}.video-embed iframe{display:block;width:100%;aspect-ratio:16/9;border:1px solid var(--line);border-radius:8px;background:var(--code)}.bench-chart{overflow-x:auto;border:1px solid var(--line);border-radius:8px;background:color-mix(in srgb,var(--panel) 86%,var(--code));padding:12px}.bench-chart svg{display:block;min-width:720px;width:100%;height:auto}.bench-chart rect{fill:var(--accent)}.bench-chart circle{fill:var(--accent-2)}.bench-chart line{stroke:var(--line);stroke-width:2}.bench-chart text{fill:var(--text);font:13px/1.3 ui-sans-serif,system-ui,sans-serif}.bench-chart text:first-of-type{font-weight:800;font-size:18px}.content pre{overflow:auto;background:var(--code);border-radius:8px;padding:16px}.code-ref{margin:22px 0}.code-ref figcaption{border:1px solid var(--line);border-bottom:0;border-radius:8px 8px 0 0;background:var(--panel);color:var(--muted);font-size:13px;padding:8px 12px}.code-ref figcaption a{color:var(--accent);font-weight:750;text-decoration:none}.code-ref pre{margin:0;border-radius:0 0 8px 8px}.content code{font-family:ui-monospace,SFMono-Regular,Consolas,monospace}.content blockquote{margin:22px 0;padding:4px 0 4px 18px;border-left:4px solid var(--accent);color:var(--muted)}.content table{width:100%;border-collapse:collapse;margin:24px 0;display:block;overflow-x:auto}.content th,.content td{border:1px solid var(--line);padding:9px 11px;text-align:left;vertical-align:top}.content th{background:color-mix(in srgb,var(--accent) 10%,var(--panel));font-weight:800}.content tr:nth-child(even) td{background:color-mix(in srgb,var(--panel) 78%,var(--code))}.crate-link{color:var(--accent);font-weight:750;text-decoration:none}.crate-link:hover{text-decoration:underline}.related-posts{margin-top:44px;border-top:1px solid var(--line);padding-top:24px}.related-posts h2{margin:0 0 14px;font-size:20px}.related-posts>div{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,220px),1fr));gap:20px}.related-posts article{border:1px solid var(--line);border-radius:8px;background:color-mix(in srgb,var(--panel) 88%,var(--code));transition:border-color .2s ease,transform .2s ease}.related-posts a{display:grid;gap:6px;padding:14px;text-decoration:none}.related-posts a:hover{border-color:var(--accent);transform:translateY(-1px)}.related-posts span{color:var(--accent-2);font-size:13px;font-weight:800}.related-posts strong{line-height:1.2}.related-posts small{color:var(--muted);font-size:13px}.recent{display:grid;gap:10px}.recent a{color:var(--muted);text-decoration:none}.muted{color:var(--muted)}
@media(max-width:900px){.article-layout{grid-template-columns:1fr;max-width:820px}.article-layout aside{order:-1}.recent{display:flex;flex-wrap:wrap;gap:14px}}@media(max-width:760px){.hero,.layout{grid-template-columns:1fr}.hero{padding-top:42px}.repo-stats{grid-template-columns:1fr}.posts{grid-template-columns:1fr}.article{padding:24px}}
@media(max-width:600px){body{overflow-x:hidden}.layout,.article-layout{padding-left:18px;padding-right:18px}.posts,.post-card{min-width:0}}
@media(prefers-reduced-motion:reduce){*,*::before,*::after{scroll-behavior:auto!important;transition:none!important;animation:none!important}}
"#;

const BLOG_UX_STYLE: &str = r#"
body{background:linear-gradient(180deg,color-mix(in srgb,var(--bg) 92%,var(--panel)) 0,var(--bg) 260px)}
.topbar{box-shadow:0 1px 0 color-mix(in srgb,var(--line) 80%,transparent)}
.brand{letter-spacing:0}.hero{display:grid;grid-template-columns:minmax(0,940px);justify-content:center;padding:54px clamp(18px,5vw,64px) 36px}.hero h1{max-width:900px;font-size:clamp(40px,6vw,76px)}.hero p{max-width:760px}.hero .inline-link{font-weight:800;color:var(--accent);text-decoration:none;border-bottom:1px solid color-mix(in srgb,var(--accent) 45%,transparent)}
.repo-stats{grid-template-columns:repeat(2,minmax(0,1fr));max-width:780px}.repo-stats div{background:color-mix(in srgb,var(--panel) 92%,var(--code));box-shadow:0 10px 28px color-mix(in srgb,var(--text) 7%,transparent)}
.layout{grid-template-columns:minmax(190px,250px) minmax(0,1fr);max-width:1180px;margin:0 auto;padding-top:42px}.layout aside{position:sticky;top:calc(var(--topbar-h) + 22px);border:1px solid var(--line);border-radius:8px;background:color-mix(in srgb,var(--panel) 90%,transparent);padding:16px}
.posts{max-width:none;grid-template-columns:repeat(auto-fit,minmax(min(100%,300px),1fr));gap:18px}.post-card{min-height:236px;box-shadow:0 12px 34px color-mix(in srgb,var(--text) 6%,transparent)}.post-card:hover{box-shadow:0 18px 44px color-mix(in srgb,var(--text) 10%,transparent)}.post-card a{padding:24px}.post-card h2{font-size:25px}.post-card p{line-height:1.5}
.tags span,.topic-list button{background:color-mix(in srgb,var(--panel) 82%,var(--code))}.topic-list button:hover{border-color:var(--accent);color:var(--accent)}
.article-layout{grid-template-columns:minmax(0,820px) 230px;max-width:1120px;padding-top:48px}.article{box-shadow:0 14px 42px color-mix(in srgb,var(--text) 7%,transparent)}.article h1{max-width:760px}.summary{max-width:720px;line-height:1.45}.content{font-size:17px;line-height:1.72}.content h2{font-size:clamp(24px,3vw,34px);margin-top:42px}.content h3{font-size:22px}.content p,.content ul,.content ol{max-width:720px}.content ul,.content ol{padding-left:24px}.content li{margin:7px 0}.content blockquote{max-width:760px;background:color-mix(in srgb,var(--accent) 7%,transparent);border-radius:0 8px 8px 0;padding:14px 18px}.content table{border-radius:8px}.content th{white-space:nowrap}.content code{background:color-mix(in srgb,var(--code) 80%,transparent);border-radius:5px;padding:1px 4px}.content pre code{background:transparent;padding:0}
.back{display:inline-flex;align-items:center;min-height:38px;border:1px solid var(--line);border-radius:8px;background:var(--panel);padding:0 12px}.back:hover{border-color:var(--accent)}
.recent{border-left:1px solid var(--line);padding-left:14px}.recent a:hover{color:var(--accent)}.related-posts article:hover{border-color:var(--accent);transform:translateY(-1px);box-shadow:0 8px 24px color-mix(in srgb,var(--text) 8%,transparent)}
@media(max-width:900px){.layout aside{position:static}.article-layout{padding-top:28px}.recent{border-left:0;padding-left:0}.article-layout aside{padding:0 4px}}
@media(max-width:760px){.hero{padding-top:34px}.layout{padding-top:24px}.layout aside{padding:14px}.repo-stats{grid-template-columns:1fr}.article{box-shadow:none}.content{font-size:16px}.content table{font-size:14px}.site-footer{position:static}body{padding-bottom:0}}
"#;

const BLOG_DASH_STYLE: &str = r#"
.dash-blog{max-width:1180px;margin:0 auto;padding:32px clamp(16px,4vw,56px) 90px;display:grid;gap:24px}
.dash-code-hero{display:grid;gap:8px}
.dash-code-tools{display:grid;gap:12px}
.dash-code-tools label{display:grid;gap:6px;color:var(--muted);font-weight:750}
.dash-code-tools label span{font-size:12px}
.dash-code-tools input{width:100%;min-height:44px;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--text);padding:10px 12px}
.dash-quick-links{display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:10px}
.dash-quick-links button{height:42px;display:inline-flex;align-items:center;justify-content:center;padding:0 12px}
.dash-code-summary{display:grid;grid-template-columns:repeat(auto-fit,minmax(130px,1fr));gap:12px}
.dash-code-summary>div{min-height:88px;border:1px solid var(--line);border-radius:10px;padding:14px;display:grid;align-content:center;background:color-mix(in srgb,var(--panel) 92%,var(--code))}
.dash-code-summary span{color:var(--muted);font-size:13px}
.dash-code-summary strong{font-size:28px;line-height:1}
.dash-code-summary .summary-wide{grid-column:1/-1}
.dash-analytics{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:14px}
.dash-chart-card{border:1px solid var(--line);border-radius:10px;padding:14px;background:color-mix(in srgb,var(--panel) 90%,var(--code));display:grid;gap:10px}
.dash-chart-card h3{margin:0;font-size:18px}
.dash-stat-chart{display:grid;gap:12px}
.dash-stat-row{display:grid;grid-template-columns:96px 1fr auto;gap:10px;align-items:center}
.dash-stat-label-wrap{display:grid;gap:4px}
.dash-stat-label{color:var(--muted);font-size:12px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.dash-stat-meta{font-size:11px;color:var(--accent-2)}
.dash-stat-track{height:12px;border-radius:999px;background:color-mix(in srgb,var(--line) 85%,transparent);overflow:hidden}
.dash-stat-fill{--dash-stat-fill:12%;display:block;height:100%;width:var(--dash-stat-fill);background:linear-gradient(90deg,color-mix(in srgb,var(--accent) 20%,var(--accent-2)),var(--accent));border-radius:999px}
.dash-stat-row strong{font-size:14px}
.dash-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:18px;align-items:stretch}
.dash-grid .dash-card-button,.dash-grid .dash-card,.dash-grid .dash-post-card{display:grid;border:1px solid var(--line);border-radius:10px;background:var(--panel);text-decoration:none;color:inherit;padding:16px;min-height:190px;gap:8px;transition:transform .15s ease,border-color .15s ease,box-shadow .15s ease}
.dash-grid .dash-card-button{cursor:pointer}
.dash-grid .dash-card-button:hover,.dash-grid .dash-card:hover{transform:translateY(-1px);border-color:var(--accent);box-shadow:0 10px 22px color-mix(in srgb,var(--text) 8%,transparent)}
.dash-grid .dash-card strong,.dash-grid .dash-card-button strong{display:block;font-size:21px;line-height:1.15}
.dash-grid .dash-card p,.dash-grid .dash-card span,.dash-grid .dash-card small,.dash-grid .dash-card-button p,.dash-grid .dash-card-button span,.dash-grid .dash-card-button small{color:var(--muted)}
.dash-grid .dash-card-date,.dash-grid .dash-card-button .date{color:var(--accent-2);font-size:14px;font-weight:750}
.dash-search-empty,[data-search-count]{color:var(--muted)}
.dash-article .content{display:grid;gap:18px}
.dash-article-content{display:grid;gap:18px}
.article .content .related-posts{margin-top:38px;padding-top:24px;border-top:1px solid var(--line)}
.article .content .related-posts>div{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,240px),1fr));gap:14px;margin-top:14px}
.article .content .related-posts article{margin:0}
.article .content .dash-related-post{border:1px solid var(--line);border-radius:10px;background:color-mix(in srgb,var(--panel) 88%,var(--code))}
.article .content .dash-related-post button{width:100%;min-height:0;padding:12px;display:grid;gap:8px;background:transparent;border:0}
.dash-code-tools .dash-link-button,
.dash-code-tools .dash-mail-button,
.dash-quick-links button{min-height:42px;border-radius:8px}
@media(max-width:900px){.dash-code-summary{grid-template-columns:repeat(auto-fit,minmax(120px,1fr));}.dash-grid .dash-card-button,.dash-grid .dash-card{min-height:168px}}
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
    fn renders_inline_code() {
        let html = markdown_to_html("Run `edgerun-server` before [open](https://example.com).");
        assert!(html.contains("<code>edgerun-server</code>"));
        assert!(html.contains("<a href=\"https://example.com\">open</a>"));
    }

    #[test]
    fn links_visible_crate_mentions_outside_code() {
        let html = "<p>Use edgerun-blog with <code>edgerun-git</code>.</p>";
        let linked = link_visible_crates(
            html,
            &["edgerun-blog".to_string(), "edgerun-git".to_string()],
        );
        assert!(
            linked.contains("href=\"https://git.edgerun.tech/edgerun_core/crates/edgerun-blog\"")
        );
        assert!(linked.contains("<code>edgerun-git</code>"));
        assert!(
            !link_visible_crates("<p>myedgerun-blog fork</p>", &["edgerun-blog".to_string()])
                .contains("crate-link")
        );
    }

    #[test]
    fn renders_related_posts_with_recent_fallback() {
        let post = Post {
            title: "Current".to_string(),
            path: "current".to_string(),
            source_path: PathBuf::new(),
            summary: "Current summary".to_string(),
            date: "2026-04-30".to_string(),
            author: "Ken".to_string(),
            tags: vec!["one".to_string()],
            body: "Current body".to_string(),
            html: String::new(),
            missing_front_matter: Vec::new(),
        };
        let other = Post {
            title: "Other".to_string(),
            path: "other".to_string(),
            source_path: PathBuf::new(),
            summary: "Other summary".to_string(),
            date: "2026-04-29".to_string(),
            author: "Ken".to_string(),
            tags: vec!["two".to_string()],
            body: "Other body".to_string(),
            html: String::new(),
            missing_front_matter: Vec::new(),
        };
        let html = render_related_posts(default_language(), &post, &[post.clone(), other]);
        assert!(html.contains("Related articles"));
        assert!(html.contains("/posts/other.html"));
    }

    #[test]
    fn renders_markdown_tables() {
        let html = markdown_to_html(
            "| Stack | Ops |\n| --- | ---: |\n| Edgerun | 342.71 |\n| Postfix | 351.69 |",
        );
        assert!(html.contains("<table><thead><tr>"));
        assert!(html.contains("<th>Stack</th>"));
        assert!(html.contains("<th style=\"text-align:right\">Ops</th>"));
        assert!(html.contains("<td style=\"text-align:right\">342.71</td>"));
    }

    #[test]
    fn passes_trusted_html_blocks() {
        let html = markdown_to_html("<table>\n<tr><td>OK</td></tr>\n</table>");
        assert!(html.contains("<table><tr><td>OK</td></tr></table>"));
        assert!(!html.contains("&lt;/table&gt;"));
    }

    #[test]
    fn passes_trusted_svg_blocks() {
        let html = markdown_to_html(
            "<svg viewBox=\"0 0 10 10\">\n<rect x=\"1\" y=\"1\" width=\"8\" height=\"8\"></rect>\n</svg>",
        );
        assert!(html.contains("<svg viewBox=\"0 0 10 10\"><rect"));
        assert!(!html.contains("&lt;svg"));
    }

    #[test]
    fn treats_mdx_as_content() {
        assert!(is_content_file(Path::new("post.mdx")));
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
            "2026-04-30-hello-world"
        );
        assert_eq!(
            slug_for_path(Path::new("posts/releases/email.md")),
            "releases/email"
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

    #[test]
    fn removes_title_heading_from_rendered_body() {
        let body = remove_leading_heading("# Hello\n\nBody.", "Hello");
        assert_eq!(body, "Body.");
        assert!(!markdown_to_html(body).contains("<h1>Hello</h1>"));
    }

    #[test]
    fn renders_youtube_links_as_embeds() {
        let html = markdown_to_html("https://www.youtube.com/watch?v=AIMdIAoiR80");
        assert!(html.contains("class=\"video-embed\""));
        assert!(html.contains("https://www.youtube-nocookie.com/embed/AIMdIAoiR80"));
        assert!(!html.contains("<p>https://www.youtube.com"));
    }

    fn write_localized_blog_fixture(source: &Path, include_about: bool) {
        for language in ["en", "th", "et"] {
            let dir = source.join(language);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("hello.md"),
                format!(
                    "---\ntitle: Hello {language}\ndate: 2026-04-30\nauthor: Ken\nsummary: One post\ntags: [test]\n---\n# Hello {language}\n\nBody."
                ),
            )
            .unwrap();
            if include_about {
                fs::write(
                    dir.join("about.md"),
                    format!(
                        "---\ntitle: About {language}\nsummary: Why Edgerun exists.\n---\n# About {language}\n\nEverything from scratch."
                    ),
                )
                .unwrap();
            }
        }
    }

    #[test]
    fn generates_static_site_files() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("edgerun-blog-test-{stamp}"));
        let source = base.join("source");
        let output = base.join("public");
        write_localized_blog_fixture(&source, true);

        let mut config = BlogConfig::new(&source);
        config.base_url = "https://blog.edgerun.tech".to_string();
        let site = generate_static_site(&config, &output).unwrap();

        assert_eq!(site.posts, 1);
        assert!(output.join("index.html").exists());
        assert!(output.join("about.html").exists());
        assert!(output.join("posts/hello.html").exists());
        assert!(output.join("th/index.html").exists());
        assert!(output.join("th/about.html").exists());
        assert!(output.join("th/posts/hello.html").exists());
        assert!(output.join("et/index.html").exists());
        assert!(output.join("et/about.html").exists());
        assert!(output.join("et/posts/hello.html").exists());
        assert!(output.join("sitemap.xml").exists());
        assert!(output.join("opensearch.xml").exists());
        assert!(fs::read_to_string(output.join("sitemap.xml"))
            .unwrap()
            .contains("/th/about.html"));
        assert!(fs::read_to_string(output.join("search.json"))
            .unwrap()
            .contains("\"title\":\"Hello en\""));
        let post_html = fs::read_to_string(output.join("posts/hello.html")).unwrap();
        assert_eq!(post_html.matches("<h1").count(), 1);
        assert!(!post_html.contains("aria-describedby=\"search-count\""));
        let et_index = fs::read_to_string(output.join("et/index.html")).unwrap();
        assert!(et_index.contains("href=\"/et/posts/hello.html\""));
        assert!(!et_index.contains("/posts//et/posts/hello.html.html"));

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn check_reports_stale_static_site() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("edgerun-blog-check-test-{stamp}"));
        let source = base.join("source");
        let output = base.join("public");
        write_localized_blog_fixture(&source, false);
        let config = BlogConfig::new(&source);
        generate_static_site(&config, &output).unwrap();
        check_static_site(&config, &output).unwrap();
        fs::write(output.join("posts/old.html"), "stale").unwrap();
        let error = check_static_site(&config, &output).unwrap_err();
        assert!(error.to_string().contains("generated output is stale"));
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn validates_publishable_posts() {
        let posts = vec![Post {
            title: "No date".to_string(),
            path: "same".to_string(),
            source_path: PathBuf::from("posts/a.md"),
            summary: "Summary".to_string(),
            date: String::new(),
            author: String::new(),
            tags: Vec::new(),
            body: String::new(),
            html: String::new(),
            missing_front_matter: vec![
                "date".to_string(),
                "author".to_string(),
                "tags".to_string(),
            ],
        }];
        let error = validate_content(&posts, None).unwrap_err();
        assert!(error.to_string().contains("invalid or missing date"));
        assert!(error.to_string().contains("missing required front matter"));
    }
}
