/**
 * Single registry for apps.
 * Tracks AppPackage, routes, actions, pipelines.
 */

import { atom, computed } from "nanostores"
import type {
  AppPackage,
  AppRoute,
  AppAction,
  AppPipeline,
} from "@/platform/protocol/apps"
import { appStore } from "@/platform/state/app-store"

export interface AppRegistryState {
  apps: Map<string, AppPackage>
  routes: Map<string, AppRoute[]>
  actions: Map<string, AppAction[]>
  pipelines: Map<string, AppPipeline[]>
}

const initialState: AppRegistryState = {
  apps: new Map(),
  routes: new Map(),
  actions: new Map(),
  pipelines: new Map(),
}

export const appRegistry = atom<AppRegistryState>(initialState)

export const allApps = computed(appRegistry, (s) =>
  Array.from(s.apps.values()),
)

export function registerApp(app: AppPackage): void {
  const state = appRegistry.get()
  const newApps = new Map(state.apps)
  newApps.set(app.appId, app)

  const newRoutes = new Map(state.routes)
  newRoutes.set(app.appId, app.routes)

  const newActions = new Map(state.actions)
  newActions.set(app.appId, app.actions)

  const newPipelines = new Map(state.pipelines)
  newPipelines.set(app.appId, app.pipelines)

  appRegistry.set({
    apps: newApps,
    routes: newRoutes,
    actions: newActions,
    pipelines: newPipelines,
  })
}

export function getApp(appId: string): AppPackage | undefined {
  return appRegistry.get().apps.get(appId)
}

export function listApps(): AppPackage[] {
  return Array.from(appRegistry.get().apps.values())
}

export function listAppRoutes(appId: string): AppRoute[] {
  return appRegistry.get().routes.get(appId) || []
}

export function listAppActions(appId: string): AppAction[] {
  return appRegistry.get().actions.get(appId) || []
}

export function listAppPipelines(appId: string): AppPipeline[] {
  return appRegistry.get().pipelines.get(appId) || []
}

export function getAppPackageHash(appId: string): string | undefined {
  return appRegistry.get().apps.get(appId)?.packageHash
}

export function getAppByWasmRef(wasmObjectId: string): AppPackage | undefined {
  return Array.from(appRegistry.get().apps.values()).find(
    (app) => app.wasmObjectRef.objectId === wasmObjectId,
  )
}

// Sync from app store
appStore.listen((state) => {
  const registryState = appRegistry.get()
  const newApps = new Map(registryState.apps)
  const newRoutes = new Map(registryState.routes)
  const newActions = new Map(registryState.actions)
  const newPipelines = new Map(registryState.pipelines)

  for (const [id, app] of state.apps) {
    newApps.set(id, app)
    newRoutes.set(id, app.routes)
    newActions.set(id, app.actions)
    newPipelines.set(id, app.pipelines)
  }

  appRegistry.set({
    apps: newApps,
    routes: newRoutes,
    actions: newActions,
    pipelines: newPipelines,
  })
})
