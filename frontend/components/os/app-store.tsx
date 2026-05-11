"use client"

import { useEffect, useMemo, useState } from "react"
import { useStore } from "@nanostores/react"
import {
  ArrowLeft,
  BadgeCheck,
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
import { formatMicroEdge } from "@/platform/runtime/verified-app-package"
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

type StoreFilter = "all" | "catalog" | "cached" | "builtin" | "publish"

async function uninstallAndClose(appId: string) {
  const normalizedAppId = normalizeAppId(appId)
  for (const surface of appSurfacesStore.get()) {
    if (normalizeAppId(surface.appId) === normalizedAppId) closeAppSurface(surface.id)
  }
  revokeAllLocalCapabilityGrants(normalizedAppId)
  await uninstallCatalogApp(normalizedAppId).catch(() => undefined)
  uninstallApp(normalizedAppId)
}

async function cacheFromStore(app: AppDefinition) {
  if (app.source === "catalog") await installCatalogApp(app.appId)
  installApp(normalizeAppId(app.appId))
}

function appGroupRank(app: AppDefinition): number {
  if (app.source === "catalog") return 0
  if (app.kind === "builtin") return 1
  return 2
}

function metadataString(app: AppDefinition | undefined, key: string): string | undefined {
  const value = app?.displayMetadata?.[key]
  return typeof value === "string" && value.length > 0 ? value : undefined
}

function metadataNumber(app: AppDefinition | undefined, key: string): number | undefined {
  const value = app?.displayMetadata?.[key]
  return typeof value === "number" && Number.isFinite(value) ? value : undefined
}

function shortHash(value: unknown): string {
  if (typeof value !== "string" || value.length < 12) return "unknown"
  return shortHex(value, 12, 8)
}

function filterLabel(filter: StoreFilter): string {
  return filter === "cached" ? "cached" : filter
}

function capabilityChips(app: AppDefinition, cached: boolean) {
  return app.requiredCapabilityIds.map(getCapabilityInfo).map((cap) => {
    const normalizedAppId = normalizeAppId(app.appId)
    const granted = hasLocalCapabilityGrant(normalizedAppId, cap.id)
    return (
      <span
        key={cap.id}
        title={cap.why}
        className={cn(
          "inline-flex h-6 items-center gap-1 rounded border px-2 text-[11px] font-medium",
          cached && granted
            ? "border-[var(--status-online)]/20 bg-[var(--status-online)]/10 text-[var(--status-online)]"
            : riskTone(cap.risk),
        )}
      >
        <Shield className="h-3 w-3" />
        {cap.label}
      </span>
    )
  })
}

function runCost(app: AppDefinition | undefined) {
  return {
    source: metadataString(app, "runCostSource") ?? "catalog-default",
    runOnce: metadataNumber(app, "runOnceMicroEdge") ?? 0,
    cache: metadataNumber(app, "verifyAndCacheMicroEdge") ?? 0,
    retrieval: metadataNumber(app, "retrievalMicroEdge") ?? 0,
    cacheWrite: metadataNumber(app, "cacheWriteMicroEdge") ?? 0,
  }
}

export function AppStore({ onLaunchApp }: AppStoreProps) {
  const installedIds = useStore(installedAppIdsStore)
  const catalogAppState = useStore(catalogApps)
  useStore(localCapabilityGrantsStore)
  const [busyAppId, setBusyAppId] = useState<string | null>(null)
  const [installError, setInstallError] = useState<string | null>(null)
  const [selectedAppId, setSelectedAppId] = useState<string | null>(null)
  const [pendingRunAppId, setPendingRunAppId] = useState<string | null>(null)
  const [query, setQuery] = useState("")
  const [filter, setFilter] = useState<StoreFilter>("all")

  useEffect(() => {
    void seedBuiltinCatalogApps()
  }, [])

  const apps = useMemo(() => {
    const merged = new Map<string, AppDefinition>()
    for (const app of catalogAppState) merged.set(app.appId, app)
    for (const app of listBuiltinApps()) {
      if ((PUBLISHED_BUILTIN_APP_IDS as readonly string[]).includes(normalizeAppId(app.appId))) merged.set(app.appId, app)
    }
    return Array.from(merged.values()).sort((a, b) => appGroupRank(a) - appGroupRank(b) || a.name.localeCompare(b.name))
  }, [catalogAppState])

  const cachedIds = useMemo(() => installedIds.map(normalizeAppId), [installedIds])
  const cachedIdSet = useMemo(() => new Set(cachedIds), [cachedIds])
  const selectedApp = selectedAppId ? apps.find((app) => app.appId === selectedAppId) : undefined
  const pendingRunApp = pendingRunAppId ? apps.find((app) => app.appId === pendingRunAppId) : undefined

  const filteredApps = useMemo(() => {
    if (filter === "publish") return []
    const term = query.trim().toLowerCase()
    return apps.filter((app) => {
      const normalized = normalizeAppId(app.appId)
      const cached = cachedIdSet.has(normalized) || isCoreApp(normalized)
      if (filter === "catalog" && app.source !== "catalog") return false
      if (filter === "builtin" && app.source !== "builtin") return false
      if (filter === "cached" && !cached) return false
      if (!term) return true
      return [app.name, app.description, app.appId, app.signature?.developerName, app.signature?.developerId]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(term))
    })
  }, [apps, cachedIdSet, filter, query])

  const catalogCount = apps.filter((app) => app.source === "catalog").length
  const builtinCount = apps.filter((app) => app.source === "builtin").length
  const reviewCount = apps.filter((app) => {
    const normalized = normalizeAppId(app.appId)
    const cached = cachedIdSet.has(normalized) || isCoreApp(normalized)
    return cached && app.requiredCapabilityIds.some((id) => !hasLocalCapabilityGrant(normalized, id))
  }).length

  async function cacheSelected(app: AppDefinition) {
    const normalized = normalizeAppId(app.appId)
    setBusyAppId(normalized)
    setInstallError(null)
    try {
      await cacheFromStore(app)
    } catch (error) {
      setInstallError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusyAppId(null)
    }
  }

  async function removeCache(app: AppDefinition) {
    const normalized = normalizeAppId(app.appId)
    setBusyAppId(normalized)
    setInstallError(null)
    try {
      await uninstallAndClose(normalized)
    } catch (error) {
      setInstallError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusyAppId(null)
    }
  }

  function runApp(app: AppDefinition) {
    const normalized = normalizeAppId(app.appId)
    const cached = cachedIdSet.has(normalized) || isCoreApp(normalized)
    if (cached) {
      onLaunchApp?.(app)
      return
    }
    if (app.source === "catalog") {
      setPendingRunAppId(app.appId)
      return
    }
    onLaunchApp?.(app)
  }

  function runFromNetwork(app: AppDefinition) {
    const launchUrl = metadataString(app, "launchUrl")
    setPendingRunAppId(null)
    if (launchUrl) window.open(launchUrl, "_blank", "noopener,noreferrer")
    else onLaunchApp?.(app)
  }

  async function verifyCacheAndRun(app: AppDefinition) {
    setPendingRunAppId(null)
    await cacheSelected(app)
    onLaunchApp?.(app)
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      <AppHeader
        title="App Store"
        icon={<Package className="h-4 w-4" />}
        aside={<div className="w-fit rounded-md border border-border bg-background/45 px-3 py-1.5 text-[11px] font-medium text-muted-foreground">Run from network storage, cache locally, verify every hash</div>}
      >
        <MetricChip label="Catalog" value={catalogCount} />
        <MetricChip label="Builtin" value={builtinCount} />
        <MetricChip label="Cached" value={cachedIds.length} />
        {reviewCount > 0 ? <MetricChip label="Review" value={reviewCount} tone="warning" /> : null}
      </AppHeader>

      <div className="min-h-0 flex-1 overflow-auto">
        {selectedApp ? (
          <AppDetails
            app={selectedApp}
            cached={cachedIdSet.has(normalizeAppId(selectedApp.appId)) || isCoreApp(normalizeAppId(selectedApp.appId))}
            busy={busyAppId === normalizeAppId(selectedApp.appId)}
            error={installError}
            onBack={() => setSelectedAppId(null)}
            onRun={() => runApp(selectedApp)}
            onCache={() => void cacheSelected(selectedApp)}
            onRemoveCache={() => void removeCache(selectedApp)}
          />
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
                {(["all", "catalog", "cached", "builtin", "publish"] as StoreFilter[]).map((item) => (
                  <button
                    key={item}
                    onClick={() => setFilter(item)}
                    aria-pressed={filter === item}
                    className={cn(
                      "h-7 shrink-0 rounded border px-2 text-[11px] font-medium capitalize",
                      filter === item ? "border-primary/40 bg-primary/15 text-primary" : "border-border bg-secondary/25 text-muted-foreground hover:text-foreground",
                    )}
                  >
                    {filterLabel(item)}
                  </button>
                ))}
              </div>
            </AppToolbar>

            {filter === "publish" ? (
              <PublishPanel />
            ) : filteredApps.length > 0 ? (
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
                {filteredApps.map((app) => {
                  const normalized = normalizeAppId(app.appId)
                  const cached = cachedIdSet.has(normalized) || isCoreApp(normalized)
                  const cost = runCost(app)
                  return (
                    <button
                      key={app.appId}
                      onClick={() => setSelectedAppId(app.appId)}
                      className="flex min-h-44 flex-col rounded-lg border border-border bg-card/50 p-4 text-left transition-colors hover:border-primary/35 hover:bg-primary/5"
                    >
                      <div className="flex items-start justify-between gap-3">
                        <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg border border-border bg-secondary/45 text-muted-foreground">
                          {getIconById(app.iconId)}
                        </div>
                        <span className={cn("rounded px-1.5 py-0.5 text-[10px] font-medium", cached ? "bg-[var(--status-online)]/15 text-[var(--status-online)]" : "bg-secondary text-muted-foreground")}>{cached ? "Cached" : app.source === "catalog" ? "Network" : app.source}</span>
                      </div>
                      <div className="mt-3 flex min-w-0 items-center gap-2">
                        <span className="truncate text-sm font-semibold text-foreground">{app.name}</span>
                        {app.source === "catalog" && <BadgeCheck className="h-3.5 w-3.5 shrink-0 text-primary" />}
                      </div>
                      <p className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">{app.description}</p>
                      <div className="mt-auto flex items-center justify-between gap-2 pt-4 text-[10px] text-muted-foreground">
                        <span className="truncate">{app.signature?.developerName ?? app.kind}</span>
                        {!cached && app.source === "catalog" ? <span>{formatMicroEdge(cost.runOnce)} run</span> : null}
                      </div>
                    </button>
                  )
                })}
              </div>
            ) : <AppEmptyState>No apps match this view</AppEmptyState>}
          </div>
        )}
      </div>

      {pendingRunApp ? (
        <FirstRunDialog
          app={pendingRunApp}
          onRunOnce={() => runFromNetwork(pendingRunApp)}
          onCache={() => void verifyCacheAndRun(pendingRunApp)}
          onCancel={() => setPendingRunAppId(null)}
        />
      ) : null}
    </div>
  )
}

