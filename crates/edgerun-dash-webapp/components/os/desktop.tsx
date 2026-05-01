"use client"

import { useCallback, useEffect } from "react"
import { useStore } from "@nanostores/react"
import { cn } from "@/lib/utils"
import { Window } from "./window"
import { TopBar } from "./top-bar"
import { AppStore, availableApps, type AppDefinition } from "./app-store"
import { Terminal, generateMockLogs } from "./terminal"
import { CodeRunner } from "./code-runner"
import { ResourceMonitor } from "./resource-monitor"
import { Globe } from "./globe"
import { AuthOverlay } from "./auth-overlay"
import { HelpApp } from "./help-app"
import { ContactsApp } from "./contacts-app"
import { CallingApp } from "./calling-app"
import { ChatApp } from "./chat-app"
import { WalletApp } from "./wallet-app"
import { CalculatorApp } from "./calculator-app"
import { WasmAppWindow } from "@/components/wasm-app-window"
import { wasmRegistry } from "@/lib/wasm/wasm-registry"
import { StageManager } from "./stage-manager"
import { WidgetPanel } from "./widget-panel"
import { useAuth } from "@/hooks/use-auth"
import { CapabilityGatePrompt } from "@/components/capability-gate-prompt"
import { type GuestContext } from "@/lib/capabilities"
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
  LayoutDashboard,
  PanelRight,
  Activity,
} from "lucide-react"
import {
  windowsStore,
  windowOrderStore,
  focusedWindowStore,
  terminalLogsStore,
  systemStatsStore,
  hasBootedStore,
  pendingGateStore,
  stageModeStore,
  widgetVisibleStore,
  addLog,
  openWindow,
  closeWindow,
  focusWindow,
  type OpenWindowDef,
} from "@/stores/desktop-store"

const getAppIcon = (appId: string) => {
  const icons: Record<string, React.ReactNode> = {
    "app-store": <Grid3x3 className="h-4 w-4" />,
    "terminal": <TerminalIcon className="h-4 w-4" />,
    "code-runner": <Code2 className="h-4 w-4" />,
    "db-explorer": <Database className="h-4 w-4" />,
    "network-monitor": <Network className="h-4 w-4" />,
    "file-browser": <FileText className="h-4 w-4" />,
    "git-sync": <GitBranch className="h-4 w-4" />,
    "web-server": <GlobeIcon className="h-4 w-4" />,
    "compute-node": <Cpu className="h-4 w-4" />,
    "contacts": <Users className="h-4 w-4" />,
    "calling": <Phone className="h-4 w-4" />,
    "chat": <MessageSquare className="h-4 w-4" />,
    "wallet": <Wallet className="h-4 w-4" />,
    "calculator": <Calculator className="h-4 w-4" />,
    "help": <HelpCircle className="h-4 w-4" />,
    "resource-monitor": <Activity className="h-4 w-4" />,
    "wasm-hello": <Cpu className="h-4 w-4" />,
    "wasm-calculator": <Calculator className="h-4 w-4" />,
  }
  return icons[appId] || <Grid3x3 className="h-4 w-4" />
}

