# Cloudflare Frontend Target Map

- Site frontend (`frontend/` static output):
  - Wrangler config: `wrangler.jsonc`
  - Worker name: `edgerun-www`
  - Assets directory: `out/frontend/site`
  - Deploy command: `bun run deploy:site`

## Validation
Run before deploy:

```bash
bun run cloudflare:targets:check
```

This fails if worker names or assets directories overlap.
