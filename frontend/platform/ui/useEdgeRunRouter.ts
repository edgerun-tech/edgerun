/**
 * Hook for accessing EdgeRun router.
 */

import { useStore } from "@nanostores/react"
import {
  router,
  currentPath,
  currentRoute,
  navigate,
  registerAppRoutes,
  unregisterAppRoutes,
  goBack,
  navigateIfAllowed,
  buildAppRoute,
} from "@/platform/router/edgerun-router"
import type { RouteDefinition } from "@/platform/router/route-types"

export function useEdgeRunRouter() {
  const path = useStore(currentPath)
  const route = useStore(currentRoute)
  const routerState = useStore(router)

  return {
    currentPath: path,
    currentRoute: route,
    history: routerState.history,
    registeredAppRoutes: routerState.registeredAppRoutes,
    navigate,
    registerAppRoutes: (appId: string, routes: RouteDefinition[]) =>
      registerAppRoutes(appId, routes),
    unregisterAppRoutes,
    goBack,
    navigateIfAllowed,
    buildAppRoute,
  }
}
