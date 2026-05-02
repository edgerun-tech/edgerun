/**
 * Assistant session management.
 * Replaces stores/ai-chat-store.ts.
 * Uses platform stores for context, not manual aggregation.
 */

import { atom, computed } from "nanostores"
import type { AssistantMessage, AssistantSession, AssistantToolCall } from "./assitant-types"
import { getDashboardMode } from "@/platform/runtime/dashboard-mode"

const STORAGE_KEY = "edgerun_assitant_session"

function loadFromStorage(): AssistantMessage[] {
  if (typeof window === "undefined") return []
  const raw = localStorage.getItem(STORAGE_KEY)
  if (!raw) return []
  try {
    const msgs = JSON.parse(raw) as AssistantMessage[]
    return msgs.filter(m =>
      m.id && m.role && m.content && typeof m.timestamp === "number"
    )
  } catch {
    return []
  }
}

function saveToStorage(messages: AssistantMessage[]) {
  if (typeof window === "undefined") return
  localStorage.setItem(STORAGE_KEY, JSON.stringify(messages))
}

export const assistantSession = atom<AssistantSession>({
  messages: [],
  isLoading: false,
  context: {},
})

if (typeof window !== "undefined") {
  const stored = loadFromStorage()
  assistantSession.set({
    messages: stored,
    isLoading: false,
    context: {},
  })
}

export function addMessage(
  role: "user" | "assitant",
  content: string,
  toolCalls?: AssistantToolCall[],
): AssistantMessage {
  const session = assistantSession.get()
  const msg: AssistantMessage = {
    id: `msg-${Date.now()}-${Math.random().toString(36).slice(2)}`,
    role,
    content,
    timestamp: Date.now(),
    toolCalls,
  }
  const newMessages = [...session.messages, msg]
  assistantSession.set({ ...session, messages: newMessages })
  saveToStorage(newMessages)
  return msg
}

export function setLoading(loading: boolean) {
  const session = assistantSession.get()
  assistantSession.set({ ...session, isLoading: loading })
}

export function clearSession() {
  assistantSession.set({ messages: [], isLoading: false, context: {} })
  saveToStorage([])
}

export function getMessages(): AssistantMessage[] {
  return assistantSession.get().messages
}

export function isEmpty(): boolean {
  return assistantSession.get().messages.length === 0
}

/**
 * Build context from platform stores only.
 * No fake data. Uses real platform state.
 */
export function buildPlatformContext(): string {
  const mode = getDashboardMode()

  if (mode === "demo") {
    return `Mode: Demo — some data is simulated.`
  }

  // Import platform stores lazily to avoid SSR issues
  const { nodeStore } = require("@/platform/state/node-store")
  const { appStore } = require("@/platform/state/app-store")
  const { capabilityStore } = require("@/platform/state/capability-store")
  const { windowsStore } = require("@/stores/desktop-store")

  const node = nodeStore.get()
  const apps = appStore.get()
  const caps = capabilityStore.get()
  const windows = windowsStore.get()

  const runningApps = windows
    .map((w: any) => w.appId)
    .filter((id: string) => id !== "app-store" && id !== "app-studio")

  const appList = Array.from(apps.apps.values())
    .slice(0, 10)
    .map((a: any) => `- ${a.name || a.appId} (${a.version || "unknown"})`)
    .join("\n")

  const capList = Array.from(caps.descriptors.values())
    .slice(0, 8)
    .map((c: any) => `- ${c.capabilityId}: ${c.description || c.capabilityType}`)
    .join("\n")

  return `
## Session Context

**Mode**: ${mode}
**Message Count**: ${assitantSession.get().messages.length}

## Node Status

- **Node ID**: ${node.currentNode?.nodeId || "Not registered"}
- **Health**: ${node.currentNode?.health || "Unknown"}
- **Runtime**: ${node.currentNode?.runtimeVersion || "Unknown"}

## Running Applications (${runningApps.length})

${runningApps.length > 0 ? runningApps.map((id: string) => `- ${id}`).join("\n") : "No apps running"}

## Installed Apps

${appList || "No apps installed"}

## Available Capabilities

${capList || "No capabilities available"}
`
}
