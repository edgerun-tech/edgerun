/**
 * Single source of truth for local node status.
 * Tracks node identity, health, stream head, runtime version, protocol versions, sync status.
 */

import { atom, computed } from "nanostores"
import { protocolClient } from "@/platform/protocol/client"

export interface NodeStatus {
  nodeId: string
  identity: string
  health: "healthy" | "degraded" | "offline"
  streamHead?: {
    streamId: string
    lastSeq: number
    lastEventId: string
  }
  runtimeVersion: string
  protocolVersions: string[]
  syncStatus: "synced" | "syncing" | "stale" | "error"
  lastRefresh: string
  error?: string
}

export interface ReachableNode {
  nodeId: string
  identity: string
  target: string
  runtime: "wasm" | "native" | string
  relationship: "self" | "controlled" | "reachable" | string
  controlled: boolean
  reachable: boolean
  health: "healthy" | "reachable" | "degraded" | "offline" | string
  username?: string
  registeredAtIso?: string
  installedAppCount?: number
}

export interface NodeStore {
  currentNode: NodeStatus | null
  nodes: ReachableNode[]
  isConnected: boolean
  isLoading: boolean
  error: string | null
}

const initialState: NodeStore = {
  currentNode: null,
  nodes: [],
  isConnected: false,
  isLoading: false,
  error: null,
}

export const nodeStore = atom<NodeStore>(initialState)

export const currentNodeStatus = computed(
  nodeStore,
  (s) => s.currentNode,
)

export const isNodeConnected = computed(
  nodeStore,
  (s) => s.isConnected,
)

export const isNodeLoading = computed(
  nodeStore,
  (s) => s.isLoading,
)

export const reachableNodes = computed(nodeStore, (s) => s.nodes)

export async function refreshNodeStatus(): Promise<void> {
  const state = nodeStore.get()
  nodeStore.set({ ...state, isLoading: true, error: null })

  try {
    const [response, nodesResponse] = await Promise.all([
      protocolClient.send({
        method: "GET",
        path: "/protocol/node/status",
      }),
      protocolClient.send({
        method: "GET",
        path: "/protocol/nodes",
        headers: { Accept: "application/json" },
      }),
    ])

    if (response.status === 200) {
      const text = new TextDecoder().decode(response.body)
      const status = JSON.parse(text) as NodeStatus
      const nodesText = new TextDecoder().decode(nodesResponse.body)
      const nodesPayload = nodesResponse.status === 200 ? JSON.parse(nodesText) as { nodes?: ReachableNode[] } : {}
      nodeStore.set({
        currentNode: status,
        nodes: nodesPayload.nodes ?? [],
        isConnected: true,
        isLoading: false,
        error: null,
      })
    } else {
      throw new Error(`Failed to fetch node status: ${response.status}`)
    }
  } catch (err) {
    nodeStore.set({
      ...nodeStore.get(),
      isConnected: false,
      isLoading: false,
      error: err instanceof Error ? err.message : "Unknown error",
    })
  }
}

export function getNodeId(): string | null {
  return nodeStore.get().currentNode?.nodeId ?? null
}

export function getNodeHealth(): "healthy" | "degraded" | "offline" {
  return nodeStore.get().currentNode?.health ?? "offline"
}

export function setNodeRegistration(nodeId: string, target: string) {
  protocolClient.setNodeRegistration({
    nodeId,
    nodeTarget: target,
    username: "",
    registeredAtIso: new Date().toISOString(),
  })
  void refreshNodeStatus()
}
