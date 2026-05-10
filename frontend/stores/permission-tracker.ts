/**
 * Tracks user/session permission state.
 * Handles approvals, permission prompts, denied grants.
 */

import { atom, computed } from "nanostores"
import type {
  PermissionScope,
  PendingApproval,
  PermissionPrompt,
} from "@/platform/state/permission-store"
import { permissionStore } from "@/platform/state/permission-store"

export interface PermissionTrackerState {
  sessionId: string | null
  isAuthenticated: boolean
  pendingApprovals: Map<string, PendingApproval>
  permissionPrompts: Map<string, PermissionPrompt>
  deniedGrants: Map<string, { scope: PermissionScope; reason: string; at: string }>
  externalAuth: Map<string, { connected: boolean; expiresAt?: string }>
}

const initialState: PermissionTrackerState = {
  sessionId: null,
  isAuthenticated: false,
  pendingApprovals: new Map(),
  permissionPrompts: new Map(),
  deniedGrants: new Map(),
  externalAuth: new Map(),
}

export const permissionTracker = atom<PermissionTrackerState>(initialState)

export const hasPending = computed(permissionTracker, (s) =>
  s.pendingApprovals.size > 0,
)

export function hasPermission(scope: PermissionScope): boolean {
  const state = permissionTracker.get()
  if (!state.isAuthenticated && scope !== "app_launch") return false
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
    return `This operation (${operation}) requires approval.`
  }
  return `Operation (${operation}) can proceed.`
}

export function listPendingApprovals(): PendingApproval[] {
  return Array.from(permissionTracker.get().pendingApprovals.values())
}

export function addPendingApproval(approval: PendingApproval): void {
  const state = permissionTracker.get()
  const newApprovals = new Map(state.pendingApprovals)
  newApprovals.set(approval.approvalId, approval)
  permissionTracker.set({ ...state, pendingApprovals: newApprovals })
}

export function approvePending(approvalId: string): void {
  const state = permissionTracker.get()
  const newApprovals = new Map(state.pendingApprovals)
  newApprovals.delete(approvalId)
  permissionTracker.set({ ...state, pendingApprovals: newApprovals })
}

export function rejectPending(approvalId: string, reason: string): void {
  const state = permissionTracker.get()
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
    permissionTracker.set({
      ...state,
      pendingApprovals: newApprovals,
      deniedGrants: newDenied,
    })
  }
}

export function setExternalAuth(
  provider: string,
  connected: boolean,
  expiresAt?: string,
): void {
  const state = permissionTracker.get()
  const newAuth = new Map(state.externalAuth)
  newAuth.set(provider, { connected, expiresAt })
  permissionTracker.set({ ...state, externalAuth: newAuth })
}

// Sync from permission store
permissionStore.listen((state) => {
  permissionTracker.set({
    sessionId: state.currentSession,
    isAuthenticated: state.isAuthenticated,
    pendingApprovals: state.pendingApprovals,
    permissionPrompts: state.permissionPrompts,
    deniedGrants: state.deniedGrants,
    externalAuth: state.externalAuthStatus,
  })
})
