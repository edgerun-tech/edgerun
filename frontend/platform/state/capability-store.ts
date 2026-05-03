/**
 * Single source of truth for capabilities available and granted across nodes/apps/sessions.
 * Uses generated protobuf types.
 */

import { atom, computed } from "nanostores"
import { protocolClient } from "@/platform/protocol/client"
import { edgerun as edgerunCap } from "@/gen/edgerun/v0/capability"
import { edgerun as edgerunTrust } from "@/gen/edgerun/v0/trust"

export interface CapabilityStoreState {
  descriptors: Map<string, edgerunCap.v0.capability.CapabilityDescriptor>
  grants: Map<string, edgerunCap.v0.capability.CapabilityGrant[]>
  delegations: Map<string, edgerunTrust.v0.trust.DelegationRecord>
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

export function listCapabilitiesByNode(nodeId: string): edgerunCap.v0.capability.CapabilityDescriptor[] {
  return Array.from(capabilityStore.get().descriptors.values()).filter(
    (d) => {
      const providerId = d.provider_node?.node_id
      return providerId && Buffer.from(providerId).toString("hex").startsWith(nodeId)
    },
  )
}

export function listGrantsForAppFromStore(appId: string): edgerunCap.v0.capability.CapabilityGrant[] {
  return capabilityStore.get().grants.get(appId) ?? []
}

export async function loadCapabilities(): Promise<void> {
  const state = capabilityStore.get()
  capabilityStore.set({ ...state, isLoading: true, error: null })

  try {
    const response = await protocolClient.send({ method: "GET", path: "/protocol/capabilities" })
    if (response.status !== 200) throw new Error(`Failed: ${response.status}`)

    const text = new TextDecoder().decode(response.body)
    const items: Array<any> = JSON.parse(text)
    const descriptors = items.map((obj) => edgerunCap.v0.capability.CapabilityDescriptor.fromObject(obj))
    const newDescriptors = new Map<string, edgerunCap.v0.capability.CapabilityDescriptor>()
    for (const desc of descriptors) {
      const id = Buffer.from(desc.capability_id).toString("hex")
      newDescriptors.set(id, desc)
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
      error: err instanceof Error ? err.message : "Failed to load capabilities",
    })
  }
}

export async function loadGrantsForApp(appId: string): Promise<void> {
  try {
    const response = await protocolClient.send({
      method: "GET",
      path: `/protocol/app/${appId}/grants`,
    })
    if (response.status !== 200) return

    const text = new TextDecoder().decode(response.body)
    const items: Array<any> = JSON.parse(text)
    const grants = items.map((obj) => edgerunCap.v0.capability.CapabilityGrant.fromObject(obj))
    const state = capabilityStore.get()
    const newGrants = new Map(state.grants)
    newGrants.set(appId, grants)
    capabilityStore.set({ ...state, grants: newGrants })
  } catch {
    // Silently fail
  }
}
