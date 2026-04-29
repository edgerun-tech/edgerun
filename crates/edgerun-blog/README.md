# edgerun-blog

`edgerun-blog` is an implemented host-only static blog server for a Git
checkout. It has no build step, no authentication, and no non-Edgerun crate
dependencies.

The crate scans Markdown and HTML files from the configured root at request
time, renders a simple organized layout, serves a client-side search index, and
supports light and dark modes.

Run it with:

```bash
cargo run -p edgerun-blog -- \
  --root /srv/blog \
  --bind 127.0.0.1:8088 \
  --title "Edgerun Blog" \
  --base-url https://blog.edgerun.tech
```

The Git-backed content model is implemented in code. Hosting
`blog.edgerun.tech` on the current mail host is currently blocked on adding DNS
for `blog.edgerun.tech` and routing that host through the active
`edgerun-mail-server` HTTP listener or another frontend bound to ports 80/443.
