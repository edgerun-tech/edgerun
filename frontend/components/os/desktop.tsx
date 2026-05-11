"use client"

import { useCallback, useEffect, useMemo } from "react"
import type React from "react"
import { useStore } from "@nanostores/react"
import { Bot, IdCard, Settings, Store } from "lucide-react"
import { AuthOverlay } from "./auth-overlay"
import { AppOverlayHost } from "./app-overlay-host"
import { AgentUiBridge } from "./agent-ui-bridge"
import { DevToolsStrip } from "./dev-tools-strip"
import { ProfileMenu } from "./profile-menu"
import { ProjectChecklist } from "./project-checklist"
import { CapabilityGatePrompt } from "@/components/capability-gate-prompt"
import { NodeNetworkBuilder } from "@/components/node-network-builder"
import { useAuth } from "@/hooks/use-auth"
import { FloatingDock, type FloatingDockContext, type FloatingDockItem } from "@/components/ui/floating-dock"
import { appSurfaceOrderStore, appSurfacesStore, focusedAppSurfaceStore, focusAppSurface, pendingGateStore } from "@/stores/desktop-store"
import { getAppIcon, handleCloseAppSurface, launchApp, launchAppById } from "@/stores/app-launcher"
import { listBuiltinApps } from "@/platform/registries/builtin-app-registry"
import { isRemovedAppId } from "@/platform/registries/app-id-policy"
import { catalogApps } from "@/platform/registries/app-catalog-registry"
import { grantLocalCapabilities } from "@/stores/local-capability-grants-store"
import { installedAppIdsStore, normalizeAppId } from "@/stores/installed-apps-store"
import { executeUiCommand, focusDockInput, registerUiCommandHandler } from "@/stores/ui-command-center"

function DockIcon({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex h-full w-full items-center justify-center rounded-full text-white">
      {children}
    </div>
  )
}

function focusCodexDockInput() {
  focusDockInput("~")
}

const PINNED_DOCK_APP_IDS = new Set(["identity", "app-store", "settings"])

