import { computed } from 'nanostores'
import { contextSnapshotStore, getContextSummary } from './assistant-types-context'
import { toolRegistry } from '../registries/tool-registry'
import { approvalTracker } from '../auth/approval-tracker'

export interface PromptSection {
  name: string
  content: string
  priority: number
}

export interface SystemPrompt {
  base: string
  dynamicSections: PromptSection[]
  version: string
}

const BASE_PROMPT = `You are EdgeRun Control Assistant, not a generic chatbot.

Your role is to help users operate and debug the EdgeRun platform through its native services.

Core rules:
- Never invent state. Only report what platform evidence confirms.
- Use platform tools to read and propose actions.
- State-changing actions require approval through the platform approval flow.
- Cite evidence refs in your answers when referencing platform state.
- Treat generated protobuf/domain types as canonical.
- Payments are non-custodial but not risk-free - be clear about this.
- Provider names must be hidden in user-facing wallet/exchange flows.

You can help with:
- Node health diagnostics and debugging
- App installation, inspection, and management
- Capability explanation and troubleshooting
- Workflow creation and execution
- Permission and approval management
- System metrics interpretation

When you don't know something, say so. When you need to change platform state, propose it as an approval request, never execute directly.`

export function getBasePrompt(): string {
  return BASE_PROMPT
}

export function getDynamicContextSection(): PromptSection {
  const snapshot = contextSnapshotStore.get()
  if (!snapshot) {
    return {
      name: 'Context',
      content: 'No platform context available',
      priority: 1,
    }
  }

  return {
    name: 'Current Context',
    content: snapshot.textSummary,
    priority: 2,
  }
}

export function getToolsSection(): PromptSection {
  const registry = toolRegistry.get()
  const enabledTools = Array.from(registry.tools.values()).filter((t) => t.isEnabled)

  const toolList = enabledTools
    .slice(0, 20)
    .map((t: any) => `- ${t.name}: ${t.description}`)
    .join('\n')

  return {
    name: 'Available Tools',
    content: `Platform tools (${enabledTools.length} enabled):\n${toolList}`,
    priority: 3,
  }
}

export function getPendingApprovalsSection(): PromptSection {
  const approvals = approvalTracker.get()

  if (approvals.pending.size === 0) {
    return {
      name: 'Approvals',
      content: 'No pending approvals',
      priority: 4,
    }
  }

  const pendingList = Array.from(approvals.pending.values())
    .map((a) => `- ${a.operation} (${a.riskLevel})`)
    .join('\n')

  return {
    name: 'Pending Approvals',
    content: pendingList,
    priority: 4,
  }
}

export function buildSystemPrompt(): SystemPrompt {
  const sections: PromptSection[] = [
    { name: 'Base', content: BASE_PROMPT, priority: 0 },
    getDynamicContextSection(),
    getToolsSection(),
    getPendingApprovalsSection(),
  ]

  sections.sort((a, b) => a.priority - b.priority)

  return {
    base: BASE_PROMPT,
    dynamicSections: sections,
    version: '1.0.0',
  }
}

export function getPromptForModel(): string {
  const sections = buildSystemPrompt()
  return sections.dynamicSections.map((s) => s.content).join('\n\n')
}

export function getWelcomeMessage(): string {
  const context = getContextSummary()
  return `I can inspect this node, explain apps/capabilities, propose workflows, and help debug.\n\nCurrent state: ${context}\n\nWhat would you like to do?`
}
