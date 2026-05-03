/**
 * Window registry.
 * Centralizes default window sizes and positions for apps.
 * Replaces hardcoded window size mappings in stores/app-launcher.tsx.
 */

import type { AppDefinition } from "@/platform/types/app-definition"

export interface WindowSpec {
  defaultWidth: number
  defaultHeight: number
  minWidth?: number
  minHeight?: number
  resizable?: boolean
}

const DEFAULT_SPEC: WindowSpec = {
  defaultWidth: 600,
  defaultHeight: 400,
  minWidth: 300,
  minHeight: 200,
  resizable: true,
}

const registry: Map<string, WindowSpec> = new Map()

// Initialize builtin window specs
const BUILTIN_SPECS: Record<string, WindowSpec> = {
  terminal: { defaultWidth: 700, defaultHeight: 450 },
  "code-runner": { defaultWidth: 800, defaultHeight: 500 },
  "db-explorer": { defaultWidth: 700, defaultHeight: 450 },
  "network-monitor": { defaultWidth: 600, defaultHeight: 400 },
  "resource-monitor": { defaultWidth: 700, defaultHeight: 500 },
  "file-browser": { defaultWidth: 600, defaultHeight: 400 },
  "git-sync": { defaultWidth: 600, defaultHeight: 400 },
  "web-server": { defaultWidth: 600, defaultHeight: 400 },
  "compute-node": { defaultWidth: 700, defaultHeight: 500 },
  contacts: { defaultWidth: 560, defaultHeight: 460 },
  calling: { defaultWidth: 340, defaultHeight: 480 },
  chat: { defaultWidth: 580, defaultHeight: 460 },
  "ai-assistant": { defaultWidth: 500, defaultHeight: 550 },
  "workflow-builder": { defaultWidth: 900, defaultHeight: 600 },
  wallet: { defaultWidth: 360, defaultHeight: 520 },
  calculator: { defaultWidth: 300, defaultHeight: 420 },
  help: { defaultWidth: 420, defaultHeight: 480 },
  "app-store": { defaultWidth: 700, defaultHeight: 500 },
  "app-studio": { defaultWidth: 800, defaultHeight: 600 },
  gmail: { defaultWidth: 700, defaultHeight: 500 },
  settings: { defaultWidth: 600, defaultHeight: 500 },
  "wasm-hello": { defaultWidth: 700, defaultHeight: 450 },
  "wasm-calculator": { defaultWidth: 700, defaultHeight: 450 },
}

// Initialize registry
for (const [appId, spec] of Object.entries(BUILTIN_SPECS)) {
  registry.set(appId, spec)
}

export function registerWindowSpec(appId: string, spec: WindowSpec): void {
  registry.set(appId, spec)
}

export function getWindowSpec(appId: string): WindowSpec {
  return registry.get(appId) || DEFAULT_SPEC
}

export function getDefaultSize(appId: string): { width: number; height: number } {
  const spec = getWindowSpec(appId)
  return { width: spec.defaultWidth, height: spec.defaultHeight }
}

/**
 * Update the window spec for an app definition if not already set.
 */
export function enrichAppWithWindowSpec(app: AppDefinition): AppDefinition {
  const spec = getWindowSpec(app.appId)
  return { ...app, displayMetadata: { ...app.displayMetadata, windowSpec: spec } }
}
