"use client"

import { useEffect, useCallback, useMemo } from "react"
import { useStore } from "@nanostores/react"
import { TopBar } from "./top-bar"
import { XrayDesktopSurface } from "./xray-desktop-surface"
import { AppOverlayHost } from "./app-overlay-host"
import { launchApp, getAppIcon } from "@/stores/app-launcher"
import { AuthOverlay } from "./auth-overlay"
import { WidgetPanel } from "./widget-panel"
import { useAuth } from "@/hooks/use-auth"
import { CapabilityGatePrompt } from "@/components/capability-gate-prompt"
import { getBuiltinApp } from "@/platform/registries/builtin-app-registry"
import { FloatingDock, type FloatingDockContext } from "@/components/ui/floating-dock"
import { installedAppIdsStore, CORE_APP_IDS } from "@/stores/installed-apps-store"
import { grantLocalCapabilities } from "@/stores/local-capability-grants-store"
import { MessageSquare, Phone, Users } from "lucide-react"
import {
  appSurfacesStore,
  appSurfaceOrderStore,
  focusedAppSurfaceStore,
  systemStatsStore,
  pendingGateStore,
  widgetVisibleStore,
  terminalLogsStore,
  openAppSurface,
  closeAppSurface,
  focusAppSurface,
  addLog,
  type AppSurfaceDef,
} from "@/stores/desktop-store"

function ChatHead({ label, tone = "primary" }: { label: string; tone?: "primary" | "green" | "blue" | "amber" }) {
  const tones = {
    primary: "from-primary/80 to-primary/35",
    green: "from-emerald-400/90 to-emerald-700/50",
    blue: "from-sky-400/90 to-sky-700/50",
    amber: "from-amber-300/90 to-amber-700/50",
  }
  return (
    <div className={`flex h-full w-full items-center justify-center rounded-full bg-gradient-to-br ${tones[tone]} font-mono text-xs font-bold text-white shadow-inner ring-1 ring-white/20`}>
      {label}
    </div>
  )
}

