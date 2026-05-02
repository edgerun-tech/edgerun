/**
 * Multi-node resource state.
 * Aggregates resources across all user nodes.
 * Tracks per-node and aggregate metrics with source labels.
 */

import { atom, computed } from "nanostores"
import type { NodeContext } from "./assitant-context"

export type ResourceSource = 
  | "node_reported"
  | "coordinator_observed"
  | "locally_cached"
  | "git_derived"
  | "test_artifact"
  | "benchmark_artifact"
  | "agent_claim"
  | "command_event"
  | "unknown"

export interface NodeResource {
  nodeId: string
  label?: string
  ownerId?: string
  connectionState: "online" | "offline" | "degraded"
  lastHeartbeat: number // timestamp
  health: string | null
  cpu: {
    cores: number
    utilization: number // 0-1
    source: ResourceSource
    timestamp: number
  }
  memory: {
    total: number // bytes
    used: number // bytes
    source: ResourceSource
    timestamp: number
  }
  disk: {
    total: number // bytes
    used: number // bytes
    source: ResourceSource
    timestamp: number
  }
  gpu?: {
    cores: number
    utilization: number
    source: ResourceSource
    timestamp: number
  }
  network: {
    rxBytes: number
    txBytes: number
    activeConnections: number
    source: ResourceSource
    timestamp: number
  }
  activeJobs: number
  activeApps: string[]
  runtimeVersion?: string
  schedulerRole?: "coordinator" | "worker" | "unknown"
  authoritySource: "node_stream" | "coordinator_claim" | "unknown"
}

export interface AggregateResource {
  totalNodes: number
  connectedNodes: number
  unhealthyNodes: number
  staleNodes: number
  totalCores: number
  weightedCpuUtilization: number // weighted by capacity
  totalMemory: number // bytes
  usedMemory: number // bytes
  totalDisk: number // bytes
  usedDisk: number // bytes
  activeJobs: number
  availableCapacity: number
  sources: ResourceSource[]
}

export interface ResourceState {
  nodes: Map<string, NodeResource>
  aggregate: AggregateResource | null
  lastUpdated: number
  primarySource: ResourceSource
}

const initialState: ResourceState = {
  nodes: new Map(),
  aggregate: null,
  lastUpdated: 0,
  primarySource: "unknown",
}

export const resourceStore = atom<ResourceState>(initialState)

export const allNodes = computed(resourceStore, (s) =>
  Array.from(s.nodes.values()),
)

export const aggregate = computed(resourceStore, (s) => s.aggregate)

export const connectedCount = computed(resourceStore, (s) =>
  Array.from(s.nodes.values()).filter(n => n.connectionState === "online").length,
)

export function updateNodeResource(nodeId: string, update: Partial<NodeResource>): void {
  const state = resourceStore.get()
  const existing = state.nodes.get(nodeId) || {
    nodeId,
    connectionState: "offline",
    lastHeartbeat: 0,
    health: null,
    cpu: { cores: 0, utilization: 0, source: "unknown", timestamp: 0 },
    memory: { total: 0, used: 0, source: "unknown", timestamp: 0 },
    disk: { total: 0, used: 0, source: "unknown", timestamp: 0 },
    network: { rxBytes: 0, txBytes: 0, activeConnections: 0, source: "unknown", timestamp: 0 },
    activeJobs: 0,
    activeApps: [],
    authoritySource: "unknown",
  }
  
  const updated = { ...existing, ...update }
  const newNodes = new Map(state.nodes)
  newNodes.set(nodeId, updated)
  
  // Recompute aggregate
  const nodesArray = Array.from(newNodes.values())
  const connected = nodesArray.filter(n => n.connectionState === "online")
  const unhealthy = nodesArray.filter(n => n.health === "unhealthy" || n.health === "failed")
  
  // Weighted CPU utilization: sum(cores * utilization) / sum(cores)
  let totalWeightedCpu = 0
  let totalCores = 0
  for (const n of nodesArray) {
    totalWeightedCpu += n.cpu.cores * n.cpu.utilization
    totalCores += n.cpu.cores
  }
  const weightedCpuUtilization = totalCores > 0 ? totalWeightedCpu / totalCores : 0
  
  const totalMemory = nodesArray.reduce((sum, n) => sum + n.memory.total, 0)
  const usedMemory = nodesArray.reduce((sum, n) => sum + n.memory.used, 0)
  const totalDisk = nodesArray.reduce((sum, n) => sum + n.disk.total, 0)
  const usedDisk = nodesArray.reduce((sum, n) => sum + n.disk.used, 0)
  const activeJobs = nodesArray.reduce((sum, n) => sum + n.activeJobs, 0)
  
  const allSources = new Set<ResourceSource>()
  for (const n of nodesArray) {
    allSources.add(n.cpu.source)
    allSources.add(n.memory.source)
  }
  
  const agg: AggregateResource = {
    totalNodes: nodesArray.length,
    connectedNodes: connected.length,
    unhealthyNodes: unhealthy.length,
    staleNodes: nodesArray.filter(n => Date.now() - n.lastHeartbeat > 60000).length,
    totalCores: totalCores,
    weightedCpuUtilization,
    totalMemory,
    usedMemory,
    totalDisk,
    usedDisk,
    activeJobs,
    availableCapacity: totalCores - nodesArray.filter(n => n.connectionState !== "online").length,
    sources: Array.from(allSources),
  }
  
  resourceStore.set({
    nodes: newNodes,
    aggregate: agg,
    lastUpdated: Date.now(),
    primarySource: state.primarySource,
  })
}

export function removeNode(nodeId: string): void {
  const state = resourceStore.get()
  const newNodes = new Map(state.nodes)
  newNodes.delete(nodeId)
  // Recompute aggregate...
  resourceStore.set({ ...state, nodes: newNodes })
}

export function setPrimarySource(source: ResourceSource): void {
  const state = resourceStore.get()
  resourceStore.set({ ...state, primarySource: source })
}

export function formatResourceSummary(): string {
  const agg = resourceStore.get().aggregate
  if (!agg) return "No resource data"
  
  const cpuPct = agg.totalCores > 0 
    ? Math.round(agg.weightedCpuUtilization * 100) 
    : 0
  
  const memGB = Math.round(agg.usedMemory / 1024 / 1024 / 1024 * 10) / 10
  const totalMemGB = Math.round(agg.totalMemory / 1024 / 1024 / 1024 * 10) / 10
  
  return `${agg.totalCores} cores · ${cpuPct}% utilization · ${memGB}/${totalMemGB} GB · ${agg.connectedNodes}/${agg.totalNodes} nodes online · ${agg.activeJobs} active jobs`
}
