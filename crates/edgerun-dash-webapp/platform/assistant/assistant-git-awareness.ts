import { atom, computed } from 'nanostores'

export type GitStatus = 'clean' | 'dirty' | 'unknown'
export type OtherAgentStatus = 'detected' | 'not_detected' | 'unknown'

export interface GitInfo {
  status: GitStatus
  branch: string
  changes: string[]
  lastCommit: string | null
  lastChecked: number
}

export interface OtherAgentInfo {
  status: OtherAgentStatus
  detectedAgents: string[]
  lastChecked: number
}

export interface GitAwarenessState {
  git: GitInfo
  otherAgents: OtherAgentInfo
}

const DEFAULT_GIT_INFO: GitInfo = {
  status: 'unknown',
  branch: '',
  changes: [],
  lastCommit: null,
  lastChecked: 0,
}

const DEFAULT_OTHER_AGENT_INFO: OtherAgentInfo = {
  status: 'unknown',
  detectedAgents: [],
  lastChecked: 0,
}

export const gitAwarenessStore = atom<GitAwarenessState>({
  git: DEFAULT_GIT_INFO,
  otherAgents: DEFAULT_OTHER_AGENT_INFO,
})

export const gitStatus = computed(gitAwarenessStore, (state) => state.git.status)
export const otherAgentStatus = computed(gitAwarenessStore, (state) => state.otherAgents.status)

export function updateGitStatus(
  status: GitStatus,
  branch: string = '',
  changes: string[] = [],
  lastCommit: string | null = null
): void {
  const state = gitAwarenessStore.get()
  gitAwarenessStore.set({
    ...state,
    git: {
      status,
      branch,
      changes,
      lastCommit,
      lastChecked: Date.now(),
    },
  })
}

export function detectOtherAgents(agents: string[]): void {
  const state = gitAwarenessStore.get()
  gitAwarenessStore.set({
    ...state,
    otherAgents: {
      status: agents.length > 0 ? 'detected' : 'not_detected',
      detectedAgents: agents,
      lastChecked: Date.now(),
    },
  })
}

export function setOtherAgentUnknown(): void {
  const state = gitAwarenessStore.get()
  gitAwarenessStore.set({
    ...state,
    otherAgents: {
      ...state.otherAgents,
      status: 'unknown',
      lastChecked: Date.now(),
    },
  })
}

export function getGitSummary(): string {
  const { git } = gitAwarenessStore.get()
  if (git.status === 'unknown') return 'Git status unknown'
  if (git.status === 'clean') return `Clean - ${git.branch}`
  return `Dirty (${git.changes.length} changes) - ${git.branch}`
}

export function hasUnrelatedChanges(files: string[], focusFiles: string[]): boolean {
  const { git } = gitAwarenessStore.get()
  if (git.status !== 'dirty') return false

  const unrelatedFiles = git.changes.filter(
    (f) => !focusFiles.some((focus) => f.startsWith(focus))
  )
  return unrelatedFiles.length > 0
}

export function getChangedFilesForContext(focusPaths: string[]): string[] {
  const { git } = gitAwarenessStore.get()
  if (git.status !== 'dirty') return []

  return git.changes.filter((f) => focusPaths.some((p) => f.startsWith(p)))
}

export function getOtherAgentSummary(): string {
  const { otherAgents } = gitAwarenessStore.get()
  if (otherAgents.status === 'unknown') return 'Unknown'
  if (otherAgents.status === 'not_detected') return 'No other agents'
  return `Active: ${otherAgents.detectedAgents.join(', ')}`
}
