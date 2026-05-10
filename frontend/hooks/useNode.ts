/**
 * Hook for accessing node state and actions.
 */

import { useStore } from "@nanostores/react"
import {
  nodeStore,
  currentNodeStatus,
  isNodeConnected,
  isNodeLoading,
  refreshNodeStatus,
  getNodeId,
  getNodeHealth,
  setNodeRegistration,
} from "@/stores/node-store"

export function useNode() {
  const store = useStore(nodeStore)
  const node = useStore(currentNodeStatus)
  const connected = useStore(isNodeConnected)
  const loading = useStore(isNodeLoading)

  return {
    node,
    connected,
    loading,
    error: store.error,
    refresh: refreshNodeStatus,
    getNodeId,
    getNodeHealth,
    setNodeRegistration,
  }
}
