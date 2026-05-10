import { atom, computed } from "nanostores"
import { patchStore } from "@/platform/utils/store"

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
  expiredTokens: Set<string>
  availableResources: Map<string, string[]>
  isLoading: boolean
  error: string | null
  lastRefresh: string | null
}

const initialState: ConnectionStoreState = {
  connections: new Map(),
  expiredTokens: new Set(),
  availableResources: new Map(),
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

export const connectedCount = computed(connectionStore, (s) =>
  Array.from(s.connections.values()).filter((c) => c.status === "connected").length,
)

export const connectionByType = computed(connectionStore, (s) => {
  const byType = new Map<string, ExternalConnection[]>()
  for (const conn of s.connections.values()) {
    const list = byType.get(conn.type) || []
    list.push(conn)
    byType.set(conn.type, list)
  }
  return byType
})

export function getConnection(connectionId: string): ExternalConnection | undefined {
  return connectionStore.get().connections.get(connectionId)
}

export function listConnectionsByType(type: ConnectionType): ExternalConnection[] {
  return connectionByType.get().get(type) || []
}

export function isConnected(connectionId: string): boolean {
  return getConnection(connectionId)?.status === "connected"
}

export function hasExpiredToken(connectionId: string): boolean {
  const conn = getConnection(connectionId)
  if (!conn?.tokenExpiresAt) return false
  return new Date(conn.tokenExpiresAt) < new Date()
}

export function listAvailableResources(connectionId: string): string[] {
  return connectionStore.get().availableResources.get(connectionId) || []
}

function updateExpiredTokenSet(connection: ExternalConnection, expired: Set<string>): void {
  if (connection.tokenExpiresAt) {
    if (new Date(connection.tokenExpiresAt) < new Date()) {
      expired.add(connection.connectionId)
    } else {
      expired.delete(connection.connectionId)
    }
  }
}

export function updateConnection(
  connectionId: string,
  update: Partial<ExternalConnection>,
): void {
  const state = connectionStore.get()
  const existing = state.connections.get(connectionId)
  if (!existing) return
  const updated = { ...existing, ...update }
  const newConnections = new Map(state.connections)
  newConnections.set(connectionId, updated)
  const newExpired = new Set(state.expiredTokens)
  updateExpiredTokenSet(updated, newExpired)
  connectionStore.set({ ...state, connections: newConnections, expiredTokens: newExpired })
}

export function updateConnectionStatus(
  connectionId: string,
  status: ExternalConnection["status"],
): void {
  updateConnection(connectionId, { status })
}

export function addConnection(connection: ExternalConnection): void {
  const state = connectionStore.get()
  const newConnections = new Map(state.connections)
  newConnections.set(connection.connectionId, connection)
  const newExpired = new Set(state.expiredTokens)
  updateExpiredTokenSet(connection, newExpired)
  connectionStore.set({ ...state, connections: newConnections, expiredTokens: newExpired })
}

export function removeConnection(connectionId: string): void {
  const state = connectionStore.get()
  const newConnections = new Map(state.connections)
  newConnections.delete(connectionId)
  const newExpired = new Set(state.expiredTokens)
  newExpired.delete(connectionId)
  connectionStore.set({ ...state, connections: newConnections, expiredTokens: newExpired })
}

export function setAvailableResources(connectionId: string, resources: string[]): void {
  const state = connectionStore.get()
  const newResources = new Map(state.availableResources)
  newResources.set(connectionId, resources)
  connectionStore.set({ ...state, availableResources: newResources })
}

export async function refreshConnections(): Promise<void> {
  patchStore(connectionStore, { isLoading: true, error: null })
  try {
    patchStore(connectionStore, { isLoading: false, lastRefresh: new Date().toISOString() })
  } catch (err) {
    patchStore(connectionStore, {
      isLoading: false,
      error: err instanceof Error ? err.message : "Failed to refresh connections",
    })
  }
}
