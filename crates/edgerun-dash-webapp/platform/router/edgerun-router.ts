/**
 * Single app-aware router for EdgeRun dashboard.
 * Handles dashboard routes, installed app routes, route guards.
 */

import { atom, computed } from "nanostores"
import type { RouteDefinition, ResolvedRoute } from "./route-types"
import { DASHBOARD_ROUTES } from "./route-types"
import { resolveRoute, buildRoutePath } from "./route-resolver"
import { createCapabilityGuard, combineGuards } from "./route-guards"

export interface RouterState {
  currentPath: string
  currentRoute: ResolvedRoute | null
  history: string[]
  registeredAppRoutes: RouteDefinition[]
}

const initialState: RouterState = {
  currentPath: "/dashboard",
  currentRoute: null,
  history: ["/dashboard"],
  registeredAppRoutes: [],
}

export const router = atom<RouterState>(initialState)

export const currentPath = computed(router, (s) => s.currentPath)

export const currentRoute = computed(router, (s) => s.currentRoute)

export function navigate(path: string): void {
  const resolved = resolveRoute(path, [
    ...DASHBOARD_ROUTES,
    ...router.get().registeredAppRoutes,
  ])

  if (resolved) {
    const state = router.get()
    const newHistory = [...state.history, path].slice(-50)
    router.set({
      currentPath: path,
      currentRoute: resolved,
      history: newHistory,
      registeredAppRoutes: state.registeredAppRoutes,
    })
  }
}

export function registerAppRoutes(appId: string, routes: RouteDefinition[]): void {
  const state = router.get()
  const appRoutes = routes.map((r) => ({
    ...r,
    appId,
    path: `/apps/${appId}${r.path === "/" ? "" : r.path}`,
  }))
  router.set({
    ...state,
    registeredAppRoutes: [...state.registeredAppRoutes, ...appRoutes],
  })
}

export function unregisterAppRoutes(appId: string): void {
  const state = router.get()
  router.set({
    ...state,
    registeredAppRoutes: state.registeredAppRoutes.filter(
      (r) => r.appId !== appId,
    ),
  })
}

export function goBack(): void {
  const state = router.get()
  if (state.history.length > 1) {
    const newHistory = state.history.slice(0, -1)
    const previousPath = newHistory[newHistory.length - 1]
    navigate(previousPath)
  }
}

export function checkRouteGuard(route: ResolvedRoute): boolean {
  const guard = route.definition.guard
  if (!guard) return true

  const result = guard.check()
  if (typeof result === "boolean") return result
  return false
}

export function navigateIfAllowed(path: string): boolean {
  const resolved = resolveRoute(path, [
    ...DASHBOARD_ROUTES,
    ...router.get().registeredAppRoutes,
  ])

  if (!resolved) return false

  if (!checkRouteGuard(resolved)) {
    if (resolved.definition.guard?.redirectTo) {
      navigate(resolved.definition.guard.redirectTo)
    }
    return false
  }

  navigate(path)
  return true
}

export function buildAppRoute(
  appId: string,
  subPath: string,
  params?: Record<string, string>,
): string {
  const path = `/apps/${appId}${subPath}`
  if (params) {
    return buildRoutePath(path, params)
  }
  return path
}