function buildAppComponent(
  app: AppDefinition,
  windowId: string
): React.ReactNode {
  const logs = terminalLogsStore.get()
  const stats = systemStatsStore.get()

  switch (app.id) {
    case "terminal":
      return (
        <Terminal
          logs={logs}
          onCommand={(cmd) => {
            addLog("info", `$ ${cmd}`)
            setTimeout(() => {
              if (cmd === "help") {
                addLog("system", "Available commands: help, status, nodes, clear")
              } else if (cmd === "status") {
                addLog("success", "All systems operational")
              } else if (cmd === "nodes") {
                addLog("info", `Connected to ${stats.nodeCount} nodes`)
              } else if (cmd === "clear") {
                terminalLogsStore.set([])
              } else {
                addLog("warning", `Unknown command: ${cmd}`)
              }
            }, 100)
          }}
        />
      )

    case "code-runner":
      return (
        <CodeRunner
          onOutput={(output) => {
            addLog("info", `Code executed: ${output.split("\n")[0]}...`)
          }}
        />
      )

    case "contacts":
      return (
        <ContactsApp
          onCall={(contact) => {
            addLog("info", `Calling ${contact.name}...`)
            launchAppById("calling")
          }}
          onMessage={(contact) => {
            addLog("info", `Opening chat with ${contact.name}`)
            launchAppById("chat")
          }}
        />
      )

    case "calling":
      return <CallingApp />

    case "chat":
      return <ChatApp />

    case "wallet":
      return <WalletApp />

    case "calculator":
      return <CalculatorApp />

    case "help":
      return (
        <HelpApp
          onFinish={() => {
            closeWindow(windowId)
          }}
        />
      )

    case "resource-monitor":
      return (
        <ResourceMonitor
          onLog={(msg) => addLog("info", `[Resource Monitor] ${msg}`)}
        />
      )

      case "wasm-hello":
        return (
          <WasmAppWindow
            wasmUrl={app.wasmUrl || "/edgerun-app.wasm"}
            appName={app.name}
            onLog={(msg) => addLog("info", `[${app.name}] ${msg}`)}
          />
        )

      case "wasm-calculator":
        return (
          <WasmAppWindow
            wasmUrl={app.wasmUrl || "/calculator.wasm"}
            appName={app.name}
            onLog={(msg) => addLog("info", `[${app.name}] ${msg}`)}
          />
        )

    default:
      if (app.isWasm && app.wasmUrl) {
        return (
          <WasmAppWindow
            wasmUrl={app.wasmUrl}
            appName={app.name}
            onLog={(msg) => addLog("info", `[${app.name}] ${msg}`)}
          />
        )
      }
      return (
        <div className="flex h-full items-center justify-center p-6 text-center">
          <div>
            <div className="mb-4 flex justify-center">
              <div className="flex h-16 w-16 items-center justify-center rounded-xl bg-primary/10 text-primary">
                {app.icon}
              </div>
            </div>
            <h3 className="text-lg font-semibold text-foreground">{app.name}</h3>
            <p className="mt-1 text-sm text-muted-foreground">{app.description}</p>
            <div className="mt-4 rounded-lg bg-secondary/50 p-3 text-xs text-muted-foreground">
              <p>This app is a demo placeholder.</p>
              <p className="mt-1">Resource allocation: {app.ram} RAM, {app.cpu} CPU</p>
            </div>
          </div>
        </div>
      )
  }
}

function launchAppById(appId: string) {
  const app = availableApps.find((a) => a.id === appId)
  if (!app) return
  launchApp(app)
}

function launchApp(app: AppDefinition) {
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
    case "wasm-hello": defaultSize = { width: 700, height: 450 }; break
    default: if (app.isWasm) defaultSize = { width: 700, height: 450 }; break
  }

  const win: OpenWindowDef = {
    id: windowId,
    appId: app.id,
    title: app.name,
    icon: getAppIcon(app.id),
    component: buildAppComponent(app, windowId),
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
}

function refreshAppStoreWindow(authState: string, hasRegistered: boolean, nodeRegistration: unknown) {
  const wins = windowsStore.get()
  const appStoreWin = wins.find((w) => w.appId === "app-store")
  if (!appStoreWin) return

  const running = wins.filter((w) => w.appId !== "app-store").map((w) => w.appId)
  const guestCtx: GuestContext = {
    isGuest: authState === "guest",
    hasIdentity: authState === "authenticated" || hasRegistered,
    hasNode: !!nodeRegistration,
  }

  const updated: OpenWindowDef = {
    ...appStoreWin,
    component: (
      <AppStore
        onLaunchApp={launchApp}
        onAppBlocked={(app, blocked) => pendingGateStore.set({ app, blocked })}
        runningApps={running}
        onRemoveWasm={(name) => wasmRegistry.remove(name)}
        guestCtx={guestCtx}
      />
    ),
  }

  windowsStore.set(wins.map((w) => (w.id === appStoreWin.id ? updated : w)))
}

