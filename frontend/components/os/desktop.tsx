"use client"

import { useCallback, useMemo, useState } from "react"
import type React from "react"
import { Contact, IdCard, Inbox, Settings, Store } from "lucide-react"
import { AuthOverlay } from "./auth-overlay"
import { AppStore } from "./app-store"
import { ContactsApp } from "./contacts-app"
import { IdentityApp } from "./identity-app"
import { MessagesApp } from "./messages-app"
import { ProfileMenu } from "./profile-menu"
import { useAuth, type ContactRecord } from "@/hooks/use-auth"
import { FloatingDock, type FloatingDockContext, type FloatingDockItem } from "@/components/ui/floating-dock"
import { addLog, terminalLogsStore } from "@/stores/desktop-store"
import type { AppDefinition } from "@/platform/types/app-definition"
import { createAppLaunchPlan } from "@/platform/runtime/app-manager"

function DockIcon({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex h-full w-full items-center justify-center rounded-full text-white">
      {children}
    </div>
  )
}

export function Desktop() {
  const auth = useAuth()
  const [openApp, setOpenApp] = useState<"identity" | "contacts" | "messages" | "app-store" | null>(null)
  const [launchedApp, setLaunchedApp] = useState<AppDefinition | null>(null)
  const [messageRecipientId, setMessageRecipientId] = useState<string | undefined>(undefined)
  const showDesktop = auth.authState === "authenticated"

  const dockItems = useMemo<FloatingDockItem[]>(() => [
    {
      title: "Identity",
      icon: <DockIcon><IdCard className="h-5 w-5" /></DockIcon>,
      kind: "trigger",
      onClick: () => setOpenApp("identity"),
    },
    {
      title: "Contacts",
      icon: <DockIcon><Contact className="h-5 w-5" /></DockIcon>,
      kind: "trigger",
      onClick: () => setOpenApp("contacts"),
    },
    {
      title: "Messages",
      icon: <DockIcon><Inbox className="h-5 w-5" /></DockIcon>,
      kind: "trigger",
      onClick: () => {
        setMessageRecipientId(undefined)
        setOpenApp("messages")
      },
    },
    {
      title: "App Store",
      icon: <DockIcon><Store className="h-5 w-5" /></DockIcon>,
      kind: "trigger",
      onClick: () => setOpenApp("app-store"),
    },
    {
      title: "Settings",
      icon: <DockIcon><Settings className="h-5 w-5" /></DockIcon>,
      kind: "trigger",
      onClick: () => addLog("info", "Settings UI is hidden in the onboarding-only shell."),
    },
  ], [])

  const openMessagesForContact = useCallback((contact: ContactRecord) => {
    setMessageRecipientId(contact.identityIdHex)
    setOpenApp("messages")
  }, [])

  const dockContext = useMemo<FloatingDockContext>(() => ({ mode: "apps" }), [])

  const handleDockCommand = useCallback((command: string) => {
    if (command === "/lock") {
      auth.lock()
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
          profileSummaries={auth.profileSummaries}
          activeProfileId={auth.activeProfileId}
          onRegister={auth.register}
          onAuthenticate={auth.authenticate}
          onAuthenticateWithWebAuthn={auth.authenticateWithWebAuthn}
          onSwitchProfile={auth.switchProfile}
          onClearError={auth.clearError}
          onExportProfile={auth.exportProfile}
          onImportProfile={auth.importProfile}
        />
      </div>
    )
  }

  return (
    <div className="relative h-screen w-screen overflow-hidden bg-zinc-950">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_42%,rgba(80,120,120,0.16),transparent_34%)]" />
      <div className="absolute left-1/2 top-1/2 w-[min(760px,calc(100vw-3rem))] -translate-x-1/2 -translate-y-1/2 text-center">
        <div className="mx-auto mb-4 h-2 w-2 rounded-full bg-primary shadow-[0_0_28px_var(--primary)]" />
        <p className="font-mono text-xs uppercase tracking-[0.24em] text-muted-foreground">Identity unlocked</p>
        <p className="mt-2 text-sm text-muted-foreground">
          Your profile is open in memory. Locking it removes usable key material from this session.
        </p>
      </div>
      <FloatingDock
        items={dockItems}
        context={dockContext}
        onCommandSubmit={handleDockCommand}
        desktopClassName="fixed bottom-4 left-1/2 z-50 -translate-x-1/2"
        mobileClassName="fixed bottom-4 left-1/2 z-50 -translate-x-1/2"
      />
      <ProfileMenu />
      {openApp === "identity" ? <IdentityApp onClose={() => setOpenApp(null)} /> : null}
      {openApp === "contacts" ? <ContactsApp onClose={() => setOpenApp(null)} onMessage={openMessagesForContact} /> : null}
      {openApp === "messages" ? <MessagesApp onClose={() => setOpenApp(null)} initialRecipientId={messageRecipientId} /> : null}
      {openApp === "app-store" ? (
        <div className="fixed left-1/2 top-1/2 z-40 flex h-[min(760px,calc(100vh-7rem))] w-[min(1120px,calc(100vw-3rem))] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-lg border border-border bg-background/96 shadow-2xl backdrop-blur-xl">
          <button onClick={() => setOpenApp(null)} className="absolute right-3 top-3 z-10 rounded-md border border-border bg-background/80 px-2 py-1 text-xs text-muted-foreground hover:bg-secondary hover:text-foreground">
            Close
          </button>
          <AppStore onLaunchApp={(app) => setLaunchedApp(app)} />
        </div>
      ) : null}
      {launchedApp ? (
        <div className="fixed left-1/2 top-1/2 z-50 flex h-[min(820px,calc(100vh-5rem))] w-[min(1240px,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-lg border border-border bg-background shadow-2xl">
          <div className="flex h-10 shrink-0 items-center justify-between border-b border-border px-3">
            <div className="min-w-0 truncate text-xs font-semibold text-foreground">{launchedApp.name}</div>
            <button onClick={() => setLaunchedApp(null)} className="rounded-md border border-border bg-background/80 px-2 py-1 text-xs text-muted-foreground hover:bg-secondary hover:text-foreground">
              Close
            </button>
          </div>
          <div className="min-h-0 flex-1">
            {createAppLaunchPlan(launchedApp, { launchApp: setLaunchedApp }).component}
          </div>
        </div>
      ) : null}
    </div>
  )
}
