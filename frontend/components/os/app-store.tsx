"use client"

import { useMemo, useState } from "react"
import { useStore } from "@nanostores/react"
import {
  BadgeCheck,
  CheckCircle2,
  Copy,
  Download,
  ExternalLink,
  Filter,
  Loader2,
  Package,
  Play,
  Search,
  Shield,
  Trash2,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { getIconById, listBuiltinApps } from "@/platform/registries/builtin-app-registry"
import { catalogApps, installCatalogApp, uninstallCatalogApp } from "@/platform/registries/app-catalog-registry"
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

type StoreFilter = "all" | "catalog" | "installed" | "builtin"

async function uninstallAndClose(appId: string) {
  const normalizedAppId = normalizeAppId(appId)
  for (const surface of appSurfacesStore.get()) {
    if (normalizeAppId(surface.appId) === normalizedAppId) closeAppSurface(surface.id)
  }
  revokeAllLocalCapabilityGrants(normalizedAppId)
  await uninstallCatalogApp(normalizedAppId).catch(() => undefined)
  uninstallApp(normalizedAppId)
}

async function installFromStore(app: AppDefinition) {
  if (app.source === "catalog") await installCatalogApp(app.appId)
  installApp(normalizeAppId(app.appId))
}

function appGroupRank(app: AppDefinition): number {
  if (app.source === "catalog") return 0
  if (app.kind === "builtin") return 1
  return 2
}

function formatBytes(value: unknown): string {
  if (typeof value !== "number" || !Number.isFinite(value)) return "unknown"
  if (value < 1024) return `${value} B`
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`
  return `${(value / (1024 * 1024)).toFixed(1)} MB`
}

function shortHash(value: unknown): string {
  if (typeof value !== "string" || value.length < 12) return "unknown"
  return `${value.slice(0, 12)}...${value.slice(-8)}`
}

function metadataString(app: AppDefinition, key: string): string | undefined {
  const value = app.displayMetadata?.[key]
  return typeof value === "string" && value.length > 0 ? value : undefined
}

function metadataNumber(app: AppDefinition, key: string): number | undefined {
  const value = app.displayMetadata?.[key]
  return typeof value === "number" && Number.isFinite(value) ? value : undefined
}

function capabilityChips(app: AppDefinition, installed: boolean) {
  return app.requiredCapabilityIds.map(getCapabilityInfo).map((cap) => {
    const normalizedAppId = normalizeAppId(app.appId)
    const granted = hasLocalCapabilityGrant(normalizedAppId, cap.id)
    return (
      <span
        key={cap.id}
        title={cap.why}
        className={cn(
          "inline-flex h-6 items-center gap-1 rounded border px-2 text-[11px] font-medium",
          installed && granted
            ? "border-[var(--status-online)]/20 bg-[var(--status-online)]/10 text-[var(--status-online)]"
            : riskTone(cap.risk),
        )}
      >
        {installed && granted ? <CheckCircle2 className="h-3 w-3" /> : <Shield className="h-3 w-3" />}
        {cap.label}
      </span>
    )
  })
}

export function AppStore({ onLaunchApp }: AppStoreProps) {
  const installedIds = useStore(installedAppIdsStore)
  const [busyAppId, setBusyAppId] = useState<string | null>(null)
  const [installError, setInstallError] = useState<string | null>(null)
  const [selectedAppId, setSelectedAppId] = useState<string | null>(null)
  const [query, setQuery] = useState("")
  const [filter, setFilter] = useState<StoreFilter>("all")
  useStore(localCapabilityGrantsStore)
  const catalogAppState = useStore(catalogApps)

  const apps = useMemo(() => {
    const merged = new Map<string, AppDefinition>()
    for (const app of catalogAppState) merged.set(app.appId, app)
    for (const app of listBuiltinApps()) {
      if (isCoreApp(app.appId)) merged.set(app.appId, app)
    }
    return Array.from(merged.values()).sort((a, b) => appGroupRank(a) - appGroupRank(b) || a.name.localeCompare(b.name))
  }, [catalogAppState])

  const normalizedInstalledIds = useMemo(
    () => installedIds.map(normalizeAppId),
    [installedIds],
  )

  const filteredApps = useMemo(() => {
    const term = query.trim().toLowerCase()
    return apps.filter((app) => {
      const normalizedAppId = normalizeAppId(app.appId)
      const installed = normalizedInstalledIds.includes(normalizedAppId) || isCoreApp(normalizedAppId)
      if (filter === "catalog" && app.source !== "catalog") return false
      if (filter === "builtin" && app.source !== "builtin") return false
      if (filter === "installed" && !installed) return false
      if (!term) return true
      return [app.name, app.description, app.appId, app.signature?.developerName, app.signature?.developerId]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(term))
    })
  }, [apps, filter, normalizedInstalledIds, query])

  const selectedApp = useMemo(() => {
    if (!filteredApps.length) return undefined
    return filteredApps.find((app) => app.appId === selectedAppId) ?? filteredApps[0]
  }, [filteredApps, selectedAppId])

  const selectedNormalizedId = selectedApp ? normalizeAppId(selectedApp.appId) : ""
  const selectedInstalled = selectedApp ? normalizedInstalledIds.includes(selectedNormalizedId) || isCoreApp(selectedNormalizedId) : false
  const selectedCore = selectedApp ? isCoreApp(selectedNormalizedId) : false
  const selectedMissingCount = selectedApp
    ? selectedApp.requiredCapabilityIds.filter((id) => !hasLocalCapabilityGrant(selectedNormalizedId, id)).length
    : 0
  const runtime = selectedApp ? metadataString(selectedApp, "runtime") : undefined
  const packageUrl = selectedApp ? metadataString(selectedApp, "packageUrl") : undefined
  const manifestUrl = selectedApp ? metadataString(selectedApp, "manifestUrl") : undefined
  const launchUrl = selectedApp ? metadataString(selectedApp, "launchUrl") : undefined
  const eappSha256 = selectedApp ? metadataString(selectedApp, "eappSha256") ?? selectedApp.signature?.packageHash : undefined
  const manifestSha256 = selectedApp ? metadataString(selectedApp, "manifestSha256") ?? selectedApp.signature?.manifestHash : undefined
  const packageBytes = selectedApp ? metadataNumber(selectedApp, "packageBytes") : undefined
  const releaseId = selectedApp ? metadataString(selectedApp, "releaseId") : undefined
  const runtimeAppId = selectedApp ? metadataString(selectedApp, "runtimeAppId") : undefined
  const verifiedAssets = selectedApp ? metadataNumber(selectedApp, "verifiedAssets") : undefined

  async function installSelected(app: AppDefinition) {
    const normalizedAppId = normalizeAppId(app.appId)
    setBusyAppId(normalizedAppId)
    setInstallError(null)
    try {
      await installFromStore(app)
    } catch (error) {
      setInstallError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusyAppId(null)
    }
  }

  async function uninstallSelected(app: AppDefinition) {
    const normalizedAppId = normalizeAppId(app.appId)
    setBusyAppId(normalizedAppId)
    setInstallError(null)
    try {
      await uninstallAndClose(normalizedAppId)
    } catch (error) {
      setInstallError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusyAppId(null)
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      <div className="flex shrink-0 items-center justify-between border-b border-border px-4 py-3">
        <div className="flex min-w-0 items-center gap-3">
          <div className="flex h-9 w-9 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <Package className="h-4 w-4" />
          </div>
          <div className="min-w-0">
            <h2 className="truncate text-sm font-semibold text-foreground">App Store</h2>
            <div className="mt-0.5 flex items-center gap-2 text-[11px] text-muted-foreground">
              <span>{apps.filter((app) => app.source === "catalog").length} catalog</span>
              <span className="h-1 w-1 rounded-full bg-border" />
              <span>{normalizedInstalledIds.length} installed</span>
              <span className="h-1 w-1 rounded-full bg-border" />
              <span>edgerun-node verified</span>
            </div>
          </div>
        </div>
        <div className="rounded-md border border-border bg-secondary/35 px-3 py-1.5 text-[11px] font-medium text-muted-foreground">
          CLI submissions only
        </div>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,4fr)_minmax(260px,1fr)]">
          <div className="flex min-h-0 flex-col border-r border-border">
            <div className="shrink-0 border-b border-border p-3">
              <div className="flex h-9 items-center gap-2 rounded-md border border-border bg-secondary/35 px-3">
                <Search className="h-3.5 w-3.5 text-muted-foreground" />
                <input
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder="Search apps"
                  className="min-w-0 flex-1 bg-transparent text-xs text-foreground outline-none placeholder:text-muted-foreground"
                />
              </div>
              <div className="mt-2 flex items-center gap-1 overflow-x-auto">
                <Filter className="mr-1 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                {(["all", "catalog", "installed", "builtin"] as StoreFilter[]).map((item) => (
                  <button
                    key={item}
                    onClick={() => setFilter(item)}
                    className={cn(
                      "h-7 shrink-0 rounded border px-2 text-[11px] font-medium capitalize",
                      filter === item
                        ? "border-primary/40 bg-primary/15 text-primary"
                        : "border-border bg-secondary/25 text-muted-foreground hover:text-foreground",
                    )}
                  >
                    {item}
                  </button>
                ))}
              </div>
            </div>

            <div className="min-h-0 flex-1 overflow-auto p-2">
              {filteredApps.map((app) => {
                const normalizedAppId = normalizeAppId(app.appId)
                const installed = normalizedInstalledIds.includes(normalizedAppId) || isCoreApp(normalizedAppId)
                const missingCount = app.requiredCapabilityIds.filter((id) => !hasLocalCapabilityGrant(normalizedAppId, id)).length
                const selected = selectedApp?.appId === app.appId
                return (
                  <button
                    key={app.appId}
                    onClick={() => setSelectedAppId(app.appId)}
                    className={cn(
                      "mb-1 flex w-full items-start gap-3 rounded-md border p-2.5 text-left transition-colors",
                      selected
                        ? "border-primary/35 bg-primary/10"
                        : "border-transparent bg-transparent hover:border-border hover:bg-secondary/25",
                    )}
                  >
                    <div className={cn("flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground", selected && "bg-primary text-primary-foreground")}>
                      {getIconById(app.iconId)}
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex min-w-0 items-center gap-2">
                        <span className="truncate text-xs font-semibold text-foreground">{app.name}</span>
                        {app.source === "catalog" && <BadgeCheck className="h-3.5 w-3.5 shrink-0 text-primary" />}
                      </div>
                      <p className="mt-0.5 line-clamp-2 text-[11px] leading-4 text-muted-foreground">{app.description}</p>
                      <div className="mt-2 flex items-center gap-1.5">
                        <span className={cn(
                          "rounded px-1.5 py-0.5 text-[10px] font-medium",
                          installed ? "bg-[var(--status-online)]/15 text-[var(--status-online)]" : "bg-secondary text-muted-foreground",
                        )}>
                          {installed ? "Installed" : app.source}
                        </span>
                        {installed && missingCount > 0 && (
                          <span className="rounded bg-[var(--status-warning)]/15 px-1.5 py-0.5 text-[10px] font-medium text-[var(--status-warning)]">
                            {missingCount} permission{missingCount === 1 ? "" : "s"}
                          </span>
                        )}
                      </div>
                    </div>
                  </button>
                )
              })}
              {filteredApps.length === 0 && (
                <div className="flex h-40 items-center justify-center rounded-md border border-dashed border-border text-xs text-muted-foreground">
                  No apps match this view
                </div>
              )}
            </div>
          </div>

          <div className="min-h-0 overflow-auto">
            {selectedApp ? (
              <div className="p-4">
                <div className="flex items-start justify-between gap-3">
                  <div className="flex min-w-0 items-start gap-3">
                    <div className="flex h-14 w-14 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground">
                      {getIconById(selectedApp.iconId)}
                    </div>
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <h3 className="text-lg font-semibold leading-6 text-foreground">{selectedApp.name}</h3>
                        {selectedApp.signature?.verified ? (
                          <span className="inline-flex h-6 items-center gap-1 rounded border border-[var(--status-online)]/20 bg-[var(--status-online)]/10 px-2 text-[11px] font-medium text-[var(--status-online)]">
                            <BadgeCheck className="h-3 w-3" />
                            Verified
                          </span>
                        ) : selectedApp.signature ? (
                          <span className="inline-flex h-6 items-center gap-1 rounded border border-primary/20 bg-primary/10 px-2 text-[11px] font-medium text-primary">
                            <BadgeCheck className="h-3 w-3" />
                            Signed
                          </span>
                        ) : null}
                      </div>
                      <p className="mt-1 max-w-xl text-xs leading-5 text-muted-foreground">{selectedApp.description}</p>
                    </div>
                  </div>
                  <span className="rounded border border-border bg-secondary/35 px-2 py-1 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                    {selectedApp.source}
                  </span>
                </div>

                <div className="mt-4 grid grid-cols-3 gap-2">
                  <InfoCell label="Runtime" value={runtime ?? selectedApp.kind} />
                  <InfoCell label="Package" value={formatBytes(packageBytes)} />
                  <InfoCell label="Assets" value={typeof verifiedAssets === "number" ? `${verifiedAssets} verified` : "on install"} />
                </div>

                <div className="mt-4 flex flex-wrap gap-2">
                  {selectedInstalled ? (
                    <button
                      onClick={() => onLaunchApp?.(selectedApp)}
                      className="inline-flex h-9 items-center justify-center gap-2 rounded-md bg-primary px-3 text-xs font-semibold text-primary-foreground hover:bg-primary/90"
                    >
                      <Play className="h-3.5 w-3.5" />
                      {selectedMissingCount > 0 ? "Open / grant" : "Open"}
                    </button>
                  ) : (
                    <button
                      onClick={() => void installSelected(selectedApp)}
                      disabled={busyAppId === selectedNormalizedId}
                      className="inline-flex h-9 items-center justify-center gap-2 rounded-md bg-primary px-3 text-xs font-semibold text-primary-foreground hover:bg-primary/90 disabled:cursor-wait disabled:opacity-70"
                    >
                      {busyAppId === selectedNormalizedId ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Download className="h-3.5 w-3.5" />}
                      {busyAppId === selectedNormalizedId ? "Installing" : "Install"}
                    </button>
                  )}
                  <button
                    onClick={() => void uninstallSelected(selectedApp)}
                    disabled={!selectedInstalled || selectedCore || busyAppId === selectedNormalizedId}
                    title={selectedCore ? "Core app cannot be uninstalled" : "Uninstall and revoke app permissions"}
                    className="inline-flex h-9 items-center justify-center gap-2 rounded-md border border-border bg-secondary/35 px-3 text-xs font-semibold text-muted-foreground hover:border-[var(--status-error)]/30 hover:bg-[var(--status-error)]/10 hover:text-[var(--status-error)] disabled:cursor-not-allowed disabled:opacity-40"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                    Uninstall
                  </button>
                  {launchUrl && (
                    <a
                      href={launchUrl}
                      target="_blank"
                      rel="noreferrer"
                      className="inline-flex h-9 items-center justify-center gap-2 rounded-md border border-border bg-secondary/35 px-3 text-xs font-semibold text-muted-foreground hover:text-foreground"
                    >
                      <ExternalLink className="h-3.5 w-3.5" />
                      Launch URL
                    </a>
                  )}
                </div>

                {installError && (
                  <div className="mt-3 rounded-md border border-[var(--status-error)]/25 bg-[var(--status-error)]/10 px-3 py-2 text-xs text-[var(--status-error)]">
                    {installError}
                  </div>
                )}

                <SectionTitle>Permissions</SectionTitle>
                <div className="flex flex-wrap gap-2">
                  {selectedApp.requiredCapabilityIds.length > 0 ? capabilityChips(selectedApp, selectedInstalled) : (
                    <span className="text-xs text-muted-foreground">No required capabilities</span>
                  )}
                  {selectedApp.optionalCapabilityIds.map((capabilityId) => {
                    const cap = getCapabilityInfo(capabilityId)
                    return (
                      <span key={capabilityId} title={cap.why} className="inline-flex h-6 items-center gap-1 rounded border border-border bg-secondary/25 px-2 text-[11px] font-medium text-muted-foreground">
                        <Shield className="h-3 w-3" />
                        {cap.label}
                      </span>
                    )
                  })}
                </div>

                <SectionTitle>Package Proof</SectionTitle>
                <div className="grid gap-2">
                  <ProofRow label="eapp" value={shortHash(eappSha256)} raw={eappSha256} />
                  <ProofRow label="manifest" value={shortHash(manifestSha256)} raw={manifestSha256} />
                  <ProofRow label="runtime app" value={shortHash(runtimeAppId)} raw={runtimeAppId} />
                  <ProofRow label="release" value={shortHash(releaseId)} raw={releaseId} />
                </div>

                {(packageUrl || manifestUrl) && (
                  <>
                    <SectionTitle>Files</SectionTitle>
                    <div className="grid gap-2">
                      {packageUrl && <FileRow label="Package" value={packageUrl} />}
                      {manifestUrl && <FileRow label="Manifest" value={manifestUrl} />}
                    </div>
                  </>
                )}
              </div>
            ) : (
              <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
                Select an app
              </div>
            )}
          </div>
      </div>
    </div>
  )
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return <h4 className="mb-2 mt-5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{children}</h4>
}

function InfoCell({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-secondary/25 px-3 py-2">
      <div className="text-[10px] uppercase tracking-wide text-muted-foreground">{label}</div>
      <div className="mt-1 truncate text-xs font-semibold text-foreground">{value}</div>
    </div>
  )
}

function ProofRow({ label, value, raw }: { label: string; value: string; raw?: string }) {
  return (
    <div className="flex h-9 items-center justify-between gap-3 rounded-md border border-border bg-secondary/25 px-3">
      <span className="shrink-0 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">{label}</span>
      <div className="flex min-w-0 items-center gap-2">
        <span className="truncate font-mono text-[11px] text-foreground">{value}</span>
        {raw && raw !== "unknown" && (
          <button
            onClick={() => void navigator.clipboard?.writeText(raw)}
            className="flex h-6 w-6 shrink-0 items-center justify-center rounded text-muted-foreground hover:bg-secondary hover:text-foreground"
            title="Copy"
          >
            <Copy className="h-3 w-3" />
          </button>
        )}
      </div>
    </div>
  )
}

function FileRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex h-9 items-center justify-between gap-3 rounded-md border border-border bg-secondary/25 px-3">
      <span className="shrink-0 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">{label}</span>
      <span className="min-w-0 truncate font-mono text-[11px] text-foreground">{value}</span>
    </div>
  )
}
