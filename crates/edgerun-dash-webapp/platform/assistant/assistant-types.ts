/**
 * Assistant types.
 * Canonical types for the EdgeRun Control Assistant.
 * Replaces inline types in stores/ai-chat-store.ts.
 */

export interface AssistantMessage {
  id: string
  role: "user" | "assistant"
  content: string
  timestamp: number
  toolCalls?: AssistantToolCall[]
  evidenceRefs?: string[]
}

export interface AssistantToolCall {
  toolId: string
  input: Record<string, unknown>
  result?: unknown
  status: "pending" | "approved" | "rejected" | "executed" | "failed"
  approvalId?: string
  error?: string
}

export interface AssistantSession {
  messages: AssistantMessage[]
  isLoading: boolean
  context: Record<string, unknown>
}

export type RiskClass = "none" | "low" | "medium" | "high" | "critical"

export interface ToolSpec {
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
}

export interface ToolCallPlan {
  toolId: string
  input: Record<string, unknown>
  riskClass: RiskClass
  requiresApproval: boolean
  reasonIfBlocked?: string
}

export interface EvidenceRef {
  kind: "node" | "app" | "capability" | "object" | "command" | "event" | "workflow" | "file" | "test_report" | "benchmark" | "receipt"
  id: string
  label: string
  url?: string
}

export interface AssistantFeedback {
  messageId: string
  kind: "thumbs_up" | "thumbs_down" | "wrong" | "remember" | "forget"
  note?: string
  turnedInto?: "workflow" | "tool" | "issue"
}
