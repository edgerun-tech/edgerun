"use client"

import { useEffect, useCallback, useState } from "react"
import { useStore } from "@nanostores/react"
import { cn } from "@/lib/utils"
import { Window } from "./window"
import { TopBar } from "./top-bar"
import { launchApp, getAppIcon } from "@/stores/app-launcher"
import { Terminal, generateMockLogs } from "./terminal"
import { CodeRunner } from "./code-runner"
import { ResourceMonitor } from "./resource-monitor"
import { AuthOverlay } from "./auth-overlay"
import { HelpApp } from "./help-app"
import { ContactsApp } from "./contacts-app"
import { CallingApp } from "./calling-app"
import { DemoChatApp } from "./chat-app"
import { WalletApp } from "./wallet-app"
import { CalculatorApp } from "./calculator-app"
import { AIAssistant } from "./ai-assitant"
import { WasmAppWindow } from "@/components/wasm-app-window"
import { removeWasm } from "@/stores/wasm-store"
import { StageManager } from "./stage-manager"
import { WidgetPanel } from "./widget-panel"
import { WorkspaceStatusPanel } from "@/components/workspace"
import { useAuth } from "@/hooks/use-auth"
import { CapabilityGatePrompt } from "@/components/capability-gate-prompt"
import { getBuiltinApp, listBuiltinApps } from "@/platform/registries/builtin-app-registry"
import { getDashboardMode, isDemoMode } from "@/platform/runtime/dashboard-mode"
import {
  LayoutDashboard,
  PanelRight,
  Users,
} from "lucide-react"
import { FloatingDock } from "@/components/ui/floating-dock"
import dynamic from "next/dynamic"
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

const XrayWorkspace = dynamic(
  () => import("@/features/xray/XrayWorkspace").then(mod => ({ default: mod.default })),
  { ssr: false }
)

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
  const mode = getDashboardMode()

  const showDesktop = auth.authState === "authenticated" || auth.authState === "guest"

  // Keyboard shortcuts
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "a") {
      e.preventDefault()
      const aiApp = getBuiltinApp("ai-assitant")
      if (aiApp) launchApp(aiApp)
    }
  }, [])

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [handleKeyDown])

  // App IDs to show in dock
  const dockAppIds = [
    "app-store", "app-studio", "ai-assitant", "workflow-builder",
    "contacts", "calling", "chat", "wallet", "calculator",
    "file-browser", "terminal", "code-runner", "wasm-calculator",
    "wasm-hello", "db-explorer", "network-monitor", "git-sync",
    "web-server", "compute-node", "resource-monitor", "help",
  ]

  const handleCloseWindow = useCallback((id: string) => {
    closeWindow(id)
  }, [])

  return (
    <div className="relative h-screen w-screen overflow-hidden bg-background">
      {/* Xray graph - center visualization (replaces globe) */}
      {showDesktop && (
        <div className="absolute inset-0">
          <XrayWorkspace />
        </div>
      )}

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
              onClose={handleCloseWindow}
              onFocus={(id) => focusWindow(id)}
              isStageHidden={!stageMode}
              allWindowsMinimized={windows.length > 0 && windows.every(w => w.isMinimized)}
            >
              {windows.map((win) => {
                const app = getBuiltinApp(win.appId)
                if (!app) return null
                const isHidden = win.isMinimized || (stageMode && win.id !== focusedWindow)
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
                    stageHidden={isHidden}
                    stageMode={stageMode}
                  >
                    {win.component}
                  </Window>
                )
              })}
              <WidgetPanel visible={widgetVisible} />
              <WorkspaceStatusPanel />
            </StageManager>
          </div>

          {/* Dock */}
          <div className="fixed bottom-4 left-1/2 -translate-x-1/2 z-50">
            <FloatingDock
              items={dockAppIds.map(id => {
                const app = getBuiltinApp(id)
                return app ? { title: app.name, icon: app.icon, href: `#${id}` } : null
              }).filter(Boolean) as { title: string; icon: React.ReactNode; href: string }[]}
            />
          </div>
        </>
      )}
    </div>
  )
}
