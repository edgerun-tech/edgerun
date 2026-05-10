/**
 * Pipeline progress model.
 * Real pipeline state with evidence-backed completion.
 */

import { atom, computed } from "nanostores"

export type StepStatus = 
  | "pending"
  | "ready"
  | "running"
  | "waiting_approval"
  | "waiting_external"
  | "blocked"
  | "succeeded"
  | "failed"
  | "skipped"
  | "cancelled"
  | "verifying"
  | "verified"

export interface PipelineStep {
  stepId: string
  name: string
  status: StepStatus
  startedAt?: number
  completedAt?: number
  duration?: number
  evidenceRefs: string[]
  logsArtifact?: string
  outputHash?: string
  dependencies: string[] // stepIds
}

export interface PipelineRun {
  runId: string
  trigger: string
  agentId?: string
  taskId?: string
  steps: PipelineStep[]
  status: "pending" | "running" | "blocked" | "completed" | "failed" | "cancelled"
  startedAt?: number
  completedAt?: number
  blockedReason?: string
  artifacts: string[] // evidence refs
  percentComplete: number // meaningful only if backed by steps
  currentStep?: string
}

export interface PipelineState {
  runs: Map<string, PipelineRun>
  activeCount: number
  blockedCount: number
}

const initialState: PipelineState = {
  runs: new Map(),
  activeCount: 0,
  blockedCount: 0,
}

export const pipelineStore = atom<PipelineState>(initialState)

export const allRuns = computed(pipelineStore, (s) =>
  Array.from(s.runs.values()),
)

export const activeRuns = computed(pipelineStore, (s) =>
  Array.from(s.runs.values()).filter(r => r.status === "running"),
)

export function registerPipelineRun(run: PipelineRun): void {
  const state = pipelineStore.get()
  const newRuns = new Map(state.runs)
  newRuns.set(run.runId, run)
  recomputeCounts(newRuns)
}

export function updateStep(
  runId: string,
  stepId: string,
  update: Partial<PipelineStep>,
): void {
  const state = pipelineStore.get()
  const run = state.runs.get(runId)
  if (!run) return
  const newSteps = run.steps.map(s =>
    s.stepId === stepId ? { ...s, ...update } : s
  )
  const updatedRun = { ...run, steps: newSteps, ...computeRunStatus(newSteps) }
  const newRuns = new Map(state.runs)
  newRuns.set(runId, updatedRun)
  recomputeCounts(newRuns)
}

function computeRunStatus(steps: PipelineStep[]): {
  status: PipelineRun["status"]
  percentComplete: number
  currentStep?: string
} {
  const total = steps.length
  const done = steps.filter(s => s.status === "succeeded" || s.status === "verified" || s.status === "skipped").length
  const running = steps.find(s => s.status === "running")
  const blocked = steps.find(s => s.status === "blocked")
  
  const percentComplete = total > 0 ? (done / total) * 100 : 0
  const currentStep = running?.stepId || blocked?.stepId
  
  let status: PipelineRun["status"] = "pending"
  if (blocked) status = "blocked"
  else if (running) status = "running"
  else if (done === total && total > 0) status = "completed"
  else if (steps.some(s => s.status === "failed")) status = "failed"
  
  return { status, percentComplete, currentStep }
}

function recomputeCounts(runs: Map<string, PipelineRun>): void {
  const arr = Array.from(runs.values())
  pipelineStore.set({
    runs,
    activeCount: arr.filter(r => r.status === "running").length,
    blockedCount: arr.filter(r => r.status === "blocked").length,
  })
}

export function getRun(runId: string): PipelineRun | undefined {
  return pipelineStore.get().runs.get(runId)
}
