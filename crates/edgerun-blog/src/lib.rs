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
    files.push(generated_file(output.join("favicon.svg"), FAVICON_SVG));
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
    files.push(generated_file(output.join("style.css"), STYLE));
    files.push(generated_file(output.join("app.js"), APP_JS));

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

fn load_git_stats(root: &Path) -> GitStats {
    GitStats {
        commit_count: git_output(root, &["rev-list", "--count", "HEAD"]),
        last_commit_epoch: git_output(root, &["log", "-1", "--format=%ct"])
            .and_then(|value| value.parse().ok()),
        last_commit_message: git_output(root, &["log", "-1", "--format=%s"]),
    }
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
            "<main id=\"content\" class=\"article-layout\" tabindex=\"-1\"><div class=\"article-stack\"><a class=\"back\" href=\"{}\">{}</a><article class=\"article\" aria-labelledby=\"post-title\"><p class=\"date\"><time datetime=\"{}\">{}</time> by <span class=\"author\">{}</span></p><h1 id=\"post-title\">{}</h1><p class=\"summary\">{}</p><div class=\"tags\">{}</div><div class=\"content\">{}</div></article></div><aside aria-label=\"{}\"><h2>{}</h2><nav class=\"recent\" aria-label=\"{}\">{}</nav></aside></main>",
            escape_attr(&localized_path(language, "/")),
            escape_html(language.back_label),
            escape_attr(&post.date),
            escape_html(&post.date),
            escape_html(&post.author),
            escape_html(&post.title),
            escape_html(&post.summary),
            render_tags(&post.tags),
            post.html,
            escape_attr(language.recent_label),
            escape_html(language.recent_label),
            escape_attr(language.recent_label),
            nav
        ),
    )
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
    let mut head = format!(
        "<meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"color-scheme\" content=\"light dark\"><meta name=\"theme-color\" content=\"#146c63\"><meta name=\"referrer\" content=\"strict-origin-when-cross-origin\"><meta name=\"generator\" content=\"edgerun-blog\"><title>{}</title><meta name=\"description\" content=\"{}\">",
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
    let script_src = format!("/app.js?v={}", asset_version(APP_JS));
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
    head.push_str(&format!(
        "<link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\"><link rel=\"manifest\" href=\"/site.webmanifest\"><link rel=\"search\" type=\"application/opensearchdescription+xml\" href=\"/opensearch.xml\"><link rel=\"alternate\" type=\"application/atom+xml\" href=\"/feed.xml\"><style>{}</style>",
        STYLE
    ));
    let language_links = render_language_links(language, route);
    format!(
        "<!doctype html><html lang=\"{}\"><head>{}</head><body><a class=\"skip-link\" href=\"#content\">Skip to content</a><header class=\"topbar\"><a class=\"brand\" href=\"{}\" aria-label=\"{} home\">{}</a><form class=\"header-search\" role=\"search\" action=\"{}\" method=\"get\"><label for=\"search\">{}</label><input id=\"search\" name=\"q\" type=\"search\" placeholder=\"{}\" autocomplete=\"off\"><button type=\"submit\" title=\"{}\" aria-label=\"{}\"><svg aria-hidden=\"true\" viewBox=\"0 0 24 24\"><circle cx=\"11\" cy=\"11\" r=\"7\"></circle><path d=\"m16 16 4 4\"></path></svg></button></form><nav aria-label=\"Theme\"><er-theme-toggle></er-theme-toggle></nav></header>{}<footer class=\"site-footer\"><a href=\"{}\">{}</a><a href=\"{}\">{}</a>{}</footer><script src=\"{}\" defer></script></body></html>",
        escape_attr(language.html_lang),
        head,
        escape_attr(&localized_path(language, "/")),
        escape_attr(language.title),
        escape_html(language.title),
        escape_attr(&localized_path(language, "/")),
        escape_html(language.search_label),
        escape_attr(language.search_label),
        escape_attr(language.search_label),
        escape_attr(language.search_label),
        body,
        escape_attr(&localized_path(language, "/about.html")),
        escape_html(language.about_label),
        escape_attr(&localized_path(language, "/feed.xml")),
        escape_html(language.feed_label),
        language_links,
        escape_attr(&script_src)
    )
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
        .with_body(FAVICON_SVG)
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
if(!customElements.get('er-theme-toggle')){customElements.define('er-theme-toggle',class extends HTMLElement{connectedCallback(){this.attachShadow({mode:'open'}).innerHTML='<style>button{width:44px;height:44px;display:grid;place-items:center;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--text);cursor:pointer;font:24px/1 system-ui}button:hover{border-color:var(--accent)}</style><button type="button"></button>';const btn=this.shadowRoot.querySelector('button');const current=()=>root.dataset.theme||(matchMedia('(prefers-color-scheme:dark)').matches?'dark':'light');const render=()=>{const dark=current()==='dark';btn.textContent=dark?'☾':'☀';btn.title=dark?'Dark mode: switch to light mode':'Light mode: switch to dark mode';btn.setAttribute('aria-label',btn.title)};btn.onclick=()=>{const next=current()==='dark'?'light':'dark';root.dataset.theme=next;localStorage.setItem('theme',next);render()};render()}})}
const search=document.getElementById('search');
const cards=[...document.querySelectorAll('.post-card')];
const count=document.getElementById('search-count');
const topicButtons=[...document.querySelectorAll('[data-topic]')];
function applyFilter(term){const q=term.trim().toLowerCase();let shown=0;for(const card of cards){const ok=!q||card.dataset.search.includes(q);card.hidden=!ok;if(ok)shown++}if(count){count.textContent=shown+' post'+(shown===1?'':'s')}for(const btn of topicButtons){btn.setAttribute('aria-pressed',btn.dataset.topic.toLowerCase()===q?'true':'false')}}
if(search){search.addEventListener('input',e=>applyFilter(e.target.value))}
const initialQuery=new URLSearchParams(location.search).get('q');
if(search&&initialQuery){search.value=initialQuery;applyFilter(initialQuery)}
for(const btn of topicButtons){btn.addEventListener('click',()=>{if(search){search.value=btn.dataset.topic;applyFilter(btn.dataset.topic);search.focus()}})}
"#;

const STYLE: &str = r#"
:root{color-scheme:light dark;--bg:#f7f3eb;--panel:#fffdf8;--text:#1c2430;--muted:#627084;--line:#d8cfc0;--accent:#146c63;--accent-ink:#f4fffb;--accent-2:#8b3f2f;--code:#eee6d8}
:root[data-theme=dark]{--bg:#101418;--panel:#171d22;--text:#f2ede4;--muted:#a5b2bf;--line:#2b353d;--accent:#6fc7b8;--accent-ink:#06201d;--accent-2:#dfa06b;--code:#232b31}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:16px/1.6 ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}a{color:inherit}:focus-visible{outline:3px solid var(--accent);outline-offset:3px}.skip-link{position:absolute;left:12px;top:-60px;z-index:10;background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:8px 12px}.skip-link:focus{top:12px}.topbar{position:sticky;top:0;z-index:2;display:grid;grid-template-columns:max-content minmax(220px,560px) 1fr max-content;gap:14px;align-items:center;padding:12px clamp(14px,3vw,44px);background:color-mix(in srgb,var(--bg) 88%,transparent);border-bottom:1px solid var(--line);backdrop-filter:blur(12px)}.brand{font-weight:800;text-decoration:none;white-space:nowrap}.topbar nav{grid-column:4;display:flex;align-items:center;justify-content:end}.header-search{position:relative;min-width:0}.header-search label{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}.header-search input{width:100%;min-width:0;height:44px;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--text);padding:10px 44px 10px 13px}.header-search input:focus{border-color:var(--accent)}.header-search button{position:absolute;right:4px;top:4px;width:36px;height:36px;display:grid;place-items:center;border:0;border-radius:6px;background:transparent;color:var(--muted);cursor:pointer}.header-search button:hover{color:var(--accent);background:color-mix(in srgb,var(--accent) 10%,transparent)}.header-search svg{width:20px;height:20px;fill:none;stroke:currentColor;stroke-width:2;stroke-linecap:round}button,input{font:inherit}.hero{padding:64px clamp(18px,4vw,56px) 42px;border-bottom:1px solid var(--line)}.hero h1{margin:0;font-size:clamp(42px,7vw,82px);line-height:.95;letter-spacing:0}.hero p{max-width:720px;color:var(--muted);font-size:19px}.eyebrow{margin:0 0 12px;color:var(--accent);font-weight:800;text-transform:uppercase;font-size:13px;letter-spacing:.08em}.repo-stats{display:grid;grid-template-columns:minmax(130px,180px) minmax(0,1fr);gap:12px;max-width:760px;margin:28px 0 0}.repo-stats div{min-width:0;border:1px solid var(--line);border-radius:8px;background:var(--panel);padding:12px 14px}.repo-stats dt{color:var(--muted);font-size:12px;font-weight:800;text-transform:uppercase;letter-spacing:.08em}.repo-stats dd{margin:4px 0 0;font-weight:800;overflow-wrap:anywhere}.repo-stats .message{font-weight:650;color:var(--text)}.repo-stats .message span{color:var(--muted);font-weight:750;white-space:nowrap}.layout{display:grid;grid-template-columns:minmax(180px,240px) minmax(0,720px);gap:40px;align-items:start;margin:0;padding:34px clamp(18px,4vw,56px) 80px}.article-layout{display:grid;grid-template-columns:minmax(0,780px) 220px;gap:42px;align-items:start;max-width:1060px;margin:0 auto;padding:44px 18px 90px}.article-layout-single{display:block;max-width:820px}aside{color:var(--muted)}aside h2{margin:0 0 12px;color:var(--text);font-size:15px;text-transform:uppercase;letter-spacing:.08em}.topic-list{display:flex;flex-wrap:wrap;gap:8px}.topic-list button{display:inline-flex;gap:7px;align-items:center;border:1px solid var(--line);background:var(--panel);color:var(--text);border-radius:999px;padding:7px 10px;cursor:pointer}.topic-list button[aria-pressed=true]{border-color:var(--accent);background:color-mix(in srgb,var(--accent) 12%,var(--panel))}.topic-list span{color:var(--muted);font-size:13px;font-weight:750}.posts{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,320px),1fr));gap:16px;max-width:720px}.post-card{min-height:220px;background:var(--panel);border:1px solid var(--line);border-radius:8px;transition:transform .15s ease,border-color .15s ease}.post-card:hover{transform:translateY(-2px);border-color:var(--accent)}.post-card a{display:flex;min-height:100%;flex-direction:column;padding:22px;text-decoration:none}.date{color:var(--accent-2);font-size:14px;font-weight:750}.post-card h2{margin:12px 0 10px;font-size:24px;line-height:1.15;letter-spacing:0}.post-card p{margin:0 0 20px;color:var(--muted)}.tags{display:flex;gap:7px;flex-wrap:wrap;margin-top:auto}.tags span{border:1px solid var(--line);border-radius:999px;padding:3px 8px;color:var(--muted);font-size:13px}.article-stack{display:grid;gap:14px}.article{width:100%;background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:clamp(28px,5vw,52px)}.article h1{font-size:clamp(34px,5vw,58px);line-height:1;margin:10px 0 14px;letter-spacing:0}.summary{font-size:20px;color:var(--muted)}.back{justify-self:start;color:var(--accent);font-weight:800;text-decoration:none}.content{margin-top:32px}.content h1,.content h2,.content h3{line-height:1.15;margin:32px 0 10px;letter-spacing:0}.content p{margin:14px 0}.video-embed{margin:28px 0}.video-embed iframe{display:block;width:100%;aspect-ratio:16/9;border:1px solid var(--line);border-radius:8px;background:var(--code)}.content pre{overflow:auto;background:var(--code);border-radius:8px;padding:16px}.code-ref{margin:22px 0}.code-ref figcaption{border:1px solid var(--line);border-bottom:0;border-radius:8px 8px 0 0;background:var(--panel);color:var(--muted);font-size:13px;padding:8px 12px}.code-ref figcaption a{color:var(--accent);font-weight:750;text-decoration:none}.code-ref pre{margin:0;border-radius:0 0 8px 8px}.content code{font-family:ui-monospace,SFMono-Regular,Consolas,monospace}.content blockquote{margin:22px 0;padding:4px 0 4px 18px;border-left:4px solid var(--accent);color:var(--muted)}.recent{display:grid;gap:10px}.recent a{color:var(--muted);text-decoration:none}.empty{max-width:720px;margin:80px auto;padding:0 18px}.muted{color:var(--muted)}.site-footer{display:flex;gap:18px;align-items:center;flex-wrap:wrap;border-top:1px solid var(--line);padding:22px clamp(18px,4vw,56px);color:var(--muted)}.site-footer a{text-decoration:none}.language-links{display:flex;gap:10px;margin-left:auto}.language-links a{font-weight:750;text-transform:uppercase}.language-links a[aria-current=true]{color:var(--accent)}
@media(max-width:900px){.article-layout{grid-template-columns:1fr;max-width:820px}.article-layout aside{order:-1}.recent{display:flex;flex-wrap:wrap;gap:14px}}@media(max-width:760px){.hero,.layout{grid-template-columns:1fr}.hero{padding-top:42px}.repo-stats{grid-template-columns:1fr}.posts{grid-template-columns:1fr}.article{padding:24px}}
@media(max-width:600px){body{overflow-x:hidden}.topbar{position:static;display:flex;flex-wrap:wrap;gap:12px;padding:12px 14px}.brand{flex:1 1 auto}.topbar nav{flex:0 0 auto;margin-left:auto}.header-search{order:2;flex:1 0 100%;width:100%}.layout,.article-layout{padding-left:18px;padding-right:18px}.posts,.post-card{min-width:0}}
@media(prefers-reduced-motion:reduce){*,*::before,*::after{scroll-behavior:auto!important;transition:none!important;animation:none!important}}
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
