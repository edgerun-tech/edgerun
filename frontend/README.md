# Edgerun Frontend (Static Solid Build)

This frontend is built as a static site using:
- SolidJS `renderToString` for server-side HTML generation
- Tailwind CSS CLI for stylesheet output
- esbuild for native ESM browser scripts
- Bun for package/runtime tooling

## Prerequisites

```bash
bun install
```

## Build

```bash
bun run build
```

Build output:
- `../out/frontend/site/` static website output
- `../out/frontend/wiki/` versioned markdown docs for wiki sync
- `../out/frontend/tmp/` temporary build artifacts
- `../out/frontend/generated/` generated auxiliary snapshots

## Quality checks

```bash
bun run check
```

This runs:
- TypeScript type checks (`bun run typecheck`)
- ESLint (`bun run lint`)
- style-guide enforcement (`bun run style-guide:check`)

## End-to-end tests (Cypress)

Run full frontend E2E coverage (core + routed terminal stack):

```bash
bun run e2e:run
```

Notes:
- `e2e:run` requires an existing frontend build in `../out/frontend/site/index.html`.
- The workflow runs:
- core specs against static output on `http://127.0.0.1:4173`
- routed terminal compose/local stack spec via scheduler + term-server harness

Run only core Cypress specs (without routed terminal stack harness):

```bash
bun run e2e:core
```

Run only routed terminal stack E2E:

```bash
bun run e2e:compose
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
- `SOLANA_CLUSTER`
- `SOLANA_RPC_URL`
- `EDGERUN_TREASURY_ACCOUNT`

These values are embedded in pages and release artifacts.
