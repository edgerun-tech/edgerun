"use client"

import { useMemo } from "react"
import { useStore } from "@nanostores/react"
import { Download, Package, Trash2, Play, Shield, CheckCircle2, BadgeCheck } from "lucide-react"
import { cn } from "@/lib/utils"
import { getIconById, listBuiltinApps } from "@/platform/registries/builtin-app-registry"
import { appCatalogRegistry, installCatalogApp, listCatalogApps } from "@/platform/registries/app-catalog-registry"
import type { AppDefinition } from "@/platform/types/app-definition"
import {
  installedAppIdsStore,
  installApp,
  uninstallApp,
  isCoreApp,
  normalizeAppId,
} from "@/stores/installed-apps-store"
import {
  localCapabilityGrantsStore,
  hasLocalCapabilityGrant,
  revokeAllLocalCapabilityGrants,
} from "@/stores/local-capability-grants-store"
import { getCapabilityInfo, riskTone } from "@/platform/capabilities/capability-catalog"
import { appSurfacesStore, closeAppSurface } from "@/stores/desktop-store"

interface AppStoreProps {
  onLaunchApp?: (app: AppDefinition) => void
}

function uninstallAndClose(appId: string) {
  const normalizedAppId = normalizeAppId(appId)
  for (const surface of appSurfacesStore.get()) {
    if (normalizeAppId(surface.appId) === normalizedAppId) closeAppSurface(surface.id)
  }
  revokeAllLocalCapabilityGrants(normalizedAppId)
  uninstallApp(normalizedAppId)
}

function installFromStore(app: AppDefinition) {
  if (app.source === "catalog") installCatalogApp(app.appId)
  installApp(normalizeAppId(app.appId))
}

function appGroupRank(app: AppDefinition): number {
  if (app.source === "catalog") return 0
  if (app.kind === "builtin") return 1
  return 2
}

