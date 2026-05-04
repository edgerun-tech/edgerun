"use client"

import { persistentAtom } from "@nanostores/persistent"

export const CORE_APP_IDS = ["app-store", "settings"] as const

export const DEFAULT_INSTALLED_APP_IDS = [
  "app-store",
  "settings",
  "terminal",
  "people",
  "trust-manager",
  "code-runner",
  "file-browser",
  "resource-monitor",
] as const

export const installedAppIdsStore = persistentAtom<string[]>(
  "edgerun:installedAppIds",
  [...DEFAULT_INSTALLED_APP_IDS],
  {
    encode: JSON.stringify,
    decode: (value) => {
      const parsed = JSON.parse(value)
      if (!Array.isArray(parsed)) return [...DEFAULT_INSTALLED_APP_IDS]
      return Array.from(new Set([...DEFAULT_INSTALLED_APP_IDS.filter((id) => id === "app-store" || id === "settings"), ...parsed]))
    },
  },
)

export function isCoreApp(appId: string): boolean {
  return (CORE_APP_IDS as readonly string[]).includes(appId)
}

export function isAppInstalled(appId: string): boolean {
  return installedAppIdsStore.get().includes(appId) || isCoreApp(appId)
}

export function installApp(appId: string): void {
  const ids = installedAppIdsStore.get()
  if (ids.includes(appId)) return
  installedAppIdsStore.set([...ids, appId])
}

export function uninstallApp(appId: string): void {
  if (isCoreApp(appId)) return
  installedAppIdsStore.set(installedAppIdsStore.get().filter((id) => id !== appId))
}
