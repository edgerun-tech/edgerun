//! Dashboard surface render functions.
//!
//! Surfaces are HTML fragments injected into the workspace via HTMX-like fetch.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use edgerun_web_ui::escape_attr;
use edgerun_web_ui::escape_html;

pub mod apps;
pub mod mail;

/// Generic surface wrapper with title and subtitle header.
pub fn render_surface(label: &str, subtitle: &str, content: &str) -> String {
    format!(
        "<section class=\"dash-stage\" aria-label=\"Workspace surface\">\
            <header><div>\
                <strong id=\"surfaceTitle\">{}</strong>\
                <span id=\"surfaceUrl\">{}</span>\
            </div></header>\
            <div class=\"dash-surface\">{}</div>\
        </section>",
        escape_html(label),
        escape_attr(subtitle),
        content,
    )
}

/// Blog (build log) surface with quick-link cards.
pub fn render_blog_surface() -> String {
    render_surface(
        "Build Log",
        "blog.edgerun.tech",
        "<div class=\"dash-quick-links\">\
            <button class=\"dash-card dash-card-button\" type=\"button\" \
                data-surface-module=\"build-log\" data-surface=\"build-log\" \
                hx-get=\"/surface/blog/posts/edgerun-onboarding.html\" data-search-text=\"\">\
                <strong>Onboarding</strong>\
                <span>Start with architecture, capabilities, and workflow.</span>\
            </button>\
            <button class=\"dash-card dash-card-button\" type=\"button\" \
                data-surface-module=\"build-log\" data-surface=\"build-log\" \
                hx-get=\"/surface/blog/posts/build-your-own-edgerun-app.html\" data-search-text=\"\">\
                <strong>Build your own app</strong>\
                <span>Step-by-step guide for real app onboarding.</span>\
            </button>\
            <div class=\"dash-code-tools\" aria-label=\"Blog tools\">\
                <button class=\"dash-card dash-card-button\" type=\"button\" \
                    data-surface-module=\"build-log\" data-surface=\"build-log\" \
                    hx-get=\"/surface/blog\" data-search-text=\"\">\
                    <strong>Latest posts</strong>\
                    <span>Follow feature-by-feature work as it lands.</span>\
                </button>\
                <button class=\"dash-card dash-card-button\" type=\"button\" \
                    data-surface-module=\"build-log\" data-surface=\"build-log\" \
                    hx-get=\"/surface/blog/about\" data-search-text=\"\">\
                    <strong>About Edgerun</strong>\
                    <span>The philosophy and direction behind the project.</span>\
                </button>\
                <button class=\"dash-card dash-card-button\" type=\"button\" \
                    data-surface-module=\"build-log\" data-surface=\"build-log\" \
                    hx-get=\"/surface/blog/feed\" data-search-text=\"\">\
                    <strong>Feed</strong>\
                    <span>Subscribe to release notes and build notes.</span>\
                </button>\
            </div>\
        </div>",
    )
}

/// Code (git) surface with repository cards.
pub fn render_code_surface() -> String {
    render_surface(
        "Code",
        "git.edgerun.tech",
        "<div class=\"dash-grid\">\
            <button class=\"dash-card dash-card-button\" type=\"button\" \
                data-surface-module=\"code\" data-surface=\"code\" \
                hx-get=\"/surface/git\" data-search-text=\"\">\
                <strong>Repositories</strong>\
                <span>Browse released source surfaces.</span>\
            </button>\
            <button class=\"dash-card dash-card-button\" type=\"button\" \
                data-surface-module=\"code\" data-surface=\"code\" \
                hx-get=\"/surface/git/crates\" data-search-text=\"\">\
                <strong>Crate explorer</strong>\
                <span>Navigate visible crates, metadata, APIs, and relationships.</span>\
            </button>\
            <button class=\"dash-card dash-card-button\" type=\"button\" \
                data-surface-module=\"code\" data-surface=\"code\" \
                hx-get=\"/surface/git/source\" data-search-text=\"\">\
                <strong>Source tree</strong>\
                <span>Open the public source tree directly.</span>\
            </button>\
        </div>",
    )
}

/// Status footer with session counters, RPS, memory, CPU, and chat dock.
pub fn render_status_footer() -> String {
    String::from(
        "<footer class=\"site-footer dash-status-footer\" aria-label=\"Server status\">\
            <div class=\"dash-status\" data-dash-status>\
                <span>sessions <strong data-status-sessions>--</strong></span>\
                <span>req/s <strong data-status-rps>--</strong></span>\
                <span>mem <strong data-status-memory>--</strong></span>\
                <span>cpu <strong data-status-cpu>--</strong></span>\
                <span>bin <strong data-status-binary>--</strong></span>\
                <div class=\"dash-status-actions\">\
                    <section id=\"dashChatDock\" class=\"dash-chat-dock is-open\" aria-label=\"Global chat dock\">\
                        <button id=\"dashChatBubble\" class=\"dash-chat-bubble\" type=\"button\" \
                            data-chat-action=\"open\" title=\"Open chat\" aria-label=\"Open global chat\">\
                            <span aria-hidden=\"true\">💬</span>\
                        </button>\
                        <section id=\"dashChatPanel\" class=\"dash-chat-panel\">\
                            <header class=\"dash-chat-panel-head\">\
                                <h2>Global chat</h2>\
                                <div class=\"dash-chat-panel-controls\">\
                                    <button id=\"dashChatMinimize\" class=\"dash-chat-panel-button\" type=\"button\" \
                                        data-chat-action=\"minimize\" aria-label=\"Minimize chat\">▁</button>\
                                    <button id=\"dashChatClose\" class=\"dash-chat-panel-button\" type=\"button\" \
                                        data-chat-action=\"close\" aria-label=\"Close chat\">×</button>\
                                </div>\
                            </header>\
                            <div class=\"dash-chat-shell\">\
                                <form id=\"dashChatForm\" class=\"dash-chat-form\" autocomplete=\"off\">\
                                    <label for=\"dashChatName\"><span>Name</span>\
                                        <input id=\"dashChatName\" required maxlength=\"24\" placeholder=\"your name\">\
                                    </label>\
                                    <label for=\"dashChatMessage\"><span>Message</span>\
                                        <textarea id=\"dashChatMessage\" required maxlength=\"800\" \
                                            placeholder=\"Say something to everyone\"></textarea>\
                                    </label>\
                                    <button id=\"dashChatSend\" type=\"submit\" aria-label=\"Post message\">Post</button>\
                                </form>\
                                <p class=\"dash-chat-status\" id=\"dashChatStatus\" role=\"status\"></p>\
                                <div id=\"dashChatLog\" class=\"dash-chat-log\"></div>\
                            </div>\
                        </section>\
                    </section>\
                    <button type=\"button\" class=\"dash-status-refresh\" data-status-refresh \
                        title=\"Refresh status\" aria-label=\"Refresh status\">\
                        <span aria-hidden=\"true\">⟳</span>\
                    </button>\
                    <span class=\"dash-status-theme\"><er-theme-toggle></er-theme-toggle></span>\
                </div>\
            </div>\
        </footer>",
    )
}
