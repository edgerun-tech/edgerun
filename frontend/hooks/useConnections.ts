import { useStore } from "@nanostores/react"
import {
  connectionStore,
  allConnections,
  localNodeConnection,
  githubConnections,
  getConnection,
  listConnectionsByType,
  isConnected,
  hasExpiredToken,
  listAvailableResources,
  addConnection,
  removeConnection,
  updateConnection,
  updateConnectionStatus,
  setAvailableResources,
  refreshConnections,
} from "@/stores/connection-store"

export function useConnections() {
  const store = useStore(connectionStore)
  const connections = useStore(allConnections)
  const localNode = useStore(localNodeConnection)
  const github = useStore(githubConnections)

  return {
    connections,
    localNode,
    githubConnections: github,
    isLoading: store.isLoading,
    error: store.error,
    getConnection,
    listConnectionsByType,
    isConnected,
    hasExpiredToken,
    listAvailableResources,
    addConnection,
    removeConnection,
    updateConnection,
    updateConnectionStatus,
    setAvailableResources,
    refresh: refreshConnections,
  }
}
