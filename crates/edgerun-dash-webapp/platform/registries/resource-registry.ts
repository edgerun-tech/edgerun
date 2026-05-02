/**
 * Resource registry.
 * Normalizes resource lookups and per-node resource resolution.
 */

import { atom, computed } from "nanostores"
import type { NodeResource, ResourceSource } from "./resource-store"

export interface ResourceRegistryState {
  nodeResources: Map<string, NodeResource>
  labelMap: Map<string, string> // nodeId → label
}

const initialState: ResourceRegistryState = {
  nodeResources: new Map(),
  labelMap: new Map(),
}

export const resourceRegistry = atom<ResourceRegistryState>(initialState)

export const allNodeResources = computed(resourceRegistry, (s) =>
  Array.from(s.nodeResources.values()),
)

export function registerNodeResource(nodeId: string, resource: NodeResource): void {
  const state = resourceRegistry.get()
  const newMap = new Map(state.nodeResources)
  newMap.set(nodeId, resource)
  resourceRegistry.set({ ...state, nodeResources: newMap })
}

export function getNodeResource(nodeId: string): NodeResource | undefined {
  return resourceRegistry.get().nodeResources.get(nodeId)
}

export function listUnhealthyNodes(): NodeResource[] {
  return Array.from(resourceRegistry.get().nodeResources.values())
    .filter(n => n.health === "unhealthy" || n.health === "failed")
}

export function listStaleNodes(timeoutMs = 60000): NodeResource[] {
  const now = Date.now()
  return Array.from(resourceRegistry.get().nodeResources.values())
    .filter(n => now - n.lastHeartbeat > timeoutMs)
}

export function getWeightedCpuUtilization(): number {
  const nodes = Array.from(resourceRegistry.get().nodeResources.values())
  let weighted = 0
  let totalCores = 0
  for (const n of nodes) {
    weighted += n.cpu.cores * n.cpu.utilization
    totalCores += n.cpu.cores
  }
  return totalCores > 0 ? weighted / totalCores : 0
}

export function formatNodeLabel(nodeId: string): string {
  return resourceRegistry.get().labelMap.get(nodeId) || nodeId.slice(0, 12)
}
