/**
 * Assistant prompts.
 * System prompts for different assistant modes.
 */

import { assistantContext } from "./assistant-context"
import { listRegisteredComponents, listRegisteredTools } from "@/platform/registries/tool-registry"
import { approvalTracker, getPendingApprovals } from "@/platform/auth/approval-tracker"

export function buildAssistantSystemPrompt(): string {
  const ctx = assistantContext.get()

  const tools = listRegisteredTools()
  const toolNames = tools.map((t: { toolId: string }) => t.toolId).join(", ")

  return `You are the EdgeRun platform assistant.
Current state:
- Installed apps: ${ctx.installedApps.length}
- Available capabilities: ${ctx.availableCapabilities.length}
- Connections: ${ctx.connections.join(", ") || "none"}
- Pending approvals: ${ctx.pendingApprovals}
- Registered tools: ${toolNames}

Always follow protocol invariants.
Never invent RAM/CPU/price data.
If unknown, say unknown.`
}

export function buildTaskPrompt(task: string): string {
  return `Task: ${task}\n\nFollow EdgeRun protocol invariants.`
}
