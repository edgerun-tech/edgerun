/**
 * Dashboard mode boundary — updated for bootstrap coordination topology.
 *
 * Real mode: connected to a node (coordination or regular).
 * Demo mode: sample data with "Demo" badges.
 * Offline mode: unavailable state.
 *
 * Bootstrap topology note:
 * - All nodes may connect to operator coordination node during development.
 * - Coordination node is NOT the trust root, NOT the authority.
 * - Command authority = target node's stream commit only.
 * - Dashboard must distinguish: coordinator observation vs authoritative node stream.
 */

import { atom } from "nanostores"

export type DashboardMode = "real" | "demo" | "offline"

export interface DashboardModeState {
  mode: DashboardMode
  reason?: string
  // Bootstrap topology awareness
  connectedTo?: "coordinator" | "node" | "unknown"
  isBootstrapTopology?: boolean
}

const initialState: DashboardModeState = {
  mode: "offline",
  reason: "No node connection",
  isBootstrapTopology: true, // assume bootstrap during development
}

export const dashboardModeStore = atom<DashboardModeState>(initialState)

export function setDashboardMode(
  mode: DashboardMode,
  reason?: string,
  connectedTo?: "coordinator" | "node" | "unknown",
): void {
  dashboardModeStore.set({ ...dashboardModeStore.get(), mode, reason, connectedTo })
}

export function getDashboardMode(): DashboardMode {
  return dashboardModeStore.get().mode
}

export function getDashboardModeReason(): string | undefined {
  return dashboardModeStore.get().reason
}

export function getConnectedTo(): "coordinator" | "node" | "unknown" {
  return dashboardModeStore.get().connectedTo || "unknown"
}

export function isBootstrapTopology(): boolean {
  return dashboardModeStore.get().isBootstrapTopology ?? true
}

export function isRealMode(): boolean {
  return dashboardModeStore.get().mode === "real"
}

export function isDemoMode(): boolean {
  return dashboardModeStore.get().mode === "demo"
}

export function isOfflineMode(): boolean {
  return dashboardModeStore.get().mode === "offline"
}

// Auto-detect: if node-store reports a connected node, switch to real
if (typeof window !== "undefined") {
  import("@/stores/node-store").then((mod) => {
    import("@/stores/connection-store").then((connMod) => {
      mod.nodeStore.listen((state) => {
        const current = dashboardModeStore.get()
        const connections = Array.from(connMod.connectionStore.get().connections.values())
        const hasCoordinationNode = connections.some(
          (c) => c.type === "local_node" && c.status === "connected"
        )
        const connectedTo = hasCoordinationNode ? "coordinator" : "node"

        if (state.currentNode && state.currentNode.health === "healthy") {
          if (current.mode === "demo" || current.mode === "offline") {
            setDashboardMode("real", undefined, connectedTo)
          } else if (current.connectedTo !== connectedTo) {
            dashboardModeStore.set({ ...current, connectedTo })
          }
        } else if (current.mode === "real") {
          setDashboardMode("offline", "Node unavailable", undefined)
        }
      })
    })
  })
}
