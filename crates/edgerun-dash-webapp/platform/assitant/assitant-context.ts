import { atom, computed } from 'nanostores'
import { nodeStore } from '../state/node-store'
import { appStore } from '../state/app-store'
import { capabilityStore } from '../state/capability-store'
import { connectionStore } from '../state/connection-store'
import type { ExternalConnection } from '../state/connection-store'
import { permissionStore } from '../state/permission-store'
import { objectStore } from '../state/object-store'
import { commandStore } from '../state/command-store'
import { runtimeStore } from '../state/runtime-store'
import { approvalTracker } from '../auth/approval-tracker'
import { permissionTracker } from '../auth/permission-tracker'

export interface ContextSnapshot {
  id: string
  timestamp: number
  node: NodeContext
  apps: AppsContext
  capabilities: CapabilitiesContext
  connections: ConnectionsContext
  approvals: ApprovalsContext
  runtime: RuntimeContext
  textSummary: string
}

export interface NodeContext {
  nodeId: string | null
  health: string | null
  runtimeVersion: string | null
  syncStatus: string | null
  isConnected: boolean
}

export interface AppsContext {
  installed: AppInfo[]
  running: string[]
  total: number
}

export interface AppInfo {
  appId: string
  name: string
  version: string
}

export interface CapabilitiesContext {
  descriptors: string[]
  total: number
}

export interface ConnectionsContext {
  active: number
  total: number
}

export interface ApprovalsContext {
  pending: ApprovalInfo[]
  recent: ApprovalInfo[]
}

export interface ApprovalInfo {
  id: string
  action: string
  status: string
  timestamp: number
}

export interface RuntimeContext {
  isRunning: boolean
  runningAppsCount: number
}

const CONTEXT_STALE_THRESHOLD = 30000

export const contextSnapshotStore = atom<ContextSnapshot | null>(null)
export const contextAge = atom<number>(0)

export const isContextStale = computed(contextAge, (age) => age > CONTEXT_STALE_THRESHOLD)

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
}

export function buildContext(): ContextSnapshot {
  const node = nodeStore.get()
  const apps = appStore.get()
  const caps = capabilityStore.get()
  const connections = connectionStore.get()
  const approvals = approvalTracker.get()
  const runtime = runtimeStore.get()

  const nodeContext: NodeContext = {
    nodeId: node.currentNode?.nodeId || null,
    health: node.currentNode?.health || null,
    runtimeVersion: node.currentNode?.runtimeVersion || null,
    syncStatus: node.currentNode?.syncStatus || null,
    isConnected: node.isConnected,
  }

  const installedApps: AppInfo[] = Array.from(apps.apps.entries())
    .slice(0, 20)
    .map(([appId, pkg]) => ({
      appId,
      name: pkg.name || appId,
      version: String(pkg.version),
    }))

  const runningAppIds = Array.from(runtime.runningApps.values())
    .filter((a) => a.isRunning)
    .map((a) => a.appId)

  const capabilitiesContext: CapabilitiesContext = {
    descriptors: Array.from(caps.descriptors.values()).map((d) => {
      if (d.capability_id && d.capability_id.length > 0) {
        return bytesToHex(d.capability_id)
      }
      return d.capability_kind?.toString() || 'unknown'
    }),
    total: caps.descriptors.size,
  }

  const connectionsContext: ConnectionsContext = {
    active: Array.from(connections.connections.values()).filter((c) => c.status === 'connected').length,
    total: connections.connections.size,
  }

  const pendingApprovals = Array.from(approvals.pending.values())
  const historyEntries = Array.from(approvals.history.entries())

  const approvalsContext: ApprovalsContext = {
    pending: pendingApprovals.map((a) => ({
      id: a.approvalId,
      action: a.action || 'unknown',
      status: 'pending',
      timestamp: new Date().getTime(),
    })),
    recent: historyEntries
      .slice(0, 5)
      .map(([, h]) => ({
        id: '',
        action: 'historical',
        status: h.approved ? 'approved' : 'rejected',
        timestamp: new Date(h.at).getTime(),
      })),
  }

  const runtimeContext: RuntimeContext = {
    isRunning: runtime.runningApps.size > 0,
    runningAppsCount: runtime.runningApps.size,
  }

  const textSummary = buildTextSummary(
    nodeContext,
    { installed: installedApps, running: runningAppIds, total: installedApps.length },
    capabilitiesContext,
    connectionsContext,
    approvalsContext,
    runtimeContext,
  )

  const snapshot: ContextSnapshot = {
    id: `ctx_${Date.now()}`,
    timestamp: Date.now(),
    node: nodeContext,
    apps: { installed: installedApps, running: runningAppIds, total: installedApps.length },
    capabilities: capabilitiesContext,
    connections: connectionsContext,
    approvals: approvalsContext,
    runtime: runtimeContext,
    textSummary,
  }

  contextSnapshotStore.set(snapshot)
  contextAge.set(0)

  return snapshot
}

