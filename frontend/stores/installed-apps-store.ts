"use client"

import { persistentAtom } from "@nanostores/persistent"

const APP_ID_ALIASES: Record<string, string> = {
  wallet: "finances",
}

const REMOVED_APP_IDS = new Set(["resource-monitor"])

export const CORE_APP_IDS = ["app-store", "settings"] as const

export const DEFAULT_INSTALLED_APP_IDS = [
  "app-store",
  "settings",
  "terminal",
  "people",
  "trust-manager",
  "finances",
  "code-runner",
  "file-browser",
] as const

export function normalizeAppId(appId: string): string {
  return APP_ID_ALIASES[appId] || appId
}

function normalizeInstalledIds(ids: unknown[]): string[] {
  return ids
    .map((id) => typeof id === "string" ? normalizeAppId(id) : "")
    .filter((id) => id && !REMOVED_APP_IDS.has(id))
}

export const installedAppIdsStore = persistentAtom<string[]>(
  "edgerun:installedAppIds",
  [...DEFAULT_INSTALLED_APP_IDS],
  {
    encode: JSON.stringify,
    decode: (value) => {
      const parsed = JSON.parse(value)
      if (!Array.isArray(parsed)) return [...DEFAULT_INSTALLED_APP_IDS]
      const normalized = normalizeInstalledIds(parsed)
      return Array.from(new Set([...CORE_APP_IDS, ...normalized]))
    },
  },
)

export function isCoreApp(appId: string): boolean {
  return (CORE_APP_IDS as readonly string[]).includes(normalizeAppId(appId))
}

export function isAppInstalled(appId: string): boolean {
  const normalized = normalizeAppId(appId)
  if (REMOVED_APP_IDS.has(normalized)) return false
  return installedAppIdsStore.get().map(normalizeAppId).includes(normalized) || isCoreApp(normalized)
}

export function installApp(appId: string): void {
  const normalized = normalizeAppId(appId)
  if (REMOVED_APP_IDS.has(normalized)) return
  const ids = normalizeInstalledIds(installedAppIdsStore.get())
  if (ids.includes(normalized)) return
  installedAppIdsStore.set([...ids, normalized])
}

export function uninstallApp(appId: string): void {
  const normalized = normalizeAppId(appId)
  if (isCoreApp(normalized)) return
  installedAppIdsStore.set(normalizeInstalledIds(installedAppIdsStore.get()).filter((id) => id !== normalized))
}
