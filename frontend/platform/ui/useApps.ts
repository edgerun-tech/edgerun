/**
 * Hook for accessing app state and actions.
 */

import { useStore } from "@nanostores/react"
import {
  appStore,
  installedApps,
  appCount,
  getApp,
  listApps,
  loadApps,
} from "@/platform/state/app-store"
import {
  appRegistry,
  getApp as getAppFromRegistry,
  listApps as listAppsFromRegistry,
  registerApp,
} from "@/platform/registries/app-registry"

export function useApps() {
  const store = useStore(appStore)
  const apps = useStore(installedApps)
  const count = useStore(appCount)
  const registry = useStore(appRegistry)

  return {
    apps,
    count,
    isLoading: store.isLoading,
    error: store.error,
    getApp,
    listApps,
    refresh: loadApps,
    registry: {
      getApp: getAppFromRegistry,
      listApps: listAppsFromRegistry,
      registerApp,
      allApps: registry.apps,
    },
  }
}
