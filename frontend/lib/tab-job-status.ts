export type JobTabPhase = 'running' | 'quorum' | 'finalized' | 'settled' | 'slashed' | 'error'

export type JobTabStatus = {
  phase: JobTabPhase
  progressPercent?: number
  workersActive?: number
  flashIfHidden?: boolean
}

const JOB_TAB_STATUS_EVENT = 'edgerun:job-tab-status'

export function publishJobTabStatus(status: JobTabStatus | null): void {
  if (typeof window === 'undefined') return
  window.dispatchEvent(new CustomEvent(JOB_TAB_STATUS_EVENT, { detail: status }))
}

export function clearJobTabStatus(): void {
  publishJobTabStatus(null)
}

export { JOB_TAB_STATUS_EVENT }
