"use client"

import { useEffect, useMemo, useState } from "react"
import { useStore } from "@nanostores/react"
import {
  ArrowLeft,
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
import { formatBytes, shortHex } from "@/lib/format"
import { getIconById, listBuiltinApps } from "@/platform/registries/builtin-app-registry"
import { catalogApps, installCatalogApp, seedBuiltinCatalogApps, uninstallCatalogApp } from "@/platform/registries/app-catalog-registry"
import type { AppDefinition } from "@/platform/types/app-definition"
import {
  installedAppIdsStore,
  installApp,
  uninstallApp,
  isCoreApp,
  PUBLISHED_BUILTIN_APP_IDS,
  normalizeAppId,
} from "@/stores/installed-apps-store"
import {
  localCapabilityGrantsStore,
  hasLocalCapabilityGrant,
  revokeAllLocalCapabilityGrants,
} from "@/stores/local-capability-grants-store"
import { getCapabilityInfo, riskTone } from "@/platform/capabilities/capability-catalog"
import { appSurfacesStore, closeAppSurface } from "@/stores/desktop-store"
import { AppEmptyState, AppHeader, AppToolbar } from "@/components/os/app-chrome"

interface AppStoreProps {
  onLaunchApp?: (app: AppDefinition) => void
}

type StoreFilter = "all" | "catalog" | "installed" | "builtin" | "publish"

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

function shortHash(value: unknown): string {
  if (typeof value !== "string" || value.length < 12) return "unknown"
  return shortHex(value, 12, 8)
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
      if ((PUBLISHED_BUILTIN_APP_IDS as readonly string[]).includes(normalizeAppId(app.appId))) merged.set(app.appId, app)
    }
    return Array.from(merged.values()).sort((a, b) => appGroupRank(a) - appGroupRank(b) || a.name.localeCompare(b.name))
  }, [catalogAppState])

  const normalizedInstalledIds = useMemo(() => installedIds.map(normalizeAppId), [installedIds])
  const installedIdSet = useMemo(() => new Set(normalizedInstalledIds), [normalizedInstalledIds])

  const filteredApps = useMemo(() => {
    if (filter === "publish") return []
    const term = query.trim().toLowerCase()
    return apps.filter((app) => {
      const normalizedAppId = normalizeAppId(app.appId)
      const installed = installedIdSet.has(normalizedAppId) || isCoreApp(normalizedAppId)
      if (filter === "catalog" && app.source !== "catalog") return false
      if (filter === "builtin" && app.source !== "builtin") return false
      if (filter === "installed" && !installed) return false
      if (!term) return true
      return [app.name, app.description, app.appId, app.signature?.developerName, app.signature?.developerId]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(term))
    })
  }, [apps, filter, installedIdSet, query])

  const selectedApp = useMemo(() => {
    if (!selectedAppId) return undefined
    return apps.find((app) => app.appId === selectedAppId)
  }, [apps, selectedAppId])

  const selectedNormalizedId = selectedApp ? normalizeAppId(selectedApp.appId) : ""
  const selectedInstalled = selectedApp ? installedIdSet.has(selectedNormalizedId) || isCoreApp(selectedNormalizedId) : false
  const selectedCore = selectedApp ? isCoreApp(selectedNormalizedId) : false
  const selectedMissingCount = selectedApp
    ? selectedApp.requiredCapabilityIds.filter((id) => !hasLocalCapabilityGrant(selectedNormalizedId, id)).length
    : 0
  const runtime = selectedApp ? metadataString(selectedApp, "runtime") : undefined
  const sourceOfTruth = selectedApp ? metadataString(selectedApp, "sourceOfTruth") : undefined
  const packageUrl = selectedApp ? metadataString(selectedApp, "packageUrl") : undefined
  const manifestUrl = selectedApp ? metadataString(selectedApp, "manifestUrl") : undefined
  const launchUrl = selectedApp ? metadataString(selectedApp, "launchUrl") : undefined
  const eappSha256 = selectedApp ? metadataString(selectedApp, "eappSha256") ?? selectedApp.signature?.packageHash : undefined
  const manifestSha256 = selectedApp ? metadataString(selectedApp, "manifestSha256") ?? selectedApp.signature?.manifestHash : undefined
  const packageBytes = selectedApp ? metadataNumber(selectedApp, "packageBytes") : undefined
  const releaseId = selectedApp ? metadataString(selectedApp, "releaseId") : undefined
  const runtimeAppId = selectedApp ? metadataString(selectedApp, "runtimeAppId") : undefined
  const developerId = selectedApp ? metadataString(selectedApp, "developerId") ?? selectedApp.signature?.developerId : undefined
  const developerName = selectedApp ? metadataString(selectedApp, "developerName") ?? selectedApp.signature?.developerName : undefined
  const authorityRef = selectedApp ? metadataString(selectedApp, "authorityRef") ?? selectedApp.signature?.authorityRef : undefined
  const proofRef = selectedApp ? metadataString(selectedApp, "proofRef") ?? selectedApp.signature?.proofRef : undefined
  const installEventKind = selectedApp ? metadataString(selectedApp, "installEventKind") : undefined
  const verifiedAssets = selectedApp ? metadataNumber(selectedApp, "verifiedAssets") : undefined
  const catalogCount = apps.filter((app) => app.source === "catalog").length
  const builtinCount = apps.filter((app) => app.source === "builtin").length
  const permissionReviewCount = apps.filter((app) => {
    const normalizedAppId = normalizeAppId(app.appId)
    const installed = installedIdSet.has(normalizedAppId) || isCoreApp(normalizedAppId)
    return installed && app.requiredCapabilityIds.some((id) => !hasLocalCapabilityGrant(normalizedAppId, id))
  }).length

  useEffect(() => {
    void seedBuiltinCatalogApps()
  }, [])

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
      <AppHeader
        title="App Store"
        icon={<Package className="h-4 w-4" />}
        aside={(
          <div className="w-fit rounded-md border border-border bg-background/45 px-3 py-1.5 text-[11px] font-medium text-muted-foreground">
            Signed SDK packages, content hashes, local install proof
          </div>
        )}
      >
        <MetricChip label="Catalog" value={catalogCount} />
        <MetricChip label="Builtin" value={builtinCount} />
        <MetricChip label="Installed" value={normalizedInstalledIds.length} />
        {permissionReviewCount > 0 ? <MetricChip label="Review" value={permissionReviewCount} tone="warning" /> : null}
      </AppHeader>

      <div className="min-h-0 flex-1 overflow-auto">
        {selectedApp ? (
          <div className="mx-auto max-w-5xl p-4 sm:p-5">
            <button
              onClick={() => setSelectedAppId(null)}
              className="mb-4 inline-flex h-8 items-center gap-2 rounded-md border border-border bg-secondary/30 px-3 text-xs font-medium text-muted-foreground hover:text-foreground"
            >
              <ArrowLeft className="h-3.5 w-3.5" />
              Apps
            </button>
            <div className="rounded-xl border border-border bg-card/55 p-4">
              <div className="flex items-start justify-between gap-3">
                <div className="flex min-w-0 items-start gap-3">
                  <div className="flex h-14 w-14 shrink-0 items-center justify-center rounded-lg border border-primary/20 bg-primary/15 text-primary">
                    {getIconById(selectedApp.iconId)}
                  </div>
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <h3 className="text-lg font-semibold leading-6 text-foreground">{selectedApp.name}</h3>
                      {selectedApp.signature?.verified ? (
                        <span className="inline-flex h-6 items-center gap-1 rounded border border-[var(--status-online)]/20 bg-[var(--status-online)]/10 px-2 text-[11px] font-medium text-[var(--status-online)]">
                          <BadgeCheck className="h-3 w-3" />
                          Verified package
                        </span>
                      ) : selectedApp.signature ? (
                        <span className="inline-flex h-6 items-center gap-1 rounded border border-primary/20 bg-primary/10 px-2 text-[11px] font-medium text-primary">
                          <BadgeCheck className="h-3 w-3" />
                          Signed package
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
                    {busyAppId === selectedNormalizedId ? "Verifying" : selectedApp.source === "catalog" ? "Verify & install" : "Install"}
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
            </div>

            {installError && (
              <div className="mt-3 rounded-md border border-[var(--status-error)]/25 bg-[var(--status-error)]/10 px-3 py-2 text-xs text-[var(--status-error)]">
                {installError}
              </div>
            )}

            <div className="mt-4 grid gap-2 sm:grid-cols-3">
              <InfoCell label="Runtime" value={runtime ?? selectedApp.kind} />
              <InfoCell label="Package" value={typeof packageBytes === "number" ? formatBytes(packageBytes) : "unknown"} />
              <InfoCell label="Assets" value={typeof verifiedAssets === "number" ? `${verifiedAssets} verified` : "on install"} />
            </div>

            <SectionTitle>SDK Package</SectionTitle>
            <div className="rounded-md border border-primary/20 bg-primary/5 px-3 py-2 text-xs leading-5 text-muted-foreground">
              {sourceOfTruth === "sdk-signed-content-addressed-package"
                ? "This app is projected from SDK package artifacts. The catalog is only discovery; install verifies package bytes, manifest bytes, developer signature, and asset hashes through the browser node."
                : "Builtin platform app. Package proof applies to catalog apps built with the Edgerun SDK."}
            </div>
            <div className="mt-2 grid gap-2 sm:grid-cols-2">
              <ProofRow label="developer" value={developerName || shortHash(developerId)} raw={developerId} />
              <ProofRow label="authority" value={authorityRef || "unknown"} raw={authorityRef} />
              <ProofRow label="proof" value={proofRef || "unknown"} raw={proofRef} />
              <ProofRow label="install event" value={installEventKind || "pending"} raw={installEventKind} />
            </div>

            <SectionTitle>Permissions</SectionTitle>
            <div className="mb-2 rounded-md border border-border bg-secondary/25 px-3 py-2 text-xs leading-5 text-muted-foreground">
              Install records stay in this browser. Packaged apps are hash-checked through edgerun-node before launch; capabilities are granted separately through Trust Manager.
            </div>
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
          <div className="p-4 sm:p-5">
            <AppToolbar className="mb-4">
              <div className="flex h-9 min-w-0 flex-1 items-center gap-2 rounded-md border border-border bg-card/70 px-3 sm:max-w-sm">
                <Search className="h-3.5 w-3.5 text-muted-foreground" />
                <input
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder="Search apps"
                  aria-label="Search apps"
                  className="min-w-0 flex-1 bg-transparent text-xs text-foreground outline-none placeholder:text-muted-foreground"
                />
              </div>
              <div className="flex items-center gap-1 overflow-x-auto pb-0.5">
                <Filter className="mr-1 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                {(["all", "catalog", "installed", "builtin", "publish"] as StoreFilter[]).map((item) => (
                  <button
                    key={item}
                    onClick={() => setFilter(item)}
                    aria-pressed={filter === item}
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
            </AppToolbar>

            {filter === "publish" ? (
              <PublishPanel />
            ) : filteredApps.length > 0 ? (
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
                {filteredApps.map((app) => {
                  const normalizedAppId = normalizeAppId(app.appId)
                  const installed = installedIdSet.has(normalizedAppId) || isCoreApp(normalizedAppId)
                  const missingCount = app.requiredCapabilityIds.filter((id) => !hasLocalCapabilityGrant(normalizedAppId, id)).length
                  return (
                    <button
                      key={app.appId}
                      onClick={() => setSelectedAppId(app.appId)}
                      className="flex min-h-40 flex-col rounded-lg border border-border bg-card/50 p-4 text-left transition-colors hover:border-primary/35 hover:bg-primary/5"
                    >
                      <div className="flex items-start justify-between gap-3">
                        <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg border border-border bg-secondary/45 text-muted-foreground">
                          {getIconById(app.iconId)}
                        </div>
                        <span
                          className={cn(
                            "rounded px-1.5 py-0.5 text-[10px] font-medium",
                            installed ? "bg-[var(--status-online)]/15 text-[var(--status-online)]" : "bg-secondary text-muted-foreground",
                          )}
                        >
                          {installed ? "Installed" : app.source === "catalog" ? "Signed" : app.source}
                        </span>
                      </div>
                      <div className="mt-3 flex min-w-0 items-center gap-2">
                        <span className="truncate text-sm font-semibold text-foreground">{app.name}</span>
                        {app.source === "catalog" && <BadgeCheck className="h-3.5 w-3.5 shrink-0 text-primary" />}
                      </div>
                      <p className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">{app.description}</p>
                      <div className="mt-auto flex items-center gap-1.5 pt-4">
                        {installed && missingCount > 0 ? (
                          <span className="rounded bg-[var(--status-warning)]/15 px-1.5 py-0.5 text-[10px] font-medium text-[var(--status-warning)]">
                            {missingCount} permission{missingCount === 1 ? "" : "s"}
                          </span>
                        ) : null}
                        <span className="truncate text-[10px] text-muted-foreground">
                          {app.source === "catalog" ? app.signature?.developerName ?? "catalog" : app.kind}
                        </span>
                      </div>
                    </button>
                  )
                })}
              </div>
            ) : (
              <AppEmptyState>No apps match this view</AppEmptyState>
            )}
          </div>
        )}
      </div>
    </div>
  )
}

