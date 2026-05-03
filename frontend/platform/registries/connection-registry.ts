/**
 * Single registry for external connections.
 * Tracks connection state and provides lookup/normalization.
 */

import { atom, computed } from "nanostores"
import type { ExternalConnection } from "@/platform/state/connection-store"

export interface ConnectionRegistryState {
  connections: Map<string, ExternalConnection>
  connectionByType: Map<string, ExternalConnection[]>
}

const initialState: ConnectionRegistryState = {
  connections: new Map(),
  connectionByType: new Map(),
}

export const connectionRegistry = atom<ConnectionRegistryState>(initialState)

export const allConnections = computed(connectionRegistry, (s) =>
  Array.from(s.connections.values()),
)

export function registerConnection(conn: ExternalConnection): void {
  const state = connectionRegistry.get()
  const newConnections = new Map(state.connections)
  newConnections.set(conn.connectionId, conn)

  const newByType = new Map(state.connectionByType)
  const typeList = newByType.get(conn.type) || []
  newByType.set(conn.type, [...typeList, conn])

  connectionRegistry.set({
    connections: newConnections,
    connectionByType: newByType,
  })
}

export function getConnection(
  connectionId: string,
): ExternalConnection | undefined {
  return connectionRegistry.get().connections.get(connectionId)
}

export function listConnectionsByType(
  type: string,
): ExternalConnection[] {
  return (
    connectionRegistry.get().connectionByType.get(type) || []
  )
}

export function isConnected(connectionId: string): boolean {
  return getConnection(connectionId)?.status === "connected"
}

export function hasExpiredToken(connectionId: string): boolean {
  const conn = getConnection(connectionId)
  if (!conn?.tokenExpiresAt) return false
  return new Date(conn.tokenExpiresAt) < new Date()
}

export function updateConnectionStatus(
  connectionId: string,
  status: ExternalConnection["status"],
): void {
  const state = connectionRegistry.get()
  const existing = state.connections.get(connectionId)
  if (existing) {
    const newConnections = new Map(state.connections)
    newConnections.set(connectionId, { ...existing, status })
    connectionRegistry.set({ ...state, connections: newConnections })
  }
}
