# edgerun-blog

`edgerun-blog` is an implemented host-only static blog server/generator for a
Git checkout. It has no authentication and no non-Edgerun crate dependencies.

The crate reads Git statistics from the configured repo root and scans Markdown
and HTML files from a configured content directory inside that checkout. It can
render the same deterministic output into a static directory for a Git hook. It
serves a simple organized layout, a client-side search index, and light/dark
modes.

Content is mandatory in three languages. The configured content directory must
contain matching `en`, `th`, and `et` subdirectories. English is the default and
keeps the root URLs; Thai is emitted under `/th/`; Estonian is emitted under
`/et/`. Post slugs and the about page must exist in all three languages.

Baseline site files are implemented in code: `/about.html`, `/favicon.svg`,
`/robots.txt`, `/sitemap.xml`, `/opensearch.xml`, `/site.webmanifest`,
`/feed.xml`, `/style.css`, `/app.js`, and `/search.json`. HTML pages include
canonical URLs, description metadata, Open Graph/Twitter summary metadata,
JSON-LD, accessible landmarks, and a skip link.

Post front matter is mandatory:

```yaml
---
title: Email Release
date: 2026-04-30
author: Ken
summary: Notes about the email feature.
tags: [email, release]
---
```

The optional about page is sourced from `about.md`, `about.markdown`, or
`about.html` in each language directory. Its front matter must include `title`
and `summary`.

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
  --root /srv/edgerun_core \
  --content-dir docs/blog \
  --static-root /srv/blog/.generated \
  --bind 127.0.0.1:8088 \
  --title "EdgeRun Build Log" \
  --base-url https://blog.edgerun.tech
```

Generate static output with:

```bash
cargo run -p edgerun-blog -- generate \
  --root /srv/edgerun_core \
  --content-dir docs/blog \
  --out /path/to/blog/.generated \
  --title "EdgeRun Build Log" \
  --base-url https://blog.edgerun.tech
```

Check committed generated output with:

```bash
cargo run -p edgerun-blog -- generate \
  --root /srv/edgerun_core \
  --content-dir docs/blog \
  --out /path/to/blog/.generated \
  --base-url https://blog.edgerun.tech \
  --check
```

The generator is intended for developer-side automation such as a Git
`pre-commit` hook. It rewrites the generated `posts/` directory and emits
`index.html`, `about.html` when present, `favicon.svg`, `robots.txt`,
`sitemap.xml`, `opensearch.xml`, `site.webmanifest`, `feed.xml`, `style.css`,
`app.js`, and `search.json` from the current checkout. Generation validates
publishable posts for duplicate slugs and mandatory `title`, `date`, `author`,
`summary`, and `tags` front matter. A sample hook lives at
`crates/edgerun-blog/hooks/pre-commit.sample`. When `--static-root` is
configured, the HTTP handler serves those generated files first and falls back
to live rendering if a file is missing.

The Git-backed content model is implemented in code. On the current mail host,
DNS for `blog.edgerun.tech` is served from the Edgerun server's own
`DnsZone` config. Public hosting should route that host through an Edgerun HTTP
listener, either by mounting `BlogHandler` in the existing `edgerun-server`
process or by running this binary on its own Edgerun-managed listener.
