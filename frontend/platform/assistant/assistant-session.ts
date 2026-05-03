import { atom, computed } from 'nanostores'

export type AssistantMessageRole = 'user' | 'assistant' | 'system_event'

export interface AssistantMessage {
  id: string
  role: AssistantMessageRole
  content: string
  timestamp: number
  evidenceRefs?: AssistantEvidenceRef[]
  toolCallIds?: string[]
  approvalIds?: string[]
}

export type AssistantToolCallStatus = 'planned' | 'running' | 'succeeded' | 'failed' | 'blocked' | 'approval_required'

export type AssistantRiskClass =
  | 'read_only'
  | 'sensitive_read'
  | 'write'
  | 'destructive'
  | 'signing'
  | 'payment'
  | 'external_connection'
  | 'admin'

export interface AssistantToolCall {
  id: string
  toolId: string
  title: string
  input: unknown
  inputHash: string
  status: AssistantToolCallStatus
  risk: AssistantRiskClass
  requiredCapabilities: string[]
  startedAt?: number
  completedAt?: number
  resultSummary?: string
  resultHash?: string
  error?: string
}

export type AssistantEvidenceKind = 'node' | 'app' | 'capability' | 'object' | 'command' | 'event' | 'workflow' | 'file' | 'test_report' | 'benchmark' | 'receipt'

export interface AssistantEvidenceRef {
  kind: AssistantEvidenceKind
  id: string
  label: string
  hash?: string
}

export interface AssistantSession {
  id: string
  messages: AssistantMessage[]
  toolCalls: AssistantToolCall[]
  pendingApprovalIds: string[]
  contextSnapshotId: string | null
  activeAppId: string | null
  activeObjectRef: string | null
  lastError: string | null
  isLoading: boolean
  modelStatus: string
  userFeedbackEvents: UserFeedbackEvent[]
}

export type UserFeedbackType = 'thumbs_up' | 'thumbs_down' | 'wrong' | 'not_useful' | 'remember_this' | 'forget_this' | 'create_issue' | 'turn_into_workflow'

export interface UserFeedbackEvent {
  id: string
  messageId: string
  type: UserFeedbackType
  timestamp: number
  note?: string
}

const DEFAULT_SESSION: AssistantSession = {
  id: '',
  messages: [],
  toolCalls: [],
  pendingApprovalIds: [],
  contextSnapshotId: null,
  activeAppId: null,
  activeObjectRef: null,
  lastError: null,
  isLoading: false,
  modelStatus: 'idle',
  userFeedbackEvents: [],
}

function generateSessionId(): string {
  return `session_${Date.now()}_${Math.random().toString(36).slice(2, 11)}`
}

export const assistantSessionStore = atom<AssistantSession>({
  ...DEFAULT_SESSION,
  id: generateSessionId(),
})

export const messages = computed(assistantSessionStore, (s) => s.messages)
export const toolCalls = computed(assistantSessionStore, (s) => s.toolCalls)
export const isLoading = computed(assistantSessionStore, (s) => s.isLoading)
export const pendingApprovals = computed(assistantSessionStore, (s) => s.pendingApprovalIds)

export function addAssistantMessage(
  role: AssistantMessageRole,
  content: string,
  evidenceRefs: AssistantEvidenceRef[] = [],
  toolCallIds: string[] = [],
  approvalIds: string[] = []
): string {
  const id = `msg_${Date.now()}_${Math.random().toString(36).slice(2, 11)}`
  const session = assistantSessionStore.get()

  const message: AssistantMessage = {
    id,
    role,
    content,
    timestamp: Date.now(),
    evidenceRefs,
    toolCallIds,
    approvalIds,
  }

  assistantSessionStore.set({
    ...session,
    messages: [...session.messages, message],
  })

  return id
}

export function updateAssistantMessage(id: string, updates: Partial<AssistantMessage>): void {
  const session = assistantSessionStore.get()
  const updated = session.messages.map((m) =>
    m.id === id ? { ...m, ...updates } : m
  )
  assistantSessionStore.set({ ...session, messages: updated })
}

export function addToolCall(toolCall: Omit<AssistantToolCall, 'id'>): string {
  const id = `tool_${Date.now()}_${Math.random().toString(36).slice(2, 11)}`
  const session = assistantSessionStore.get()

  assistantSessionStore.set({
    ...session,
    toolCalls: [...session.toolCalls, { ...toolCall, id }],
  })

  return id
}

export function updateToolCall(id: string, updates: Partial<AssistantToolCall>): void {
  const session = assistantSessionStore.get()
  const updated = session.toolCalls.map((tc) =>
    tc.id === id ? { ...tc, ...updates } : tc
  )
  assistantSessionStore.set({ ...session, toolCalls: updated })
}

export function setAssistantLoading(loading: boolean): void {
  const session = assistantSessionStore.get()
  assistantSessionStore.set({ ...session, isLoading: loading })
}

export function setModelStatus(status: string): void {
  const session = assistantSessionStore.get()
  assistantSessionStore.set({ ...session, modelStatus: status })
}

export function setLastError(error: string | null): void {
  const session = assistantSessionStore.get()
  assistantSessionStore.set({ ...session, lastError: error })
}

export function addPendingApproval(approvalId: string): void {
  const session = assistantSessionStore.get()
  assistantSessionStore.set({
    ...session,
    pendingApprovalIds: [...session.pendingApprovalIds, approvalId],
  })
}

export function removePendingApproval(approvalId: string): void {
  const session = assistantSessionStore.get()
  assistantSessionStore.set({
    ...session,
    pendingApprovalIds: session.pendingApprovalIds.filter((id) => id !== approvalId),
  })
}

export function setActiveApp(appId: string | null): void {
  const session = assistantSessionStore.get()
  assistantSessionStore.set({ ...session, activeAppId: appId })
}

export function setActiveObjectRef(ref: string | null): void {
  const session = assistantSessionStore.get()
  assistantSessionStore.set({ ...session, activeObjectRef: ref })
}

export function addUserFeedback(messageId: string, type: UserFeedbackType, note?: string): void {
  const session = assistantSessionStore.get()
  const event: UserFeedbackEvent = {
    id: `fb_${Date.now()}_${Math.random().toString(36).slice(2, 11)}`,
    messageId,
    type,
    timestamp: Date.now(),
    note,
  }
  assistantSessionStore.set({
    ...session,
    userFeedbackEvents: [...session.userFeedbackEvents, event],
  })
}

export function clearAssistantSession(): void {
  assistantSessionStore.set({
    ...DEFAULT_SESSION,
    id: generateSessionId(),
  })
}

export function getSessionSummary(): string {
  const session = assistantSessionStore.get()
  return `Messages: ${session.messages.length}, Tools: ${session.toolCalls.length}, Pending: ${session.pendingApprovalIds.length}, Errors: ${session.lastError ? 1 : 0}`
}
