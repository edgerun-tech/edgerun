"use client"

import { useMemo, useState } from "react"
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Clock,
  Copy,
  Database,
  Download,
  Eye,
  Fingerprint,
  GitBranch,
  KeyRound,
  Lock,
  Network,
  Plus,
  RefreshCw,
  Route,
  Save,
  ScrollText,
  Search,
  Shield,
  SlidersHorizontal,
  Trash2,
  XCircle,
  type LucideIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"

type TrustTab = "overview" | "capsules" | "capabilities" | "routes" | "delegations" | "events"
type Risk = "low" | "medium" | "high"
type Status = "strong" | "verified" | "review" | "danger" | "active" | "limited" | "revoked"
type FilterMode = "all" | "review" | "high" | "danger"

type TrustItem = {
  id: string
  title: string
  subtitle: string
  meta: string
  status: Status
  risk?: Risk
  icon: LucideIcon
  authorityRef: string
  proofRef: string
  lastChecked: string
  details: string[]
  actions?: string[]
}

type ActivityEntry = {
  id: string
  time: string
  message: string
  status: Status
}

const tabs: { id: TrustTab; label: string; icon: LucideIcon }[] = [
  { id: "overview", label: "Overview", icon: Activity },
  { id: "capsules", label: "Capsules", icon: Shield },
  { id: "capabilities", label: "Capabilities", icon: SlidersHorizontal },
  { id: "routes", label: "Routes", icon: Route },
  { id: "delegations", label: "Delegations", icon: GitBranch },
  { id: "events", label: "Audit", icon: ScrollText },
]

const filters: { id: FilterMode; label: string }[] = [
  { id: "all", label: "All" },
  { id: "review", label: "Needs review" },
  { id: "high", label: "High risk" },
  { id: "danger", label: "Denied / danger" },
]

