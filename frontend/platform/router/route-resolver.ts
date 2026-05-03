/**
 * Route resolution logic.
 */

import type { RouteDefinition, RouteParams, ResolvedRoute } from "./route-types"
import { DASHBOARD_ROUTES } from "./route-types"

export function resolveRoute(
  path: string,
  routes: RouteDefinition[] = DASHBOARD_ROUTES,
): ResolvedRoute | null {
  for (const route of routes) {
    const match = matchRoutePattern(route.path, path)
    if (match) {
      return {
        definition: route,
        params: match,
        query: parseQueryString(window.location.search),
      }
    }
    if (route.children) {
      const childResolved = resolveRoute(path, route.children)
      if (childResolved) return childResolved
    }
  }
  return null
}

export function matchRoutePattern(
  pattern: string,
  path: string,
): RouteParams | null {
  const paramNames: string[] = []
  const regexPattern = pattern.replace(/:(\w+)/g, (_match, paramName) => {
    paramNames.push(paramName)
    return "([^/]+)"
  })

  const regex = new RegExp(`^${regexPattern}$`)
  const match = path.match(regex)

  if (!match) return null

  const params: RouteParams = {}
  paramNames.forEach((name, index) => {
    params[name] = match[index + 1]
  })

  return params
}

export function buildRoutePath(
  pattern: string,
  params: RouteParams,
): string {
  let path = pattern
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) {
      path = path.replace(`:${key}`, value)
    }
  }
  return path
}

export function parseQueryString(query: string): Record<string, string> {
  const params = new URLSearchParams(query)
  const result: Record<string, string> = {}
  for (const [key, value] of params.entries()) {
    result[key] = value
  }
  return result
}

export function isAppRoute(path: string): boolean {
  return path.startsWith("/apps/")
}

export function isObjectRoute(path: string): boolean {
  return path.startsWith("/objects/")
}

export function isPipelineRoute(path: string): boolean {
  return path.startsWith("/pipelines")
}

export function extractAppId(path: string): string | null {
  const match = path.match(/\/apps\/([^/]+)/)
  return match ? match[1] : null
}
