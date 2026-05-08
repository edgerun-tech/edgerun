import type React from "react"
import {
  appSurfacesStore,
  openAppSurface,
  closeAppSurface,
  focusAppSurface,
  addLog,
  pendingGateStore,
  type AppSurfaceDef,
} from "./desktop-store"
import { getBuiltinApp, BUILTIN_ICON_MAP } from "@/platform/registries/builtin-app-registry"
import { getCatalogApp } from "@/platform/registries/app-catalog-registry"
import { getAppSurfaceSpec, getDefaultSurfaceVariant } from "@/platform/registries/app-surface-registry"
import { createAppLaunchPlan } from "@/platform/runtime/app-manager"
import { isAppInstalled } from "@/stores/installed-apps-store"
import { getMissingCapabilities } from "@/stores/local-capability-grants-store"
import type { AppDefinition } from "@/platform/types/app-definition"

function notifyAppSurfaceOpening(appId: string) {
  if (typeof window === "undefined") return
  window.dispatchEvent(new CustomEvent("edgerun:app-surface-opening", { detail: { appId } }))
}

export function getAppIcon(appId: string): React.ReactNode {
  const app = getBuiltinApp(appId)
  if (app) {
    return BUILTIN_ICON_MAP[app.iconId] || BUILTIN_ICON_MAP["wasm-generic"]
  }
  const catalogApp = getCatalogApp(appId)
  if (catalogApp) {
    return BUILTIN_ICON_MAP[catalogApp.iconId] || BUILTIN_ICON_MAP["wasm-generic"]
  }
  return <div className="h-4 w-4 rounded bg-primary/20" />
}

export function launchApp(app: AppDefinition, component?: React.ReactNode): AppSurfaceDef | null {
  if (!component && !isAppInstalled(app.appId)) {
    addLog("warning", `${app.name} is not installed`)
    return null
  }

  const spec = getAppSurfaceSpec(app.appId)
  const surfaces = appSurfacesStore.get()

  if (!component) {
    const missing = getMissingCapabilities(app.appId, app.requiredCapabilityIds)
    if (missing.length > 0) {
      pendingGateStore.set({ app, blocked: missing })
      addLog("warning", `${app.name} requires permission: ${missing.join(", ")}`)
      return null
    }
  }

  if (spec.kind === "pinned-widget") {
    const existing = surfaces.find((candidate) => candidate.appId === app.appId && candidate.kind === "pinned-widget")
    if (existing) {
      focusAppSurface(existing.id)
      addLog("info", `${app.name} is already pinned`)
      return existing
    }
  }

  if (spec.kind === "overlay") {
    const existingOverlays = surfaces.filter((s) => s.kind === "overlay")
    for (const overlay of existingOverlays) {
      closeAppSurface(overlay.id)
    }
  }

  if (!component) {
    const plan = createAppLaunchPlan(app, { launchApp })
    component = plan.component
    addLog("info", `${app.name} runtime: ${plan.runtime}`)
  }

  notifyAppSurfaceOpening(app.appId)

  const surfaceId = `surface-${app.appId}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`

  const surface: AppSurfaceDef = {
    id: surfaceId,
    appId: app.appId,
    title: app.name,
    icon: getAppIcon(app.appId),
    component,
    kind: spec.kind,
    variant: getDefaultSurfaceVariant(app.appId),
    dismissOnOutsideClick: spec.dismissOnOutsideClick,
    preferredSlot: spec.preferredSlot,
  }

  openAppSurface(surface)
  addLog("success", `${spec.kind === "pinned-widget" ? "Pinned" : "Opened"} ${app.name}`)
  return surface
}

export function launchAppById(appId: string, component?: React.ReactNode): AppSurfaceDef | null {
  const app = getBuiltinApp(appId) ?? getCatalogApp(appId)
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
