import type React from "react"
import { windowsStore, openWindow, closeWindow, addLog, pendingGateStore, type OpenWindowDef } from "./desktop-store"
import { getBuiltinApp, BUILTIN_ICON_MAP } from "@/platform/registries/builtin-app-registry"
import { getDefaultSize } from "@/platform/registries/window-registry"
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

export function launchApp(app: AppDefinition, component?: React.ReactNode): OpenWindowDef | null {
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

  const windows = windowsStore.get()
  const offset = (windows.length % 8) * 30
  const position = { x: 150 + offset, y: 80 + offset }
  const windowId = `window-${app.appId}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`

  const defaultSize = getDefaultSize(app.appId)

  const win: OpenWindowDef = {
    id: windowId,
    appId: app.appId,
    title: app.name,
    icon: getAppIcon(app.appId),
    component,
    defaultPosition: position,
    defaultSize,
  }

  openWindow(win)
  addLog("success", `Launched ${app.name}`)
  return win
}

export function launchAppById(appId: string, component?: React.ReactNode): OpenWindowDef | null {
  const app = getBuiltinApp(appId)
  if (!app) return null
  return launchApp(app, component)
}

export function handleCloseWindow(windowId: string) {
  const closed = closeWindow(windowId)
  if (closed) {
    addLog("info", `Closed ${closed.title}`)
  }
  return closed
}
