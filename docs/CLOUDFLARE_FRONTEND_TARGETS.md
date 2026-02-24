# Cloudflare Frontend Target Map

- Site frontend (`frontend/` static output):
  - Wrangler config: `wrangler.jsonc`
  - Worker name: `edgerun-www`
  - Assets directory: `out/frontend/site`
  - KV binding: `EMAIL_SIGNUPS` (required for `/api/lead` email capture)
  - Deploy command: `bun run deploy:site`

- Cloud OS frontend (`cloud-os/` Astro output):
  - Wrangler config: `cloud-os/wrangler.jsonc`
  - Worker name: `cloud-os`
  - Assets directory: `cloud-os/dist`
  - Deploy command: `bun run deploy:cloud-os`

## Validation
Run before deploy:

```bash
bun run cloudflare:targets:check
```

This fails if worker names or assets directories overlap.
