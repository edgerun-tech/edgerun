import { atom, computed } from 'nanostores'

export type SelfReviewState = {
  turnCount: number
  lastReviewAt: number | null
  failures: string[]
  unclearRequirements: string[]
  missingTools: string[]
  badAssumptions: string[]
  userCorrections: string[]
  automationOpportunities: string[]
  promptWeaknesses: string[]
}

export type SelfReviewResult = {
  needsReview: boolean
  issues: string[]
  suggestions: string[]
}

const REVIEW_INTERVAL_TURNS = 10

export const selfReviewStore = atom<SelfReviewState>({
  turnCount: 0,
  lastReviewAt: null,
  failures: [],
  unclearRequirements: [],
  missingTools: [],
  badAssumptions: [],
  userCorrections: [],
  automationOpportunities: [],
  promptWeaknesses: [],
})

export const needsReview = computed(selfReviewStore, (state) => {
  return state.turnCount > 0 && state.turnCount % REVIEW_INTERVAL_TURNS === 0
})

export function incrementTurn(): void {
  const current = selfReviewStore.get()
  selfReviewStore.set({
    ...current,
    turnCount: current.turnCount + 1,
  })
}

export function recordFailure(failure: string): void {
  const current = selfReviewStore.get()
  selfReviewStore.set({
    ...current,
    failures: [...current.failures, failure],
  })
}

export function recordUnclearRequirement(requirement: string): void {
  const current = selfReviewStore.get()
  selfReviewStore.set({
    ...current,
    unclearRequirements: [...current.unclearRequirements, requirement],
  })
}

export function recordMissingTool(tool: string): void {
  const current = selfReviewStore.get()
  selfReviewStore.set({
    ...current,
    missingTools: [...current.missingTools, tool],
  })
}

export function recordBadAssumption(assumption: string): void {
  const current = selfReviewStore.get()
  selfReviewStore.set({
    ...current,
    badAssumptions: [...current.badAssumptions, assumption],
  })
}

export function recordUserCorrection(correction: string): void {
  const current = selfReviewStore.get()
  selfReviewStore.set({
    ...current,
    userCorrections: [...current.userCorrections, correction],
  })
}

export function recordAutomationOpportunity(opportunity: string): void {
  const current = selfReviewStore.get()
  selfReviewStore.set({
    ...current,
    automationOpportunities: [...current.automationOpportunities, opportunity],
  })
}

export function recordPromptWeakness(weakness: string): void {
  const current = selfReviewStore.get()
  selfReviewStore.set({
    ...current,
    promptWeaknesses: [...current.promptWeaknesses, weakness],
  })
}

export function performSelfReview(): SelfReviewResult {
  const state = selfReviewStore.get()
  const issues: string[] = []
  const suggestions: string[] = []

  if (state.failures.length > 0) {
    issues.push(`${state.failures.length} repeated failures detected`)
    suggestions.push('Fix recurring failure patterns')
  }

  if (state.unclearRequirements.length > 0) {
    issues.push(`${state.unclearRequirements.length} unclear requirements`)
    suggestions.push('Request clarification before proceeding')
  }

  if (state.missingTools.length > 0) {
    issues.push(`Missing tools: ${state.missingTools.join(', ')}`)
    suggestions.push('Create missing tool implementations')
  }

  if (state.badAssumptions.length > 0) {
    issues.push(`${state.badAssumptions.length} bad assumptions made`)
    suggestions.push('Verify assumptions before acting')
  }

  if (state.userCorrections.length > 0) {
    issues.push(`${state.userCorrections.length} user corrections needed`)
    suggestions.push('Learn from user corrections')
  }

  if (state.automationOpportunities.length > 0) {
    suggestions.push(`Automate: ${state.automationOpportunities[0]}`)
  }

  if (state.promptWeaknesses.length > 0) {
    suggestions.push('Propose improved system prompt')
  }

  selfReviewStore.set({
    ...state,
    lastReviewAt: Date.now(),
    failures: [],
    unclearRequirements: [],
    missingTools: [],
    badAssumptions: [],
    userCorrections: [],
    automationOpportunities: [],
    promptWeaknesses: [],
  })

  return {
    needsReview: issues.length > 0,
    issues,
    suggestions,
  }
}

export function resetSelfReview(): void {
  selfReviewStore.set({
    turnCount: 0,
    lastReviewAt: null,
    failures: [],
    unclearRequirements: [],
    missingTools: [],
    badAssumptions: [],
    userCorrections: [],
    automationOpportunities: [],
    promptWeaknesses: [],
  })
}
