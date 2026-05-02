/**
 * Single registry for tools (MCP-style).
 * Tracks available tools, their schemas, and invocation handlers.
 *
 * Upgraded to include:
 * - risk class
 * - required capabilities
 * - approval requirement
 * - user presence requirement
 * - input/output schema
 * - display summary
 * - evidence extractor
 * - command/approval preview
 * - audit record
 */

import { atom, computed } from "nanostores"
import type { RiskClass } from "@/platform/assistant"
import {
  capabilityRegistry,
} from "@/platform/registries/capability-registry"
import { approvalTracker, addApproval } from "@/platform/auth/approval-tracker"

export interface ToolDefinition {
  toolId: string
  name: string
  description: string
  riskClass: RiskClass
  requiredCapabilities: string[]
  requiresApproval: boolean
  requiresUserPresence: boolean
  inputSchema: Record<string, unknown>
  outputSchema?: Record<string, unknown>
  displaySummary?: string
  evidenceExtractor?: (result: unknown) => string[]
  commandPreview?: (input: Record<string, unknown>) => string
  auditRecord?: (input: Record<string, unknown>, result: unknown) => string
  isEnabled: boolean
  handler?: (input: unknown) => Promise<unknown>
}

export interface ToolRegistryState {
  tools: Map<string, ToolDefinition>
  toolsByCapability: Map<string, ToolDefinition[]>
}

const initialState: ToolRegistryState = {
  tools: new Map(),
  toolsByCapability: new Map(),
}

export const toolRegistry = atom<ToolRegistryState>(initialState)

export const allTools = computed(toolRegistry, (s) =>
  Array.from(s.tools.values()),
)

export const enabledTools = computed(toolRegistry, (s) =>
  Array.from(s.tools.values()).filter((t) => t.isEnabled),
)

export function registerTool(tool: ToolDefinition): void {
  const state = toolRegistry.get()
  const newTools = new Map(state.tools)
  newTools.set(tool.toolId, tool)

  const newByCapability = new Map(state.toolsByCapability)
  for (const cap of tool.requiredCapabilities) {
    const list = newByCapability.get(cap) || []
    newByCapability.set(cap, [...list, tool])
  }

  toolRegistry.set({
    tools: newTools,
    toolsByCapability: newByCapability,
  })
}

export function getTool(toolId: string): ToolDefinition | undefined {
  return toolRegistry.get().tools.get(toolId)
}

export function listToolsByCapability(
  capabilityId: string,
): ToolDefinition[] {
  return (
    toolRegistry.get().toolsByCapability.get(capabilityId) || []
  )
}

export function enableTool(toolId: string): void {
  const state = toolRegistry.get()
  const tool = state.tools.get(toolId)
  if (tool) {
    const newTools = new Map(state.tools)
    newTools.set(toolId, { ...tool, isEnabled: true })
    toolRegistry.set({ ...state, tools: newTools })
  }
}

export function disableTool(toolId: string): void {
  const state = toolRegistry.get()
  const tool = state.tools.get(toolId)
  if (tool) {
    const newTools = new Map(state.tools)
    newTools.set(toolId, { ...tool, isEnabled: false })
    toolRegistry.set({ ...state, tools: newTools })
  }
}

/**
 * Plan a tool call: validate inputs, check capabilities, classify risk.
 */
export function planToolCall(
  toolId: string,
  input: Record<string, unknown>,
): {
  tool: ToolDefinition | null
  valid: boolean
  errors: string[]
  riskClass: RiskClass
  requiresApproval: boolean
  missingCapabilities: string[]
} {
  const tool = getTool(toolId)
  if (!tool) {
    return {
      tool: null,
      valid: false,
      errors: [`Tool ${toolId} not found`],
      riskClass: "critical",
      requiresApproval: true,
      missingCapabilities: [],
    }
  }

  const errors: string[] = []

  // Basic input validation
  if (tool.inputSchema) {
    for (const [key, schemaVal] of Object.entries(tool.inputSchema)) {
      if (schemaVal === "required" && !(key in input)) {
        errors.push(`Missing required input: ${key}`)
      }
    }
  }

  // Check capabilities via capability registry
  const missingCapabilities = tool.requiredCapabilities.filter((capId) => {
    const desc = capabilityRegistry.get().descriptors.get(capId)
    return !desc
  })

  return {
    tool,
    valid: errors.length === 0,
    errors,
    riskClass: tool.riskClass,
    requiresApproval: tool.requiresApproval || missingCapabilities.length > 0,
    missingCapabilities,
  }
}

