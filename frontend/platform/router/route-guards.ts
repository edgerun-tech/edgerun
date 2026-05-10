/**
 * Route guards based on capabilities/permissions.
 */

import type { RouteGuard, RouteParams } from "./route-types"
import { hasPermission } from "@/platform/auth/permission-tracker"
import { canSatisfy } from "@/platform/registries/capability-registry"
import type { PermissionScope } from "@/platform/state/permission-store"
import { bytesToHex } from "@/platform/utils/bytes"

export function createCapabilityGuard(
  requiredCapabilities: string[],
): RouteGuard {
  return {
    check: () => {
      if (requiredCapabilities.length === 0) return true
      const available = canSatisfy({})
      const availableIds = new Set(
        available.map((c) => bytesToHex(c.capability_id))
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
    check: () => hasPermission(permission as PermissionScope),
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
  const allowed = guard.check()
  if (!allowed) {
    return {
      allowed: false,
      redirectTo: guard.redirectTo,
      message: guard.message,
    }
  }
  return { allowed: true }
}
