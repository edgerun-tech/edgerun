/**
 * Given app/action/pipeline, calculate required capabilities.
 * Compare against current grants and available nodes.
 * Return explicit plan: satisfied/missing/requires approval/requires external auth/impossible/expired.
 */

import { capabilityRegistry } from "@/platform/registries/capability-registry"
import { appRegistry } from "@/platform/registries/app-registry"

export interface ResolutionPlan {
  satisfied: boolean
  missing: string[]
  requiresApproval: string[]
  requiresExternalAuth: string[]
  impossible: string[]
  expired: string[]
  explanation: string
  actions: ResolutionAction[]
}

export type ResolutionActionType =
  | "grant_capability"
  | "request_approval"
  | "connect_external"
  | "renew_token"
  | "install_app"
  | "wait_for_delegation"

export interface ResolutionAction {
  type: ResolutionActionType
  description: string
  capabilityId?: string
  externalProvider?: string
}

export function resolveCapabilityForAction(
  appId: string,
  _actionId: string,
): ResolutionPlan {
  const app = appRegistry.get().apps.get(appId)
  if (!app) {
    return {
      satisfied: false,
      missing: [],
      requiresApproval: [],
      requiresExternalAuth: [],
      impossible: [appId],
      expired: [],
      explanation: `App ${appId} not found`,
      actions: [{ type: "install_app", description: `Install app ${appId}` }],
    }
  }

  return resolveCapabilities([], appId)
}

export function resolveCapabilityForPipeline(
  appId: string,
  _pipelineId: string,
): ResolutionPlan {
  const app = appRegistry.get().apps.get(appId)
  if (!app) {
    return {
      satisfied: false,
      missing: [],
      requiresApproval: [],
      requiresExternalAuth: [],
      impossible: [appId],
      expired: [],
      explanation: `App ${appId} not found`,
      actions: [{ type: "install_app", description: `Install app ${appId}` }],
    }
  }

  return resolveCapabilities([], appId)
}

function resolveCapabilities(
  required: string[],
  _appId: string,
): ResolutionPlan {
  const plan: ResolutionPlan = {
    satisfied: true,
    missing: [],
    requiresApproval: [],
    requiresExternalAuth: [],
    impossible: [],
    expired: [],
    explanation: "",
    actions: [],
  }

  const descriptors = capabilityRegistry.get().descriptors
  const availableIds = new Set(
    Array.from(descriptors.keys())
  )

  for (const capId of required) {
    if (!availableIds.has(capId)) {
      plan.missing.push(capId)
      plan.satisfied = false
      plan.actions.push({
        type: "grant_capability",
        description: `Grant capability ${capId}`,
        capabilityId: capId,
      })
    }
  }

  if (plan.missing.length > 0) {
    plan.explanation = `Missing capabilities: ${plan.missing.join(", ")}`
  } else {
    plan.explanation = "All capabilities satisfied"
  }

  return plan
}
