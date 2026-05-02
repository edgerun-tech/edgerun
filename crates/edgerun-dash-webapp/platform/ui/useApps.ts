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
  listAppRoutes,
  listAppActions,
  listAppPipelines,
  getAppPackageHash,
  loadApps,
  installApp,
  uninstallApp,
} from "@/platform/state/app-store"
import { appRegistry } from "@/platform/registries/app-registry"

export function useApps() {
  const store = useStore(appStore)
  const apps = useStore(installedApps)
  const count = useStore(appCount)

  return {
    apps,
    count,
    isLoading: store.isLoading,
    error: store.error,
    getApp,
    listApps,
    listAppRoutes,
    listAppActions,
    listAppPipelines,
    getAppPackageHash,
    refresh: loadApps,
    install: installApp,
    uninstall: uninstallApp,
    registry: {
      getApp: appRegistry.getApp,
      listApps: appRegistry.listApps,
      registerApp: appRegistry.registerApp,
    },
  }
}
