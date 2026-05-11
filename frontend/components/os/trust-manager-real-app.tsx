"use client"

import { useMemo, useState } from "react"
import { useStore } from "@nanostores/react"
import {
  Activity,
  AlertTriangle,
  BadgeCheck,
  Copy,
  Fingerprint,
  KeyRound,
  Lock,
  Package,
  Route,
  ScrollText,
  Shield,
  type LucideIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { AppEmptyState, AppHeader, AppToolbar } from "@/components/os/app-chrome"
import { useAuth } from "@/hooks/use-auth"
import { browserAppInstallStore } from "@/platform/runtime/browser-app-install-store"
import { runtimeEventLog, runtimeEventLogStore } from "@/platform/runtime/runtime-event-log"
import { buildTrustProjection, type TrustProjectionItem, type TrustProjectionTab } from "@/platform/trust/trust-manager-projection"
import { uninstallCatalogApp } from "@/platform/registries/app-catalog-registry"
import { listBuiltinApps } from "@/platform/registries/builtin-app-registry"
import { uninstallApp } from "@/stores/installed-apps-store"
import { localCapabilityGrantsStore, revokeLocalCapabilityGrant } from "@/stores/local-capability-grants-store"
import type { AppDefinition } from "@/platform/types/app-definition"

type Tab = TrustProjectionTab

type Filter = "all" | "review" | "danger" | "packages" | "capabilities"

interface TrustManagerSurfaceProps {
  onLaunchApp?: (app: AppDefinition) => void
}

const tabs: { id: Tab; label: string; icon: LucideIcon }[] = [
  { id: "overview", label: "Overview", icon: Activity },
  { id: "capsules", label: "Trust Container", icon: Shield },
  { id: "packages", label: "Packages", icon: Package },
  { id: "capabilities", label: "Capabilities", icon: KeyRound },
  { id: "routes", label: "Routes", icon: Route },
  { id: "events", label: "Audit", icon: ScrollText },
]

const tabIconById = Object.fromEntries(tabs.map((tab) => [tab.id, tab.icon])) as Record<Tab, LucideIcon>

function statusClass(status: TrustProjectionItem["status"] | TrustProjectionItem["risk"]) {
  switch (status) {
    case "strong":
    case "verified":
    case "active":
    case "low":
      return "border-[var(--status-online)]/30 bg-[var(--status-online)]/10 text-[var(--status-online)]"
    case "review":
    case "limited":
    case "medium":
      return "border-[var(--status-warning)]/30 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
    case "danger":
    case "revoked":
    case "high":
      return "border-[var(--status-error)]/30 bg-[var(--status-error)]/10 text-[var(--status-error)]"
    default:
      return "border-border bg-secondary/30 text-muted-foreground"
  }
}

function short(value: string | undefined) {
  if (!value) return "missing"
  return value.length > 22 ? `${value.slice(0, 14)}…${value.slice(-6)}` : value
}

function matchesFilter(item: TrustProjectionItem, filter: Filter) {
  if (filter === "all") return true
  if (filter === "review") return item.status === "review" || item.status === "limited"
  if (filter === "danger") return item.status === "danger" || item.status === "revoked" || item.risk === "high"
  if (filter === "packages") return item.tab === "packages"
  if (filter === "capabilities") return item.tab === "capabilities"
  return true
}

function builtinApp(appId: string): AppDefinition | undefined {
  return listBuiltinApps().find((app) => app.appId === appId)
}

function capabilityFromAuthorityRef(authorityRef: string): { appId: string; capabilityId: string } | null {
  const prefix = "capability:"
  if (!authorityRef.startsWith(prefix)) return null
  const rest = authorityRef.slice(prefix.length)
  const index = rest.indexOf(":")
  if (index <= 0) return null
  return { appId: rest.slice(0, index), capabilityId: rest.slice(index + 1) }
}

function appIdFromPackageItem(item: TrustProjectionItem): string | null {
  const prefix = "package-"
  return item.id.startsWith(prefix) ? item.id.slice(prefix.length) : null
}

function Badge({ value }: { value?: TrustProjectionItem["status"] | TrustProjectionItem["risk"] }) {
  if (!value) return null
  return <span className={cn("rounded-md border px-1.5 py-0.5 text-[10px] font-semibold uppercase", statusClass(value))}>{value}</span>
}

function ItemIcon({ item }: { item: TrustProjectionItem }) {
  const Icon = tabIconById[item.tab] ?? Shield
  return (
    <div className={cn("flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border", statusClass(item.risk ?? item.status))}>
      <Icon className="h-4 w-4" />
    </div>
  )
}

function TrustRow({ item, selected, onSelect }: { item: TrustProjectionItem; selected: boolean; onSelect: () => void }) {
  return (
    <button
      onClick={onSelect}
      className={cn(
        "flex w-full items-start gap-3 border-b border-border/60 px-3 py-3 text-left transition-colors hover:bg-secondary/50",
        selected && "bg-primary/10",
      )}
    >
      <ItemIcon item={item} />
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-2">
          <div className="truncate text-sm font-medium text-foreground">{item.title}</div>
          <Badge value={item.risk ?? item.status} />
        </div>
        <div className="mt-0.5 truncate text-xs text-muted-foreground">{item.subtitle}</div>
        <div className="mt-1 truncate text-[11px] text-muted-foreground/75">{item.meta}</div>
      </div>
    </button>
  )
}

function Inspector({ item, onAction }: { item: TrustProjectionItem | undefined; onAction: (item: TrustProjectionItem, action: string) => void }) {
  if (!item) {
    return (
      <aside className="flex w-80 shrink-0 items-center justify-center border-l border-border bg-[var(--window-header)]/25 p-4 text-sm text-muted-foreground">
        Select a trust record
      </aside>
    )
  }
  return (
    <aside className="flex w-80 shrink-0 flex-col border-l border-border bg-[var(--window-header)]/25">
      <div className="border-b border-border p-4">
        <div className="mb-3 flex items-start justify-between gap-3">
          <ItemIcon item={item} />
          <div className="flex gap-1">
            <Badge value={item.status} />
            <Badge value={item.risk} />
          </div>
        </div>
        <h3 className="text-sm font-semibold text-foreground">{item.title}</h3>
        <p className="mt-1 text-xs text-muted-foreground">{item.subtitle}</p>
        <p className="mt-2 text-[11px] text-muted-foreground/75">{item.meta}</p>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-4">
        <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Protocol evidence</div>
        <div className="mb-4 space-y-2 rounded-lg border border-border bg-background/60 p-2 font-mono text-[10px] text-muted-foreground">
          <EvidenceRow label="authority" value={item.authorityRef} />
          <EvidenceRow label="proof" value={item.proofRef} />
          <EvidenceRow label="checked" value={item.lastChecked} />
        </div>

        <div className="mb-3 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Why this matters</div>
        <div className="space-y-2">
          {item.details.map((detail) => (
            <div key={detail} className="rounded-lg border border-border bg-background/60 p-2 text-xs leading-relaxed text-muted-foreground">
              {detail}
            </div>
          ))}
        </div>

        <div className="mt-5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Actions</div>
        <div className="mt-2 grid grid-cols-2 gap-2">
          {(item.actions ?? ["Inspect"]).map((action, index) => (
            <button
              key={action}
              onClick={() => onAction(item, action)}
              className={cn(
                "rounded-md px-2 py-1.5 text-xs font-medium transition-colors",
                index === 0 ? "bg-primary text-primary-foreground hover:bg-primary/90" : "bg-secondary text-secondary-foreground hover:bg-secondary/80",
              )}
            >
              {action}
            </button>
          ))}
        </div>
      </div>
    </aside>
  )
}

function EvidenceRow({ label, value }: { label: string; value: string }) {
  return (
    <button
      onClick={() => void navigator.clipboard?.writeText(value)}
      className="flex w-full items-center justify-between gap-2 text-left hover:text-foreground"
      title="Copy"
    >
      <span>{label}</span>
      <span className="min-w-0 truncate text-foreground">{short(value)}</span>
      <Copy className="h-3 w-3 shrink-0" />
    </button>
  )
}

export function TrustManagerSurface({ onLaunchApp }: TrustManagerSurfaceProps) {
  const auth = useAuth()
  const appState = useStore(browserAppInstallStore)
  const capabilityGrants = useStore(localCapabilityGrantsStore)
  const runtimeEvents = useStore(runtimeEventLogStore)
  const [tab, setTab] = useState<Tab>("overview")
  const [filter, setFilter] = useState<Filter>("all")
  const [query, setQuery] = useState("")
  const [selectedId, setSelectedId] = useState<string | null>(null)

  const projection = useMemo(() => buildTrustProjection({
    profile: auth.unlockedProfile,
    appState,
    capabilityGrants,
    runtimeEvents,
  }), [appState, auth.unlockedProfile, capabilityGrants, runtimeEvents])

  const allItems = useMemo(() => Object.values(projection).flat(), [projection])
  const tabItems = projection[tab] ?? []
  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    return tabItems.filter((item) => {
      const text = `${item.title} ${item.subtitle} ${item.meta} ${item.authorityRef} ${item.proofRef}`.toLowerCase()
      return (!q || text.includes(q)) && matchesFilter(item, filter)
    })
  }, [filter, query, tabItems])
  const selected = filtered.find((item) => item.id === selectedId) ?? filtered[0]
  const activeTab = tabs.find((item) => item.id === tab) ?? tabs[0]
  const ActiveIcon = activeTab.icon
  const okCount = allItems.filter((item) => ["strong", "verified", "active"].includes(item.status)).length
  const reviewCount = allItems.filter((item) => ["review", "limited"].includes(item.status)).length
  const dangerCount = allItems.filter((item) => ["danger", "revoked"].includes(item.status)).length

  function selectTab(next: Tab) {
    setTab(next)
    setSelectedId(null)
    setQuery("")
    setFilter("all")
  }

  async function handleAction(item: TrustProjectionItem, action: string) {
    const normalized = action.toLowerCase()
    if (normalized.includes("identity") || normalized.includes("passkey") || normalized.includes("export")) {
      const app = builtinApp("identity")
      if (app) onLaunchApp?.(app)
      return
    }
    if (normalized.includes("app store") || normalized.includes("package")) {
      const app = builtinApp("app-store")
      if (app) onLaunchApp?.(app)
      return
    }
    if (normalized.includes("revoke")) {
      const grant = capabilityFromAuthorityRef(item.authorityRef)
      if (grant) {
        revokeLocalCapabilityGrant(grant.appId, grant.capabilityId)
        runtimeEventLog.append({
          kind: "capability_grant_denied",
          actor: grant.appId,
          capabilityId: grant.capabilityId,
          reason: "revoked from Trust Manager",
        })
      }
      return
    }
    if (normalized.includes("remove cache")) {
      const appId = appIdFromPackageItem(item)
      if (appId) {
        await uninstallCatalogApp(appId).catch(() => undefined)
        uninstallApp(appId)
        runtimeEventLog.append({
          kind: "app_stopped",
          actor: appId,
          reason: "local cache removed from Trust Manager",
        })
      }
      return
    }
    if (normalized.includes("copy")) {
      await navigator.clipboard?.writeText(item.proofRef)
    }
  }

  return (
    <div className="flex h-full min-h-0 bg-background text-foreground">
      <aside className="flex w-56 shrink-0 flex-col border-r border-border bg-[var(--window-header)]/70">
        <div className="border-b border-border p-3">
          <div className="flex items-center gap-2">
            <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/15 text-primary">
              <Shield className="h-5 w-5" />
            </div>
            <div className="min-w-0">
              <div className="truncate text-sm font-semibold">Trust Manager</div>
              <div className="text-[10px] text-muted-foreground">real local authority</div>
            </div>
          </div>
        </div>
        <nav className="flex-1 space-y-1 p-2">
          {tabs.map((item) => {
            const Icon = item.icon
            const count = projection[item.id].length
            return (
              <button
                key={item.id}
                onClick={() => selectTab(item.id)}
                className={cn(
                  "flex w-full items-center gap-2 rounded-md px-2 py-2 text-left text-xs font-medium transition-colors",
                  tab === item.id ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-secondary hover:text-foreground",
                )}
              >
                <Icon className="h-3.5 w-3.5" />
                <span className="min-w-0 flex-1 truncate">{item.label}</span>
                <span className="rounded bg-background/40 px-1.5 py-0.5 text-[10px]">{count}</span>
              </button>
            )
          })}
        </nav>
        <div className="border-t border-border p-3 text-[10px] text-muted-foreground">
          <div className="flex items-center gap-2"><Fingerprint className="h-3 w-3" />{auth.unlockedProfile ? short(auth.unlockedProfile.owner.identityIdHex) : "locked"}</div>
        </div>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col">
        <AppHeader title={activeTab.label} icon={<ActiveIcon className="h-4 w-4" />}>
          <div className="flex items-center gap-2">
            <Metric label="ok" value={okCount} icon={BadgeCheck} />
            <Metric label="review" value={reviewCount} icon={AlertTriangle} />
            <Metric label="danger" value={dangerCount} icon={Lock} />
          </div>
        </AppHeader>
        <AppToolbar className="border-b border-border">
          <div className="flex h-9 min-w-0 flex-1 items-center gap-2 rounded-md border border-border bg-card/70 px-3 sm:max-w-sm">
            <Activity className="h-3.5 w-3.5 text-muted-foreground" />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search authority, package hash, proof..."
              className="min-w-0 flex-1 bg-transparent text-xs text-foreground outline-none placeholder:text-muted-foreground"
            />
          </div>
          <div className="flex items-center gap-1 overflow-x-auto pb-0.5">
            {(["all", "review", "danger", "packages", "capabilities"] as Filter[]).map((item) => (
              <button
                key={item}
                onClick={() => setFilter(item)}
                className={cn(
                  "h-7 shrink-0 rounded border px-2 text-[11px] font-medium capitalize",
                  filter === item ? "border-primary/40 bg-primary/15 text-primary" : "border-border bg-secondary/25 text-muted-foreground hover:text-foreground",
                )}
              >
                {item}
              </button>
            ))}
          </div>
        </AppToolbar>

        <div className="grid min-h-0 flex-1 grid-cols-[minmax(300px,1fr)_320px]">
          <section className="min-w-0 overflow-auto border-r border-border">
            {filtered.length > 0 ? filtered.map((item) => (
              <TrustRow key={item.id} item={item} selected={selected?.id === item.id} onSelect={() => setSelectedId(item.id)} />
            )) : <AppEmptyState>No trust records in this view</AppEmptyState>}
          </section>
          <Inspector item={selected} onAction={(item, action) => void handleAction(item, action)} />
        </div>
      </main>
    </div>
  )
}

function Metric({ label, value, icon: Icon }: { label: string; value: number; icon: LucideIcon }) {
  return (
    <span className="inline-flex h-5 items-center gap-1 rounded border border-border bg-background/45 px-1.5 text-[10px] font-medium text-muted-foreground">
      <Icon className="h-3 w-3" />
      <span className="font-mono text-foreground">{value}</span>
      {label}
    </span>
  )
}