function AppDetails({ app, cached, busy, error, onBack, onRun, onCache, onRemoveCache }: {
  app: AppDefinition
  cached: boolean
  busy: boolean
  error: string | null
  onBack: () => void
  onRun: () => void
  onCache: () => void
  onRemoveCache: () => void
}) {
  const runtime = metadataString(app, "runtime")
  const distributionModel = metadataString(app, "distributionModel")
  const sourceOfTruth = metadataString(app, "sourceOfTruth")
  const packageBytes = metadataNumber(app, "packageBytes")
  const verifiedAssets = metadataNumber(app, "verifiedAssets")
  const eappSha256 = metadataString(app, "eappSha256") ?? app.signature?.packageHash
  const manifestSha256 = metadataString(app, "manifestSha256") ?? app.signature?.manifestHash
  const releaseId = metadataString(app, "releaseId")
  const runtimeAppId = metadataString(app, "runtimeAppId")
  const developerId = metadataString(app, "developerId") ?? app.signature?.developerId
  const developerName = metadataString(app, "developerName") ?? app.signature?.developerName
  const appPolicyHash = metadataString(app, "appPolicyHash")
  const accessPolicyMode = metadataString(app, "accessPolicyMode") ?? "free-run"
  const packageUrl = metadataString(app, "packageUrl")
  const manifestUrl = metadataString(app, "manifestUrl")
  const cost = runCost(app)

  return (
    <div className="mx-auto max-w-5xl p-4 sm:p-5">
      <button onClick={onBack} className="mb-4 inline-flex h-8 items-center gap-2 rounded-md border border-border bg-secondary/30 px-3 text-xs font-medium text-muted-foreground hover:text-foreground">
        <ArrowLeft className="h-3.5 w-3.5" /> Apps
      </button>
      <div className="rounded-xl border border-border bg-card/55 p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 items-start gap-3">
            <div className="flex h-14 w-14 shrink-0 items-center justify-center rounded-lg border border-primary/20 bg-primary/15 text-primary">{getIconById(app.iconId)}</div>
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <h3 className="text-lg font-semibold leading-6 text-foreground">{app.name}</h3>
                <span className={cn("inline-flex h-6 items-center gap-1 rounded border px-2 text-[11px] font-medium", cached ? "border-[var(--status-online)]/20 bg-[var(--status-online)]/10 text-[var(--status-online)]" : "border-primary/20 bg-primary/10 text-primary")}>
                  <BadgeCheck className="h-3 w-3" /> {cached ? "Cached + verified" : "Network package"}
                </span>
              </div>
              <p className="mt-1 max-w-xl text-xs leading-5 text-muted-foreground">{app.description}</p>
            </div>
          </div>
          <span className="rounded border border-border bg-secondary/35 px-2 py-1 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">{app.source}</span>
        </div>
        <div className="mt-4 flex flex-wrap gap-2">
          <button onClick={onRun} className="inline-flex h-9 items-center justify-center gap-2 rounded-md bg-primary px-3 text-xs font-semibold text-primary-foreground hover:bg-primary/90">
            <Play className="h-3.5 w-3.5" /> Run
          </button>
          {!cached ? (
            <button onClick={onCache} disabled={busy} className="inline-flex h-9 items-center justify-center gap-2 rounded-md border border-border bg-secondary/35 px-3 text-xs font-semibold text-muted-foreground hover:text-foreground disabled:cursor-wait disabled:opacity-70">
              {busy ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Download className="h-3.5 w-3.5" />} {busy ? "Caching" : "Verify & cache"}
            </button>
          ) : (
            <button onClick={onRemoveCache} disabled={busy} className="inline-flex h-9 items-center justify-center gap-2 rounded-md border border-border bg-secondary/35 px-3 text-xs font-semibold text-muted-foreground hover:border-[var(--status-error)]/30 hover:bg-[var(--status-error)]/10 hover:text-[var(--status-error)] disabled:cursor-not-allowed disabled:opacity-40">
              <Trash2 className="h-3.5 w-3.5" /> Remove cache
            </button>
          )}
        </div>
      </div>

      {error ? <div className="mt-3 rounded-md border border-[var(--status-error)]/25 bg-[var(--status-error)]/10 px-3 py-2 text-xs text-[var(--status-error)]">{error}</div> : null}

      <div className="mt-4 grid gap-2 sm:grid-cols-4">
        <InfoCell label="Runtime" value={runtime ?? app.kind} />
        <InfoCell label="Distribution" value={distributionModel === "network-storage-run" ? "network storage" : distributionModel ?? app.source} />
        <InfoCell label="Cache" value={cached ? "local cache" : "pay per retrieval"} />
        <InfoCell label="Assets" value={typeof verifiedAssets === "number" ? `${verifiedAssets} verified` : "verify on run"} />
      </div>

      <SectionTitle>Run cost</SectionTitle>
      <div className="grid gap-2 sm:grid-cols-4">
        <InfoCell label="Run once" value={formatMicroEdge(cost.runOnce)} />
        <InfoCell label="Verify & cache" value={formatMicroEdge(cost.cache)} />
        <InfoCell label="Retrieval" value={formatMicroEdge(cost.retrieval)} />
        <InfoCell label="Pricing" value={cost.source} />
      </div>
      <div className="mt-2 rounded-md border border-primary/20 bg-primary/5 px-3 py-2 text-xs leading-5 text-muted-foreground">
        Cost is deterministic from package bytes and the app policy/pricing schedule. Cached apps verify local hashes and avoid repeated network retrieval payments.
      </div>

      <SectionTitle>App policy</SectionTitle>
      <div className="grid gap-2 sm:grid-cols-2">
        <ProofRow label="access" value={accessPolicyMode} raw={accessPolicyMode} />
        <ProofRow label="policy" value={shortHash(appPolicyHash)} raw={appPolicyHash} />
      </div>

      <SectionTitle>SDK package</SectionTitle>
      <div className="rounded-md border border-primary/20 bg-primary/5 px-3 py-2 text-xs leading-5 text-muted-foreground">
        {sourceOfTruth === "sdk-signed-content-addressed-package" ? "This app runs from signed SDK package artifacts stored on the network. The catalog is discovery only." : "Builtin platform app. Network package proof applies to catalog apps built with the EdgeRun SDK."}
      </div>
      <div className="mt-2 grid gap-2 sm:grid-cols-2">
        <ProofRow label="developer" value={developerName || shortHash(developerId)} raw={developerId} />
        <ProofRow label="eapp" value={shortHash(eappSha256)} raw={eappSha256} />
        <ProofRow label="manifest" value={shortHash(manifestSha256)} raw={manifestSha256} />
        <ProofRow label="release" value={shortHash(releaseId)} raw={releaseId} />
        <ProofRow label="runtime app" value={shortHash(runtimeAppId)} raw={runtimeAppId} />
        <ProofRow label="package" value={typeof packageBytes === "number" ? formatBytes(packageBytes) : "unknown"} />
      </div>

      <SectionTitle>Permissions</SectionTitle>
      <div className="mb-2 rounded-md border border-border bg-secondary/25 px-3 py-2 text-xs leading-5 text-muted-foreground">Capabilities are granted separately through Trust Manager.</div>
      <div className="flex flex-wrap gap-2">{app.requiredCapabilityIds.length > 0 ? capabilityChips(app, cached) : <span className="text-xs text-muted-foreground">No required capabilities</span>}</div>

      {(packageUrl || manifestUrl) ? (
        <>
          <SectionTitle>Network storage</SectionTitle>
          <div className="grid gap-2">
            {packageUrl ? <FileRow label="Package object" value={packageUrl} /> : null}
            {manifestUrl ? <FileRow label="Manifest object" value={manifestUrl} /> : null}
          </div>
        </>
      ) : null}
    </div>
  )
}