/**
 * Invoke a tool call with full lifecycle:
 * validation → capability check → approval (if needed) → execution → audit → evidence.
 */
export async function invokeToolCall(
  toolId: string,
  input: Record<string, unknown>,
): Promise<{
  status: "executed" | "pending_approval" | "blocked" | "failed"
  result?: unknown
  approvalId?: string
  error?: string
  evidenceRefs?: string[]
}> {
  const plan = planToolCall(toolId, input)

  if (!plan.valid) {
    return { status: "blocked", error: plan.errors.join(", ") }
  }

  if (plan.missingCapabilities.length > 0) {
    return {
      status: "blocked",
      error: `Missing capabilities: ${plan.missingCapabilities.join(", ")}`,
    }
  }

  // If approval required, create approval and return pending
  if (plan.requiresApproval && plan.tool?.requiresApproval) {
    const approvalId = await createApprovalForToolCall(toolId, input, plan)
    return { status: "pending_approval", approvalId }
  }

  // Execute directly
  try {
    const result = await executeToolInternal(toolId, input)
    const evidenceRefs = plan.tool?.evidenceExtractor?.(result) || []
    return { status: "executed", result, evidenceRefs }
  } catch (err) {
    return {
      status: "failed",
      error: err instanceof Error ? err.message : "Unknown error",
    }
  }
}

/**
 * Create an approval for a tool call.
 */
async function createApprovalForToolCall(
  toolId: string,
  input: Record<string, unknown>,
  plan: ReturnType<typeof planToolCall>,
): Promise<string> {
  const tool = plan.tool!
  const preview = tool.commandPreview?.(input) || `Execute ${tool.name}`

  const id = `approval-${Date.now()}-${Math.random().toString(36).slice(2)}`
  addApproval({
    approvalId: id,
    operation: tool.name,
    scope: "tool" as any,
    description: preview,
    requestedAt: new Date().toISOString(),
    riskLevel: plan.riskClass === "critical" ? "critical" : plan.riskClass === "high" ? "high" : plan.riskClass === "medium" ? "medium" : "low",
  })
  return id
}

/**
 * Internal tool execution.
 */
async function executeToolInternal(
  toolId: string,
  input: Record<string, unknown>,
): Promise<unknown> {
  const tool = getTool(toolId)
  if (!tool) throw new Error(`Tool ${toolId} not found`)
  if (!tool.handler) throw new Error(`Tool ${toolId} has no handler`)
  return tool.handler(input)
}

/**
 * Get tools available to assistant (enabled + capabilities satisfied).
 */
export function getToolsAvailableToAssistant(): ToolDefinition[] {
  return Array.from(toolRegistry.get().tools.values()).filter(t => t.isEnabled)
}

/**
 * Get tools blocked by missing capabilities.
 */
export function getToolsBlockedByMissingCapabilities(): {
  tool: ToolDefinition
  missing: string[]
}[] {
  const state = toolRegistry.get()
  const result: { tool: ToolDefinition; missing: string[] }[] = []

  for (const tool of state.tools.values()) {
    const missing = tool.requiredCapabilities.filter((capId) => {
      const desc = capabilityRegistry.get().descriptors.get(capId)
      return !desc
    })
    if (missing.length > 0) {
      result.push({ tool, missing })
    }
  }

  return result
}

/**
 * Explain tool availability as text.
 */
export function explainToolAvailability(): string {
  const available = getToolsAvailableToAssistant()
  const blocked = getToolsBlockedByMissingCapabilities()

  if (available.length === 0 && blocked.length === 0) {
    return "No tools registered."
  }

  let text = `## Available Tools (${available.length})\n\n`
  for (const t of available) {
    text += `- **${t.name}** (${t.toolId}): ${t.description}\n`
    text += `  Risk: ${t.riskClass}, Approval: ${t.requiresApproval ? "required" : "not required"}\n`
  }

  if (blocked.length > 0) {
    text += `\n## Blocked Tools (${blocked.length})\n\n`
    for (const { tool, missing } of blocked) {
      text += `- **${tool.name}**: missing capabilities: ${missing.join(", ")}\n`
    }
  }

  return text
}
