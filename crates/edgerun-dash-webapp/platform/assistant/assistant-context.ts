import { atom, computed } from 'nanostores'
import { nodeStore } from '../state/node-store'
import { appStore } from '../state/app-store'
import { capabilityStore } from '../state/capability-store'
import { connectionStore } from '../state/connection-store'
import { permissionStore } from '../state/permission-store'
import { objectStore } from '../state/object-store'
import { commandStore } from '../state/command-store'
import { runtimeStore } from '../state/runtime-store'
import { approvalTrackerStore } from '../auth/approval-tracker'
import { permissionTrackerStore } from '../auth/permission-tracker'

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
  available: string[]
  granted: string[]
  pending: string[]
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
  status: string
  uptime: number | null
  memory: { used: number; total: number } | null
}

const CONTEXT_STALE_THRESHOLD = 30000

export const contextSnapshotStore = atom<ContextSnapshot | null>(null)
export const contextAge = atom<number>(0)

export const isContextStale = computed(contextAge, (age) => age > CONTEXT_STALE_THRESHOLD)

export function buildContext(): ContextSnapshot {
  const node = nodeStore.get()
  const apps = appStore.get()
  const caps = capabilityStore.get()
  const connections = connectionStore.get()
  const approvals = approvalTrackerStore.get()
  const runtime = runtimeStore.get()

  const nodeContext: NodeContext = {
    nodeId: node.currentNode?.nodeId || null,
    health: node.currentNode?.health || null,
    runtimeVersion: node.currentNode?.runtimeVersion || null,
    syncStatus: node.currentNode?.syncStatus || null,
    isConnected: node.isConnected,
  }

  const installedApps: AppInfo[] = Array.from(apps.apps.values())
    .slice(0, 20)
    .map((a) => ({
      appId: a.appId,
      name: a.name || a.appId,
      version: a.version || 'unknown',
    }))

  const runningApps = Array.from(apps.runningApps)

  const capabilitiesContext: CapabilitiesContext = {
    available: Array.from(caps.descriptors.values())
      .filter((d) => d.isAvailable)
      .map((d) => d.capabilityId),
    granted: caps.grantedCapabilities,
    pending: caps.pendingCapabilities,
    total: caps.descriptors.size,
  }

  const connectionsContext: ConnectionsContext = {
    active: connections.connections.filter((c) => c.isConnected).length,
    total: connections.connections.length,
  }

  const approvalsContext: ApprovalsContext = {
    pending: approvals.pending.map((a) => ({
      id: a.id,
      action: a.action,
      status: a.status,
      timestamp: a.createdAt,
    })),
    recent: approvals.recent.slice(0, 5).map((a) => ({
      id: a.id,
      action: a.action,
      status: a.status,
      timestamp: a.createdAt,
    })),
  }

  const runtimeContext: RuntimeContext = {
    status: runtime.status || 'unknown',
    uptime: runtime.uptime || null,
    memory: runtime.memory || null,
  }

  const textSummary = buildTextSummary(
    nodeContext,
    { installed: installedApps, running: runningApps, total: installedApps.length },
    capabilitiesContext,
    connectionsContext,
    approvalsContext,
    runtimeContext
  )

  const snapshot: ContextSnapshot = {
    id: `ctx_${Date.now()}`,
    timestamp: Date.now(),
    node: nodeContext,
    apps: { installed: installedApps, running: runningApps, total: installedApps.length },
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
  runtime: RuntimeContext
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
  lines.push(`Available: ${caps.available.length}, Granted: ${caps.granted.length}`)
  if (caps.available.length > 0) {
    lines.push(`List: ${caps.available.slice(0, 10).join(', ')}${caps.available.length > 10 ? '...' : ''}`)
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
  lines.push(`Status: ${runtime.status}`)
  if (runtime.uptime) {
    lines.push(`Uptime: ${Math.floor(runtime.uptime / 60)}m`)
  }
  if (runtime.memory) {
    const pct = Math.round((runtime.memory.used / runtime.memory.total) * 100)
    lines.push(`Memory: ${runtime.memory.used}MB / ${runtime.memory.total}MB (${pct}%)`)
  }

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
  return `Context: ${snapshot.node.nodeId || 'no node'} | Apps: ${snapshot.apps.running.length} running | Caps: ${snapshot.capabilities.available.length} | Pending: ${snapshot.approvals.pending.length} | Age: ${age}s`
}

export function initializeContext(): void {
  buildContext()

  setInterval(() => {
    refreshContextAge()
  }, 5000)
}
