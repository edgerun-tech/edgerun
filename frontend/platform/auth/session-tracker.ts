/**
 * Tracks UI session state.
 */

import { atom, computed } from "nanostores"

export type SessionState = "guest" | "authenticated" | "locked" | "unauthenticated"

export interface SessionTrackerState {
  sessionId: string | null
  sessionState: SessionState
  username: string
  webAuthnAvailable: boolean
  nodeRegistration: {
    nodeId: string
    nodeTarget: string
    username: string
    registeredAtIso: string
  } | null
  lastActivity: string
}

const initialState: SessionTrackerState = {
  sessionId: null,
  sessionState: "guest",
  username: "",
  webAuthnAvailable: false,
  nodeRegistration: null,
  lastActivity: new Date().toISOString(),
}

export const sessionTracker = atom<SessionTrackerState>(initialState)

export const isAuthenticated = computed(
  sessionTracker,
  (s) => s.sessionState === "authenticated",
)

export const isGuest = computed(sessionTracker, (s) => s.sessionState === "guest")

export function setSessionState(state: SessionState): void {
  sessionTracker.set({
    ...sessionTracker.get(),
    sessionState: state,
    lastActivity: new Date().toISOString(),
  })
}

export function setNodeRegistration(reg: SessionTrackerState["nodeRegistration"]): void {
  sessionTracker.set({
    ...sessionTracker.get(),
    nodeRegistration: reg,
  })
}

export function setUsername(name: string): void {
  sessionTracker.set({
    ...sessionTracker.get(),
    username: name,
  })
}

export function updateActivity(): void {
  sessionTracker.set({
    ...sessionTracker.get(),
    lastActivity: new Date().toISOString(),
  })
}

export function getNodeId(): string | null {
  return sessionTracker.get().nodeRegistration?.nodeId ?? null
}

export function getNodeTarget(): string | null {
  return sessionTracker.get().nodeRegistration?.nodeTarget ?? null
}
