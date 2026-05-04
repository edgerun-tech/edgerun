"use client"

import { persistentAtom } from "@nanostores/persistent"

const APP_ID_ALIASES: Record<string, string> = {
  wallet: "finances",
}

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
  "resource-monitor",
] as const

export function normalizeAppId(appId: string): string {
  return APP_ID_ALIASES[appId] || appId
}

export const installedAppIdsStore = persistentAtom<string[]>(
  "edgerun:installedAppIds",
  [...DEFAULT_INSTALLED_APP_IDS],
  {
    encode: JSON.stringify,
    decode: (value) => {
      const parsed = JSON.parse(value)
      if (!Array.isArray(parsed)) return [...DEFAULT_INSTALLED_APP_IDS]
      const normalized = parsed.map((id) => typeof id === "string" ? normalizeAppId(id) : "").filter(Boolean)
      return Array.from(new Set([...DEFAULT_INSTALLED_APP_IDS.filter((id) => id === "app-store" || id === "settings"), ...normalized]))
    },
  },
)

export function isCoreApp(appId: string): boolean {
  return (CORE_APP_IDS as readonly string[]).includes(normalizeAppId(appId))
}

export function isAppInstalled(appId: string): boolean {
  const normalized = normalizeAppId(appId)
  return installedAppIdsStore.get().map(normalizeAppId).includes(normalized) || isCoreApp(normalized)
}

export function installApp(appId: string): void {
  const normalized = normalizeAppId(appId)
  const ids = installedAppIdsStore.get().map(normalizeAppId)
  if (ids.includes(normalized)) return
  installedAppIdsStore.set([...ids, normalized])
}

export function uninstallApp(appId: string): void {
  const normalized = normalizeAppId(appId)
  if (isCoreApp(normalized)) return
  installedAppIdsStore.set(installedAppIdsStore.get().map(normalizeAppId).filter((id) => id !== normalized))
}
