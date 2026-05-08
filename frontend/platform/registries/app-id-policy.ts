export const APP_ID_ALIASES: Record<string, string> = {
  wallet: "finances",
  contacts: "people",
  calling: "people",
  chat: "people",
}

export const REMOVED_APP_IDS = new Set([
  "resource-monitor",
  "network-monitor",
  "terminal",
  "people",
  "trust-manager",
  "finances",
  "wallet",
  "contacts",
  "calling",
  "chat",
  "code-runner",
  "storage",
  "db-explorer",
  "git-sync",
  "web-server",
  "compute-node",
  "workflow-builder",
  "calculator",
  "help",
])

export const CORE_APP_IDS = ["app-store", "settings"] as const

export const PUBLISHED_BUILTIN_APP_IDS = [
  "app-store",
  "settings",
  "file-browser",
  "gmail",
  "google-drive",
  "google-photos",
  "google-contacts",
  "github",
  "cloudflare",
] as const

export const DEFAULT_INSTALLED_APP_IDS = [
  "app-store",
  "settings",
] as const

export function normalizeAppId(appId: string): string {
  return APP_ID_ALIASES[appId] || appId
}

export function isRemovedAppId(appId: string): boolean {
  return REMOVED_APP_IDS.has(normalizeAppId(appId))
}

export function isCoreAppId(appId: string): boolean {
  return (CORE_APP_IDS as readonly string[]).includes(normalizeAppId(appId))
}
