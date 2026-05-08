/**
 * Hook for accessing permission state and actions.
 */

import { useStore } from "@nanostores/react"
import {
  permissionStore,
  hasPermission,
  requiresApproval,
  explainPermission,
  listPendingApprovals,
  addPendingApproval,
  approvePending,
  rejectPending,
  setExternalAuthStatus,
  isExternalAuthConnected,
} from "@/platform/state/permission-store"
import {
  permissionTracker,
  hasPending,
  addPendingApproval as trackerAddPending,
  approvePending as trackerApprove,
  rejectPending as trackerReject,
} from "@/platform/auth/permission-tracker"
import {
  approvalTracker,
  addApproval,
  approve,
  reject,
  listPending,
  listHistory,
} from "@/platform/auth/approval-tracker"

export function usePermissions() {
  const store = useStore(permissionStore)
  const tracker = useStore(permissionTracker)
  const approvals = useStore(approvalTracker)

  return {
    // Permission store
    isAuthenticated: store.isAuthenticated,
    currentSession: store.currentSession,
    hasPermission,
    requiresApproval,
    explainPermission,
    listPendingApprovals,
    addPendingApproval,
    approvePending,
    rejectPending,
    setExternalAuthStatus,
    isExternalAuthConnected,
    // Permission tracker
    hasPending: tracker.pendingApprovals.size > 0,
    tracker: {
      hasPermission,
      addPendingApproval: trackerAddPending,
      approvePending: trackerApprove,
      rejectPending: trackerReject,
    },
    // Approval tracker
    approvals: {
      pendingCount: approvals.pending.size,
      addApproval,
      approve,
      reject,
      listPending,
      listHistory,
    },
  }
}
