/**
 * Given app/action/pipeline, calculate required capabilities.
 * Compare against current grants and available nodes.
 * Return explicit plan: satisfied/missing/requires approval/requires external auth/impossible/expired.
 */

import { capabilityRegistry } from "@/platform/registries/capability-registry"
import { appRegistry } from "@/platform/registries/app-registry"
import { edgerun as edgerunTrust } from "@/gen/edgerun/v0/trust"
import { edgerun as edgerunStream } from "@/gen/edgerun/v0/stream"
import { edgerun as edgerunCap } from "@/gen/edgerun/v0/capability"
import { bytesToHex } from "@/platform/utils/bytes"

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

function appRequiredCapabilityLabels(
  app: edgerunStream.v0.stream.AppPackage,
): string[] {
  const required = app.required_capabilities || []
  const labels: string[] = []
  for (const cap of required) {
    if (cap.capability_kind !== undefined && cap.capability_kind !== edgerunTrust.v0.trust.CapabilityKind.CAPABILITY_KIND_UNSPECIFIED) {
      labels.push(edgerunTrust.v0.trust.CapabilityKind[cap.capability_kind] || `kind_${cap.capability_kind}`)
    }
  }
  return labels
}

function registeredCapabilityLabels(): Map<string, edgerunCap.v0.capability.CapabilityDescriptor> {
  const state = capabilityRegistry.get()
  const byLabel = new Map<string, edgerunCap.v0.capability.CapabilityDescriptor>()
  for (const [, desc] of state.descriptors) {
    const id = bytesToHex(desc.capability_id)
    if (desc.provider_name) {
      byLabel.set(desc.provider_name, desc)
    }
    byLabel.set(id, desc)
  }
  return byLabel
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

  const requiredLabels = appRequiredCapabilityLabels(app)
  return resolveCapabilities(requiredLabels, appId)
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

  const requiredLabels = appRequiredCapabilityLabels(app)
  return resolveCapabilities(requiredLabels, appId)
}

function resolveCapabilities(
  required: string[],
  appId: string,
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

  const available = registeredCapabilityLabels()
  const registryState = capabilityRegistry.get()

  const appGrants = new Set<string>()
  const grants = registryState.grants.get(appId) || []
  for (const grant of grants) {
    if (grant.selector?.capability_id) {
      const grantCapId = bytesToHex(grant.selector.capability_id)
      appGrants.add(grantCapId)
    }
    if (grant.grantee?.identity_id) {
      const granteeId = bytesToHex(grant.grantee.identity_id)
      if (granteeId === appId) {
        appGrants.add(granteeId)
      }
    }
  }

  for (const label of required) {
    const desc = available.get(label)
    const isAvailable = !!desc
    const isGranted = desc ? appGrants.has(bytesToHex(desc.capability_id)) : false

    if (!isAvailable) {
      plan.missing.push(label)
      plan.satisfied = false
      plan.actions.push({
        type: "grant_capability",
        description: `Grant capability ${label}`,
        capabilityId: label,
      })
    } else if (!isGranted) {
      plan.requiresApproval.push(label)
      plan.satisfied = false
      plan.actions.push({
        type: "request_approval",
        description: `Request approval for capability ${label}`,
        capabilityId: label,
      })
    }
  }

  if (plan.missing.length > 0) {
    plan.explanation = `Missing capabilities: ${plan.missing.join(", ")}`
  } else if (plan.requiresApproval.length > 0) {
    plan.explanation = `Requires approval for: ${plan.requiresApproval.join(", ")}`
  } else {
    plan.explanation = "All capabilities satisfied"
  }

  return plan
}