function buildTextSummary(
  node: NodeContext,
  apps: AppsContext,
  caps: CapabilitiesContext,
  connections: ConnectionsContext,
  approvals: ApprovalsContext,
  runtime: RuntimeContext,
): string {
  const lines: string[] = []

  lines.push('## System Context')
  lines.push('')

  if (node.nodeId) {
    lines.push(`**Node**: ${node.nodeId}`)
    lines.push(`**Health**: ${node.health || 'unknown'}`)
    lines.push(`**Runtime**: ${node.runtimeVersion || 'unknown'}`)
    lines.push(`**Sync**: ${node.syncStatus || 'unknown'}`)
    lines.push(`**Connected**: ${node.isConnected ? 'Yes' : 'No'}`)
  } else {
    lines.push('**Node**: Not registered')
  }

  lines.push('')
  lines.push('## Apps')
  lines.push(`Installed: ${apps.total}, Running: ${apps.running.length}`)
  if (apps.running.length > 0) {
    lines.push(`Running: ${apps.running.join(', ')}`)
  }

  lines.push('')
  lines.push('## Capabilities')
  lines.push(`Available: ${caps.descriptors.length}`)
  if (caps.descriptors.length > 0) {
    lines.push(`List: ${caps.descriptors.slice(0, 10).join(', ')}${caps.descriptors.length > 10 ? '...' : ''}`)
  }

  lines.push('')
  lines.push('## Connections')
  lines.push(`Active: ${connections.active}/${connections.total}`)

  lines.push('')
  lines.push('## Approvals')
  lines.push(`Pending: ${approvals.pending.length}`)
  if (approvals.pending.length > 0) {
    lines.push(`Waiting: ${approvals.pending.map((a) => a.action).join(', ')}`)
  }

  lines.push('')
  lines.push('## Runtime')
  lines.push(`Status: ${runtime.isRunning ? 'Running' : 'Idle'}`)
  lines.push(`Apps: ${runtime.runningAppsCount}`)

  return lines.join('\n')
}

export function refreshContextAge(): void {
  const snapshot = contextSnapshotStore.get()
  if (snapshot) {
    contextAge.set(Date.now() - snapshot.timestamp)
  }
}

export function getContextSummary(): string {
  const snapshot = contextSnapshotStore.get()
  if (!snapshot) {
    return 'No context available'
  }

  const age = Math.floor((Date.now() - snapshot.timestamp) / 1000)
  return `Context: ${snapshot.node.nodeId || 'no node'} | Apps: ${snapshot.apps.running.length} running | Caps: ${snapshot.capabilities.descriptors.length} | Pending: ${snapshot.approvals.pending.length} | Age: ${age}s`
}

export function initializeContext(): void {
  buildContext()

  setInterval(() => {
    refreshContextAge()
  }, 5000)
}
