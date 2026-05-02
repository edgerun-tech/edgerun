/**
 * DEMO-ONLY workflow store.
 *
 * This store is NOT connected to real platform state.
 * For real workflow support, use platform/state/command-store.ts + protocol.
 *
 * Marked as demo because:
 * - Uses localStorage, not node protocol
 * - Uses eval() for condition actions (unsafe)
 * - Has fake execution with setTimeout
 * - Directly mutates workflows without approval flow
 *
 * TODO: Replace with platform tool registry entries for workflow actions.
 */

import { atom, computed } from "nanostores"

export type TriggerType = "manual" | "schedule" | "event" | "webhook"
export type ActionType = "http" | "deploy" | "notify" | "transform" | "condition" | "delay" | "log"
export type StageStatus = "idle" | "running" | "success" | "failed" | "skipped"

export interface WorkflowAction {
  actionId: string
  type: ActionType
  name: string
  config: Record<string, unknown>
  status: StageStatus
}

export interface WorkflowStage {
  stageId: string
  name: string
  actions: WorkflowAction[]
  status: StageStatus
}

export interface Workflow {
  workflowId: string
  name: string
  description: string
  trigger: TriggerType
  stages: WorkflowStage[]
  createdAt: number
  updatedAt: number
  isEnabled: boolean
}

export interface WorkflowExecution {
  executionId: string
  workflowId: string
  status: "running" | "success" | "failed" | "cancelled"
  startedAt: number
  completedAt?: number
  stageResults: Array<{ stageId: string; status: StageStatus }>
}

export interface WorkflowStoreState {
  workflows: Map<string, Workflow>
  executions: Map<string, WorkflowExecution>
  isLoading: boolean
  error: string | null
}

const initialState: WorkflowStoreState = {
  workflows: new Map(),
  executions: new Map(),
  isLoading: false,
  error: null,
}

export const workflowStore = atom<WorkflowStoreState>(initialState)

export const allWorkflows = computed(workflowStore, (s) =>
  Array.from(s.workflows.values()),
)

export function createWorkflow(name: string, description: string, trigger: TriggerType): string {
  const id = `wf-${Date.now()}-${Math.random().toString(36).slice(2)}`
  const state = workflowStore.get()
  const workflows = new Map(state.workflows)
  workflows.set(id, {
    workflowId: id,
    name,
    description,
    trigger,
    stages: [],
    createdAt: Date.now(),
    updatedAt: Date.now(),
    isEnabled: true,
  })
  workflowStore.set({ ...state, workflows })
  return id
}

export function updateWorkflow(id: string, update: Partial<Workflow>): void {
  const state = workflowStore.get()
  const existing = state.workflows.get(id)
  if (existing) {
    const workflows = new Map(state.workflows)
    workflows.set(id, { ...existing, ...update, updatedAt: Date.now() })
    workflowStore.set({ ...state, workflows })
  }
}

export function deleteWorkflow(id: string): void {
  const state = workflowStore.get()
  const workflows = new Map(state.workflows)
  workflows.delete(id)
  workflowStore.set({ ...state, workflows })
}

export function addStage(workflowId: string, name: string): void {
  const state = workflowStore.get()
  const workflow = state.workflows.get(workflowId)
  if (workflow) {
    const stage: WorkflowStage = {
      stageId: `stage-${Date.now()}`,
      name,
      actions: [],
      status: "idle",
    }
    const workflows = new Map(state.workflows)
    workflows.set(workflowId, {
      ...workflow,
      stages: [...workflow.stages, stage],
      updatedAt: Date.now(),
    })
    workflowStore.set({ ...state, workflows })
  }
}

export function addAction(workflowId: string, stageId: string, action: Omit<WorkflowAction, "actionId" | "status">): void {
  const state = workflowStore.get()
  const workflow = state.workflows.get(workflowId)
  if (workflow) {
    const newAction: WorkflowAction = {
      ...action,
      actionId: `action-${Date.now()}`,
      status: "idle",
    }
    const workflows = new Map(state.workflows)
    workflows.set(workflowId, {
      ...workflow,
      stages: workflow.stages.map((s) =>
        s.stageId === stageId ? { ...s, actions: [...s.actions, newAction] } : s
      ),
      updatedAt: Date.now(),
    })
    workflowStore.set({ ...state, workflows })
  }
}

export function removeAction(workflowId: string, stageId: string, actionId: string): void {
  const state = workflowStore.get()
  const workflow = state.workflows.get(workflowId)
  if (workflow) {
    const workflows = new Map(state.workflows)
    workflows.set(workflowId, {
      ...workflow,
      stages: workflow.stages.map((s) =>
        s.stageId === stageId
          ? { ...s, actions: s.actions.filter((a) => a.actionId !== actionId) }
          : s
      ),
      updatedAt: Date.now(),
    })
    workflowStore.set({ ...state, workflows })
  }
}

export function removeStage(workflowId: string, stageId: string): void {
  const state = workflowStore.get()
  const workflow = state.workflows.get(workflowId)
  if (workflow) {
    const workflows = new Map(state.workflows)
    workflows.set(workflowId, {
      ...workflow,
      stages: workflow.stages.filter((s) => s.stageId !== stageId),
      updatedAt: Date.now(),
    })
    workflowStore.set({ ...state, workflows })
  }
}

export async function executeWorkflow(workflowId: string): Promise<string> {
  const state = workflowStore.get()
  const workflow = state.workflows.get(workflowId)
  if (!workflow) throw new Error("Workflow not found")

  const executionId = `exec-${Date.now()}`
  const execution: WorkflowExecution = {
    executionId,
    workflowId,
    status: "running",
    startedAt: Date.now(),
    stageResults: workflow.stages.map((s) => ({ stageId: s.stageId, status: "idle" })),
  }

  const executions = new Map(state.executions)
  executions.set(executionId, execution)
  workflowStore.set({ ...state, executions })

  return executionId
}

export function getWorkflowsForAI(): Workflow[] {
  return Array.from(workflowStore.get().workflows.values())
}

export function getWorkflowDetailsForAI(workflowId: string): Workflow | undefined {
  return workflowStore.get().workflows.get(workflowId)
}

export function createWorkflowFromAI(name: string, description: string, trigger: TriggerType): string {
  return createWorkflow(name, description, trigger)
}

export function updateWorkflowFromAI(id: string, update: Partial<Workflow>): void {
  updateWorkflow(id, update)
}

export function getExecutionHistory(workflowId: string): WorkflowExecution[] {
  return Array.from(workflowStore.get().executions.values())
    .filter((e) => e.workflowId === workflowId)
}