function FirstRunDialog({ app, onRunOnce, onCache, onCancel }: { app: AppDefinition; onRunOnce: () => void; onCache: () => void; onCancel: () => void }) {
  const cost = runCost(app)
  const packageBytes = metadataNumber(app, "packageBytes")
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/70 p-4 backdrop-blur-sm">
      <div className="w-full max-w-lg rounded-xl border border-border bg-card p-5 shadow-xl">
        <div className="flex items-start gap-3">
          <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg border border-primary/20 bg-primary/15 text-primary">{getIconById(app.iconId)}</div>
          <div className="min-w-0">
            <h3 className="text-base font-semibold text-foreground">Run {app.name} from network storage?</h3>
            <p className="mt-1 text-xs leading-5 text-muted-foreground">Your browser node will retrieve signed package bytes, verify hashes, and run locally. Retrieval pays storage/CDN providers according to the app policy.</p>
          </div>
        </div>
        <div className="mt-4 grid gap-2 sm:grid-cols-3">
          <InfoCell label="Package" value={typeof packageBytes === "number" ? formatBytes(packageBytes) : "unknown"} />
          <InfoCell label="Run once" value={formatMicroEdge(cost.runOnce)} />
          <InfoCell label="Cache" value={formatMicroEdge(cost.cache)} />
        </div>
        <div className="mt-4 rounded-md border border-border bg-secondary/25 px-3 py-2 text-xs leading-5 text-muted-foreground">Cache locally to avoid repeated retrieval payments. You can remove the cache later from App Store or Trust Manager.</div>
        <div className="mt-5 flex flex-wrap justify-end gap-2">
          <button onClick={onCancel} className="h-9 rounded-md border border-border bg-secondary/35 px-3 text-xs font-semibold text-muted-foreground hover:text-foreground">Cancel</button>
          <button onClick={onRunOnce} className="h-9 rounded-md border border-border bg-secondary/35 px-3 text-xs font-semibold text-foreground hover:bg-secondary">Run once · {formatMicroEdge(cost.runOnce)}</button>
          <button onClick={onCache} className="h-9 rounded-md bg-primary px-3 text-xs font-semibold text-primary-foreground hover:bg-primary/90">Verify & cache · {formatMicroEdge(cost.cache)}</button>
        </div>
      </div>
    </div>
  )
}

