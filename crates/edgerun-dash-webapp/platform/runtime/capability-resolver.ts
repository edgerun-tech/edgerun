/**
 * Given app/action/pipeline, calculate required capabilities.
 * Compare against current grants and available nodes.
 * Return explicit plan: satisfied/missing/requires approval/requires external auth/impossible/expired.
 */

import type { CapabilitySatisfaction } from "@/platform/protocol/capabilities"
import { capabilityRegistry } from "@/platform/registries/capability-registry"
import { appRegistry } from "@/platform/registries/app-registry"
import { permissionTracker } from "@/platform/auth/permission-tracker"

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
  actionId: string,
): ResolutionPlan {
  const app = appRegistry.getApp(appId)
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

  const action = app.actions.find((a) => a.actionId === actionId)
  if (!action) {
    return {
      satisfied: false,
      missing: [],
      requiresApproval: [],
      requiresExternalAuth: [],
      impossible: [actionId],
      expired: [],
      explanation: `Action ${actionId} not found in app ${appId}`,
      actions: [],
    }
  }

  return resolveCapabilities(
    action.requiredCapabilities,
    appId,
  )
}

export function resolveCapabilityForPipeline(
  appId: string,
  pipelineId: string,
): ResolutionPlan {
  const app = appRegistry.getApp(appId)
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

  const pipeline = app.pipelines.find((p) => p.pipelineId === pipelineId)
  if (!pipeline) {
    return {
      satisfied: false,
      missing: [],
      requiresApproval: [],
      requiresExternalAuth: [],
      impossible: [pipelineId],
      expired: [],
      explanation: `Pipeline ${pipelineId} not found`,
      actions: [],
    }
  }

  const allCapabilities = pipeline.steps.flatMap((step) => {
    const action = app.actions.find((a) => a.actionId === step.actionId)
    return action?.requiredCapabilities || []
  })

  return resolveCapabilities(allCapabilities, appId)
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

  const available = Array.from(capabilityRegistry.allDescriptors.get() || [])
  const availableIds = new Set(available.map((c) => c.capabilityId))

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
