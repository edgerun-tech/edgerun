import { windowsStore, windowOrderStore, focusedWindowStore, openWindow, closeWindow, addLog, systemStatsStore, type OpenWindowDef } from "./desktop-store"
import { getBuiltinApp, listBuiltinApps } from "@/platform/registries/builtin-app-registry"
import { getComponent } from "@/platform/registries/component-registry"
import { getDefaultSize } from "@/platform/registries/window-registry"
import { getDashboardMode, isDemoMode } from "@/platform/runtime/dashboard-mode"
import { Terminal } from "@/components/os/terminal"
import { CodeRunner } from "@/components/os/code-runner"
import { ResourceMonitor } from "@/components/os/resource-monitor"
import { AppStore as AppStoreComponent } from "@/components/os/app-store"
import { ContactsApp } from "@/components/os/contacts-app"
import { CallingApp } from "@/components/os/calling-app"
import { DemoChatApp as ChatApp } from "@/components/os/chat-app"
import { WalletApp } from "@/components/os/wallet-app"
import { CalculatorApp } from "@/components/os/calculator-app"
import { HelpApp } from "@/components/os/help-app"
import { AIAssistant } from "@/components/os/ai-assitant"
import { WorkflowBuilder } from "@/components/os/workflow-builder"
import { FileManager } from "@/components/os/file-manager"
import { SettingsApp } from "@/components/os/settings-app"
import { BUILTIN_ICON_MAP } from "@/platform/registries/builtin-app-registry"
import type { AppDefinition } from "@/platform/types/app-definition"

// Map appId → component for builtin apps
// Centralized here; component-registry owns the declarative mapping.
function resolveComponent(app: AppDefinition): React.ReactNode {
  // WASM apps
  if (app.kind === "wasm" || app.wasmUrl) {
    if (app.wasmUrl) {
      return <div className="p-4 text-muted-foreground">WASM app: {app.name}</div>
    }
    return <div className="p-4 text-muted-foreground">WASM URL not configured</div>
  }

  // Builtin apps by appId
  switch (app.appId) {
    case "terminal":
      return <Terminal logs={[]} onCommand={() => {}} />
    case "code-runner":
      return <CodeRunner />
    case "resource-monitor":
      return <ResourceMonitor />
    case "app-store":
      return (
        <AppStoreComponent
          onLaunchApp={(a) => launchApp(a)}
          onAppBlocked={() => {}}
          runningApps={[]}
        />
      )
    case "contacts":
      return <ContactsApp />
    case "calling":
      return <CallingApp />
    case "chat":
      return <ChatApp />
    case "wallet":
      return <WalletApp />
    case "calculator":
      return <CalculatorApp />
    case "help":
      return <HelpApp />
    case "ai-assistant":
      return <AIAssistant />
    case "workflow-builder":
      return <WorkflowBuilder onClose={() => {}} />
    case "file-browser":
      return <FileManager />
    case "settings":
      return <SettingsApp />
    case "app-studio":
      return <div className="p-4 text-muted-foreground">App Studio coming soon</div>
    default: {
      // Try component registry
      const registered = getComponent(app.componentKey || app.appId)
      if (registered) {
        const Comp = registered.component
        return <Comp />
      }
      return <div className="p-4 text-muted-foreground">App not implemented</div>
    }
  }
}

export function getAppIcon(appId: string): React.ReactNode {
  const app = getBuiltinApp(appId)
  if (app) {
    return BUILTIN_ICON_MAP[app.iconId] || BUILTIN_ICON_MAP["wasm-generic"]
  }
  // Fallback
  return <div className="h-4 w-4 rounded bg-primary/20" />
}

export function launchApp(app: AppDefinition, component?: React.ReactNode): OpenWindowDef {
  if (!component) {
    component = resolveComponent(app)
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

  // NOTE: Real RAM tracking must come from node runtime, not fake values.
  // The old parseFloat(app.ram) / 1000 update is removed.
  // RAM usage should be tracked via platform runtime store.

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
