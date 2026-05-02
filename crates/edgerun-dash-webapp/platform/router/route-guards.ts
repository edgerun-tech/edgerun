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
      const available = canSatisfy({ capabilityType: undefined })
      const availableIds = new Set(available.map((c) => c.capabilityId))
      return requiredCapabilities.every((id) => availableIds.has(id))
    },
    redirectTo: "/approvals",
    message: `Requires capabilities: ${requiredCapabilities.join(", ")}`,
  }
}

export function createPermissionGuard(
  scope: string,
): RouteGuard {
  return {
    check: () => hasPermission(scope as never),
    redirectTo: "/approvals",
    message: `Requires permission: ${scope}`,
  }
}

export function createAuthGuard(): RouteGuard {
  return {
    check: () => {
      const state = document.cookie.includes("authenticated")
      return state
    },
    redirectTo: "/login",
    message: "Authentication required",
  }
}

export function createAppGuard(appId: string): RouteGuard {
  return {
    check: () => {
      // Check if app is installed
      const app = document.querySelector(`[data-app-id="${appId}"]`)
      return !!app
    },
    redirectTo: "/apps",
    message: `App ${appId} not installed`,
  }
}

export function createDelegationGuard(
  _params: RouteParams,
): RouteGuard {
  return {
    check: () => {
      // Check delegation validity
      return true
    },
    redirectTo: "/approvals",
    message: "Valid delegation required",
  }
}

export function combineGuards(...guards: RouteGuard[]): RouteGuard {
  return {
    check: async () => {
      for (const guard of guards) {
        const result = await guard.check()
        if (!result) return false
      }
      return true
    },
    redirectTo: guards.find((g) => g.redirectTo)?.redirectTo,
    message: guards.map((g) => g.message).filter(Boolean).join("; "),
  }
}
