/**
 * Tracks user/session permission state.
 * Handles approvals, external auth status, permission prompts.
 */

import { atom, computed } from "nanostores"

export type PermissionScope =
  | "identity"
  | "node_connection"
  | "network_access"
  | "filesystem"
  | "hardware_signing"
  | "payments"
  | "voice_call"
  | "app_launch"
  | "app_install"
  | "capability_grant"

export interface PendingApproval {
  approvalId: string
  operation: string
  scope: PermissionScope
  description: string
  requestedAt: string
  expiresAt?: string
  riskLevel: "low" | "medium" | "high" | "critical"
}

export interface PermissionPrompt {
  promptId: string
  title: string
  message: string
  scopes: PermissionScope[]
  action: string
  details?: Record<string, string>
}

export interface PermissionStoreState {
  currentSession: string | null
  isAuthenticated: boolean
  pendingApprovals: Map<string, PendingApproval>
  permissionPrompts: Map<string, PermissionPrompt>
  deniedGrants: Map<string, { scope: PermissionScope; reason: string; at: string }>
  externalAuthStatus: Map<string, { connected: boolean; expiresAt?: string }>
  isLoading: boolean
  error: string | null
}

const initialState: PermissionStoreState = {
  currentSession: null,
  isAuthenticated: false,
  pendingApprovals: new Map(),
  permissionPrompts: new Map(),
  deniedGrants: new Map(),
  externalAuthStatus: new Map(),
  isLoading: false,
  error: null,
}

export const permissionStore = atom<PermissionStoreState>(initialState)

export const hasPendingApprovals = computed(permissionStore, (s) =>
  s.pendingApprovals.size > 0,
)

export const pendingApprovalsList = computed(permissionStore, (s) =>
  Array.from(s.pendingApprovals.values()),
)

export function hasPermission(scope: PermissionScope): boolean {
  const state = permissionStore.get()
  if (!state.isAuthenticated && scope !== "app_launch") return false
  // Check if there are any denied grants for this scope
  const denied = Array.from(state.deniedGrants.values()).some(
    (d) => d.scope === scope,
  )
  return !denied
}

export function requiresApproval(operation: string): boolean {
  const highRiskOps = [
    "capability_grant",
    "app_install",
    "hardware_signing",
    "payments",
  ]
  return highRiskOps.some((op) => operation.includes(op))
}

export function explainPermission(operation: string): string {
  if (requiresApproval(operation)) {
    return `This operation (${operation}) requires approval due to security restrictions.`
  }
  return `Operation (${operation}) can proceed.`
}

export function listPendingApprovals(): PendingApproval[] {
  return Array.from(permissionStore.get().pendingApprovals.values())
}

export function addPendingApproval(approval: PendingApproval): void {
  const state = permissionStore.get()
  const newApprovals = new Map(state.pendingApprovals)
  newApprovals.set(approval.approvalId, approval)
  permissionStore.set({ ...state, pendingApprovals: newApprovals })
}

export function approvePending(approvalId: string): void {
  const state = permissionStore.get()
  const newApprovals = new Map(state.pendingApprovals)
  newApprovals.delete(approvalId)
  permissionStore.set({ ...state, pendingApprovals: newApprovals })
}

export function rejectPending(approvalId: string, reason: string): void {
  const state = permissionStore.get()
  const approval = state.pendingApprovals.get(approvalId)
  if (approval) {
    const newDenied = new Map(state.deniedGrants)
    newDenied.set(approvalId, {
      scope: approval.scope,
      reason,
      at: new Date().toISOString(),
    })
    const newApprovals = new Map(state.pendingApprovals)
    newApprovals.delete(approvalId)
    permissionStore.set({
      ...state,
      pendingApprovals: newApprovals,
      deniedGrants: newDenied,
    })
  }
}

export function setExternalAuthStatus(
  provider: string,
  connected: boolean,
  expiresAt?: string,
): void {
  const state = permissionStore.get()
  const newStatus = new Map(state.externalAuthStatus)
  newStatus.set(provider, { connected, expiresAt })
  permissionStore.set({ ...state, externalAuthStatus: newStatus })
}

export function isExternalAuthConnected(provider: string): boolean {
  return (
    permissionStore.get().externalAuthStatus.get(provider)?.connected ?? false
  )
}
