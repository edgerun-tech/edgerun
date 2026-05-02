import { computed } from 'nanostores'
import { contextSnapshotStore, getContextSummary } from './assistant-context'
import { toolRegistryStore } from '../registries/tool-registry'
import { approvalTrackerStore } from '../auth/approval-tracker'

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

  const lines: string[] = []

  if (snapshot.node.nodeId) {
    lines.push(`Current node: ${snapshot.node.nodeId}`)
    lines.push(`Node health: ${snapshot.node.health || 'unknown'}`)
  }

  if (snapshot.apps.running.length > 0) {
    lines.push(`Running apps: ${snapshot.apps.running.join(', ')}`)
  }

  lines.push(`Available capabilities: ${snapshot.capabilities.available.length}`)

  if (snapshot.approvals.pending.length > 0) {
    lines.push(`Pending approvals: ${snapshot.approvals.pending.map((a) => a.action).join(', ')}`)
  }

  return {
    name: 'Current Context',
    content: lines.join('\n'),
    priority: 2,
  }
}

export function getToolsSection(): PromptSection {
  const registry = toolRegistryStore.get()
  const enabledTools = registry.tools.filter((t) => t.isEnabled)

  const toolList = enabledTools
    .slice(0, 20)
    .map((t) => `- ${t.name}: ${t.description}`)
    .join('\n')

  return {
    name: 'Available Tools',
    content: `Platform tools (${enabledTools.length} enabled):\n${toolList}`,
    priority: 3,
  }
}

export function getPendingApprovalsSection(): PromptSection {
  const approvals = approvalTrackerStore.get()

  if (approvals.pending.length === 0) {
    return {
      name: 'Approvals',
      content: 'No pending approvals',
      priority: 4,
    }
  }

  const pendingList = approvals.pending
    .map((a) => `- ${a.action} (${a.status})`)
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
