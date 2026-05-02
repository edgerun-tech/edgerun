/**
 * Single registry for capabilities.
 * Tracks descriptors, grants, and provides lookup/normalization.
 */

import { atom, computed } from "nanostores"
import type {
  CapabilityDescriptor,
  CapabilityGrant,
  CapabilitySelector,
  CapabilitySatisfaction,
} from "@/platform/protocol/capabilities"
import { capabilityStore } from "@/platform/state/capability-store"

export interface CapabilityRegistryState {
  descriptors: Map<string, CapabilityDescriptor>
  grants: Map<string, CapabilityGrant[]>
  nodeCapabilities: Map<string, string[]>
}

const initialState: CapabilityRegistryState = {
  descriptors: new Map(),
  grants: new Map(),
  nodeCapabilities: new Map(),
}

export const capabilityRegistry = atom<CapabilityRegistryState>(initialState)

export const allDescriptors = computed(capabilityRegistry, (s) =>
  Array.from(s.descriptors.values()),
)

export function registerCapability(desc: CapabilityDescriptor): void {
  const state = capabilityRegistry.get()
  const newDescriptors = new Map(state.descriptors)
  newDescriptors.set(desc.capabilityId, desc)
  capabilityRegistry.set({ ...state, descriptors: newDescriptors })
}

export function registerGrant(grant: CapabilityGrant): void {
  const state = capabilityRegistry.get()
  const newGrants = new Map(state.grants)
  const existing = newGrants.get(grant.granteeId) || []
  newGrants.set(grant.granteeId, [...existing, grant])
  capabilityRegistry.set({ ...state, grants: newGrants })
}

export function getCapabilityById(
  capabilityId: string,
): CapabilityDescriptor | undefined {
  return capabilityRegistry.get().descriptors.get(capabilityId)
}

export function listCapabilitiesByNode(nodeId: string): CapabilityDescriptor[] {
  const state = capabilityRegistry.get()
  const capabilityIds = state.nodeCapabilities.get(nodeId) || []
  return capabilityIds
    .map((id) => state.descriptors.get(id))
    .filter(Boolean) as CapabilityDescriptor[]
}

export function listGrantsForApp(appId: string): CapabilityGrant[] {
  return capabilityRegistry.get().grants.get(appId) || []
}

export function listGrantsForSession(sessionId: string): CapabilityGrant[] {
  return Array.from(capabilityRegistry.get().grants.values())
    .flat()
    .filter((g) => g.granteeId === sessionId && g.granteeType === "session")
}

export function canSatisfy(
  selector: CapabilitySelector,
): CapabilityDescriptor[] {
  const state = capabilityRegistry.get()
  return Array.from(state.descriptors.values()).filter((cap) => {
    if (selector.capabilityType && cap.capabilityType !== selector.capabilityType)
      return false
    if (selector.riskClass && cap.riskClass !== selector.riskClass)
      return false
    if (
      selector.requiresUserPresence !== undefined &&
      cap.requiresUserPresence !== selector.requiresUserPresence
    )
      return false
    return true
  })
}

export function explainMissingCapabilities(
  required: string[],
): string {
  const state = capabilityRegistry.get()
  const availableIds = new Set(state.descriptors.keys())
  const missing = required.filter((id) => !availableIds.has(id))
  if (missing.length === 0) return "All capabilities satisfied"
  return `Missing capabilities: ${missing.join(", ")}`
}

export function resolveCapabilityForAction(
  _appId: string,
  _actionId: string,
): CapabilitySatisfaction {
  return {
    satisfied: true,
    missing: [],
    requiresApproval: false,
    requiresExternalAuth: false,
    impossible: false,
    expired: false,
    explanation: "Capability satisfied",
  }
}

export function resolveCapabilityForPipeline(
  _pipelineId: string,
): CapabilitySatisfaction {
  return {
    satisfied: true,
    missing: [],
    requiresApproval: false,
    requiresExternalAuth: false,
    impossible: false,
    expired: false,
    explanation: "Capability satisfied",
  }
}

// Sync from capability store
capabilityStore.listen((state) => {
  const registryState = capabilityRegistry.get()
  const newDescriptors = new Map(registryState.descriptors)
  const newGrants = new Map(registryState.grants)

  for (const [id, desc] of state.descriptors) {
    newDescriptors.set(id, desc)
  }
  for (const [id, grants] of state.grants) {
    newGrants.set(id, grants)
  }

  capabilityRegistry.set({
    ...registryState,
    descriptors: newDescriptors,
    grants: newGrants,
  })
})
