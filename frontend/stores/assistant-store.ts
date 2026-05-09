"use client"

import { atom } from "nanostores"
import { persistentAtom } from "@nanostores/persistent"

export type AssistantStatus = "checking" | "ready" | "offline"

export type AssistantMessage = {
  id: string
  role: "user" | "assistant"
  content: string
  createdAt?: number
}

export const assistantMessagesStore = persistentAtom<AssistantMessage[]>(
  "edgerun:assistant:messages",
  [],
  {
    encode: JSON.stringify,
    decode: (value) => {
      if (!value) return []
      const parsed = JSON.parse(value) as unknown
      if (!Array.isArray(parsed)) return []
      return parsed.filter((item): item is AssistantMessage => (
        item &&
        typeof item === "object" &&
        "id" in item &&
        "role" in item &&
        "content" in item &&
        typeof item.id === "string" &&
        (item.role === "user" || item.role === "assistant") &&
        typeof item.content === "string" &&
        (!("createdAt" in item) || typeof item.createdAt === "number")
      ))
    },
  },
)

export const assistantLastDurationStore = persistentAtom<number | null>(
  "edgerun:assistant:last-duration-ms",
  null,
  {
    encode: JSON.stringify,
    decode: (value) => {
      if (!value) return null
      const parsed = JSON.parse(value) as unknown
      return typeof parsed === "number" && Number.isFinite(parsed) ? parsed : null
    },
  },
)

export const assistantStatusStore = atom<AssistantStatus>("checking")
export const assistantLoadingStore = atom(false)
export const assistantStartedAtStore = atom<number | null>(null)
export const assistantElapsedMsStore = atom(0)
export const assistantAbortControllerStore = atom<AbortController | null>(null)

function stampMessages(messages: AssistantMessage[]) {
  const now = Date.now()
  return messages.map((message) => message.createdAt ? message : { ...message, createdAt: now })
}

export function updateAssistantMessages(updater: (current: AssistantMessage[]) => AssistantMessage[]) {
  assistantMessagesStore.set(stampMessages(updater(assistantMessagesStore.get())).slice(-12))
}

export function appendAssistantMessage(message: Omit<AssistantMessage, "id">, limit = 12) {
  const id = `${message.role}-${Date.now()}-${Math.random().toString(36).slice(2)}`
  updateAssistantMessages((current) => [
    ...current.slice(-(limit - 1)),
    { ...message, id, createdAt: message.createdAt ?? Date.now() },
  ])
  return id
}

export function formatAssistantMessageTime(createdAt?: number, now = Date.now()) {
  if (!createdAt) return ""
  const elapsedMs = Math.max(0, now - createdAt)
  const elapsedMinutes = Math.floor(elapsedMs / 60_000)
  if (elapsedMinutes < 1) return "now"
  if (elapsedMinutes < 60) return `${elapsedMinutes}m ago`

  const elapsedHours = Math.floor(elapsedMinutes / 60)
  if (elapsedHours < 24) return `${elapsedHours}h ago`

  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(createdAt))
}

export function getLastAssistantUserMessage(messages = assistantMessagesStore.get()): AssistantMessage | null {
  for (let index = messages.length - 1; index >= 0; index--) {
    if (messages[index].role === "user") return messages[index]
  }
  return null
}

export function getAssistantUserMessageContents(messages = assistantMessagesStore.get()): string[] {
  return messages
    .filter((message) => message.role === "user" && message.content.trim())
    .map((message) => message.content.trim())
}

export function hasRetryableAssistantError(messages = assistantMessagesStore.get()): boolean {
  return messages.some((message) => (
    message.role === "assistant" &&
    /^(Error:|Codex bridge is offline\.|Network error\.)/i.test(message.content.trim())
  ))
}

export function clearAssistantMessages() {
  assistantMessagesStore.set([])
}

export function setAssistantStatus(status: AssistantStatus) {
  assistantStatusStore.set(status)
}

export function startAssistantRequest(startedAt = Date.now()) {
  assistantLoadingStore.set(true)
  assistantStartedAtStore.set(startedAt)
  assistantElapsedMsStore.set(0)
  assistantLastDurationStore.set(null)
  assistantStatusStore.set("checking")
}

export function finishAssistantRequest(startedAt: number, status: AssistantStatus) {
  assistantLastDurationStore.set(Date.now() - startedAt)
  assistantStartedAtStore.set(null)
  assistantLoadingStore.set(false)
  assistantAbortControllerStore.set(null)
  assistantStatusStore.set(status)
}

export function setAssistantAbortController(controller: AbortController | null) {
  assistantAbortControllerStore.set(controller)
}

export function abortAssistantRequest() {
  const controller = assistantAbortControllerStore.get()
  if (!controller || controller.signal.aborted) return false
  controller.abort()
  return true
}

export function isAssistantAbortError(error: unknown) {
  return error instanceof DOMException && error.name === "AbortError"
}
