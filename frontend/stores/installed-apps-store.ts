"use client"

import { persistentAtom } from "@nanostores/persistent"
import {
  CORE_APP_IDS,
  DEFAULT_INSTALLED_APP_IDS,
  PUBLISHED_BUILTIN_APP_IDS,
  isCoreAppId,
  isRemovedAppId,
  normalizeAppId,
} from "@/platform/registries/app-id-policy"

export {
  CORE_APP_IDS,
  DEFAULT_INSTALLED_APP_IDS,
  PUBLISHED_BUILTIN_APP_IDS,
  normalizeAppId,
}

function normalizeInstalledIds(ids: unknown[]): string[] {
  return ids
    .map((id) => typeof id === "string" ? normalizeAppId(id) : "")
    .filter((id) => id && !isRemovedAppId(id))
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
  return isCoreAppId(appId)
}

export function isAppInstalled(appId: string): boolean {
  const normalized = normalizeAppId(appId)
  if (isRemovedAppId(normalized)) return false
  return installedAppIdsStore.get().map(normalizeAppId).includes(normalized) || isCoreApp(normalized)
}

export function installApp(appId: string): void {
  const normalized = normalizeAppId(appId)
  if (isRemovedAppId(normalized)) return
  const ids = normalizeInstalledIds(installedAppIdsStore.get())
  if (ids.includes(normalized)) return
  installedAppIdsStore.set([...ids, normalized])
}

export function uninstallApp(appId: string): void {
  const normalized = normalizeAppId(appId)
  if (isCoreApp(normalized)) return
  installedAppIdsStore.set(normalizeInstalledIds(installedAppIdsStore.get()).filter((id) => id !== normalized))
}
