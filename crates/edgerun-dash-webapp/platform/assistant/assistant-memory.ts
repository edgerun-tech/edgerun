import { atom, computed } from 'nanostores'

export type MemoryKind =
  | 'preference'
  | 'project_fact'
  | 'known_issue'
  | 'accepted_action'
  | 'rejected_action'
  | 'workflow_pattern'
  | 'debug_note'

export type MemoryConfidence = 'low' | 'medium' | 'high'
export type MemorySource = 'user' | 'approval' | 'tool_result' | 'manual'

export interface AssistantMemory {
  id: string
  kind: MemoryKind
  text: string
  source: MemorySource
  confidence: MemoryConfidence
  createdAt: number
  updatedAt: number
  evidenceRefs: string[]
  expiresAt?: number
  tags: string[]
}

export interface MemoryState {
  memories: AssistantMemory[]
  contextAge: number
  lastRefresh: number
}

const MEMORY_STORAGE_KEY = 'edgerun-assistant-memory'
const CONTEXT_STALE_THRESHOLD = 60000

export const memoryStore = atom<MemoryState>({
  memories: [],
  contextAge: 0,
  lastRefresh: Date.now(),
})

export const memories = computed(memoryStore, (state) => state.memories)
export const contextAge = computed(memoryStore, (state) => state.contextAge)
export const isContextStale = computed(memoryStore, (state) => {
  return state.contextAge > CONTEXT_STALE_THRESHOLD
})

export function generateMemoryId(): string {
  return `mem_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
}

export function addMemory(
  kind: MemoryKind,
  text: string,
  source: MemorySource,
  confidence: MemoryConfidence = 'medium',
  evidenceRefs: string[] = [],
  tags: string[] = [],
  expiresAt?: number
): void {
  const state = memoryStore.get()
  const memory: AssistantMemory = {
    id: generateMemoryId(),
    kind,
    text,
    source,
    confidence,
    createdAt: Date.now(),
    updatedAt: Date.now(),
    evidenceRefs,
    tags,
    expiresAt,
  }
  memoryStore.set({
    ...state,
    memories: [...state.memories, memory],
  })
  saveMemories()
}

export function updateMemory(id: string, updates: Partial<AssistantMemory>): void {
  const state = memoryStore.get()
  const updated = state.memories.map((m) =>
    m.id === id ? { ...m, ...updates, updatedAt: Date.now() } : m
  )
  memoryStore.set({ ...state, memories: updated })
  saveMemories()
}

export function deleteMemory(id: string): void {
  const state = memoryStore.get()
  memoryStore.set({
    ...state,
    memories: state.memories.filter((m) => m.id !== id),
  })
  saveMemories()
}

export function getMemoriesByKind(kind: MemoryKind): AssistantMemory[] {
  return memoryStore.get().memories.filter((m) => m.kind === kind)
}

export function getMemoriesByTag(tag: string): AssistantMemory[] {
  return memoryStore.get().memories.filter((m) => m.tags.includes(tag))
}

export function getHighConfidenceMemories(): AssistantMemory[] {
  return memoryStore.get().memories.filter((m) => m.confidence === 'high')
}

export function clearExpiredMemories(): void {
  const state = memoryStore.get()
  const now = Date.now()
  memoryStore.set({
    ...state,
    memories: state.memories.filter(
      (m) => !m.expiresAt || m.expiresAt > now
    ),
  })
}

export function refreshContextAge(): void {
  const state = memoryStore.get()
  memoryStore.set({
    ...state,
    contextAge: Date.now() - state.lastRefresh,
    lastRefresh: Date.now(),
  })
}

export function saveMemories(): void {
  try {
    const state = memoryStore.get()
    localStorage.setItem(MEMORY_STORAGE_KEY, JSON.stringify(state.memories))
  } catch {
    // Storage unavailable
  }
}

export function loadMemories(): void {
  try {
    const saved = localStorage.getItem(MEMORY_STORAGE_KEY)
    if (saved) {
      const memories = JSON.parse(saved) as AssistantMemory[]
      memoryStore.set({
        memories,
        contextAge: 0,
        lastRefresh: Date.now(),
      })
    }
  } catch {
    // Storage unavailable or corrupted
  }
}

export function recordUserCorrection(correction: string): void {
  addMemory('rejected_action', correction, 'user', 'medium')
}

export function recordAcceptedAction(action: string, evidenceRefs: string[] = []): void {
  addMemory('accepted_action', action, 'approval', 'high', evidenceRefs)
}

export function recordRejectedAction(reason: string): void {
  addMemory('rejected_action', reason, 'user', 'high')
}

export function recordPreference(key: string, value: string): void {
  addMemory('preference', `${key}: ${value}`, 'user', 'high')
}

export function recordKnownIssue(issue: string, evidenceRefs: string[] = []): void {
  addMemory('known_issue', issue, 'tool_result', 'medium', evidenceRefs)
}

export function getMemorySummary(): string {
  const state = memoryStore.get()
  const byKind = new Map<MemoryKind, number>()
  state.memories.forEach((m) => {
    byKind.set(m.kind, (byKind.get(m.kind) || 0) + 1)
  })
  return Array.from(byKind.entries())
    .map(([kind, count]) => `${kind}: ${count}`)
    .join(', ')
}
