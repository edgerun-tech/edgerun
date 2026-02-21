# Edgerun Static Site

Static site generator based on:

- Solid SSR (`renderToString`)
- Tailwind CLI
- esbuild
- native browser ESM modules

## Build

```bash
cd site
bun install
bun run build
```

Outputs:

- `site/dist/` static site (for GitHub Pages)
- `site/wiki/` generated wiki markdown content

Build metadata inputs:

- `EDGERUN_VERSION`
- `EDGERUN_BUILD_NUMBER`
- `EDGERUN_VERSIONS`
- `EDGERUN_SITE_URL`
- `EDGERUN_SITE_DOMAIN`

