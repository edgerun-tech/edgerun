/**
 * Route guards based on capabilities/permissions.
 */

import type { RouteGuard, RouteParams } from "./route-types"
import { hasPermission } from "@/platform/auth/permission-tracker"
import { canSatisfy } from "@/platform/registries/capability-registry"

export function createCapabilityGuard(
  requiredCapabilities: string[],
): RouteGuard {
  return {
    check: () => {
      if (requiredCapabilities.length === 0) return true
      const available = canSatisfy({})
      const availableIds = new Set(
        available.map((c) => Buffer.from(c.capability_id).toString("hex"))
      )
      return requiredCapabilities.every((id) => availableIds.has(id))
    },
    redirectTo: "/approvals",
    message: `Requires capabilities: ${requiredCapabilities.join(", ")}`,
  }
}

export function createPermissionGuard(
  permission: string,
): RouteGuard {
  return {
    check: () => hasPermission(permission),
    redirectTo: "/login",
    message: `Requires permission: ${permission}`,
  }
}

export const defaultGuards: Record<string, RouteGuard> = {
  admin: createPermissionGuard("admin"),
  apps: createCapabilityGuard([]),
  settings: createPermissionGuard("settings"),
}

export function checkRouteGuard(
  guard: RouteGuard | undefined,
  params: RouteParams,
): { allowed: boolean; redirectTo?: string; message?: string } {
  if (!guard) return { allowed: true }
  const allowed = guard.check(params)
  if (!allowed) {
    return {
      allowed: false,
      redirectTo: guard.redirectTo,
      message: guard.message,
    }
  }
  return { allowed: true }
}
