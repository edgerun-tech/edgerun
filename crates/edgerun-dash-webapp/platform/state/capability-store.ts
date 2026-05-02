/**
 * Single source of truth for capabilities available and granted across nodes/apps/sessions.
 * Tracks descriptors, grants, selectors, delegation state.
 */

import { atom, computed } from "nanostores"
import { protocolClient } from "@/platform/protocol/client"
import type {
  CapabilityDescriptor,
  CapabilityGrant,
  CapabilitySelector,
  CapabilitySatisfaction,
  DelegationRecord,
} from "@/platform/protocol/capabilities"
import { listAvailableCapabilities, listGrantsForApp } from "@/platform/protocol/capabilities"

export interface CapabilityStoreState {
  descriptors: Map<string, CapabilityDescriptor>
  grants: Map<string, CapabilityGrant[]>
  delegations: Map<string, DelegationRecord>
  isLoading: boolean
  error: string | null
  lastRefresh: string | null
}

const initialState: CapabilityStoreState = {
  descriptors: new Map(),
  grants: new Map(),
  delegations: new Map(),
  isLoading: false,
  error: null,
  lastRefresh: null,
}

export const capabilityStore = atom<CapabilityStoreState>(initialState)

export const availableCapabilities = computed(capabilityStore, (s) =>
  Array.from(s.descriptors.values()),
)

export const capabilityCount = computed(
  capabilityStore,
  (s) => s.descriptors.size,
)

export function listAvailableCapabilitiesByNode(): CapabilityDescriptor[] {
  return Array.from(capabilityStore.get().descriptors.values())
}

export function listCapabilitiesByNode(nodeId: string): CapabilityDescriptor[] {
  return Array.from(capabilityStore.get().descriptors.values()).filter(
    (d) => d.capabilityId.startsWith(nodeId),
  )
}

export function listGrantsForAppFromStore(appId: string): CapabilityGrant[] {
  return capabilityStore.get().grants.get(appId) ?? []
}

export function listGrantsForSession(sessionId: string): CapabilityGrant[] {
  return Array.from(capabilityStore.get().grants.values())
    .flat()
    .filter((g) => g.granteeId === sessionId && g.granteeType === "session")
}

export function canSatisfy(
  selector: CapabilitySelector,
): CapabilityDescriptor[] {
  const descriptors = Array.from(
    capabilityStore.get().descriptors.values(),
  )
  return descriptors.filter((cap) => {
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

export function explainMissingCapabilities(required: string[]): string {
  const availableIds = new Set(
    Array.from(capabilityStore.get().descriptors.keys()),
  )
  const missing = required.filter((id) => !availableIds.has(id))
  if (missing.length === 0) return "All capabilities satisfied"
  return `Missing capabilities: ${missing.join(", ")}`
}

export async function loadCapabilities(): Promise<void> {
  const state = capabilityStore.get()
  capabilityStore.set({ ...state, isLoading: true, error: null })

  try {
    const descriptors = await listAvailableCapabilities()
    const newDescriptors = new Map<string, CapabilityDescriptor>()
    for (const desc of descriptors) {
      newDescriptors.set(desc.capabilityId, desc)
    }
    capabilityStore.set({
      ...capabilityStore.get(),
      descriptors: newDescriptors,
      isLoading: false,
      lastRefresh: new Date().toISOString(),
    })
  } catch (err) {
    capabilityStore.set({
      ...capabilityStore.get(),
      isLoading: false,
      error:
        err instanceof Error ? err.message : "Failed to load capabilities",
    })
  }
}

export async function loadGrantsForApp(appId: string): Promise<void> {
  try {
    const grants = await listGrantsForApp(appId)
    const state = capabilityStore.get()
    const newGrants = new Map(state.grants)
    newGrants.set(appId, grants)
    capabilityStore.set({ ...state, grants: newGrants })
  } catch {
    // Silently fail
  }
}

export function getCapabilitySatisfaction(
  appId: string,
  actionId: string,
): CapabilitySatisfaction {
  const appGrants = listGrantsForAppFromStore(appId)
  const appCapabilities = appGrants.map((g) => g.capabilityId)
  const available = Array.from(
    capabilityStore.get().descriptors.keys(),
  )

  const missing = appCapabilities.filter((id) => !available.includes(id))

  return {
    satisfied: missing.length === 0,
    missing,
    requiresApproval: false,
    requiresExternalAuth: false,
    impossible: false,
    expired: false,
    explanation:
      missing.length === 0
        ? "All capabilities satisfied"
        : `Missing: ${missing.join(", ")}`,
  }
}
