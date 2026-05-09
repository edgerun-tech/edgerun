/**
 * App surface registry.
 *
 * Canonical place for app presentation metadata. The desktop no longer owns
 * draggable window placement. Apps open as layout-managed surfaces over the
 * xray workspace, or as pinned widgets in fixed slots.
 *
 * Surface variants are semantic. The host owns responsive sizing.
 */

import type { AppSurfaceKind, AppSurfaceSlot, AppSurfaceVariant } from "@/stores/desktop-store"

export interface AppSurfaceSpec {
  kind: AppSurfaceKind
  variant: AppSurfaceVariant
  dismissOnOutsideClick: boolean
  preferredSlot?: AppSurfaceSlot
}

const DEFAULT_SPEC: AppSurfaceSpec = { kind: "overlay", variant: "standard", dismissOnOutsideClick: true }

const registry = new Map<string, AppSurfaceSpec>()

const BUILTIN_SPECS: Record<string, AppSurfaceSpec> = {
  identity: { kind: "overlay", variant: "standard", dismissOnOutsideClick: true },
  "db-explorer": { kind: "overlay", variant: "standard", dismissOnOutsideClick: true },
  "network-monitor": { kind: "pinned-widget", variant: "widget", dismissOnOutsideClick: false, preferredSlot: "left-top" },
  "file-browser": { kind: "overlay", variant: "standard", dismissOnOutsideClick: true },
  storage: { kind: "overlay", variant: "full", dismissOnOutsideClick: true },
  "git-sync": { kind: "overlay", variant: "standard", dismissOnOutsideClick: true },
  "web-server": { kind: "overlay", variant: "standard", dismissOnOutsideClick: true },
  "compute-node": { kind: "pinned-widget", variant: "widget", dismissOnOutsideClick: false, preferredSlot: "right-top" },
  people: { kind: "overlay", variant: "wide", dismissOnOutsideClick: true },
  contacts: { kind: "overlay", variant: "wide", dismissOnOutsideClick: true },
  calling: { kind: "overlay", variant: "wide", dismissOnOutsideClick: true },
  chat: { kind: "overlay", variant: "wide", dismissOnOutsideClick: true },
  "trust-manager": { kind: "overlay", variant: "wide", dismissOnOutsideClick: true },
  "workflow-builder": { kind: "overlay", variant: "wide", dismissOnOutsideClick: true },
  finances: { kind: "overlay", variant: "full", dismissOnOutsideClick: true },
  wallet: { kind: "overlay", variant: "full", dismissOnOutsideClick: true },
  calculator: { kind: "overlay", variant: "compact", dismissOnOutsideClick: true },
  help: { kind: "overlay", variant: "compact", dismissOnOutsideClick: true },
  "app-store": { kind: "overlay", variant: "wide", dismissOnOutsideClick: true },
  "app-studio": { kind: "overlay", variant: "wide", dismissOnOutsideClick: true },
  gmail: { kind: "overlay", variant: "wide", dismissOnOutsideClick: true },
  settings: { kind: "overlay", variant: "standard", dismissOnOutsideClick: true },
  "wasm-hello": { kind: "overlay", variant: "standard", dismissOnOutsideClick: true },
  "wasm-calculator": { kind: "overlay", variant: "compact", dismissOnOutsideClick: true },
}

for (const [appId, spec] of Object.entries(BUILTIN_SPECS)) {
  registry.set(appId, spec)
}

export function getAppSurfaceSpec(appId: string): AppSurfaceSpec {
  return registry.get(appId) || DEFAULT_SPEC
}

export function getDefaultSurfaceVariant(appId: string): AppSurfaceVariant {
  return getAppSurfaceSpec(appId).variant
}
