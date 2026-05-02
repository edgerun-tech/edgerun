"use client"

import { useState } from "react"
import { useStore } from "@nanostores/react"
import {
  Upload,
  Trash2,
  Package,
  Lock,
  Shield,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { useApps } from "@/platform/ui/useApps"
import { useCapabilities } from "@/platform/ui/useCapabilities"
import { useRuntime } from "@/platform/ui/useRuntime"
import { getBuiltinApp, listBuiltinApps, getIconById } from "@/platform/registries/builtin-app-registry"
import type { AppDefinition } from "@/platform/types/app-definition"
import { getDashboardMode, isDemoMode } from "@/platform/runtime/dashboard-mode"

// Demo-only WASM installer component (isolated)
function WasmInstallerDemo({
  onCancel,
}: {
  onCancel: () => void
}) {
  return (
    <div className="flex h-full items-center justify-center">
      <div className="text-center">
        <p className="text-sm text-muted-foreground mb-2">WASM installation is a demo feature.</p>
        <button
          onClick={onCancel}
          className="text-xs text-muted-foreground hover:text-foreground"
        >
          Close
        </button>
      </div>
    </div>
  )
}

// Demo badge component
function DemoBadge() {
  if (!isDemoMode()) return null
  return (
    <span className="ml-2 rounded bg-yellow-500/20 px-1.5 py-0.5 text-[9px] font-medium text-yellow-400">
      Demo
    </span>
  )
}

interface AppStoreProps {
  onLaunchApp: (app: AppDefinition) => void
  onAppBlocked: (app: AppDefinition, blocked: string[]) => void
  runningApps: string[]
}

export function AppStore({ onLaunchApp, onAppBlocked, runningApps }: AppStoreProps) {
  const [showInstaller, setShowInstaller] = useState(false)
  const { apps: installedApps, registry } = useApps()
  const { listGrantsForApp, resolveCapabilityForApp } = useCapabilities()
  const { getCachedWasm } = useRuntime()
  const mode = getDashboardMode()

  // Merge builtin app definitions with installed app state
  const builtinApps = listBuiltinApps()
  const allApps = builtinApps.map((def) => {
    const installed = installedApps.find((a) => a.appId === def.appId)
    return {
      ...def,
      status: installed ? "installed" : def.status,
    }
  })

  const installedWasm = installedApps.filter((app) => app.wasmObjectRef)

  if (showInstaller) {
    return (
      <WasmInstallerDemo
        onCancel={() => setShowInstaller(false)}
      />
    )
  }

  return (
    <div className="flex h-full flex-col p-4">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground flex items-center">
            App Store
            {mode === "demo" && <DemoBadge />}
            {mode === "offline" && (
              <span className="ml-2 rounded bg-red-500/20 px-1.5 py-0.5 text-[9px] font-medium text-red-400">
                Offline
              </span>
            )}
          </h2>
          <p className="text-xs text-muted-foreground">
            {mode === "real" ? "Launch or install applications" : "Demo application listings"}
          </p>
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
                    className="flex flex-1 flex-col text-left transition-all hover:border-primary/50"
                  >
                    <div className="flex items-start justify-between">
                      <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-muted text-muted-foreground transition-colors group-hover:bg-primary group-hover:text-primary-foreground">
                        <Package className="h-5 w-5" />
                      </div>
                      {isRunning && <span className="h-1.5 w-1.5 rounded-full bg-[var(--status-online)]" />}
                    </div>
                    <div className="mt-2">
                      <h3 className="text-sm font-medium text-foreground">{app.name}</h3>
                      <p className="text-xs text-muted-foreground">
                        {wasmBytes ? `${(wasmBytes.length / 1024).toFixed(1)} KB` : "Loading..."}
                      </p>
                    </div>
                  </button>
                </div>
              )
            })}
          </div>
        </div>
      )}

      <div className="mb-2">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Built-in Apps
        </h3>
      </div>

      <div className="grid flex-1 grid-cols-2 gap-3 overflow-auto">
        {allApps.map((app) => {
          const isRunning = runningApps.includes(app.appId)
          const satisfaction = resolveCapabilityForApp?.(app.appId)
          const isBlocked = satisfaction && !satisfaction.satisfied
          const blockedIds = satisfaction?.missing || []

          return (
            <div key={app.appId} className="group relative">
              <button
                onClick={() => {
                  if (isBlocked) {
                    onAppBlocked(app, blockedIds)
                  } else {
                    onLaunchApp(app)
                  }
                }}
                disabled={!!isBlocked}
                className={cn(
                  "flex w-full flex-col rounded-lg border border-border bg-secondary/50 p-3 text-left transition-all",
                  isBlocked
                    ? "cursor-not-allowed opacity-60"
                    : "hover:border-primary/50 hover:bg-secondary",
                  isRunning && "border-primary/30 bg-primary/5"
                )}
              >
                <div className="flex items-start justify-between">
                  <div className={cn(
                    "flex h-9 w-9 items-center justify-center rounded-lg bg-muted text-muted-foreground transition-colors",
                    !isBlocked && "group-hover:bg-primary group-hover:text-primary-foreground",
                    isRunning && "bg-primary/20 text-primary"
                  )}>
                    {isBlocked ? <Lock className="h-4 w-4" /> : getIconById(app.iconId)}
                  </div>
                  <div className="flex items-center gap-1">
                    {isBlocked && (
                      <span className="rounded bg-[var(--status-error)]/20 text-[var(--status-error)] px-1.5 py-0.5 text-[10px] font-medium">
                        Locked
                      </span>
                    )}
                    {app.source === "demo" && (
                      <span className="rounded bg-yellow-500/20 text-yellow-400 px-1.5 py-0.5 text-[10px] font-medium">
                        Demo
                      </span>
                    )}
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

                {app.requiredCapabilityIds.length > 0 && (
                  <div className="mt-auto flex flex-wrap gap-1 pt-2">
                    {app.requiredCapabilityIds.map((capId) => {
                      const granted = !blockedIds.includes(capId)
                      return (
                        <span
                          key={capId}
                          className={cn(
                            "flex items-center gap-1 rounded px-1.5 py-0.5 text-[9px] font-medium",
                            granted
                              ? "bg-primary/10 text-primary"
                              : "bg-[var(--status-error)]/10 text-[var(--status-error)]"
                          )}
                        >
                          <Shield className="h-2.5 w-2.5" />
                          {capId}
                        </span>
                      )
                    })}
                  </div>
                )}

                {app.footprint && (
                  <div className="mt-1 flex items-center gap-3 text-[10px] text-muted-foreground">
                    {app.footprint.ramBytes && <span>RAM: {(app.footprint.ramBytes / 1024 / 1024).toFixed(0)} MB</span>}
                    {app.footprint.cpuMillisPerSec && <span>CPU: {(app.footprint.cpuMillisPerSec / 10).toFixed(1)}%</span>}
                  </div>
                )}
              </button>
            </div>
          )
        })}
      </div>
    </div>
  )
}