const initialItems: Record<TrustTab, TrustItem[]> = {
  overview: [
    { id: "root-policy", title: "Root policy", subtitle: "Ken Personal Root", meta: "2-of-3 devices · Framework · Phone · YubiKey", status: "strong", icon: Fingerprint, authorityRef: "stream:ken-root@head:482", proofRef: "proof:root-quorum:2of3", lastChecked: "just now", details: ["Authority is derived from the latest valid signed root state.", "Current quorum requires 2 trusted devices.", "Old phone key was revoked 3 hours ago."], actions: ["Open root", "Add backup", "Export capsule"] },
    { id: "review-needed", title: "3 routes need review", subtitle: "Policy drift detected", meta: "Home Assistant route · external identity route · payment threshold", status: "review", icon: AlertTriangle, authorityRef: "policy:local-routes@head:91", proofRef: "proof:route-drift:3", lastChecked: "43 sec ago", details: ["A route can still be valid while needing review.", "Review does not revoke existing signed history.", "Unreviewed future actions should ask first."], actions: ["Review routes", "Simulate policy"] },
    { id: "denied-actions", title: "4 denied actions", subtitle: "Blocked by local policy", meta: "last 24 hours", status: "danger", icon: XCircle, authorityRef: "audit:local-denials@head:144", proofRef: "event:event-denied-private-folder", lastChecked: "2 min ago", details: ["Unknown App attempted private folder access.", "No matching capability grant existed.", "The denial was recorded as local audit evidence."], actions: ["Inspect audit", "Block identity"] },
  ],
  capsules: [
    { id: "ken-personal", title: "Ken Personal", subtitle: "Personal root capsule", meta: "Local file · phone · encrypted backup", status: "strong", icon: Shield, authorityRef: "capsule:ken-personal:v3", proofRef: "proof:local-backup-set", lastChecked: "just now", details: ["Root policy: 2-of-3 devices.", "Portable trust capsule can be exported or backed up.", "Used for local identity, app approval, and delegation roots."], actions: ["Export", "Backup", "Rotate"] },
    { id: "edgerun-org", title: "EdgeRun Organization", subtitle: "Organization capsule", meta: "Board authority · build authority", status: "verified", icon: Database, authorityRef: "capsule:edgerun-org:v1", proofRef: "proof:domain+release-key", lastChecked: "5 min ago", details: ["Verified by domain proof and release key fingerprint.", "Trusted for software releases and infrastructure metadata.", "Does not grant access to personal data."], actions: ["Open", "Pin", "Publish"] },
    { id: "web-pki", title: "Public Web PKI", subtitle: "External trust root", meta: "Browser bundle · website certificates only", status: "limited", icon: Lock, authorityRef: "compat:web-pki:system", proofRef: "proof:browser-root-bundle", lastChecked: "boot", details: ["Imported compatibility root.", "Scope is limited to website certificate interpretation.", "Not trusted for app authority or payment policy."], actions: ["Limit scope", "Remove"] },
  ],
  capabilities: [
    { id: "manage-invoices", title: "Manage Invoices", subtitle: "Read invoice emails and create accounting records", meta: "used by 2 routes", status: "active", risk: "medium", icon: SlidersHorizontal, authorityRef: "cap:manage-invoices:v2", proofRef: "delegation:invoice-agent", lastChecked: "2 min ago", details: ["Allows reading invoice emails and extracting fields.", "Allows creating accounting records with source hashes.", "Does not allow sending emails, deleting emails, or approving payments."], actions: ["Edit scope", "Show routes", "Revoke"] },
    { id: "sign-releases", title: "Sign Software Releases", subtitle: "Authorize versioned software artifacts", meta: "used by Build Authority", status: "active", risk: "medium", icon: KeyRound, authorityRef: "cap:sign-releases:v1", proofRef: "delegation:build-authority", lastChecked: "18 min ago", details: ["Allows signing release binaries and publishing manifests.", "Allows revoking compromised releases.", "Does not allow source-code edits or user-data access."], actions: ["Edit policy", "Inspect key"] },
    { id: "approve-payments", title: "Approve Payments", subtitle: "Authorize outbound payouts within limits", meta: "high risk · ask above $500", status: "review", risk: "high", icon: Lock, authorityRef: "cap:approve-payments:v1", proofRef: "delegation:payment-authority", lastChecked: "3 hr ago", details: ["Allows approving payouts under configured limits.", "Final spending must require explicit confirmation.", "Does not allow changing payout addresses or treasury withdrawal."], actions: ["Review", "Tighten", "Disable"] },
  ],
  routes: [
    { id: "invoice-route", title: "Gmail → Invoice Agent → Accounting", subtitle: "Executor: Ken Laptop Agent", meta: "Auto under $500 · ask above · last run 2 min ago", status: "active", risk: "medium", icon: Route, authorityRef: "route:invoice-agent:v5", proofRef: "event:invoice-route-commit", lastChecked: "2 min ago", details: ["Source: Gmail invoice messages.", "Capability: Manage Invoices.", "Destination: Accounting record with signed source hash."], actions: ["Simulate", "Edit", "Pause"] },
    { id: "release-route", title: "GitHub → Build Authority → Release Registry", subtitle: "Executor: Build Authority", meta: "Auto · last run 18 min ago", status: "active", risk: "medium", icon: Route, authorityRef: "route:release-signing:v2", proofRef: "event:release-signing-commit", lastChecked: "18 min ago", details: ["Source: GitHub release event.", "Capability: Sign Software Releases.", "Destination: signed release manifest."], actions: ["Inspect", "Pause"] },
    { id: "home-route", title: "Phone Presence → Home Assistant → Gate", subtitle: "Executor: Local Agent", meta: "Ask first · last run 2 days ago", status: "review", risk: "medium", icon: Route, authorityRef: "route:home-gate:v1", proofRef: "policy-review:physical-access", lastChecked: "2 days ago", details: ["Route affects physical-world behavior.", "Review required before future automatic execution.", "Recommended policy: require local presence + confirmation."], actions: ["Review", "Disable"] },
  ],
  delegations: [
    { id: "build-authority", title: "Build Authority", subtitle: "Issued by Ken Root", meta: "expires Dec 31, 2026", status: "verified", risk: "medium", icon: GitBranch, authorityRef: "delegation:build-authority:v1", proofRef: "chain:ken-root→build-authority", lastChecked: "18 min ago", details: ["Can sign software releases and approve runtime versions.", "Can revoke compromised releases.", "Cannot access user data or change payment settings."], actions: ["Inspect chain", "Rotate", "Revoke"] },
    { id: "dns-authority", title: "DNS Authority", subtitle: "Issued by Infrastructure Authority", meta: "expires Jun 1, 2026", status: "verified", risk: "medium", icon: GitBranch, authorityRef: "delegation:dns-authority:v2", proofRef: "chain:ken-root→infra→dns", lastChecked: "1 hr ago", details: ["Can update records under *.edgerun.tech.", "Cannot transfer domain or change registrar.", "All changes must be logged."], actions: ["Inspect chain", "Revoke"] },
    { id: "payment-authority", title: "Payment Authority", subtitle: "Issued by Ken Root", meta: "expires Mar 15, 2026", status: "review", risk: "high", icon: GitBranch, authorityRef: "delegation:payment-authority:v1", proofRef: "chain:ken-root→payment-authority", lastChecked: "3 hr ago", details: ["Can approve payouts under $500.", "Cannot change payout address or withdraw treasury.", "Recommended: require second factor for every use."], actions: ["Review", "Revoke"] },
  ],
  events: [
    { id: "event-invoice", title: "Invoice Agent created accounting record #4821", subtitle: "Actor: Invoice Agent", meta: "2 min ago · verified", status: "verified", icon: CheckCircle2, authorityRef: "stream:ken-laptop@seq:4821", proofRef: "event-hash:invoice-4821", lastChecked: "2 min ago", details: ["Reason: Manage Invoices capability.", "Result: committed local record with source hash.", "Audit: signed and append-only."], actions: ["Open proof", "Copy hash"] },
    { id: "event-release", title: "Build Authority signed edgerun-node v0.4.2", subtitle: "Actor: Build Authority", meta: "18 min ago · verified", status: "verified", icon: CheckCircle2, authorityRef: "stream:build-authority@seq:87", proofRef: "event-hash:release-v042", lastChecked: "18 min ago", details: ["Reason: Build signing delegation.", "Result: signed release manifest.", "Audit: delegation chain verified."], actions: ["Open release", "Copy proof"] },
    { id: "event-denied", title: "Unknown App denied private folder access", subtitle: "Actor: Unknown App", meta: "7 hr ago · denied", status: "danger", icon: XCircle, authorityRef: "stream:local-policy@seq:144", proofRef: "event-hash:denied-private-folder", lastChecked: "7 hr ago", details: ["Reason: no matching filesystem capability grant.", "Result: blocked before action.", "Recommendation: keep denied unless the app is identified and scoped."], actions: ["Block app", "Inspect source"] },
  ],
}