function PublishPanel() {
  return (
    <div className="grid gap-4 lg:grid-cols-[1.1fr_0.9fr]">
      <div className="rounded-xl border border-border bg-card/55 p-4">
        <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
          <Package className="h-4 w-4 text-primary" />
          Publish an SDK package
        </div>
        <p className="mt-2 text-xs leading-5 text-muted-foreground">
          Build with edgerun-sdk, then publish the signed package artifacts. The app store indexes the release; users install by verifying hashes and developer signatures through their browser node.
        </p>
        <div className="mt-4 grid gap-2">
          <PublishStep index={1} title="Package" body="Run the SDK packager and produce app.edapp, app.eapp, and developer.esig." />
          <PublishStep index={2} title="Store" body="Upload the package objects to content-addressed storage/CDN so retrieval can be verified by hash." />
          <PublishStep index={3} title="Sell" body="Attach a content-addressed policy for price, capabilities, and distribution terms. Payments settle from usage and installs." />
        </div>
      </div>
      <div className="rounded-xl border border-border bg-secondary/20 p-4">
        <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">SDK artifacts</div>
        <div className="mt-3 grid gap-2">
          <InfoCell label="Manifest" value="app.edapp" />
          <InfoCell label="Package graph" value="app.eapp" />
          <InfoCell label="Developer signature" value="developer.esig" />
        </div>
        <div className="mt-4 rounded-md border border-primary/20 bg-primary/5 px-3 py-2 text-xs leading-5 text-muted-foreground">
          Next wiring target: drag these files here, verify them in-browser, then create a catalog submission backed by Edgerun storage and payment policy.
        </div>
      </div>
    </div>
  )
}

function PublishStep({ index, title, body }: { index: number; title: string; body: string }) {
  return (
    <div className="rounded-md border border-border bg-secondary/25 px-3 py-2">
      <div className="text-xs font-semibold text-foreground">{index}. {title}</div>
      <div className="mt-1 text-xs leading-5 text-muted-foreground">{body}</div>
    </div>
  )
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return <h4 className="mb-2 mt-5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{children}</h4>
}

function MetricChip({ label, value, tone = "default" }: { label: string; value: number; tone?: "default" | "warning" }) {
  return (
    <span className={cn(
      "inline-flex h-5 items-center gap-1 rounded border px-1.5 text-[10px] font-medium",
      tone === "warning"
        ? "border-[var(--status-warning)]/25 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
        : "border-border bg-background/45 text-muted-foreground",
    )}>
      <span className="font-mono text-foreground">{value}</span>
      {label}
    </span>
  )
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
            aria-label={`Copy ${label} proof`}
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
