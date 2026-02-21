# Edgerun Frontend (Static Solid Build)

This frontend is built as a static site using:
- SolidJS `renderToString` for server-side HTML generation
- Tailwind CSS CLI for stylesheet output
- esbuild for native ESM browser scripts
- Bun for package/runtime tooling

## Build

```bash
bun install
bun run build
```

Build output:
- `../out/frontend/site/` static website for GitHub Pages
- `../out/frontend/wiki/` versioned markdown docs for GitHub wiki sync
- `../out/frontend/tmp/` temporary build artifacts
- `../out/frontend/generated/` generated auxiliary snapshots

## Quality checks

```bash
bun run check
```

## Build metadata

The generator reads environment variables:
- `EDGERUN_VERSION`
- `EDGERUN_VERSIONS`
- `EDGERUN_BUILD_NUMBER`
- `EDGERUN_SITE_URL`
- `EDGERUN_SITE_DOMAIN`
- `EDGERUN_FRONTEND_DIST_ROOT`
- `EDGERUN_FRONTEND_WIKI_ROOT`

These values are embedded in pages and release artifacts.
