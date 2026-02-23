# CloudOS Deployment Guide

Reproducible deployment process for CloudOS to Cloudflare Workers.

---

## Quick Start

### One-Line Deploy
```bash
./scripts/deploy.sh
```

### Clean Deploy (recommended for production)
```bash
./scripts/deploy.sh --clean
```

---

## Prerequisites

### Required Software
- **Node.js** v18 or higher
- **npm** (comes with Node.js)
- **Bun** (optional, for some build steps)

### Cloudflare Authentication

Set one of these environment variables:

```bash
# Option 1: API Token (recommended)
export CLOUDFLARE_API_TOKEN="your-api-token"

# Option 2: Global API Key + Email
export CLOUDFLARE_API_KEY="your-global-api-key"
export CLOUDFLARE_EMAIL="your-email@example.com"
```

### API Token Permissions

Your Cloudflare API token needs these permissions:
- `Workers Scripts:Edit`
- `Workers KV Storage:Edit`
- `Cloudflare Pages:Edit` (if using Pages)

Create token at: https://dash.cloudflare.com/profile/api-tokens

---

## Deployment Steps

### Step 1: Verify Environment

```bash
# Check Node.js version
node -v  # Should be v18+

# Check Cloudflare authentication
npx wrangler whoami

# Should show your account info
```

### Step 2: Clean Build (Optional but Recommended)

```bash
# Remove all build artifacts
rm -rf dist
rm -rf public/workers
rm -rf node_modules/.vite
```

### Step 3: Build MCP Workers

```bash
# Build all MCP server workers
node scripts/build-workers.mjs
```

Expected output:
```
Root dir: /home/ken/src/browser-os
Out dir: /home/ken/src/browser-os/public/workers/mcp
Building .../base.ts -> .../base.js
Built base.js
Building .../browser-os.ts -> .../browser-os.js
Built browser-os.js
Building .../github.ts -> .../github.js
Built github.js
Building .../cloudflare.ts -> .../cloudflare.js
Built cloudflare.js
Building .../terminal.ts -> .../terminal.js
Built terminal.js
Building .../qwen.ts -> .../qwen.js
Built qwen.js
```

### Step 4: Build Project

```bash
npm run build
```

Expected output:
```
[vite] dist/_astro/...
[vite] ✓ built in X.XXs
[build] Complete!
```

### Step 5: Deploy to Cloudflare Workers

```bash
npx wrangler deploy
```

Expected output:
```
✨ Success! Uploaded X files
Deployed cloud-os triggers
https://cloud-os.kensservices.workers.dev
```

---

## Automated Deployment

### Using the Deploy Script

The `scripts/deploy.sh` script automates all steps:

```bash
# Standard deploy
./scripts/deploy.sh

# Clean deploy (removes old build artifacts)
./scripts/deploy.sh --clean

# Deploy without rebuilding (uses existing dist/)
./scripts/deploy.sh --skip-build

# Show help
./scripts/deploy.sh --help
```

### CI/CD Integration

Example GitHub Actions workflow:

```yaml
# .github/workflows/deploy.yml
name: Deploy to Cloudflare Workers

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '18'
      
      - name: Install dependencies
        run: npm ci
      
      - name: Build workers
        run: node scripts/build-workers.mjs
      
      - name: Build project
        run: npm run build
      
      - name: Deploy to Cloudflare
        run: npx wrangler deploy
        env:
          CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
```

---

## Environment Variables

### Required for OAuth

Set these in Cloudflare Workers dashboard:

1. Go to: https://dash.cloudflare.com/?to=/:account/workers-and-pages/services/view/cloud-os/settings
2. Navigate to "Environment Variables"
3. Add variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `QWEN_CLIENT_ID` | Qwen OAuth client ID | `qwen-code-client` |
| `QWEN_CLIENT_SECRET` | Qwen OAuth client secret | `your-secret` |

### Setting via Wrangler

```bash
# Set production environment variable
npx wrangler secret put QWEN_CLIENT_ID
npx wrangler secret put QWEN_CLIENT_SECRET
```

---

## Verification

### Post-Deployment Checklist

- [ ] Build completed without errors
- [ ] All 6 MCP workers built successfully
  - `public/workers/mcp/base.js`
  - `public/workers/mcp/browser-os.js`
  - `public/workers/mcp/github.js`
  - `public/workers/mcp/cloudflare.js`
  - `public/workers/mcp/terminal.js`
  - `public/workers/mcp/qwen.js`
- [ ] Deployment URL accessible: https://cloud-os.kensservices.workers.dev
- [ ] OAuth flow works (test with Qwen Code)
- [ ] MCP servers connect properly
- [ ] Console shows no errors

### Test Commands

```bash
# Check deployment is live
curl -I https://cloud-os.kensservices.workers.dev

# Should return HTTP 200

# Check workers are built
ls -la public/workers/mcp/

# Should show 6 .js files
```

---

## Troubleshooting

### Build Errors

**Problem:** `Cannot find module`

```bash
# Clean and reinstall
rm -rf dist node_modules package-lock.json
npm install
npm run build
```

**Problem:** `Worker compilation failed`

```bash
# Rebuild workers only
node scripts/build-workers.mjs

# Then deploy
npx wrangler deploy
```

### Deployment Errors

**Problem:** `Authentication error`

```bash
# Check authentication
npx wrangler whoami

# Re-login if needed
npx wrangler login
```

**Problem:** `Project not found`

```bash
# Create project first
npx wrangler pages project create cloud-os

# Or use workers deploy
npx wrangler deploy
```

**Problem:** `KV namespace not found`

```bash
# Create KV namespace
npx wrangler kv:namespace create SESSION

# Update wrangler.jsonc with new ID
```

### Runtime Errors

**Problem:** OAuth not working

1. Check environment variables are set:
   ```bash
   npx wrangler secret list
   ```

2. Verify OAuth endpoints in code match Qwen's current API

3. Check worker logs:
   ```bash
   npx wrangler tail
   ```

---

## Rollback

### Deploy Previous Version

```bash
# List deployments
npx wrangler deployments list

# Rollback to specific version
npx wrangler rollback <deployment-id>
```

### Quick Rollback

```bash
# Rollback to last known good version
npx wrangler rollback
```

---

## Production URL

**https://cloud-os.kensservices.workers.dev**

All deployments update this URL automatically.

---

## Deployment History

View deployment history:

```bash
npx wrangler deployments list
```

Example output:
```
┌──────────────────────────────────────┬──────────────┬─────────────┐
│ Deployment ID                        │ Created      │ Author      │
├──────────────────────────────────────┼──────────────┼─────────────┤
│ 3eaf6385-cb48-4b6c-be01-00f1a7993f1e │ 2 mins ago   │ user@example│
└──────────────────────────────────────┴──────────────┴─────────────┘
```

---

## Support

For issues:
1. Check worker logs: `npx wrangler tail`
2. Review build output for errors
3. Verify Cloudflare authentication
4. Check environment variables are set

---

**Last Updated:** 2026-02-17
**Version:** 1.0.0
