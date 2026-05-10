/**
 * Top-level provider that wires platform services together.
 * Wraps the app with context for protocol client, stores, registries, router, runtime, auth trackers.
 */

"use client"

import React, { createContext, useContext, useEffect, type ReactNode } from "react"
import { protocolClient } from "@/platform/protocol/client"
import { syncProtocolApprovals } from "@/platform/protocol/approvals"
import { nodeStore, refreshNodeStatus } from "@/stores/node-store"
import { appStore, loadApps } from "@/stores/app-store"
import { capabilityStore, loadCapabilities } from "@/stores/capability-store"
import { connectionStore } from "@/stores/connection-store"
import { permissionStore } from "@/stores/permission-store"
import { wasmRegistry } from "@/platform/runtime/wasm-registry"
import { appRegistry } from "@/platform/registries/app-registry"
import { syncBrowserCatalogRegistry } from "@/platform/registries/app-catalog-registry"
import { browserAppInstallStore } from "@/platform/runtime/browser-app-install-store"
import { capabilityRegistry } from "@/platform/registries/capability-registry"
import { toolRegistry } from "@/platform/registries/tool-registry"
import { router } from "@/platform/router/edgerun-router"
import { permissionTracker } from "@/stores/permission-tracker"
import { sessionTracker, setNodeRegistration } from "@/stores/session-tracker"
import { approvalTracker } from "@/stores/approval-tracker"
import { appRuntime } from "@/platform/runtime/app-runtime"
import { bootstrapBrowserCapabilityRuntime } from "@/platform/runtime/browser-runtime"
import { NodeWebSocketBridge } from "@/platform/runtime/edgerun-node"
import { registerPlatformTools } from "@/platform/tools/register-tools"
import { applyUiSettings, uiSettingsStore } from "@/stores/ui-settings-store"
import { bootstrapBrowserCdpRelay } from "@/platform/dev/browser-cdp-relay"
import { UsageGuideModal } from "@/platform/dev/usage-guide-modal"

interface PlatformContextValue {
  protocolClient: typeof protocolClient
  stores: {
    node: typeof nodeStore
    app: typeof appStore
    capability: typeof capabilityStore
    connection: typeof connectionStore
    permission: typeof permissionStore
  }
  registries: {
    app: typeof appRegistry
    capability: typeof capabilityRegistry
    tool: typeof toolRegistry
  }
  router: typeof router
  auth: {
    permission: typeof permissionTracker
    session: typeof sessionTracker
    approval: typeof approvalTracker
  }
  runtime: {
    wasm: typeof wasmRegistry
    app: typeof appRuntime
  }
}

const PlatformContext = createContext<PlatformContextValue | null>(null)

export function PlatformProvider({ children }: { children: ReactNode }) {
  useEffect(() => {
    let nodeBridge: NodeWebSocketBridge | null = null

    const unsubscribeBrowserApps = browserAppInstallStore.subscribe(syncBrowserCatalogRegistry)
    refreshNodeStatus()
    loadApps()
    loadCapabilities()
    registerPlatformTools()
    bootstrapBrowserCapabilityRuntime()
    const cdpRelay = bootstrapBrowserCdpRelay()
    cdpRelay?.setBackendBridge(async (message) => {
      const response = await fetch("/api/codex", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        cache: "no-store",
        body: JSON.stringify({ prompt: message, resume: true, stream: false }),
      })
      const data = await response.json().catch(() => ({})) as { text?: unknown; error?: unknown }
      if (!response.ok) throw new Error(typeof data.error === "string" ? data.error : `Backend bridge failed: HTTP ${response.status}`)
      return typeof data.text === "string" ? data.text : data
    })
    applyUiSettings(uiSettingsStore.get())
    const unsubscribeUiSettings = uiSettingsStore.subscribe(applyUiSettings)

    if (typeof window !== "undefined") {
      const stored = localStorage.getItem("edgerun_node_registration_v1")
      if (stored) {
        try {
          const reg = JSON.parse(stored)
          protocolClient.setNodeRegistration(reg)
          setNodeRegistration(reg)
        } catch {
          // Ignore parse errors
        }
      }

      const nodeWsUrl =
        localStorage.getItem("edgerun_node_ws_url_v1")
        || process.env.NEXT_PUBLIC_EDGERUN_NODE_WS_URL
      if (nodeWsUrl) {
        nodeBridge = new NodeWebSocketBridge(nodeWsUrl)
        nodeBridge.start()
      }
    }

    void syncProtocolApprovals().catch(() => undefined)
    const interval = window.setInterval(() => {
      void syncProtocolApprovals().catch(() => undefined)
    }, 2500)

    return () => {
      nodeBridge?.stop()
      unsubscribeBrowserApps()
      unsubscribeUiSettings()
      window.clearInterval(interval)
    }
  }, [])

  const value: PlatformContextValue = {
    protocolClient,
    stores: {
      node: nodeStore,
      app: appStore,
      capability: capabilityStore,
      connection: connectionStore,
      permission: permissionStore,
    },
    registries: {
      app: appRegistry,
      capability: capabilityRegistry,
      tool: toolRegistry,
    },
    router,
    auth: {
      permission: permissionTracker,
      session: sessionTracker,
      approval: approvalTracker,
    },
    runtime: {
      wasm: wasmRegistry,
      app: appRuntime,
    },
  }

  return (
    <PlatformContext.Provider value={value}>
      {children}
      <UsageGuideModal />
    </PlatformContext.Provider>
  )
}

export function usePlatform(): PlatformContextValue {
  const context = useContext(PlatformContext)
  if (!context) {
    throw new Error("usePlatform must be used within a PlatformProvider")
  }
  return context
}
