import type React from "react"
import {
  appSurfacesStore,
  openAppSurface,
  closeAppSurface,
  addLog,
  pendingGateStore,
  type AppSurfaceDef,
} from "./desktop-store"
import { getBuiltinApp, BUILTIN_ICON_MAP } from "@/platform/registries/builtin-app-registry"
import { getAppSurfaceSpec, getDefaultSurfaceSize } from "@/platform/registries/app-surface-registry"
import { createAppLaunchPlan } from "@/platform/runtime/app-manager"
import { isAppInstalled } from "@/stores/installed-apps-store"
import { getMissingCapabilities } from "@/stores/local-capability-grants-store"
import type { AppDefinition } from "@/platform/types/app-definition"

export function getAppIcon(appId: string): React.ReactNode {
  const app = getBuiltinApp(appId)
  if (app) {
    return BUILTIN_ICON_MAP[app.iconId] || BUILTIN_ICON_MAP["wasm-generic"]
  }
  return <div className="h-4 w-4 rounded bg-primary/20" />
}

export function launchApp(app: AppDefinition, component?: React.ReactNode): AppSurfaceDef | null {
  if (!component && !isAppInstalled(app.appId)) {
    addLog("warning", `${app.name} is not installed`)
    return null
  }

  if (!component) {
    const missing = getMissingCapabilities(app.appId, app.requiredCapabilityIds)
    if (missing.length > 0) {
      pendingGateStore.set({ app, blocked: missing })
      addLog("warning", `${app.name} requires permission: ${missing.join(", ")}`)
      return null
    }

    const plan = createAppLaunchPlan(app, { launchApp })
    component = plan.component
    addLog("info", `${app.name} runtime: ${plan.runtime}`)
  }

  const surfaces = appSurfacesStore.get()
  const surfaceId = `surface-${app.appId}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
  const spec = getAppSurfaceSpec(app.appId)

  const surface: AppSurfaceDef = {
    id: surfaceId,
    appId: app.appId,
    title: app.name,
    icon: getAppIcon(app.appId),
    component,
    kind: spec.kind,
    dismissOnOutsideClick: spec.dismissOnOutsideClick,
    preferredSlot: spec.preferredSlot,
    defaultSize: getDefaultSurfaceSize(app.appId),
    defaultPosition: { x: 0, y: 0 },
  }

  openAppSurface(surface)
  addLog("success", `${spec.kind === "pinned-widget" ? "Pinned" : "Opened"} ${app.name}`)

  // Temporary first cut: pinned widget surfaces still open as overlays until the
  // slot renderer is wired. Keeping the kind on the surface makes the next step
  // unambiguous without keeping old window terminology alive.
  if (surface.kind === "pinned-widget" && surfaces.some((candidate) => candidate.appId === app.appId)) {
    addLog("info", `${app.name} already has an active surface`)
  }

  return surface
}

export function launchAppById(appId: string, component?: React.ReactNode): AppSurfaceDef | null {
  const app = getBuiltinApp(appId)
  if (!app) return null
  return launchApp(app, component)
}

export function handleCloseAppSurface(surfaceId: string) {
  const closed = closeAppSurface(surfaceId)
  if (closed) {
    addLog("info", `Closed ${closed.title}`)
  }
  return closed
}

/** @deprecated Use handleCloseAppSurface. */
export const handleCloseWindow = handleCloseAppSurface