import { statusClass } from "@/lib/status"

function nowLabel() {
  return new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })
}

function Badge({ value }: { value: Status | Risk }) {
  return <span className={cn("rounded-md border px-1.5 py-0.5 text-[10px] font-semibold uppercase", statusClass(value))}>{value}</span>
}

function TrustItemRow({ item, active, onSelect }: { item: TrustItem; active: boolean; onSelect: () => void }) {
  const Icon = item.icon
  return (
    <button onClick={onSelect} className={cn("flex w-full items-start gap-3 border-b border-border/60 px-3 py-3 text-left transition-colors hover:bg-secondary/60", active && "bg-primary/10")}>
      <div className={cn("mt-0.5 flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg border", statusClass(item.risk ?? item.status))}>
        <Icon className="h-4 w-4" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <div className="truncate text-sm font-medium text-foreground">{item.title}</div>
          <Badge value={item.risk ?? item.status} />
        </div>
        <div className="mt-0.5 truncate text-xs text-muted-foreground">{item.subtitle}</div>
        <div className="mt-1 truncate text-[11px] text-muted-foreground/70">{item.meta}</div>
      </div>
    </button>
  )
}

function Inspector({ item, onAction, onCopy }: { item: TrustItem; onAction: (action: string) => void; onCopy: (value: string, label: string) => void }) {
  const Icon = item.icon
  return (
    <aside className="flex w-80 flex-shrink-0 flex-col border-l border-border bg-[var(--window-header)]/30">
      <div className="border-b border-border p-4">
        <div className="mb-3 flex items-start justify-between gap-3">
          <div className={cn("flex h-10 w-10 items-center justify-center rounded-xl border", statusClass(item.risk ?? item.status))}>
            <Icon className="h-5 w-5" />
          </div>
          <div className="flex gap-1">
            <Badge value={item.status} />
            {item.risk && <Badge value={item.risk} />}
          </div>
        </div>
        <h3 className="text-sm font-semibold text-foreground">{item.title}</h3>
        <p className="mt-1 text-xs text-muted-foreground">{item.subtitle}</p>
        <p className="mt-2 text-[11px] text-muted-foreground/70">{item.meta}</p>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-4">
        <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Protocol evidence</div>
        <div className="mb-4 space-y-2 rounded-lg border border-border bg-background/60 p-2 font-mono text-[10px] text-muted-foreground">
          <button onClick={() => onCopy(item.authorityRef, "authority ref")} className="flex w-full items-center justify-between gap-2 text-left hover:text-foreground"><span>authority</span><span className="truncate text-foreground">{item.authorityRef}</span><Copy className="h-3 w-3" /></button>
          <button onClick={() => onCopy(item.proofRef, "proof ref")} className="flex w-full items-center justify-between gap-2 text-left hover:text-foreground"><span>proof</span><span className="truncate text-foreground">{item.proofRef}</span><Copy className="h-3 w-3" /></button>
          <div className="flex items-center justify-between gap-2"><span>checked</span><span className="truncate text-foreground">{item.lastChecked}</span></div>
        </div>

        <div className="mb-3 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Why trusted / blocked</div>
        <div className="space-y-2">
          {item.details.map((detail) => <div key={detail} className="rounded-lg border border-border bg-background/60 p-2 text-xs leading-relaxed text-muted-foreground">{detail}</div>)}
        </div>

        <div className="mt-5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Actions</div>
        <div className="mt-2 grid grid-cols-2 gap-2">
          {(item.actions ?? ["Inspect"]).map((action, index) => (
            <button key={action} onClick={() => onAction(action)} className={cn("rounded-md px-2 py-1.5 text-xs font-medium transition-colors", index === 0 ? "bg-primary text-primary-foreground hover:bg-primary/90" : "bg-secondary text-secondary-foreground hover:bg-secondary/80")}>
              {action}
            </button>
          ))}
        </div>
      </div>
    </aside>
  )
}

