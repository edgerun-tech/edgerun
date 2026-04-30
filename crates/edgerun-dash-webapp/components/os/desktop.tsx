"use client"

import { useState, useCallback, useEffect } from "react"
import { cn } from "@/lib/utils"
import { Window } from "./window"
import { TopBar } from "./top-bar"
import { AppStore, availableApps, type AppDefinition } from "./app-store"
import { Terminal, generateMockLogs } from "./terminal"
import { CodeRunner } from "./code-runner"
import { Globe } from "./globe"
import { AuthOverlay } from "./auth-overlay"
import { HelpApp } from "./help-app"
import { ContactsApp } from "./contacts-app"
import { CallingApp } from "./calling-app"
import { ChatApp } from "./chat-app"
import { WalletApp } from "./wallet-app"
import { CalculatorApp } from "./calculator-app"
import { StageManager } from "./stage-manager"
import { WidgetPanel } from "./widget-panel"
import { useAuth } from "@/hooks/use-auth"
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
} from "lucide-react"

interface OpenWindow {
  id: string
  appId: string
  title: string
  icon: React.ReactNode
  component: React.ReactNode
  defaultPosition: { x: number; y: number }
  defaultSize: { width: number; height: number }
}

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
  }
  return icons[appId] || <Grid3x3 className="h-4 w-4" />
}

