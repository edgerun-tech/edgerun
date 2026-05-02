/**
 * Replaced by platform/assistant/.
 * This file is deprecated. Use platform/assistant/ instead.
 *
 * Kept temporarily for backward compatibility during migration.
 * TODO: Remove once all consumers are migrated.
 */

import { assistantSession, addMessage, setLoading, clearSession, buildPlatformContext } from "@/platform/assistant"

// Re-export for backward compatibility
export const aiChatStore = assistantSession
export { addMessage, setLoading, clearSession }

// getSystemContext is replaced by buildPlatformContext() in platform/assistant/
export function getSystemContext(): string {
  if (typeof window !== "undefined") {
    return buildPlatformContext?.() || "Platform context not available"
  }
  return "Server-side rendering"
}

// Types for backward compatibility
export interface ChatMessage {
  id: string
  role: "user" | "assistant"
  content: string
  timestamp: number
}

export interface ToolCall {
  name: string
  args: Record<string, unknown>
  result?: unknown
}
