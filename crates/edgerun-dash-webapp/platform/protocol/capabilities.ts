/**
 * Capability protocol helpers.
 * Handles CapabilityGrant, CapabilityDescriptor, and capability-related protocol messages.
 */

import { protocolClient } from "./client"
import type { ObjectRef, NodeRef } from "./refs"

export interface CapabilityDescriptor {
  capabilityId: string
  capabilityType: string
  label: string
  description: string
  riskClass: "low" | "medium" | "high" | "critical"
  requiresUserPresence: boolean
  requiredDelegations: string[]
  constraints: CapabilityConstraint[]
}

export interface CapabilityConstraint {
  key: string
  operator: "equals" | "contains" | "greater_than" | "less_than"
  value: string
}

export interface CapabilityGrant {
  grantId: string
  capabilityId: string
  granteeId: string
  granteeType: "app" | "session" | "identity"
  grantedAt: string
  expiresAt?: string
  constraints: CapabilityConstraint[]
  delegations: string[]
  revoked: boolean
}

export interface DelegationRecord {
  delegationId: string
  parentDelegationId?: string
  issuerIdentity: string
  targetIdentity: string
  actions: string[]
  scope: string[]
  notBefore?: string
  notAfter?: string
  constraints: CapabilityConstraint[]
  signatureBytes: Uint8Array
}

export interface CapabilitySelector {
  capabilityType?: string
  riskClass?: string
  requiresUserPresence?: boolean
  requiredDelegations?: string[]
}

export interface CapabilitySatisfaction {
  satisfied: boolean
  missing: string[]
  requiresApproval: boolean
  requiresExternalAuth: boolean
  impossible: boolean
  expired: boolean
  explanation: string
}

export async function listAvailableCapabilities(
  nodeId?: string,
): Promise<CapabilityDescriptor[]> {
  const path = nodeId
    ? `/protocol/capabilities?node=${nodeId}`
    : "/protocol/capabilities"
  const response = await protocolClient.send({ method: "GET", path })
  if (response.status !== 200) return []
  const text = new TextDecoder().decode(response.body)
  return JSON.parse(text) as CapabilityDescriptor[]
}

export async function listGrantsForApp(
  appId: string,
): Promise<CapabilityGrant[]> {
  const response = await protocolClient.send({
    method: "GET",
    path: `/protocol/app/${appId}/grants`,
  })
  if (response.status !== 200) return []
  const text = new TextDecoder().decode(response.body)
  return JSON.parse(text) as CapabilityGrant[]
}

export async function listGrantsForSession(
  sessionId: string,
): Promise<CapabilityGrant[]> {
  const response = await protocolClient.send({
    method: "GET",
    path: `/protocol/session/${sessionId}/grants`,
  })
  if (response.status !== 200) return []
  const text = new TextDecoder().decode(response.body)
  return JSON.parse(text) as CapabilityGrant[]
}

export function canSatisfy(
  selector: CapabilitySelector,
  available: CapabilityDescriptor[],
): CapabilityDescriptor[] {
  return available.filter((cap) => {
    if (selector.capabilityType && cap.capabilityType !== selector.capabilityType)
      return false
    if (selector.riskClass && cap.riskClass !== selector.riskClass) return false
    if (
      selector.requiresUserPresence !== undefined &&
      cap.requiresUserPresence !== selector.requiresUserPresence
    )
      return false
    if (selector.requiredDelegations) {
      for (const req of selector.requiredDelegations) {
        if (!cap.requiredDelegations.includes(req)) return false
      }
    }
    return true
  })
}

export function explainMissingCapabilities(
  required: string[],
  available: CapabilityDescriptor[],
): string {
  const availableIds = new Set(available.map((c) => c.capabilityId))
  const missing = required.filter((id) => !availableIds.has(id))
  if (missing.length === 0) return "All capabilities satisfied"
  return `Missing capabilities: ${missing.join(", ")}`
}

export async function resolveCapabilityForAction(
  appId: string,
  actionId: string,
): Promise<CapabilitySatisfaction> {
  const response = await protocolClient.send({
    method: "POST",
    path: "/protocol/capability/resolve",
    body: new TextEncoder().encode(
      JSON.stringify({ appId, actionId }),
    ),
  })
  if (response.status !== 200) {
    return {
      satisfied: false,
      missing: [],
      requiresApproval: false,
      requiresExternalAuth: false,
      impossible: true,
      expired: false,
      explanation: "Failed to resolve capability",
    }
  }
  const text = new TextDecoder().decode(response.body)
  return JSON.parse(text) as CapabilitySatisfaction
}

export async function resolveCapabilityForPipeline(
  pipelineId: string,
): Promise<CapabilitySatisfaction> {
  const response = await protocolClient.send({
    method: "POST",
    path: "/protocol/capability/resolve-pipeline",
    body: new TextEncoder().encode(JSON.stringify({ pipelineId })),
  })
  if (response.status !== 200) {
    return {
      satisfied: false,
      missing: [],
      requiresApproval: false,
      requiresExternalAuth: false,
      impossible: true,
      expired: false,
      explanation: "Failed to resolve capability",
    }
  }
  const text = new TextDecoder().decode(response.body)
  return JSON.parse(text) as CapabilitySatisfaction
}
