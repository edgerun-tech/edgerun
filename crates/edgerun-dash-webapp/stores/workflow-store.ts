import { atom } from "nanostores"

export type TriggerType = "manual" | "schedule" | "event" | "webhook"
export type ActionType = "http" | "deploy" | "notify" | "transform" | "condition" | "delay" | "log"
export type StageStatus = "idle" | "running" | "success" | "failed" | "skipped"

export interface WorkflowTrigger {
  type: TriggerType
  config: Record<string, unknown>
}

export interface WorkflowAction {
  id: string
  type: ActionType
  name: string
  config: Record<string, unknown>
}

export interface WorkflowStage {
  id: string
  name: string
  actions: WorkflowAction[]
  onSuccess?: string
  onFailure?: string
}

export interface Workflow {
  id: string
  name: string
  description: string
  enabled: boolean
  createdAt: number
  updatedAt: number
  triggers: WorkflowTrigger[]
  stages: WorkflowStage[]
  variables: Record<string, string>
}

export interface WorkflowExecution {
  id: string
  workflowId: string
  status: StageStatus
  startedAt: number
  completedAt?: number
  currentStage?: string
  logs: { stage: string; action: string; message: string; timestamp: number }[]
  error?: string
}

export interface WorkflowStore {
  workflows: Workflow[]
  executions: WorkflowExecution[]
  activeWorkflowId: string | null
  isExecuting: boolean
}

const STORAGE_KEY = "edgerun_workflows"

function loadFromStorage(): Workflow[] {
  if (typeof window === "undefined") return []
  const raw = localStorage.getItem(STORAGE_KEY)
  if (!raw) return []
  try {
    return JSON.parse(raw)
  } catch {
    return []
  }
}

function saveToStorage(workflows: Workflow[]) {
  if (typeof window === "undefined") return
  localStorage.setItem(STORAGE_KEY, JSON.stringify(workflows))
}

export const workflowStore = atom<WorkflowStore>({
  workflows: [],
  executions: [],
  activeWorkflowId: null,
  isExecuting: false,
})

if (typeof window !== "undefined") {
  const stored = loadFromStorage()
  workflowStore.set({ workflows: stored, executions: [], activeWorkflowId: null, isExecuting: false })
}

