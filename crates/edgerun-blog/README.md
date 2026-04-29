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

The Git-backed content model is implemented in code. On the current mail host,
DNS for `blog.edgerun.tech` is served from the Edgerun mail server's own
`DnsZone` config. Public hosting should route that host through an Edgerun HTTP
listener, either by mounting `BlogHandler` in the existing mail server process or
by running this binary on its own Edgerun-managed listener.
