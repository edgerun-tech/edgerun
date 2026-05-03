/**
 * Tracks external connection state.
 * Monitors auth status, token expiration, available resources.
 */

import { atom, computed } from "nanostores"
import type { ExternalConnection } from "@/platform/state/connection-store"
import { connectionRegistry } from "@/platform/registries/connection-registry"

export interface ExternalConnectionTrackerState {
  connections: Map<string, ExternalConnection>
  expiredTokens: Set<string>
  availableResources: Map<string, string[]>
}

const initialState: ExternalConnectionTrackerState = {
  connections: new Map(),
  expiredTokens: new Set(),
  availableResources: new Map(),
}

export const externalConnectionTracker = atom<ExternalConnectionTrackerState>(initialState)

export const connectedCount = computed(
  externalConnectionTracker,
  (s) =>
    Array.from(s.connections.values()).filter((c) => c.status === "connected")
      .length,
)

export function getConnection(
  connectionId: string,
): ExternalConnection | undefined {
  return externalConnectionTracker.get().connections.get(connectionId)
}

export function isConnected(connectionId: string): boolean {
  return getConnection(connectionId)?.status === "connected"
}

export function hasExpiredToken(connectionId: string): boolean {
  return externalConnectionTracker.get().expiredTokens.has(connectionId)
}

export function listAvailableResources(
  connectionId: string,
): string[] {
  return (
    externalConnectionTracker.get().availableResources.get(connectionId) || []
  )
}

export function updateConnection(
  connectionId: string,
  update: Partial<ExternalConnection>,
): void {
  const state = externalConnectionTracker.get()
  const existing = state.connections.get(connectionId)
  if (existing) {
    const updated = { ...existing, ...update }
    const newConnections = new Map(state.connections)
    newConnections.set(connectionId, updated)

    // Check token expiration
    const newExpired = new Set(state.expiredTokens)
    if (updated.tokenExpiresAt) {
      if (new Date(updated.tokenExpiresAt) < new Date()) {
        newExpired.add(connectionId)
      } else {
        newExpired.delete(connectionId)
      }
    }

    externalConnectionTracker.set({
      connections: newConnections,
      expiredTokens: newExpired,
      availableResources: state.availableResources,
    })
  }
}

export function setAvailableResources(
  connectionId: string,
  resources: string[],
): void {
  const state = externalConnectionTracker.get()
  const newResources = new Map(state.availableResources)
  newResources.set(connectionId, resources)
  externalConnectionTracker.set({ ...state, availableResources: newResources })
}

// Sync from connection registry
connectionRegistry.listen((_state) => {
  const trackerState = externalConnectionTracker.get()
  const newConnections = new Map(trackerState.connections)

  // This would be populated from the registry
  // For now, just update the tracker state structure

  externalConnectionTracker.set({
    ...trackerState,
    connections: newConnections,
  })
})
