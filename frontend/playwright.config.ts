import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './tests/e2e',
  timeout: 30000,
  use: {
    baseURL: 'http://127.0.0.1:4173',
    trace: 'on-first-retry'
  },
  webServer: {
    command: 'bun run build && bunx --bun serve dist -l 4173',
    port: 4173,
    reuseExistingServer: true,
    timeout: 180000
  }
})