function generateId(): string {
  return `wf-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
}

export function createWorkflow(name: string, description: string = ""): Workflow {
  const workflow: Workflow = {
    id: generateId(),
    name,
    description,
    enabled: true,
    createdAt: Date.now(),
    updatedAt: Date.now(),
    triggers: [{ type: "manual", config: {} }],
    stages: [
      {
        id: generateId(),
        name: "Start",
        actions: [],
      },
    ],
    variables: {},
  }
  
  const store = workflowStore.get()
  const newWorkflows = [...store.workflows, workflow]
  workflowStore.set({ ...store, workflows: newWorkflows })
  saveToStorage(newWorkflows)
  
  return workflow
}

export function updateWorkflow(id: string, updates: Partial<Workflow>) {
  const store = workflowStore.get()
  const newWorkflows = store.workflows.map(w => 
    w.id === id ? { ...w, ...updates, updatedAt: Date.now() } : w
  )
  workflowStore.set({ ...store, workflows: newWorkflows })
  saveToStorage(newWorkflows)
}

export function deleteWorkflow(id: string) {
  const store = workflowStore.get()
  const newWorkflows = store.workflows.filter(w => w.id !== id)
  workflowStore.set({ ...store, workflows: newWorkflows })
  saveToStorage(newWorkflows)
}

export function getWorkflow(id: string): Workflow | undefined {
  const store = workflowStore.get()
  return store.workflows.find(w => w.id === id)
}

export function addStage(workflowId: string, name: string): WorkflowStage | null {
  const store = workflowStore.get()
  const workflow = store.workflows.find(w => w.id === workflowId)
  if (!workflow) return null
  
  const stage: WorkflowStage = {
    id: generateId(),
    name,
    actions: [],
  }
  
  const newStages = [...workflow.stages, stage]
  updateWorkflow(workflowId, { stages: newStages })
  
  return stage
}

export function addAction(workflowId: string, stageId: string, action: Omit<WorkflowAction, "id">): WorkflowAction | null {
  const store = workflowStore.get()
  const workflow = store.workflows.find(w => w.id === workflowId)
  if (!workflow) return null
  
  const newStages = workflow.stages.map(stage => {
    if (stage.id !== stageId) return stage
    return {
      ...stage,
      actions: [...stage.actions, { ...action, id: generateId() }],
    }
  })
  
  updateWorkflow(workflowId, { stages: newStages })
  
  return { ...action, id: generateId() }
}

export function removeAction(workflowId: string, stageId: string, actionId: string) {
  const store = workflowStore.get()
  const workflow = store.workflows.find(w => w.id === workflowId)
  if (!workflow) return
  
  const newStages = workflow.stages.map(stage => {
    if (stage.id !== stageId) return stage
    return {
      ...stage,
      actions: stage.actions.filter(a => a.id !== actionId),
    }
  })
  
  updateWorkflow(workflowId, { stages: newStages })
}

export function removeStage(workflowId: string, stageId: string) {
  const store = workflowStore.get()
  const workflow = store.workflows.find(w => w.id === workflowId)
  if (!workflow || workflow.stages.length <= 1) return
  
  const newStages = workflow.stages.filter(s => s.id !== stageId)
  updateWorkflow(workflowId, { stages: newStages })
}

export async function executeWorkflow(workflowId: string): Promise<WorkflowExecution> {
  const store = workflowStore.get()
  const workflow = store.workflows.find(w => w.id === workflowId)
  if (!workflow) {
    throw new Error("Workflow not found")
  }
  
  const execution: WorkflowExecution = {
    id: `exec-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    workflowId,
    status: "running",
    startedAt: Date.now(),
    logs: [],
  }
  
  workflowStore.set(s => ({ 
    ...s, 
    executions: [...s.executions.slice(-49), execution], 
    isExecuting: true 
  }))
  
  try {
    for (const stage of workflow.stages) {
      execution.currentStage = stage.id
      
      workflowStore.set(s => ({
        ...s,
        executions: s.executions.map(e => e.id === execution.id ? execution : e),
      }))
      
      for (const action of stage.actions) {
        const log = await executeAction(action, workflow.variables)
        execution.logs.push({
          stage: stage.name,
          action: action.name,
          message: log,
          timestamp: Date.now(),
        })
        
        workflowStore.set(s => ({
          ...s,
          executions: s.executions.map(e => e.id === execution.id ? execution : e),
        }))
      }
    }
    
    execution.status = "success"
    execution.completedAt = Date.now()
  } catch (err) {
    execution.status = "failed"
    execution.error = err instanceof Error ? err.message : "Unknown error"
    execution.completedAt = Date.now()
  }
  
  workflowStore.set(s => ({ 
    ...s, 
    executions: s.executions.map(e => e.id === execution.id ? execution : e),
    isExecuting: false,
  }))
  
  return execution
}

async function executeAction(action: WorkflowAction, vars: Record<string, string>): Promise<string> {
  const resolvedConfig = resolveVariables(action.config, vars)
  
  switch (action.type) {
    case "log":
      return `LOG: ${resolvedConfig.message || "No message"}`
    
    case "delay":
      const ms = Number(resolvedConfig.duration) * 1000
      await new Promise(r => setTimeout(r, ms))
      return `Delayed for ${resolvedConfig.duration}s`
    
    case "http":
      try {
        const url = String(resolvedConfig.url)
        const method = String(resolvedConfig.method || "GET")
        const body = resolvedConfig.body ? JSON.stringify(resolvedConfig.body) : undefined
        
        const response = await fetch(url, { method, body, headers: resolvedConfig.headers as Record<string, string> })
        return `HTTP ${method} ${url} -> ${response.status} ${response.statusText}`
      } catch (e) {
        return `HTTP ERROR: ${e instanceof Error ? e.message : "Unknown"}`
      }
    
    case "notify":
      return `Notification: ${resolvedConfig.title || "No title"} - ${resolvedConfig.message || "No message"}`
    
    case "deploy":
      return `Deploy action: ${resolvedConfig.target || "No target"} - ${resolvedConfig.image || "No image"}`
    
    case "condition":
      const condition = String(resolvedConfig.condition)
      const result = eval(condition)
      return `Condition "${condition}" = ${result}`
    
    case "transform":
      return `Transform: ${resolvedConfig.operation || "none"} on ${resolvedConfig.field || "unknown"}`
    
    default:
      return `Unknown action type: ${action.type}`
  }
}

