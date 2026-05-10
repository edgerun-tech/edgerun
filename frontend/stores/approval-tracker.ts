/**
 * Tracks pending approvals and approval flow.
 */

import { atom, computed } from "nanostores"
import type { PendingApproval } from "@/stores/permission-store"
import { permissionTracker } from "@/stores/permission-tracker"

export interface ApprovalTrackerState {
  pending: Map<string, PendingApproval>
  history: Map<string, { approved: boolean; at: string; reason?: string }>
}

const initialState: ApprovalTrackerState = {
  pending: new Map(),
  history: new Map(),
}

export const approvalTracker = atom<ApprovalTrackerState>(initialState)

export const pendingCount = computed(approvalTracker, (s) => s.pending.size)

export function addApproval(approval: PendingApproval): void {
  const state = approvalTracker.get()
  const newPending = new Map(state.pending)
  newPending.set(approval.approvalId, approval)
  approvalTracker.set({ ...state, pending: newPending })
}

export function approve(approvalId: string): void {
  const state = approvalTracker.get()
  const approval = state.pending.get(approvalId)
  if (approval) {
    const newPending = new Map(state.pending)
    newPending.delete(approvalId)
    const newHistory = new Map(state.history)
    newHistory.set(approvalId, {
      approved: true,
      at: new Date().toISOString(),
    })
    approvalTracker.set({
      pending: newPending,
      history: newHistory,
    })
    // Also update permission tracker
    permissionTracker.set({
      ...permissionTracker.get(),
      pendingApprovals: newPending,
    })
  }
}

export function reject(approvalId: string, reason: string): void {
  const state = approvalTracker.get()
  const approval = state.pending.get(approvalId)
  if (approval) {
    const newPending = new Map(state.pending)
    newPending.delete(approvalId)
    const newHistory = new Map(state.history)
    newHistory.set(approvalId, {
      approved: false,
      at: new Date().toISOString(),
      reason,
    })
    approvalTracker.set({
      pending: newPending,
      history: newHistory,
    })
    // Also update permission tracker
    permissionTracker.set({
      ...permissionTracker.get(),
      pendingApprovals: newPending,
    })
  }
}

export function listPending(): PendingApproval[] {
  return Array.from(approvalTracker.get().pending.values())
}

export function listHistory(): Array<{
  approvalId: string
  approved: boolean
  at: string
  reason?: string
}> {
  return Array.from(approvalTracker.get().history.entries()).map(
    ([id, val]) => ({ approvalId: id, ...val }),
  )
}

// Sync from permission tracker
permissionTracker.listen((state) => {
  const trackerState = approvalTracker.get()
  const newPending = new Map(trackerState.pending)

  for (const [id, approval] of state.pendingApprovals) {
    newPending.set(id, approval)
  }

  approvalTracker.set({
    ...trackerState,
    pending: newPending,
  })
})
