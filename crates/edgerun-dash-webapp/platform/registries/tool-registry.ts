/**
 * Single registry for tools (MCP-style).
 * Tracks available tools, their schemas, and invocation handlers.
 */

import { atom, computed } from "nanostores"

export interface ToolDefinition {
  toolId: string
  name: string
  description: string
  inputSchema: Record<string, unknown>
  outputSchema?: Record<string, unknown>
  requiredCapabilities: string[]
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

export async function invokeTool(
  toolId: string,
  input: unknown,
): Promise<unknown> {
  const tool = getTool(toolId)
  if (!tool) throw new Error(`Tool ${toolId} not found`)
  if (!tool.isEnabled) throw new Error(`Tool ${toolId} is disabled`)
  if (!tool.handler) throw new Error(`Tool ${toolId} has no handler`)
  return tool.handler(input)
}
