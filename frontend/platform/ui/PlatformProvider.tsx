/**
 * Top-level provider that wires platform services together.
 * Wraps the app with context for protocol client, stores, registries, router, runtime, auth trackers.
 */

"use client"

import React, { createContext, useContext, useEffect, type ReactNode } from "react"
import { protocolClient } from "@/platform/protocol/client"
import { syncProtocolApprovals } from "@/platform/protocol/approvals"
import { nodeStore, refreshNodeStatus } from "@/platform/state/node-store"
import { appStore, loadApps } from "@/platform/state/app-store"
import { capabilityStore, loadCapabilities } from "@/platform/state/capability-store"
import { connectionStore } from "@/platform/state/connection-store"
import { permissionStore } from "@/platform/state/permission-store"
import { wasmRegistry } from "@/platform/runtime/wasm-registry"
import { appRegistry } from "@/platform/registries/app-registry"
import { seedBuiltinCatalogApps } from "@/platform/registries/app-catalog-registry"
import { capabilityRegistry } from "@/platform/registries/capability-registry"
import { connectionRegistry } from "@/platform/registries/connection-registry"
import { toolRegistry } from "@/platform/registries/tool-registry"
import { router } from "@/platform/router/edgerun-router"
import { permissionTracker } from "@/platform/auth/permission-tracker"
import { sessionTracker, setNodeRegistration } from "@/platform/auth/session-tracker"
import { approvalTracker } from "@/platform/auth/approval-tracker"
import { appRuntime } from "@/platform/runtime/app-runtime"
import { registerPlatformTools } from "@/platform/tools/register-tools"

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
    connection: typeof connectionRegistry
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
    seedBuiltinCatalogApps()
    refreshNodeStatus()
    loadApps()
    loadCapabilities()
    registerPlatformTools()

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
    }

    void syncProtocolApprovals().catch(() => undefined)
    const interval = window.setInterval(() => {
      void syncProtocolApprovals().catch(() => undefined)
    }, 2500)

    return () => window.clearInterval(interval)
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
      connection: connectionRegistry,
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