export function Desktop() {
  const auth = useAuth()

  const [windows, setWindows] = useState<OpenWindow[]>([])
  const [windowOrder, setWindowOrder] = useState<string[]>([])
  const [focusedWindow, setFocusedWindow] = useState<string | null>(null)
  const [terminalLogs, setTerminalLogs] = useState(generateMockLogs())
  const [stageMode, setStageMode] = useState(false)
  const [widgetVisible, setWidgetVisible] = useState(false)

  // System stats
  const [systemStats, setSystemStats] = useState({
    nodeCount: 12,
    activeSessions: 3,
    ramUsage: { used: 4.2, total: 8 },
    isConnected: true,
  })

  // Add system log
  const addLog = useCallback((type: "info" | "success" | "warning" | "error" | "system", message: string) => {
    setTerminalLogs((prev) => [
      ...prev,
      {
        id: `log-${Date.now()}-${Math.random()}`,
        timestamp: new Date(),
        type,
        message,
      },
    ])
  }, [])

  // Calculate next window position
  const getNextPosition = useCallback(() => {
    const offset = (windows.length % 8) * 30
    return { x: 150 + offset, y: 80 + offset }
  }, [windows.length])

  // Forward-declare handleLaunchApp so we can reference it in useEffect
  const handleLaunchApp = useCallback((app: AppDefinition) => {
    const windowId = `window-${app.id}-${Date.now()}`
    const position = getNextPosition()

    let component: React.ReactNode
    let defaultSize = { width: 600, height: 400 }

    switch (app.id) {
      case "terminal":
        component = (
          <Terminal
            logs={terminalLogs}
            onCommand={(cmd) => {
              addLog("info", `$ ${cmd}`)
              setTimeout(() => {
                if (cmd === "help") {
                  addLog("system", "Available commands: help, status, nodes, clear")
                } else if (cmd === "status") {
                  addLog("success", "All systems operational")
                } else if (cmd === "nodes") {
                  addLog("info", `Connected to ${systemStats.nodeCount} nodes`)
                } else if (cmd === "clear") {
                  setTerminalLogs([])
                } else {
                  addLog("warning", `Unknown command: ${cmd}`)
                }
              }, 100)
            }}
          />
        )
        defaultSize = { width: 700, height: 450 }
        break

      case "code-runner":
        component = (
          <CodeRunner
            onOutput={(output) => {
              addLog("info", `Code executed: ${output.split("\n")[0]}...`)
            }}
          />
        )
        defaultSize = { width: 800, height: 500 }
        break

      case "contacts":
        component = (
          <ContactsApp
            onCall={(contact) => {
              addLog("info", `Calling ${contact.name}...`)
              handleLaunchApp({ id: "calling", name: "Calling", description: "Encrypted P2P voice calls", icon: <Phone className="h-5 w-5" />, ram: "48MB", cpu: "1.0%", price: "Free" })
            }}
            onMessage={(contact) => {
              addLog("info", `Opening chat with ${contact.name}`)
              handleLaunchApp({ id: "chat", name: "Chat", description: "E2E encrypted messaging", icon: <MessageSquare className="h-5 w-5" />, ram: "24MB", cpu: "0.2%", price: "Free" })
            }}
          />
        )
        defaultSize = { width: 560, height: 460 }
        break

      case "calling":
        component = <CallingApp />
        defaultSize = { width: 340, height: 480 }
        break

      case "chat":
        component = <ChatApp />
        defaultSize = { width: 580, height: 460 }
        break

      case "wallet":
        component = <WalletApp />
        defaultSize = { width: 360, height: 520 }
        break

      case "calculator":
        component = <CalculatorApp />
        defaultSize = { width: 300, height: 420 }
        break

      case "help":
        component = (
          <HelpApp
            onFinish={() => {
              setWindows((prev) => prev.filter((w) => w.id !== windowId))
              setWindowOrder((prev) => prev.filter((id) => id !== windowId))
            }}
          />
        )
        defaultSize = { width: 420, height: 480 }
        break

      default:
        component = (
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

    const newWindow: OpenWindow = {
      id: windowId,
      appId: app.id,
      title: app.name,
      icon: getAppIcon(app.id),
      component,
      defaultPosition: position,
      defaultSize,
    }

    setWindows((prev) => [...prev, newWindow])
    setWindowOrder((prev) => [...prev, windowId])
    setFocusedWindow(windowId)
    addLog("success", `Launched ${app.name}`)

    const ramIncrease = parseFloat(app.ram) / 1000
    setSystemStats((prev) => ({
      ...prev,
      ramUsage: { ...prev.ramUsage, used: Math.min(prev.ramUsage.total, prev.ramUsage.used + ramIncrease) },
      activeSessions: prev.activeSessions + 1,
    }))
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [getNextPosition, addLog, systemStats.nodeCount, terminalLogs])

  // Open App Store + Help on first authenticated load
  const [hasBooted, setHasBooted] = useState(false)
  useEffect(() => {
    if (auth.authState !== "authenticated" || hasBooted) return
    setHasBooted(true)

    const appStoreWindow: OpenWindow = {
      id: "window-app-store",
      appId: "app-store",
      title: "App Store",
      icon: <Grid3x3 className="h-4 w-4" />,
      component: (
        <AppStore onLaunchApp={handleLaunchApp} runningApps={[]} />
      ),
      defaultPosition: { x: 100, y: 80 },
      defaultSize: { width: 520, height: 480 },
    }

    const helpApp = availableApps.find((a) => a.id === "help")!
    const helpWindowId = "window-help-boot"
    const helpWindow: OpenWindow = {
      id: helpWindowId,
      appId: "help",
      title: "Help & Onboarding",
      icon: <HelpCircle className="h-4 w-4" />,
      component: (
        <HelpApp
          onFinish={() => {
            setWindows((prev) => prev.filter((w) => w.id !== helpWindowId))
            setWindowOrder((prev) => prev.filter((id) => id !== helpWindowId))
          }}
        />
      ),
      defaultPosition: { x: 340, y: 100 },
      defaultSize: { width: 420, height: 480 },
    }

    setWindows([appStoreWindow, helpWindow])
    setWindowOrder(["window-app-store", helpWindowId])
    setFocusedWindow(helpWindowId)
    addLog("system", `Welcome back, ${auth.username || "operator"}. Desktop ready.`)
    addLog("success", "Connected to Edgerun network — 12 peers online")
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [auth.authState])

  // Keep App Store updated with running apps list
  useEffect(() => {
    setWindows((prev) =>
      prev.map((w) =>
        w.appId === "app-store"
          ? {
              ...w,
              component: (
                <AppStore
                  onLaunchApp={handleLaunchApp}
                  runningApps={prev.filter((win) => win.appId !== "app-store").map((win) => win.appId)}
                />
              ),
            }
          : w
      )
    )
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [windows.length, handleLaunchApp])

  // Handle window close
  const handleCloseWindow = useCallback((windowId: string) => {
    const closedWindow = windows.find((w) => w.id === windowId)
    setWindows((prev) => prev.filter((w) => w.id !== windowId))
    setWindowOrder((prev) => prev.filter((id) => id !== windowId))

    if (focusedWindow === windowId) {
      setFocusedWindow(windowOrder[windowOrder.length - 2] || null)
    }

    if (closedWindow) {
      addLog("info", `Closed ${closedWindow.title}`)
      const app = availableApps.find((a) => a.id === closedWindow.appId)
      if (app) {
        const ramDecrease = parseFloat(app.ram) / 1000
        setSystemStats((prev) => ({
          ...prev,
          ramUsage: { ...prev.ramUsage, used: Math.max(0, prev.ramUsage.used - ramDecrease) },
          activeSessions: Math.max(0, prev.activeSessions - 1),
        }))
      }
    }
  }, [windows, focusedWindow, windowOrder, addLog])

  // Handle window focus
  const handleFocusWindow = useCallback((windowId: string) => {
    setFocusedWindow(windowId)
    setWindowOrder((prev) => {
      const filtered = prev.filter((id) => id !== windowId)
      return [...filtered, windowId]
    })
  }, [])

  // Simulate node count fluctuation
  useEffect(() => {
    const interval = setInterval(() => {
      setSystemStats((prev) => ({
        ...prev,
        nodeCount: Math.max(5, Math.min(24, prev.nodeCount + Math.floor(Math.random() * 3) - 1)),
      }))
    }, 5000)
    return () => clearInterval(interval)
  }, [])

  // Show auth overlay when not authenticated or locked
  const showAuth = auth.authState !== "authenticated"

  return (
    <div className="relative h-screen w-screen overflow-hidden bg-background">
      {/* Globe background — always rendered (visible through auth overlay too) */}
      <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
        <Globe
          nodeCount={24}
          className="h-full w-full opacity-40"
        />
      </div>

      {/* Auth overlay — shown when unauthenticated or locked */}
      {showAuth && (
        <AuthOverlay
          authState={auth.authState}
          username={auth.username}
          isLoading={auth.isLoading}
          error={auth.error}
          hasRegistered={auth.hasRegistered}
          webAuthnAvailable={auth.webAuthnAvailable}
          onRegister={auth.register}
          onAuthenticate={auth.authenticate}
          onClearError={auth.clearError}
        />
      )}

      {/* Desktop — only visible when authenticated */}
      {!showAuth && (
        <>
          {/* Top Bar */}
          <TopBar
            nodeCount={systemStats.nodeCount}
            activeSessions={systemStats.activeSessions}
            ramUsage={systemStats.ramUsage}
            isConnected={systemStats.isConnected}
            username={auth.username}
            onLock={auth.lock}
          />

          {/* Desktop Area */}
          <div className="absolute inset-0 top-10">

            {/* Stage Manager strip */}
            <StageManager
              windows={windows}
              focusedWindowId={focusedWindow}
              onFocus={handleFocusWindow}
              onClose={handleCloseWindow}
              enabled={stageMode}
            />

            {/* Widget Panel (right side) */}
            <WidgetPanel
              visible={widgetVisible}
              onToggle={() => setWidgetVisible((v) => !v)}
            />

            {/* Dock */}
            <div className="absolute bottom-4 left-1/2 z-40 flex -translate-x-1/2 items-center gap-1 rounded-xl border border-border bg-[var(--window-bg)]/80 p-1.5 backdrop-blur-md">
              {/* Stage Manager toggle */}
              <button
                onClick={() => setStageMode((s) => !s)}
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

              {/* Widget toggle */}
              <button
                onClick={() => setWidgetVisible((v) => !v)}
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

              {["app-store", "contacts", "calling", "chat", "wallet", "calculator", "terminal", "code-runner", "help"].map((appId) => {
                const app = availableApps.find((a) => a.id === appId)
                if (!app) return null
                const isRunning = windows.some((w) => w.appId === app.id)
                return (
                  <button
                    key={app.id}
                    onClick={() => handleLaunchApp(app)}
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

            {/* Windows */}
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
                  onFocus={() => handleFocusWindow(win.id)}
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
