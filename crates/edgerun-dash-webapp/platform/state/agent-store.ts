/**
 * Agent truth tracker.
 * Tracks agent activity with verification status.
 * Claimed "done" is NOT verified done.
 */

import { atom, computed } from "nanostores"
import type { AssistantMessage } from "../assistant"

export type AgentStatus = 
  | "idle"
  | "planning"
  | "editing"
  | "testing"
  | "waiting_approval"
  | "blocked"
  | "claimed_done"
  | "verified_done"
  | "failed"
  | "abandoned"

export interface AgentInfo {
  agentId: string
  model?: string
  provider?: string
  taskId?: string
  status: AgentStatus
  currentFiles: string[]
  gitBranch?: string
  lastMessage: string
  claimedCompletion: boolean
  evidenceRefs: string[]
  verificationStatus: "pending" | "passed" | "failed" | "not_run"
  testsRun: boolean
  errors: string[]
  tokenUsage: { prompt: number; completion: number; total: number }
  startedAt: number
  updatedAt: number
  ownerAgentId?: string
  isClaimVerified: boolean
}

export interface AgentState {
  agents: Map<string, AgentInfo>
  activeCount: number
  blockedCount: number
  claimedDoneCount: number
  verifiedDoneCount: number
}

const initialState: AgentState = {
  agents: new Map(),
  activeCount: 0,
  blockedCount: 0,
  claimedDoneCount: 0,
  verifiedDoneCount: 0,
}

export const agentStore = atom<AgentState>(initialState)

export const allAgents = computed(agentStore, (s) =>
  Array.from(s.agents.values()),
)

export const activeAgents = computed(agentStore, (s) =>
  Array.from(s.agents.values()).filter(a => a.status !== "idle" && a.status !== "verified_done" && a.status !== "abandoned"),
)

export function registerAgent(agent: AgentInfo): void {
  const state = agentStore.get()
  const newAgents = new Map(state.agents)
  newAgents.set(agent.agentId, agent)
  recomputeCounts(newAgents)
}

export function updateAgentStatus(agentId: string, status: AgentStatus): void {
  const state = agentStore.get()
  const agent = state.agents.get(agentId)
  if (!agent) return
  const updated = { ...agent, status, updatedAt: Date.now() }
  const newAgents = new Map(state.agents)
  newAgents.set(agentId, updated)
  recomputeCounts(newAgents)
}

export function claimCompletion(agentId: string, evidenceRefs: string[]): void {
  const state = agentStore.get()
  const agent = state.agents.get(agentId)
  if (!agent) return
  const updated = {
    ...agent,
    claimedCompletion: true,
    status: "claimed_done" as AgentStatus,
    evidenceRefs,
    updatedAt: Date.now(),
  }
  const newAgents = new Map(state.agents)
  newAgents.set(agentId, updated)
  recomputeCounts(newAgents)
}

export function verifyCompletion(agentId: string, passed: boolean, evidence: string): void {
  const state = agentStore.get()
  const agent = state.agents.get(agentId)
  if (!agent) return
  const updated = {
    ...agent,
    isClaimVerified: passed,
    status: passed ? "verified_done" as AgentStatus : "failed" as AgentStatus,
    verificationStatus: passed ? "passed" as const : "failed" as const,
    evidenceRefs: [...agent.evidenceRefs, evidence],
    updatedAt: Date.now(),
  }
  const newAgents = new Map(state.agents)
  newAgents.set(agentId, updated)
  recomputeCounts(newAgents)
}

function recomputeCounts(agents: Map<string, AgentInfo>): void {
  const arr = Array.from(agents.values())
  agentStore.set({
    agents,
    activeCount: arr.filter(a => a.status !== "idle" && a.status !== "verified_done" && a.status !== "abandoned").length,
    blockedCount: arr.filter(a => a.status === "blocked").length,
    claimedDoneCount: arr.filter(a => a.status === "claimed_done").length,
    verifiedDoneCount: arr.filter(a => a.status === "verified_done").length,
  })
}

export function getAgent(agentId: string): AgentInfo | undefined {
  return agentStore.get().agents.get(agentId)
}

/**
 * Check if agent's claimed completion is verified.
 * Critical rule: claimed_done !== verified_done.
 */
export function isCompletionVerified(agentId: string): boolean {
  const agent = getAgent(agentId)
  return agent?.isClaimVerified ?? false
}

/**
 * Get agents that have touched the same files.
 */
export function getAgentsTouchingFiles(files: string[]): AgentInfo[] {
  return Array.from(agentStore.get().agents.values())
    .filter(a => a.currentFiles.some(f => files.includes(f)))
}
