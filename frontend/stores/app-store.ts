/**
 * Single source of truth for installed apps.
 * Uses generated protobuf types from gen/edgerun/v0/stream.ts.
 */

import { atom, computed } from "nanostores"
import { protocolClient } from "@/platform/protocol/client"
import { edgerun as edgerunStream } from "@/gen/edgerun/v0/stream"
import { bytesToHex } from "@/platform/utils/bytes"

export interface AppStoreState {
  apps: Map<string, edgerunStream.v0.stream.AppPackage>
  principals: Map<string, edgerunStream.v0.stream.AppPrincipal>
  isLoading: boolean
  error: string | null
  lastRefresh: string | null
}

const initialState: AppStoreState = {
  apps: new Map(),
  principals: new Map(),
  isLoading: false,
  error: null,
  lastRefresh: null,
}

export const appStore = atom<AppStoreState>(initialState)

export const installedApps = computed(appStore, (s) =>
  Array.from(s.apps.values()),
)

export const appCount = computed(appStore, (s) => s.apps.size)

export function appPackageId(app: edgerunStream.v0.stream.AppPackage): string {
  return bytesToHex(app.wasm_object?.object_id) || app.name || "unknown-app"
}

function normalizeAppListPayload(payload: unknown): unknown[] {
  if (Array.isArray(payload)) return payload
  if (payload && typeof payload === "object" && Array.isArray((payload as { apps?: unknown[] }).apps)) {
    return (payload as { apps: unknown[] }).apps
  }
  return []
}

export function getApp(appId: string): edgerunStream.v0.stream.AppPackage | undefined {
  return appStore.get().apps.get(appId)
}

export function listApps(): edgerunStream.v0.stream.AppPackage[] {
  return Array.from(appStore.get().apps.values())
}

export function upsertApp(app: edgerunStream.v0.stream.AppPackage): string {
  const state = appStore.get()
  const appId = appPackageId(app)
  const apps = new Map(state.apps)
  apps.set(appId, app)
  appStore.set({ ...state, apps, lastRefresh: new Date().toISOString() })
  return appId
}

export function removeApp(appId: string): void {
  const state = appStore.get()
  const apps = new Map(state.apps)
  const principals = new Map(state.principals)
  apps.delete(appId)
  principals.delete(appId)
  appStore.set({ ...state, apps, principals, lastRefresh: new Date().toISOString() })
}

export function getAppPrincipal(appId: string): edgerunStream.v0.stream.AppPrincipal | undefined {
  return appStore.get().principals.get(appId)
}

export async function loadApps(): Promise<void> {
  const state = appStore.get()
  appStore.set({ ...state, isLoading: true, error: null })

  try {
    const response = await protocolClient.send({
      method: "GET",
      path: "/protocol/apps",
      headers: { Accept: "application/json" },
    })

    if (response.status !== 200) {
      throw new Error(`Failed to load apps: ${response.status}`)
    }

    const text = new TextDecoder().decode(response.body)
    const parsed = JSON.parse(text)
    const apps = normalizeAppListPayload(parsed).map((obj) => edgerunStream.v0.stream.AppPackage.fromObject(obj as any))
    const newApps = new Map<string, edgerunStream.v0.stream.AppPackage>()
    for (const app of apps) {
      newApps.set(appPackageId(app), app)
    }
    appStore.set({
      ...appStore.get(),
      apps: newApps,
      isLoading: false,
      lastRefresh: new Date().toISOString(),
    })
  } catch (err) {
    appStore.set({
      ...appStore.get(),
      isLoading: false,
      error: err instanceof Error ? err.message : "Failed to load apps",
    })
  }
}


