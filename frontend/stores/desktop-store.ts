import { atom, computed } from "nanostores"
import { persistentAtom } from "@nanostores/persistent"
import type { AppDefinition } from "@/platform/types/app-definition"

export interface LogEntry {
  id: string
  timestamp: Date
  type: "info" | "success" | "warning" | "error" | "system"
  message: string
}

export type AppSurfaceKind = "overlay" | "pinned-widget"
export type AppSurfaceSlot = "left-top" | "left-bottom" | "right-top" | "right-bottom"

export interface AppSurfaceDef {
  id: string
  appId: string
  title: string
  icon: React.ReactNode
  component: React.ReactNode
  kind: AppSurfaceKind
  dismissOnOutsideClick: boolean
  preferredSlot?: AppSurfaceSlot
  defaultSize?: { width: number; height: number }
  /** @deprecated Draggable window positioning is legacy. App surfaces are layout-managed. */
  defaultPosition?: { x: number; y: number }
}

/** @deprecated Use AppSurfaceDef. Kept as a transition alias for older launcher call sites. */
export type OpenWindowDef = AppSurfaceDef

/** @deprecated Draggable window layout is legacy. */
export interface WindowLayoutState {
  position: { x: number; y: number }
  size: { width: number; height: number }
  maximized: boolean
}

/** @deprecated Draggable window layout is legacy. Kept only so old components fail less loudly during migration. */
export const windowLayoutStore = persistentAtom<Record<string, WindowLayoutState>>(
  "edgerun:legacyWindowLayout",
  {},
  {
    encode: JSON.stringify,
    decode: JSON.parse,
  }
)

/** @deprecated Stage manager is no longer part of the desktop model. Apps open as overlays or pinned surfaces. */
export const stageModeStore = persistentAtom("edgerun:legacyStageMode", false, {
  encode: String,
  decode: () => false,
})

export const appSurfacesStore = atom<AppSurfaceDef[]>([])
export const appSurfaceOrderStore = atom<string[]>([])
export const focusedAppSurfaceStore = atom<string | null>(null)

/** @deprecated Use appSurfacesStore. */
export const windowsStore = appSurfacesStore
/** @deprecated Use appSurfaceOrderStore. */
export const windowOrderStore = appSurfaceOrderStore
/** @deprecated Use focusedAppSurfaceStore. */
export const focusedWindowStore = focusedAppSurfaceStore

export const widgetVisibleStore = persistentAtom("edgerun:widgetVisible", false, {
  encode: String,
  decode: (v) => v === "true",
})

/** @deprecated Old conky layer was merged into XrayDesktopSurface responsive metrics. */
export const desktopTelemetryVisibleStore = persistentAtom("edgerun:legacyDesktopTelemetryVisible", true, {
  encode: String,
  decode: (v) => v !== "false",
})

export const terminalLogsStore = atom<LogEntry[]>([])

export const systemStatsStore = atom({
  nodeCount: 12,
  activeSessions: 3,
  ramUsage: { used: 4.2, total: 8 },
  isConnected: true,
})

export const hasBootedStore = atom(false)

export interface PendingGate {
  app: AppDefinition
  blocked: string[]
}
export const pendingGateStore = atom<PendingGate | null>(null)

export const runningAppsStore = computed(appSurfacesStore, (surfaces) =>
  surfaces.map((surface) => surface.appId)
)

export function addLog(type: LogEntry["type"], message: string) {
  terminalLogsStore.set([...terminalLogsStore.get(), {
    id: `log-${Date.now()}-${Math.random()}`,
    timestamp: new Date(),
    type,
    message,
  }])
}

export function openAppSurface(surface: AppSurfaceDef) {
  const existing = appSurfacesStore.get().filter((candidate) => candidate.id !== surface.id)
  appSurfacesStore.set([...existing, surface])
  appSurfaceOrderStore.set([...appSurfaceOrderStore.get().filter((id) => id !== surface.id), surface.id])
  focusedAppSurfaceStore.set(surface.id)
}

/** @deprecated Use openAppSurface. */
export const openWindow = openAppSurface

export function closeAppSurface(surfaceId: string) {
  const surfaces = appSurfacesStore.get()
  const closed = surfaces.find((surface) => surface.id === surfaceId)
  appSurfacesStore.set(surfaces.filter((surface) => surface.id !== surfaceId))
  appSurfaceOrderStore.set(appSurfaceOrderStore.get().filter((id) => id !== surfaceId))

  if (focusedAppSurfaceStore.get() === surfaceId) {
    const remainingOrder = appSurfaceOrderStore.get().filter((id) => id !== surfaceId)
    focusedAppSurfaceStore.set(remainingOrder[remainingOrder.length - 1] || null)
  }

  return closed
}

/** @deprecated Use closeAppSurface. */
export const closeWindow = closeAppSurface

export function focusAppSurface(surfaceId: string) {
  focusedAppSurfaceStore.set(surfaceId)
  appSurfaceOrderStore.set(
    [...appSurfaceOrderStore.get().filter((id) => id !== surfaceId), surfaceId]
  )
}

/** @deprecated Use focusAppSurface. */
export const focusWindow = focusAppSurface

/** @deprecated Draggable window layout is legacy. */
export function saveWindowLayout(appId: string, layout: WindowLayoutState) {
  const store = windowLayoutStore.get()
  windowLayoutStore.set({ ...store, [appId]: layout })
}

/** @deprecated Draggable window layout is legacy. */
export function getWindowLayout(appId: string): WindowLayoutState | null {
  return windowLayoutStore.get()[appId] || null
}