export function AppStore({ onLaunchApp }: AppStoreProps) {
  const installedIds = useStore(installedAppIdsStore)
  useStore(localCapabilityGrantsStore)
  const catalogState = useStore(appCatalogRegistry)
  const apps = useMemo(() => {
    const merged = new Map<string, AppDefinition>()
    for (const app of listCatalogApps()) merged.set(app.appId, app)
    for (const app of listBuiltinApps()) merged.set(app.appId, app)
    return Array.from(merged.values()).sort((a, b) => appGroupRank(a) - appGroupRank(b) || a.name.localeCompare(b.name))
  }, [catalogState])
  const normalizedInstalledIds = useMemo(
    () => installedIds.map(normalizeAppId),
    [installedIds],
  )

  return (
    <div className="flex h-full flex-col p-4">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h2 className="flex items-center gap-2 text-lg font-semibold text-foreground">
            <Package className="h-5 w-5" />
            App Store
          </h2>
          <p className="text-xs text-muted-foreground">
            Signed apps install into the platform runtime. Permissions are delegated per app and can be revoked.
          </p>
        </div>
        <div className="rounded-md border border-border bg-secondary/50 px-2 py-1 font-mono text-[10px] text-muted-foreground">
          {normalizedInstalledIds.length} installed
        </div>
      </div>

      <div className="grid flex-1 grid-cols-2 gap-3 overflow-auto pr-1">
        {apps.map((app) => {
          const normalizedAppId = normalizeAppId(app.appId)
          const installed = normalizedInstalledIds.includes(normalizedAppId) || isCoreApp(normalizedAppId)
          const core = isCoreApp(normalizedAppId)
          const capabilityInfos = app.requiredCapabilityIds.map(getCapabilityInfo)
          const missingCount = app.requiredCapabilityIds.filter((id) => !hasLocalCapabilityGrant(normalizedAppId, id)).length
          const signed = Boolean(app.signature)
          const verified = Boolean(app.signature?.verified)

          return (
            <div
              key={app.appId}
              className={cn(
                "group flex min-h-[190px] flex-col rounded-lg border border-border bg-secondary/40 p-3 transition-colors",
                installed && "border-primary/30 bg-primary/5",
              )}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-muted text-muted-foreground transition-colors group-hover:bg-primary group-hover:text-primary-foreground">
                  {getIconById(app.iconId)}
                </div>
                <div className="flex items-center gap-1">
                  {signed && (
                    <span
                      title={verified ? `Verified package by ${app.signature?.developerName || app.signature?.developerId}` : `Signed by ${app.signature?.developerName || app.signature?.developerId}, not verified yet`}
                      className={cn(
                        "inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] font-medium",
                        verified ? "bg-[var(--status-online)]/15 text-[var(--status-online)]" : "bg-primary/10 text-primary",
                      )}
                    >
                      <BadgeCheck className="h-2.5 w-2.5" />
                      {verified ? "Verified" : "Signed"}
                    </span>
                  )}
                  {missingCount > 0 && installed && (
                    <span className="rounded bg-[var(--status-warning)]/15 px-1.5 py-0.5 text-[10px] font-medium text-[var(--status-warning)]">
                      {missingCount} permission{missingCount === 1 ? "" : "s"}
                    </span>
                  )}
                  {installed && missingCount === 0 && (
                    <span className="rounded bg-[var(--status-online)]/15 px-1.5 py-0.5 text-[10px] font-medium text-[var(--status-online)]">
                      Ready
                    </span>
                  )}
                </div>
              </div>

              <div className="mt-2 min-h-0 flex-1">
                <div className="flex items-center gap-2">
                  <h3 className="truncate text-sm font-medium text-foreground">{app.name}</h3>
                  {app.source === "catalog" && <span className="shrink-0 rounded bg-secondary px-1.5 py-0.5 text-[9px] text-muted-foreground">catalog</span>}
                </div>
                <p className="mt-0.5 line-clamp-2 text-xs text-muted-foreground">{app.description}</p>

                {app.signature && (
                  <div className="mt-2 truncate font-mono text-[10px] text-muted-foreground">
                    dev: <span className="text-foreground">{app.signature.developerName || app.signature.developerId}</span>
                  </div>
                )}

                {capabilityInfos.length > 0 && (
                  <div className="mt-2 flex flex-wrap gap-1">
                    {capabilityInfos.map((cap) => {
                      const granted = hasLocalCapabilityGrant(normalizedAppId, cap.id)
                      return (
                        <span
                          key={cap.id}
                          title={cap.why}
                          className={cn(
                            "inline-flex items-center gap-1 rounded border px-1.5 py-0.5 text-[10px] font-medium",
                            granted
                              ? "border-[var(--status-online)]/20 bg-[var(--status-online)]/10 text-[var(--status-online)]"
                              : riskTone(cap.risk),
                          )}
                        >
                          {granted ? <CheckCircle2 className="h-2.5 w-2.5" /> : <Shield className="h-2.5 w-2.5" />}
                          {cap.label}
                        </span>
                      )
                    })}
                  </div>
                )}
              </div>

              <div className="mt-3 flex items-center gap-2">
                {installed ? (
                  <button
                    onClick={() => onLaunchApp?.(app)}
                    className="flex flex-1 items-center justify-center gap-1.5 rounded-md bg-primary px-2 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                  >
                    <Play className="h-3.5 w-3.5" />
                    {missingCount > 0 ? "Open / grant" : "Open"}
                  </button>
                ) : (
                  <button
                    onClick={() => installFromStore(app)}
                    className="flex flex-1 items-center justify-center gap-1.5 rounded-md bg-primary px-2 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                  >
                    <Download className="h-3.5 w-3.5" />
                    Install
                  </button>
                )}

                <button
                  onClick={() => uninstallAndClose(normalizedAppId)}
                  disabled={!installed || core}
                  title={core ? "Core app cannot be uninstalled" : "Uninstall and revoke app permissions"}
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
