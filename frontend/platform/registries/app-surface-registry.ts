/**
 * App surface registry.
 *
 * Canonical place for app presentation metadata. The desktop no longer owns
 * draggable window placement. Apps open as layout-managed surfaces over the
 * xray workspace, or as pinned widgets in fixed slots.
 */

import type { AppDefinition } from "@/platform/types/app-definition"
import type { AppSurfaceKind, AppSurfaceSlot } from "@/stores/desktop-store"

export interface AppSurfaceSpec {
  kind: AppSurfaceKind
  defaultWidth: number
  defaultHeight: number
  dismissOnOutsideClick: boolean
  preferredSlot?: AppSurfaceSlot
}

const DEFAULT_SPEC: AppSurfaceSpec = {
  kind: "overlay",
  defaultWidth: 960,
  defaultHeight: 680,
  dismissOnOutsideClick: true,
}

const registry = new Map<string, AppSurfaceSpec>()

const FINANCE_HUB_SPEC: AppSurfaceSpec = {
  kind: "overlay",
  defaultWidth: 1280,
  defaultHeight: 820,
  dismissOnOutsideClick: true,
}

const BUILTIN_SPECS: Record<string, AppSurfaceSpec> = {
  terminal: { kind: "overlay", defaultWidth: 900, defaultHeight: 620, dismissOnOutsideClick: true },
  "code-runner": { kind: "overlay", defaultWidth: 980, defaultHeight: 680, dismissOnOutsideClick: true },
  "db-explorer": { kind: "overlay", defaultWidth: 940, defaultHeight: 640, dismissOnOutsideClick: true },
  "network-monitor": { kind: "pinned-widget", defaultWidth: 224, defaultHeight: 260, dismissOnOutsideClick: false, preferredSlot: "left-top" },
  "resource-monitor": { kind: "pinned-widget", defaultWidth: 224, defaultHeight: 260, dismissOnOutsideClick: false, preferredSlot: "right-bottom" },
  "file-browser": { kind: "overlay", defaultWidth: 920, defaultHeight: 640, dismissOnOutsideClick: true },
  "git-sync": { kind: "overlay", defaultWidth: 820, defaultHeight: 560, dismissOnOutsideClick: true },
  "web-server": { kind: "overlay", defaultWidth: 820, defaultHeight: 560, dismissOnOutsideClick: true },
  "compute-node": { kind: "pinned-widget", defaultWidth: 224, defaultHeight: 260, dismissOnOutsideClick: false, preferredSlot: "right-top" },
  people: { kind: "overlay", defaultWidth: 980, defaultHeight: 680, dismissOnOutsideClick: true },
  contacts: { kind: "overlay", defaultWidth: 980, defaultHeight: 680, dismissOnOutsideClick: true },
  calling: { kind: "overlay", defaultWidth: 980, defaultHeight: 680, dismissOnOutsideClick: true },
  chat: { kind: "overlay", defaultWidth: 980, defaultHeight: 680, dismissOnOutsideClick: true },
  "trust-manager": { kind: "overlay", defaultWidth: 1120, defaultHeight: 760, dismissOnOutsideClick: true },
  "workflow-builder": { kind: "overlay", defaultWidth: 1120, defaultHeight: 760, dismissOnOutsideClick: true },
  finances: FINANCE_HUB_SPEC,
  wallet: FINANCE_HUB_SPEC,
  calculator: { kind: "overlay", defaultWidth: 420, defaultHeight: 520, dismissOnOutsideClick: true },
  help: { kind: "overlay", defaultWidth: 640, defaultHeight: 620, dismissOnOutsideClick: true },
  "app-store": { kind: "overlay", defaultWidth: 980, defaultHeight: 700, dismissOnOutsideClick: true },
  "app-studio": { kind: "overlay", defaultWidth: 1040, defaultHeight: 720, dismissOnOutsideClick: true },
  gmail: { kind: "overlay", defaultWidth: 980, defaultHeight: 680, dismissOnOutsideClick: true },
  settings: { kind: "overlay", defaultWidth: 960, defaultHeight: 700, dismissOnOutsideClick: true },
  "wasm-hello": { kind: "overlay", defaultWidth: 860, defaultHeight: 580, dismissOnOutsideClick: true },
  "wasm-calculator": { kind: "overlay", defaultWidth: 760, defaultHeight: 560, dismissOnOutsideClick: true },
}

for (const [appId, spec] of Object.entries(BUILTIN_SPECS)) {
  registry.set(appId, spec)
}

export function registerAppSurfaceSpec(appId: string, spec: AppSurfaceSpec): void {
  registry.set(appId, spec)
}

export function getAppSurfaceSpec(appId: string): AppSurfaceSpec {
  return registry.get(appId) || DEFAULT_SPEC
}

export function getDefaultSurfaceSize(appId: string): { width: number; height: number } {
  const spec = getAppSurfaceSpec(appId)
  return { width: spec.defaultWidth, height: spec.defaultHeight }
}

export function enrichAppWithSurfaceSpec(app: AppDefinition): AppDefinition {
  const spec = getAppSurfaceSpec(app.appId)
  return { ...app, displayMetadata: { ...app.displayMetadata, surfaceSpec: spec } }
}