export function Desktop() {
  const auth = useAuth()

  const appSurfaces = useStore(appSurfacesStore)
  const appSurfaceOrder = useStore(appSurfaceOrderStore)
  const focusedAppSurface = useStore(focusedAppSurfaceStore)
  const systemStats = useStore(systemStatsStore)
  const pendingGate = useStore(pendingGateStore)
  const widgetVisible = useStore(widgetVisibleStore)
  const installedAppIds = useStore(installedAppIdsStore)

  const showDesktop = auth.authState === "authenticated" || auth.authState === "guest"
  const pinnedSurfaces = useMemo(
    () => appSurfaces.filter((surface) => surface.kind === "pinned-widget"),
    [appSurfaces],
  )

  const activeSurface = useMemo(() => {
    if (focusedAppSurface) {
      const focused = appSurfaces.find((surface) => surface.id === focusedAppSurface)
      if (focused) return focused
    }
    for (let i = appSurfaceOrder.length - 1; i >= 0; i--) {
      const surface = appSurfaces.find((candidate) => candidate.id === appSurfaceOrder[i])
      if (surface?.kind === "overlay") return surface
    }
    return null
  }, [appSurfaceOrder, appSurfaces, focusedAppSurface])

  useEffect(() => {
    if (!showDesktop) return
    const PINNED_APP_IDS = ["network-monitor", "compute-node"]
    const surfaces = appSurfacesStore.get()
    for (const appId of PINNED_APP_IDS) {
      const alreadyPinned = surfaces.some((s) => s.appId === appId && s.kind === "pinned-widget")
      if (!alreadyPinned) {
        const app = getBuiltinApp(appId)
        if (app) launchApp(app)
      }
    }
  }, [showDesktop])

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

  const handleCloseSurface = useCallback((id: string) => {
    closeAppSurface(id)
  }, [])

  const dockItems = useMemo(() => {
    const dockAppIds = Array.from(new Set([...CORE_APP_IDS, ...installedAppIds]))
    return dockAppIds
      .map((appId) => {
        const app = getBuiltinApp(appId)
        if (!app) return null
        return {
          title: app.name,
          icon: getAppIcon(app.appId),
          kind: "app" as const,
          onClick: () => {
            const fullApp = getBuiltinApp(app.appId)
            if (fullApp) launchApp(fullApp)
          },
        }
      })
      .filter(Boolean)
  }, [installedAppIds])

  const dockContext = useMemo<FloatingDockContext>(() => {
    const activeAppId = activeSurface?.appId
    if (activeAppId === "people" || activeAppId === "chat" || activeAppId === "contacts" || activeAppId === "calling") {
      return {
        mode: "chat-heads",
        label: "People",
        items: [
          {
            title: "Ara",
            subtitle: "online",
            kind: "person",
            icon: <ChatHead label="A" tone="green" />,
            onClick: () => addLog("info", "Selected Ara conversation"),
          },
          {
            title: "Elias",
            subtitle: "build thread",
            kind: "person",
            icon: <ChatHead label="E" tone="blue" />,
            onClick: () => addLog("info", "Selected Elias conversation"),
          },
          {
            title: "Max",
            subtitle: "nearby",
            kind: "person",
            icon: <ChatHead label="M" tone="amber" />,
            onClick: () => addLog("info", "Selected Max conversation"),
          },
          {
            title: "New chat",
            subtitle: "compose",
            kind: "trigger",
            icon: <MessageSquare className="h-full w-full rounded-full bg-primary/10 p-2 text-primary" />,
            onClick: () => addLog("info", "New chat trigger"),
          },
          {
            title: "Call",
            subtitle: "voice",
            kind: "trigger",
            icon: <Phone className="h-full w-full rounded-full bg-primary/10 p-2 text-primary" />,
            onClick: () => addLog("info", "Call trigger"),
          },
        ],
      }
    }

    return {
      mode: "apps",
      label: activeSurface ? activeSurface.title : "Apps",
    }
  }, [activeSurface])

  const handleDockCommand = useCallback((command: string) => {
    addLog("info", `Dock command: ${command}`)
    terminalLogsStore.set([
      ...terminalLogsStore.get(),
      {
        id: `dock-command-${Date.now()}`,
        timestamp: new Date(),
        type: "system",
        message: `dock> ${command}`,
      },
    ])
  }, [])

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

  const openIdentitySetup = useCallback(async () => {
    const authPromptSurface: AppSurfaceDef = {
      id: "surface-auth-prompt",
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
      kind: "overlay",
      dismissOnOutsideClick: true,
      defaultSize: { width: 420, height: 420 },
    }
    openAppSurface(authPromptSurface)
  }, [auth])

  if (!showDesktop) {
    return (
      <div className="relative h-screen w-screen overflow-hidden bg-background">
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
      </div>
    )
  }

  return (
    <div className="relative h-screen w-screen overflow-hidden bg-background">
      <TopBar
        nodeCount={systemStats.nodeCount}
        activeSessions={systemStats.activeSessions}
        ramUsage={systemStats.ramUsage}
        isConnected={systemStats.isConnected}
        username={auth.username}
        isGuest={auth.authState === "guest"}
        onLock={auth.lock}
        onSignIn={openIdentitySetup}
      />

      <div className="absolute inset-0 top-10 z-10">
        <XrayDesktopSurface
          nodeCount={systemStats.nodeCount}
          activeSessions={systemStats.activeSessions}
          ramUsage={systemStats.ramUsage}
          isConnected={systemStats.isConnected}
          runningApps={appSurfaces.length}
          pinnedSurfaces={pinnedSurfaces}
        />

        <WidgetPanel
          visible={widgetVisible}
          onToggle={() => widgetVisibleStore.set(!widgetVisible)}
        />

        <FloatingDock
          items={dockItems as any}
          context={dockContext}
          onCommandSubmit={handleDockCommand}
          desktopClassName="fixed bottom-4 left-1/2 z-50 -translate-x-1/2"
        />

        {pendingGate ? (
          <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/35 p-6 backdrop-blur-[2px]">
            <div className="w-[min(460px,calc(100vw-3rem))] overflow-hidden rounded-[28px] border border-white/10 bg-background/95 shadow-[0_32px_120px_rgba(0,0,0,0.78)]">
              <CapabilityGatePrompt
                appName={pendingGate.app.name}
                blockedCapabilities={pendingGate.blocked}
                onGrant={grantPendingAndOpen}
                onDismiss={() => pendingGateStore.set(null)}
              />
            </div>
          </div>
        ) : null}

        <AppOverlayHost
          surfaces={appSurfaces}
          surfaceOrder={appSurfaceOrder}
          focusedSurfaceId={focusedAppSurface}
          onFocus={focusAppSurface}
          onClose={handleCloseSurface}
        />
      </div>
    </div>
  )
}
