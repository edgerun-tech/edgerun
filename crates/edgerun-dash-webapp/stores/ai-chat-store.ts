/**
 * Replaced by platform/assitant/assitant-session.ts.
 * This file is deprecated. Use platform/assitant/ instead.
 *
 * Kept temporarily for backward compatibility during migration.
 * TODO: Remove once all consumers are migrated.
 */

import { assistantSession, addMessage, setLoading, clearSession } from "@/platform/assitant/assitant-session"

// Re-export for backward compatibility
export const aiChatStore = assistantSession
export { addMessage, setLoading, clearSession }

// getSystemContext is replaced by buildPlatformContext() in platform/assitant/assitant-session.ts
export function getSystemContext(): string {
  if (typeof window !== "undefined") {
    const { buildPlatformContext } = require("@/platform/assitant/assitant-session")
    return buildPlatformContext?.() || "Platform context not available"
  }
  return "Server-side rendering"
}

// Types for backward compatibility
export interface ChatMessage {
  id: string
  role: "user" | "assitant"
  content: string
  timestamp: number
}

export interface ToolCall {
  name: string
  args: Record<string, unknown>
  result?: unknown
}
