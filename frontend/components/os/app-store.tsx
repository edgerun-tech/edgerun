"use client"

import { useMemo } from "react"
import { useStore } from "@nanostores/react"
import { Download, Package, Trash2, Play, Shield } from "lucide-react"
import { cn } from "@/lib/utils"
import { getIconById, listBuiltinApps } from "@/platform/registries/builtin-app-registry"
import type { AppDefinition } from "@/platform/types/app-definition"
import {
  installedAppIdsStore,
  installApp,
  uninstallApp,
  isCoreApp,
} from "@/stores/installed-apps-store"

interface AppStoreProps {
  onLaunchApp?: (app: AppDefinition) => void
}

export function AppStore({ onLaunchApp }: AppStoreProps) {
  const installedIds = useStore(installedAppIdsStore)
  const apps = useMemo(() => listBuiltinApps(), [])

  return (
    <div className="flex h-full flex-col p-4">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h2 className="flex items-center gap-2 text-lg font-semibold text-foreground">
            <Package className="h-5 w-5" />
            App Store
          </h2>
          <p className="text-xs text-muted-foreground">
            Install apps to show them in the dock. Uninstalled app code is lazy and stays unloaded.
          </p>
        </div>
        <div className="rounded-md border border-border bg-secondary/50 px-2 py-1 font-mono text-[10px] text-muted-foreground">
          {installedIds.length} installed
        </div>
      </div>

      <div className="grid flex-1 grid-cols-2 gap-3 overflow-auto pr-1">
        {apps.map((app) => {
          const installed = installedIds.includes(app.appId) || isCoreApp(app.appId)
          const core = isCoreApp(app.appId)

          return (
            <div
              key={app.appId}
              className={cn(
                "group flex min-h-[132px] flex-col rounded-lg border border-border bg-secondary/40 p-3 transition-colors",
                installed && "border-primary/30 bg-primary/5",
              )}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-muted text-muted-foreground transition-colors group-hover:bg-primary group-hover:text-primary-foreground">
                  {getIconById(app.iconId)}
                </div>
                <div className="flex items-center gap-1">
                  {app.requiredCapabilityIds.length > 0 && (
                    <span className="rounded bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary">
                      <Shield className="mr-1 inline h-2.5 w-2.5" />
                      cap
                    </span>
                  )}
                  {installed && (
                    <span className="rounded bg-[var(--status-online)]/15 px-1.5 py-0.5 text-[10px] font-medium text-[var(--status-online)]">
                      Installed
                    </span>
                  )}
                </div>
              </div>

              <div className="mt-2 min-h-0 flex-1">
                <h3 className="text-sm font-medium text-foreground">{app.name}</h3>
                <p className="mt-0.5 line-clamp-2 text-xs text-muted-foreground">{app.description}</p>
              </div>

              <div className="mt-3 flex items-center gap-2">
                {installed ? (
                  <button
                    onClick={() => onLaunchApp?.(app)}
                    className="flex flex-1 items-center justify-center gap-1.5 rounded-md bg-primary px-2 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                  >
                    <Play className="h-3.5 w-3.5" />
                    Open
                  </button>
                ) : (
                  <button
                    onClick={() => installApp(app.appId)}
                    className="flex flex-1 items-center justify-center gap-1.5 rounded-md bg-primary px-2 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                  >
                    <Download className="h-3.5 w-3.5" />
                    Install
                  </button>
                )}

                <button
                  onClick={() => uninstallApp(app.appId)}
                  disabled={!installed || core}
                  title={core ? "Core app cannot be uninstalled" : "Uninstall"}
                  className="flex h-8 w-8 items-center justify-center rounded-md bg-secondary text-muted-foreground transition-colors hover:bg-[var(--status-error)]/15 hover:text-[var(--status-error)] disabled:cursor-not-allowed disabled:opacity-40"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
