# CloudOS Quick Reference

## Deployment

### Standard Deploy
```bash
npm run deploy
```

### Clean Deploy (Production)
```bash
npm run deploy:clean
```

### Manual Steps
```bash
# 1. Build workers
npm run build:workers

# 2. Build project
npm run build

# 3. Deploy
npx wrangler deploy
```

---

## Development

```bash
# Start dev server
npm run dev

# Run linter
npm run lint

# Run tests
npm run test

# Build workers only
npm run build:workers
```

---

## Environment Setup

### First Time Setup
```bash
# Install dependencies
npm install

# Set Cloudflare credentials
export CLOUDFLARE_API_TOKEN="your-token"

# Verify authentication
npx wrangler whoami
```

### Set Secrets (One Time)
```bash
npx wrangler secret put QWEN_CLIENT_ID
npx wrangler secret put QWEN_CLIENT_SECRET
```

---

## URLs

| Environment | URL |
|-------------|-----|
| Production | https://cloud-os.kensservices.workers.dev |
| Local Dev | http://localhost:4321 |

---

## Troubleshooting

```bash
# Check authentication
npx wrangler whoami

# View worker logs
npx wrangler tail

# List deployments
npx wrangler deployments list

# Rollback if needed
npx wrangler rollback
```

---

## Build Artifacts

After build, verify:
```bash
# Should show 6 worker files
ls -la public/workers/mcp/

# Should show dist directory
ls -la dist/
```

---

## Common Commands

| Command | Description |
|---------|-------------|
| `npm run deploy` | Deploy to Cloudflare |
| `npm run deploy:clean` | Clean build + deploy |
| `npm run dev` | Start local dev server |
| `npm run build` | Build for production |
| `npm run lint` | Run linter |
| `npm run test` | Run tests |
| `npx wrangler tail` | View live logs |
| `npx wrangler deploy` | Deploy directly |

---

**Full Documentation:** See [DEPLOY.md](./DEPLOY.md)
