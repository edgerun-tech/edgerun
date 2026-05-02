"use client"

import { useState } from "react"
import { useStore } from "@nanostores/react"
import { Terminal, Code2, Database, Globe, FileText, GitBranch, Cpu, Network, HelpCircle, Users, Phone, MessageSquare, Wallet, Calculator, Upload, Trash2, Package, Lock, Shield, Activity, Sparkles, Workflow } from "lucide-react"
import { cn } from "@/lib/utils"
import { WasmInstaller, type WasmInstallResult } from "@/components/wasm-installer"
import { useApps } from "@/platform/ui/useApps"
import { useCapabilities } from "@/platform/ui/useCapabilities"
import { useRuntime } from "@/platform/ui/useRuntime"
import { type GuestContext, CAPABILITY_REGISTRY } from "@/lib/capabilities"

export interface AppDefinition {
  id: string
  name: string
  description: string
  icon: React.ReactNode
  ram: string
  cpu: string
  price: string | "Free"
  wasmUrl?: string
  isWasm?: boolean
  requiredCapabilities?: string[]
  optionalCapabilities?: string[]
}

export const availableApps: AppDefinition[] = [
  {
    id: "terminal",
    name: "Terminal",
    description: "System shell & logs",
    icon: <Terminal className="h-5 w-5" />,
    ram: "32MB",
    cpu: "0.1%",
    price: "Free",
  },
  {
    id: "code-runner",
    name: "Code Runner",
    description: "Execute WASM modules",
    icon: <Code2 className="h-5 w-5" />,
    ram: "128MB",
    cpu: "2.5%",
    price: "Free",
  },
  {
    id: "db-explorer",
    name: "DB Explorer",
    description: "Query distributed state",
    icon: <Database className="h-5 w-5" />,
    ram: "64MB",
    cpu: "0.5%",
    price: "$5/mo",
    optionalCapabilities: ["node_connection"],
  },
  {
    id: "network-monitor",
    name: "Network",
    description: "P2P connection status",
    icon: <Network className="h-5 w-5" />,
    ram: "48MB",
    cpu: "0.3%",
    price: "Free",
  },
  {
    id: "resource-monitor",
    name: "Resource Monitor",
    description: "System metrics & performance",
    icon: <Activity className="h-5 w-5" />,
    ram: "32MB",
    cpu: "0.5%",
    price: "Free",
  },
  {
    id: "file-browser",
    name: "Files",
    description: "Virtual filesystem",
    icon: <FileText className="h-5 w-5" />,
    ram: "24MB",
    cpu: "0.1%",
    price: "Free",
    optionalCapabilities: ["filesystem"],
  },
  {
    id: "git-sync",
    name: "Git Sync",
    description: "Decentralized repos",
    icon: <GitBranch className="h-5 w-5" />,
    ram: "96MB",
    cpu: "1.2%",
    price: "$3/mo",
    optionalCapabilities: ["node_connection"],
  },
  {
    id: "web-server",
    name: "Web Server",
    description: "Serve static content",
    icon: <Globe className="h-5 w-5" />,
    ram: "64MB",
    cpu: "0.8%",
    price: "Free",
    optionalCapabilities: ["network_access"],
  },
  {
    id: "compute-node",
    name: "Compute",
    description: "Distributed processing",
    icon: <Cpu className="h-5 w-5" />,
    ram: "256MB",
    cpu: "5.0%",
    price: "$10/mo",
    optionalCapabilities: ["node_connection"],
  },
  {
    id: "contacts",
    name: "Contacts",
    description: "Manage your peer network",
    icon: <Users className="h-5 w-5" />,
    ram: "12MB",
    cpu: "0.0%",
    price: "Free",
    requiredCapabilities: ["identity"],
  },
  {
    id: "calling",
    name: "Calling",
    description: "Encrypted P2P voice calls",
    icon: <Phone className="h-5 w-5" />,
    ram: "48MB",
    cpu: "1.0%",
    price: "Free",
    requiredCapabilities: ["identity", "voice_call"],
  },
  {
    id: "chat",
    name: "Chat",
    description: "E2E encrypted messaging",
    icon: <MessageSquare className="h-5 w-5" />,
    ram: "24MB",
    cpu: "0.2%",
    price: "Free",
    requiredCapabilities: ["identity"],
  },
  {
    id: "ai-assistant",
    name: "AI Assistant",
    description: "LLM-powered helper",
    icon: <Sparkles className="h-5 w-5" />,
    ram: "128MB",
    cpu: "1.0%",
    price: "Free",
  },
  {
    id: "workflow-builder",
    name: "Workflow Builder",
    description: "Create & manage automation workflows",
    icon: <Workflow className="h-5 w-5" />,
    ram: "64MB",
    cpu: "0.5%",
    price: "Free",
  },
  {
    id: "wallet",
    name: "Wallet",
    description: "EDGE token & payments",
    icon: <Wallet className="h-5 w-5" />,
    ram: "16MB",
    cpu: "0.1%",
    price: "Free",
    requiredCapabilities: ["identity", "payments"],
  },
  {
    id: "calculator",
    name: "Calculator",
    description: "System utility",
    icon: <Calculator className="h-5 w-5" />,
    ram: "4MB",
    cpu: "0.0%",
    price: "Free",
  },
  {
    id: "help",
    name: "Help & Onboarding",
    description: "Platform guide & setup",
    icon: <HelpCircle className="h-5 w-5" />,
    ram: "2MB",
    cpu: "0.0%",
    price: "Free",
  },
  {
    id: "wasm-hello",
    name: "WASM Hello",
    description: "Native EdgeRun WASM app",
    icon: <Cpu className="h-5 w-5" />,
    ram: "4MB",
    cpu: "0.5%",
    price: "Free",
    wasmUrl: "/edgerun-app.wasm",
    isWasm: true,
  },
  {
    id: "wasm-calculator",
    name: "Calculator (WASM)",
    description: "WASM-based calculator",
    icon: <Calculator className="h-5 w-5" />,
    ram: "2MB",
    cpu: "0.1%",
    price: "Free",
    wasmUrl: "/calculator.wasm",
    isWasm: true,
  },
]

