export * from './assistant-operating-contract'
export * from './assistant-dependency-policy'
export * from './assistant-verification'
export * from './assistant-git-awareness'
export * from './assistant-session'
export * from './assistant-context'
export * from './assistant-prompts'

// Explicit exports to avoid ambiguity
export {
  selfReviewStore,
  needsReview,
  incrementTurn,
  recordFailure,
  recordUnclearRequirement,
  recordMissingTool,
  recordBadAssumption,
  recordUserCorrection as recordSelfReviewCorrection,
  recordAutomationOpportunity,
  recordPromptWeakness,
  performSelfReview,
  resetSelfReview,
  type SelfReviewState,
  type SelfReviewResult,
} from './assistant-self-review'

export {
  memoryStore,
  memories,
  contextAge,
  isContextStale,
  generateMemoryId,
  addMemory,
  updateMemory,
  deleteMemory,
  getMemoriesByKind,
  getMemoriesByTag,
  getHighConfidenceMemories,
  clearExpiredMemories,
  refreshContextAge,
  saveMemories,
  loadMemories,
  type MemoryKind,
  type MemoryConfidence,
  type MemorySource,
  type AssistantMemory,
  type MemoryState,
} from './assistant-memory'

export { recordUserCorrection } from './assistant-self-review'

// Legacy-assitant files (renamed and merged into assistant/)
export { contextSnapshotStore, buildContext, getContextSummary as getContextSummaryText, initializeContext } from './assistant-types-context'
export type { ContextSnapshot, NodeContext, AppsContext, AppInfo, CapabilitiesContext, ConnectionsContext, ApprovalsContext, ApprovalInfo, RuntimeContext } from './assistant-types-context'
export { getBasePrompt, getDynamicContextSection, getToolsSection, getPendingApprovalsSection, buildSystemPrompt, getPromptForModel, getWelcomeMessage } from './assistant-types-prompts'
export { assistantSession, addMessage, setLoading, clearSession, buildPlatformContext } from './assistant-types-session'
export type { AssistantMessage, AssistantToolCall, AssistantSession, AssistantFeedback, EvidenceRef, RiskClass, ToolSpec, ToolCallPlan } from './assistant-types'
