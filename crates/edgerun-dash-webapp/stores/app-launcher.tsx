import { windowsStore, windowOrderStore, focusedWindowStore, openWindow, closeWindow, addLog, systemStatsStore, type OpenWindowDef } from "./desktop-store"
import { availableApps, type AppDefinition } from "@/components/os/app-store"
import {
  Grid3x3,
  Terminal as TerminalIcon,
  Code2,
  Database,
  Network,
  FileText,
  GitBranch,
  Globe as GlobeIcon,
  Cpu,
  HelpCircle,
  Users,
  Phone,
  MessageSquare,
  Wallet,
  Calculator,
  Activity,
  Sparkles,
  Workflow,
} from "lucide-react"
import { Terminal, generateMockLogs } from "@/components/os/terminal"
import { CodeRunner } from "@/components/os/code-runner"
import { ResourceMonitor } from "@/components/os/resource-monitor"
import { AppStore } from "@/components/os/app-store"
import { ContactsApp } from "@/components/os/contacts-app"
import { CallingApp } from "@/components/os/calling-app"
import { ChatApp } from "@/components/os/chat-app"
import { WalletApp } from "@/components/os/wallet-app"
import { CalculatorApp } from "@/components/os/calculator-app"
import { HelpApp } from "@/components/os/help-app"
import { AIAssistant } from "@/components/os/ai-assistant"
import { WorkflowBuilder } from "@/components/os/workflow-builder"
// AppStudio removed - will be rebuilt using platform services

export function getAppIcon(appId: string): React.ReactNode {
  switch (appId) {
    case "app-store": return <Grid3x3 className="h-4 w-4" />
    case "terminal": return <TerminalIcon className="h-4 w-4" />
    case "code-runner": return <Code2 className="h-4 w-4" />
    case "db-explorer": return <Database className="h-4 w-4" />
    case "network-monitor": return <Network className="h-4 w-4" />
    case "file-browser": return <FileText className="h-4 w-4" />
    case "git-sync": return <GitBranch className="h-4 w-4" />
    case "web-server": return <GlobeIcon className="h-4 w-4" />
    case "compute-node": return <Cpu className="h-4 w-4" />
    case "contacts": return <Users className="h-4 w-4" />
    case "calling": return <Phone className="h-4 w-4" />
    case "chat": return <MessageSquare className="h-4 w-4" />
    case "wallet": return <Wallet className="h-4 w-4" />
    case "calculator": return <Calculator className="h-4 w-4" />
    case "help": return <HelpCircle className="h-4 w-4" />
    case "resource-monitor": return <Activity className="h-4 w-4" />
    case "ai-assistant": return <Sparkles className="h-4 w-4" />
    case "workflow-builder": return <Workflow className="h-4 w-4" />
    case "wasm-hello": return <Cpu className="h-4 w-4" />
    case "wasm-calculator": return <Calculator className="h-4 w-4" />
    default: return <Grid3x3 className="h-4 w-4" />
  }
}

export function buildAppComponent(app: AppDefinition): React.ReactNode {
  switch (app.id) {
    case "terminal":
      return <Terminal logs={[]} onCommand={() => {}} />
    case "code-runner":
      return <CodeRunner />
    case "resource-monitor":
      return <ResourceMonitor />
    case "app-store":
      return <AppStore
        onLaunchApp={(app) => launchApp(app)}
        onAppBlocked={() => {}}
        runningApps={[]}
        guestCtx={{ hasIdentity: false, capabilities: [] }}
      />
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
    case "app-studio":
      return <div className="p-4 text-muted-foreground">App Studio coming soon</div>
    case "wasm-hello":
    case "wasm-calculator":
      if (app.wasmUrl) {
        return <WasmAppWindow wasmUrl={app.wasmUrl} appName={app.name} />
      }
      return <div className="p-4 text-muted-foreground">WASM URL not configured</div>
    default:
      if (app.isWasm && app.wasmUrl) {
        return <WasmAppWindow wasmUrl={app.wasmUrl} appName={app.name} />
      }
      return <div className="p-4 text-muted-foreground">App not implemented</div>
  }
}

export function launchApp(app: AppDefinition, component?: React.ReactNode): OpenWindowDef {
  if (!component) {
    component = buildAppComponent(app)
  }
  const windows = windowsStore.get()
  const offset = (windows.length % 8) * 30
  const position = { x: 150 + offset, y: 80 + offset }
  const windowId = `window-${app.id}-${Date.now()}`

  let defaultSize = { width: 600, height: 400 }
  switch (app.id) {
    case "terminal": defaultSize = { width: 700, height: 450 }; break
    case "code-runner": defaultSize = { width: 800, height: 500 }; break
    case "contacts": defaultSize = { width: 560, height: 460 }; break
    case "calling": defaultSize = { width: 340, height: 480 }; break
    case "chat": defaultSize = { width: 580, height: 460 }; break
    case "wallet": defaultSize = { width: 360, height: 520 }; break
    case "calculator": defaultSize = { width: 300, height: 420 }; break
    case "help": defaultSize = { width: 420, height: 480 }; break
    case "ai-assistant": defaultSize = { width: 500, height: 550 }; break
    case "workflow-builder": defaultSize = { width: 900, height: 600 }; break
    case "wasm-hello": defaultSize = { width: 700, height: 450 }; break
    default: if (app.isWasm) defaultSize = { width: 700, height: 450 }; break
  }

  const win: OpenWindowDef = {
    id: windowId,
    appId: app.id,
    title: app.name,
    icon: getAppIcon(app.id),
    component,
    defaultPosition: position,
    defaultSize,
  }

  openWindow(win)
  addLog("success", `Launched ${app.name}`)

  const ramIncrease = parseFloat(app.ram) / 1000
  const stats = systemStatsStore.get()
  systemStatsStore.set({
    ...stats,
    ramUsage: { ...stats.ramUsage, used: Math.min(stats.ramUsage.total, stats.ramUsage.used + ramIncrease) },
    activeSessions: stats.activeSessions + 1,
  })

  return win
}

export function launchAppById(appId: string, component: React.ReactNode): OpenWindowDef | null {
  const app = availableApps.find((a) => a.id === appId)
  if (!app) return null
  return launchApp(app, component)
}

export function handleCloseWindow(windowId: string) {
  const closed = closeWindow(windowId)
  if (closed) {
    addLog("info", `Closed ${closed.title}`)
    const app = availableApps.find((a) => a.id === closed.appId)
    if (app) {
      const stats = systemStatsStore.get()
      const ramDecrease = parseFloat(app.ram) / 1000
      systemStatsStore.set({
        ...stats,
        ramUsage: { ...stats.ramUsage, used: Math.max(0, stats.ramUsage.used - ramDecrease) },
        activeSessions: Math.max(0, stats.activeSessions - 1),
      })
    }
  }
  return closed
}
