/**
 * Hook for accessing connection state and actions.
 */

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
  addConnection,
  removeConnection,
  refreshConnections,
} from "@/platform/state/connection-store"
import {
  connectionRegistry,
  allConnections as registryAllConnections,
  getConnection as registryGetConnection,
  listConnectionsByType as registryListByType,
  isConnected as registryIsConnected,
  updateConnectionStatus,
} from "@/platform/registries/connection-registry"

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
    addConnection,
    removeConnection,
    refresh: refreshConnections,
    registry: {
      allConnections: registryAllConnections,
      getConnection: registryGetConnection,
      listConnectionsByType: registryListByType,
      isConnected: registryIsConnected,
      updateConnectionStatus,
    },
  }
}
