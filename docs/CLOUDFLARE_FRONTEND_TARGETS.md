# Cloudflare Frontend Target Map

- Site frontend (`frontend/` static output):
  - Wrangler config: `wrangler.jsonc`
  - Worker name: `edgerun-www`
  - Assets directory: `out/frontend/site`
  - KV binding: `EMAIL_SIGNUPS` (required for `/api/lead` email capture)
  - Deploy command: `bun run deploy:site`

- OS frontend (shared frontend assets + OS worker gate):
  - Wrangler config: `wrangler-os.jsonc`
  - Worker name: `edgerun-os`
  - Assets directory: `out/frontend/os`
  - Deploy command: `bun run deploy:os`

## Validation
Run before deploy:

```bash
bun run cloudflare:targets:check
```

This fails if worker names or assets directories overlap.