function wasmAppDef(name: string, wasmUrl: string): AppDefinition {
  return {
    id: `wasm-${name}`,
    name,
    description: "Custom WASM app",
    icon: <Package className="h-5 w-5" />,
    ram: "8MB",
    cpu: "1.0%",
    price: "Free",
    wasmUrl,
    isWasm: true,
  }
}

interface AppStoreProps {
  onLaunchApp: (app: AppDefinition) => void
  onAppBlocked: (app: AppDefinition, blocked: string[]) => void
  runningApps: string[]
  onRemoveWasm?: (name: string) => void
  guestCtx: GuestContext
}

export function AppStore({ onLaunchApp, onAppBlocked, runningApps, onRemoveWasm, guestCtx }: AppStoreProps) {
  const [showInstaller, setShowInstaller] = useState(false)
  const { apps, useApps } = useApps()
  const { listGrantsForApp } = useCapabilities()
  const { getCachedWasm, removeWasm } = useRuntime()

  const installedWasm = apps.filter((app) => app.wasmObjectRef)

  if (showInstaller) {
    return (
      <WasmInstaller
        onInstall={(result) => {
          // Use platform runtime to cache WASM
          cacheWasm(result.name, result.wasmBytes, result.hash)
          setShowInstaller(false)
        }}
        onCancel={() => setShowInstaller(false)}
      />
    )
  }

  return (
    <div className="flex h-full flex-col p-4">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground">App Store</h2>
          <p className="text-xs text-muted-foreground">Launch or install applications</p>
        </div>
        <button
          onClick={() => setShowInstaller(true)}
          className="flex items-center gap-1.5 rounded-md bg-primary/10 px-2.5 py-1.5 text-xs font-medium text-primary hover:bg-primary/20 transition-colors"
        >
          <Upload className="h-3.5 w-3.5" />
          Install WASM
        </button>
      </div>

      {installedWasm.length > 0 && (
        <div className="mb-4">
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Installed WASM Apps
          </h3>
          <div className="grid grid-cols-2 gap-3">
            {installedWasm.map((app) => {
              const isRunning = runningApps.includes(app.appId)
              const wasmBytes = getCachedWasm(app.appId)
              return (
                <div key={app.appId} className="group flex flex-col rounded-lg border border-border bg-secondary/50 p-3">
                  <button
                    onClick={() => onLaunchApp(app as unknown as AppDefinition)}
                    className={cn(
                      "flex flex-1 flex-col text-left transition-all hover:border-primary/50",
                      isRunning && "border-primary/30"
                    )}
                  >
                    <div className="flex items-start justify-between">
                      <div className={cn(
                        "flex h-9 w-9 items-center justify-center rounded-lg bg-muted text-muted-foreground transition-colors group-hover:bg-primary group-hover:text-primary-foreground",
                        isRunning && "bg-primary/20 text-primary"
                      )}>
                        <Package className="h-5 w-5" />
                      </div>
                      {isRunning && <span className="h-1.5 w-1.5 rounded-full bg-[var(--status-online)]" />}
                    </div>
                    <div className="mt-2">
                      <h3 className="text-sm font-medium text-foreground">{app.name}</h3>
                      <p className="text-xs text-muted-foreground">
                        {wasmBytes ? `${(wasmBytes.length / 1024).toFixed(1)} KB` : 'Loading...'}
                      </p>
                    </div>
                  </button>
                  {onRemoveWasm && (
                    <button
                      onClick={() => onRemoveWasm(app.name)}
                      className="mt-2 flex items-center justify-center gap-1 rounded border border-border/50 bg-transparent px-2 py-1 text-[10px] text-muted-foreground hover:border-[var(--status-error)]/50 hover:text-[var(--status-error)] transition-colors"
                    >
                      <Trash2 className="h-3 w-3" />
                      Remove
                    </button>
                  )}
                </div>
              )
            })}
          </div>
        </div>
      )}

      <div className="mb-2">
        <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Built-in Apps
        </h3>
      </div>

      <div className="grid flex-1 grid-cols-2 gap-3 overflow-auto">
        {availableApps.map((app) => {
          const isRunning = runningApps.includes(app.id)
          const required = app.requiredCapabilities ?? []
          const optional = app.optionalCapabilities ?? []
          const check = checkCapabilities(required, optional, guestCtx)
          const isBlocked = check.blocked.length > 0

          return (
            <div key={app.id} className="group relative">
              <button
                onClick={() => {
                  if (isBlocked) {
                    onAppBlocked(app, check.blocked.map((b) => b.info.label))
                  } else {
                    onLaunchApp(app)
                  }
                }}
                disabled={isBlocked}
                className={cn(
                  "flex w-full flex-col rounded-lg border border-border bg-secondary/50 p-3 text-left transition-all",
                  isBlocked
                    ? "cursor-not-allowed opacity-60"
                    : cn(
                        "hover:border-primary/50 hover:bg-secondary",
                        isRunning && "border-primary/30 bg-primary/5"
                      )
                )}
              >
                <div className="flex items-start justify-between">
                  <div className={cn(
                    "flex h-9 w-9 items-center justify-center rounded-lg bg-muted text-muted-foreground transition-colors",
                    !isBlocked && "group-hover:bg-primary group-hover:text-primary-foreground",
                    isRunning && "bg-primary/20 text-primary"
                  )}>
                    {isBlocked ? <Lock className="h-4 w-4" /> : app.icon}
                  </div>
                  <div className="flex items-center gap-1">
                    {check.blocked.length > 0 && (
                      <span className="rounded bg-[var(--status-error)]/20 text-[var(--status-error)] px-1.5 py-0.5 text-[10px] font-medium">
                        Locked
                      </span>
                    )}
                    <span className={cn(
                      "rounded px-1.5 py-0.5 text-[10px] font-medium",
                      app.price === "Free"
                        ? "bg-primary/20 text-primary"
                        : "bg-[var(--status-warning)]/20 text-[var(--status-warning)]"
                    )}>
                      {app.price}
                    </span>
                  </div>
                </div>

                <div className="mt-2">
                  <div className="flex items-center gap-1.5">
                    <h3 className="text-sm font-medium text-foreground">{app.name}</h3>
                    {isRunning && !isBlocked && (
                      <span className="h-1.5 w-1.5 rounded-full bg-[var(--status-online)]" />
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground">{app.description}</p>
                </div>

                {required.length > 0 && (
                  <div className="mt-auto flex flex-wrap gap-1 pt-2">
                    {required.map((cap) => {
                      const info = CAPABILITY_REGISTRY[cap as never]
                      const available = guestCtx.hasIdentity || !info.requiresAuth
                      return (
                        <span
                          key={cap}
                          className={cn(
                            "flex items-center gap-1 rounded px-1.5 py-0.5 text-[9px] font-medium",
                            available
                              ? "bg-primary/10 text-primary"
                              : "bg-[var(--status-error)]/10 text-[var(--status-error)]"
                          )}
                        >
                          <Shield className="h-2.5 w-2.5" />
                          {info.label}
                        </span>
                      )
                    })}
                  </div>
                )}

                <div className="mt-1 flex items-center gap-3 text-[10px] text-muted-foreground">
                  <span>RAM: {app.ram}</span>
                  <span>CPU: {app.cpu}</span>
                </div>
              </button>
            </div>
          )
        })}
      </div>
    </div>
  )
}
