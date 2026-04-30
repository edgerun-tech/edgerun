# edgerun-git

`edgerun-git` is implemented host-only code for serving public slices of Git
repositories through Edgerun HTTP. Repositories are hidden unless they opt in
with `.edgerun/git.yaml`, and files or directories are hidden unless released
with `.gitvisible`.

The crate explorer is backed by generated type/catalog material in
`.edgerun/git/crates/*.txt`. That catalog is produced on the developer machine:

```bash
cargo run -p edgerun-git -- generate \
  --repo . \
  --out .edgerun/git/crates
```

Check committed catalog freshness with:

```bash
cargo run -p edgerun-git -- generate \
  --repo . \
  --out .edgerun/git/crates \
  --check
```

The sample developer-side hook lives at
`crates/edgerun-git/hooks/pre-commit.sample`. Serving reads the generated
catalog first so request handling stays small; live source parsing exists only
as a fallback when catalog files are missing during local development.
