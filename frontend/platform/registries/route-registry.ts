/**
 * Single registry for routes.
 * Tracks dashboard and app routes.
 */

import { atom, computed } from "nanostores"

export interface RouteDefinition {
  path: string
  appId?: string
  routeType: "dashboard" | "app" | "object" | "command" | "pipeline" | "settings"
  requiredCapabilities: string[]
  viewSpec?: unknown
}

export interface RouteRegistryState {
  routes: Map<string, RouteDefinition>
  appRoutes: Map<string, RouteDefinition[]>
}

const initialState: RouteRegistryState = {
  routes: new Map(),
  appRoutes: new Map(),
}

export const routeRegistry = atom<RouteRegistryState>(initialState)

export const allRoutes = computed(routeRegistry, (s) =>
  Array.from(s.routes.values()),
)

export function registerRoute(route: RouteDefinition): void {
  const state = routeRegistry.get()
  const newRoutes = new Map(state.routes)
  newRoutes.set(route.path, route)

  if (route.appId) {
    const newAppRoutes = new Map(state.appRoutes)
    const existing = newAppRoutes.get(route.appId) || []
    newAppRoutes.set(route.appId, [...existing, route])
    routeRegistry.set({
      routes: newRoutes,
      appRoutes: newAppRoutes,
    })
  } else {
    routeRegistry.set({ ...state, routes: newRoutes })
  }
}

export function getRoute(path: string): RouteDefinition | undefined {
  return routeRegistry.get().routes.get(path)
}

export function listAppRoutes(appId: string): RouteDefinition[] {
  return routeRegistry.get().appRoutes.get(appId) || []
}

export function matchRoute(path: string): RouteDefinition | undefined {
  const routes = routeRegistry.get().routes
  // Try exact match first
  const exact = routes.get(path)
  if (exact) return exact

  // Try pattern matching
  for (const route of routes.values()) {
    const pattern = route.path.replace(/:\w+/g, "[^/]+")
    const regex = new RegExp(`^${pattern}$`)
    if (regex.test(path)) return route
  }

  return undefined
}
