# edgerun-blog

`edgerun-blog` is an implemented host-only static blog server/generator for a
Git checkout. It has no authentication and no non-Edgerun crate dependencies.

The crate scans Markdown and HTML files from the configured root at request
time, or renders the same deterministic output into a static directory for a
Git hook. It serves a simple organized layout, a client-side search index, and
light/dark modes.

Baseline site files are implemented in code: `/favicon.svg`, `/robots.txt`,
`/sitemap.xml`, `/site.webmanifest`, `/feed.xml`, `/style.css`, `/app.js`, and
`/search.json`. HTML pages include canonical URLs, description metadata,
Open Graph/Twitter summary metadata, accessible landmarks, and a skip link.

Posts are intended to live in the same Git checkout as the code. Fenced code
blocks can carry real source pointers:

````markdown
```rust path=crates/edgerun-server/src/bin/edgerun-server.rs commit=<git-sha> lines=400-460
// excerpt copied from that exact commit
```
````

When `commit` is present, the rendered caption links to the future
`git.edgerun.tech` code explorer using the commit hash, path, and first line.
Until that explorer exists, the post still preserves the source identity in the
HTML.

Run it with:

```bash
cargo run -p edgerun-blog -- serve \
  --root /srv/blog \
  --static-root /srv/blog-public \
  --bind 127.0.0.1:8088 \
  --title "Edgerun Blog" \
  --base-url https://blog.edgerun.tech
```

Generate static output with:

```bash
cargo run -p edgerun-blog -- generate \
  --root /srv/blog \
  --out /srv/blog-public \
  --title "Edgerun Blog" \
  --base-url https://blog.edgerun.tech
```

The generator is intended for host automation such as a Git `post-receive`
hook. It rewrites the generated `posts/` directory and emits `index.html`,
`favicon.svg`, `robots.txt`, `sitemap.xml`, `site.webmanifest`, `feed.xml`,
`style.css`, `app.js`, and `search.json` from the current checkout. A sample
hook lives at `deploy/server/edgerun-blog-post-receive.sample`. When
`--static-root` is configured, the HTTP handler serves those generated files
first and falls back to live rendering if a file is missing.

The Git-backed content model is implemented in code. On the current mail host,
DNS for `blog.edgerun.tech` is served from the Edgerun server's own
`DnsZone` config. Public hosting should route that host through an Edgerun HTTP
listener, either by mounting `BlogHandler` in the existing `edgerun-server`
process or by running this binary on its own Edgerun-managed listener.