function resolveVariables(config: Record<string, unknown>, vars: Record<string, string>): Record<string, unknown> {
  const resolved: Record<string, unknown> = {}
  
  for (const [key, value] of Object.entries(config)) {
    if (typeof value === "string" && value.startsWith("${") && value.endsWith("}")) {
      const varName = value.slice(2, -1)
      resolved[key] = vars[varName] || value
    } else if (typeof value === "object" && value !== null) {
      resolved[key] = resolveVariables(value as Record<string, unknown>, vars)
    } else {
      resolved[key] = value
    }
  }
  
  return resolved
}

export function getWorkflowsForAI(): string {
  const store = workflowStore.get()
  if (store.workflows.length === 0) {
    return "No workflows defined yet."
  }
  
  return store.workflows.map(w => {
    const stageSummary = w.stages.map(s => `${s.name}: ${s.actions.length} actions`).join(", ")
    return `- **${w.name}** (${w.id.slice(0, 8)}): ${w.description || "No description"} | Stages: ${stageSummary} | Enabled: ${w.enabled}`
  }).join("\n")
}

export function getWorkflowDetailsForAI(workflowId: string): string {
  const workflow = getWorkflow(workflowId)
  if (!workflow) return "Workflow not found"
  
  const stages = workflow.stages.map(s => {
    const actions = s.actions.map(a => `  - ${a.type}: ${a.name}`).join("\n")
    return `### Stage: ${s.name}\n${actions || "  (no actions)"}`
  }).join("\n\n")
  
  const vars = Object.entries(workflow.variables).map(([k, v]) => `${k}=${v}`).join(", ")
  
  return `
## Workflow: ${workflow.name}
**ID**: ${workflow.id}
**Description**: ${workflow.description || "None"}
**Enabled**: ${workflow.enabled}
**Variables**: ${vars || "None"}

**Triggers**:
${workflow.triggers.map(t => `- ${t.type}`).join("\n")}

**Stages**:
${stages}
`
}

export function updateWorkflowFromAI(workflowId: string, aiUpdate: string) {
  const store = workflowStore.get()
  const workflow = store.workflows.find(w => w.id === workflowId)
  if (!workflow) return
  
  try {
    const parsed = JSON.parse(aiUpdate)
    updateWorkflow(workflowId, parsed)
  } catch {
    console.error("Failed to parse AI workflow update:", aiUpdate)
  }
}

export function createWorkflowFromAI(aiSpec: string): Workflow | null {
  try {
    const parsed = JSON.parse(aiSpec)
    const workflow: Workflow = {
      id: generateId(),
      name: parsed.name || "AI Workflow",
      description: parsed.description || "",
      enabled: true,
      createdAt: Date.now(),
      updatedAt: Date.now(),
      triggers: parsed.triggers || [{ type: "manual", config: {} }],
      stages: parsed.stages || [{ id: generateId(), name: "Start", actions: [] }],
      variables: parsed.variables || {},
    }
    
    const store = workflowStore.get()
    const newWorkflows = [...store.workflows, workflow]
    workflowStore.set({ ...store, workflows: newWorkflows })
    saveToStorage(newWorkflows)
    
    return workflow
  } catch {
    console.error("Failed to create workflow from AI:", aiSpec)
    return null
  }
}

export function getExecutionHistory(workflowId: string): WorkflowExecution[] {
  const store = workflowStore.get()
  return store.executions.filter(e => e.workflowId === workflowId).slice(-10)
}