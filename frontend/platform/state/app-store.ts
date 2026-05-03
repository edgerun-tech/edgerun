/**
 * Single source of truth for installed apps.
 * Uses generated protobuf types from gen/edgerun/v0/stream.ts.
 */

import { atom, computed } from "nanostores"
import { protocolClient } from "@/platform/protocol/client"
import { edgerun as edgerunStream } from "@/gen/edgerun/v0/stream"
import { edgerun as edgerunCap } from "@/gen/edgerun/v0/capability"

export interface AppStoreState {
  apps: Map<string, edgerunStream.v0.stream.AppPackage>
  principals: Map<string, edgerunStream.v0.stream.AppPrincipal>
  grants: Map<string, edgerunCap.v0.capability.CapabilityGrant[]>
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

export function getApp(appId: string): edgerunStream.v0.stream.AppPackage | undefined {
  return appStore.get().apps.get(appId)
}

export function listApps(): edgerunStream.v0.stream.AppPackage[] {
  return Array.from(appStore.get().apps.values())
}

export function getAppPrincipal(appId: string): edgerunStream.v0.stream.AppPrincipal | undefined {
  return appStore.get().principals.get(appId)
}

export function listGrantsForApp(appId: string): edgerunCap.v0.capability.CapabilityGrant[] {
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
      const items = JSON.parse(text) as Array<any>
      const apps = items.map((obj) => edgerunStream.v0.stream.AppPackage.fromObject(obj))
      const newApps = new Map<string, edgerunStream.v0.stream.AppPackage>()
      for (const app of apps) {
        newApps.set(app.name, app)
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
      const items = JSON.parse(text) as Array<any>
      const grants = items.map((obj) => edgerunCap.v0.capability.CapabilityGrant.fromObject(obj))
      const state = appStore.get()
      const newGrants = new Map(state.grants)
      newGrants.set(appId, grants)
      appStore.set({ ...state, grants: newGrants })
    }
  } catch {
    // Silently fail for grants
  }
}