function OverviewGraph({ selectedId }: { selectedId: string }) {
  const nodes = [
    { id: "root", x: 180, y: 44, label: "Ken Root", status: "strong" as Status },
    { id: "infra", x: 88, y: 130, label: "Infra", status: "verified" as Status },
    { id: "build", x: 272, y: 130, label: "Build", status: "verified" as Status },
    { id: "dns", x: 48, y: 218, label: "DNS", status: "active" as Status },
    { id: "server", x: 130, y: 218, label: "Server", status: "active" as Status },
    { id: "release", x: 312, y: 218, label: "Release", status: "active" as Status },
    { id: "unknown", x: 218, y: 286, label: "Denied", status: "danger" as Status },
  ]
  const edges = [["root", "infra"], ["root", "build"], ["infra", "dns"], ["infra", "server"], ["build", "release"], ["server", "unknown"]]
  const byId = Object.fromEntries(nodes.map((node) => [node.id, node]))

  return (
    <div className="h-full rounded-xl border border-border bg-background/70 p-3">
      <div className="mb-2 flex items-center justify-between">
        <div><div className="text-sm font-semibold text-foreground">Live trust graph</div><div className="text-[11px] text-muted-foreground">authority edges and denied path</div></div>
        <Network className="h-4 w-4 text-muted-foreground" />
      </div>
      <svg viewBox="0 0 360 330" className="h-[calc(100%-42px)] w-full">
        {edges.map(([a, b]) => {
          const from = byId[a]
          const to = byId[b]
          return <line key={`${a}-${b}`} x1={from.x} y1={from.y} x2={to.x} y2={to.y} stroke="currentColor" className="text-border" strokeDasharray="4 4" />
        })}
        {nodes.map((node) => <g key={node.id}><circle cx={node.x} cy={node.y} r={selectedId.includes(node.id) ? 18 : 13} className={cn("stroke-current", statusClass(node.status))} fill="currentColor" fillOpacity="0.12" strokeWidth="1.5" /><text x={node.x} y={node.y + 30} textAnchor="middle" className="fill-muted-foreground text-[10px] font-mono">{node.label}</text></g>)}
      </svg>
    </div>
  )
}

