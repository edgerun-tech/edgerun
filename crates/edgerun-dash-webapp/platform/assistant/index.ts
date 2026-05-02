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
