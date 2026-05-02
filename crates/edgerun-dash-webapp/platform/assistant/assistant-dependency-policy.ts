import { atom, computed } from 'nanostores'

export interface DependencyRequest {
  name: string
  version: string
  reason: string
  existingAlternatives: string[]
  functionality: string
  securityConsiderations: string
  licenseConsiderations: string
  dependencyTreeImpact: string
  status: 'pending' | 'approved' | 'rejected'
}

export interface DependencyPolicyState {
  pendingRequests: DependencyRequest[]
  approvedDeps: string[]
  rejectedDeps: string[]
  checkEnabled: boolean
}

export type PolicyStatus = 'no_new_deps' | 'approval_required' | 'pending_review'

const DEPENDENCY_POLICY_KEY = 'edgerun-dependency-policy'

export const dependencyPolicyStore = atom<DependencyPolicyState>({
  pendingRequests: [],
  approvedDeps: [],
  rejectedDeps: [],
  checkEnabled: true,
})

export const policyStatus = computed(dependencyPolicyStore, (state) => {
  if (!state.checkEnabled) return 'no_new_deps'
  if (state.pendingRequests.length > 0) return 'pending_review'
  return 'approval_required'
})

export function createDependencyRequest(request: Omit<DependencyRequest, 'status'>): void {
  const state = dependencyPolicyStore.get()
  dependencyPolicyStore.set({
    ...state,
    pendingRequests: [
      ...state.pendingRequests,
      { ...request, status: 'pending' },
    ],
  })
}

export function approveDependency(name: string): void {
  const state = dependencyPolicyStore.get()
  const updated = state.pendingRequests.map((r) =>
    r.name === name ? { ...r, status: 'approved' as const } : r
  )
  const approved = updated.find((r) => r.name === name && r.status === 'approved')
  dependencyPolicyStore.set({
    pendingRequests: updated,
    approvedDeps: approved ? [...state.approvedDeps, name] : state.approvedDeps,
  })
}

export function rejectDependency(name: string): void {
  const state = dependencyPolicyStore.get()
  const updated = state.pendingRequests.map((r) =>
    r.name === name ? { ...r, status: 'rejected' as const } : r
  )
  const rejected = updated.find((r) => r.name === name && r.status === 'rejected')
  dependencyPolicyStore.set({
    pendingRequests: updated,
    rejectedDeps: rejected ? [...state.rejectedDeps, name] : state.rejectedDeps,
  })
}

export function isDependencyApproved(name: string): boolean {
  const state = dependencyPolicyStore.get()
  return state.approvedDeps.includes(name)
}

export function wasDependencyRejected(name: string): boolean {
  const state = dependencyPolicyStore.get()
  return state.rejectedDeps.includes(name)
}

export function togglePolicyCheck(enabled: boolean): void {
  const state = dependencyPolicyStore.get()
  dependencyPolicyStore.set({
    ...state,
    checkEnabled: enabled,
  })
}

export function clearPolicyState(): void {
  dependencyPolicyStore.set({
    pendingRequests: [],
    approvedDeps: [],
    rejectedDeps: [],
    checkEnabled: true,
  })
}

export function loadPolicyState(): void {
  try {
    const saved = localStorage.getItem(DEPENDENCY_POLICY_KEY)
    if (saved) {
      const parsed = JSON.parse(saved) as DependencyPolicyState
      dependencyPolicyStore.set(parsed)
    }
  } catch {
    clearPolicyState()
  }
}

export function savePolicyState(): void {
  try {
    const state = dependencyPolicyStore.get()
    localStorage.setItem(DEPENDENCY_POLICY_KEY, JSON.stringify(state))
  } catch {
    // Storage unavailable
  }
}

export function checkExternalDependency(
  name: string,
  version: string,
  reason: string,
  alternatives: string[] = []
): boolean {
  if (!dependencyPolicyStore.get().checkEnabled) {
    return true
  }

  if (isDependencyApproved(name)) {
    return true
  }

  if (wasDependencyRejected(name)) {
    return false
  }

  return false
}

export function requestExternalDependency(
  name: string,
  version: string,
  reason: string,
  functionality: string,
  alternatives: string[] = []
): void {
  createDependencyRequest({
    name,
    version,
    reason,
    functionality,
    existingAlternatives: alternatives,
    securityConsiderations: 'Not evaluated',
    licenseConsiderations: 'Not evaluated',
    dependencyTreeImpact: 'Unknown',
  })
}
