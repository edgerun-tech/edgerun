# 🚀 CloudOS Deployment Checklist

Reproducible deployment process for CloudOS.

---

## ✅ Pre-Deployment Checklist

### Environment Setup
- [ ] Node.js v18+ installed (`node -v`)
- [ ] Cloudflare API token set (`CLOUDFLARE_API_TOKEN`)
- [ ] Wrangler authenticated (`npx wrangler whoami`)
- [ ] In project root directory (`pwd` shows `/home/ken/src/browser-os`)

### Secrets Configured
- [ ] `QWEN_CLIENT_ID` secret set
- [ ] `QWEN_CLIENT_SECRET` secret set
- [ ] KV namespace `SESSION` configured

---

## 📦 Deployment Process

### Option 1: Automated (Recommended)

```bash
# Clean deploy (production)
npm run deploy:clean

# Or standard deploy
npm run deploy
```

### Option 2: Manual Steps

```bash
# 1. Clean build artifacts
rm -rf dist public/workers node_modules/.vite

# 2. Build MCP workers
npm run build:workers

# 3. Build project
npm run build

# 4. Verify build
ls -la dist/
ls -la public/workers/mcp/

# 5. Deploy
npx wrangler deploy
```

---

## ✅ Post-Deployment Verification

### Immediate Checks
- [ ] Deployment succeeded (exit code 0)
- [ ] URL displayed: https://cloud-os.kensservices.workers.dev
- [ ] No errors in deployment output

### Browser Testing
- [ ] Site loads without errors
- [ ] Console shows no errors
- [ ] MCP servers connect (check console)
- [ ] OAuth flow works (test Qwen Code)

### Worker Logs
```bash
# Monitor for errors
npx wrangler tail
```

---

## 🔧 Troubleshooting

### Build Fails
```bash
# Clean everything
rm -rf dist public/workers node_modules/.vite package-lock.json

# Reinstall
npm install

# Rebuild
npm run build:workers && npm run build
```

### Deploy Fails
```bash
# Check authentication
npx wrangler whoami

# Re-login if needed
npx wrangler login

# Try deploy again
npx wrangler deploy
```

### Runtime Errors
```bash
# Check secrets
npx wrangler secret list

# View logs
npx wrangler tail

# Rollback if needed
npx wrangler rollback
```

---

## 📊 Deployment Info

| Item | Value |
|------|-------|
| **Project** | cloud-os |
| **Target** | Cloudflare Workers |
| **URL** | https://cloud-os.kensservices.workers.dev |
| **Build Script** | `./scripts/deploy.sh` |
| **NPM Command** | `npm run deploy:clean` |

---

## 📝 Deployment Log

```
Date: _______________
Version: ___________
Deployed by: _______
Build number: ______
Notes:
_________________________________
_________________________________
```

---

## 🆘 Quick Help

- **Full docs:** [DEPLOY.md](./DEPLOY.md)
- **Quick ref:** [QUICKREF.md](./QUICKREF.md)
- **Script help:** `./scripts/deploy.sh --help`

---

**Last Updated:** 2026-02-17
**Status:** ✅ Production Ready
