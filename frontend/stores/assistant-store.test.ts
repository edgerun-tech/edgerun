import { beforeEach, describe, expect, it, vi } from "vitest"
import { cleanTestStorage, useTestStorageEngine as enableTestStorageEngine } from "@nanostores/persistent"

enableTestStorageEngine()

const {
  abortAssistantRequest,
  assistantAbortControllerStore,
  assistantElapsedMsStore,
  assistantLastDurationStore,
  assistantLoadingStore,
  assistantMessagesStore,
  assistantStartedAtStore,
  assistantStatusStore,
  finishAssistantRequest,
  getAssistantUserMessageContents,
  isAssistantAbortError,
  setAssistantAbortController,
  startAssistantRequest,
} = await import("./assistant-store")

describe("assistant store", () => {
  beforeEach(() => {
    cleanTestStorage()
    vi.useRealTimers()
    assistantMessagesStore.set([])
    assistantLoadingStore.set(false)
    assistantStartedAtStore.set(null)
    assistantElapsedMsStore.set(0)
    assistantLastDurationStore.set(null)
    assistantStatusStore.set("checking")
    assistantAbortControllerStore.set(null)
  })

  it("returns trimmed user message contents in display order", () => {
    assistantMessagesStore.set([
      { id: "assistant-1", role: "assistant", content: "Hello" },
      { id: "user-1", role: "user", content: "  first prompt  " },
      { id: "user-blank", role: "user", content: "   " },
      { id: "assistant-2", role: "assistant", content: "Answer" },
      { id: "user-2", role: "user", content: "second prompt" },
    ])

    expect(getAssistantUserMessageContents()).toEqual(["first prompt", "second prompt"])
  })

  it("aborts the current assistant request once", () => {
    expect(abortAssistantRequest()).toBe(false)

    const controller = new AbortController()
    setAssistantAbortController(controller)

    expect(abortAssistantRequest()).toBe(true)
    expect(controller.signal.aborted).toBe(true)
    expect(abortAssistantRequest()).toBe(false)
    expect(assistantAbortControllerStore.get()).toBe(controller)
  })

  it("clears request state when finishing", () => {
    vi.spyOn(Date, "now").mockReturnValue(1_000)

    startAssistantRequest(250)
    setAssistantAbortController(new AbortController())
    finishAssistantRequest(250, "ready")

    expect(assistantLastDurationStore.get()).toBe(750)
    expect(assistantStartedAtStore.get()).toBeNull()
    expect(assistantLoadingStore.get()).toBe(false)
    expect(assistantAbortControllerStore.get()).toBeNull()
    expect(assistantStatusStore.get()).toBe("ready")
  })

  it("recognizes abort errors", () => {
    expect(isAssistantAbortError(new DOMException("Stopped", "AbortError"))).toBe(true)
    expect(isAssistantAbortError(new Error("AbortError"))).toBe(false)
  })
})
