"use client"

import { useEffect, useCallback, useMemo } from "react"
import { useStore } from "@nanostores/react"
import { Window } from "./window"
import { TopBar } from "./top-bar"
import { DesktopTelemetry } from "./desktop-telemetry"
import { launchApp, getAppIcon } from "@/stores/app-launcher"
import { AuthOverlay } from "./auth-overlay"
import { StageManager } from "./stage-manager"
import { WidgetPanel } from "./widget-panel"
import { WorkspaceStatusPanel } from "@/components/workspace"
import { useAuth } from "@/hooks/use-auth"
import { CapabilityGatePrompt } from "@/components/capability-gate-prompt"
import { getBuiltinApp } from "@/platform/registries/builtin-app-registry"
import { FloatingDock } from "@/components/ui/floating-dock"
import { installedAppIdsStore, CORE_APP_IDS } from "@/stores/installed-apps-store"
import { grantLocalCapabilities } from "@/stores/local-capability-grants-store"
import { Users } from "lucide-react"
import {
  windowsStore,
  windowOrderStore,
  focusedWindowStore,
  systemStatsStore,
  pendingGateStore,
  stageModeStore,
  widgetVisibleStore,
  desktopTelemetryVisibleStore,
  openWindow,
  closeWindow,
  focusWindow,
  type OpenWindowDef,
} from "@/stores/desktop-store"

export function Desktop() {
  const auth = useAuth()

  const windows = useStore(windowsStore)
  const windowOrder = useStore(windowOrderStore)
  const focusedWindow = useStore(focusedWindowStore)
  const systemStats = useStore(systemStatsStore)
  const pendingGate = useStore(pendingGateStore)
  const stageMode = useStore(stageModeStore)
  const widgetVisible = useStore(widgetVisibleStore)
  const desktopTelemetryVisible = useStore(desktopTelemetryVisibleStore)
  const installedAppIds = useStore(installedAppIdsStore)

  const showDesktop = auth.authState === "authenticated" || auth.authState === "guest"

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "t") {
      e.preventDefault()
      const terminalApp = getBuiltinApp("terminal")
      if (terminalApp) launchApp(terminalApp)
    }
  }, [])

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [handleKeyDown])

  const handleCloseWindow = useCallback((id: string) => {
    closeWindow(id)
  }, [])

  const dockItems = useMemo(() => {
    const dockAppIds = Array.from(new Set([...CORE_APP_IDS, ...installedAppIds]))
    return dockAppIds.map((appId) => {
      const app = getBuiltinApp(appId)
      if (!app) return null
      return {
        title: app.name + (app.source === "demo" ? " (Demo)" : ""),
        icon: getAppIcon(app.appId),
        onClick: () => {
          const fullApp = getBuiltinApp(app.appId)
          if (fullApp) launchApp(fullApp)
        },
      }
    }).filter(Boolean)
  }, [installedAppIds])

  const grantPendingAndOpen = useCallback(() => {
    const gate = pendingGateStore.get()
    if (!gate) return
    grantLocalCapabilities(
      gate.app.appId,
      gate.blocked,
      `User approved ${gate.blocked.join(", ")} for ${gate.app.name}`,
    )
    pendingGateStore.set(null)
    launchApp(gate.app)
  }, [])

  return (
    <div className="relative h-screen w-screen overflow-hidden bg-background">
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
                ),
                defaultPosition: { x: 250, y: 120 },
                defaultSize: { width: 420, height: 420 },
              }
              openWindow(authPromptWin)
            }}
          />

          <div className="absolute inset-0 top-10 z-10">
            {desktopTelemetryVisible && (
              <DesktopTelemetry
                nodeCount={systemStats.nodeCount}
                activeSessions={systemStats.activeSessions}
                ramUsage={systemStats.ramUsage}
                isConnected={systemStats.isConnected}
                runningApps={windows.length}
              />
            )}

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

            <FloatingDock
              items={dockItems as any}
              desktopClassName="fixed bottom-4 left-1/2 -translate-x-1/2 z-40"
            />

            {pendingGate && (
              <Window
                id="window-capability-gate"
                title="Permission Request"
                icon={<Users className="h-4 w-4" />}
                defaultPosition={{ x: 250, y: 120 }}
                defaultSize={{ width: 460, height: 520 }}
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
                  onGrant={grantPendingAndOpen}
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
                  appId={win.appId}
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

            <WorkspaceStatusPanel />
          </div>
        </>
      )}
    </div>
  )
}
