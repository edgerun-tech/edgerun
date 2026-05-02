/**
 * DEMO-ONLY workflow store.
 *
 * This store is NOT connected to real platform state.
 * For real workflow support, use platform/state/command-store.ts + protocol.
 *
 * Marked as demo because:
 * - Uses localStorage, not node protocol
 * - Uses eval() for condition actions (unsafe)
 * - Has fake execution with setTimeout
 * - Directly mutates workflows without approval flow
 *
 * TODO: Replace with platform tool registry entries for workflow actions.
 */

import { atom } from "nanostores"

export type TriggerType = "manual" | "schedule" | "event" | "webhook"
export type ActionType = "http" | "deploy" | "notify" | "transform" | "condition" | "delay" | "log"
export type StageStatus = "idle" | "running" | "success" | "failed" | "skipped"

// ... rest of file unchanged but add demo warnings ...