function PublishPanel() {
  return (
    <div className="grid gap-4 lg:grid-cols-[1.1fr_0.9fr]">
      <div className="rounded-xl border border-border bg-card/55 p-4">
        <div className="flex items-center gap-2 text-sm font-semibold text-foreground"><Package className="h-4 w-4 text-primary" />Publish from the CLI</div>
        <p className="mt-2 text-xs leading-5 text-muted-foreground">Developers register an identity, pay a hosting/status deposit, and publish signed SDK artifacts from the CLI. The app lives in network storage; users run it by hash and can optionally cache it locally.</p>
        <div className="mt-4 grid gap-2">
          <PublishStep index={1} title="Register" body="Developer identity and deposit establish publisher status and hosting window." />
          <PublishStep index={2} title="Publish" body="CLI packages app.edapp, app.eapp, developer.esig and stores objects on the EdgeRun network." />
          <PublishStep index={3} title="Run or cache" body="Users run from network storage, or cache verified bytes locally to avoid paying retrieval every time." />
        </div>
      </div>
      <div className="rounded-xl border border-border bg-secondary/20 p-4">
        <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">SDK artifacts</div>
        <div className="mt-3 grid gap-2">
          <InfoCell label="Manifest" value="app.edapp" />
          <InfoCell label="Package graph" value="app.eapp" />
          <InfoCell label="Developer signature" value="developer.esig" />
        </div>
        <div className="mt-4 rounded-md border border-primary/20 bg-primary/5 px-3 py-2 text-xs leading-5 text-muted-foreground">Publishing is a CLI flow. The App Store consumes the signed catalog result and lets users verify, run, and cache by content hash.</div>
      </div>
    </div>
  )
}

