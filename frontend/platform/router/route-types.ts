/**
 * Route type definitions for the EdgeRun router.
 */

export interface RouteDefinition {
  path: string
  component?: React.ComponentType<unknown>
  appId?: string
  requiredCapabilities: string[]
  guard?: RouteGuard
  children?: RouteDefinition[]
}

export interface RouteGuard {
  check: () => boolean | Promise<boolean>
  redirectTo?: string
  message?: string
}

export interface RouteParams {
  [key: string]: string | undefined
}

export interface ResolvedRoute {
  definition: RouteDefinition
  params: RouteParams
  query: Record<string, string>
}

export type RouteChangeCallback = (route: ResolvedRoute) => void

export const DASHBOARD_ROUTES: RouteDefinition[] = [
  {
    path: "/dashboard",
    requiredCapabilities: [],
  },
  {
    path: "/apps",
    requiredCapabilities: [],
  },
  {
    path: "/apps/:appId",
    requiredCapabilities: [],
  },
  {
    path: "/apps/:appId/routes/*",
    requiredCapabilities: [],
  },
  {
    path: "/objects/:objectRef",
    requiredCapabilities: [],
  },
  {
    path: "/commands/:commandRef",
    requiredCapabilities: [],
  },
  {
    path: "/pipelines",
    requiredCapabilities: [],
  },
  {
    path: "/pipelines/:pipelineId",
    requiredCapabilities: [],
  },
  {
    path: "/approvals",
    requiredCapabilities: ["capability_grant"],
  },
  {
    path: "/connections",
    requiredCapabilities: [],
  },
  {
    path: "/settings",
    requiredCapabilities: [],
  },
  {
    path: "/studio",
    requiredCapabilities: [],
  },
]
