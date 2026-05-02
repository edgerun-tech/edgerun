/**
 * Single source of truth for installed apps.
 * Tracks AppPackage, AppPrincipal, app routes, actions, pipelines, runtime status.
 */

import { atom, computed } from "nanostores"
import { protocolClient } from "@/platform/protocol/client"
import type {
  AppPackage,
  AppRoute,
  AppAction,
  AppPipeline,
  AppPrincipal,
} from "@/platform/protocol/apps"
import type { CapabilityGrant } from "@/platform/protocol/capabilities"

export interface AppStoreState {
  apps: Map<string, AppPackage>
  principals: Map<string, AppPrincipal>
  grants: Map<string, CapabilityGrant[]>
  isLoading: boolean
  error: string | null
  lastRefresh: string | null
}

const initialState: AppStoreState = {
  apps: new Map(),
  principals: new Map(),
  grants: new Map(),
  isLoading: false,
  error: null,
  lastRefresh: null,
}

export const appStore = atom<AppStoreState>(initialState)

export const installedApps = computed(appStore, (s) =>
  Array.from(s.apps.values()),
)

export const appCount = computed(appStore, (s) => s.apps.size)

export function getApp(appId: string): AppPackage | undefined {
  return appStore.get().apps.get(appId)
}

export function listApps(): AppPackage[] {
  return Array.from(appStore.get().apps.values())
}

export function listAppRoutes(appId: string): AppRoute[] {
  const app = getApp(appId)
  return app?.routes ?? []
}

export function listAppActions(appId: string): AppAction[] {
  const app = getApp(appId)
  return app?.actions ?? []
}

export function listAppPipelines(appId: string): AppPipeline[] {
  const app = getApp(appId)
  return app?.pipelines ?? []
}

export function getAppPackageHash(appId: string): string | undefined {
  const app = getApp(appId)
  return app?.packageHash
}

export function getAppPrincipal(appId: string): AppPrincipal | undefined {
  return appStore.get().principals.get(appId)
}

export function listGrantsForApp(appId: string): CapabilityGrant[] {
  return appStore.get().grants.get(appId) ?? []
}

export async function loadApps(): Promise<void> {
  const state = appStore.get()
  appStore.set({ ...state, isLoading: true, error: null })

  try {
    const response = await protocolClient.send({
      method: "GET",
      path: "/protocol/apps",
    })

    if (response.status === 200) {
      const text = new TextDecoder().decode(response.body)
      const apps = JSON.parse(text) as AppPackage[]
      const newApps = new Map<string, AppPackage>()
      for (const app of apps) {
        newApps.set(app.appId, app)
      }
      appStore.set({
        ...appStore.get(),
        apps: newApps,
        isLoading: false,
        lastRefresh: new Date().toISOString(),
      })
    } else {
      throw new Error(`Failed to load apps: ${response.status}`)
    }
  } catch (err) {
    appStore.set({
      ...appStore.get(),
      isLoading: false,
      error: err instanceof Error ? err.message : "Failed to load apps",
    })
  }
}

export async function loadAppGrants(appId: string): Promise<void> {
  try {
    const response = await protocolClient.send({
      method: "GET",
      path: `/protocol/app/${appId}/grants`,
    })

    if (response.status === 200) {
      const text = new TextDecoder().decode(response.body)
      const grants = JSON.parse(text) as CapabilityGrant[]
      const state = appStore.get()
      const newGrants = new Map(state.grants)
      newGrants.set(appId, grants)
      appStore.set({ ...state, grants: newGrants })
    }
  } catch {
    // Silently fail for grants
  }
}

export async function installApp(appId: string): Promise<boolean> {
  try {
    const response = await protocolClient.send({
      method: "POST",
      path: "/protocol/app/install",
      body: new TextEncoder().encode(JSON.stringify({ appId })),
    })
    if (response.status === 200) {
      await loadApps()
      return true
    }
    return false
  } catch {
    return false
  }
}

export async function uninstallApp(appId: string): Promise<boolean> {
  try {
    const response = await protocolClient.send({
      method: "POST",
      path: "/protocol/app/uninstall",
      body: new TextEncoder().encode(JSON.stringify({ appId })),
    })
    if (response.status === 200) {
      const state = appStore.get()
      const newApps = new Map(state.apps)
      newApps.delete(appId)
      const newGrants = new Map(state.grants)
      newGrants.delete(appId)
      appStore.set({ ...state, apps: newApps, grants: newGrants })
      return true
    }
    return false
  } catch {
    return false
  }
}
