"use client"

import { AssistantApp } from "@/components/assitant/AssistantApp"

/**
 * Thin wrapper around the platform assistant.
 * The old ai-assitant.tsx logic has been moved to platform/assitant/.
 */
export function AIAssistant() {
  return <AssistantApp />
}
