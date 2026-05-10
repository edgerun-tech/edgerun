/**
 * Single registry for apps.
 * Tracks AppPackage from generated protobuf types.
 */

import { atom, computed } from "nanostores"
import { edgerun as edgerunStream } from "@/gen/edgerun/v0/stream"
import { appStore } from "@/stores/app-store"
import { bytesToHex } from "@/platform/utils/bytes"

export interface AppRegistryState {
  apps: Map<string, edgerunStream.v0.stream.AppPackage>
}

const initialState: AppRegistryState = {
  apps: new Map(),
}

export const appRegistry = atom<AppRegistryState>(initialState)

export const allApps = computed(appRegistry, (s) =>
  Array.from(s.apps.values()),
)

export function registerApp(app: edgerunStream.v0.stream.AppPackage): void {
  const state = appRegistry.get()
  const newApps = new Map(state.apps)
  const appId = bytesToHex(app.wasm_object?.object_id)
  newApps.set(appId, app)
  appRegistry.set({ apps: newApps })
}

export function getApp(appId: string): edgerunStream.v0.stream.AppPackage | undefined {
  return appRegistry.get().apps.get(appId)
}

export function listApps(): edgerunStream.v0.stream.AppPackage[] {
  return Array.from(appRegistry.get().apps.values())
}

// Sync from app store
appStore.listen((state) => {
  const registryState = appRegistry.get()
  const newApps = new Map(registryState.apps)

  for (const [id, app] of state.apps) {
    newApps.set(id, app)
  }

  appRegistry.set({ apps: newApps })
})