export function Desktop() {
  const auth = useAuth()

  const windows = useStore(windowsStore)
  const windowOrder = useStore(windowOrderStore)
  const focusedWindow = useStore(focusedWindowStore)
  const terminalLogs = useStore(terminalLogsStore)
  const systemStats = useStore(systemStatsStore)
  const hasBooted = useStore(hasBootedStore)
  const pendingGate = useStore(pendingGateStore)
  const stageMode = useStore(stageModeStore)
  const widgetVisible = useStore(widgetVisibleStore)

  const showDesktop = auth.authState === "authenticated" || auth.authState === "guest"

  useEffect(() => {
    if (!showDesktop || hasBooted) return
    hasBootedStore.set(true)

    const appStoreWindow: OpenWindowDef = {
      id: "window-app-store",
      appId: "app-store",
      title: "App Store",
      icon: <Grid3x3 className="h-4 w-4" />,
      component: (
        <AppStore
          onLaunchApp={launchApp}
          onAppBlocked={(app, blocked) => pendingGateStore.set({ app, blocked })}
          runningApps={[]}
          onRemoveWasm={(name) => wasmRegistry.remove(name)}
          guestCtx={{
            isGuest: auth.authState === "guest",
            hasIdentity: auth.authState === "authenticated" || !!auth.hasRegistered(),
            hasNode: !!auth.nodeRegistration,
          }}
        />
      ),
      defaultPosition: { x: 100, y: 80 },
      defaultSize: { width: 520, height: 480 },
    }

    const helpWindowId = "window-help-boot"
    const helpWindow: OpenWindowDef = {
      id: helpWindowId,
      appId: "help",
      title: "Help & Onboarding",
      icon: <HelpCircle className="h-4 w-4" />,
      component: (
        <HelpApp
          onFinish={() => closeWindow(helpWindowId)}
        />
      ),
      defaultPosition: { x: 340, y: 100 },
      defaultSize: { width: 420, height: 480 },
    }

    windowsStore.set([appStoreWindow, helpWindow])
    windowOrderStore.set(["window-app-store", helpWindowId])
    focusedWindowStore.set(helpWindowId)
    addLog("system", `Welcome back, ${auth.username || "operator"}. Desktop ready.`)
    addLog("success", "Connected to Edgerun network — 12 peers online")
  }, [showDesktop, hasBooted])

  useEffect(() => {
    if (!showDesktop) return
    refreshAppStoreWindow(auth.authState, auth.hasRegistered(), auth.nodeRegistration)
  }, [windows.length, showDesktop])

  useEffect(() => {
    terminalLogsStore.set(generateMockLogs())
  }, [])

  useEffect(() => {
    const interval = setInterval(() => {
      const stats = systemStatsStore.get()
      systemStatsStore.set({
        ...stats,
        nodeCount: Math.max(5, Math.min(24, stats.nodeCount + Math.floor(Math.random() * 3) - 1)),
      })
    }, 5000)
    return () => clearInterval(interval)
  }, [])

  const handleCloseWindow = useCallback((windowId: string) => {
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
  }, [])

  return (
    <div className="relative h-screen w-screen overflow-hidden bg-background">
      <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
        <Globe
          nodeCount={24}
          className="h-full w-full opacity-40"
        />
      </div>

      {!showDesktop && (
        <AuthOverlay
          authState={auth.authState}
          username={auth.username}
          isLoading={auth.isLoading}
          error={auth.error}
          hasRegistered={auth.hasRegistered}
          webAuthnAvailable={auth.webAuthnAvailable}
          onRegister={auth.register}
          onAuthenticate={auth.authenticate}
          onContinueAsGuest={auth.continueAsGuest}
          onClearError={auth.clearError}
        />
      )}

      {showDesktop && (
        <>
          <TopBar
            nodeCount={systemStats.nodeCount}
            activeSessions={systemStats.activeSessions}
            ramUsage={systemStats.ramUsage}
            isConnected={systemStats.isConnected}
            username={auth.username}
            isGuest={auth.authState === "guest"}
            onLock={auth.lock}
            onSignIn={async () => {
              stageModeStore.set(false)
              const authPromptWin: OpenWindowDef = {
                id: "window-auth-prompt",
                appId: "auth-prompt",
                title: "Set up Identity",
                icon: <Users className="h-4 w-4" />,
                component: (
                  <CapabilityGatePrompt
                    appName="Edgerun"
                    blockedCapabilities={["Identity"]}
                    onSetupIdentity={async (name) => {
                      const ok = await auth.register(name)
                      if (ok) closeWindow("window-auth-prompt")
                      return ok
                    }}
                    onDismiss={() => closeWindow("window-auth-prompt")}
                  />
                ),
                defaultPosition: { x: 250, y: 120 },
                defaultSize: { width: 400, height: 380 },
              }
              openWindow(authPromptWin)
            }}
          />

          <div className="absolute inset-0 top-10">
            <StageManager
              windows={windows}
              focusedWindowId={focusedWindow}
              onFocus={focusWindow}
              onClose={handleCloseWindow}
              enabled={stageMode}
            />

            <WidgetPanel
              visible={widgetVisible}
              onToggle={() => widgetVisibleStore.set(!widgetVisible)}
            />

            <div className="absolute bottom-4 left-1/2 z-40 flex -translate-x-1/2 items-center gap-1 rounded-xl border border-border bg-[var(--window-bg)]/80 p-1.5 backdrop-blur-md">
              <button
                onClick={() => stageModeStore.set(!stageMode)}
                className={cn(
                  "relative flex h-10 w-10 items-center justify-center rounded-lg transition-all",
                  stageMode
                    ? "bg-primary/15 text-primary"
                    : "text-muted-foreground hover:bg-secondary hover:text-foreground"
                )}
                title={stageMode ? "Disable Stage Manager" : "Enable Stage Manager"}
              >
                <LayoutDashboard className="h-4 w-4" />
              </button>

              <button
                onClick={() => widgetVisibleStore.set(!widgetVisible)}
                className={cn(
                  "relative flex h-10 w-10 items-center justify-center rounded-lg transition-all",
                  widgetVisible
                    ? "bg-primary/15 text-primary"
                    : "text-muted-foreground hover:bg-secondary hover:text-foreground"
                )}
                title={widgetVisible ? "Hide Widgets" : "Show Widgets"}
              >
                <PanelRight className="h-4 w-4" />
              </button>

              <div className="mx-1 h-6 w-px bg-border" />

               {["app-store", "contacts", "calling", "chat", "wallet", "calculator", "file-browser", "terminal", "code-runner", "wasm-calculator", "wasm-hello", "db-explorer", "network-monitor", "git-sync", "web-server", "compute-node", "resource-monitor", "help"].map((appId) => {
                const app = availableApps.find((a) => a.id === appId)
                if (!app) return null
                const isRunning = windows.some((w) => w.appId === app.id)
                return (
                  <button
                    key={app.id}
                    onClick={() => launchApp(app)}
                    className="group relative flex h-10 w-10 items-center justify-center rounded-lg text-muted-foreground transition-all hover:bg-secondary hover:text-foreground"
                    title={app.name}
                  >
                    {getAppIcon(app.id)}
                    {isRunning && (
                      <span className="absolute -bottom-0.5 left-1/2 h-1 w-1 -translate-x-1/2 rounded-full bg-primary" />
                    )}
                  </button>
                )
              })}
            </div>

            {pendingGate && (
              <Window
                id="window-capability-gate"
                title="Capability Required"
                icon={<Users className="h-4 w-4" />}
                defaultPosition={{ x: 250, y: 120 }}
                defaultSize={{ width: 400, height: 380 }}
                onClose={() => pendingGateStore.set(null)}
                onFocus={() => focusWindow("window-capability-gate")}
                isFocused={focusedWindow === "window-capability-gate"}
                zIndex={windowOrder.length + 10}
                stageHidden={false}
                stageMode={false}
              >
                <CapabilityGatePrompt
                  appName={pendingGate.app.name}
                  blockedCapabilities={pendingGate.blocked}
                  onSetupIdentity={async (name) => {
                    const ok = await auth.register(name)
                    if (ok) pendingGateStore.set(null)
                    return ok
                  }}
                  onDismiss={() => pendingGateStore.set(null)}
                />
              </Window>
            )}

            {windows.map((win) => {
              const isStageHidden = stageMode && win.id !== focusedWindow
              return (
                <Window
                  key={win.id}
                  id={win.id}
                  title={win.title}
                  icon={win.icon}
                  defaultPosition={win.defaultPosition}
                  defaultSize={win.defaultSize}
                  onClose={() => handleCloseWindow(win.id)}
                  onFocus={() => focusWindow(win.id)}
                  isFocused={focusedWindow === win.id}
                  zIndex={windowOrder.indexOf(win.id) + 10}
                  stageHidden={isStageHidden}
                  stageMode={stageMode}
                >
                  {win.component}
                </Window>
              )
            })}
          </div>
        </>
      )}
    </div>
  )
}
