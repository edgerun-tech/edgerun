"use client"

import { useCallback, useMemo } from "react"
import type React from "react"
import { useStore } from "@nanostores/react"
import { Contact, IdCard, Inbox, Settings, Store } from "lucide-react"
import { AuthOverlay } from "./auth-overlay"
import { AppOverlayHost } from "./app-overlay-host"
import { PeopleApp, type PeopleTab } from "./people-app"
import { ProfileMenu } from "./profile-menu"
import { CapabilityGatePrompt } from "@/components/capability-gate-prompt"
import { useAuth } from "@/hooks/use-auth"
import { FloatingDock, type FloatingDockContext, type FloatingDockItem } from "@/components/ui/floating-dock"
import { addLog, appSurfaceOrderStore, appSurfacesStore, focusedAppSurfaceStore, focusAppSurface, pendingGateStore, terminalLogsStore } from "@/stores/desktop-store"
import { getAppIcon, handleCloseAppSurface, launchApp, launchAppById } from "@/stores/app-launcher"
import { getBuiltinApp } from "@/platform/registries/builtin-app-registry"
import { listBuiltinApps } from "@/platform/registries/builtin-app-registry"
import { catalogApps, getCatalogApp } from "@/platform/registries/app-catalog-registry"
import { grantLocalCapabilities } from "@/stores/local-capability-grants-store"
import { installedAppIdsStore, normalizeAppId } from "@/stores/installed-apps-store"

function DockIcon({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex h-full w-full items-center justify-center rounded-full text-white">
      {children}
    </div>
  )
}

function resolveCommandAppId(value: string): string | null {
  const normalized = normalizeAppId(value)
  if (listBuiltinApps().some((app) => app.appId === normalized) || getCatalogApp(normalized)) return normalized

  const term = value.trim().toLowerCase()
  const builtin = listBuiltinApps().find((app) => app.name.toLowerCase() === term || app.name.toLowerCase().startsWith(term))
  return builtin?.appId ?? null
}

function sendAssistantInput(message: string) {
  window.dispatchEvent(new CustomEvent("edgerun:assistant-input", {
    detail: { message, submit: true },
  }))
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

  const openIdentity = useCallback(() => {
    return launchAppById("identity")
  }, [])

  const openSurfaceById = useCallback((appId: string) => {
    return launchAppById(appId)
  }, [])

  const openPeople = useCallback((initialTab: PeopleTab, initialRecipientId?: string) => {
    const app = getBuiltinApp("people")
    if (!app) return null
    return launchApp(app, <PeopleApp initialTab={initialTab} initialRecipientId={initialRecipientId} />)
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
        title: "Identity",
        icon: <DockIcon><IdCard className="h-5 w-5" /></DockIcon>,
        kind: "trigger",
        onClick: openIdentity,
      },
      {
        title: "Contacts",
        icon: <DockIcon><Contact className="h-5 w-5" /></DockIcon>,
        kind: "trigger",
        onClick: () => openPeople("contacts"),
      },
      {
        title: "Messages",
        icon: <DockIcon><Inbox className="h-5 w-5" /></DockIcon>,
        kind: "trigger",
        onClick: () => openPeople("messages"),
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
  }, [catalogAppList, installedAppIds, openIdentity, openPeople, openSurfaceById])

  const dockContext = useMemo<FloatingDockContext>(() => ({ mode: "apps" }), [])

  const handleDockCommand = useCallback((command: string) => {
    if (command === "/lock") {
      auth.lock()
      return
    }

    if (command.startsWith("~")) {
      const message = command.slice(1).trim()
      if (message) sendAssistantInput(message)
      return
    }

    if (command.startsWith("/open ")) {
      const target = command.slice("/open ".length).trim().toLowerCase()
      if (target.includes("contact")) {
        openPeople("contacts")
        return
      }
      if (target.includes("message") || target.includes("chat")) {
        openPeople("messages")
        return
      }
      const appId = resolveCommandAppId(target)
      if (appId && launchAppById(appId)) {
        addLog("success", `Opened ${appId}`)
      } else {
        addLog("warning", `Could not open app: ${command.slice("/open ".length).trim()}`)
      }
      return
    }

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
  }, [auth, openPeople])

  if (!showDesktop) {
    return (
      <div className="relative h-screen w-screen overflow-hidden bg-background">
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
    <div className="relative h-screen w-screen overflow-hidden bg-[oklch(0.075_0.018_255)]">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_36%,rgba(62,128,255,0.20),transparent_32%),linear-gradient(180deg,rgba(255,255,255,0.035),transparent_28%,rgba(0,0,0,0.24))]" />
      <div className="absolute inset-x-4 top-4 flex items-center justify-center sm:top-6">
          <div className="w-[calc(100vw-2rem)] max-w-[560px] rounded-full border border-white/10 bg-white/[0.055] px-4 py-2 text-center shadow-2xl backdrop-blur-xl">
          <p className="text-xs font-medium text-foreground">Identity unlocked</p>
          <p className="mt-0.5 text-[11px] leading-4 text-muted-foreground">
            Your keys are available until you lock this browser session.
          </p>
        </div>
      </div>
      <FloatingDock
        items={dockItems}
        context={dockContext}
        onCommandSubmit={handleDockCommand}
        desktopClassName="fixed bottom-5 left-1/2 z-50 -translate-x-1/2"
        mobileClassName="fixed bottom-5 left-1/2 z-50 -translate-x-1/2"
      />
      <ProfileMenu />
      <AppOverlayHost
        surfaces={appSurfaces}
        surfaceOrder={appSurfaceOrder}
        focusedSurfaceId={focusedAppSurfaceId}
        onFocus={focusAppSurface}
        onClose={handleCloseAppSurface}
      />
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