function matchesFilter(item: TrustItem, filter: FilterMode) {
  if (filter === "all") return true
  if (filter === "review") return item.status === "review"
  if (filter === "high") return item.risk === "high"
  if (filter === "danger") return item.status === "danger" || item.status === "revoked"
  return true
}

export function TrustManagerApp() {
  const [items, setItems] = useState(initialItems)
  const [activity, setActivity] = useState<ActivityEntry[]>([])
  const [tab, setTab] = useState<TrustTab>("overview")
  const [query, setQuery] = useState("")
  const [filter, setFilter] = useState<FilterMode>("all")
  const list = items[tab]
  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    return list.filter((item) => {
      const textMatch = !q || `${item.title} ${item.subtitle} ${item.meta}`.toLowerCase().includes(q)
      return textMatch && matchesFilter(item, filter)
    })
  }, [list, query, filter])
  const [selectedId, setSelectedId] = useState<string>(list[0]?.id ?? "")
  const selected = filtered.find((item) => item.id === selectedId) ?? filtered[0] ?? list[0]
  const activeTab = tabs.find((item) => item.id === tab) ?? tabs[0]
  const ActiveIcon = activeTab.icon
  const allItems = useMemo(() => Object.values(items).flat(), [items])
  const statusCounts = useMemo(() => ({
    verified: allItems.filter((item) => ["strong", "verified", "active"].includes(item.status)).length,
    review: allItems.filter((item) => item.status === "review").length,
    danger: allItems.filter((item) => item.status === "danger" || item.status === "revoked").length,
  }), [allItems])

  function log(message: string, status: Status = "verified") {
    setActivity((prev) => [{ id: `${Date.now()}-${Math.random()}`, time: nowLabel(), message, status }, ...prev].slice(0, 8))
  }

  function updateSelected(patch: Partial<TrustItem>) {
    if (!selected) return
    setItems((prev) => ({
      ...prev,
      [tab]: prev[tab].map((item) => item.id === selected.id ? { ...item, ...patch } : item),
    }))
  }

  function removeSelected() {
    if (!selected) return
    const nextList = items[tab].filter((item) => item.id !== selected.id)
    setItems((prev) => ({ ...prev, [tab]: nextList }))
    setSelectedId(nextList[0]?.id ?? "")
    log(`Removed ${selected.title}`, "review")
  }

  async function copyValue(value: string, label: string) {
    await navigator.clipboard.writeText(value)
    log(`Copied ${label}: ${value}`)
  }

  function exportSelected() {
    if (!selected) return
    const blob = new Blob([JSON.stringify(selected, null, 2)], { type: "application/json" })
    const url = URL.createObjectURL(blob)
    const a = document.createElement("a")
    a.href = url
    a.download = `${selected.id}.trust.json`
    a.click()
    URL.revokeObjectURL(url)
    log(`Exported ${selected.title}`)
  }

  function refreshAll() {
    setItems((prev) => Object.fromEntries(Object.entries(prev).map(([key, value]) => [key, value.map((item) => ({ ...item, lastChecked: "just now" }))])) as Record<TrustTab, TrustItem[]>)
    log("Refreshed local trust snapshot")
  }

  function addRecord() {
    const id = `local-${tab}-${Date.now()}`
    const item: TrustItem = {
      id,
      title: `New ${activeTab.label} record`,
      subtitle: "Local draft",
      meta: "not committed · local only",
      status: "review",
      risk: tab === "capabilities" || tab === "routes" || tab === "delegations" ? "medium" : undefined,
      icon: activeTab.icon,
      authorityRef: `draft:${id}`,
      proofRef: "proof:pending",
      lastChecked: "just now",
      details: ["This is a local draft record.", "It is not authoritative until validated and committed to the appropriate stream."],
      actions: ["Review", "Remove"],
    }
    setItems((prev) => ({ ...prev, [tab]: [item, ...prev[tab]] }))
    setSelectedId(id)
    log(`Created draft ${item.title}`, "review")
  }

  function handleAction(action: string) {
    if (!selected) return
    const normalized = action.toLowerCase()
    if (normalized.includes("copy")) return void copyValue(selected.proofRef, "proof ref")
    if (normalized.includes("export")) return exportSelected()
    if (normalized.includes("remove")) return removeSelected()
    if (normalized.includes("revoke") || normalized.includes("disable") || normalized.includes("block")) {
      updateSelected({ status: "revoked", meta: `${selected.meta} · revoked ${nowLabel()}`, lastChecked: "just now" })
      log(`${selected.title} marked revoked`, "danger")
      return
    }
    if (normalized.includes("pause")) {
      updateSelected({ status: "review", meta: `${selected.meta} · paused`, lastChecked: "just now" })
      log(`${selected.title} paused for review`, "review")
      return
    }
    if (normalized.includes("review") || normalized.includes("tighten") || normalized.includes("edit")) {
      updateSelected({ status: "verified", meta: `${selected.meta} · reviewed ${nowLabel()}`, lastChecked: "just now" })
      log(`${selected.title} reviewed and verified`)
      return
    }
    if (normalized.includes("simulate")) {
      const event: TrustItem = { id: `sim-${Date.now()}`, title: `Simulated ${selected.title}`, subtitle: "Policy simulation", meta: `generated ${nowLabel()} · no state committed`, status: "verified", icon: CheckCircle2, authorityRef: selected.authorityRef, proofRef: `simulation:${selected.id}`, lastChecked: "just now", details: ["Simulation used current local policy snapshot.", "No command was committed.", "Result is advisory evidence only."], actions: ["Copy proof", "Remove"] }
      setItems((prev) => ({ ...prev, events: [event, ...prev.events] }))
      log(`Simulation completed for ${selected.title}`)
      return
    }
    updateSelected({ lastChecked: "just now" })
    log(`${action} executed for ${selected.title}`)
  }

  function selectTab(next: TrustTab) {
    setTab(next)
    setQuery("")
    setFilter("all")
    setSelectedId(items[next][0]?.id ?? "")
  }

  return (
    <div className="flex h-full min-h-0 bg-background text-foreground">
      <aside className="flex w-52 flex-shrink-0 flex-col border-r border-border bg-[var(--window-header)]/70">
        <div className="border-b border-border p-3"><div className="flex items-center gap-2"><div className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/15 text-primary"><Shield className="h-5 w-5" /></div><div className="min-w-0"><div className="truncate text-sm font-semibold">Trust Manager</div><div className="text-[10px] text-muted-foreground">authority control</div></div></div></div>
        <nav className="flex-1 space-y-1 p-2">{tabs.map((item) => { const Icon = item.icon; const count = items[item.id].length; return <button key={item.id} onClick={() => selectTab(item.id)} className={cn("flex w-full items-center gap-2 rounded-md px-2 py-2 text-left text-xs font-medium transition-colors", tab === item.id ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-secondary hover:text-foreground")}><Icon className="h-3.5 w-3.5" /><span className="min-w-0 flex-1 truncate">{item.label}</span><span className="rounded bg-background/40 px-1.5 py-0.5 text-[10px]">{count}</span></button> })}</nav>
        <div className="border-t border-border p-3"><div className="flex items-center gap-2 text-[10px] text-muted-foreground"><Clock className="h-3 w-3" />local policy snapshot</div></div>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 flex-shrink-0 items-center justify-between border-b border-border bg-[var(--window-header)]/40 px-3">
          <div className="flex items-center gap-2"><ActiveIcon className="h-4 w-4 text-primary" /><div><h2 className="text-sm font-semibold">{activeTab.label}</h2><p className="text-[11px] text-muted-foreground">explicit trust, bounded authority, auditable decisions</p></div></div>
          <div className="flex items-center gap-2">
            <div className="relative"><Search className="absolute left-2 top-1/2 h-3 w-3 -translate-y-1/2 text-muted-foreground" /><input value={query} onChange={(event) => setQuery(event.target.value)} className="h-8 w-52 rounded-md border border-border bg-background pl-7 pr-2 text-xs outline-none focus:border-primary/50" placeholder="Search trust state..." /></div>
            <button onClick={refreshAll} className="flex h-8 items-center gap-1.5 rounded-md bg-secondary px-2 text-xs font-medium hover:bg-secondary/80"><RefreshCw className="h-3.5 w-3.5" />Refresh</button>
            <button onClick={addRecord} className="flex h-8 items-center gap-1.5 rounded-md bg-primary px-2 text-xs font-medium text-primary-foreground hover:bg-primary/90"><Plus className="h-3.5 w-3.5" />New</button>
            <button onClick={exportSelected} disabled={!selected} className="flex h-8 items-center gap-1.5 rounded-md bg-secondary px-2 text-xs font-medium hover:bg-secondary/80 disabled:opacity-40"><Download className="h-3.5 w-3.5" />Export</button>
          </div>
        </header>

        <div className="flex h-10 flex-shrink-0 items-center gap-2 border-b border-border bg-background/60 px-3">{filters.map((item) => <button key={item.id} onClick={() => setFilter(item.id)} className={cn("rounded-md px-2 py-1 text-[11px] font-medium transition-colors", filter === item.id ? "bg-secondary text-foreground" : "text-muted-foreground hover:bg-secondary/60 hover:text-foreground")}>{item.label}</button>)}<div className="ml-auto flex items-center gap-2 text-[10px] text-muted-foreground"><span>{filtered.length} shown</span><span>·</span><span>{statusCounts.verified} ok</span><span>{statusCounts.review} review</span><span>{statusCounts.danger} danger</span></div></div>

        <div className="grid min-h-0 flex-1 grid-cols-[minmax(300px,1fr)_320px]">
          <section className="min-w-0 overflow-hidden border-r border-border">
            {tab === "overview" ? <div className="grid h-full min-h-0 grid-rows-[160px_1fr] gap-3 p-3"><div className="grid grid-cols-3 gap-3">{items.overview.map((item) => { const Icon = item.icon; return <button key={item.id} onClick={() => setSelectedId(item.id)} className={cn("rounded-xl border border-border bg-card p-3 text-left transition-colors hover:bg-secondary/50", selected?.id === item.id && "border-primary/40 bg-primary/5")}><div className="mb-3 flex items-center justify-between"><Icon className="h-4 w-4 text-primary" /><Badge value={item.status} /></div><div className="text-sm font-semibold text-foreground">{item.title}</div><div className="mt-1 truncate text-[11px] text-muted-foreground">{item.subtitle}</div><div className="mt-1 truncate text-[10px] text-muted-foreground/70">{item.meta}</div></button> })}</div><OverviewGraph selectedId={selected?.id ?? ""} /></div> : <div className="h-full overflow-auto">{filtered.length > 0 ? filtered.map((item) => <TrustItemRow key={item.id} item={item} active={selected?.id === item.id} onSelect={() => setSelectedId(item.id)} />) : <div className="flex h-full items-center justify-center text-sm text-muted-foreground">No matching trust records</div>}</div>}
          </section>
          {selected && <Inspector item={selected} onAction={handleAction} onCopy={copyValue} />}
        </div>

        <footer className="flex h-7 flex-shrink-0 items-center justify-between border-t border-border bg-[var(--window-header)]/40 px-3 font-mono text-[10px] text-muted-foreground">
          <span>{activity[0]?.message ?? "policy engine: local · grants: bounded · stream authority: append-only"}</span>
          <span>selected: {selected?.id ?? "none"}</span>
        </footer>
      </main>
    </div>
  )
}

/** @deprecated Use TrustManagerApp instead */
export const TrustManagerSurface = TrustManagerApp
