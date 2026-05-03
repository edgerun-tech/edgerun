/**
 * Platform assistant context.
 * Aggregates platform state for AI assistant access.
 */

import { atom, computed } from "nanostores"
import { appStore } from "@/platform/state/app-store"
import { capabilityStore } from "@/platform/state/capability-store"
import { nodeStore } from "@/platform/state/node-store"
import { connectionStore } from "@/platform/state/connection-store"
import { permissionTracker } from "@/platform/auth/permission-tracker"
import { approvalTracker } from "@/platform/auth/approval-tracker"

export interface AppInfo {
  appId: string
  name: string
  version: string
}

export interface CapabilityInfo {
  id: string
  name: string
  riskLevel: string
  isAvailable: boolean
}

export interface AssistantContextState {
  nodeCount: number
  activeSessions: number
  installedApps: AppInfo[]
  availableCapabilities: CapabilityInfo[]
  grantedCapabilities: string[]
  pendingCapabilities: string[]
  connections: string[]
  activePermissions: string[]
  pendingApprovals: number
  ramUsage: { used: number; total: number }
  isConnected: boolean
}

const initialState: AssistantContextState = {
  nodeCount: 0,
  activeSessions: 0,
  installedApps: [],
  availableCapabilities: [],
  grantedCapabilities: [],
  pendingCapabilities: [],
  connections: [],
  activePermissions: [],
  pendingApprovals: 0,
  ramUsage: { used: 0, total: 0 },
  isConnected: false,
}

export const assistantContext = atom<AssistantContextState>(initialState)

export function updateAssistantContext(): void {
  const appState = appStore.get()
  const capState = capabilityStore.get()
  const nodeState = nodeStore.get()
  const connState = connectionStore.get()

  const installedApps: AppInfo[] = Array.from(appState.apps.values()).map((app) => ({
    appId: app.name,
    name: app.name,
    version: app.version?.toString() || "",
  }))

  const availableCapabilities: CapabilityInfo[] = Array.from(capState.descriptors.values()).map((cap) => ({
    id: Buffer.from(cap.capability_id).toString("hex"),
    name: cap.provider_name,
    riskLevel: "unknown",
    isAvailable: true,
  }))

  const connections = Array.from(connState.connections.entries()).map(([k]) => k)

  const context: AssistantContextState = {
    nodeCount: 1,
    activeSessions: 0,
    installedApps,
    availableCapabilities,
    grantedCapabilities: [],
    pendingCapabilities: [],
    connections,
    activePermissions: [],
    pendingApprovals: 0,
    ramUsage: { used: 0, total: 0 },
    isConnected: nodeState?.isConnected ?? false,
  }

  assistantContext.set(context)
}
