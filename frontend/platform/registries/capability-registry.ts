/**
 * Single registry for capabilities.
 * Tracks descriptors, grants, and provides lookup/normalization.
 */

import { atom, computed } from "nanostores"
import { edgerun as edgerunCap } from "@/gen/edgerun/v0/capability"
import { capabilityStore } from "@/stores/capability-store"
import { bytesToHex } from "@/platform/utils/bytes"

export interface CapabilityRegistryState {
  descriptors: Map<string, edgerunCap.v0.capability.CapabilityDescriptor>
  grants: Map<string, edgerunCap.v0.capability.CapabilityGrant[]>
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

export function registerCapability(desc: edgerunCap.v0.capability.CapabilityDescriptor): void {
  const state = capabilityRegistry.get()
  const newDescriptors = new Map(state.descriptors)
  const id = bytesToHex(desc.capability_id)
  newDescriptors.set(id, desc)
  capabilityRegistry.set({ ...state, descriptors: newDescriptors })
}

export function registerGrant(grant: edgerunCap.v0.capability.CapabilityGrant): void {
  const state = capabilityRegistry.get()
  const newGrants = new Map(state.grants)
  const granteeId = bytesToHex(grant.grantee?.identity_id)
  const existing = newGrants.get(granteeId) || []
  newGrants.set(granteeId, [...existing, grant])
  capabilityRegistry.set({ ...state, grants: newGrants })
}

export function getCapabilityById(
  capabilityId: string,
): edgerunCap.v0.capability.CapabilityDescriptor | undefined {
  return capabilityRegistry.get().descriptors.get(capabilityId)
}

export function listGrantsForApp(appId: string): edgerunCap.v0.capability.CapabilityGrant[] {
  return capabilityRegistry.get().grants.get(appId) || []
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


export function canSatisfy(
  selector: { capabilityType?: string; riskClass?: string; requiresUserPresence?: boolean },
): edgerunCap.v0.capability.CapabilityDescriptor[] {
  const state = capabilityRegistry.get()
  return Array.from(state.descriptors.values()).filter((cap) => {
    if (selector.capabilityType && bytesToHex(cap.capability_id) !== selector.capabilityType)
      return false
    return true
  })
}
