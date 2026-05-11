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

function normalizeCachedIds(ids: unknown[]): string[] {
  return ids
    .map((id) => typeof id === "string" ? normalizeAppId(id) : "")
    .filter((id) => id && !isRemovedAppId(id))
}

export const cachedAppIdsStore = persistentAtom<string[]>(
  "edgerun:installedAppIds",
  [...DEFAULT_INSTALLED_APP_IDS],
  {
    encode: JSON.stringify,
    decode: (value) => {
      if (!value) return [...DEFAULT_INSTALLED_APP_IDS]
      const parsed = JSON.parse(value)
      if (!Array.isArray(parsed)) return [...DEFAULT_INSTALLED_APP_IDS]
      const normalized = normalizeCachedIds(parsed)
      return Array.from(new Set([...CORE_APP_IDS, ...normalized]))
    },
  },
)

export function isCoreApp(appId: string): boolean {
  return isCoreAppId(appId)
}

export function isAppCached(appId: string): boolean {
  const normalized = normalizeAppId(appId)
  if (isRemovedAppId(normalized)) return false
  return cachedAppIdsStore.get().map(normalizeAppId).includes(normalized) || isCoreApp(normalized)
}

export function cacheApp(appId: string): void {
  const normalized = normalizeAppId(appId)
  if (isRemovedAppId(normalized)) return
  const ids = normalizeCachedIds(cachedAppIdsStore.get())
  if (ids.includes(normalized)) return
  cachedAppIdsStore.set([...ids, normalized])
}

export function removeCachedApp(appId: string): void {
  const normalized = normalizeAppId(appId)
  if (isCoreApp(normalized)) return
  cachedAppIdsStore.set(normalizeCachedIds(cachedAppIdsStore.get()).filter((id) => id !== normalized))
}

/** @deprecated Use normalizeCachedIds internally. */
function normalizeInstalledIds(ids: unknown[]): string[] {
  return normalizeCachedIds(ids)
}

/** @deprecated Use cachedAppIdsStore. Kept for compatibility with older call sites. */
export const installedAppIdsStore = cachedAppIdsStore

/** @deprecated Use isAppCached. */
export function isAppInstalled(appId: string): boolean {
  return isAppCached(appId)
}

/** @deprecated Use cacheApp. */
export function installApp(appId: string): void {
  cacheApp(appId)
}

/** @deprecated Use removeCachedApp. */
export function uninstallApp(appId: string): void {
  removeCachedApp(appId)
}
