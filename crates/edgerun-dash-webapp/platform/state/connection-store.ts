/**
 * Single source of truth for external integrations.
 * Tracks GitHub, cloud providers, local node, OAuth/OIDC status, tokens.
 */

import { atom, computed } from "nanostores"

export type ConnectionType =
  | "local_node"
  | "github"
  | "cloud_provider"
  | "oauth_oidc"
  | "browser_agent"
  | "external_service"

export type ConnectionStatus =
  | "connected"
  | "disconnected"
  | "expired"
  | "error"
  | "pending"

export interface ExternalConnection {
  connectionId: string
  type: ConnectionType
  name: string
  status: ConnectionStatus
  authMethod?: "oauth" | "token" | "certificate" | "none"
  tokenExpiresAt?: string
  lastAuthError?: string
  availableResources: string[]
  metadata: Record<string, string>
  connectedAt?: string
}

export interface ConnectionStoreState {
  connections: Map<string, ExternalConnection>
  isLoading: boolean
  error: string | null
  lastRefresh: string | null
}

const initialState: ConnectionStoreState = {
  connections: new Map(),
  isLoading: false,
  error: null,
  lastRefresh: null,
}

export const connectionStore = atom<ConnectionStoreState>(initialState)

export const allConnections = computed(connectionStore, (s) =>
  Array.from(s.connections.values()),
)

export const localNodeConnection = computed(connectionStore, (s) =>
  Array.from(s.connections.values()).find((c) => c.type === "local_node"),
)

export const githubConnections = computed(connectionStore, (s) =>
  Array.from(s.connections.values()).filter((c) => c.type === "github"),
)

export function getConnection(connectionId: string): ExternalConnection | undefined {
  return connectionStore.get().connections.get(connectionId)
}

export function listConnectionsByType(type: ConnectionType): ExternalConnection[] {
  return Array.from(connectionStore.get().connections.values()).filter(
    (c) => c.type === type,
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

export function updateConnection(
  connectionId: string,
  update: Partial<ExternalConnection>,
): void {
  const state = connectionStore.get()
  const existing = state.connections.get(connectionId)
  if (existing) {
    const newConnections = new Map(state.connections)
    newConnections.set(connectionId, { ...existing, ...update })
    connectionStore.set({ ...state, connections: newConnections })
  }
}

export function addConnection(connection: ExternalConnection): void {
  const state = connectionStore.get()
  const newConnections = new Map(state.connections)
  newConnections.set(connection.connectionId, connection)
  connectionStore.set({ ...state, connections: newConnections })
}

export function removeConnection(connectionId: string): void {
  const state = connectionStore.get()
  const newConnections = new Map(state.connections)
  newConnections.delete(connectionId)
  connectionStore.set({ ...state, connections: newConnections })
}

export async function refreshConnections(): Promise<void> {
  const state = connectionStore.get()
  connectionStore.set({ ...state, isLoading: true, error: null })

  try {
    // This would call the actual API
    connectionStore.set({
      ...connectionStore.get(),
      isLoading: false,
      lastRefresh: new Date().toISOString(),
    })
  } catch (err) {
    connectionStore.set({
      ...connectionStore.get(),
      isLoading: false,
      error:
        err instanceof Error ? err.message : "Failed to refresh connections",
    })
  }
}
