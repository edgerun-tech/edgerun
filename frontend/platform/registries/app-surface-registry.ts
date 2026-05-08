/**
 * App surface registry.
 *
 * Canonical place for app presentation metadata. The desktop no longer owns
 * draggable window placement. Apps open as layout-managed surfaces over the
 * xray workspace, or as pinned widgets in fixed slots.
 *
 * Standard sizes:
 *   - sm:  small tools (420×520)
 *   - md:  standard overlay (960×680)
 *   - lg:  wide overlay (1120×760)
 *   - xl:  wide hub/chat (1280×820)
 *   - widget: pinned widget (224×260)
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

const SIZE = { sm: { defaultWidth: 420, defaultHeight: 520 }, md: { defaultWidth: 960, defaultHeight: 680 }, lg: { defaultWidth: 1120, defaultHeight: 760 }, xl: { defaultWidth: 1280, defaultHeight: 820 }, widget: { defaultWidth: 224, defaultHeight: 260 } } as const

const DEFAULT_SPEC: AppSurfaceSpec = { kind: "overlay", defaultWidth: SIZE.md.defaultWidth, defaultHeight: SIZE.md.defaultHeight, dismissOnOutsideClick: true }

const registry = new Map<string, AppSurfaceSpec>()

const BUILTIN_SPECS: Record<string, AppSurfaceSpec> = {
  terminal: { kind: "overlay", ...SIZE.md, dismissOnOutsideClick: true },
  "code-runner": { kind: "overlay", ...SIZE.md, dismissOnOutsideClick: true },
  "db-explorer": { kind: "overlay", ...SIZE.md, dismissOnOutsideClick: true },
  "network-monitor": { kind: "pinned-widget", ...SIZE.widget, dismissOnOutsideClick: false, preferredSlot: "left-top" },
  "file-browser": { kind: "overlay", ...SIZE.md, dismissOnOutsideClick: true },
  storage: { kind: "overlay", ...SIZE.xl, dismissOnOutsideClick: true },
  "git-sync": { kind: "overlay", ...SIZE.md, dismissOnOutsideClick: true },
  "web-server": { kind: "overlay", ...SIZE.md, dismissOnOutsideClick: true },
  "compute-node": { kind: "pinned-widget", ...SIZE.widget, dismissOnOutsideClick: false, preferredSlot: "right-top" },
  people: { kind: "overlay", ...SIZE.lg, dismissOnOutsideClick: true },
  contacts: { kind: "overlay", ...SIZE.lg, dismissOnOutsideClick: true },
  calling: { kind: "overlay", ...SIZE.lg, dismissOnOutsideClick: true },
  chat: { kind: "overlay", ...SIZE.lg, dismissOnOutsideClick: true },
  "trust-manager": { kind: "overlay", ...SIZE.lg, dismissOnOutsideClick: true },
  "workflow-builder": { kind: "overlay", ...SIZE.lg, dismissOnOutsideClick: true },
  finances: { kind: "overlay", ...SIZE.xl, dismissOnOutsideClick: true },
  wallet: { kind: "overlay", ...SIZE.xl, dismissOnOutsideClick: true },
  calculator: { kind: "overlay", ...SIZE.sm, dismissOnOutsideClick: true },
  help: { kind: "overlay", ...SIZE.sm, dismissOnOutsideClick: true },
  "app-store": { kind: "overlay", ...SIZE.lg, dismissOnOutsideClick: true },
  "app-studio": { kind: "overlay", ...SIZE.lg, dismissOnOutsideClick: true },
  gmail: { kind: "overlay", ...SIZE.lg, dismissOnOutsideClick: true },
  settings: { kind: "overlay", ...SIZE.md, dismissOnOutsideClick: true },
  "wasm-hello": { kind: "overlay", ...SIZE.md, dismissOnOutsideClick: true },
  "wasm-calculator": { kind: "overlay", ...SIZE.sm, dismissOnOutsideClick: true },
}

for (const [appId, spec] of Object.entries(BUILTIN_SPECS)) {
  registry.set(appId, spec)
}

export function getAppSurfaceSpec(appId: string): AppSurfaceSpec {
  return registry.get(appId) || DEFAULT_SPEC
}

export function getDefaultSurfaceSize(appId: string): { width: number; height: number } {
  const spec = getAppSurfaceSpec(appId)
  return { width: spec.defaultWidth, height: spec.defaultHeight }
}