function PublishStep({ index, title, body }: { index: number; title: string; body: string }) {
  return <div className="rounded-md border border-border bg-secondary/25 px-3 py-2"><div className="text-xs font-semibold text-foreground">{index}. {title}</div><div className="mt-1 text-xs leading-5 text-muted-foreground">{body}</div></div>
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return <h4 className="mb-2 mt-5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{children}</h4>
}

function MetricChip({ label, value, tone = "default" }: { label: string; value: number; tone?: "default" | "warning" }) {
  return <span className={cn("inline-flex h-5 items-center gap-1 rounded border px-1.5 text-[10px] font-medium", tone === "warning" ? "border-[var(--status-warning)]/25 bg-[var(--status-warning)]/10 text-[var(--status-warning)]" : "border-border bg-background/45 text-muted-foreground")}><span className="font-mono text-foreground">{value}</span>{label}</span>
}

function InfoCell({ label, value }: { label: string; value: string }) {
  return <div className="rounded-md border border-border bg-secondary/25 px-3 py-2"><div className="text-[10px] uppercase tracking-wide text-muted-foreground">{label}</div><div className="mt-1 truncate text-xs font-semibold text-foreground">{value}</div></div>
}

function ProofRow({ label, value, raw }: { label: string; value: string; raw?: string }) {
  return (
    <div className="flex h-9 items-center justify-between gap-3 rounded-md border border-border bg-secondary/25 px-3">
      <span className="shrink-0 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">{label}</span>
      <div className="flex min-w-0 items-center gap-2">
        <span className="truncate font-mono text-[11px] text-foreground">{value}</span>
        {raw && raw !== "unknown" ? <button onClick={() => void navigator.clipboard?.writeText(raw)} aria-label={`Copy ${label} proof`} className="flex h-6 w-6 shrink-0 items-center justify-center rounded text-muted-foreground hover:bg-secondary hover:text-foreground" title="Copy"><Copy className="h-3 w-3" /></button> : null}
      </div>
    </div>
  )
}

function FileRow({ label, value }: { label: string; value: string }) {
  return <div className="flex h-9 items-center justify-between gap-3 rounded-md border border-border bg-secondary/25 px-3"><span className="shrink-0 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">{label}</span><span className="min-w-0 truncate font-mono text-[11px] text-foreground">{value}</span></div>
}