export function Desktop() {
  const auth = useAuth()
  const appSurfaces = useStore(appSurfacesStore)
  const appSurfaceOrder = useStore(appSurfaceOrderStore)
  const focusedAppSurfaceId = useStore(focusedAppSurfaceStore)
  const pendingGate = useStore(pendingGateStore)
  const installedAppIds = useStore(installedAppIdsStore)
  const catalogAppList = useStore(catalogApps)
  const showDesktop = auth.authState === "authenticated"

  useEffect(() => {
    return registerUiCommandHandler("/lock", () => {
      auth.lock()
      return "Locked browser session"
    })
  }, [auth])

  const openIdentity = useCallback(() => {
    return launchAppById("identity")
  }, [])

  const openSurfaceById = useCallback((appId: string) => {
    return launchAppById(appId)
  }, [])

  const grantPendingGate = useCallback(() => {
    const pending = pendingGateStore.get()
    if (!pending) return
    grantLocalCapabilities(pending.app.appId, pending.blocked, "Approved from app launch prompt")
    pendingGateStore.set(null)
    launchApp(pending.app)
  }, [])

  const dockItems = useMemo<FloatingDockItem[]>(() => {
    const builtinApps = listBuiltinApps()
    const appsById = new Map([...builtinApps, ...catalogAppList].map((app) => [normalizeAppId(app.appId), app]))
    const installedItems = Array.from(new Set(installedAppIds.map(normalizeAppId)))
      .filter((appId) => !PINNED_DOCK_APP_IDS.has(appId))
      .filter((appId) => !isRemovedAppId(appId))
      .map((appId) => appsById.get(appId))
      .filter((app): app is NonNullable<typeof app> => Boolean(app))
      .sort((a, b) => a.name.localeCompare(b.name))
      .map<FloatingDockItem>((app) => ({
        title: app.name,
        subtitle: "Installed",
        icon: <DockIcon>{getAppIcon(app.appId)}</DockIcon>,
        kind: "app",
        onClick: () => openSurfaceById(app.appId),
      }))

    return [
      {
        title: "Codex",
        icon: <DockIcon><Bot className="h-5 w-5" /></DockIcon>,
        kind: "trigger",
        onClick: focusCodexDockInput,
      },
      {
        title: "Identity",
        icon: <DockIcon><IdCard className="h-5 w-5" /></DockIcon>,
        kind: "trigger",
        onClick: openIdentity,
      },
      ...installedItems,
      {
        title: "App Store",
        icon: <DockIcon><Store className="h-5 w-5" /></DockIcon>,
        kind: "trigger",
        onClick: () => openSurfaceById("app-store"),
      },
      {
        title: "Settings",
        icon: <DockIcon><Settings className="h-5 w-5" /></DockIcon>,
        kind: "trigger",
        onClick: () => openSurfaceById("settings"),
      },
    ]
  }, [catalogAppList, installedAppIds, openIdentity, openSurfaceById])

  const dockContext = useMemo<FloatingDockContext>(() => ({ mode: "apps" }), [])
  const showDesktopChrome = false

  const handleDockCommand = useCallback(async (command: string) => {
    return executeUiCommand(command, "dock")
  }, [])

  if (!showDesktop) {
    return (
      <div className="relative h-screen w-screen overflow-hidden bg-black">
        <div className="fixed bottom-2 left-1/2 z-50 flex -translate-x-1/2 flex-col items-center gap-1">
          <DevToolsStrip />
        </div>
        <AuthOverlay
          authState={auth.authState}
          username={auth.username}
          isLoading={auth.isLoading}
          error={auth.error}
          hasRegistered={auth.hasRegistered}
          profileSummaries={auth.profileSummaries}
          activeProfileId={auth.activeProfileId}
          onRegister={auth.register}
          onAuthenticate={auth.authenticate}
          onAuthenticateWithWebAuthn={auth.authenticateWithWebAuthn}
          onBindWebAuthn={auth.bindWebAuthn}
          onSwitchProfile={auth.switchProfile}
          onClearError={auth.clearError}
          onExportProfile={auth.exportProfile}
          onImportProfile={auth.importProfile}
        />
      </div>
    )
  }

  return (
    <div className="relative h-screen w-screen overflow-hidden bg-black">
      <div className="absolute inset-x-3 bottom-24 top-3 z-10 overflow-hidden rounded-2xl border border-white/10 bg-background shadow-2xl md:inset-x-6 md:top-6">
        <NodeNetworkBuilder />
      </div>
      <div className="fixed bottom-2 left-1/2 z-50 flex -translate-x-1/2 flex-col items-center gap-1">
        <FloatingDock
          items={dockItems}
          context={dockContext}
          onCommandSubmit={handleDockCommand}
          desktopClassName="relative z-50"
          mobileClassName="relative z-50"
        />
        <DevToolsStrip />
      </div>
      <AgentUiBridge />
      {showDesktopChrome ? <ProjectChecklist /> : null}
      {showDesktopChrome ? <ProfileMenu /> : null}
      {showDesktopChrome ? (
        <AppOverlayHost
          surfaces={appSurfaces}
          surfaceOrder={appSurfaceOrder}
          focusedSurfaceId={focusedAppSurfaceId}
          onFocus={focusAppSurface}
          onClose={handleCloseAppSurface}
        />
      ) : null}
      {pendingGate ? (
        <div className="fixed inset-0 z-[70] flex items-center justify-center bg-black/45 p-4 backdrop-blur-sm">
          <div className="h-[min(620px,calc(100vh-2rem))] w-[min(460px,calc(100vw-2rem))] overflow-hidden rounded-xl border border-border bg-background shadow-2xl">
            <CapabilityGatePrompt
              appName={pendingGate.app.name}
              blockedCapabilities={pendingGate.blocked}
              onGrant={grantPendingGate}
              onDismiss={() => pendingGateStore.set(null)}
            />
          </div>
        </div>
      ) : null}
    </div>
  )
}
