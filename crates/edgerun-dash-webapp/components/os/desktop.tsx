"use client"

import { useEffect, useCallback } from "react"
import { useStore } from "@nanostores/react"
import { cn } from "@/lib/utils"
import { Window } from "./window"
import { TopBar } from "./top-bar"
import { AppStore, availableApps, type AppDefinition } from "./app-store"
import { launchApp, getAppIcon } from "@/stores/app-launcher"
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
import { AIAssistant } from "./ai-assistant"
import { WasmAppWindow } from "@/components/wasm-app-window"
import { removeWasm } from "@/stores/wasm-store"
import { StageManager } from "./stage-manager"
import { WidgetPanel } from "./widget-panel"
import { useAuth } from "@/hooks/use-auth"
import { CapabilityGatePrompt } from "@/components/capability-gate-prompt"
import { type GuestContext } from "@/lib/capabilities"
import {
  LayoutDashboard,
  PanelRight,
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
  closeWindow,
  focusWindow,
  type OpenWindowDef,
} from "@/stores/desktop-store"
import { launchAppById, handleCloseWindow, buildAppComponent } from "@/stores/app-launcher"
import { startSystemStatsSimulation, generateInitialLogs } from "@/stores/system-store"

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

  // Keyboard shortcuts
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "a") {
      e.preventDefault()
      const aiApp = availableApps.find(a => a.id === "ai-assistant")
      if (aiApp) launchApp(aiApp)
    }
  }, [])

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [handleKeyDown])

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
                title="Toggle Stage Manager"
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

               {["app-store", "app-studio", "ai-assistant", "workflow-builder", "contacts", "calling", "chat", "wallet", "calculator", "file-browser", "terminal", "code-runner", "wasm-calculator", "wasm-hello", "db-explorer", "network-monitor", "git-sync", "web-server", "compute-node", "resource-monitor", "help"].map((appId) => {
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
